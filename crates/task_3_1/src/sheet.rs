//! Printed section 3.1 lines.
//!
//! These values are the sheet and the clauses cited by the task. An [`Award::Undecided`]
//! line has two readings in the professor PDF. This module does not pick one and it
//! does not add a run total.

/// A printed value, or a conflict this module does not settle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Award {
    Points(i32),
    Undecided,
}

/// Named operator-rediscovery help. The point values are the printed examples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Help {
    Natural,
    Unnatural,
    ComeBack,
    Contact,
}

/// A recorded administration finding. The trajectory does not create one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finding {
    Cheat,
    NotTrying,
    MajorCollision,
    MissingJury,
    JuryMissedPerformance,
}

/// What a recorded finding does. Contact in the simulator is not one of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingResult {
    Disqualified { points: i32 },
    Penalty(i32),
    ScoreZero,
}

pub const SEE: i32 = 15;
pub const PICK: i32 = 100;
pub const FOLLOW: i32 = 300;
pub const CROWD: i32 = 50;
pub const SMALL: i32 = 50;
pub const HARD: i32 = 50;
pub const BAR: i32 = 50;
pub const BACK: i32 = 100;
pub const QUEUE: i32 = 300;
pub const TOTAL: i32 = 1015;
pub const OUTSTANDING: i32 = 101;
pub const DROP: i32 = -50;
pub const NATURAL: i32 = -50;
pub const UNNATURAL: i32 = -100;
pub const COME_BACK: i32 = -150;
pub const CONTACT: i32 = -200;
pub const HELP_CAP: i32 = 250;
pub const ABSENT_PROSE: i32 = 250;
pub const ABSENT_NOTICE_MIN: i32 = 60;
pub const ABSENT_SHEET: i32 = -500;
pub const ALT_START_LINE: i32 = -100;
pub const CHEAT_POINTS: i32 = -500;
pub const NOT_TRYING: i32 = -250;
pub const JURY_POINTS: i32 = -250;

pub const OPEN: [&str; 7] = [
    "CML-GEN-MAIN",
    "CML-SHEET-START",
    "CML-OPEN-PARTIAL",
    "CML-OPEN-ABSENT",
    "CML-OPEN-CAP",
    "CML-OPEN-DROP",
    "CML-OPEN-FOLLOW",
];

/// The nine positive lines, in sheet order.
pub const fn positive_lines() -> [(&'static str, i32); 9] {
    [
        ("see", SEE),
        ("pick", PICK),
        ("follow", FOLLOW),
        ("crowd", CROWD),
        ("small", SMALL),
        ("hard", HARD),
        ("barrier", BAR),
        ("reenter", BACK),
        ("queue", QUEUE),
    ]
}

pub const fn positive_sum() -> i32 {
    SEE + PICK + FOLLOW + CROWD + SMALL + HARD + BAR + BACK + QUEUE
}

pub const fn outstanding_is_automatic() -> bool {
    false
}

pub const fn help_points(help: Help) -> i32 {
    match help {
        Help::Natural => NATURAL,
        Help::Unnatural => UNNATURAL,
        Help::ComeBack => COME_BACK,
        Help::Contact => CONTACT,
    }
}

/// Named rediscovery penalties, capped at 250 points of magnitude.
///
/// `scored_after` is false when the run scores nothing later. A drop is not an
/// argument: whether a drop shares the cap is [`Award::Undecided`].
pub fn help_total(events: &[(Help, bool)]) -> i32 {
    let mut magnitude = 0i32;
    for (help, scored_after) in events {
        if *scored_after {
            magnitude = magnitude.saturating_add(help_points(*help).saturating_abs());
        }
    }
    -magnitude.min(HELP_CAP)
}

pub const fn first_drop() -> i32 {
    DROP
}

pub const fn second_drop() -> Award {
    Award::Undecided
}

pub const fn drop_shares_cap() -> Award {
    Award::Undecided
}

pub const fn partial_without_the_car() -> Award {
    Award::Undecided
}

pub const fn alternative_start() -> Award {
    Award::Undecided
}

pub const fn absent_choice() -> Award {
    Award::Undecided
}

pub const fn follow_after_recovery() -> Award {
    Award::Undecided
}

/// A spoken word, a door, or a button is not defined as this line.
pub const fn interpret_start_signal(_name: &str) -> Award {
    Award::Undecided
}

pub const fn pick_line(robot_grasped_pointed: bool, human_carried: bool) -> i32 {
    if human_carried || !robot_grasped_pointed {
        0
    } else {
        PICK
    }
}

pub const fn bonus_line(line: i32, help_used: bool) -> i32 {
    if help_used { 0 } else { line }
}

pub const fn follow_withheld_without_return() -> bool {
    false
}

pub const fn network_blocks_score(_down: bool) -> bool {
    false
}

pub const fn social_bonus_added() -> i32 {
    0
}

pub const fn finding_from_contact(_contact: bool) -> Option<Finding> {
    None
}

pub const fn finding_from_low_score(_score: i32) -> Option<Finding> {
    None
}

pub const fn finding_from_trajectory() -> Option<Finding> {
    None
}

pub const fn fair_play_list() -> Option<&'static str> {
    None
}

pub const fn apply_finding(finding: Finding) -> FindingResult {
    match finding {
        Finding::Cheat => FindingResult::Disqualified {
            points: CHEAT_POINTS,
        },
        Finding::NotTrying => FindingResult::Penalty(NOT_TRYING),
        Finding::MajorCollision => FindingResult::ScoreZero,
        Finding::MissingJury => FindingResult::Penalty(JURY_POINTS),
        Finding::JuryMissedPerformance => FindingResult::ScoreZero,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_lines_sum_to_the_printed_total() {
        assert_eq!(positive_sum(), TOTAL);
        assert_eq!(positive_sum(), 1015);
        assert_eq!(positive_lines().len(), 9);
        assert_eq!(OUTSTANDING, 101);
        assert!(!outstanding_is_automatic());
        let tenth = TOTAL * 10 / 100;
        assert_eq!(tenth, OUTSTANDING);
        assert_ne!(TOTAL * 10, OUTSTANDING * 100);
    }

    #[test]
    fn six_natural_rediscoveries_stop_at_250() {
        let events = [(Help::Natural, true); 6];
        assert_eq!(help_total(&events), -HELP_CAP);
        assert_ne!(help_total(&events), 6 * NATURAL);
        assert_eq!(help_total(&[(Help::Natural, false); 6]), 0);
        let mixed = [
            (Help::Natural, true),
            (Help::Unnatural, true),
            (Help::ComeBack, true),
            (Help::Contact, true),
        ];
        assert_eq!(help_total(&mixed), -HELP_CAP);
        assert_eq!(help_points(Help::Natural), -50);
        assert_eq!(help_points(Help::Unnatural), -100);
        assert_eq!(help_points(Help::ComeBack), -150);
        assert_eq!(help_points(Help::Contact), -200);
    }
}
