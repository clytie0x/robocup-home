//! Pointed-bag selection from a pointing sample and visible candidates.
//!
//! The professor PDF says the operator, standing in front of the robot, points at
//! one bag, and that at least two bags are within 2 m and visible. It states no
//! angle, sensor, tie rule, hand point, or bag width. Those choices live in
//! [`assume`] and are not printed rules.
//!
//! The selector never receives a referee id or any other hidden truth. A missing,
//! zero, reversed, hidden, or ambiguous sample does not become a unique bag.
//!
//! ```
//! use domain::{Id, Mm, Point};
//! use perception::{Candidate, select};
//!
//! let origin = Point::new(Mm::new(0), Mm::new(0));
//! let east = Candidate {
//!     id: Id::new(4).unwrap(),
//!     at: Point::new(Mm::new(1000), Mm::new(0)),
//!     visible: true,
//! };
//! let north = Candidate {
//!     id: Id::new(3).unwrap(),
//!     at: Point::new(Mm::new(0), Mm::new(1000)),
//!     visible: true,
//! };
//! let chosen = select(Some(origin), Some((1000, 0)), &[north, east]).unwrap();
//! assert_eq!(chosen, east.id);
//! ```

mod select;

pub mod assume {
    //! Selector stand-ins. No constant here is printed in the professor PDF.

    /// Half-width of the acceptance corridor around the pointing ray, in millimeters.
    ///
    /// A household pointing study reported about 9.6 cm of average position error.
    /// This fixture's nominal ray is the integer vector from the operator to the
    /// aimed bag, so the corridor is only how far a candidate may sit from that ray.
    /// It is not a measured error for this model. The test multiplies this width
    /// by `floor(hypot)` of the direction, so a short direction is a stricter
    /// tube than a longer writing of the same ray. That difference is not physical.
    pub const CORRIDOR_MM: i32 = 150;

    /// Perpendicular gap, in millimeters, that is still too small to be unique.
    ///
    /// Stored coordinates are integer millimeters. Moving one candidate by one
    /// millimeter changes its perpendicular distance by at most one millimeter, and
    /// both candidates can move, so a gap of this size or smaller stays ambiguous.
    pub const TIE_GAP_MM: i32 = 2;
}

pub use select::{Candidate, SelectError, select};
