//! Seeded world and simulation clock.
//!
//! The clock moves only when a caller advances it. Lengths in [`assume`] are simulation assumptions unless the comment names a printed rule.

mod clock;
mod rng;
mod world;

pub mod assume {
    //! Model stand-ins. They are not official measurements unless the comment says the professor PDF states them.

    /// Printed bag limit is 2 m. This lower bound is not printed.
    pub const BAG_NEAR_MIN_MM: i32 = 400;
    /// Inclusive upper bound. 2000 mm is the printed "within 2 m".
    pub const BAG_NEAR_MAX_MM: i32 = 2000;
    pub const ROBOT_X_MM: i32 = 0;
    pub const ROBOT_Y_MM: i32 = 0;
    /// The operator stands in front of the robot. That distance is not printed.
    pub const OPERATOR_X_MM: i32 = 0;
    pub const OPERATOR_Y_MM: i32 = 1000;
    /// The inside stand-in is the half-plane `x` at or below this value. It is not a printed wall.
    pub const INSIDE_X_MAX_MM: i32 = 2500;
    pub const CAR_X_MM: i32 = 8000;
    pub const CAR_Y_MM: i32 = -1500;
    pub const OBSTACLE_X0_MM: i32 = 4000;
    pub const OBSTACLE_PITCH_MM: i32 = 700;
    pub const OBSTACLE_Y_MM: i32 = 0;
    /// Not a printed clearance.
    pub const CLEARANCE_MM: i32 = 400;
    /// xoshiro256 star-star 1.0 seeded by splitmix64 (2015). Not a cryptographic generator.
    pub const PRNG: &str = "xoshiro256**-1.0+splitmix64-2015";
    /// Layout consumption order for this generator.
    pub const LAYOUT: &str = "m3-layout-1";
    /// One clock count. The PDF does not define the unit.
    pub const TICK_UNIT: &str = "ms";
}

pub use clock::Clock;
pub use world::{SimArm, SimBase, SimWorld, Truth};
