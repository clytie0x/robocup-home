//! Section 3.2 instruction cycle.
//!
//! `DoorOpen` emits [`Effect::GoToPoint`]. The announced point stays in the harness.
//! This crate does not parse a command and does not depend on `sim_runtime`.
//!
//! ```
//! use task_3_2::{Effect, Event, Phase, Visit};
//! let mut visit = Visit::new();
//! let outcome = visit.apply(0, Event::DoorOpen).unwrap();
//! assert_eq!(outcome.effect, Some(Effect::GoToPoint));
//! assert_eq!(visit.phase(), Phase::GoingToPoint);
//! ```

/// Where the robot is in the instruction cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Outside,
    GoingToPoint,
    AtPoint,
}

/// Mocked input. No variant carries a coordinate or a score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    DoorOpen,
    Arrived,
    Tick,
}

/// Command for the adapter. Emitting it is not arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    GoToPoint,
}

/// Why `apply` left the visit unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    OutOfOrder,
    Duplicate,
}

impl std::fmt::Display for Reject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Reject::OutOfOrder => "out of order",
            Reject::Duplicate => "duplicate",
        })
    }
}

impl std::error::Error for Reject {}

/// Result of an accepted stamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub effect: Option<Effect>,
}

/// Section 3.2 start. Fields stay private so callers use `apply`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visit {
    phase: Phase,
    now_ms: Option<u64>,
}

impl Default for Visit {
    fn default() -> Self {
        Self::new()
    }
}

impl Visit {
    pub const fn new() -> Self {
        Self {
            phase: Phase::Outside,
            now_ms: None,
        }
    }

    pub const fn phase(self) -> Phase {
        self.phase
    }

    pub const fn now_ms(self) -> Option<u64> {
        self.now_ms
    }

    pub fn apply(&mut self, now_ms: u64, event: Event) -> Result<Outcome, Reject> {
        let stamp = domain::Step::from_ms(now_ms).ms();
        let (phase, effect) = match (self.phase, event) {
            (Phase::Outside, Event::DoorOpen) => (Phase::GoingToPoint, Some(Effect::GoToPoint)),
            (Phase::GoingToPoint, Event::Arrived) => (Phase::AtPoint, None),
            (_, Event::Tick) => (self.phase, None),
            (Phase::GoingToPoint, Event::DoorOpen) | (Phase::AtPoint, Event::Arrived) => {
                return Err(Reject::Duplicate);
            }
            _ => return Err(Reject::OutOfOrder),
        };
        self.phase = phase;
        self.now_ms = Some(stamp);
        Ok(Outcome { effect })
    }
}

#[cfg(test)]
mod tests {
    use super::{Effect, Event, Phase, Reject, Visit};

    #[test]
    fn door_open_sends_the_robot_toward_the_instruction_point() {
        let mut visit = Visit::new();
        assert_eq!(visit.phase(), Phase::Outside);
        let outcome = visit.apply(0, Event::DoorOpen).unwrap();
        assert_eq!(outcome.effect, Some(Effect::GoToPoint));
        assert_eq!(visit.phase(), Phase::GoingToPoint);
        assert_eq!(visit.now_ms(), Some(0));
    }

    #[test]
    fn arrival_before_the_door_leaves_the_phase_outside() {
        let mut visit = Visit::new();
        assert_eq!(visit.apply(0, Event::Arrived), Err(Reject::OutOfOrder));
        assert_eq!(visit.phase(), Phase::Outside);
        assert_eq!(visit.now_ms(), None);
    }

    #[test]
    fn arrival_ends_the_move_and_a_later_door_is_out_of_order() {
        let mut visit = Visit::new();
        visit.apply(0, Event::DoorOpen).unwrap();
        let outcome = visit.apply(10, Event::Arrived).unwrap();
        assert_eq!(outcome.effect, None);
        assert_eq!(visit.phase(), Phase::AtPoint);
        assert_eq!(visit.apply(11, Event::DoorOpen), Err(Reject::OutOfOrder));
        assert_eq!(visit.phase(), Phase::AtPoint);
        assert_eq!(visit.now_ms(), Some(10));
    }
}
