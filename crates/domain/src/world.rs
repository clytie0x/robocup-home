use crate::{Id, Mm, Point, Step, TimeError};

/// Bag definition from section 2.1.5. The fixture records the definition. It does not simulate stiffness or weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BagKind {
    LightweightStiffVerticalHandles,
}

/// The four obstacle classes in section 3.1. Geometry, when present, is a model, not a printed size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObstacleClass {
    SmallGround,
    HardToSee,
    Crowd,
    Barriers,
}

/// Position of one body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pose {
    point: Point,
}

impl Pose {
    pub const fn new(point: Point) -> Self {
        Self { point }
    }

    pub const fn point(self) -> Point {
        self.point
    }
}

/// Robot or operator as seen by the task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actor {
    pub id: Id,
    pub pose: Pose,
}

/// Where a bag is allowed to be. A held bag is not free at its previous point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Custody {
    Free,
    Gripper,
    Operator,
}

/// A fixture bag. `visible` records the setup fact, not a perception result.
///
/// `custody` says who has the bag. It is not a stiffness measurement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bag {
    pub id: Id,
    pub pose: Pose,
    pub kind: BagKind,
    pub visible: bool,
    pub custody: Custody,
}

/// One disk. The radius is a model assumption, not a printed size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disk {
    pub id: Id,
    pub center: Point,
    pub radius: Mm,
}

/// One obstacle. `bodies` is the geometry. A crowd has one body per person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Obstacle {
    pub id: Id,
    pub pose: Pose,
    pub class: ObstacleClass,
    pub clearance: Mm,
    pub bodies: Vec<Disk>,
}

/// Task-facing snapshot. The car coordinate is not a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub robot: Actor,
    pub operator: Actor,
    pub bags: Vec<Bag>,
    pub obstacles: Vec<Obstacle>,
    pub now_ms: u64,
}

impl Observation {
    pub fn points(&self) -> impl Iterator<Item = Point> + '_ {
        std::iter::once(self.robot.pose.point())
            .chain(std::iter::once(self.operator.pose.point()))
            .chain(self.bags.iter().map(|bag| bag.pose.point()))
            .chain(self.obstacles.iter().map(|obstacle| obstacle.pose.point()))
            .chain(
                self.obstacles
                    .iter()
                    .flat_map(|obstacle| obstacle.bodies.iter().map(|body| body.center)),
            )
    }
}

/// Trace entry. Layout data here is task-visible. The car coordinate is not an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub at_ms: u64,
    pub kind: EventKind,
}

/// What an explicit world call recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    Layout {
        bag_east_mm: i32,
        bag_west_mm: i32,
        order: [ObstacleClass; 4],
    },
    Advanced {
        step_ms: u64,
    },
    Placed {
        bag: Id,
    },
    Grasped {
        bag: Id,
    },
    Dropped {
        bag: Id,
    },
    Handed {
        bag: Id,
        to: Id,
    },
    Slid {
        x_mm: i32,
        y_mm: i32,
    },
}

/// Simulator-free world port. `revealed_car` stays empty. The car coordinate is not a task field.
pub trait World {
    fn now_ms(&self) -> u64;
    fn advance(&mut self, step: Step) -> Result<(), TimeError>;
    fn observe(&self) -> Observation;
    fn revealed_car(&self) -> Option<Point>;
    fn events(&self) -> &[Event];
}

/// Arm port. Carrying a bag is reported here. Grasping is not this trait.
pub trait Arm {
    fn carried(&self) -> Option<Id>;
}

/// Base port. The reported pose is the task-visible robot pose.
pub trait Base {
    fn pose(&self) -> Pose;
}
