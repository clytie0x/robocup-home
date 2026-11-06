//! Shared contract between task logic and adapters.
//!
//! `sim_runtime` implements these traits. Task crates depend on this crate and not on the simulator.

mod geom;
mod id;
mod time;
mod world;

pub use geom::{GeomError, Mm, Point, within_reach};
pub use id::Id;
pub use time::{Step, TimeError};
pub use world::{
    Actor, Arm, Bag, BagKind, Base, Custody, Event, EventKind, Observation, Obstacle,
    ObstacleClass, Pose, World,
};
