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
    Cue, Follower, Intent, Posted, SampleHistory, Taken, accept, assume::ROBOT_RADIUS_MM, cue_from,
    plan_approach, plan_follow,
};
use sim_runtime::{SimWorld, SlideError};
use task_3_1::{Effect, Event, LIMIT_MS, Outcome, Phase, Reject, Trip};

use crate::FeedError;

/// What the base was asked to do. The request is not motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    Follow {
        person: Id,
    },
    Approach {
        bag: Id,
    },
    Cancel,
    /// Follow one visible person after the main trip is complete.
    ///
    /// The car coordinate is not a field. A late stamp does not move the base
    /// and does not rewrite [`Phase::Complete`].
    Accompany {
        person: Id,
    },
}

/// A committed navigation step. Arrival is a trip release command, not a handoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavEnd {
    Moved {
        to: Point,
    },
    Waiting,
    /// No usable sample. The base stays. This is not a fresh world read.
    NeedFresh,
    Blocked,
    Stuck,
    Lost,
    WrongPerson,
    Refound,
    Arrived {
        at: Point,
        release: Effect,
    },
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
    posted: &Posted,
    history: &mut SampleHistory,
) -> Result<NavEnd, FeedError> {
    let now = shared(world, trip)?;
    if matches!(trip.phase(), Phase::Complete | Phase::TimeUp) {
        if let Drive::Accompany { person } = drive
            && trip.phase() == Phase::Complete
        {
            return accompany(world, trip, follower, at_ms, person, posted, history);
        }
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
    let (next_history, taken, _) = accept(*history, posted, now);
    let view = match taken {
        Taken::Rejected => {
            return hold_still(world, trip, follower, at_ms, NavEnd::Waiting);
        }
        Taken::NeedFresh => {
            return hold_still(world, trip, follower, at_ms, NavEnd::NeedFresh);
        }
        Taken::Act(view) => view,
    };
    let result = match drive {
        Drive::Cancel => unreachable!(),
        Drive::Follow { person } => follow(world, trip, follower, at_ms, person, &view),
        Drive::Approach { bag } => approach(world, trip, follower, at_ms, bag, &view),
        Drive::Accompany { .. } => Err(FeedError::Trip(Reject::OutOfOrder)),
    };
    if result.is_ok() {
        *history = next_history;
    }
    result
}

fn hold_still(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    end: NavEnd,
) -> Result<NavEnd, FeedError> {
    commit_event(
        world,
        trip,
        follower,
        at_ms,
        *follower,
        Event::Tick,
        |next, outcome| {
            outcome.effect.is_none().then_some(end).filter(|_| {
                matches!(
                    next.phase(),
                    Phase::Following | Phase::Lost | Phase::Picking | Phase::Complete
                )
            })
        },
    )
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
    view: &navigation::SensorView,
) -> Result<NavEnd, FeedError> {
    if !matches!(trip.phase(), Phase::Following | Phase::Lost) {
        return Err(FeedError::Trip(Reject::OutOfOrder));
    }
    if follower.person() != person {
        return Err(FeedError::Trip(Reject::WrongId));
    }
    let sight = view.sight();
    let cue = cue_from(view);
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
    view: &navigation::SensorView,
) -> Result<NavEnd, FeedError> {
    if trip.phase() != Phase::Picking {
        return Err(FeedError::Trip(Reject::OutOfOrder));
    }
    let Some(target) = view.bags.iter().find(|item| item.id == bag) else {
        return Err(FeedError::Arm(manipulation::GripError::Missing));
    };
    if target.custody != Custody::Free {
        return Err(FeedError::Arm(manipulation::GripError::NotFree));
    }
    let before = *follower;
    match plan_approach(view.robot, target.pose.point(), &view.bodies, REACH_MM) {
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
                    outcome.effect.is_none().then_some(end).filter(|_| {
                        matches!(
                            next_trip.phase(),
                            Phase::Following | Phase::Picking | Phase::Complete
                        )
                    })
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
            && matches!(
                next_trip.phase(),
                Phase::Following | Phase::Picking | Phase::Complete
            ) =>
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

fn accompany(
    world: &mut SimWorld,
    trip: &mut Trip,
    follower: &mut Follower,
    at_ms: u64,
    person: Id,
    posted: &Posted,
    history: &mut SampleHistory,
) -> Result<NavEnd, FeedError> {
    let now = world.now_ms();
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
        return Err(FeedError::TimeUp);
    }
    if follower.person() != person {
        return Err(FeedError::Trip(Reject::WrongId));
    }
    let (next_history, taken, _) = accept(*history, posted, now);
    let view = match taken {
        Taken::Act(view) => view,
        Taken::Rejected => return hold_still(world, trip, follower, at_ms, NavEnd::Waiting),
        Taken::NeedFresh => return hold_still(world, trip, follower, at_ms, NavEnd::NeedFresh),
    };
    let sight = view.sight();
    let before = *follower;
    let (planned, intent) = plan_follow(&before, &sight, Cue::None);
    let done = |next: &Trip, outcome: Outcome, end: NavEnd| {
        (next.phase() == Phase::Complete && outcome.effect.is_none()).then_some(end)
    };
    let result = match intent {
        Intent::Move(to) => move_or_block(world, trip, follower, at_ms, before, planned, to),
        Intent::Stay | Intent::Arrived | Intent::InReach => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| done(next, outcome, NavEnd::Waiting),
        ),
        Intent::Blocked => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| done(next, outcome, NavEnd::Blocked),
        ),
        Intent::Stuck => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| done(next, outcome, NavEnd::Stuck),
        ),
        Intent::Lost => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| done(next, outcome, NavEnd::Lost),
        ),
        Intent::WrongPerson => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| done(next, outcome, NavEnd::WrongPerson),
        ),
        Intent::Cancelled => commit_event(
            world,
            trip,
            follower,
            at_ms,
            planned,
            Event::Tick,
            |next, outcome| done(next, outcome, NavEnd::Cancelled),
        ),
    };
    if result.is_ok() {
        *history = next_history;
    }
    result
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

    fn live_post(world: &SimWorld, cue: Cue) -> Posted {
        let observed = world.observe();
        let offered = cue == Cue::TakeBag;
        let view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            offered,
            observed.now_ms,
        );
        Posted::ready(view, observed.now_ms)
    }

    fn step_drive(
        world: &mut SimWorld,
        trip: &mut Trip,
        follower: &mut Follower,
        at_ms: u64,
        drive: Drive,
        cue: Cue,
    ) -> Result<NavEnd, FeedError> {
        let posted = live_post(world, cue);
        let mut history = SampleHistory::new();
        super::step_drive(world, trip, follower, at_ms, drive, &posted, &mut history)
    }

    use crate::{PickEnd, confirm_ready, execute_handoff, execute_pick};
    use domain::{Id, Mm, Obstacle, ObstacleClass, Pose, Step, World, within_reach};
    use manipulation::GripError;
    use navigation::assume::{SENSOR_MM, STUCK_LIMIT};
    use navigation::sense;
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
    fn accompany_after_the_limit_keeps_complete_and_does_not_move() {
        let (mut world, mut trip, operator, bag) = ready();
        trip.apply(0, Event::Arrived).unwrap();
        execute_handoff(&mut world, &mut trip, 0, operator).unwrap();
        assert_eq!(trip.phase(), Phase::Complete);
        assert_eq!(world.carried(), None);
        let pose_before = pose(&world);
        let clock = world.now_ms();
        let events = world.events().len();
        let mut follower = Follower::follow(operator);
        let err = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            LIMIT_MS + 1,
            Drive::Accompany { person: operator },
            Cue::None,
        );
        assert_eq!(err, Err(FeedError::TimeUp));
        assert_eq!(trip.phase(), Phase::Complete);
        assert!(trip.main_complete());
        assert_eq!(pose(&world), pose_before);
        assert_eq!(world.now_ms(), clock);
        assert_eq!(world.events().len(), events);
        assert_eq!(world.carried(), None);
        let _ = bag;
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
    fn an_unsensed_extreme_bag_does_not_move_the_base() {
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
            let err = step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                100,
                Drive::Approach { bag },
                Cue::None,
            )
            .unwrap_err();
            assert_eq!(err, FeedError::Arm(GripError::Missing), "{x}");
            assert_eq!(pose(&world), point(0, 0), "{x}");
            assert_eq!(world.now_ms(), 0, "{x}");
            assert_eq!(trip.now_ms(), Some(0), "{x}");
            assert_eq!(trip.phase(), Phase::Picking, "{x}");
            assert!(trip.carried().is_none(), "{x}");
            let placed = world.observe();
            let free = placed.bags.iter().find(|item| item.id == bag).unwrap();
            assert_eq!(free.pose.point(), point(x, 0), "{x}");
            assert_eq!(free.custody, domain::Custody::Free, "{x}");
        }
    }

    fn following_trip(world: &SimWorld) -> (Trip, Id) {
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        trip.apply(0, Event::Grasped { bag }).unwrap();
        trip.apply(0, Event::ReadyTold).unwrap();
        (trip, operator)
    }

    #[test]
    fn a_rejected_command_does_not_keep_the_sample_history() {
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        world.advance(Step::from_ms(400)).unwrap();
        trip.apply(400, Event::Tick).unwrap();
        let observed = world.observe();
        let mut view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            400,
        );
        view.captured_ms = 400;
        let posted = Posted::ready(view, 400);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let pose_before = pose(&world);
        let stranger = Id::new(99).unwrap();
        let err = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            600,
            Drive::Follow { person: stranger },
            &posted,
            &mut history,
        );
        assert_eq!(err, Err(FeedError::Trip(Reject::WrongId)));
        assert_eq!(history.last_capture_ms(), None);
        assert_eq!(pose(&world), pose_before);
        assert_eq!(world.now_ms(), 400);
        let mut recovery = sense(
            world.observe().robot.pose.point(),
            world.observe().operator,
            &world.observe().bags,
            &world.obstacles_for_planner(),
            false,
            100,
        );
        recovery.captured_ms = 100;
        let posted = Posted::ready(recovery, 400);
        super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            600,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        assert_eq!(history.last_capture_ms(), Some(100));
        assert_ne!(pose(&world), pose_before);

        let mut world = SimWorld::from_seed(1);
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        let view = sense(
            world.observe().robot.pose.point(),
            world.observe().operator,
            &world.observe().bags,
            &world.obstacles_for_planner(),
            false,
            0,
        );
        let posted = Posted::ready(view, 0);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let err = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        );
        assert_eq!(err, Err(FeedError::Trip(Reject::OutOfOrder)));
        assert_eq!(history.last_capture_ms(), None);
        assert_eq!(world.now_ms(), 0);

        let err = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Accompany { person: operator },
            &posted,
            &mut history,
        );
        assert_eq!(err, Err(FeedError::Trip(Reject::OutOfOrder)));
        assert_eq!(history.last_capture_ms(), None);
    }

    #[test]
    fn a_held_post_is_not_replayed_across_later_decisions() {
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        let observed = world.observe();
        let view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            0,
        );
        assert_eq!(view.robot, point(0, 0));
        assert_eq!(view.people[0].at, point(0, 1_000));
        world.place_operator(point(1_500, 0));
        let posted = Posted::ready(view, 0);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let end = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        assert!(matches!(end, NavEnd::Moved { .. }));
        let first = pose(&world);
        assert_eq!(first.x().get(), 0);
        assert!(first.y().get() > 0);
        assert_eq!(history.last_capture_ms(), Some(0));
        assert_eq!(posted.delivered_ms(), 0);
        for at in [400_u64, 600, 800, 1_000, 1_200, 1_400] {
            let end = super::step_drive(
                &mut world,
                &mut trip,
                &mut follower,
                at,
                Drive::Follow { person: operator },
                &posted,
                &mut history,
            )
            .unwrap();
            assert_eq!(end, NavEnd::NeedFresh, "{at}");
            assert_eq!(pose(&world), first, "{at}");
            assert_eq!(history.last_capture_ms(), Some(0), "{at}");
            assert_eq!(world.now_ms(), at, "{at}");
        }

        let mut late = SimWorld::from_seed(1);
        let (mut late_trip, late_operator) = following_trip(&late);
        late.advance(Step::from_ms(100_000)).unwrap();
        late_trip.apply(100_000, Event::Tick).unwrap();
        late.place_operator(point(1_500, 0));
        let mut late_view = sense(
            point(0, 0),
            late.observe().operator,
            &late.observe().bags,
            &late.obstacles_for_planner(),
            false,
            0,
        );
        late_view.captured_ms = 0;
        late_view.people[0].at = point(0, 1_000);
        late_view.robot = point(0, 0);
        let late_posted = Posted::ready(late_view, 0);
        let before = pose(&late);
        let mut late_history = SampleHistory::new();
        let mut late_follower = Follower::follow(late_operator);
        let end = super::step_drive(
            &mut late,
            &mut late_trip,
            &mut late_follower,
            100_200,
            Drive::Follow {
                person: late_operator,
            },
            &late_posted,
            &mut late_history,
        )
        .unwrap();
        assert_eq!(end, NavEnd::NeedFresh);
        assert_eq!(pose(&late), before);
        assert_eq!(late_history.last_capture_ms(), None);
        assert_eq!(late.now_ms(), 100_200);

        let recovered = sense(
            world.observe().robot.pose.point(),
            world.observe().operator,
            &world.observe().bags,
            &world.obstacles_for_planner(),
            false,
            world.now_ms(),
        );
        let captured = world.now_ms();
        let fresh = Posted::ready(recovered, captured);
        super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            captured + 200,
            Drive::Follow { person: operator },
            &fresh,
            &mut history,
        )
        .unwrap();
        assert_eq!(history.last_capture_ms(), Some(captured));
        assert!(pose(&world).x().get() > 0, "{:?}", pose(&world));
    }

    #[test]
    fn a_physical_failure_does_not_keep_the_sample_history() {
        let mut world = SimWorld::from_seed(1);
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        world.place_free(bag, point(0, 3_000)).unwrap();
        let observed = world.observe();
        let mut view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            0,
        );
        assert!(view.bags.iter().any(|item| item.id == bag));
        view.robot = point(1_000, 0);
        let posted = Posted::ready(view, 0);
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let err = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Approach { bag },
            &posted,
            &mut history,
        );
        assert_eq!(err, Err(FeedError::NotACommand));
        assert_eq!(history.last_capture_ms(), None);
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(world.now_ms(), 0);
        assert_eq!(trip.now_ms(), Some(0));

        let (mut done, mut done_trip, operator, _bag) = ready();
        done_trip.apply(0, Event::Arrived).unwrap();
        execute_handoff(&mut done, &mut done_trip, 0, operator).unwrap();
        assert_eq!(done_trip.phase(), Phase::Complete);
        let pose_before = pose(&done);
        let clock = done.now_ms();
        let posted = live_post(&done, Cue::None);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let stranger = Id::new(99).unwrap();
        let err = super::step_drive(
            &mut done,
            &mut done_trip,
            &mut follower,
            clock + 200,
            Drive::Accompany { person: stranger },
            &posted,
            &mut history,
        );
        assert_eq!(err, Err(FeedError::Trip(Reject::WrongId)));
        assert_eq!(history.last_capture_ms(), None);
        assert_eq!(pose(&done), pose_before);
        assert_eq!(done.now_ms(), clock);
        assert_eq!(done_trip.phase(), Phase::Complete);
    }

    #[test]
    fn a_held_sample_is_followed_after_the_operator_moves() {
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        let observed = world.observe();
        let view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            observed.now_ms,
        );
        assert_eq!(view.people[0].at, point(0, 1_000));
        world.place_operator(point(1_500, 0));
        let posted = Posted::ready(view, world.now_ms());
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            100,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        let at = pose(&world);
        assert!(at.y().get() > 0, "{at:?}");
        assert_eq!(at.x().get(), 0, "{at:?}");
        assert_ne!(world.observe().operator.pose.point(), point(0, 1_000));
    }

    fn deliver_old_follow(delay_ms: u64) -> (navigation::SampleStamp, (i32, i32)) {
        use navigation::{SampleKind, accept};
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        let observed = world.observe();
        let mut view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            0,
        );
        view.captured_ms = 0;
        world.advance(domain::Step::from_ms(delay_ms)).unwrap();
        trip.apply(delay_ms, Event::Tick).unwrap();
        world.place_operator(point(1_500, 0));
        let posted = Posted::ready(view, delay_ms);
        let (_history, _taken, stamp) = accept(SampleHistory::new(), &posted, delay_ms);
        assert_eq!(stamp.kind, SampleKind::Ready);
        assert_eq!(stamp.captured_ms, Some(0));
        assert_eq!(stamp.delivered_ms, delay_ms);
        assert_eq!(stamp.age_ms, Some(delay_ms));
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            delay_ms + 200,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        let at = pose(&world);
        (stamp, (at.x().get(), at.y().get()))
    }

    #[test]
    fn a_200ms_sample_and_a_1000ms_sample_move_from_the_old_pose() {
        let (stamp, at) = deliver_old_follow(200);
        assert_eq!(stamp.age_ms, Some(200));
        assert!(at.1 > 0, "{at:?}");
        assert_eq!(at.0, 0, "{at:?}");
        let (stamp, at) = deliver_old_follow(1_000);
        assert_eq!(stamp.age_ms, Some(1_000));
        assert!(at.1 > 0, "{at:?}");
        assert_eq!(at.0, 0, "{at:?}");
    }

    #[test]
    fn a_held_bag_sample_is_approached_after_the_bag_moves() {
        use crate::point_from_sample;
        let mut world = SimWorld::from_seed(1);
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let other = world.observe().bags[1].id;
        world.set_visible(other, false).unwrap();
        world.place_free(bag, point(0, 3_000)).unwrap();
        let observed = world.observe();
        let origin = observed.operator.pose.point();
        let view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            observed.now_ms,
        );
        let chosen = point_from_sample(Some(origin), Some((0, 2_000)), &view.bags).unwrap();
        assert_eq!(chosen, bag);
        world.place_free(bag, point(4_000, 0)).unwrap();
        let fresh = sense(
            world.observe().robot.pose.point(),
            world.observe().operator,
            &world.observe().bags,
            &world.obstacles_for_planner(),
            false,
            world.now_ms(),
        );
        assert!(fresh.bags.iter().all(|item| item.id != bag));
        let posted = Posted::ready(view, world.now_ms());
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            100,
            Drive::Approach { bag },
            &posted,
            &mut history,
        )
        .unwrap();
        let at = pose(&world);
        assert_eq!(at.x().get(), 0, "{at:?}");
        assert!(at.y().get() > 0, "{at:?}");
        assert_eq!(
            world
                .observe()
                .bags
                .iter()
                .find(|item| item.id == bag)
                .unwrap()
                .pose
                .point(),
            point(4_000, 0)
        );
    }

    #[test]
    fn missing_and_stale_samples_do_not_chase_and_do_not_read_truth() {
        use navigation::SampleKind;
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        world.place_operator(point(1_500, 0));
        let posted = Posted::missing(0);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let end = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        assert_eq!(end, NavEnd::NeedFresh);
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(history.last_capture_ms(), None);
        assert_eq!(world.observe().operator.pose.point(), point(1_500, 0));

        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        world.advance(domain::Step::from_ms(1_001)).unwrap();
        trip.apply(1_001, Event::Tick).unwrap();
        world.place_operator(point(1_500, 0));
        let observed = world.observe();
        let mut stale = sense(
            point(0, 0),
            observed.operator,
            &observed.bags,
            &[],
            false,
            0,
        );
        let _ = observed;
        stale.captured_ms = 0;
        stale.people = vec![navigation::SeenPerson {
            id: operator,
            at: point(0, 1_000),
        }];
        let posted = Posted::ready(stale, 1_001);
        let (_history, taken, stamp) = navigation::accept(SampleHistory::new(), &posted, 1_001);
        assert_eq!(stamp.kind, SampleKind::Stale);
        assert_eq!(taken, navigation::Taken::NeedFresh);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let end = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            1_201,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        assert_eq!(end, NavEnd::NeedFresh);
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(history.last_capture_ms(), None);
    }

    #[test]
    fn a_reversed_sample_does_not_change_pose_custody_or_history() {
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        world.advance(domain::Step::from_ms(100)).unwrap();
        trip.apply(100, Event::Tick).unwrap();
        let observed = world.observe();
        let mut view = sense(
            observed.robot.pose.point(),
            observed.operator,
            &observed.bags,
            &world.obstacles_for_planner(),
            false,
            100,
        );
        view.captured_ms = 100;
        let posted = Posted::ready(view, 100);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            300,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        let pose_after = pose(&world);
        let custody = world.carried();
        assert_eq!(history.last_capture_ms(), Some(100));
        world.place_operator(point(1_500, 0));
        let mut older = sense(
            world.observe().robot.pose.point(),
            world.observe().operator,
            &world.observe().bags,
            &world.obstacles_for_planner(),
            false,
            40,
        );
        older.captured_ms = 40;
        let reversed = Posted::ready(older, world.now_ms());
        let end = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            500,
            Drive::Follow { person: operator },
            &reversed,
            &mut history,
        )
        .unwrap();
        assert_eq!(end, NavEnd::Waiting);
        assert_eq!(pose(&world), pose_after);
        assert_eq!(world.carried(), custody);
        assert_eq!(history.last_capture_ms(), Some(100));
        assert_eq!(world.now_ms(), 500);
    }

    #[test]
    fn the_episode_budget_reaches_five_minutes_with_a_bad_sample() {
        let mut world = SimWorld::from_seed(1);
        let (mut trip, operator) = following_trip(&world);
        let posted = Posted::missing(0);
        let mut history = SampleHistory::new();
        let mut follower = Follower::follow(operator);
        let end = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            LIMIT_MS,
            Drive::Follow { person: operator },
            &posted,
            &mut history,
        )
        .unwrap();
        assert_eq!(end, NavEnd::NeedFresh);
        assert_eq!(world.now_ms(), LIMIT_MS);
        assert_eq!(trip.now_ms(), Some(LIMIT_MS));
        assert_ne!(trip.phase(), Phase::TimeUp);
        assert_eq!(pose(&world), point(0, 0));
        let future = Posted::ready(
            {
                let mut view = sense(
                    point(0, 0),
                    world.observe().operator,
                    &world.observe().bags,
                    &[],
                    false,
                    0,
                );
                view.captured_ms = 9_000_000;
                view
            },
            LIMIT_MS,
        );
        let err = super::step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            LIMIT_MS + 1,
            Drive::Follow { person: operator },
            &future,
            &mut history,
        );
        assert_eq!(err, Err(FeedError::TimeUp));
        assert_eq!(world.now_ms(), LIMIT_MS + 1);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(history.last_capture_ms(), None);
    }

    #[test]
    fn the_synthetic_shift_does_not_write_truth_and_repeats() {
        use navigation::{SYNTHETIC_SHIFT_MM, shift_view};
        let world = SimWorld::from_seed(1);
        let before = world.observe();
        let view = sense(
            before.robot.pose.point(),
            before.operator,
            &before.bags,
            &world.obstacles_for_planner(),
            false,
            before.now_ms,
        );
        let once = shift_view(&view, SYNTHETIC_SHIFT_MM, -SYNTHETIC_SHIFT_MM).unwrap();
        let twice = shift_view(&view, SYNTHETIC_SHIFT_MM, -SYNTHETIC_SHIFT_MM).unwrap();
        assert_eq!(once, twice);
        assert_eq!(world.observe().robot, before.robot);
        assert_eq!(world.observe().bags, before.bags);
        let other = SimWorld::from_seed(7);
        let other_view = sense(
            other.observe().robot.pose.point(),
            other.observe().operator,
            &other.observe().bags,
            &other.obstacles_for_planner(),
            false,
            0,
        );
        let other_shift = shift_view(&other_view, SYNTHETIC_SHIFT_MM, -SYNTHETIC_SHIFT_MM).unwrap();
        assert_eq!(
            other_shift.people[0].at.x().get() - other_view.people[0].at.x().get(),
            SYNTHETIC_SHIFT_MM
        );
        let mut reached = SimWorld::from_seed(1);
        let bag = reached.observe().bags[0].id;
        reached.place_free(bag, point(2_496, 0)).unwrap();
        let seen = sense(
            reached.observe().robot.pose.point(),
            reached.observe().operator,
            &reached.observe().bags,
            &reached.obstacles_for_planner(),
            false,
            0,
        );
        let shifted = shift_view(&seen, SYNTHETIC_SHIFT_MM, 0).unwrap();
        let reported = shifted.bags.iter().find(|item| item.id == bag).unwrap();
        assert!(reported.pose.point().x().get() > manipulation::assume::REACH_MM);
        assert_eq!(reached.pick(bag, manipulation::assume::REACH_MM), Ok(()));
        assert_eq!(
            reached
                .observe()
                .bags
                .iter()
                .find(|item| item.id == bag)
                .unwrap()
                .pose
                .point(),
            reached.observe().robot.pose.point()
        );
    }

    #[test]
    fn an_unseen_operator_halo_is_not_selected_or_chased() {
        use crate::point_from_sample;
        use sim_runtime::ArmError;
        let mut world = SimWorld::from_seed(1);
        let operator = world.observe().operator.id;
        let bag = world.observe().bags[0].id;
        let other = world.observe().bags[1].id;
        assert_eq!(bag.get(), 3);
        world.place_operator(point(0, 20_000));
        world.place_free(bag, point(0, 19_000)).unwrap();
        world.place_free(other, point(50_000, 50_000)).unwrap();
        let obs = world.observe();
        let view = sense(
            obs.robot.pose.point(),
            obs.operator,
            &obs.bags,
            &obs.obstacles,
            true,
            obs.now_ms,
        );
        assert!(view.people.is_empty());
        assert!(!view.gesture);
        assert_eq!(cue_from(&view), Cue::None);
        assert!(view.bags.iter().all(|item| item.id != bag));
        assert!(point_from_sample(Some(point(0, 20_000)), Some((0, -1_000)), &view.bags).is_err());
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        trip.apply(0, Event::Pointed { operator, bag }).unwrap();
        trip.apply(0, Event::Named { bag }).unwrap();
        let mut follower = Follower::follow(operator);
        let err = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            100,
            Drive::Approach { bag },
            Cue::None,
        )
        .unwrap_err();
        assert_eq!(err, FeedError::Arm(GripError::Missing));
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(world.now_ms(), 0);
        assert_eq!(trip.now_ms(), Some(0));
        assert_eq!(trip.phase(), Phase::Picking);
        assert_eq!(
            world.pick(bag, manipulation::assume::REACH_MM),
            Err(ArmError::OutOfReach)
        );
        assert_eq!(pose(&world), point(0, 0));
        assert_eq!(
            world
                .observe()
                .bags
                .iter()
                .find(|item| item.id == bag)
                .unwrap()
                .pose
                .point(),
            point(0, 19_000)
        );
    }

    fn one_disk(class: ObstacleClass, x: i32, y: i32, radius: i32) -> Obstacle {
        Obstacle {
            id: Id::new(6).unwrap(),
            pose: Pose::new(point(x, y)),
            class,
            clearance: Mm::new(400),
            bodies: vec![domain::Disk {
                id: Id::new(6).unwrap(),
                center: point(x, y),
                radius: Mm::new(radius),
            }],
        }
    }

    #[test]
    fn an_unseen_disk_blocks_the_handoff_and_a_visible_disk_blocks_arrival() {
        let (mut world, mut trip, operator, bag) = ready();
        let mut follower = Follower::follow(operator);
        let moved = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            200,
            Drive::Follow { person: operator },
            Cue::None,
        )
        .unwrap();
        assert_eq!(moved, NavEnd::Moved { to: point(0, 200) });
        world.place_operator(point(0, 1400));
        world.set_obstacles(vec![one_disk(ObstacleClass::HardToSee, 0, 1060, 60)]);
        let cued = step_drive(
            &mut world,
            &mut trip,
            &mut follower,
            400,
            Drive::Follow { person: operator },
            Cue::TakeBag,
        )
        .unwrap();
        assert!(matches!(cued, NavEnd::Arrived { .. }));
        assert!(trip.arrived());
        assert!(!trip.main_complete());
        let before = (
            world.observe(),
            world.events().to_vec(),
            trip,
            world.now_ms(),
        );
        assert_eq!(
            execute_handoff(&mut world, &mut trip, 400, operator),
            Err(FeedError::Arm(GripError::Blocked))
        );
        assert_eq!(world.observe().robot, before.0.robot);
        assert_eq!(world.observe().bags, before.0.bags);
        assert_eq!(world.events(), before.1.as_slice());
        assert_eq!(trip.phase(), Phase::Releasing);
        assert!(!trip.main_complete());
        assert_eq!(trip.carried(), Some(bag));
        assert_eq!(world.carried(), Some(bag));
        assert_eq!(world.now_ms(), before.3);
        assert!(!world.handoff_fault());
        let held = world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id == bag)
            .unwrap();
        assert_eq!(held.custody, domain::Custody::Gripper);
        assert_eq!(held.pose.point(), point(0, 200));
        assert_ne!(held.pose.point(), point(0, 1400));

        let (mut seen, mut seen_trip, seen_operator, seen_bag) = ready();
        let mut seen_follower = Follower::follow(seen_operator);
        step_drive(
            &mut seen,
            &mut seen_trip,
            &mut seen_follower,
            200,
            Drive::Follow {
                person: seen_operator,
            },
            Cue::None,
        )
        .unwrap();
        seen.place_operator(point(0, 1400));
        seen.set_obstacles(vec![one_disk(ObstacleClass::SmallGround, 0, 1060, 60)]);
        let refused = step_drive(
            &mut seen,
            &mut seen_trip,
            &mut seen_follower,
            400,
            Drive::Follow {
                person: seen_operator,
            },
            Cue::TakeBag,
        )
        .unwrap();
        assert!(!matches!(refused, NavEnd::Arrived { .. }));
        assert!(!seen_trip.arrived());
        assert!(execute_handoff(&mut seen, &mut seen_trip, 400, seen_operator).is_err());
        assert!(!seen_trip.main_complete());
        assert_eq!(seen.carried(), Some(seen_bag));

        let (mut late, mut late_trip, late_operator, late_bag) = ready();
        late_trip.apply(10, Event::Arrived).unwrap();
        late.set_obstacles(vec![one_disk(ObstacleClass::HardToSee, 0, 500, 60)]);
        let mark = late.observe();
        assert_eq!(
            execute_handoff(&mut late, &mut late_trip, LIMIT_MS + 1, late_operator),
            Err(FeedError::TimeUp)
        );
        assert_eq!(late_trip.phase(), Phase::TimeUp);
        assert!(!late_trip.main_complete());
        assert_eq!(late.carried(), Some(late_bag));
        assert_eq!(late.observe().bags, mark.bags);
        assert_eq!(late.observe().robot, mark.robot);
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
