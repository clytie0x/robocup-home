//! Pure section 3.1 trip.
//!
//! `Begin` arms the budget. Elapsed time is `now_ms - start_ms`. The printed maximum is 5 minutes,
//! stored as [`LIMIT_MS`] with the millisecond assumption from `domain::Step`. [`CLOCK_START`] names
//! that start event. It is not a printed start signal. [`ALT_START`] stays unresolved.
//! A stamp is inside the budget when elapsed is at most [`LIMIT_MS`]. A later stamp moves an
//! unfinished trip to [`Phase::TimeUp`] and does not apply the event. [`Phase::Complete`] stays complete.
//!
//! A rejection does not change the trip. These accepted events are the failure transitions:
//! `GraspFailed` of the pointed bag moves `Picking` to `PickFailed`.
//! `Grasped` of another bag moves `Picking` to `WrongBag`.
//! `Dropped` of the carried bag moves a holding phase to `Dropped`.
//! `OperatorLost` moves `Following` to `Lost`.
//! `OperatorFound` of that same operator moves `Lost` back to `Following` and does not emit a new follow command.
//! `HandoffFailed` moves `Releasing` to `HandoffFailed`.
//! `Received` of the carried bag by the operator moves `Releasing` or `HandoffFailed` to `Complete`.
//!
//! Optional re-entry and the queue are [`Reject::OutOfScope`]. They are checked before the clock.
//! The machine does not score, and it does not choose an open scoring row.

use domain::Id;

/// Printed 5 minutes, in the millisecond assumption. Inclusive upper bound.
pub const LIMIT_MS: u64 = 300_000;

/// Assumption label for the event that arms the budget. Not a printed start signal.
pub const CLOCK_START: &str = "begin";

/// The sheet line exists. This machine does not interpret it.
pub const ALT_START: &str = "unresolved";

/// Optional return and queue are not this machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    NotThisMachine,
}

/// One command for a later capability crate. Emitting it is not confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    ReportBag { bag: Id },
    Pick { bag: Id },
    SignalReady { to: Id },
    Follow { person: Id },
    Release { bag: Id, to: Id },
}

/// Mocked input. No variant carries a car coordinate or a score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Begin,
    Tick,
    Pointed { operator: Id, bag: Id },
    Named { bag: Id },
    Grasped { bag: Id },
    GraspFailed { bag: Id },
    ReadyTold,
    OperatorLost,
    OperatorFound { person: Id },
    Arrived,
    Dropped { bag: Id },
    HandoffFailed,
    Received { bag: Id, by: Id },
    Reenter,
    JoinQueue,
}

/// Why `apply` left the trip unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    OutOfOrder,
    Duplicate,
    Stale,
    WrongId,
    Role,
    Impossible,
    Regressed,
    OutOfScope,
}

impl std::fmt::Display for Reject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Reject::OutOfOrder => "out of order",
            Reject::Duplicate => "duplicate",
            Reject::Stale => "stale",
            Reject::WrongId => "wrong id",
            Reject::Role => "id role",
            Reject::Impossible => "impossible",
            Reject::Regressed => "clock regression",
            Reject::OutOfScope => "out of scope",
        })
    }
}

impl std::error::Error for Reject {}

/// Result of an accepted stamp. `effect` is the command, not a confirmed success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub effect: Option<Effect>,
}

/// Phase of one trip. Failure phases are explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    AwaitPoint,
    Naming,
    Picking,
    Signaling,
    Following,
    Lost,
    Releasing,
    HandoffFailed,
    WrongBag,
    PickFailed,
    Dropped,
    Complete,
    TimeUp,
}

/// Section 3.1 main trip. Fields stay private so callers use `apply`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trip {
    phase: Phase,
    operator: Option<Id>,
    pointed: Option<Id>,
    carried: Option<Id>,
    named: bool,
    ready: bool,
    arrived: bool,
    main_complete: bool,
    start_ms: Option<u64>,
    now_ms: Option<u64>,
}

impl Trip {
    pub const fn new() -> Self {
        Self {
            phase: Phase::Idle,
            operator: None,
            pointed: None,
            carried: None,
            named: false,
            ready: false,
            arrived: false,
            main_complete: false,
            start_ms: None,
            now_ms: None,
        }
    }

    pub const fn phase(self) -> Phase {
        self.phase
    }

    pub const fn carried(self) -> Option<Id> {
        self.carried
    }

    pub const fn named(self) -> bool {
        self.named
    }

    pub const fn ready(self) -> bool {
        self.ready
    }

    pub const fn arrived(self) -> bool {
        self.arrived
    }

    pub const fn main_complete(self) -> bool {
        self.main_complete
    }

    pub const fn now_ms(self) -> Option<u64> {
        self.now_ms
    }

    pub const fn start_ms(self) -> Option<u64> {
        self.start_ms
    }

    pub const fn optional_goals(self) -> Scope {
        Scope::NotThisMachine
    }

    /// Accepts `event` at `now_ms`, or rejects it and leaves `self` unchanged.
    pub fn apply(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let mut next = *self;
        let outcome = next.transition(now_ms, event)?;
        debug_assert!(next.consistent());
        *self = next;
        Ok(outcome)
    }

    fn transition(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        if matches!(event, Event::Reenter | Event::JoinQueue) {
            return Err(Reject::OutOfScope);
        }
        if let Some(prev) = self.now_ms
            && now_ms < prev
        {
            return Err(Reject::Regressed);
        }
        if let Some(start) = self.start_ms
            && self.phase != Phase::Complete
            && self.phase != Phase::TimeUp
        {
            match now_ms.checked_sub(start) {
                None => return Err(Reject::Regressed),
                Some(elapsed) if elapsed > LIMIT_MS => {
                    self.now_ms = Some(now_ms);
                    self.phase = Phase::TimeUp;
                    return Ok(Outcome { effect: None });
                }
                Some(_) => {}
            }
        }
        if self.phase == Phase::Complete || self.phase == Phase::TimeUp {
            if matches!(event, Event::Tick) {
                self.now_ms = Some(now_ms);
                return Ok(Outcome { effect: None });
            }
            return Err(Reject::Stale);
        }
        if matches!(event, Event::Tick) {
            if self.phase == Phase::Idle {
                return Err(Reject::OutOfOrder);
            }
            self.now_ms = Some(now_ms);
            return Ok(Outcome { effect: None });
        }
        if matches!(event, Event::Begin) {
            if self.phase != Phase::Idle {
                return Err(Reject::Duplicate);
            }
            self.start_ms = Some(now_ms);
            self.phase = Phase::AwaitPoint;
            self.now_ms = Some(now_ms);
            return Ok(Outcome { effect: None });
        }
        self.dispatch(now_ms, event)
    }

    fn dispatch(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        match self.phase {
            Phase::Idle => Err(Reject::OutOfOrder),
            Phase::AwaitPoint => self.on_await(now_ms, event),
            Phase::Naming => self.on_naming(now_ms, event),
            Phase::Picking => self.on_picking(now_ms, event),
            Phase::Signaling => self.on_signaling(now_ms, event),
            Phase::Following => self.on_following(now_ms, event),
            Phase::Lost => self.on_lost(now_ms, event),
            Phase::Releasing => self.on_releasing(now_ms, event),
            Phase::HandoffFailed => self.on_handoff_failed(now_ms, event),
            Phase::WrongBag => self.on_wrong(now_ms, event),
            Phase::PickFailed => self.on_pick_failed(event),
            Phase::Dropped => self.on_dropped(event),
            Phase::Complete | Phase::TimeUp => Err(Reject::Stale),
        }
    }

    fn on_await(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        match event {
            Event::Pointed { operator, bag } if operator == bag => Err(Reject::Role),
            Event::Pointed { operator, bag } => {
                self.operator = Some(operator);
                self.pointed = Some(bag);
                Ok(self.accept(now_ms, Phase::Naming, Some(Effect::ReportBag { bag })))
            }
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_naming(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let (operator, pointed) = self.pair()?;
        match event {
            Event::Named { bag } if bag == operator => Err(Reject::Role),
            Event::Named { bag } if bag != pointed => Err(Reject::WrongId),
            Event::Named { bag } => {
                self.named = true;
                Ok(self.accept(now_ms, Phase::Picking, Some(Effect::Pick { bag })))
            }
            Event::Pointed { .. } => Err(Reject::Stale),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_picking(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let (operator, pointed) = self.pair()?;
        match event {
            Event::Named { .. } => Err(Reject::Stale),
            Event::Grasped { bag } if bag == operator => Err(Reject::Role),
            Event::Grasped { bag } if bag == pointed => {
                self.carried = Some(bag);
                Ok(self.accept(
                    now_ms,
                    Phase::Signaling,
                    Some(Effect::SignalReady { to: operator }),
                ))
            }
            Event::Grasped { bag } => {
                self.carried = Some(bag);
                Ok(self.accept(now_ms, Phase::WrongBag, None))
            }
            Event::GraspFailed { bag } if bag == operator => Err(Reject::Role),
            Event::GraspFailed { bag } if bag != pointed => Err(Reject::WrongId),
            Event::GraspFailed { .. } => Ok(self.accept(now_ms, Phase::PickFailed, None)),
            Event::Dropped { .. } => Err(Reject::Impossible),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_signaling(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let (operator, _) = self.pair()?;
        match event {
            Event::ReadyTold => {
                self.ready = true;
                Ok(self.accept(
                    now_ms,
                    Phase::Following,
                    Some(Effect::Follow { person: operator }),
                ))
            }
            Event::Named { .. } | Event::Grasped { .. } => Err(Reject::Stale),
            Event::Dropped { bag } => self.drop_carried(now_ms, bag),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_following(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let (operator, pointed) = self.pair()?;
        match event {
            Event::ReadyTold | Event::Named { .. } | Event::Grasped { .. } => Err(Reject::Stale),
            Event::Arrived => {
                self.arrived = true;
                Ok(self.accept(
                    now_ms,
                    Phase::Releasing,
                    Some(Effect::Release {
                        bag: pointed,
                        to: operator,
                    }),
                ))
            }
            Event::OperatorLost => Ok(self.accept(now_ms, Phase::Lost, None)),
            Event::OperatorFound { .. } => Err(Reject::Impossible),
            Event::Dropped { bag } => self.drop_carried(now_ms, bag),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_lost(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let (operator, _) = self.pair()?;
        match event {
            Event::OperatorLost => Err(Reject::Duplicate),
            Event::OperatorFound { person } if person == operator => {
                Ok(self.accept(now_ms, Phase::Following, None))
            }
            Event::OperatorFound { person } if self.is_bag(person) => Err(Reject::Role),
            Event::OperatorFound { .. } => Err(Reject::WrongId),
            Event::Arrived => Err(Reject::OutOfOrder),
            Event::Dropped { bag } => self.drop_carried(now_ms, bag),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_releasing(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        match event {
            Event::Arrived | Event::ReadyTold | Event::Named { .. } | Event::Grasped { .. } => {
                Err(Reject::Stale)
            }
            Event::HandoffFailed => Ok(self.accept(now_ms, Phase::HandoffFailed, None)),
            Event::Dropped { bag } => self.drop_carried(now_ms, bag),
            Event::Received { bag, by } => self.receive(now_ms, bag, by),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_handoff_failed(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        match event {
            Event::HandoffFailed => Err(Reject::Duplicate),
            Event::Arrived | Event::ReadyTold | Event::Named { .. } | Event::Grasped { .. } => {
                Err(Reject::Stale)
            }
            Event::Dropped { bag } => self.drop_carried(now_ms, bag),
            Event::Received { bag, by } => self.receive(now_ms, bag, by),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_wrong(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        match event {
            Event::Dropped { bag } => self.drop_carried(now_ms, bag),
            Event::Grasped { .. } | Event::Named { .. } => Err(Reject::Stale),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_pick_failed(&mut self, event: Event) -> Result<Outcome, Reject> {
        match event {
            Event::Grasped { .. } | Event::Named { .. } => Err(Reject::Stale),
            Event::GraspFailed { .. } => Err(Reject::Duplicate),
            Event::Dropped { .. } => Err(Reject::Impossible),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn on_dropped(&mut self, event: Event) -> Result<Outcome, Reject> {
        match event {
            Event::Dropped { .. } => Err(Reject::Impossible),
            Event::Named { .. } | Event::Grasped { .. } => Err(Reject::Stale),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn drop_carried(&mut self, now_ms: u64, bag: Id) -> Result<Outcome, Reject> {
        if self.operator == Some(bag) {
            return Err(Reject::Role);
        }
        if self.carried != Some(bag) {
            return Err(Reject::WrongId);
        }
        self.carried = None;
        Ok(self.accept(now_ms, Phase::Dropped, None))
    }

    fn receive(&mut self, now_ms: u64, bag: Id, by: Id) -> Result<Outcome, Reject> {
        let (operator, pointed) = self.pair()?;
        if bag == operator || by == pointed || self.carried == Some(by) {
            return Err(Reject::Role);
        }
        if self.carried != Some(bag) || bag != pointed || by != operator {
            return Err(Reject::WrongId);
        }
        self.carried = None;
        self.main_complete = true;
        Ok(self.accept(now_ms, Phase::Complete, None))
    }

    fn accept(&mut self, now_ms: u64, phase: Phase, effect: Option<Effect>) -> Outcome {
        self.now_ms = Some(now_ms);
        self.phase = phase;
        Outcome { effect }
    }

    fn pair(self) -> Result<(Id, Id), Reject> {
        match (self.operator, self.pointed) {
            (Some(operator), Some(bag)) => Ok((operator, bag)),
            _ => Err(Reject::OutOfOrder),
        }
    }

    fn is_bag(self, id: Id) -> bool {
        self.pointed == Some(id) || self.carried == Some(id)
    }

    fn holding(self) -> bool {
        matches!(
            self.phase,
            Phase::Signaling
                | Phase::Following
                | Phase::Lost
                | Phase::Releasing
                | Phase::HandoffFailed
                | Phase::WrongBag
        )
    }

    fn consistent(self) -> bool {
        if self.main_complete != (self.phase == Phase::Complete) {
            return false;
        }
        if self.phase == Phase::Complete
            && (self.carried.is_some() || !self.named || !self.ready || !self.arrived)
        {
            return false;
        }
        if self.phase == Phase::TimeUp && self.main_complete {
            return false;
        }
        if (self.phase == Phase::Idle) != self.start_ms.is_none()
            || self.start_ms.is_none() != self.now_ms.is_none()
        {
            return false;
        }
        if self.phase != Phase::TimeUp && self.holding() != self.carried.is_some() {
            return false;
        }
        if self.phase == Phase::WrongBag && self.carried == self.pointed {
            return false;
        }
        if matches!(
            self.phase,
            Phase::Signaling
                | Phase::Following
                | Phase::Lost
                | Phase::Releasing
                | Phase::HandoffFailed
        ) && self.carried != self.pointed
        {
            return false;
        }
        true
    }
}

impl Default for Trip {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ids {
        operator: Id,
        bag: Id,
        other: Id,
        stranger: Id,
    }

    fn ids() -> Ids {
        Ids {
            operator: Id::new(2).unwrap(),
            bag: Id::new(3).unwrap(),
            other: Id::new(4).unwrap(),
            stranger: Id::new(5).unwrap(),
        }
    }

    fn keep(trip: &mut Trip, event: Event, reject: Reject) {
        let now = trip.now_ms().unwrap_or(0);
        let before = *trip;
        assert_eq!(trip.apply(now, event), Err(reject));
        assert_eq!(*trip, before);
    }

    fn begin() -> Trip {
        let mut trip = Trip::new();
        assert_eq!(trip.apply(0, Event::Begin).unwrap().effect, None);
        assert_eq!(trip.phase(), Phase::AwaitPoint);
        assert_eq!(trip.start_ms(), Some(0));
        trip
    }

    fn naming() -> Trip {
        let ids = ids();
        let mut trip = begin();
        let outcome = trip
            .apply(
                10,
                Event::Pointed {
                    operator: ids.operator,
                    bag: ids.bag,
                },
            )
            .unwrap();
        assert_eq!(outcome.effect, Some(Effect::ReportBag { bag: ids.bag }));
        assert!(!trip.named());
        trip
    }

    fn picking() -> Trip {
        let ids = ids();
        let mut trip = naming();
        let outcome = trip.apply(20, Event::Named { bag: ids.bag }).unwrap();
        assert_eq!(outcome.effect, Some(Effect::Pick { bag: ids.bag }));
        assert!(trip.named());
        assert_eq!(trip.carried(), None);
        trip
    }

    fn signaling() -> Trip {
        let ids = ids();
        let mut trip = picking();
        let outcome = trip.apply(30, Event::Grasped { bag: ids.bag }).unwrap();
        assert_eq!(
            outcome.effect,
            Some(Effect::SignalReady { to: ids.operator })
        );
        assert_eq!(trip.carried(), Some(ids.bag));
        assert!(!trip.ready());
        trip
    }

    fn following() -> Trip {
        let ids = ids();
        let mut trip = signaling();
        let outcome = trip.apply(40, Event::ReadyTold).unwrap();
        assert_eq!(
            outcome.effect,
            Some(Effect::Follow {
                person: ids.operator
            })
        );
        assert!(trip.ready());
        assert!(!trip.arrived());
        assert_ne!(ids.operator, ids.bag);
        trip
    }

    fn releasing() -> Trip {
        let ids = ids();
        let mut trip = following();
        let outcome = trip.apply(50, Event::Arrived).unwrap();
        assert_eq!(
            outcome.effect,
            Some(Effect::Release {
                bag: ids.bag,
                to: ids.operator
            })
        );
        assert!(trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(trip.carried(), Some(ids.bag));
        trip
    }

    #[test]
    fn new_trip_is_idle() {
        let trip = Trip::new();
        assert_eq!(trip.phase(), Phase::Idle);
        assert_eq!(trip.carried(), None);
        assert!(!trip.named());
        assert!(!trip.ready());
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(trip.now_ms(), None);
        assert_eq!(trip.start_ms(), None);
        assert_eq!(trip.optional_goals(), Scope::NotThisMachine);
        assert_eq!(LIMIT_MS, 300_000);
        assert_eq!(CLOCK_START, "begin");
        assert_eq!(ALT_START, "unresolved");
        assert_eq!(Id::new(0), None);
        assert_eq!(Trip::default(), trip);
    }

    #[test]
    fn main_trip_confirms_each_step_before_completion() {
        let ids = ids();
        let mut trip = releasing();
        assert_eq!(trip.phase(), Phase::Releasing);
        let done = trip
            .apply(
                60,
                Event::Received {
                    bag: ids.bag,
                    by: ids.operator,
                },
            )
            .unwrap();
        assert_eq!(done.effect, None);
        assert!(trip.main_complete());
        assert_eq!(trip.carried(), None);
        assert_eq!(trip.phase(), Phase::Complete);
        assert!(trip.named());
        assert!(trip.ready());
        assert!(trip.arrived());
    }

    #[test]
    fn the_same_script_replays() {
        fn script() -> (Trip, Vec<Result<Outcome, Reject>>) {
            let ids = ids();
            let mut trip = Trip::new();
            let events = [
                (0, Event::Begin),
                (
                    0,
                    Event::Pointed {
                        operator: ids.operator,
                        bag: ids.bag,
                    },
                ),
                (5, Event::Named { bag: ids.bag }),
                (4, Event::Grasped { bag: ids.bag }),
                (6, Event::Grasped { bag: ids.bag }),
                (7, Event::ReadyTold),
                (8, Event::OperatorLost),
                (
                    9,
                    Event::OperatorFound {
                        person: ids.operator,
                    },
                ),
                (10, Event::Arrived),
                (11, Event::HandoffFailed),
                (
                    12,
                    Event::Received {
                        bag: ids.bag,
                        by: ids.operator,
                    },
                ),
                (13, Event::Reenter),
            ];
            let mut log = Vec::new();
            for (now, event) in events {
                log.push(trip.apply(now, event));
            }
            (trip, log)
        }
        let (left, left_log) = script();
        let (right, right_log) = script();
        assert_eq!(left, right);
        assert_eq!(left_log, right_log);
        assert!(left.main_complete());
        assert_eq!(left.carried(), None);
        assert!(left_log[3].is_err());
        assert_eq!(left_log[11], Err(Reject::OutOfScope));
    }

    #[test]
    fn nothing_before_begin_moves_the_trip() {
        let ids = ids();
        let mut trip = Trip::new();
        let events = [
            Event::Tick,
            Event::Pointed {
                operator: ids.operator,
                bag: ids.bag,
            },
            Event::Named { bag: ids.bag },
            Event::Grasped { bag: ids.bag },
            Event::ReadyTold,
            Event::Arrived,
            Event::Received {
                bag: ids.bag,
                by: ids.operator,
            },
            Event::OperatorLost,
            Event::Dropped { bag: ids.bag },
        ];
        for event in events {
            keep(&mut trip, event, Reject::OutOfOrder);
        }
        assert_eq!(trip.phase(), Phase::Idle);
        assert_eq!(trip.apply(u64::MAX, Event::Tick), Err(Reject::OutOfOrder));
        assert_eq!(trip.now_ms(), None);
    }

    #[test]
    fn a_second_begin_is_a_duplicate() {
        let mut trip = begin();
        keep(&mut trip, Event::Begin, Reject::Duplicate);
        assert_eq!(trip.start_ms(), Some(0));
        assert_eq!(trip.phase(), Phase::AwaitPoint);
    }

    #[test]
    fn the_wrong_bag_is_not_identified_or_picked() {
        let ids = ids();
        let mut trip = naming();
        keep(&mut trip, Event::Named { bag: ids.other }, Reject::WrongId);
        assert!(!trip.named());
        assert_eq!(trip.phase(), Phase::Naming);
        let outcome = trip.apply(20, Event::Named { bag: ids.bag }).unwrap();
        assert_eq!(outcome.effect, Some(Effect::Pick { bag: ids.bag }));
        assert!(trip.named());
        assert_eq!(trip.carried(), None);
        assert_eq!(trip.phase(), Phase::Picking);
        assert!(!trip.ready());
    }

    #[test]
    fn a_person_id_is_not_used_as_a_bag() {
        let ids = ids();
        let mut trip = begin();
        keep(
            &mut trip,
            Event::Pointed {
                operator: ids.bag,
                bag: ids.bag,
            },
            Reject::Role,
        );
        trip.apply(
            10,
            Event::Pointed {
                operator: ids.operator,
                bag: ids.bag,
            },
        )
        .unwrap();
        keep(&mut trip, Event::Named { bag: ids.operator }, Reject::Role);
        trip.apply(20, Event::Named { bag: ids.bag }).unwrap();
        keep(
            &mut trip,
            Event::Grasped { bag: ids.operator },
            Reject::Role,
        );
        keep(
            &mut trip,
            Event::GraspFailed { bag: ids.operator },
            Reject::Role,
        );
        assert_eq!(trip.phase(), Phase::Picking);
        assert_eq!(trip.carried(), None);
    }

    #[test]
    fn a_bag_id_is_not_used_as_a_person() {
        let ids = ids();
        let mut trip = following();
        trip.apply(45, Event::OperatorLost).unwrap();
        keep(
            &mut trip,
            Event::OperatorFound { person: ids.bag },
            Reject::Role,
        );
        trip.apply(
            46,
            Event::OperatorFound {
                person: ids.operator,
            },
        )
        .unwrap();
        trip.apply(50, Event::Arrived).unwrap();
        keep(
            &mut trip,
            Event::Received {
                bag: ids.bag,
                by: ids.bag,
            },
            Reject::Role,
        );
        keep(
            &mut trip,
            Event::Received {
                bag: ids.operator,
                by: ids.operator,
            },
            Reject::Role,
        );
        assert!(!trip.main_complete());
        assert_eq!(trip.carried(), Some(ids.bag));
        assert_eq!(trip.phase(), Phase::Releasing);
    }

    #[test]
    fn a_wrong_grasp_is_not_ready() {
        let ids = ids();
        let mut trip = picking();
        let outcome = trip.apply(30, Event::Grasped { bag: ids.other }).unwrap();
        assert_eq!(outcome.effect, None);
        assert_eq!(trip.phase(), Phase::WrongBag);
        assert_eq!(trip.carried(), Some(ids.other));
        assert!(!trip.ready());
        assert!(!trip.main_complete());
        keep(&mut trip, Event::Dropped { bag: ids.bag }, Reject::WrongId);
        let dropped = trip.apply(31, Event::Dropped { bag: ids.other }).unwrap();
        assert_eq!(dropped.effect, None);
        assert_eq!(trip.phase(), Phase::Dropped);
        assert_eq!(trip.carried(), None);
        assert!(!trip.main_complete());
        keep(
            &mut trip,
            Event::Received {
                bag: ids.other,
                by: ids.operator,
            },
            Reject::OutOfOrder,
        );
    }

    #[test]
    fn a_failed_grasp_does_not_hold_the_bag() {
        let ids = ids();
        let mut trip = picking();
        keep(
            &mut trip,
            Event::GraspFailed { bag: ids.other },
            Reject::WrongId,
        );
        let outcome = trip.apply(30, Event::GraspFailed { bag: ids.bag }).unwrap();
        assert_eq!(outcome.effect, None);
        assert_eq!(trip.phase(), Phase::PickFailed);
        assert!(trip.named());
        assert_eq!(trip.carried(), None);
        assert!(!trip.ready());
        keep(&mut trip, Event::Grasped { bag: ids.bag }, Reject::Stale);
        keep(&mut trip, Event::ReadyTold, Reject::OutOfOrder);
        assert!(!trip.main_complete());
    }

    #[test]
    fn out_of_order_arrival_does_not_finish() {
        let mut naming = naming();
        keep(&mut naming, Event::Arrived, Reject::OutOfOrder);
        let mut picking = picking();
        keep(&mut picking, Event::ReadyTold, Reject::OutOfOrder);
        keep(&mut picking, Event::Arrived, Reject::OutOfOrder);
        let mut signaling = signaling();
        keep(&mut signaling, Event::Arrived, Reject::OutOfOrder);
        assert!(!naming.arrived());
        assert!(!picking.ready());
        assert!(!signaling.arrived());
        assert!(!signaling.main_complete());
    }

    #[test]
    fn stale_and_duplicate_confirmations_do_not_advance() {
        let ids = ids();
        let mut picking = picking();
        keep(&mut picking, Event::Named { bag: ids.bag }, Reject::Stale);
        let mut signaling = signaling();
        keep(
            &mut signaling,
            Event::Grasped { bag: ids.bag },
            Reject::Stale,
        );
        let mut walking = following();
        keep(&mut walking, Event::ReadyTold, Reject::Stale);
        let mut releasing = releasing();
        keep(&mut releasing, Event::Arrived, Reject::Stale);
        releasing.apply(55, Event::HandoffFailed).unwrap();
        keep(&mut releasing, Event::HandoffFailed, Reject::Duplicate);
        let mut lost = following();
        lost.apply(45, Event::OperatorLost).unwrap();
        keep(&mut lost, Event::OperatorLost, Reject::Duplicate);
        assert_eq!(lost.phase(), Phase::Lost);
        assert!(!lost.arrived());
    }

    #[test]
    fn a_drop_is_not_a_handoff() {
        let ids = ids();
        let mut following = following();
        let dropped = following
            .apply(45, Event::Dropped { bag: ids.bag })
            .unwrap();
        assert_eq!(dropped.effect, None);
        assert_eq!(following.phase(), Phase::Dropped);
        assert_eq!(following.carried(), None);
        assert!(!following.arrived());
        assert!(!following.main_complete());
        keep(
            &mut following,
            Event::Received {
                bag: ids.bag,
                by: ids.operator,
            },
            Reject::OutOfOrder,
        );

        let mut releasing = releasing();
        releasing
            .apply(55, Event::Dropped { bag: ids.bag })
            .unwrap();
        assert!(releasing.arrived());
        assert!(!releasing.main_complete());
        assert_eq!(releasing.carried(), None);
        assert_eq!(releasing.phase(), Phase::Dropped);
    }

    #[test]
    fn a_drop_with_nothing_carried_is_rejected() {
        let ids = ids();
        let mut trip = picking();
        keep(
            &mut trip,
            Event::Dropped { bag: ids.bag },
            Reject::Impossible,
        );
        let mut dropped = following();
        dropped.apply(45, Event::Dropped { bag: ids.bag }).unwrap();
        keep(
            &mut dropped,
            Event::Dropped { bag: ids.bag },
            Reject::Impossible,
        );
        assert_eq!(dropped.phase(), Phase::Dropped);
        assert_eq!(dropped.carried(), None);
    }

    #[test]
    fn loss_and_the_wrong_person_do_not_arrive() {
        let ids = ids();
        let mut trip = following();
        let lost = trip.apply(45, Event::OperatorLost).unwrap();
        assert_eq!(lost.effect, None);
        assert_eq!(trip.phase(), Phase::Lost);
        keep(&mut trip, Event::Arrived, Reject::OutOfOrder);
        keep(
            &mut trip,
            Event::OperatorFound {
                person: ids.stranger,
            },
            Reject::WrongId,
        );
        assert!(!trip.arrived());
        assert!(!trip.main_complete());
        assert_eq!(trip.carried(), Some(ids.bag));
    }

    #[test]
    fn finding_the_same_operator_resumes_following() {
        let ids = ids();
        let mut trip = following();
        trip.apply(45, Event::OperatorLost).unwrap();
        let found = trip
            .apply(
                46,
                Event::OperatorFound {
                    person: ids.operator,
                },
            )
            .unwrap();
        assert_eq!(found.effect, None);
        assert_eq!(trip.phase(), Phase::Following);
        assert!(!trip.arrived());
        trip.apply(50, Event::Arrived).unwrap();
        assert!(trip.arrived());
        assert!(!trip.main_complete());
    }

    #[test]
    fn a_failed_handoff_still_holds_the_bag() {
        let ids = ids();
        let mut trip = releasing();
        let failed = trip.apply(55, Event::HandoffFailed).unwrap();
        assert_eq!(failed.effect, None);
        assert_eq!(trip.phase(), Phase::HandoffFailed);
        assert!(trip.arrived());
        assert_eq!(trip.carried(), Some(ids.bag));
        assert!(!trip.main_complete());
        keep(
            &mut trip,
            Event::Received {
                bag: ids.other,
                by: ids.operator,
            },
            Reject::WrongId,
        );
        keep(
            &mut trip,
            Event::Received {
                bag: ids.bag,
                by: ids.stranger,
            },
            Reject::WrongId,
        );
        assert_eq!(trip.carried(), Some(ids.bag));
    }

    #[test]
    fn receipt_after_a_failed_handoff_completes_without_carrying() {
        let ids = ids();
        let mut trip = releasing();
        trip.apply(55, Event::HandoffFailed).unwrap();
        let done = trip
            .apply(
                56,
                Event::Received {
                    bag: ids.bag,
                    by: ids.operator,
                },
            )
            .unwrap();
        assert_eq!(done.effect, None);
        assert!(trip.main_complete());
        assert_eq!(trip.carried(), None);
        assert_eq!(trip.phase(), Phase::Complete);
        keep(&mut trip, Event::Grasped { bag: ids.bag }, Reject::Stale);
        assert_eq!(trip.carried(), None);
    }

    #[test]
    fn optional_events_are_out_of_scope() {
        let ids = ids();
        let mut samples = [Trip::new(), naming(), releasing()];
        for trip in &mut samples {
            let late = trip
                .now_ms()
                .map(|now| now.saturating_add(LIMIT_MS + 5))
                .unwrap_or(9);
            let before = *trip;
            assert_eq!(trip.apply(late, Event::Reenter), Err(Reject::OutOfScope));
            assert_eq!(trip.apply(0, Event::JoinQueue), Err(Reject::OutOfScope));
            assert_eq!(*trip, before);
            assert_eq!(trip.optional_goals(), Scope::NotThisMachine);
        }
        let mut done = releasing();
        done.apply(
            60,
            Event::Received {
                bag: ids.bag,
                by: ids.operator,
            },
        )
        .unwrap();
        let before = done;
        assert_eq!(done.apply(400_000, Event::Reenter), Err(Reject::OutOfScope));
        assert_eq!(
            done.apply(400_000, Event::JoinQueue),
            Err(Reject::OutOfScope)
        );
        assert_eq!(done, before);
        assert!(done.main_complete());
        assert_eq!(done.carried(), None);
    }

    #[test]
    fn clock_regression_is_rejected() {
        let ids = ids();
        let mut trip = naming();
        let before = trip;
        assert_eq!(
            trip.apply(9, Event::Named { bag: ids.bag }),
            Err(Reject::Regressed)
        );
        assert_eq!(trip, before);
        assert!(!trip.named());
        let outcome = trip.apply(10, Event::Named { bag: ids.bag }).unwrap();
        assert_eq!(outcome.effect, Some(Effect::Pick { bag: ids.bag }));
        assert!(trip.named());
    }

    #[test]
    fn the_budget_includes_the_limit_millisecond_and_stops_after_it() {
        let ids = ids();
        let start = 1_000u64;
        let mut on_time = Trip::new();
        on_time.apply(start, Event::Begin).unwrap();
        on_time
            .apply(
                start + 10,
                Event::Pointed {
                    operator: ids.operator,
                    bag: ids.bag,
                },
            )
            .unwrap();
        on_time
            .apply(start + 20, Event::Named { bag: ids.bag })
            .unwrap();
        on_time
            .apply(start + 30, Event::Grasped { bag: ids.bag })
            .unwrap();
        on_time.apply(start + 40, Event::ReadyTold).unwrap();
        on_time.apply(start + 50, Event::Arrived).unwrap();
        on_time
            .apply(
                start + LIMIT_MS,
                Event::Received {
                    bag: ids.bag,
                    by: ids.operator,
                },
            )
            .unwrap();
        assert!(on_time.main_complete());
        assert_eq!(on_time.phase(), Phase::Complete);
        assert_eq!(on_time.now_ms(), Some(start + LIMIT_MS));

        let mut late = Trip::new();
        late.apply(start, Event::Begin).unwrap();
        late.apply(
            start + 10,
            Event::Pointed {
                operator: ids.operator,
                bag: ids.bag,
            },
        )
        .unwrap();
        late.apply(start + 20, Event::Named { bag: ids.bag })
            .unwrap();
        late.apply(start + 30, Event::Grasped { bag: ids.bag })
            .unwrap();
        late.apply(start + 40, Event::ReadyTold).unwrap();
        late.apply(start + 50, Event::Arrived).unwrap();
        let outcome = late
            .apply(
                start + LIMIT_MS + 1,
                Event::Received {
                    bag: ids.bag,
                    by: ids.operator,
                },
            )
            .unwrap();
        assert_eq!(outcome.effect, None);
        assert!(!late.main_complete());
        assert_eq!(late.phase(), Phase::TimeUp);
        assert_eq!(late.carried(), Some(ids.bag));
        assert!(late.arrived());
        let before = late;
        assert_eq!(
            late.apply(
                start + LIMIT_MS,
                Event::Received {
                    bag: ids.bag,
                    by: ids.operator
                }
            ),
            Err(Reject::Regressed)
        );
        assert_eq!(late, before);

        let mut follow = Trip::new();
        follow.apply(start, Event::Begin).unwrap();
        follow
            .apply(
                start + 10,
                Event::Pointed {
                    operator: ids.operator,
                    bag: ids.bag,
                },
            )
            .unwrap();
        follow
            .apply(start + 20, Event::Named { bag: ids.bag })
            .unwrap();
        follow
            .apply(start + 30, Event::Grasped { bag: ids.bag })
            .unwrap();
        follow.apply(start + 40, Event::ReadyTold).unwrap();
        let blocked = follow.apply(start + LIMIT_MS + 1, Event::Arrived).unwrap();
        assert_eq!(blocked.effect, None);
        assert!(!follow.arrived());
        assert!(!follow.main_complete());
        assert_eq!(follow.phase(), Phase::TimeUp);
    }

    #[test]
    fn a_stamp_at_u64_max_does_not_wrap() {
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        let outcome = trip.apply(u64::MAX, Event::Tick).unwrap();
        assert_eq!(outcome.effect, None);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert!(!trip.main_complete());
        assert_eq!(trip.now_ms(), Some(u64::MAX));
        let again = trip.apply(u64::MAX, Event::Tick).unwrap();
        assert_eq!(again.effect, None);
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert_eq!(trip.now_ms(), Some(u64::MAX));
    }

    #[test]
    fn a_start_near_u64_max_stays_in_budget() {
        let ids = ids();
        let start = u64::MAX - 10;
        assert!(start.wrapping_add(LIMIT_MS) < start);
        let mut trip = Trip::new();
        trip.apply(start, Event::Begin).unwrap();
        let outcome = trip.apply(u64::MAX, Event::Tick).unwrap();
        assert_eq!(outcome.effect, None);
        assert_eq!(trip.phase(), Phase::AwaitPoint);
        assert_eq!(trip.now_ms(), Some(u64::MAX));
        let pointed = trip
            .apply(
                u64::MAX,
                Event::Pointed {
                    operator: ids.operator,
                    bag: ids.bag,
                },
            )
            .unwrap();
        assert_eq!(pointed.effect, Some(Effect::ReportBag { bag: ids.bag }));
        assert_eq!(trip.phase(), Phase::Naming);

        let mut top = Trip::new();
        top.apply(u64::MAX, Event::Begin).unwrap();
        top.apply(
            u64::MAX,
            Event::Pointed {
                operator: ids.operator,
                bag: ids.bag,
            },
        )
        .unwrap();
        assert_eq!(top.phase(), Phase::Naming);
        assert!(!top.main_complete());
    }

    #[test]
    fn stepped_timestamps_cross_the_limit_once() {
        let start = 1_000u64;
        let mut trip = Trip::new();
        trip.apply(start, Event::Begin).unwrap();
        for step in 1u64..=3 {
            trip.apply(start + step * 100_000, Event::Tick).unwrap();
            assert_eq!(trip.phase(), Phase::AwaitPoint);
        }
        assert_eq!(trip.now_ms(), Some(start + LIMIT_MS));
        trip.apply(start + LIMIT_MS + 1, Event::Tick).unwrap();
        assert_eq!(trip.phase(), Phase::TimeUp);
        assert!(!trip.main_complete());
        let before = trip;
        assert_eq!(
            trip.apply(
                start + LIMIT_MS + 2,
                Event::Pointed {
                    operator: ids().operator,
                    bag: ids().bag,
                }
            ),
            Err(Reject::Stale)
        );
        assert_eq!(trip, before);
    }

    #[test]
    fn time_after_complete_does_not_undo_the_handoff() {
        let ids = ids();
        let mut trip = releasing();
        trip.apply(
            60,
            Event::Received {
                bag: ids.bag,
                by: ids.operator,
            },
        )
        .unwrap();
        let outcome = trip.apply(400_000, Event::Tick).unwrap();
        assert_eq!(outcome.effect, None);
        assert_eq!(trip.phase(), Phase::Complete);
        assert!(trip.main_complete());
        assert_eq!(trip.carried(), None);
        assert_eq!(trip.now_ms(), Some(400_000));
        keep(&mut trip, Event::Grasped { bag: ids.bag }, Reject::Stale);
        keep(&mut trip, Event::Arrived, Reject::Stale);
        assert_eq!(trip.carried(), None);
        assert!(trip.main_complete());
    }

    #[test]
    fn every_rejection_leaves_the_previous_trip() {
        let ids = ids();
        let events = [
            Event::Begin,
            Event::Tick,
            Event::Pointed {
                operator: ids.operator,
                bag: ids.bag,
            },
            Event::Pointed {
                operator: ids.bag,
                bag: ids.bag,
            },
            Event::Named { bag: ids.bag },
            Event::Named { bag: ids.operator },
            Event::Named { bag: ids.other },
            Event::Grasped { bag: ids.bag },
            Event::Grasped { bag: ids.other },
            Event::Grasped { bag: ids.operator },
            Event::GraspFailed { bag: ids.bag },
            Event::GraspFailed { bag: ids.other },
            Event::ReadyTold,
            Event::OperatorLost,
            Event::OperatorFound {
                person: ids.operator,
            },
            Event::OperatorFound { person: ids.bag },
            Event::OperatorFound {
                person: ids.stranger,
            },
            Event::Arrived,
            Event::Dropped { bag: ids.bag },
            Event::Dropped { bag: ids.other },
            Event::Dropped { bag: ids.operator },
            Event::HandoffFailed,
            Event::Received {
                bag: ids.bag,
                by: ids.operator,
            },
            Event::Received {
                bag: ids.bag,
                by: ids.bag,
            },
            Event::Received {
                bag: ids.operator,
                by: ids.operator,
            },
            Event::Received {
                bag: ids.other,
                by: ids.operator,
            },
            Event::Reenter,
            Event::JoinQueue,
        ];
        let mut prefixes = vec![
            Trip::new(),
            begin(),
            naming(),
            picking(),
            following(),
            releasing(),
        ];
        let mut lost = following();
        lost.apply(45, Event::OperatorLost).unwrap();
        prefixes.push(lost);
        let mut failed = picking();
        failed
            .apply(30, Event::GraspFailed { bag: ids.bag })
            .unwrap();
        prefixes.push(failed);
        for mut trip in prefixes {
            for (index, event) in events.iter().enumerate() {
                let now = trip.now_ms().unwrap_or(0).saturating_add(index as u64);
                let before = trip;
                if trip.apply(now, *event).is_err() {
                    assert_eq!(trip, before);
                }
            }
        }
    }

    #[test]
    fn a_large_stamp_does_not_sleep() {
        let mut trip = Trip::new();
        trip.apply(0, Event::Begin).unwrap();
        let started = std::time::Instant::now();
        trip.apply(30_000, Event::Tick).unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert_eq!(trip.now_ms(), Some(30_000));
        assert_eq!(trip.phase(), Phase::AwaitPoint);
    }

    #[test]
    fn the_same_timestamp_is_not_a_regression() {
        let ids = ids();
        let mut trip = Trip::new();
        trip.apply(5, Event::Begin).unwrap();
        let outcome = trip
            .apply(
                5,
                Event::Pointed {
                    operator: ids.operator,
                    bag: ids.bag,
                },
            )
            .unwrap();
        assert_eq!(outcome.effect, Some(Effect::ReportBag { bag: ids.bag }));
        assert_eq!(trip.phase(), Phase::Naming);
        assert_eq!(trip.now_ms(), Some(5));
    }
}
