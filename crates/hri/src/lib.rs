//! Human-robot interaction.
//!
//! [`MockReport`] is a stand-in for showing a bag on a screen or saying its id.
//! It stores the id it was given. It does not observe the arena, and it does not
//! replace that id with a referee assignment.

use domain::Id;

/// Mock screen or spoken report.
///
/// Confirmation is the id passed to [`MockReport::show`]. A fresh mock has not
/// confirmed anything.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MockReport {
    shown: Option<Id>,
}

impl MockReport {
    pub const fn new() -> Self {
        Self { shown: None }
    }

    pub const fn confirmed(self) -> Option<Id> {
        self.shown
    }

    /// Records `bag` as the report that was shown. This is not a perception choice.
    pub fn show(&mut self, bag: Id) {
        self.shown = Some(bag);
    }
}

/// Mock ready signal. Telling the operator is not implied by a grasp.
///
/// A fresh mock has not confirmed a recipient.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MockReady {
    told: Option<Id>,
}

impl MockReady {
    pub const fn new() -> Self {
        Self { told: None }
    }

    pub const fn confirmed(self) -> Option<Id> {
        self.told
    }

    /// Records the operator who was told. This is not a grasp and not a follow command.
    pub fn tell(&mut self, operator: Id) {
        self.told = Some(operator);
    }
}

#[cfg(test)]
mod tests {
    use super::{MockReady, MockReport};
    use domain::Id;

    #[test]
    fn the_mock_confirms_only_the_bag_it_was_shown() {
        let fresh = MockReport::new();
        assert_eq!(fresh.confirmed(), None);
        let mut report = fresh;
        let shown = Id::new(4).unwrap();
        let other = Id::new(3).unwrap();
        report.show(shown);
        assert_eq!(report.confirmed(), Some(shown));
        assert_ne!(report.confirmed(), Some(other));
        assert_eq!(fresh.confirmed(), None);
        let replaced = Id::new(9).unwrap();
        report.show(replaced);
        assert_eq!(report.confirmed(), Some(replaced));
        assert_ne!(report.confirmed(), Some(shown));
    }

    #[test]
    fn the_ready_mock_confirms_only_the_operator_it_was_told() {
        let fresh = MockReady::new();
        assert_eq!(fresh.confirmed(), None);
        let mut ready = fresh;
        let told = Id::new(2).unwrap();
        let other = Id::new(4).unwrap();
        ready.tell(told);
        assert_eq!(ready.confirmed(), Some(told));
        assert_ne!(ready.confirmed(), Some(other));
        assert_eq!(fresh.confirmed(), None);
    }
}
