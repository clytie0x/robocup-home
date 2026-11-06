//! Task 3.1 orchestration.
//!
//! Events are mocked observations and confirmations. Effects are commands for later
//! perception, manipulation, HRI, and navigation work. This crate does not implement
//! those capabilities, and it does not depend on `sim_runtime` or a scorer.
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

mod trip;

pub use trip::{
    ALT_START, CLOCK_START, Effect, Event, LIMIT_MS, Outcome, Phase, Reject, Scope, Trip,
};
