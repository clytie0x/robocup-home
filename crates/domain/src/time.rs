/// A requested simulation step. One count is one millisecond.
///
/// The millisecond unit is a simulation assumption. The professor PDF states a five-minute maximum and does not state this unit or which event starts the test clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step(u64);

impl Step {
    pub const fn from_ms(ms: u64) -> Self {
        Self(ms)
    }

    pub const fn ms(self) -> u64 {
        self.0
    }

    /// Rejects a negative count. Zero is a valid step.
    ///
    /// ```
    /// use domain::Step;
    /// assert!(Step::try_from_i64(-5).is_err());
    /// assert_eq!(Step::try_from_i64(0).unwrap().ms(), 0);
    /// ```
    pub fn try_from_i64(ms: i64) -> Result<Self, TimeError> {
        u64::try_from(ms).map(Self).map_err(|_| TimeError::Negative)
    }
}

/// Rejected simulation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    Negative,
    Overflow,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TimeError::Negative => "negative duration",
            TimeError::Overflow => "duration overflow",
        })
    }
}

impl std::error::Error for TimeError {}

#[cfg(test)]
mod tests {
    use super::{Step, TimeError};

    #[test]
    fn negative_counts_are_rejected_and_zero_is_kept() {
        assert_eq!(Step::try_from_i64(-1), Err(TimeError::Negative));
        assert_eq!(Step::try_from_i64(i64::MIN), Err(TimeError::Negative));
        assert_eq!(Step::try_from_i64(0).unwrap().ms(), 0);
        assert_eq!(Step::try_from_i64(i64::MAX).unwrap().ms(), i64::MAX as u64);
    }
}
