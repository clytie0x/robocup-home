//! Help Me Carry sheet from the 2025.1 main-league rulebook, section 5.1.
//!
//! The nine positive lines sum to the printed total 1115. This crate records
//! that sheet. It does not follow, pick, or move a base.
//!
//! ```
//! use domain::Id;
//! use task_5_1::{Facts, score};
//! let assigned = Id::new(2).unwrap();
//! let facts = Facts {
//!     followed_to_car: false,
//!     perceived: None,
//!     pointed: assigned,
//!     assigned,
//!     picked: None,
//!     reached_start: false,
//!     dropped: false,
//!     timed_out: false,
//! };
//! let sheet = score(&facts);
//! assert_eq!(sheet.printed_total(), 1115);
//! assert_eq!(sheet.measured_subtotal(), 0);
//! ```

mod sheet;

pub use sheet::{
    ALT_START, AVOID, DEM_CAP, DEM_COME_BACK, DEM_CONTACT, DEM_NATURAL, DEM_UNNATURAL, DROP,
    FOLLOW, Facts, LIMIT_MS, NOT_ATTENDING, OUTSTANDING, PERCEIVE, PICK, QUEUE, RETURN_START, Row,
    Sheet, Status, TOTAL, dem_penalty, score,
};
