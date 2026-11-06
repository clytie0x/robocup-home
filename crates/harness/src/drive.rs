//! One coordinated base step.
//!
//! The trip stamp is the timeout authority for every drive, including cancel.
//! The world clock is moved to that same stamp or not at all. A split clock, a
//! backward stamp, and the same stamp change nothing. A stamp after the budget
//! stores `TimeUp` and does not apply the command. An in-budget cancel consumes
//! the stamp, marks the follower cancelled, and does not move the base.
//! A blocked step consumes the stamp and does not move the base. `Ok` from
//! `Trip::apply` is not success: a deadline is `TimeUp`.

use domain::{Custody, Id, Point, Step, World};
use manipulation::assume::REACH_MM;
use navigation::{
    Cue, Follower, Intent, Sight, assume::ROBOT_RADIUS_MM, look, plan_approach, plan_follow,
};
use sim_runtime::{SimWorld, SlideError};
use task_3_1::{Effect, Event, LIMIT_MS, Outcome, Phase, Reject, Trip};

use crate::FeedError;

/// What the base was asked to do. The request is not motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    Follow { person: Id },
    Approach { bag: Id },
    Cancel,
}

/// A committed navigation step. Arrival is a trip release command, not a handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavEnd {
    Moved { to: Point },
    Waiting,
    Blocked,
    Stuck,
    Lost,
    WrongPerson,
    Refound,
    Arrived { at: Point, release: Effect },
    Cancelled,
    InReach,
    SameStamp,
}

enum Staged {
    Ready {
        world: SimWorld,
        trip: Trip,
        outcome: Outcome,
    },
    TimedOut {
        world: SimWorld,
        trip: Trip,
    },
}

/// Advances both clocks to `at_ms` and, when the plan is free, slides the base.
///
/// The car coordinate is not read. A held bag moves only inside [`SimWorld::try_slide`].
pub fn step_drive(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    drive: Drive,
    cue: Cue,
) -> Result<NavEnd, FeedError> {
    let now = shared(world, trip)?;
    if matches!(trip.phase(), Phase::Complete | Phase::TimeUp) {
        return Err(FeedError::Terminal);
    }
    if at_ms < now {
        return Err(FeedError::Trip(Reject::Regressed));
    }
    if at_ms == now {
        return Ok(NavEnd::SameStamp);
    }
    let start = trip.start_ms().ok_or(FeedError::Trip(Reject::OutOfOrder))?;
    let elapsed = at_ms
        .checked_sub(start)
        .ok_or(FeedError::Trip(Reject::Regressed))?;
    if elapsed > LIMIT_MS {
        return timeout(world, trip, at_ms);
    }
    if matches!(drive, Drive::Cancel) {
        if !matches!(trip.phase(), Phase::Following | Phase::Lost) {
            return Err(FeedError::Trip(Reject::OutOfOrder));
        }
        let cancelled = follower.cancel();
        return commit_event(
            world,
            trip,
            follower,
            at_ms,
            cancelled,
            Event::Tick,
            |next, outcome| {
                (matches!(next.phase(), Phase::Following | Phase::Lost) && outcome.effect.is_none())
                    .then_some(NavEnd::Cancelled)
            },
        );
    }
    match drive {
        Drive::Cancel => unreachable!(),
        Drive::Follow { person } => follow(world, trip, follower, at_ms, person, cue),
        Drive::Approach { bag } => approach(world, trip, follower, at_ms, bag),
    }
}

fn shared(world: &SimWorld, trip: &Trip) -> Result<u64, FeedError> {
    match trip.now_ms() {
        Some(now) if now == world.now_ms() => Ok(now),
        _ => Err(FeedError::Split),
    }
}

fn timeout(world: &mut SimWorld, trip: &mut Trip, at_ms: u64) -> Result<NavEnd, FeedError> {
    match stage(world, trip, at_ms, None, Event::Tick)? {
        Staged::TimedOut {
            world: next_world,
            trip: next_trip,
        } => {
            *world = next_world;
            *trip = next_trip;
            Err(FeedError::TimeUp)
        }
        Staged::Ready { .. } => Err(FeedError::NotACommand),
    }
}

fn follow(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    person: Id,
    cue: Cue,
) -> Result<NavEnd, FeedError> {
    if !matches!(trip.phase(), Phase::Following | Phase::Lost) {
        return Err(FeedError::Trip(Reject::OutOfOrder));
    }
    if follower.person() != person {
        return Err(FeedError::Trip(Reject::WrongId));
    }
    let observed = world.observe();
    let (people, bodies) = look(
        observed.robot.pose.point(),
        observed.operator,
        &observed.obstacles,
    );
    let sight = Sight {
        robot: observed.robot.pose.point(),
        people: &people,
        bodies: &bodies,
    };
    let before = *follower;
    let (planned, intent) = plan_follow(&before, &sight, cue);
    if trip.phase() == Phase::Lost {
        return while_lost(world, trip, follower, at_ms, person, planned, intent);
    }
    match intent {
        Intent::Cancelled => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (matches!(next.phase(), Phase::Following | Phase::Lost) && outcome.effect.is_none())
                    .then_some(NavEnd::Cancelled)
            },
        ),
        Intent::Lost => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::OperatorLost,
            |next, outcome| {
                (next.phase() == Phase::Lost && outcome.effect.is_none()).then_some(NavEnd::Lost)
            },
        ),
        Intent::WrongPerson => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::OperatorLost,
            |next, outcome| {
                (next.phase() == Phase::Lost && outcome.effect.is_none())
                    .then_some(NavEnd::WrongPerson)
            },
        ),
        Intent::Arrived => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Arrived,
            |next, outcome| match outcome.effect {
                Some(release @ Effect::Release { .. })
                    if next.phase() == Phase::Releasing
                        && next.arrived()
                        && !next.main_complete()
                        && next.carried().is_some() =>
                {
                    Some(NavEnd::Arrived {
                        at: Point::new(domain::Mm::new(0), domain::Mm::new(0)),
                        release,
                    })
                }
                _ => None,
            },
        ),
        Intent::Stay => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Following && outcome.effect.is_none())
                    .then_some(NavEnd::Waiting)
            },
        ),
        Intent::Blocked => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Following && outcome.effect.is_none())
                    .then_some(NavEnd::Blocked)
            },
        ),
        Intent::Stuck => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Following && outcome.effect.is_none())
                    .then_some(NavEnd::Stuck)
            },
        ),
        Intent::Move(to) => move_or_block(world, trip, follower, at_ms, before, planned, to),
        Intent::InReach => Err(FeedError::NotACommand),
    }
}

fn while_lost(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    person: Id,
    planned: Follower,
    intent: Intent,
) -> Result<NavEnd, FeedError> {
    match intent {
        Intent::Lost => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Lost && outcome.effect.is_none()).then_some(NavEnd::Lost)
            },
        ),
        Intent::WrongPerson => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Lost && outcome.effect.is_none())
                    .then_some(NavEnd::WrongPerson)
            },
        ),
        Intent::Cancelled => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| {
                (matches!(next.phase(), Phase::Following | Phase::Lost) && outcome.effect.is_none())
                    .then_some(NavEnd::Cancelled)
            },
        ),
        _ => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::OperatorFound { person },
            |next, outcome| {
                (next.phase() == Phase::Following && !next.arrived() && outcome.effect.is_none())
                    .then_some(NavEnd::Refound)
            },
        ),
    }
}

fn approach(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    bag: Id,
) -> Result<NavEnd, FeedError> {
    if trip.phase() != Phase::Picking {
        return Err(FeedError::Trip(Reject::OutOfOrder));
    }
    let observed = world.observe();
    let Some(target) = observed.bags.iter().find(|item| item.id == bag) else {
        return Err(FeedError::Arm(manipulation::GripError::Missing));
    };
    if target.custody != Custody::Free {
        return Err(FeedError::Arm(manipulation::GripError::NotFree));
    }
    if !target.visible {
        return Err(FeedError::Arm(manipulation::GripError::Invisible));
    }
    let (_people, bodies) = look(
        observed.robot.pose.point(),
        observed.operator,
        &observed.obstacles,
    );
    let before = *follower;
    match plan_approach(
        observed.robot.pose.point(),
        target.pose.point(),
        &bodies,
        REACH_MM,
    ) {
        Intent::InReach => commit_event(
            world,
            trip,
            follower,
            at_ms,
            before,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Picking && outcome.effect.is_none())
                    .then_some(NavEnd::InReach)
            },
        ),
        Intent::Stay => commit_event(
            world,
            trip,
            follower,
            at_ms,
            before,
            Event::Tick,
            |next, outcome| {
                (next.phase() == Phase::Picking && outcome.effect.is_none())
                    .then_some(NavEnd::Waiting)
            },
        ),
        Intent::Blocked | Intent::Stuck => {
            let (next, blocked) = before.note_blocked();
            let end = match blocked {
                Intent::Stuck => NavEnd::Stuck,
                _ => NavEnd::Blocked,
            };
            commit_event(
                world,
                trip,
                follower,
                at_ms,
                next,
                Event::Tick,
                |trip_next, outcome| {
                    (trip_next.phase() == Phase::Picking && outcome.effect.is_none()).then_some(end)
                },
            )
        }
        Intent::Move(to) => move_or_block(world, trip, follower, at_ms, before, before, to),
        _ => Err(FeedError::NotACommand),
    }
}

fn move_or_block(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    before: Follower,
    planned: Follower,
    to: Point,
) -> Result<NavEnd, FeedError> {
    let from = world.observe().robot.pose.point();
    if step_too_long(from, to) {
        return Err(FeedError::NotACommand);
    }
    let mut probe = world.clone();
    match probe.try_slide(to, ROBOT_RADIUS_MM) {
        Err(SlideError::Blocked) => {
            let (next, blocked) = before.note_blocked();
            let end = match blocked {
                Intent::Stuck => NavEnd::Stuck,
                _ => NavEnd::Blocked,
            };
            commit_event(
                world,
                trip,
                follower,
                at_ms,
                next,
                Event::Tick,
                |next_trip, outcome| {
                    outcome
                        .effect
                        .is_none()
                        .then_some(end)
                        .filter(|_| matches!(next_trip.phase(), Phase::Following | Phase::Picking))
                },
            )
        }
        Ok(()) => commit_slide(world, trip, follower, at_ms, planned, to),
    }
}

fn commit_slide(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    planned: Follower,
    to: Point,
) -> Result<NavEnd, FeedError> {
    match stage(world, trip, at_ms, Some(to), Event::Tick)? {
        Staged::TimedOut {
            world: next_world,
            trip: next_trip,
        } => {
            *world = next_world;
            *trip = next_trip;
            Err(FeedError::TimeUp)
        }
        Staged::Ready {
            world: next_world,
            trip: next_trip,
            outcome,
        } if outcome.effect.is_none()
            && next_world.observe().robot.pose.point() == to
            && matches!(next_trip.phase(), Phase::Following | Phase::Picking) =>
        {
            *world = next_world;
            *trip = next_trip;
            *follower = planned;
            Ok(NavEnd::Moved { to })
        }
        Staged::Ready { .. } => Err(FeedError::NotACommand),
    }
}

fn commit_event(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    planned: Follower,
    event: Event,
    accept: impl FnOnce(&Trip, Outcome) -> Option<NavEnd>,
) -> Result<NavEnd, FeedError> {
    match stage(world, trip, at_ms, None, event)? {
        Staged::TimedOut {
            world: next_world,
            trip: next_trip,
        } => {
            *world = next_world;
            *trip = next_trip;
            Err(FeedError::TimeUp)
        }
        Staged::Ready {
            world: next_world,
            trip: next_trip,
            outcome,
        } => {
            let Some(end) = accept(&next_trip, outcome) else {
                return Err(FeedError::NotACommand);
            };
            let end = match end {
                NavEnd::Arrived { release, .. } => NavEnd::Arrived {
                    at: next_world.observe().robot.pose.point(),
                    release,
                },
                other => other,
            };
            *world = next_world;
            *trip = next_trip;
            *follower = planned;
            Ok(end)
        }
    }
}

fn step_too_long(from: Point, to: Point) -> bool {
    let dx = i128::from(to.x().get()) - i128::from(from.x().get());
    let dy = i128::from(to.y().get()) - i128::from(from.y().get());
    let cap = i128::from(navigation::assume::STEP_MM);
    dx * dx + dy * dy > cap * cap
}

fn stage(
    world: &SimWorld,
    trip: &Trip,
    at_ms: u64,
    slide: Option<Point>,
    event: Event,
) -> Result<Staged, FeedError> {
    let delta = at_ms
        .checked_sub(world.now_ms())
        .ok_or(FeedError::Trip(Reject::Regressed))?;
    let mut moved = world.clone();
    if let Some(to) = slide {
        moved
            .try_slide(to, ROBOT_RADIUS_MM)
            .map_err(|_| FeedError::NotACommand)?;
    }
    let mut clocked = moved.clone();
    clocked
        .advance(Step::from_ms(delta))
        .map_err(|_| FeedError::NotACommand)?;
    let mut next = *trip;
    let outcome = next.apply(at_ms, event).map_err(FeedError::Trip)?;
    if outcome.effect.is_none() && next.phase() == Phase::TimeUp {
        let mut only_clock = world.clone();
        only_clock
            .advance(Step::from_ms(delta))
            .map_err(|_| FeedError::NotACommand)?;
        let mut late = *trip;
        let late_outcome = late.apply(at_ms, Event::Tick).map_err(FeedError::Trip)?;
        if late_outcome.effect.is_some() || late.phase() != Phase::TimeUp {
            return Err(FeedError::NotACommand);
        }
        return Ok(Staged::TimedOut {
            world: only_clock,
            trip: late,
        });
    }
    Ok(Staged::Ready {
        world: clocked,
        trip: next,
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PickEnd, confirm_ready, execute_pick};
    use domain::{Id, Mm, Obstacle, ObstacleClass, Pose, Step, World, within_reach};
    use navigation::assume::{SENSOR_MM, STUCK_LIMIT};
    use sim_runtime::SimWorld;
    use task_3_1::{Effect, Event, Phase, Reject, Trip};

    fn point(x: i32, y: i32) -> Point {
        Point::new(Mm::new(x), Mm::new(y))
    }

    fn ready() -> (SimWorld, Trip, Id, Id) {
        let mut world = SimWorld::from_seed(1);
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        let end = execute_pick(&mut world, &mut trip, 0, Effect::Pick { bag }).unwrap();
        let PickEnd::Matched { signal, bag: held } = end else {
            panic!("pick {end:?}");
        };
        assert_eq!(held, bag);
        let follow = confirm_ready(&world, &mut trip, 0, signal).unwrap();
        assert_eq!(follow, Effect::Follow { person: operator });
        assert_eq!(Some(world.now_ms()), trip.now_ms());
        assert!(!trip.arrived());
        assert_eq!(world.observe().robot.pose.point(), point(0, 0));
        (world, trip, operator, bag)
    }

    fn pose(world: &SimWorld) -> Point {
        world.observe().robot.pose.point()
    }

    #[test]
    fn ready_does_not_arrive_and_one_step_moves_the_held_bag() {
        let (mut world, mut trip, operator, bag) = ready();
        let mut follower = Follower::follow(operator);
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        let NavEnd::Moved { to } = end else {
            panic!("{end:?}");
        };
        assert_eq!(pose(&world), to);
        assert_eq!(world.now_ms(), 200);
        assert_eq!(trip.now_ms(), Some(200));
        assert_eq!(trip.phase(), Phase::Following);
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        let held = world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id == bag)
            .unwrap();
        assert_eq!(held.custody, Custody::Gripper);
        assert_eq!(held.pose.point(), to);
        assert_ne!(to, world.truth().car());
        assert_eq!(world.revealed_car(), None);
    }

    #[test]
    fn split_clocks_and_a_backward_stamp_do_not_move() {
        let mut world = SimWorld::from_seed(1);
        world.advance(Step::from_ms(400_000)).unwrap();
        let mut trip = Trip::new();
        let operator = world.observe().operator.id;
        trip.apply(10, Event::Begin).unwrap();
        let mut follower = Follower::follow(operator);
        let before_world = world.observe();
        let before_trip = trip;
        let err = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            11,
            Drive::Follow { person: operator },
            Cue::None,
        );
        assert_eq!(err, Err(FeedError::Split));
        assert_eq!(world.now_ms(), 400_000);
        assert_eq!(trip, before_trip);
        assert_eq!(world.observe().robot, before_world.robot);
        assert_eq!(pose(&world), point(0, 0));

        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            50,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        let marked = (pose(&world), world.now_ms(), trip);
        let err = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            40,
            Drive::Follow { person: operator },
            Cue::None,
        );
        assert_eq!(err, Err(FeedError::Trip(Reject::Regressed)));
        assert_eq!(pose(&world), marked.0);
        assert_eq!(world.now_ms(), marked.1);
        assert_eq!(trip, marked.2);
    }

    #[test]
    fn the_same_stamp_is_not_a_second_step() {
        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        let events = world.events().len();
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            0,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert_eq!(end, NavEnd::SameStamp);
        assert_eq!(pose(&world), point(0, 0));
        assert!(!trip.arrived());
        assert_eq!(world.events().len(), events);
        assert_eq!(trip.now_ms(), Some(0));
    }

    #[test]
    fn the_budget_allows_300000_and_stops_at_300001_without_moving() {
        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            LIMIT_MS,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        assert_eq!(world.now_ms(), LIMIT_MS);
        assert_eq!(trip.now_ms(), Some(LIMIT_MS));
        assert_eq!(trip.phase(), Phase::Following);
        assert!(!trip.arrived());
        let frozen = pose(&world);
        let err = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            LIMIT_MS + 1,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        );
        assert_eq!(err, Err(FeedError::TimeUp));
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(pose(&world), frozen);
        assert_eq!(world.now_ms(), LIMIT_MS + 1);
        assert_eq!(trip.now_ms(), Some(LIMIT_MS + 1));
        let again = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            LIMIT_MS + 2,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        );
        assert_eq!(again, Err(FeedError::Terminal));
        assert_eq!(world.now_ms(), LIMIT_MS + 1);
        assert_eq!(pose(&world), frozen);
    }

    #[test]
    fn cumulative_steps_and_a_start_near_the_top_share_one_clock() {
        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        for stamp in [100_000u64, 200_000, LIMIT_MS] {
            step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                stamp,
                Drive::Follow { person: operator },
                Cue::None,
            )
            .unwrap();
            assert_eq!(world.now_ms(), stamp);
            assert_eq!(trip.now_ms(), Some(stamp));
            assert_ne!(trip.phase(), Phase::TimeUp);
        }
        let frozen = pose(&world);
        assert_eq!(
            step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                LIMIT_MS + 1,
                Drive::Follow { person: operator },
                Cue::None,
            ),
            Err(FeedError::TimeUp)
        );
        assert_eq!(pose(&world), frozen);

        let start = u64::MAX - LIMIT_MS - 1;
        let mut world = SimWorld::from_seed(1);
        world.advance(Step::from_ms(start)).unwrap();
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let mut trip = Trip::new();
        trip.apply(start, Event::Begin).unwrap();
        trip.apply(start, Event::Pointed { operator, bag }).unwrap();
        trip.apply(start, Event::Named { bag }).unwrap();
        let PickEnd::Matched { signal, .. } =
            execute_pick(&mut world, &mut trip, start, Effect::Pick { bag }).unwrap()
        else {
            panic!("pick");
        };
        confirm_ready(&world, &mut trip, start, signal).unwrap();
        assert_eq!(world.now_ms(), start);
        let mut follower = Follower::follow(operator);
        let inside = start + LIMIT_MS;
        step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            inside,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        assert_eq!(world.now_ms(), inside);
        assert_eq!(trip.phase(), Phase::Following);
        let frozen = pose(&world);
        assert_eq!(
            step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                u64::MAX,
                Drive::Follow { person: operator },
                Cue::TakeBag,
            ),
            Err(FeedError::TimeUp)
        );
        assert_eq!(pose(&world), frozen);
        assert_eq!(world.now_ms(), u64::MAX);
        assert_eq!(trip.now_ms(), Some(u64::MAX));
        assert!(!trip.arrived());
    }

    #[test]
    fn cancel_and_the_wrong_person_do_not_complete() {
        let (mut world, mut trip, operator, bag) = ready();
        let mut follower = Follower::follow(operator);
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            10,
            Drive::Cancel,
            Cue::TakeBag,
        )
        .unwrap();
        assert_eq!(end, NavEnd::Cancelled);
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(world.now_ms(), 10);
        assert_eq!(trip.now_ms(), Some(10));
        assert_eq!(trip.phase(), Phase::Following);
        assert!(follower.cancelled());
        assert!(!trip.arrived());
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            20,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert_eq!(end, NavEnd::Cancelled);
        assert!(!trip.arrived());
        assert_eq!(trip.carried(), Some(bag));
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(world.now_ms(), 20);
        assert_eq!(trip.phase(), Phase::Following);

        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        world.place_operator(point(0, SENSOR_MM + 500));
        world.set_obstacles(vec![crowd_near()]);
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            30,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert_eq!(end, NavEnd::WrongPerson);
        assert_eq!(trip.phase(), Phase::Lost);
        assert!(!trip.arrived());
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(world.now_ms(), 30);
        assert_ne!(pose(&world), world.truth().car());

        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        let before = (pose(&world), world.now_ms(), trip, follower.cancelled());
        assert_eq!(
            step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                LIMIT_MS + 1,
                Drive::Cancel,
                Cue::None,
            ),
            Err(FeedError::TimeUp)
        );
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(world.now_ms(), LIMIT_MS + 1);
        assert_eq!(pose(&world), before.0);
        assert!(!follower.cancelled());
        assert!(!trip.arrived());

        let mut world = SimWorld::from_seed(1);
        world.advance(Step::from_ms(400_000)).unwrap();
        let mut trip = Trip::new();
        let operator = world.observe().operator.id;
        trip.apply(10, Event::Begin).unwrap();
        let mut follower = Follower::follow(operator);
        let pose_before = pose(&world);
        assert_eq!(
            step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                11,
                Drive::Cancel,
                Cue::None,
            ),
            Err(FeedError::Split)
        );
        assert!(!follower.cancelled());
        assert_eq!(world.now_ms(), 400_000);
        assert_eq!(trip.now_ms(), Some(10));
        assert_eq!(trip.phase(), Phase::AwaitPoint);
        assert_eq!(pose(&world), pose_before);
    }

    fn crowd_near() -> Obstacle {
        Obstacle {
            id: Id::new(7).unwrap(),
            pose: Pose::new(point(400, 0)),
            class: ObstacleClass::Crowd,
            clearance: Mm::new(400),
            bodies: (0u32..3)
                .map(|index| domain::Disk {
                    id: Id::new(10 + index).unwrap(),
                    center: point(400, i32::try_from(index).unwrap() * 300),
                    radius: Mm::new(200),
                })
                .collect(),
        }
    }

    #[test]
    fn loss_refind_and_the_cue_are_not_a_car_lookup() {
        let (mut world, mut trip, operator, bag) = ready();
        let mut follower = Follower::follow(operator);
        world.set_obstacles(Vec::new());
        world.place_operator(point(0, SENSOR_MM + 800));
        let lost = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            100,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        assert_eq!(lost, NavEnd::Lost);
        assert_eq!(trip.phase(), Phase::Lost);
        assert_eq!(pose(&world), point(0, 0));
        world.place_operator(point(0, 1000));
        let found = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert_eq!(found, NavEnd::Refound);
        assert_eq!(trip.phase(), Phase::Following);
        assert!(!trip.arrived());
        assert_eq!(pose(&world), point(0, 0));
        let cued = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            300,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert_ne!(
            cued,
            NavEnd::Arrived {
                at: point(0, 0),
                release: Effect::Release { bag, to: operator },
            }
        );
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(trip.phase(), Phase::Following);
        assert_eq!(trip.carried(), Some(bag));
        assert!(crate::execute_handoff(&mut world, &mut trip, 300, operator).is_err());
        assert!(!trip.main_complete());
        assert_eq!(world.revealed_car(), None);
        assert_eq!(world.now_ms(), 300);
    }

    #[test]
    fn a_cue_at_the_start_or_across_a_disk_does_not_hand_off() {
        let (mut world, mut trip, operator, bag) = ready();
        let mut follower = Follower::follow(operator);
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert_ne!(
            end,
            NavEnd::Arrived {
                at: pose(&world),
                release: Effect::Release { bag, to: operator },
            }
        );
        assert!(!trip.arrived());
        assert!(crate::execute_handoff(&mut world, &mut trip, 200, operator).is_err());
        assert!(!trip.main_complete());
        assert_eq!(world.revealed_car(), None);

        let (mut world, mut trip, operator, _bag) = ready();
        let mut follower = Follower::follow(operator);
        step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        assert_ne!(pose(&world), point(0, 0));
        world.place_operator(point(0, 1100));
        world.set_obstacles(vec![Obstacle {
            id: Id::new(5).unwrap(),
            pose: Pose::new(point(0, 500)),
            class: ObstacleClass::SmallGround,
            clearance: Mm::new(400),
            bodies: vec![domain::Disk {
                id: Id::new(5).unwrap(),
                center: point(0, 500),
                radius: Mm::new(40),
            }],
        }]);
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            400,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert!(!matches!(end, NavEnd::Arrived { .. }));
        assert!(!trip.arrived());
        assert!(crate::execute_handoff(&mut world, &mut trip, 400, operator).is_err());
        assert!(!trip.main_complete());
    }

    #[test]
    fn a_followed_clear_journey_can_hand_off_without_reading_the_car() {
        let (mut world, mut trip, operator, bag) = ready();
        let mut follower = Follower::follow(operator);
        let car = world.truth().car();
        step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        assert_ne!(pose(&world), point(0, 0));
        world.place_operator(point(400, 1000));
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            400,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        let NavEnd::Arrived { at, release } = end else {
            panic!("{end:?}");
        };
        assert_eq!(release, Effect::Release { bag, to: operator });
        assert_eq!(at, pose(&world));
        assert_ne!(at, car);
        assert_ne!(at, point(0, 0));
        assert!(trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(world.revealed_car(), None);
        crate::execute_handoff(&mut world, &mut trip, 400, operator).unwrap();
        assert!(trip.main_complete());
        assert_eq!(world.revealed_car(), None);
        assert_ne!(pose(&world), car);
    }

    #[test]
    fn a_far_bag_approach_does_not_panic_or_overshoot() {
        for x in [2_968_220, 2_968_221, 3_000_000, 5_000_000] {
            let mut world = SimWorld::from_seed(1);
            let operator = world.observe().operator.id;
            let bag = world.observe().bags[0].id;
            let mut trip = Trip::new();
            trip.apply(0, Event::Begin).unwrap();
            trip.apply(0, Event::Pointed { operator, bag }).unwrap();
            trip.apply(0, Event::Named { bag }).unwrap();
            world.place_free(bag, point(x, 0)).unwrap();
            let mut follower = Follower::follow(operator);
            let end = step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                100,
                Drive::Approach { bag },
                Cue::None,
            )
            .unwrap();
            let NavEnd::Moved { to } = end else {
                panic!("{x} {end:?}");
            };
            assert!(to.x().get() > 0, "{x} {to:?}");
            assert_eq!(to.y().get(), 0, "{x} {to:?}");
            assert!(
                i64::from(to.x().get()) * i64::from(to.x().get()) <= 200 * 200,
                "{x}"
            );
            assert_eq!(world.now_ms(), 100);
            assert_eq!(trip.now_ms(), Some(100));
            assert_eq!(trip.phase(), Phase::Picking);
            assert!(trip.carried().is_none());
            let placed = world.observe();
            let free = placed.bags.iter().find(|item| item.id == bag).unwrap();
            assert_eq!(free.pose.point(), point(x, 0));
            assert_eq!(free.custody, domain::Custody::Free);
        }
    }

    #[test]
    fn the_harness_does_not_slide_through_a_disk() {
        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        world.place_operator(point(2000, 0));
        world.set_obstacles(vec![Obstacle {
            id: Id::new(5).unwrap(),
            pose: Pose::new(point(100, 269)),
            class: ObstacleClass::SmallGround,
            clearance: Mm::new(400),
            bodies: vec![domain::Disk {
                id: Id::new(5).unwrap(),
                center: point(100, 269),
                radius: Mm::new(20),
            }],
        }]);
        let end = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        match end {
            NavEnd::Moved { to } => {
                assert!(domain::segment_within(
                    point(0, 0),
                    point(200, 0),
                    point(100, 269),
                    ROBOT_RADIUS_MM + 20
                ));
                assert!(!domain::segment_within(
                    point(0, 0),
                    to,
                    point(100, 269),
                    ROBOT_RADIUS_MM + 20
                ));
                assert_ne!(to, point(2000, 0));
            }
            NavEnd::Blocked | NavEnd::Stuck => {}
            other => panic!("{other:?}"),
        }
        assert!(!trip.arrived());
        assert_eq!(world.revealed_car(), None);
    }

    #[test]
    fn blocked_steps_consume_time_until_stuck() {
        let (mut world, mut trip, operator, _) = ready();
        let mut follower = Follower::follow(operator);
        world.set_obstacles(ring());
        for step in 1..=u64::from(STUCK_LIMIT) {
            let end = step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                step * 1_000,
                Drive::Follow { person: operator },
                Cue::None,
            )
            .unwrap();
            assert_eq!(pose(&world), point(0, 0));
            assert_eq!(world.now_ms(), step * 1_000);
            assert_eq!(trip.now_ms(), Some(step * 1_000));
            assert!(!trip.arrived());
            if step < u64::from(STUCK_LIMIT) {
                assert_eq!(end, NavEnd::Blocked);
            } else {
                assert_eq!(end, NavEnd::Stuck);
            }
        }
        assert_eq!(trip.phase(), Phase::Following);
    }

    fn ring() -> Vec<Obstacle> {
        let mut bodies = Vec::new();
        let headings = [
            (1024, 0),
            (946, 392),
            (724, 724),
            (392, 946),
            (0, 1024),
            (-392, 946),
            (-724, 724),
            (-946, 392),
            (-1024, 0),
            (-946, -392),
            (-724, -724),
            (-392, -946),
            (0, -1024),
            (392, -946),
            (724, -724),
            (946, -392),
        ];
        for (index, (cos, sin)) in headings.iter().copied().enumerate() {
            let x = i32::try_from(i64::from(450) * i64::from(cos) / 1024).unwrap();
            let y = i32::try_from(i64::from(450) * i64::from(sin) / 1024).unwrap();
            bodies.push(domain::Disk {
                id: Id::new(20 + u32::try_from(index).unwrap()).unwrap(),
                center: point(x, y),
                radius: Mm::new(180),
            });
        }
        vec![Obstacle {
            id: Id::new(8).unwrap(),
            pose: Pose::new(point(0, 0)),
            class: ObstacleClass::Barriers,
            clearance: Mm::new(400),
            bodies,
        }]
    }

    #[test]
    fn a_legal_bag_behind_the_operator_is_reached_by_moving_the_base() {
        let mut world = SimWorld::from_seed(1);
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let other = world.observe().bags[1].pose.point();
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        world.place_free(bag, point(0, 3000)).unwrap();
        assert!(!within_reach(point(0, 0), point(0, 3000), REACH_MM));
        assert_eq!(REACH_MM, 2500);
        let mut follower = Follower::follow(operator);
        let mut stamp = 0u64;
        let mut reached = false;
        for _ in 0..20 {
            stamp += 200;
            let end = step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                stamp,
                Drive::Approach { bag },
                Cue::None,
            )
            .unwrap();
            assert_eq!(world.now_ms(), stamp);
            assert_eq!(trip.now_ms(), Some(stamp));
            assert_eq!(trip.phase(), Phase::Picking);
            assert!(trip.carried().is_none());
            let free = world
                .observe()
                .bags
                .into_iter()
                .find(|item| item.id == bag)
                .unwrap();
            assert_eq!(free.pose.point(), point(0, 3000));
            assert_eq!(free.custody, Custody::Free);
            if end == NavEnd::InReach {
                reached = true;
                break;
            }
            assert!(matches!(end, NavEnd::Moved { .. } | NavEnd::Waiting));
        }
        assert!(reached);
        assert!(within_reach(pose(&world), point(0, 3000), REACH_MM));
        assert_eq!(REACH_MM, 2500);
        let picked = execute_pick(&mut world, &mut trip, stamp, Effect::Pick { bag }).unwrap();
        assert!(matches!(picked, PickEnd::Matched { .. }));
        assert_eq!(world.carried(), Some(bag));
        let left = world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id != bag)
            .unwrap();
        assert_eq!(left.pose.point(), other);
        assert!(!trip.ready());
        assert!(!trip.arrived());
    }
}
