//! Wires a pointing sample to the selector, the trip, and the simulated gripper.
//!
//! `point_from_sample` does not take a referee id. [`stage_report`] only posts
//! the selector's bag. [`confirm_with_mock`] accepts a report effect and asks
//! [`hri::MockReport`] for the id to name. `Ok` from `Trip::apply` is not
//! confirmation: a stamp after the budget stores `TimeUp` and returns
//! [`FeedError::TimeUp`]. A pick, ready signal, follow, or release is not a
//! report.
//!
//! [`execute_pick`] runs only after the trip would accept the grasp and the
//! gripper decision agrees. [`confirm_ready`] is a later call through
//! [`hri::MockReady`]. It does not follow and it does not move the base.
//! [`execute_handoff`] does not invent arrival. A caller that already applied
//! `Event::Arrived` has supplied a fixture event, not a navigation result.
//!
//! The pick path still reads the trip stamp on its own. Neither count is a wall
//! clock. A world time of 400000 ms does not by itself make a pick at trip stamp
//! 10 late. [`step_drive`] is the navigation path: there the world clock and the
//! trip stamp are the same count, and a split or a backward stamp changes nothing.
//! [`run`] is the full task: one stamp from the report through the
//! optional return. Its pass check is the world at the designated car, not the
//! trip flag. The component cue can still set that flag.

use domain::{Bag, Custody, Id, Point, World};
use hri::{MockReady, MockReport};
use manipulation::{Act, Command, GripError, Scene, assume, decide};
use perception::{Candidate, SelectError, select};
use sim_runtime::{ArmError, SimWorld};
use task_3_1::{Effect, Event, Outcome, Phase, Reject, Trip};

mod drive;
mod episode;
mod gpsr;
#[cfg(test)]
mod manifest;
mod replay;

pub use drive::{Drive, NavEnd, step_drive};
pub use episode::{EpisodeReport, Script, run};
pub use replay::{ReplayCase, replay_svg};

/// The trip rejected the event, the effect was not a report, or the budget ended.
///
/// [`FeedError::Trip`] leaves the trip unchanged. [`FeedError::TimeUp`] means the
/// deadline was stored and the payload was not applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedError {
    Trip(Reject),
    NotAReport,
    NotACommand,
    TimeUp,
    Arm(GripError),
    Split,
    Terminal,
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FeedError::Trip(_) => "trip rejected",
            FeedError::NotAReport => "not a report",
            FeedError::NotACommand => "not a command",
            FeedError::TimeUp => "time up",
            FeedError::Arm(_) => "gripper refused",
            FeedError::Split => "clocks differ",
            FeedError::Terminal => "terminal",
        })
    }
}

impl std::error::Error for FeedError {}

/// Selects a bag from the pointing sample and these bags.
///
/// The referee assignment is not an argument. Invisible bags stay in the slice
/// so the selector can ignore them.
pub fn point_from_sample(
    origin: Option<Point>,
    direction: Option<(i32, i32)>,
    bags: &[Bag],
) -> Result<Id, SelectError> {
    let candidates: Vec<Candidate> = bags
        .iter()
        .map(|bag| Candidate {
            id: bag.id,
            at: bag.pose.point(),
            visible: bag.visible,
        })
        .collect();
    select(origin, direction, &candidates)
}

/// Posts `bag` as the pointed bag. Does not confirm the report.
///
/// `Ok` is [`Effect::ReportBag`] and the phase is `Naming`. A stamp after the
/// budget stores `TimeUp` and returns [`FeedError::TimeUp`]. The bag is not
/// posted. Any other rejection leaves the trip unchanged.
pub fn stage_report(
    trip: &mut Trip,
    now_ms: u64,
    operator: Id,
    bag: Id,
) -> Result<Effect, FeedError> {
    let (next, outcome) = admit(trip, now_ms, Event::Pointed { operator, bag })?;
    match outcome.effect {
        Some(effect @ Effect::ReportBag { .. }) => {
            *trip = next;
            Ok(effect)
        }
        _ => Err(FeedError::NotAReport),
    }
}

/// Confirms `effect` through [`MockReport`] and names that bag on the trip.
///
/// `Ok` means the trip named that bag. A stamp after the budget stores `TimeUp`
/// and returns [`FeedError::TimeUp`] without naming. Any effect other than
/// `ReportBag`, and any other rejection, leaves the trip unchanged.
pub fn confirm_with_mock(trip: &mut Trip, now_ms: u64, effect: Effect) -> Result<Id, FeedError> {
    let Effect::ReportBag { bag } = effect else {
        return Err(FeedError::NotAReport);
    };
    let mut mock = MockReport::new();
    mock.show(bag);
    let Some(confirmed) = mock.confirmed() else {
        return Err(FeedError::NotAReport);
    };
    let (next, outcome) = admit(trip, now_ms, Event::Named { bag: confirmed })?;
    if next.named()
        && next.phase() == Phase::Picking
        && outcome.effect == Some(Effect::Pick { bag: confirmed })
    {
        *trip = next;
        Ok(confirmed)
    } else {
        Err(FeedError::NotAReport)
    }
}

/// Stores a deadline that `Trip::apply` reports as `Ok` with no effect.
///
/// The payload was not applied. Other outcomes are returned for the caller to
/// commit only when the payload succeeded.
fn keep_deadline(
    trip: &mut Trip,
    next: Trip,
    outcome: Outcome,
) -> Result<(Trip, Outcome), FeedError> {
    if outcome.effect.is_none() && next.phase() == Phase::TimeUp {
        *trip = next;
        Err(FeedError::TimeUp)
    } else {
        Ok((next, outcome))
    }
}

/// Applies `event` on a copy. A deadline is stored. Any other `Ok` is not stored yet.
fn admit(trip: &mut Trip, now_ms: u64, event: Event) -> Result<(Trip, Outcome), FeedError> {
    let mut next = *trip;
    let outcome = next.apply(now_ms, event).map_err(FeedError::Trip)?;
    keep_deadline(trip, next, outcome)
}

/// Result of a pick that was admitted and executed. [`PickEnd::Matched`] is the only success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickEnd {
    Matched { bag: Id, signal: Effect },
    Mismatched { bag: Id },
    Failed { bag: Id },
}

/// Result of a handoff that was admitted. [`HandoffEnd::Released`] is the only completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandoffEnd {
    Released { bag: Id, to: Id },
    Failed { bag: Id },
}

/// Executes `command` only when it is a pick the trip would accept and the arm guard agrees.
///
/// The guard reads world truth for a bag the trip already named. It does not
/// choose the target. Selection and approach read [`navigation::sense`].
///
/// `Ok(Matched)` means the adapter holds that same bag and the trip is asking for a
/// ready signal. The ready signal has not been confirmed. A late stamp stores
/// `TimeUp` and does not pick.
pub fn execute_pick(
    world: &mut SimWorld,
    trip: &mut Trip,
    now_ms: u64,
    command: Effect,
) -> Result<PickEnd, FeedError> {
    let Effect::Pick { bag } = command else {
        return Err(FeedError::NotACommand);
    };
    let (next, outcome) = admit(trip, now_ms, Event::Grasped { bag })?;
    let applied = classify_grasp(&next, &outcome, bag);
    match executor_guard(world, Command::Pick { bag }) {
        Err(error) => Err(FeedError::Arm(error)),
        Ok(Act::GraspFailed { bag: failed }) if failed == bag => {
            let (failed_trip, failed_outcome) = admit(trip, now_ms, Event::GraspFailed { bag })?;
            let free = world
                .observe()
                .bags
                .iter()
                .find(|item| item.id == bag)
                .is_some_and(|item| item.custody == Custody::Free);
            if failed_trip.phase() != Phase::PickFailed
                || failed_trip.carried().is_some()
                || failed_outcome.effect.is_some()
                || world.carried().is_some()
                || !free
            {
                return Err(FeedError::NotACommand);
            }
            *trip = failed_trip;
            Ok(PickEnd::Failed { bag })
        }
        Ok(Act::Grasped { bag: got }) if got == bag => match applied {
            Some(GraspApply::Match { signal }) => {
                world.pick(bag, assume::REACH_MM).map_err(map_arm)?;
                debug_assert_eq!(world.carried(), Some(bag));
                *trip = next;
                Ok(PickEnd::Matched { bag, signal })
            }
            Some(GraspApply::Mismatch) => {
                world.pick(bag, assume::REACH_MM).map_err(map_arm)?;
                debug_assert_eq!(world.carried(), Some(bag));
                *trip = next;
                Ok(PickEnd::Mismatched { bag })
            }
            None => Err(FeedError::NotACommand),
        },
        Ok(_) => Err(FeedError::NotACommand),
    }
}

/// Confirms `effect` through [`MockReady`] and the adapter.
///
/// The base is not moved and arrival is not applied. A late stamp stores `TimeUp`
/// and leaves the gripper as it was.
pub fn confirm_ready(
    world: &SimWorld,
    trip: &mut Trip,
    now_ms: u64,
    effect: Effect,
) -> Result<Effect, FeedError> {
    let Effect::SignalReady { to } = effect else {
        return Err(FeedError::NotACommand);
    };
    let (next, outcome) = admit(trip, now_ms, Event::ReadyTold)?;
    let mut mock = MockReady::new();
    mock.tell(to);
    if mock.confirmed() != Some(to) {
        return Err(FeedError::NotACommand);
    }
    if world.carried().is_none() || world.carried() != trip.carried() {
        return Err(FeedError::Arm(GripError::Empty));
    }
    let Some(follow @ Effect::Follow { person }) = outcome.effect else {
        return Err(FeedError::NotACommand);
    };
    if person != to
        || !next.ready()
        || next.phase() != Phase::Following
        || next.arrived()
        || next.main_complete()
        || next.carried() != world.carried()
    {
        return Err(FeedError::NotACommand);
    }
    *trip = next;
    Ok(follow)
}

/// Drops `bag` only when the trip and the gripper both still hold it.
///
/// The bag is recorded at the robot base. A late stamp stores `TimeUp` and does not drop.
pub fn execute_drop(
    world: &mut SimWorld,
    trip: &mut Trip,
    now_ms: u64,
    bag: Id,
) -> Result<Point, FeedError> {
    let (next, outcome) = admit(trip, now_ms, Event::Dropped { bag })?;
    if next.phase() != Phase::Dropped || next.carried().is_some() || outcome.effect.is_some() {
        return Err(FeedError::NotACommand);
    }
    match executor_guard(world, Command::Drop { bag }) {
        Err(error) => Err(FeedError::Arm(error)),
        Ok(Act::Dropped { bag: got, at })
            if got == bag && at == world.observe().robot.pose.point() =>
        {
            let point = world.drop_held(bag).map_err(map_arm)?;
            debug_assert_eq!(point, at);
            debug_assert!(world.carried().is_none());
            *trip = next;
            Ok(point)
        }
        Ok(_) => Err(FeedError::NotACommand),
    }
}

/// Hands the carried bag to `to`.
///
/// Arrival is not applied here. A late stamp stores `TimeUp` and does not transfer the bag.
/// A world disk on the transfer segment refuses the handoff and leaves the trip and the
/// world unchanged. That refusal is the simulator's geometry, not a sight the navigator receives.
/// `main_complete` after a clear component handoff is the trip flag, not a car arrival and not a score.
pub fn execute_handoff(
    world: &mut SimWorld,
    trip: &mut Trip,
    now_ms: u64,
    to: Id,
) -> Result<HandoffEnd, FeedError> {
    let Some(bag) = trip.carried() else {
        let probe = world
            .observe()
            .bags
            .first()
            .map(|item| item.id)
            .unwrap_or(to);
        admit(trip, now_ms, Event::Received { bag: probe, by: to })?;
        return Err(FeedError::Arm(GripError::Empty));
    };
    let (next, outcome) = admit(trip, now_ms, Event::Received { bag, by: to })?;
    if next.phase() != Phase::Complete
        || !next.main_complete()
        || next.carried().is_some()
        || !next.arrived()
        || outcome.effect.is_some()
    {
        return Err(FeedError::NotACommand);
    }
    match world.hand_to(bag, to, navigation::assume::ROBOT_RADIUS_MM) {
        Ok(()) => {
            debug_assert!(world.carried().is_none());
            *trip = next;
            Ok(HandoffEnd::Released { bag, to })
        }
        Err(sim_runtime::ArmError::HandoffFailed) => failed_handoff(world, trip, now_ms, bag, to),
        Err(error) => Err(map_arm(error)),
    }
}

enum GraspApply {
    Match { signal: Effect },
    Mismatch,
}

fn classify_grasp(next: &Trip, outcome: &Outcome, bag: Id) -> Option<GraspApply> {
    if next.carried() != Some(bag) {
        return None;
    }
    match (next.phase(), outcome.effect) {
        (Phase::Signaling, Some(signal @ Effect::SignalReady { .. })) if !next.ready() => {
            Some(GraspApply::Match { signal })
        }
        (Phase::WrongBag, None) => Some(GraspApply::Mismatch),
        _ => None,
    }
}

fn failed_handoff(
    world: &SimWorld,
    trip: &mut Trip,
    now_ms: u64,
    bag: Id,
    to: Id,
) -> Result<HandoffEnd, FeedError> {
    match executor_guard(world, Command::Release { bag, to }) {
        Ok(Act::HandoffFailed { bag: failed }) if failed == bag => {}
        Ok(_) => return Err(FeedError::NotACommand),
        Err(error) => return Err(FeedError::Arm(error)),
    }
    let (failed_trip, failed_outcome) = admit(trip, now_ms, Event::HandoffFailed)?;
    if failed_trip.phase() != Phase::HandoffFailed
        || failed_trip.carried() != Some(bag)
        || failed_trip.main_complete()
        || failed_outcome.effect.is_some()
        || world.carried() != Some(bag)
    {
        return Err(FeedError::NotACommand);
    }
    *trip = failed_trip;
    Ok(HandoffEnd::Failed { bag })
}

/// World-truth check for a command the trip already named.
///
/// Reach, visibility, and custody here are the physical refusal. The function
/// is not a planner: [`navigation::sense`] is what follow, approach, and
/// selection read. An unseen bag is not chosen by this guard.
fn executor_guard(world: &SimWorld, command: Command) -> Result<Act, GripError> {
    let observed = world.observe();
    let scene = Scene {
        robot: observed.robot.pose.point(),
        operator: observed.operator.id,
        carried: world.carried(),
        bags: &observed.bags,
        grasp_fails: world.grasp_fault(),
        handoff_fails: world.handoff_fault(),
    };
    decide(&scene, command)
}

fn map_arm(error: ArmError) -> FeedError {
    FeedError::Arm(match error {
        ArmError::Missing => GripError::Missing,
        ArmError::Invisible => GripError::Invisible,
        ArmError::OutOfReach => GripError::OutOfReach,
        ArmError::Occupied => GripError::Occupied,
        ArmError::Empty => GripError::Empty,
        ArmError::WrongId => GripError::WrongId,
        ArmError::Role => GripError::Role,
        ArmError::NotFree => GripError::NotFree,
        ArmError::WrongRecipient => GripError::WrongRecipient,
        ArmError::Duplicate => GripError::Duplicate,
        ArmError::FailedGrasp => GripError::FailedGrasp,
        ArmError::HandoffFailed => GripError::HandoffFailed,
        ArmError::Blocked => GripError::Blocked,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        FeedError, HandoffEnd, PickEnd, confirm_ready, confirm_with_mock, execute_drop,
        execute_handoff, execute_pick, point_from_sample, stage_report,
    };
    use domain::{
        Bag, BagKind, Custody, Event as Trace, Id, Mm, Observation, Point, Pose, Step, World,
    };
    use manipulation::GripError;
    use perception::SelectError;
    use sim_runtime::Truth;
    use sim_runtime::{SimWorld, direction_mm};
    use task_3_1::{Effect, Event, LIMIT_MS, Phase, Reject, Trip};

    fn length_ok(dx: i32, dy: i32) -> bool {
        let dx = i64::from(dx);
        let dy = i64::from(dy);
        let squared = dx * dx + dy * dy;
        (400 * 400..=2_000 * 2_000).contains(&squared)
    }

    fn begin() -> Trip {
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip
    }

    #[test]
    fn each_seeded_bag_is_reported_and_not_grasped() {
        let mut seeds: Vec<u64> = (0..32).collect();
        seeds.push(u64::MAX);
        for seed in seeds {
            let world = SimWorld::from_seed(seed);
            let obs = world.observe();
            assert!(obs.bags.len() >= 2, "seed {seed}");
            for bag in &obs.bags {
                let sample = world.aim_at(bag.id).unwrap();
                assert!(
                    length_ok(sample.dx(), sample.dy()),
                    "seed {seed} bag {}",
                    bag.id.get()
                );
                assert_eq!(sample.origin(), obs.operator.pose.point());
                let delta = direction_mm(obs.operator.pose.point(), bag.pose.point()).unwrap();
                assert_eq!((sample.dx(), sample.dy()), delta);
                let chosen = point_from_sample(
                    Some(sample.origin()),
                    Some((sample.dx(), sample.dy())),
                    &obs.bags,
                )
                .unwrap();
                assert_eq!(chosen, bag.id, "seed {seed}");

                let mut trip = begin();
                let effect = stage_report(&mut trip, 1, obs.operator.id, chosen).unwrap();
                assert_eq!(effect, Effect::ReportBag { bag: chosen });
                assert!(!trip.named());
                assert_eq!(trip.phase(), Phase::Naming);
                assert!(trip.carried().is_none());

                let named = confirm_with_mock(&mut trip, 2, effect).unwrap();
                assert_eq!(named, chosen);
                assert!(trip.named());
                assert_eq!(trip.phase(), Phase::Picking);
                assert!(trip.carried().is_none());
                assert!(!trip.ready());
                assert!(!trip.arrived());
                assert!(!trip.main_complete());
            }
        }
    }

    #[test]
    fn pointing_at_the_other_bag_is_not_corrected_to_the_referee_bag() {
        let world = SimWorld::from_seed(1);
        let obs = world.observe();
        let referee = obs.bags[0].id;
        let pointed = obs.bags[1].id;
        assert_ne!(referee, pointed);
        let sample = world.aim_at(pointed).unwrap();
        let chosen = point_from_sample(
            Some(sample.origin()),
            Some((sample.dx(), sample.dy())),
            &obs.bags,
        )
        .unwrap();
        assert_eq!(chosen, pointed);
        assert_ne!(chosen, referee);

        let mut trip = begin();
        let effect = stage_report(&mut trip, 10, obs.operator.id, chosen).unwrap();
        assert_eq!(effect, Effect::ReportBag { bag: pointed });
        assert!(!trip.named());
        let named = confirm_with_mock(&mut trip, 11, effect).unwrap();
        assert_eq!(named, pointed);
        assert_ne!(named, referee);
        assert_eq!(trip.phase(), Phase::Picking);
        assert!(trip.carried().is_none());
        assert!(!trip.ready());
        assert!(!trip.main_complete());
    }

    #[test]
    fn reversed_bag_order_still_reports_the_aimed_bag() {
        let world = SimWorld::from_seed(2);
        let obs = world.observe();
        let aimed = obs.bags[0].id;
        let sample = world.aim_at(aimed).unwrap();
        let mut reversed = obs.bags.clone();
        reversed.reverse();
        assert_ne!(reversed[0].id, aimed);
        let chosen = point_from_sample(
            Some(sample.origin()),
            Some((sample.dx(), sample.dy())),
            &reversed,
        )
        .unwrap();
        assert_eq!(chosen, aimed);
    }

    #[test]
    fn a_hidden_aimed_bag_does_not_become_the_report() {
        let world = SimWorld::from_seed(4);
        let obs = world.observe();
        let mut bags = obs.bags.clone();
        let aimed = bags[0].id;
        bags[0].visible = false;
        let sample = world.aim_at(aimed).unwrap();
        let trip = begin();
        let before = trip;
        assert_eq!(
            point_from_sample(
                Some(sample.origin()),
                Some((sample.dx(), sample.dy())),
                &bags
            ),
            Err(SelectError::NoHit)
        );
        assert_eq!(trip, before);
        assert!(!trip.named());
        assert!(trip.carried().is_none());
    }

    #[test]
    fn invalid_selection_and_non_reports_leave_the_trip_unchanged() {
        let world = SimWorld::from_seed(5);
        let obs = world.observe();
        let mut trip = begin();
        let before = trip;
        assert_eq!(
            point_from_sample(Some(obs.operator.pose.point()), Some((0, 0)), &obs.bags),
            Err(SelectError::NoDirection)
        );
        assert_eq!(trip, before);

        assert_eq!(
            stage_report(&mut trip, 1, obs.operator.id, obs.operator.id),
            Err(FeedError::Trip(Reject::Role))
        );
        assert_eq!(trip, before);

        let idle = Trip::new();
        let mut still = idle;
        assert_eq!(
            stage_report(&mut still, 0, obs.operator.id, obs.bags[0].id),
            Err(FeedError::Trip(Reject::OutOfOrder))
        );
        assert_eq!(still, idle);

        let bag = obs.bags[0].id;
        let operator = obs.operator.id;
        let blocked = [
            Effect::Pick { bag },
            Effect::SignalReady { to: operator },
            Effect::Follow { person: operator },
            Effect::Release { bag, to: operator },
        ];
        for effect in blocked {
            let mut naming = begin();
            stage_report(&mut naming, 1, operator, bag).unwrap();
            let pending = naming;
            assert!(!pending.named());
            assert_eq!(
                confirm_with_mock(&mut naming, 2, effect),
                Err(FeedError::NotAReport)
            );
            assert_eq!(naming, pending);
            assert!(!naming.named());
            assert!(naming.carried().is_none());
            assert!(!naming.ready());
        }
    }

    fn began_at(now: u64) -> Trip {
        let mut trip = Trip::new();
        trip.apply(now, Event::Begin).unwrap();
        trip
    }

    fn ids() -> (Id, Id) {
        let obs = SimWorld::from_seed(1).observe();
        let operator = obs.operator.id;
        let bag = obs.bags[0].id;
        assert_eq!(operator.get(), 2);
        assert_eq!(bag.get(), 3);
        (operator, bag)
    }

    #[test]
    fn the_limit_millisecond_posts_and_names_the_bag() {
        let (operator, bag) = ids();
        let mut posted = began_at(0);
        let effect = stage_report(&mut posted, LIMIT_MS, operator, bag).unwrap();
        assert_eq!(effect, Effect::ReportBag { bag });
        assert_eq!(posted.phase(), Phase::Naming);
        assert_eq!(posted.now_ms(), Some(LIMIT_MS));
        assert_eq!(posted.start_ms(), Some(0));
        assert!(!posted.named());
        assert!(posted.carried().is_none());
        assert!(!posted.ready());
        assert!(!posted.arrived());
        assert!(!posted.main_complete());

        let mut named = began_at(0);
        let effect = stage_report(&mut named, 1, operator, bag).unwrap();
        let confirmed = confirm_with_mock(&mut named, LIMIT_MS, effect).unwrap();
        assert_eq!(confirmed, bag);
        assert_eq!(named.phase(), Phase::Picking);
        assert_eq!(named.now_ms(), Some(LIMIT_MS));
        assert!(named.named());
        assert!(named.carried().is_none());
        assert!(!named.ready());
        assert!(!named.arrived());
        assert!(!named.main_complete());
    }

    #[test]
    fn a_late_confirmation_does_not_name_and_keeps_time_up() {
        let (operator, bag) = ids();
        let mut trip = began_at(0);
        let effect = stage_report(&mut trip, 1, operator, bag).unwrap();
        assert_eq!(effect, Effect::ReportBag { bag });
        let mut direct = trip;
        let direct_outcome = direct.apply(LIMIT_MS + 1, Event::Named { bag }).unwrap();
        assert_eq!(direct_outcome.effect, None);
        let result = confirm_with_mock(&mut trip, LIMIT_MS + 1, effect);
        assert_eq!(result, Err(FeedError::TimeUp));
        assert_eq!(trip, direct);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(trip.now_ms(), Some(LIMIT_MS + 1));
        assert_eq!(trip.start_ms(), Some(0));
        assert!(!trip.named());
        assert!(trip.carried().is_none());
        assert!(!trip.ready());
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        let expired = trip;
        assert_eq!(
            confirm_with_mock(&mut trip, LIMIT_MS + 2, effect),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!(trip, expired);
        assert_eq!(
            confirm_with_mock(&mut trip, 2, effect),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!(trip, expired);
    }

    #[test]
    fn a_late_report_keeps_time_up_and_an_earlier_stamp_cannot_repost() {
        let (operator, bag) = ids();
        let start = 10;
        let mut trip = began_at(start);
        let late = start + LIMIT_MS + 1;
        let mut direct = trip;
        let direct_outcome = direct
            .apply(late, Event::Pointed { operator, bag })
            .unwrap();
        assert_eq!(direct_outcome.effect, None);
        assert_eq!(
            stage_report(&mut trip, late, operator, bag),
            Err(FeedError::TimeUp)
        );
        assert_eq!(trip, direct);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(trip.now_ms(), Some(late));
        assert_eq!(trip.start_ms(), Some(start));
        assert!(!trip.named());
        assert!(trip.carried().is_none());
        assert!(!trip.ready());
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        let expired = trip;
        assert_eq!(
            stage_report(&mut trip, 60, operator, bag),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!(trip, expired);
        assert_ne!(
            stage_report(&mut trip, 60, operator, bag),
            Ok(Effect::ReportBag { bag })
        );
        assert_eq!(trip.phase(), Phase::TimeUp);
    }

    #[test]
    fn a_start_near_u64_max_names_inside_the_budget_and_latches_just_outside_it() {
        let (operator, bag) = ids();
        let start = u64::MAX - 10;
        let mut inside = began_at(start);
        let effect = stage_report(&mut inside, u64::MAX, operator, bag).unwrap();
        assert_eq!(effect, Effect::ReportBag { bag });
        let named = confirm_with_mock(&mut inside, u64::MAX, effect).unwrap();
        assert_eq!(named, bag);
        assert_eq!(inside.phase(), Phase::Picking);
        assert_eq!(inside.now_ms(), Some(u64::MAX));
        assert!(inside.named());
        assert!(inside.carried().is_none());
        assert!(!inside.ready());
        assert!(!inside.main_complete());

        let start = u64::MAX - LIMIT_MS - 1;
        let mut outside = began_at(start);
        let effect = stage_report(&mut outside, start + 1, operator, bag).unwrap();
        assert_eq!(effect, Effect::ReportBag { bag });
        assert!(!outside.named());
        let mut direct = outside;
        assert_eq!(
            direct.apply(u64::MAX, Event::Named { bag }).unwrap().effect,
            None
        );
        assert_eq!(
            confirm_with_mock(&mut outside, u64::MAX, effect),
            Err(FeedError::TimeUp)
        );
        assert_eq!(outside, direct);
        assert_eq!(outside.phase(), Phase::TimeUp);
        assert_eq!(outside.now_ms(), Some(u64::MAX));
        assert_eq!(outside.start_ms(), Some(start));
        assert!(!outside.named());
        assert!(outside.carried().is_none());
        let expired = outside;
        assert_eq!(
            confirm_with_mock(&mut outside, u64::MAX, effect),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!(outside, expired);
    }

    #[test]
    fn a_regressing_confirmation_does_not_name_the_bag() {
        let world = SimWorld::from_seed(6);
        let obs = world.observe();
        let bag = obs.bags[1].id;
        let mut trip = begin();
        let effect = stage_report(&mut trip, 20, obs.operator.id, bag).unwrap();
        let pending = trip;
        assert_eq!(effect, Effect::ReportBag { bag });
        assert!(!pending.named());
        assert_eq!(
            confirm_with_mock(&mut trip, 19, effect),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!(trip, pending);
        assert_eq!(trip.phase(), Phase::Naming);
        assert!(!trip.named());
        assert!(trip.carried().is_none());
    }

    #[test]
    fn the_same_seed_replays_the_same_report() {
        fn once(seed: u64) -> (Id, Trip) {
            let world = SimWorld::from_seed(seed);
            let obs = world.observe();
            let bag = &obs.bags[1];
            let sample = world.aim_at(bag.id).unwrap();
            let chosen = point_from_sample(
                Some(sample.origin()),
                Some((sample.dx(), sample.dy())),
                &obs.bags,
            )
            .unwrap();
            let mut trip = begin();
            let effect = stage_report(&mut trip, 3, obs.operator.id, chosen).unwrap();
            confirm_with_mock(&mut trip, 4, effect).unwrap();
            (chosen, trip)
        }
        assert_eq!(once(11), once(11));
        let (chosen, trip) = once(11);
        assert_eq!(chosen, SimWorld::from_seed(11).observe().bags[1].id);
        assert!(trip.named());
        assert_eq!(trip.phase(), Phase::Picking);
        assert!(trip.carried().is_none());
        assert!(!trip.main_complete());
    }

    #[test]
    fn a_hand_built_duplicate_is_rejected_before_the_trip_moves() {
        let origin = Point::new(Mm::new(0), Mm::new(0));
        let at = Pose::new(Point::new(Mm::new(1000), Mm::new(0)));
        let bag = Bag {
            id: Id::new(3).unwrap(),
            pose: at,
            kind: BagKind::LightweightStiffVerticalHandles,
            visible: true,
            custody: domain::Custody::Free,
        };
        let trip = begin();
        let before = trip;
        assert_eq!(
            point_from_sample(Some(origin), Some((1000, 0)), &[bag.clone(), bag]),
            Err(SelectError::DuplicateId)
        );
        assert_eq!(trip, before);
        assert_eq!(trip.phase(), Phase::AwaitPoint);
    }

    fn mark(world: &SimWorld) -> (Observation, Vec<Trace>, Truth, Option<Id>) {
        (
            world.observe(),
            world.events().to_vec(),
            world.truth(),
            world.carried(),
        )
    }

    fn named(start: u64, index: usize) -> (SimWorld, Trip, Id, Id) {
        let world = SimWorld::from_seed(1);
        let observed = world.observe();
        let bag = observed.bags[index].id;
        let operator = observed.operator.id;
        let mut trip = Trip::new();
        trip.apply(start, Event::Begin).unwrap();
        let effect = stage_report(&mut trip, start, operator, bag).unwrap();
        confirm_with_mock(&mut trip, start, effect).unwrap();
        assert_eq!(trip.phase(), Phase::Picking);
        assert_eq!(world.carried(), None);
        assert!(!trip.ready());
        (world, trip, bag, operator)
    }

    fn held_copy(world: &SimWorld, bag: Id) -> domain::Bag {
        world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id == bag)
            .unwrap()
    }

    #[test]
    fn seeded_pickup_holds_only_the_selected_bag() {
        let mut seeds: Vec<u64> = vec![0, 1, 2, 31, u64::MAX];
        for seed in seeds.drain(..) {
            let world = SimWorld::from_seed(seed);
            let observed = world.observe();
            for (index, bag) in observed.bags.iter().enumerate() {
                let sample = world.aim_at(bag.id).unwrap();
                let chosen = point_from_sample(
                    Some(sample.origin()),
                    Some((sample.dx(), sample.dy())),
                    &observed.bags,
                )
                .unwrap();
                assert_eq!(chosen, bag.id, "seed {seed}");
                let mut trip = Trip::new();
                trip.apply(5, Event::Begin).unwrap();
                let report = stage_report(&mut trip, 5, observed.operator.id, chosen).unwrap();
                let command_bag = confirm_with_mock(&mut trip, 5, report).unwrap();
                assert_eq!(command_bag, chosen);
                assert_eq!(trip.phase(), Phase::Picking);
                assert_eq!(world.carried(), None);
                let old = bag.pose.point();
                let other = observed.bags[1 - index].clone();
                let mut world = SimWorld::from_seed(seed);
                let end =
                    execute_pick(&mut world, &mut trip, 6, Effect::Pick { bag: chosen }).unwrap();
                assert_eq!(
                    end,
                    PickEnd::Matched {
                        bag: chosen,
                        signal: Effect::SignalReady {
                            to: observed.operator.id
                        }
                    }
                );
                assert_eq!(trip.phase(), Phase::Signaling);
                assert_eq!(trip.carried(), Some(chosen));
                assert!(!trip.ready());
                assert!(!trip.arrived());
                assert!(!trip.main_complete());
                assert_eq!(world.carried(), Some(chosen));
                let held = held_copy(&world, chosen);
                assert_eq!(held.custody, Custody::Gripper);
                assert_eq!(held.pose, observed.robot.pose);
                assert_ne!(held.pose.point(), old);
                assert_eq!(
                    world
                        .observe()
                        .bags
                        .iter()
                        .filter(|item| item.id == chosen)
                        .count(),
                    1
                );
                assert_eq!(held_copy(&world, other.id), other);
                assert_eq!(
                    world.observe().robot.pose.point(),
                    Point::new(Mm::new(0), Mm::new(0))
                );
                assert_eq!(world.revealed_car(), None);
            }
        }
    }

    #[test]
    fn the_other_bag_is_not_a_matched_pick() {
        let (mut world, mut trip, pointed, operator) = named(3, 0);
        let other = world.observe().bags[1].id;
        assert_ne!(other, pointed);
        let old = held_copy(&world, pointed).pose.point();
        let end = execute_pick(&mut world, &mut trip, 4, Effect::Pick { bag: other }).unwrap();
        assert_eq!(end, PickEnd::Mismatched { bag: other });
        assert_ne!(
            end,
            PickEnd::Matched {
                bag: pointed,
                signal: Effect::SignalReady { to: operator }
            }
        );
        assert_eq!(trip.phase(), Phase::WrongBag);
        assert_eq!(trip.carried(), Some(other));
        assert_eq!(world.carried(), Some(other));
        assert!(!trip.ready());
        assert!(!trip.main_complete());
        assert_eq!(held_copy(&world, pointed).custody, Custody::Free);
        assert_eq!(held_copy(&world, pointed).pose.point(), old);
        assert_eq!(held_copy(&world, other).custody, Custody::Gripper);
    }

    #[test]
    fn hidden_far_missing_and_role_ids_leave_the_world_and_the_trip() {
        let (mut world, mut trip, bag, _) = named(8, 0);
        world.set_visible(bag, false).unwrap();
        let hidden = (mark(&world), trip);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 9, Effect::Pick { bag }),
            Err(FeedError::Arm(GripError::Invisible))
        );
        assert_eq!((mark(&world), trip), hidden);

        let (mut world, mut trip, bag, _) = named(8, 1);
        world
            .place_free(bag, Point::new(Mm::new(2501), Mm::new(0)))
            .unwrap();
        let far = (mark(&world), trip);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 9, Effect::Pick { bag }),
            Err(FeedError::Arm(GripError::OutOfReach))
        );
        assert_eq!((mark(&world), trip), far);
        assert_eq!(held_copy(&world, bag).custody, Custody::Free);

        let (mut world, mut trip, _, _) = named(8, 0);
        let before = (mark(&world), trip);
        let missing = Id::new(99).unwrap();
        assert_eq!(
            execute_pick(&mut world, &mut trip, 9, Effect::Pick { bag: missing }),
            Err(FeedError::Arm(GripError::Missing))
        );
        assert_eq!((mark(&world), trip), before);
        assert_ne!(world.carried(), Some(missing));
        assert_ne!(trip.carried(), Some(missing));

        let (mut world, mut trip, _, operator) = named(8, 0);
        let before = (mark(&world), trip);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 9, Effect::Pick { bag: operator }),
            Err(FeedError::Trip(Reject::Role))
        );
        assert_eq!((mark(&world), trip), before);
    }

    #[test]
    fn repeated_actions_and_an_injected_failure_conserve_the_bag() {
        let (mut world, mut trip, bag, operator) = named(12, 0);
        let before = (mark(&world), trip);
        assert_eq!(
            execute_drop(&mut world, &mut trip, 13, bag),
            Err(FeedError::Trip(Reject::Impossible))
        );
        assert_eq!((mark(&world), trip), before);

        let end = execute_pick(&mut world, &mut trip, 13, Effect::Pick { bag }).unwrap();
        assert!(matches!(end, PickEnd::Matched { bag: held, .. } if held == bag));
        let occupied = (mark(&world), trip);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 14, Effect::Pick { bag }),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!((mark(&world), trip), occupied);
        assert_eq!(
            execute_drop(&mut world, &mut trip, 14, operator),
            Err(FeedError::Trip(Reject::Role))
        );
        assert_eq!((mark(&world), trip), occupied);
        let other = world
            .observe()
            .bags
            .iter()
            .find(|item| item.id != bag)
            .unwrap()
            .id;
        assert_eq!(
            execute_drop(&mut world, &mut trip, 14, other),
            Err(FeedError::Trip(Reject::WrongId))
        );
        assert_eq!((mark(&world), trip), occupied);

        let at = execute_drop(&mut world, &mut trip, 15, bag).unwrap();
        assert_eq!(at, Point::new(Mm::new(0), Mm::new(0)));
        assert_eq!(trip.phase(), Phase::Dropped);
        assert_eq!(trip.carried(), None);
        assert_eq!(world.carried(), None);
        assert!(!trip.main_complete());
        let dropped = held_copy(&world, bag);
        assert_eq!(dropped.custody, Custody::Free);
        assert_eq!(dropped.pose.point(), at);
        let after = (mark(&world), trip);
        assert_eq!(
            execute_drop(&mut world, &mut trip, 16, bag),
            Err(FeedError::Trip(Reject::Impossible))
        );
        assert_eq!((mark(&world), trip), after);

        let (mut world, mut trip, bag, _) = named(12, 1);
        let old = held_copy(&world, bag).pose.point();
        world.set_grasp_fault(true);
        let before = (mark(&world), trip);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 13, Effect::Pick { bag }),
            Ok(PickEnd::Failed { bag })
        );
        assert_eq!(mark(&world), before.0);
        assert_eq!(trip.phase(), Phase::PickFailed);
        assert_eq!(trip.carried(), None);
        assert_eq!(held_copy(&world, bag).pose.point(), old);
        assert_eq!(held_copy(&world, bag).custody, Custody::Free);
        let failed = trip;
        assert_eq!(
            execute_pick(&mut world, &mut trip, 14, Effect::Pick { bag }),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!(trip, failed);
        assert_eq!(mark(&world), before.0);
    }

    #[test]
    fn ready_is_explicit_and_does_not_follow_or_move() {
        let (mut world, mut trip, bag, operator) = named(20, 0);
        let pending = trip;
        assert_eq!(
            confirm_ready(&world, &mut trip, 21, Effect::SignalReady { to: operator }),
            Err(FeedError::Trip(Reject::OutOfOrder))
        );
        assert_eq!(trip, pending);
        execute_pick(&mut world, &mut trip, 21, Effect::Pick { bag }).unwrap();
        assert!(!trip.ready());
        assert_eq!(trip.phase(), Phase::Signaling);
        let held = (mark(&world), trip);
        for effect in [
            Effect::Pick { bag },
            Effect::Follow { person: operator },
            Effect::Release { bag, to: operator },
            Effect::ReportBag { bag },
        ] {
            assert_eq!(
                confirm_ready(&world, &mut trip, 22, effect),
                Err(FeedError::NotACommand)
            );
            assert_eq!((mark(&world), trip), held);
        }
        let pose = world.observe().robot.pose;
        let follow =
            confirm_ready(&world, &mut trip, 22, Effect::SignalReady { to: operator }).unwrap();
        assert_eq!(follow, Effect::Follow { person: operator });
        assert!(trip.ready());
        assert_eq!(trip.phase(), Phase::Following);
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(trip.carried(), Some(bag));
        assert_eq!(world.carried(), Some(bag));
        assert_eq!(world.observe().robot.pose, pose);
        assert_eq!(world.revealed_car(), None);
    }

    #[test]
    fn a_mock_arrival_is_not_navigation_and_handoff_uses_it() {
        let (mut world, mut trip, bag, operator) = named(30, 0);
        execute_pick(&mut world, &mut trip, 31, Effect::Pick { bag }).unwrap();
        confirm_ready(&world, &mut trip, 32, Effect::SignalReady { to: operator }).unwrap();
        let following = (mark(&world), trip);
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 33, operator),
            Err(FeedError::Trip(Reject::OutOfOrder))
        );
        assert_eq!((mark(&world), trip), following);
        assert_eq!(
            world.observe().robot.pose.point(),
            Point::new(Mm::new(0), Mm::new(0))
        );
        // Fixture event only. This is not a navigation result and does not move the base.
        trip.apply(33, Event::Arrived).unwrap();
        assert!(trip.arrived());
        assert_eq!(trip.phase(), Phase::Releasing);
        assert!(!trip.main_complete());
        assert_eq!(
            world.observe().robot.pose.point(),
            Point::new(Mm::new(0), Mm::new(0))
        );
        assert_eq!(world.revealed_car(), None);
        let releasing = (mark(&world), trip);
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 34, Id::new(8).unwrap()),
            Err(FeedError::Trip(Reject::WrongId))
        );
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 34, bag),
            Err(FeedError::Trip(Reject::Role))
        );
        assert_eq!((mark(&world), trip), releasing);
        world.set_handoff_fault(true);
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 34, operator),
            Ok(HandoffEnd::Failed { bag })
        );
        assert_eq!(trip.phase(), Phase::HandoffFailed);
        assert_eq!(trip.carried(), Some(bag));
        assert_eq!(world.carried(), Some(bag));
        assert!(!trip.main_complete());
        assert_eq!(held_copy(&world, bag).custody, Custody::Gripper);
        world.set_handoff_fault(false);
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 35, operator),
            Ok(HandoffEnd::Released { bag, to: operator })
        );
        assert_eq!(trip.phase(), Phase::Complete);
        assert!(trip.main_complete());
        assert_eq!(trip.carried(), None);
        assert_eq!(world.carried(), None);
        assert_eq!(held_copy(&world, bag).custody, Custody::Operator);
        assert_eq!(held_copy(&world, bag).pose, world.observe().operator.pose);
        assert_eq!(
            world
                .observe()
                .bags
                .iter()
                .filter(|item| item.id == bag)
                .count(),
            1
        );
        assert_eq!(
            world.observe().robot.pose.point(),
            Point::new(Mm::new(0), Mm::new(0))
        );
        let done = (mark(&world), trip);
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 36, operator),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!((mark(&world), trip), done);
    }

    #[test]
    fn deadlines_do_not_execute_a_physical_action() {
        let start = 0;
        let (mut world, mut trip, bag, operator) = named(start, 0);
        let end = execute_pick(
            &mut world,
            &mut trip,
            start + LIMIT_MS,
            Effect::Pick { bag },
        )
        .unwrap();
        assert!(matches!(end, PickEnd::Matched { bag: held, .. } if held == bag));
        assert_eq!(trip.now_ms(), Some(start + LIMIT_MS));
        assert_eq!(world.carried(), Some(bag));

        let (mut world, mut trip, bag, _) = named(start, 0);
        let before = mark(&world);
        assert_eq!(
            execute_pick(&mut world, &mut trip, LIMIT_MS + 1, Effect::Pick { bag }),
            Err(FeedError::TimeUp)
        );
        assert_eq!(mark(&world), before);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(trip.now_ms(), Some(LIMIT_MS + 1));
        assert_eq!(trip.carried(), None);
        assert!(!trip.ready());
        assert!(!trip.main_complete());
        let expired = trip;
        assert_eq!(
            execute_pick(&mut world, &mut trip, LIMIT_MS + 1, Effect::Pick { bag }),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!(trip, expired);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 2, Effect::Pick { bag }),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!(trip, expired);
        assert_eq!(mark(&world), before);
        assert_ne!(
            execute_pick(&mut world, &mut trip, 2, Effect::Pick { bag }),
            Ok(PickEnd::Matched {
                bag,
                signal: Effect::SignalReady { to: operator }
            })
        );

        let (mut late, mut late_trip, late_bag, _) = named(40, 0);
        let pending = (mark(&late), late_trip);
        assert_eq!(
            execute_pick(
                &mut late,
                &mut late_trip,
                39,
                Effect::Pick { bag: late_bag }
            ),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!((mark(&late), late_trip), pending);

        let (mut same, mut same_trip, same_bag, same_operator) = named(15, 1);
        assert_eq!(
            execute_pick(
                &mut same,
                &mut same_trip,
                15,
                Effect::Pick { bag: same_bag }
            )
            .unwrap(),
            PickEnd::Matched {
                bag: same_bag,
                signal: Effect::SignalReady { to: same_operator }
            }
        );

        let start = u64::MAX - 10;
        let (mut near, mut near_trip, near_bag, near_operator) = named(start, 0);
        assert_eq!(
            execute_pick(
                &mut near,
                &mut near_trip,
                u64::MAX,
                Effect::Pick { bag: near_bag }
            )
            .unwrap(),
            PickEnd::Matched {
                bag: near_bag,
                signal: Effect::SignalReady { to: near_operator }
            }
        );
        assert_eq!(near_trip.now_ms(), Some(u64::MAX));
        assert_eq!(near.carried(), Some(near_bag));

        let start = u64::MAX - LIMIT_MS - 1;
        let late = start.checked_add(LIMIT_MS).unwrap().checked_add(1).unwrap();
        assert_eq!(late, u64::MAX);
        let (mut outside, mut outside_trip, outside_bag, _) = named(start, 0);
        let before = mark(&outside);
        assert_eq!(
            execute_pick(
                &mut outside,
                &mut outside_trip,
                late,
                Effect::Pick { bag: outside_bag }
            ),
            Err(FeedError::TimeUp)
        );
        assert_eq!(mark(&outside), before);
        assert_eq!(outside_trip.phase(), Phase::TimeUp);
        assert_eq!(outside.carried(), None);

        let (mut world, mut trip, bag, operator) = named(0, 0);
        execute_pick(&mut world, &mut trip, 1, Effect::Pick { bag }).unwrap();
        let follow = confirm_ready(
            &world,
            &mut trip,
            LIMIT_MS,
            Effect::SignalReady { to: operator },
        )
        .unwrap();
        assert_eq!(follow, Effect::Follow { person: operator });
        assert!(trip.ready());
        assert!(!trip.arrived());
        assert_eq!(
            world.observe().robot.pose.point(),
            Point::new(Mm::new(0), Mm::new(0))
        );

        let (mut world, mut trip, bag, operator) = named(0, 0);
        execute_pick(&mut world, &mut trip, 1, Effect::Pick { bag }).unwrap();
        let before = mark(&world);
        assert_eq!(
            confirm_ready(
                &world,
                &mut trip,
                LIMIT_MS + 1,
                Effect::SignalReady { to: operator }
            ),
            Err(FeedError::TimeUp)
        );
        assert_eq!(mark(&world), before);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert!(!trip.ready());
        assert_eq!(trip.carried(), Some(bag));
        assert_eq!(world.carried(), Some(bag));
        let expired = trip;
        assert_eq!(
            confirm_ready(&world, &mut trip, 2, Effect::SignalReady { to: operator }),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!(trip, expired);

        let (mut world, mut trip, bag, _) = named(0, 0);
        execute_pick(&mut world, &mut trip, 1, Effect::Pick { bag }).unwrap();
        let at = execute_drop(&mut world, &mut trip, LIMIT_MS, bag).unwrap();
        assert_eq!(at, Point::new(Mm::new(0), Mm::new(0)));
        assert_eq!(world.carried(), None);
        assert_eq!(held_copy(&world, bag).custody, Custody::Free);

        let (mut world, mut trip, bag, _) = named(0, 0);
        execute_pick(&mut world, &mut trip, 1, Effect::Pick { bag }).unwrap();
        let before = mark(&world);
        assert_eq!(
            execute_drop(&mut world, &mut trip, LIMIT_MS + 1, bag),
            Err(FeedError::TimeUp)
        );
        assert_eq!(mark(&world), before);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(world.carried(), Some(bag));
        assert_eq!(held_copy(&world, bag).custody, Custody::Gripper);
        let expired = trip;
        assert_eq!(
            execute_drop(&mut world, &mut trip, 4, bag),
            Err(FeedError::Trip(Reject::Regressed))
        );
        assert_eq!(trip, expired);
        assert_eq!(mark(&world), before);

        let (mut world, mut trip, bag, operator) = named(0, 0);
        execute_pick(&mut world, &mut trip, 1, Effect::Pick { bag }).unwrap();
        confirm_ready(&world, &mut trip, 2, Effect::SignalReady { to: operator }).unwrap();
        trip.apply(3, Event::Arrived).unwrap();
        assert_eq!(
            execute_handoff(&mut world, &mut trip, LIMIT_MS, operator).unwrap(),
            HandoffEnd::Released { bag, to: operator }
        );
        assert!(trip.main_complete());
        assert_eq!(held_copy(&world, bag).custody, Custody::Operator);
        assert_eq!(
            world.observe().robot.pose.point(),
            Point::new(Mm::new(0), Mm::new(0))
        );

        let (mut world, mut trip, bag, operator) = named(0, 0);
        execute_pick(&mut world, &mut trip, 1, Effect::Pick { bag }).unwrap();
        confirm_ready(&world, &mut trip, 2, Effect::SignalReady { to: operator }).unwrap();
        trip.apply(3, Event::Arrived).unwrap();
        let before = mark(&world);
        assert_eq!(
            execute_handoff(&mut world, &mut trip, LIMIT_MS + 1, operator),
            Err(FeedError::TimeUp)
        );
        assert_eq!(mark(&world), before);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert!(!trip.main_complete());
        assert_eq!(world.carried(), Some(bag));
        assert_eq!(held_copy(&world, bag).custody, Custody::Gripper);
        let expired = trip;
        assert_eq!(
            execute_handoff(&mut world, &mut trip, LIMIT_MS + 1, operator),
            Err(FeedError::Trip(Reject::Stale))
        );
        assert_eq!(trip, expired);
        assert_eq!(mark(&world), before);
    }

    #[test]
    fn a_non_pick_command_and_a_tick_do_not_change_the_hold() {
        let (mut world, mut trip, bag, operator) = named(7, 0);
        let before = (mark(&world), trip);
        for effect in [
            Effect::ReportBag { bag },
            Effect::SignalReady { to: operator },
            Effect::Follow { person: operator },
            Effect::Release { bag, to: operator },
        ] {
            assert_eq!(
                execute_pick(&mut world, &mut trip, 8, effect),
                Err(FeedError::NotACommand)
            );
            assert_eq!((mark(&world), trip), before);
        }
        execute_pick(&mut world, &mut trip, 8, Effect::Pick { bag }).unwrap();
        world.advance(Step::from_ms(500)).unwrap();
        trip.apply(80, Event::Tick).unwrap();
        assert_eq!(world.now_ms(), 500);
        assert_eq!(trip.now_ms(), Some(80));
        assert_eq!(trip.phase(), Phase::Signaling);
        assert_eq!(world.carried(), Some(bag));
        assert_eq!(trip.carried(), Some(bag));
        assert_eq!(held_copy(&world, bag).custody, Custody::Gripper);
        assert_eq!(
            world.observe().robot.pose.point(),
            Point::new(Mm::new(0), Mm::new(0))
        );
    }

    #[test]
    fn a_legal_two_meter_placement_behind_the_operator_is_outside_the_seeded_reach() {
        let (mut world, mut trip, bag, _) = named(4, 0);
        let robot = world.observe().robot.pose.point();
        let operator = world.observe().operator.pose.point();
        let at = Point::new(Mm::new(0), Mm::new(3000));
        assert_eq!(robot, Point::new(Mm::new(0), Mm::new(0)));
        assert_eq!(operator, Point::new(Mm::new(0), Mm::new(1000)));
        let from_operator = {
            let dy = i128::from(at.y().get()) - i128::from(operator.y().get());
            dy * dy
        };
        let from_robot = {
            let dy = i128::from(at.y().get()) - i128::from(robot.y().get());
            dy * dy
        };
        assert_eq!(from_operator, i128::from(2000) * i128::from(2000));
        assert_eq!(from_robot, i128::from(3000) * i128::from(3000));
        assert!(from_robot > i128::from(manipulation::assume::REACH_MM).pow(2));
        world.place_free(bag, at).unwrap();
        let placed = (mark(&world), trip);
        assert_eq!(held_copy(&world, bag).pose.point(), at);
        assert_eq!(held_copy(&world, bag).custody, Custody::Free);
        assert_eq!(
            execute_pick(&mut world, &mut trip, 5, Effect::Pick { bag }),
            Err(FeedError::Arm(GripError::OutOfReach))
        );
        assert_eq!((mark(&world), trip), placed);
        assert_eq!(trip.phase(), Phase::Picking);
        assert_eq!(trip.carried(), None);
        assert_eq!(world.carried(), None);
        assert_eq!(world.observe().robot.pose.point(), robot);
    }
}
