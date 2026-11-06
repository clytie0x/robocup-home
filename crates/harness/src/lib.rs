//! Wires a pointing sample to the selector and then to the trip's report.
//!
//! `point_from_sample` does not take a referee id. [`stage_report`] only posts
//! the selector's bag. [`confirm_with_mock`] accepts a report effect and asks
//! [`hri::MockReport`] for the id to name. A pick, ready signal, follow, or
//! release is not a report. Nothing here grasps, signals ready, follows, or
//! hands the bag off. The trip may emit `Effect::Pick` when the name is
//! confirmed; that command is not executed.

use domain::{Bag, Id, Point};
use hri::MockReport;
use perception::{Candidate, SelectError, select};
use task_3_1::{Effect, Event, Reject, Trip};

/// The trip rejected the confirmation, or the effect was not a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedError {
    Trip(Reject),
    NotAReport,
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FeedError::Trip(_) => "trip rejected",
            FeedError::NotAReport => "not a report",
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
pub fn stage_report(trip: &mut Trip, now_ms: u64, operator: Id, bag: Id) -> Result<Effect, Reject> {
    let mut next = *trip;
    let outcome = next.apply(now_ms, Event::Pointed { operator, bag })?;
    let Some(effect) = outcome.effect else {
        return Err(Reject::Impossible);
    };
    *trip = next;
    Ok(effect)
}

/// Confirms `effect` through [`MockReport`] and names that bag on the trip.
///
/// Any effect other than `ReportBag` leaves `trip` unchanged. The id passed to
/// the trip is the id the mock confirmed, which is the id in the report.
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
    next.apply(now_ms, Event::Named { bag: confirmed })
        .map_err(FeedError::Trip)?;
    *trip = next;
    Ok(confirmed)
}

#[cfg(test)]
mod tests {
    use super::{FeedError, confirm_with_mock, point_from_sample, stage_report};
    use domain::{Bag, BagKind, Id, Mm, Point, Pose, World};
    use perception::SelectError;
    use sim_runtime::{SimWorld, direction_mm};
    use task_3_1::{Effect, Event, Phase, Reject, Trip};

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
            Err(Reject::Role)
        );
        assert_eq!(trip, before);

        let idle = Trip::new();
        let mut still = idle;
        assert_eq!(
            stage_report(&mut still, 0, obs.operator.id, obs.bags[0].id),
            Err(Reject::OutOfOrder)
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
