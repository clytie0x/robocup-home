//! Task 3.1 orchestration.
//!
//! Events are mocked observations and confirmations. Effects are commands for later
//! perception, manipulation, HRI, and navigation work. This crate does not implement
//! those capabilities, and it does not depend on `sim_runtime`. The sheet records
//! printed lines and leaves the seven open conflicts undecided. It does not award
//! a run total.
//!
//! ```
//! use domain::Id;
//! use task_3_1::{Event, Trip};
//! let operator = Id::new(2).unwrap();
//! let bag = Id::new(3).unwrap();
//! let mut trip = Trip::new();
//! trip.apply(0, Event::Begin).unwrap();
//! trip.apply(0, Event::Pointed { operator, bag }).unwrap();
//! assert!(!trip.main_complete());
//! assert!(trip.carried().is_none());
//! ```

mod sheet;
mod trip;

pub use sheet::{
    ABSENT_NOTICE_MIN, ABSENT_PROSE, ABSENT_SHEET, ALT_START_LINE, Award, BACK, BAR, CHEAT_POINTS,
    COME_BACK, CONTACT, CROWD, DROP, FOLLOW, Finding, FindingResult, HARD, HELP_CAP, Help,
    JURY_POINTS, NATURAL, NOT_TRYING, OPEN, OUTSTANDING, PICK, QUEUE, SEE, SMALL, TOTAL, UNNATURAL,
    absent_choice, alternative_start, apply_finding, bonus_line, drop_shares_cap, fair_play_list,
    finding_from_contact, finding_from_low_score, finding_from_trajectory, first_drop,
    follow_after_recovery, follow_withheld_without_return, help_points, help_total,
    interpret_start_signal, network_blocks_score, outstanding_is_automatic,
    partial_without_the_car, pick_line, positive_lines, positive_sum, second_drop,
    social_bonus_added,
};
pub use trip::{
    ALT_START, CLOCK_START, Effect, Event, LIMIT_MS, Outcome, Phase, Reject, Scope, Trip,
};
