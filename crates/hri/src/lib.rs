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

#[cfg(test)]
mod tests {
    use super::MockReport;
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
}
