//! Wires a pointing sample to the selector and then to the trip's report.
//!
//! `point_from_sample` does not take a referee id. [`stage_report`] only posts
//! the selector's bag. [`confirm_with_mock`] accepts a report effect and asks
//! [`hri::MockReport`] for the id to name. `Ok` from `Trip::apply` is not
//! confirmation: a stamp after the budget stores `TimeUp` and returns
//! [`FeedError::TimeUp`]. A pick, ready signal, follow, or release is not a
//! report. Nothing here grasps, signals ready, follows, or hands the bag off.
//! The trip may emit `Effect::Pick` when the name is confirmed; that command
//! is not executed.

use domain::{Bag, Id, Point};
use hri::MockReport;
use perception::{Candidate, SelectError, select};
use task_3_1::{Effect, Event, Outcome, Phase, Reject, Trip};

/// The trip rejected the event, the effect was not a report, or the budget ended.
///
/// [`FeedError::Trip`] leaves the trip unchanged. [`FeedError::TimeUp`] means the
/// deadline was stored and the payload was not applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedError {
    Trip(Reject),
    NotAReport,
    TimeUp,
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FeedError::Trip(_) => "trip rejected",
            FeedError::NotAReport => "not a report",
            FeedError::TimeUp => "time up",
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
    let mut next = *trip;
    let outcome = next
        .apply(now_ms, Event::Pointed { operator, bag })
        .map_err(FeedError::Trip)?;
    let (next, outcome) = keep_deadline(trip, next, outcome)?;
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
    let mut next = *trip;
    let outcome = next
        .apply(now_ms, Event::Named { bag: confirmed })
        .map_err(FeedError::Trip)?;
    let (next, outcome) = keep_deadline(trip, next, outcome)?;
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

#[cfg(test)]
mod tests {
    use super::{FeedError, confirm_with_mock, point_from_sample, stage_report};
    use domain::{Bag, BagKind, Id, Mm, Point, Pose, World};
    use perception::SelectError;
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
}
