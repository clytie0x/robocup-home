//! Printed section 5.1 lines. A measured award is 0 when the run did not earn it.

use domain::Id;

/// Five minutes, in sim milliseconds.
pub const LIMIT_MS: u64 = 300_000;
pub const FOLLOW: i32 = 300;
pub const PERCEIVE: i32 = 15;
pub const PICK: i32 = 100;
pub const AVOID: i32 = 50;
pub const RETURN_START: i32 = 200;
pub const QUEUE: i32 = 300;
pub const DROP: i32 = -50;
pub const DEM_NATURAL: i32 = -50;
pub const DEM_UNNATURAL: i32 = -100;
pub const DEM_COME_BACK: i32 = -150;
pub const DEM_CONTACT: i32 = -200;
/// The rulebook caps repeated Deus ex Machina penalties at this magnitude.
pub const DEM_CAP: i32 = 250;
pub const NOT_ATTENDING: i32 = -500;
pub const ALT_START: i32 = -100;
pub const OUTSTANDING: i32 = 111;
pub const TOTAL: i32 = 1115;

/// Whether slice 1 measured the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The scenario decided an award, including a measured zero.
    Measured,
    /// The row is on the sheet and this slice does not run it.
    NotRun,
    /// The row is a referee or committee decision this sim does not make.
    OutOfSimScope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: &'static str,
    pub action: &'static str,
    pub catalog: i32,
    pub status: Status,
    pub awarded: i32,
}

/// What the scenario observed. Ids are the scripted bags, not a detector model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub followed_to_car: bool,
    pub perceived: Option<Id>,
    pub pointed: Id,
    pub assigned: Id,
    pub picked: Option<Id>,
    pub reached_start: bool,
    pub dropped: bool,
    pub timed_out: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sheet {
    rows: Vec<Row>,
}

impl Sheet {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn row(&self, id: &str) -> Option<&Row> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// The printed total. It is not the sum of a partial run.
    pub const fn printed_total(&self) -> i32 {
        TOTAL
    }

    /// Sum of awards this slice measured. Not-run rows stay out.
    pub fn measured_subtotal(&self) -> i32 {
        self.rows
            .iter()
            .filter(|row| row.status == Status::Measured && row.id != "HMC-TOTAL")
            .map(|row| row.awarded)
            .sum()
    }
}

/// Deus ex Machina total. Zero unless the robot scores again after the help.
///
/// `helps` are the negative catalog values. The result is not below `-DEM_CAP`.
/// Slice 1 does not apply this to a run.
pub fn dem_penalty(helps: &[i32], scored_afterwards: bool) -> i32 {
    if !scored_afterwards {
        return 0;
    }
    let sum: i32 = helps.iter().sum();
    sum.max(-DEM_CAP)
}

fn measured(id: &'static str, action: &'static str, catalog: i32, awarded: i32) -> Row {
    Row {
        id,
        action,
        catalog,
        status: Status::Measured,
        awarded,
    }
}

fn held(id: &'static str, action: &'static str, catalog: i32, status: Status) -> Row {
    Row {
        id,
        action,
        catalog,
        status,
        awarded: 0,
    }
}

/// Scores the rows slice 1 can measure. The other rows stay explicit.
pub fn score(facts: &Facts) -> Sheet {
    let correct = facts.pointed == facts.assigned;
    let perceived = facts.perceived == Some(facts.assigned) && correct;
    let picked = facts.picked == Some(facts.assigned) && correct;
    let rows = vec![
        measured(
            "HMC-FOLLOW",
            "Following the person to the car",
            FOLLOW,
            if facts.followed_to_car { FOLLOW } else { 0 },
        ),
        measured(
            "HMC-PERCEIVE",
            "Perceiving the correct beg (visualize on screen or say which one)",
            PERCEIVE,
            if perceived { PERCEIVE } else { 0 },
        ),
        measured(
            "HMC-PICK",
            "Picking up the correct bag",
            PICK,
            if picked { PICK } else { 0 },
        ),
        held(
            "HMC-CROWD",
            "Avoiding the crowd of people obstructing the path",
            AVOID,
            Status::NotRun,
        ),
        held(
            "HMC-SMALL",
            "Avoiding the small object on the ground",
            AVOID,
            Status::NotRun,
        ),
        held(
            "HMC-HARD",
            "Avoiding the hard-to-see object",
            AVOID,
            Status::NotRun,
        ),
        held(
            "HMC-BARRIER",
            "Avoiding the area blocked with retractable barriers",
            AVOID,
            Status::NotRun,
        ),
        measured(
            "HMC-RETURN",
            "Re-reaching the starting location",
            RETURN_START,
            if facts.reached_start { RETURN_START } else { 0 },
        ),
        held(
            "HMC-QUEUE",
            "Joining and staying in the queue on the way to the arena",
            QUEUE,
            Status::NotRun,
        ),
        measured(
            "HMC-DROP",
            "Dropping the bag",
            DROP,
            if facts.dropped { DROP } else { 0 },
        ),
        held(
            "HMC-DEM-NATURAL",
            "Rediscovering the operator by natural interaction",
            DEM_NATURAL,
            Status::NotRun,
        ),
        held(
            "HMC-DEM-UNNATURAL",
            "Rediscovering the operator by unnatural interaction",
            DEM_UNNATURAL,
            Status::NotRun,
        ),
        held(
            "HMC-DEM-BACK",
            "Rediscovering the operator by asking them to come back",
            DEM_COME_BACK,
            Status::NotRun,
        ),
        held(
            "HMC-DEM-CONTACT",
            "Rediscovering the operator by direct contact",
            DEM_CONTACT,
            Status::NotRun,
        ),
        held(
            "HMC-ABSENT",
            "Not attending (see sec. 3.9.1)",
            NOT_ATTENDING,
            Status::OutOfSimScope,
        ),
        held(
            "HMC-ALT",
            "Using alternative start signal (see sec. 3.7.8)",
            ALT_START,
            Status::NotRun,
        ),
        held(
            "HMC-OUTSTANDING",
            "Outstanding performance (see sec. 3.9.3)",
            OUTSTANDING,
            Status::OutOfSimScope,
        ),
        held(
            "HMC-TOTAL",
            "Total Score (excluding special penalties & standard bonuses)",
            TOTAL,
            Status::Measured,
        ),
    ];
    Sheet { rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag(raw: u32) -> Id {
        Id::new(raw).expect("id")
    }

    fn facts() -> Facts {
        let assigned = bag(2);
        Facts {
            followed_to_car: true,
            perceived: Some(assigned),
            pointed: assigned,
            assigned,
            picked: Some(assigned),
            reached_start: true,
            dropped: false,
            timed_out: false,
        }
    }

    #[test]
    fn printed_positive_lines_sum_to_1115() {
        let sum = FOLLOW + PERCEIVE + PICK + 4 * AVOID + RETURN_START + QUEUE;
        assert_eq!(sum, TOTAL);
        assert_eq!(score(&facts()).printed_total(), 1115);
    }

    #[test]
    fn a_correct_bag_scores_perceive_and_pick() {
        let sheet = score(&facts());
        assert_eq!(sheet.row("HMC-PERCEIVE").expect("row").awarded, 15);
        assert_eq!(sheet.row("HMC-PICK").expect("row").awarded, 100);
        assert_eq!(sheet.measured_subtotal(), 300 + 15 + 100 + 200);
    }

    #[test]
    fn the_wrong_pointed_bag_scores_neither_perceive_nor_pick() {
        let mut facts = facts();
        let other = bag(3);
        facts.pointed = other;
        facts.perceived = Some(other);
        facts.picked = Some(other);
        let sheet = score(&facts);
        assert_eq!(sheet.row("HMC-PERCEIVE").expect("row").awarded, 0);
        assert_eq!(sheet.row("HMC-PICK").expect("row").awarded, 0);
        assert_eq!(sheet.row("HMC-FOLLOW").expect("row").awarded, 300);
    }

    #[test]
    fn a_drop_is_minus_50_and_a_missed_return_is_zero() {
        let mut facts = facts();
        facts.dropped = true;
        facts.reached_start = false;
        let sheet = score(&facts);
        assert_eq!(sheet.row("HMC-DROP").expect("row").awarded, -50);
        assert_eq!(sheet.row("HMC-RETURN").expect("row").awarded, 0);
        assert_eq!(sheet.measured_subtotal(), 300 + 15 + 100 - 50);
    }

    #[test]
    fn timeout_keeps_earned_rows() {
        let mut facts = facts();
        facts.reached_start = false;
        facts.timed_out = true;
        let sheet = score(&facts);
        assert_eq!(sheet.row("HMC-FOLLOW").expect("row").awarded, 300);
        assert_eq!(sheet.row("HMC-RETURN").expect("row").awarded, 0);
        assert!(facts.timed_out);
    }

    #[test]
    fn obstacles_queue_dem_and_alt_start_stay_not_run() {
        let sheet = score(&facts());
        for id in [
            "HMC-CROWD",
            "HMC-SMALL",
            "HMC-HARD",
            "HMC-BARRIER",
            "HMC-QUEUE",
            "HMC-DEM-NATURAL",
            "HMC-DEM-UNNATURAL",
            "HMC-DEM-BACK",
            "HMC-DEM-CONTACT",
            "HMC-ALT",
        ] {
            let row = sheet.row(id).expect("row");
            assert_eq!(row.status, Status::NotRun);
            assert_eq!(row.awarded, 0);
        }
        assert_eq!(
            sheet.row("HMC-ABSENT").expect("row").status,
            Status::OutOfSimScope
        );
        assert_eq!(
            sheet.row("HMC-OUTSTANDING").expect("row").status,
            Status::OutOfSimScope
        );
    }

    #[test]
    fn dem_penalties_cap_at_250_and_only_after_later_points() {
        assert_eq!(dem_penalty(&[DEM_CONTACT, DEM_CONTACT], true), -DEM_CAP);
        assert_eq!(dem_penalty(&[DEM_NATURAL], true), DEM_NATURAL);
        assert_eq!(dem_penalty(&[DEM_CONTACT], false), 0);
        assert_eq!(sheet_dem_status(), Status::NotRun);
    }

    fn sheet_dem_status() -> Status {
        score(&facts()).row("HMC-DEM-CONTACT").expect("row").status
    }
}
