use domain::{Step, TimeError};

/// Virtual clock. `advance` does not sleep and does not read the wall clock.
///
/// ```
/// use domain::Step;
/// use sim_runtime::Clock;
/// let mut clock = Clock::new();
/// clock.advance(Step::from_ms(0)).unwrap();
/// assert_eq!(clock.now_ms(), 0);
/// clock.advance(Step::from_ms(5)).unwrap();
/// assert_eq!(clock.now_ms(), 5);
/// let mut clock = Clock::at(u64::MAX);
/// assert!(clock.advance(Step::from_ms(1)).is_err());
/// assert_eq!(clock.now_ms(), u64::MAX);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    now_ms: u64,
}

impl Clock {
    pub const fn new() -> Self {
        Self { now_ms: 0 }
    }

    pub const fn at(now_ms: u64) -> Self {
        Self { now_ms }
    }

    pub const fn now_ms(self) -> u64 {
        self.now_ms
    }

    pub fn advance(&mut self, step: Step) -> Result<(), TimeError> {
        self.now_ms = self
            .now_ms
            .checked_add(step.ms())
            .ok_or(TimeError::Overflow)?;
        Ok(())
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Clock;
    use domain::{Step, TimeError};

    #[test]
    fn zero_advance_keeps_the_time() {
        let mut clock = Clock::new();
        clock.advance(Step::from_ms(0)).unwrap();
        assert_eq!(clock.now_ms(), 0);
    }

    #[test]
    fn advance_adds_the_requested_step() {
        let mut clock = Clock::new();
        clock.advance(Step::from_ms(5)).unwrap();
        clock.advance(Step::from_ms(20)).unwrap();
        assert_eq!(clock.now_ms(), 25);
    }

    #[test]
    fn advance_can_land_on_the_maximum() {
        let mut clock = Clock::at(u64::MAX - 1);
        clock.advance(Step::from_ms(1)).unwrap();
        assert_eq!(clock.now_ms(), u64::MAX);
    }

    #[test]
    fn overflow_leaves_the_time_unchanged() {
        let mut clock = Clock::at(u64::MAX);
        assert_eq!(clock.advance(Step::from_ms(1)), Err(TimeError::Overflow));
        assert_eq!(clock.now_ms(), u64::MAX);
        assert_eq!(clock.advance(Step::from_ms(0)), Ok(()));
        assert_eq!(clock.now_ms(), u64::MAX);
    }

    #[test]
    fn a_long_step_does_not_sleep() {
        let mut clock = Clock::new();
        let started = std::time::Instant::now();
        clock.advance(Step::from_ms(30_000)).unwrap();
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        assert_eq!(clock.now_ms(), 30_000);
    }
}
