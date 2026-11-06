use domain::{
    Actor, Arm, Bag, BagKind, Base, Custody, Disk, Event, EventKind, Id, Mm, Observation, Obstacle,
    ObstacleClass, Point, Pose, Step, TimeError, World, segment_within, within_reach,
};

use crate::aim::{AimError, AimSample, direction_mm};
use crate::assume;
use crate::clock::Clock;
use crate::rng::Rng;

/// Hidden truth. Task code uses the `World` trait and does not receive this type from that trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Truth {
    car: Point,
    car_id: Id,
}

impl Truth {
    pub const fn car(self) -> Point {
        self.car
    }

    pub const fn car_id(self) -> Id {
        self.car_id
    }
}

/// Seeded fixture. The seed is consumed while the layout is built.
#[derive(Clone)]
pub struct SimWorld {
    clock: Clock,
    robot: Actor,
    operator: Actor,
    bags: Vec<Bag>,
    obstacles: Vec<Obstacle>,
    carried: Option<Id>,
    grasp_fault: bool,
    handoff_fault: bool,
    /// When set, slides do not block on this class and the follower is not shown it.
    /// The disks stay in the world, so a later live hit test can still meet them.
    collision_fault: Option<ObstacleClass>,
    car: Point,
    car_id: Id,
    events: Vec<Event>,
}

fn id(raw: u32) -> Id {
    Id::new(raw).expect("nonzero id")
}

fn bodies_for(class: ObstacleClass, x: i32, y: i32, crowd_n: usize) -> Vec<Disk> {
    match class {
        ObstacleClass::SmallGround => vec![disk(5, x, y, assume::SMALL_RADIUS_MM)],
        ObstacleClass::HardToSee => vec![disk(6, x, y, assume::HARD_RADIUS_MM)],
        ObstacleClass::Barriers => vec![disk(8, x, y, assume::BARRIER_RADIUS_MM)],
        ObstacleClass::Crowd => {
            let count = crowd_n.clamp(3, 4);
            let pitch = assume::CROWD_PITCH_MM;
            let span = i32::try_from(count - 1).expect("count") * pitch;
            (0..count)
                .map(|index| {
                    let offset = i32::try_from(index).expect("index") * pitch - span / 2;
                    disk(
                        10 + u32::try_from(index).expect("index"),
                        x,
                        y + offset,
                        assume::PERSON_RADIUS_MM,
                    )
                })
                .collect()
        }
    }
}

fn disk(raw: u32, x: i32, y: i32, radius: i32) -> Disk {
    Disk {
        id: id(raw),
        center: Point::new(Mm::new(x), Mm::new(y)),
        radius: Mm::new(radius),
    }
}

fn class_id(class: ObstacleClass) -> Id {
    id(match class {
        ObstacleClass::SmallGround => 5,
        ObstacleClass::HardToSee => 6,
        ObstacleClass::Crowd => 7,
        ObstacleClass::Barriers => 8,
    })
}

impl SimWorld {
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = Rng::from_seed(seed);
        let east = rng.uniform_i32(assume::BAG_NEAR_MIN_MM, assume::BAG_NEAR_MAX_MM);
        let west = rng.uniform_i32(assume::BAG_NEAR_MIN_MM, assume::BAG_NEAR_MAX_MM);
        let mut order = [
            ObstacleClass::SmallGround,
            ObstacleClass::HardToSee,
            ObstacleClass::Crowd,
            ObstacleClass::Barriers,
        ];
        // Durstenfeld shuffle, high index down to 1.
        for index in (1..order.len()).rev() {
            let pick = usize::try_from(rng.uniform(u64::try_from(index).expect("index") + 1))
                .expect("pick");
            order.swap(index, pick);
        }
        let robot = Actor {
            id: id(1),
            pose: Pose::new(Point::new(
                Mm::new(assume::ROBOT_X_MM),
                Mm::new(assume::ROBOT_Y_MM),
            )),
        };
        let operator = Actor {
            id: id(2),
            pose: Pose::new(Point::new(
                Mm::new(assume::OPERATOR_X_MM),
                Mm::new(assume::OPERATOR_Y_MM),
            )),
        };
        let bags = vec![
            Bag {
                id: id(3),
                pose: Pose::new(Point::new(Mm::new(east), Mm::new(assume::OPERATOR_Y_MM))),
                kind: BagKind::LightweightStiffVerticalHandles,
                visible: true,
                custody: domain::Custody::Free,
            },
            Bag {
                id: id(4),
                pose: Pose::new(Point::new(Mm::new(-west), Mm::new(assume::OPERATOR_Y_MM))),
                kind: BagKind::LightweightStiffVerticalHandles,
                visible: true,
                custody: domain::Custody::Free,
            },
        ];
        let crowd_n = 3 + usize::try_from(seed % 2).expect("bit");
        let obstacles = order
            .iter()
            .enumerate()
            .map(|(index, class)| {
                let x = assume::OBSTACLE_X0_MM
                    + i32::try_from(index).expect("index") * assume::OBSTACLE_PITCH_MM;
                Obstacle {
                    id: class_id(*class),
                    pose: Pose::new(Point::new(Mm::new(x), Mm::new(assume::OBSTACLE_Y_MM))),
                    class: *class,
                    clearance: Mm::new(assume::CLEARANCE_MM),
                    bodies: bodies_for(*class, x, assume::OBSTACLE_Y_MM, crowd_n),
                }
            })
            .collect();
        Self {
            clock: Clock::new(),
            robot,
            operator,
            bags,
            obstacles,
            carried: None,
            grasp_fault: false,
            handoff_fault: false,
            collision_fault: None,
            car: Point::new(Mm::new(assume::CAR_X_MM), Mm::new(assume::CAR_Y_MM)),
            car_id: id(9),
            events: vec![Event {
                at_ms: 0,
                kind: EventKind::Layout {
                    bag_east_mm: east,
                    bag_west_mm: west,
                    order,
                },
            }],
        }
    }

    pub fn truth(&self) -> Truth {
        Truth {
            car: self.car,
            car_id: self.car_id,
        }
    }

    /// Pointing sample from the operator toward `bag`.
    ///
    /// The sample is the integer offset of that bag. It does not contain the bag
    /// id. The caller that knows which bag the referee assigned keeps that id.
    pub fn aim_at(&self, bag: Id) -> Result<AimSample, AimError> {
        let Some(target) = self.bags.iter().find(|item| item.id == bag) else {
            return Err(AimError::UnknownBag);
        };
        let origin = self.operator.pose.point();
        let (dx, dy) = direction_mm(origin, target.pose.point())?;
        Ok(AimSample::new(origin, dx, dy))
    }

    pub const fn carried(&self) -> Option<Id> {
        self.carried
    }

    pub const fn grasp_fault(&self) -> bool {
        self.grasp_fault
    }

    pub const fn handoff_fault(&self) -> bool {
        self.handoff_fault
    }

    pub const fn set_grasp_fault(&mut self, on: bool) {
        self.grasp_fault = on;
    }

    pub const fn set_handoff_fault(&mut self, on: bool) {
        self.handoff_fault = on;
    }

    /// Guard fault for one class. The bodies stay in [`Self::observe`].
    ///
    /// The default is `None`. A set value stays until another call changes it.
    /// [`Self::try_slide`] and [`Self::obstacles_for_planner`] omit that one class.
    /// This is not deletion.
    pub const fn set_collision_fault(&mut self, class: Option<ObstacleClass>) {
        self.collision_fault = class;
    }

    pub const fn collision_fault(&self) -> Option<ObstacleClass> {
        self.collision_fault
    }

    /// Obstacles offered to a planner before that planner's own sensor.
    ///
    /// The default fault is `None`, and this list matches `observe`. `Some(class)`
    /// omits that class only. A live hit test reads `observe`, not this list.
    pub fn obstacles_for_planner(&self) -> Vec<Obstacle> {
        self.obstacles
            .iter()
            .filter(|obstacle| !self.fault_omits(obstacle.class))
            .cloned()
            .collect()
    }

    fn fault_omits(&self, class: ObstacleClass) -> bool {
        self.collision_fault == Some(class)
    }

    /// Changes the setup fact `visible`. This is not a detector.
    pub fn set_visible(&mut self, bag: Id, visible: bool) -> Result<(), ArmError> {
        let index = self.locate(bag)?;
        self.bags[index].visible = visible;
        Ok(())
    }

    /// Replaces the obstacle list. This does not reveal the car and does not move the robot.
    pub fn set_obstacles(&mut self, obstacles: Vec<Obstacle>) {
        self.obstacles = obstacles;
    }

    /// Moves the hidden car. The observation, the trace, and [`World::revealed_car`] stay unchanged.
    ///
    /// The navigator is not given this point. A scenario may read it when it moves the operator.
    pub fn set_car(&mut self, at: Point) {
        self.car = at;
    }

    /// Fixture motion for the operator. The navigator does not receive this path.
    ///
    /// A bag in operator custody moves with the operator. A free bag stays where it is.
    pub fn place_operator(&mut self, at: Point) {
        self.operator.pose = Pose::new(at);
        for bag in &mut self.bags {
            if bag.custody == Custody::Operator {
                bag.pose = Pose::new(at);
            }
        }
    }

    /// Slides the base to `to` when the whole segment clears every body.
    ///
    /// A held bag moves with the base. A refusal leaves the pose, the bags, and the trace unchanged.
    /// The clock does not move.
    pub fn try_slide(&mut self, to: Point, robot_radius_mm: i32) -> Result<(), SlideError> {
        let from = self.robot.pose.point();
        if self.blocked(from, to, robot_radius_mm) {
            return Err(SlideError::Blocked);
        }
        let pose = Pose::new(to);
        if let Some(carried) = self.carried {
            let index = self.locate(carried).expect("carried bag");
            self.bags[index].pose = pose;
        }
        self.robot.pose = pose;
        self.events.push(Event {
            at_ms: self.clock.now_ms(),
            kind: EventKind::Slid {
                x_mm: to.x().get(),
                y_mm: to.y().get(),
            },
        });
        Ok(())
    }

    fn blocked(&self, from: Point, to: Point, robot_radius_mm: i32) -> bool {
        self.obstacles.iter().any(|obstacle| {
            if self.fault_omits(obstacle.class) {
                return false;
            }
            obstacle.bodies.iter().any(|disk| {
                let limit = robot_radius_mm.saturating_add(disk.radius.get());
                segment_within(from, to, disk.center, limit)
            })
        })
    }

    /// Moves a free bag. This does not move the robot and does not pick the bag.
    pub fn place_free(&mut self, bag: Id, at: Point) -> Result<(), ArmError> {
        if bag == self.operator.id {
            return Err(ArmError::Role);
        }
        let index = self.locate(bag)?;
        if self.bags[index].custody != Custody::Free {
            return Err(ArmError::NotFree);
        }
        self.bags[index].pose = Pose::new(at);
        self.events.push(Event {
            at_ms: self.clock.now_ms(),
            kind: EventKind::Placed { bag },
        });
        Ok(())
    }

    /// Closes the gripper on `bag` when it is free, visible, and inside `reach_mm`.
    ///
    /// The held bag is recorded at the robot base. There is no gripper frame.
    /// A refusal leaves every bag, the gripper, and the trace unchanged.
    pub fn pick(&mut self, bag: Id, reach_mm: i32) -> Result<(), ArmError> {
        if bag == self.operator.id {
            return Err(ArmError::Role);
        }
        if self.carried.is_some() {
            return Err(ArmError::Occupied);
        }
        let index = self.locate(bag)?;
        let visible = self.bags[index].visible;
        let custody = self.bags[index].custody;
        let at = self.bags[index].pose.point();
        if custody != Custody::Free {
            return Err(ArmError::NotFree);
        }
        if !visible {
            return Err(ArmError::Invisible);
        }
        if !within_reach(self.robot.pose.point(), at, reach_mm) {
            return Err(ArmError::OutOfReach);
        }
        if self
            .bags
            .iter()
            .any(|other| other.custody == Custody::Gripper)
        {
            return Err(ArmError::Occupied);
        }
        if self.grasp_fault {
            return Err(ArmError::FailedGrasp);
        }
        let pose = self.robot.pose;
        let at_ms = self.clock.now_ms();
        self.bags[index].custody = Custody::Gripper;
        self.bags[index].pose = pose;
        self.carried = Some(bag);
        self.events.push(Event {
            at_ms,
            kind: EventKind::Grasped { bag },
        });
        Ok(())
    }

    /// A person takes a free bag. The gripper stays empty. This is not a robot grasp.
    pub fn human_take(&mut self, bag: Id) -> Result<(), ArmError> {
        if bag == self.operator.id {
            return Err(ArmError::Role);
        }
        if self.carried.is_some() {
            return Err(ArmError::Occupied);
        }
        let index = self.locate(bag)?;
        if self.bags[index].custody != Custody::Free {
            return Err(ArmError::NotFree);
        }
        let pose = self.operator.pose;
        let at_ms = self.clock.now_ms();
        self.bags[index].custody = Custody::Operator;
        self.bags[index].pose = pose;
        self.events.push(Event {
            at_ms,
            kind: EventKind::Handed {
                bag,
                to: self.operator.id,
            },
        });
        Ok(())
    }

    /// Opens the gripper and records the bag at the robot base.
    pub fn drop_held(&mut self, bag: Id) -> Result<Point, ArmError> {
        if bag == self.operator.id {
            return Err(ArmError::Role);
        }
        let Some(carried) = self.carried else {
            return Err(ArmError::Empty);
        };
        if carried != bag {
            return Err(ArmError::WrongId);
        }
        let index = self.locate(bag)?;
        if self.bags[index].custody != Custody::Gripper {
            return Err(ArmError::NotFree);
        }
        let at = self.robot.pose.point();
        let at_ms = self.clock.now_ms();
        self.bags[index].custody = Custody::Free;
        self.bags[index].pose = Pose::new(at);
        self.carried = None;
        self.events.push(Event {
            at_ms,
            kind: EventKind::Dropped { bag },
        });
        Ok(at)
    }

    /// Gives the held bag to the operator. The bag stays in the world, unheld.
    ///
    /// The transfer is the segment from the held bag to the operator. It uses
    /// every world disk and `robot_radius_mm`, the same grown radius as
    /// [`Self::try_slide`]. A blocked segment is a refusal. It is checked after
    /// custody and before an injected handoff fault. The navigator is not given
    /// those disks.
    pub fn hand_to(&mut self, bag: Id, to: Id, robot_radius_mm: i32) -> Result<(), ArmError> {
        if to == bag || self.bags.iter().any(|item| item.id == to) {
            return Err(ArmError::Role);
        }
        if to != self.operator.id {
            return Err(ArmError::WrongRecipient);
        }
        let Some(carried) = self.carried else {
            return Err(ArmError::Empty);
        };
        if carried != bag {
            return Err(ArmError::WrongId);
        }
        let index = self.locate(bag)?;
        if self.bags[index].custody != Custody::Gripper {
            return Err(ArmError::NotFree);
        }
        if self.blocked(
            self.bags[index].pose.point(),
            self.operator.pose.point(),
            robot_radius_mm,
        ) {
            return Err(ArmError::Blocked);
        }
        if self.handoff_fault {
            return Err(ArmError::HandoffFailed);
        }
        let pose = self.operator.pose;
        let at_ms = self.clock.now_ms();
        self.bags[index].custody = Custody::Operator;
        self.bags[index].pose = pose;
        self.carried = None;
        self.events.push(Event {
            at_ms,
            kind: EventKind::Handed { bag, to },
        });
        Ok(())
    }

    fn locate(&self, id: Id) -> Result<usize, ArmError> {
        let mut found = None;
        for (index, bag) in self.bags.iter().enumerate() {
            if bag.id == id {
                if found.is_some() {
                    return Err(ArmError::Duplicate);
                }
                found = Some(index);
            }
        }
        found.ok_or(ArmError::Missing)
    }
}

/// Why the simulated gripper did not change the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmError {
    Missing,
    Invisible,
    OutOfReach,
    Occupied,
    Empty,
    WrongId,
    Role,
    NotFree,
    WrongRecipient,
    Duplicate,
    FailedGrasp,
    HandoffFailed,
    Blocked,
}

impl std::fmt::Display for ArmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ArmError::Missing => "missing bag",
            ArmError::Invisible => "invisible bag",
            ArmError::OutOfReach => "out of reach",
            ArmError::Occupied => "gripper occupied",
            ArmError::Empty => "gripper empty",
            ArmError::WrongId => "wrong id",
            ArmError::Role => "id role",
            ArmError::NotFree => "bag not free",
            ArmError::WrongRecipient => "wrong recipient",
            ArmError::Duplicate => "duplicate bag",
            ArmError::FailedGrasp => "grasp failed",
            ArmError::HandoffFailed => "handoff failed",
            ArmError::Blocked => "blocked transfer",
        })
    }
}

impl std::error::Error for ArmError {}

/// The base did not move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlideError {
    Blocked,
}

impl std::fmt::Display for SlideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SlideError::Blocked => "blocked",
        })
    }
}

impl std::error::Error for SlideError {}

impl World for SimWorld {
    fn now_ms(&self) -> u64 {
        self.clock.now_ms()
    }

    fn advance(&mut self, step: Step) -> Result<(), TimeError> {
        self.clock.advance(step)?;
        self.events.push(Event {
            at_ms: self.clock.now_ms(),
            kind: EventKind::Advanced { step_ms: step.ms() },
        });
        Ok(())
    }

    fn observe(&self) -> Observation {
        Observation {
            robot: self.robot,
            operator: self.operator,
            bags: self.bags.clone(),
            obstacles: self.obstacles.clone(),
            now_ms: self.clock.now_ms(),
        }
    }

    fn revealed_car(&self) -> Option<Point> {
        None
    }

    fn events(&self) -> &[Event] {
        &self.events
    }
}

/// Arm adapter. It reads the carried bag and does not grasp.
pub struct SimArm<'a> {
    world: &'a SimWorld,
}

impl<'a> SimArm<'a> {
    pub const fn new(world: &'a SimWorld) -> Self {
        Self { world }
    }
}

impl Arm for SimArm<'_> {
    fn carried(&self) -> Option<Id> {
        self.world.carried
    }
}

/// Base adapter. It reports the task-visible robot pose.
pub struct SimBase<'a> {
    world: &'a SimWorld,
}

impl<'a> SimBase<'a> {
    pub const fn new(world: &'a SimWorld) -> Self {
        Self { world }
    }
}

impl Base for SimBase<'_> {
    fn pose(&self) -> Pose {
        self.world.robot.pose
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aim::{AimError, AimSample, direction_mm};
    use crate::assume;

    fn bag_placement_ok(bag: Point, operator: Point) -> bool {
        let dx = i64::from(bag.x().get()) - i64::from(operator.x().get());
        let dy = i64::from(bag.y().get()) - i64::from(operator.y().get());
        let squared = dx * dx + dy * dy;
        let near = i64::from(assume::BAG_NEAR_MIN_MM);
        squared >= near * near && squared <= 2_000 * 2_000
    }

    fn has_class(obstacles: &[Obstacle], class: ObstacleClass) -> bool {
        obstacles.iter().any(|obstacle| obstacle.class == class)
    }

    fn point(x: i32, y: i32) -> Point {
        Point::new(Mm::new(x), Mm::new(y))
    }

    #[test]
    fn seed_one_matches_layout_v1() {
        let world = SimWorld::from_seed(1);
        let obs = world.observe();
        assert_eq!(assume::LAYOUT, "m3-layout-1");
        assert_eq!(assume::TICK_UNIT, "ms");
        assert_eq!(obs.robot.pose.point(), point(0, 0));
        assert_eq!(obs.operator.pose.point(), point(0, 1000));
        assert_eq!(obs.bags.len(), 2);
        assert_eq!(obs.bags[0].pose.point(), point(1531, 1000));
        assert_eq!(obs.bags[1].pose.point(), point(-453, 1000));
        assert!(obs.bags.iter().all(|bag| bag.visible));
        assert!(
            obs.bags
                .iter()
                .all(|bag| bag.kind == BagKind::LightweightStiffVerticalHandles)
        );
        assert_eq!(
            obs.obstacles
                .iter()
                .map(|obstacle| obstacle.class)
                .collect::<Vec<_>>(),
            vec![
                ObstacleClass::Barriers,
                ObstacleClass::HardToSee,
                ObstacleClass::Crowd,
                ObstacleClass::SmallGround,
            ]
        );
        assert_eq!(obs.obstacles[0].pose.point(), point(4000, 0));
        assert_eq!(obs.obstacles[0].clearance, Mm::new(assume::CLEARANCE_MM));
        match world.events() {
            [
                Event {
                    at_ms: 0,
                    kind:
                        EventKind::Layout {
                            bag_east_mm: 1531,
                            bag_west_mm: 453,
                            order,
                        },
                },
            ] => assert_eq!(
                order,
                &[
                    ObstacleClass::Barriers,
                    ObstacleClass::HardToSee,
                    ObstacleClass::Crowd,
                    ObstacleClass::SmallGround,
                ]
            ),
            other => panic!("layout event {other:?}"),
        }
    }

    #[test]
    fn fixtures_keep_two_bags_and_four_classes() {
        let mut seeds: Vec<u64> = (0..32).collect();
        seeds.push(u64::MAX);
        for seed in seeds {
            let world = SimWorld::from_seed(seed);
            let obs = world.observe();
            assert!(obs.bags.len() >= 2, "seed {seed}");
            assert_eq!(obs.obstacles.len(), 4, "seed {seed}");
            for class in [
                ObstacleClass::SmallGround,
                ObstacleClass::HardToSee,
                ObstacleClass::Crowd,
                ObstacleClass::Barriers,
            ] {
                assert!(has_class(&obs.obstacles, class), "seed {seed} {class:?}");
            }
            assert!(obs.bags.iter().all(|bag| {
                bag.visible
                    && bag.kind == BagKind::LightweightStiffVerticalHandles
                    && bag_placement_ok(bag.pose.point(), obs.operator.pose.point())
            }));
            assert!(
                obs.obstacles
                    .iter()
                    .all(|obstacle| obstacle.clearance == Mm::new(assume::CLEARANCE_MM))
            );
            assert_eq!(
                obs.robot.pose.point(),
                point(assume::ROBOT_X_MM, assume::ROBOT_Y_MM)
            );
            assert_eq!(
                obs.operator.pose.point(),
                point(assume::OPERATOR_X_MM, assume::OPERATOR_Y_MM)
            );
            let mut ids = vec![obs.robot.id, obs.operator.id, world.truth().car_id()];
            ids.extend(obs.bags.iter().map(|bag| bag.id));
            ids.extend(obs.obstacles.iter().map(|obstacle| obstacle.id));
            let mut sorted = ids.clone();
            sorted.sort_by_key(|id| id.get());
            sorted.dedup();
            assert_eq!(sorted.len(), ids.len(), "seed {seed}");
            assert!(obs.robot.pose.point().x().get() <= assume::INSIDE_X_MAX_MM);
            assert!(
                obs.bags
                    .iter()
                    .all(|bag| bag.pose.point().x().get() <= assume::INSIDE_X_MAX_MM)
            );
            assert!(
                obs.obstacles
                    .iter()
                    .all(|obstacle| obstacle.pose.point().x().get() > assume::INSIDE_X_MAX_MM)
            );
            assert!(world.truth().car().x().get() > assume::INSIDE_X_MAX_MM);
            assert_eq!(world.revealed_car(), None);
            assert_eq!(
                world.truth().car(),
                point(assume::CAR_X_MM, assume::CAR_Y_MM)
            );
        }
    }

    #[test]
    fn the_same_seed_replays_the_same_trace() {
        fn script(seed: u64) -> SimWorld {
            let mut world = SimWorld::from_seed(seed);
            world.advance(Step::from_ms(0)).unwrap();
            world.advance(Step::from_ms(100)).unwrap();
            assert!(world.advance(Step::from_ms(u64::MAX)).is_err());
            world.advance(Step::from_ms(25)).unwrap();
            world
        }
        let left = script(11);
        let right = script(11);
        assert_eq!(left.observe(), right.observe());
        assert_eq!(left.events(), right.events());
        assert_eq!(left.truth(), right.truth());
        assert_eq!(left.now_ms(), right.now_ms());
    }

    #[test]
    fn a_different_seed_changes_the_visible_trace() {
        let left = SimWorld::from_seed(1);
        let right = SimWorld::from_seed(2);
        assert_ne!(left.observe(), right.observe());
        assert_ne!(left.events(), right.events());
        assert!(left.observe().bags.len() >= 2);
        assert!(right.observe().bags.len() >= 2);
    }

    #[test]
    fn overflow_does_not_mutate_and_a_later_step_still_applies() {
        let mut world = SimWorld::from_seed(3);
        world.advance(Step::from_ms(10)).unwrap();
        let observed = world.observe();
        let events = world.events().to_vec();
        let truth = world.truth();
        assert_eq!(
            world.advance(Step::from_ms(u64::MAX)),
            Err(TimeError::Overflow)
        );
        assert_eq!(world.observe(), observed);
        assert_eq!(world.events(), events.as_slice());
        assert_eq!(world.truth(), truth);
        world.advance(Step::from_ms(5)).unwrap();
        assert_eq!(world.now_ms(), 15);
        assert_eq!(world.observe().now_ms, 15);
        assert_eq!(observed.now_ms, 10);
    }

    #[test]
    fn the_task_port_hides_the_car() {
        let world = SimWorld::from_seed(1);
        fn task_car(world: &impl World) -> Option<Point> {
            world.revealed_car()
        }
        let car = world.truth().car();
        assert_eq!(task_car(&world), None);
        assert!(world.observe().points().all(|point| point != car));
        assert_eq!(car, point(assume::CAR_X_MM, assume::CAR_Y_MM));
        let ids: Vec<_> = std::iter::once(world.observe().robot.id)
            .chain(std::iter::once(world.observe().operator.id))
            .chain(world.observe().bags.iter().map(|bag| bag.id))
            .chain(world.observe().obstacles.iter().map(|obstacle| obstacle.id))
            .collect();
        assert!(!ids.contains(&world.truth().car_id()));
    }

    #[test]
    fn adapters_match_the_visible_robot_and_do_not_grasp() {
        let mut world = SimWorld::from_seed(1);
        let arm = SimArm::new(&world);
        let base = SimBase::new(&world);
        assert_eq!(arm.carried(), None);
        assert_eq!(base.pose(), world.observe().robot.pose);
        world.advance(Step::from_ms(40)).unwrap();
        let arm = SimArm::new(&world);
        let base = SimBase::new(&world);
        assert_eq!(arm.carried(), None);
        assert_eq!(base.pose().point(), point(0, 0));
        assert_eq!(world.observe().robot.pose.point(), point(0, 0));
    }

    fn length_ok(sample: &AimSample) -> bool {
        let dx = i64::from(sample.dx());
        let dy = i64::from(sample.dy());
        let squared = dx * dx + dy * dy;
        let near = i64::from(assume::BAG_NEAR_MIN_MM);
        let far = i64::from(assume::BAG_NEAR_MAX_MM);
        squared >= near * near && squared <= far * far
    }

    #[test]
    fn seed_one_aim_is_the_visible_bag_offset_and_not_the_car() {
        let world = SimWorld::from_seed(1);
        let obs = world.observe();
        let before = obs.clone();
        let events = world.events().to_vec();
        let truth = world.truth();
        let east = world.aim_at(obs.bags[0].id).unwrap();
        let west = world.aim_at(obs.bags[1].id).unwrap();
        assert_eq!(east.origin(), point(0, 1000));
        assert_eq!(west.origin(), point(0, 1000));
        assert_eq!((east.dx(), east.dy()), (1531, 0));
        assert_eq!((west.dx(), west.dy()), (-453, 0));
        assert_ne!((east.dx(), east.dy()), (west.dx(), west.dy()));
        let car = direction_mm(obs.operator.pose.point(), truth.car()).unwrap();
        assert_ne!((east.dx(), east.dy()), car);
        assert_ne!((west.dx(), west.dy()), car);
        assert_eq!(world.aim_at(truth.car_id()), Err(AimError::UnknownBag));
        assert_eq!(world.aim_at(obs.robot.id), Err(AimError::UnknownBag));
        assert_eq!(world.aim_at(obs.operator.id), Err(AimError::UnknownBag));
        assert_eq!(
            world.aim_at(Id::new(99).unwrap()),
            Err(AimError::UnknownBag)
        );
        assert_eq!(world.observe(), before);
        assert_eq!(world.events(), events.as_slice());
        assert_eq!(world.truth(), truth);
        assert_eq!(world.revealed_car(), None);
    }

    #[test]
    fn both_bags_stay_inside_two_meters_for_every_fixture_seed() {
        let mut seeds: Vec<u64> = (0..32).collect();
        seeds.push(u64::MAX);
        for seed in seeds {
            let left = SimWorld::from_seed(seed);
            let right = SimWorld::from_seed(seed);
            let obs = left.observe();
            assert!(obs.bags.len() >= 2, "seed {seed}");
            for bag in &obs.bags {
                assert!(bag.visible, "seed {seed}");
                assert!(
                    bag_placement_ok(bag.pose.point(), obs.operator.pose.point()),
                    "seed {seed}"
                );
                let sample = left.aim_at(bag.id).unwrap();
                assert_eq!(sample, right.aim_at(bag.id).unwrap(), "seed {seed}");
                assert_eq!(sample.origin(), obs.operator.pose.point(), "seed {seed}");
                assert!(
                    length_ok(&sample),
                    "seed {seed} {:?}",
                    (sample.dx(), sample.dy())
                );
                let delta = direction_mm(obs.operator.pose.point(), bag.pose.point()).unwrap();
                assert_eq!((sample.dx(), sample.dy()), delta, "seed {seed}");
            }
        }
    }

    fn snap(world: &SimWorld) -> (Observation, Vec<Event>, Truth, Option<Id>) {
        (
            world.observe(),
            world.events().to_vec(),
            world.truth(),
            world.carried(),
        )
    }

    fn conserved(world: &SimWorld) {
        let obs = world.observe();
        let mut ids: Vec<u32> = obs.bags.iter().map(|bag| bag.id.get()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count);
        let held: Vec<Id> = obs
            .bags
            .iter()
            .filter(|bag| bag.custody == domain::Custody::Gripper)
            .map(|bag| bag.id)
            .collect();
        match world.carried() {
            Some(id) => {
                assert_eq!(held, vec![id]);
                let bag = obs.bags.iter().find(|bag| bag.id == id).unwrap();
                assert_eq!(bag.pose, obs.robot.pose);
            }
            None => assert!(held.is_empty()),
        }
        for bag in &obs.bags {
            if bag.custody == domain::Custody::Operator {
                assert_eq!(bag.pose, obs.operator.pose);
            }
        }
        assert_eq!(obs.robot.pose.point(), point(0, 0));
    }

    #[test]
    fn each_fixture_bag_is_held_once_and_leaves_the_other_bag_in_place() {
        let mut seeds: Vec<u64> = (0..32).collect();
        seeds.push(u64::MAX);
        for seed in seeds {
            let probe = SimWorld::from_seed(seed);
            assert!(probe.observe().bags.len() >= 2, "seed {seed}");
            for index in 0..probe.observe().bags.len() {
                let mut world = SimWorld::from_seed(seed);
                let before = world.observe();
                let target = before.bags[index].clone();
                let other = before.bags[1 - index].clone();
                assert_eq!(target.custody, domain::Custody::Free);
                assert_ne!(target.pose.point(), before.robot.pose.point());
                assert!(domain::within_reach(
                    before.robot.pose.point(),
                    target.pose.point(),
                    2500
                ));
                world.pick(target.id, 2500).unwrap();
                conserved(&world);
                let held = world
                    .observe()
                    .bags
                    .into_iter()
                    .find(|bag| bag.id == target.id)
                    .unwrap();
                assert_eq!(world.carried(), Some(target.id));
                assert_eq!(held.custody, domain::Custody::Gripper);
                assert_eq!(held.pose, before.robot.pose);
                assert_ne!(held.pose.point(), target.pose.point());
                assert_eq!(
                    world
                        .observe()
                        .bags
                        .iter()
                        .filter(|bag| bag.id == target.id)
                        .count(),
                    1
                );
                let stayed = world
                    .observe()
                    .bags
                    .into_iter()
                    .find(|bag| bag.id == other.id)
                    .unwrap();
                assert_eq!(stayed, other);
                assert_eq!(world.observe().robot.pose, before.robot.pose);
                assert_eq!(world.now_ms(), before.now_ms);
                assert_eq!(world.revealed_car(), None);
                assert!(matches!(
                    world.events().last().map(|event| &event.kind),
                    Some(EventKind::Grasped { bag }) if *bag == target.id
                ));
                let occupied = snap(&world);
                assert_eq!(world.pick(other.id, 2500), Err(ArmError::Occupied));
                assert_eq!(snap(&world), occupied);
                assert_eq!(world.pick(target.id, 2500), Err(ArmError::Occupied));
                assert_eq!(snap(&world), occupied);
            }
        }
    }

    #[test]
    fn refusals_do_not_move_a_bag_or_invent_a_gripper_id() {
        let mut world = SimWorld::from_seed(1);
        let bag = world.observe().bags[0].id;
        let operator = world.observe().operator.id;
        let missing = Id::new(99).unwrap();
        let before = snap(&world);
        assert_eq!(world.pick(bag, 0), Err(ArmError::OutOfReach));
        assert_eq!(snap(&world), before);
        assert_eq!(world.pick(missing, 2500), Err(ArmError::Missing));
        assert_eq!(snap(&world), before);
        assert_ne!(world.carried(), Some(missing));
        assert_eq!(world.pick(operator, 2500), Err(ArmError::Role));
        assert_eq!(snap(&world), before);
        world.bags[0].visible = false;
        let hidden = snap(&world);
        assert_eq!(world.pick(bag, 2500), Err(ArmError::Invisible));
        assert_eq!(snap(&world), hidden);
        world.set_grasp_fault(true);
        assert_eq!(world.pick(bag, 2500), Err(ArmError::Invisible));
        assert_eq!(snap(&world), hidden);
        world.bags[0].visible = true;
        world.place_free(bag, point(2501, 0)).unwrap();
        let far = snap(&world);
        world.set_grasp_fault(true);
        assert_eq!(world.pick(bag, 2500), Err(ArmError::OutOfReach));
        assert_eq!(snap(&world), far);
        assert_eq!(world.carried(), None);
    }

    #[test]
    fn the_reach_edge_holds_and_one_millimeter_past_it_does_not() {
        let mut world = SimWorld::from_seed(2);
        let bag = world.observe().bags[0].id;
        world.place_free(bag, point(2500, 0)).unwrap();
        let robot = world.observe().robot.pose;
        world.pick(bag, 2500).unwrap();
        let held = world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id == bag)
            .unwrap();
        assert_eq!(held.custody, domain::Custody::Gripper);
        assert_eq!(held.pose, robot);
        assert_eq!(world.carried(), Some(bag));
        let mut outside = SimWorld::from_seed(2);
        let bag = outside.observe().bags[0].id;
        outside.place_free(bag, point(2501, 0)).unwrap();
        let before = snap(&outside);
        assert_eq!(outside.pick(bag, 2500), Err(ArmError::OutOfReach));
        assert_eq!(snap(&outside), before);
        let mut negative = SimWorld::from_seed(2);
        let bag = negative.observe().bags[1].id;
        negative.place_free(bag, point(-2500, 0)).unwrap();
        negative.pick(bag, 2500).unwrap();
        assert_eq!(negative.carried(), Some(bag));
        conserved(&negative);
    }

    #[test]
    fn an_injected_grasp_failure_does_not_hold_the_legal_bag() {
        let mut world = SimWorld::from_seed(4);
        let bag = world.observe().bags[1].id;
        let old = world.observe().bags[1].pose.point();
        world.set_grasp_fault(true);
        let before = snap(&world);
        assert_eq!(world.pick(bag, 2500), Err(ArmError::FailedGrasp));
        assert_eq!(snap(&world), before);
        assert_eq!(
            world
                .observe()
                .bags
                .iter()
                .find(|item| item.id == bag)
                .unwrap()
                .pose
                .point(),
            old
        );
        world.set_grasp_fault(false);
        world.pick(bag, 2500).unwrap();
        assert_eq!(world.carried(), Some(bag));
        conserved(&world);
    }

    #[test]
    fn drop_clears_the_gripper_at_the_robot_and_a_second_drop_does_not() {
        let mut world = SimWorld::from_seed(5);
        let bag = world.observe().bags[0].id;
        let operator = world.observe().operator.id;
        let old = world.observe().bags[0].pose.point();
        let empty = snap(&world);
        assert_eq!(world.drop_held(bag), Err(ArmError::Empty));
        assert_eq!(snap(&world), empty);
        world.pick(bag, 2500).unwrap();
        let other = world.observe().bags[1].id;
        let held = snap(&world);
        assert_eq!(world.drop_held(operator), Err(ArmError::Role));
        assert_eq!(world.drop_held(other), Err(ArmError::WrongId));
        assert_eq!(snap(&world), held);
        let at = world.drop_held(bag).unwrap();
        assert_eq!(at, point(0, 0));
        assert_eq!(world.carried(), None);
        let dropped = world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id == bag)
            .unwrap();
        assert_eq!(dropped.custody, domain::Custody::Free);
        assert_eq!(dropped.pose.point(), at);
        assert_ne!(dropped.pose.point(), old);
        conserved(&world);
        let after = snap(&world);
        assert_eq!(world.drop_held(bag), Err(ArmError::Empty));
        assert_eq!(snap(&world), after);
        assert!(matches!(
            world.events().last().map(|event| &event.kind),
            Some(EventKind::Dropped { bag: dropped_id }) if *dropped_id == bag
        ));
    }

    #[test]
    fn handoff_gives_the_bag_to_the_operator_and_keeps_one_copy() {
        let mut world = SimWorld::from_seed(6);
        let bag = world.observe().bags[0].id;
        let other = world.observe().bags[1].id;
        let operator = world.observe().operator.id;
        world.pick(bag, 2500).unwrap();
        let held = snap(&world);
        assert_eq!(world.hand_to(bag, other, 250), Err(ArmError::Role));
        assert_eq!(
            world.hand_to(bag, Id::new(8).unwrap(), 250),
            Err(ArmError::WrongRecipient)
        );
        assert_eq!(world.hand_to(other, operator, 250), Err(ArmError::WrongId));
        assert_eq!(snap(&world), held);
        world.set_handoff_fault(true);
        assert_eq!(
            world.hand_to(bag, operator, 250),
            Err(ArmError::HandoffFailed)
        );
        assert_eq!(snap(&world), held);
        world.set_handoff_fault(false);
        world.hand_to(bag, operator, 250).unwrap();
        assert_eq!(world.carried(), None);
        let given = world
            .observe()
            .bags
            .into_iter()
            .find(|item| item.id == bag)
            .unwrap();
        assert_eq!(given.custody, domain::Custody::Operator);
        assert_eq!(given.pose, world.observe().operator.pose);
        assert_eq!(
            world
                .observe()
                .bags
                .iter()
                .filter(|item| item.id == bag)
                .count(),
            1
        );
        conserved(&world);
        let after = snap(&world);
        assert_eq!(world.hand_to(bag, operator, 250), Err(ArmError::Empty));
        assert_eq!(world.pick(bag, 2500), Err(ArmError::NotFree));
        assert_eq!(snap(&world), after);
        assert_eq!(world.observe().robot.pose.point(), point(0, 0));
        assert_eq!(world.revealed_car(), None);
    }

    #[test]
    fn a_held_bag_survives_the_clock_and_cannot_be_replaced_by_placement() {
        let mut world = SimWorld::from_seed(7);
        let bag = world.observe().bags[0].id;
        world.pick(bag, 2500).unwrap();
        world.advance(Step::from_ms(1000)).unwrap();
        assert_eq!(world.now_ms(), 1000);
        assert_eq!(world.carried(), Some(bag));
        conserved(&world);
        let held = snap(&world);
        assert_eq!(
            world.place_free(bag, point(100, 100)),
            Err(ArmError::NotFree)
        );
        assert_eq!(snap(&world), held);
        let mut far = SimWorld::from_seed(8);
        let bag = far.observe().bags[0].id;
        far.place_free(bag, point(i32::MAX, i32::MIN)).unwrap();
        let before = snap(&far);
        assert_eq!(far.pick(bag, 2500), Err(ArmError::OutOfReach));
        assert_eq!(snap(&far), before);
        assert_eq!(far.pick(bag, i32::MAX), Err(ArmError::OutOfReach));
        assert_eq!(snap(&far), before);
    }

    #[test]
    fn a_duplicate_id_is_not_taken() {
        let mut world = SimWorld::from_seed(9);
        let extra = world.bags[0].clone();
        world.bags.push(extra);
        let bag = world.bags[0].id;
        let before = snap(&world);
        assert_eq!(world.pick(bag, 2500), Err(ArmError::Duplicate));
        assert_eq!(snap(&world), before);
        assert_eq!(world.carried(), None);
    }

    #[test]
    fn seeded_obstacles_are_disks_and_the_crowd_is_three_or_four_people() {
        let mut saw_three = false;
        let mut saw_four = false;
        for seed in 0..8 {
            let world = SimWorld::from_seed(seed);
            let obs = world.observe();
            for obstacle in &obs.obstacles {
                match obstacle.class {
                    ObstacleClass::Crowd => {
                        let count = obstacle.bodies.len();
                        assert!((3..=4).contains(&count), "seed {seed} {count}");
                        saw_three |= count == 3;
                        saw_four |= count == 4;
                        assert!(obstacle.bodies.windows(2).all(|pair| {
                            let dx = i64::from(pair[1].center.x().get() - pair[0].center.x().get());
                            let dy = i64::from(pair[1].center.y().get() - pair[0].center.y().get());
                            dx * dx + dy * dy > 0
                        }));
                        assert!(
                            obstacle
                                .bodies
                                .iter()
                                .all(|disk| { disk.radius == Mm::new(assume::PERSON_RADIUS_MM) })
                        );
                    }
                    ObstacleClass::SmallGround => {
                        assert_eq!(obstacle.bodies.len(), 1);
                        assert_eq!(obstacle.bodies[0].radius, Mm::new(assume::SMALL_RADIUS_MM));
                    }
                    ObstacleClass::HardToSee => {
                        assert_eq!(obstacle.bodies.len(), 1);
                        assert_eq!(obstacle.bodies[0].radius, Mm::new(assume::HARD_RADIUS_MM));
                    }
                    ObstacleClass::Barriers => {
                        assert_eq!(obstacle.bodies.len(), 1);
                        assert_eq!(
                            obstacle.bodies[0].radius,
                            Mm::new(assume::BARRIER_RADIUS_MM)
                        );
                    }
                }
            }
            assert_eq!(world.revealed_car(), None);
            assert!(obs.points().all(|point| point != world.truth().car()));
        }
        assert!(saw_three);
        assert!(saw_four);
    }

    #[test]
    fn a_jump_through_a_disk_is_rejected_and_a_clear_step_moves_the_held_bag() {
        let mut world = SimWorld::from_seed(1);
        let bag = world.observe().bags[0].id;
        let other = world.observe().bags[1].pose.point();
        world.pick(bag, 2500).unwrap();
        world.set_obstacles(vec![Obstacle {
            id: Id::new(8).unwrap(),
            pose: Pose::new(point(0, 0)),
            class: ObstacleClass::Barriers,
            clearance: Mm::new(assume::CLEARANCE_MM),
            bodies: vec![Disk {
                id: Id::new(8).unwrap(),
                center: point(500, 0),
                radius: Mm::new(100),
            }],
        }]);
        let before = snap(&world);
        assert_eq!(
            world.try_slide(point(1000, 0), 50),
            Err(SlideError::Blocked)
        );
        assert_eq!(snap(&world), before);
        assert_eq!(world.observe().robot.pose.point(), point(0, 0));
        world.try_slide(point(200, 0), 50).unwrap();
        let moved = world.observe();
        assert_eq!(moved.robot.pose.point(), point(200, 0));
        let held = moved.bags.iter().find(|item| item.id == bag).expect("held");
        assert_eq!(held.pose.point(), point(200, 0));
        assert_eq!(held.custody, Custody::Gripper);
        let left = moved.bags.iter().find(|item| item.id != bag).expect("free");
        assert_eq!(left.pose.point(), other);
        assert_eq!(left.custody, Custody::Free);
        assert_eq!(world.revealed_car(), None);
        world.place_operator(point(0, 1800));
        let shifted = world.observe();
        assert_eq!(shifted.operator.pose.point(), point(0, 1800));
        assert_eq!(
            shifted
                .bags
                .iter()
                .find(|item| item.id != bag)
                .expect("free")
                .pose
                .point(),
            other
        );
        world.set_obstacles(Vec::new());
        world.hand_to(bag, shifted.operator.id, 250).unwrap();
        let taken = world.observe();
        let handed = taken.bags.iter().find(|item| item.id == bag).unwrap();
        assert_eq!(handed.custody, Custody::Operator);
        assert_eq!(handed.pose.point(), point(0, 1800));
        world.place_operator(point(300, 1800));
        let followed = world.observe();
        let handed = followed.bags.iter().find(|item| item.id == bag).unwrap();
        assert_eq!(handed.pose.point(), point(300, 1800));
        assert_eq!(followed.robot.pose.point(), point(200, 0));
    }

    fn disk_obstacle(class: ObstacleClass, x: i32, y: i32, radius: i32) -> Obstacle {
        Obstacle {
            id: Id::new(6).unwrap(),
            pose: Pose::new(point(x, y)),
            class,
            clearance: Mm::new(400),
            bodies: vec![Disk {
                id: Id::new(6).unwrap(),
                center: point(x, y),
                radius: Mm::new(radius),
            }],
        }
    }

    /// Robot disk used by slides and handoffs. Same stand-in as navigation.
    const ROBOT_MM: i32 = 250;

    #[test]
    fn a_handoff_rejects_a_disk_between_clear_ends_and_leaves_the_world() {
        let mut world = SimWorld::from_seed(1);
        let bag = world.observe().bags[0].id;
        let operator = world.observe().operator.id;
        world.pick(bag, 2500).unwrap();
        world.try_slide(point(0, 200), ROBOT_MM).unwrap();
        world.place_operator(point(0, 1400));
        let grown = ROBOT_MM + 60;
        let from = point(0, 200);
        let to = point(0, 1400);
        let center = point(0, 1060);
        assert!(!segment_within(from, from, center, grown));
        assert!(!segment_within(to, to, center, grown));
        assert!(segment_within(from, to, center, grown));
        for class in [ObstacleClass::HardToSee, ObstacleClass::SmallGround] {
            world.set_obstacles(vec![disk_obstacle(class, 0, 1060, 60)]);
            world.set_handoff_fault(true);
            let before = snap(&world);
            assert_eq!(
                world.hand_to(bag, operator, ROBOT_MM),
                Err(ArmError::Blocked),
                "{class:?}"
            );
            assert_eq!(snap(&world), before);
            assert!(world.handoff_fault());
            assert_eq!(world.carried(), Some(bag));
            assert_eq!(world.now_ms(), before.0.now_ms);
        }
        assert_eq!(
            world.try_slide(point(0, 1400), ROBOT_MM),
            Err(SlideError::Blocked)
        );
        assert_eq!(world.observe().robot.pose.point(), point(0, 200));
        world.set_obstacles(Vec::new());
        world.set_obstacles(vec![
            disk_obstacle(ObstacleClass::Barriers, 4000, 0, 30),
            disk_obstacle(ObstacleClass::HardToSee, 0, 1060, 60),
        ]);
        assert_eq!(
            world.hand_to(bag, operator, ROBOT_MM),
            Err(ArmError::Blocked)
        );
        world.set_obstacles(vec![
            disk_obstacle(ObstacleClass::HardToSee, 0, 1060, 60),
            disk_obstacle(ObstacleClass::Barriers, 4000, 0, 30),
        ]);
        let ordered = snap(&world);
        assert_eq!(
            world.hand_to(bag, operator, ROBOT_MM),
            Err(ArmError::Blocked)
        );
        assert_eq!(snap(&world), ordered);
        world.set_obstacles(Vec::new());
        world.place_operator(point(1000, 0));
        world.try_slide(point(0, 0), ROBOT_MM).unwrap();
        world.set_handoff_fault(false);
        world.set_obstacles(vec![disk_obstacle(
            ObstacleClass::SmallGround,
            500,
            310,
            60,
        )]);
        world.hand_to(bag, operator, ROBOT_MM).unwrap();
        let tangent = snap(&world);
        assert_eq!(world.carried(), None);
        world.set_obstacles(vec![disk_obstacle(
            ObstacleClass::SmallGround,
            500,
            309,
            60,
        )]);
        let mut again = SimWorld::from_seed(4);
        let again_bag = again.observe().bags[0].id;
        let again_operator = again.observe().operator.id;
        again.pick(again_bag, 2500).unwrap();
        again.place_operator(point(1000, 0));
        again.set_obstacles(vec![disk_obstacle(
            ObstacleClass::SmallGround,
            500,
            309,
            60,
        )]);
        let tight = snap(&again);
        assert!(segment_within(
            point(0, 0),
            point(1000, 0),
            point(500, 309),
            ROBOT_MM + 60
        ));
        assert!(!segment_within(
            point(0, 0),
            point(0, 0),
            point(500, 309),
            ROBOT_MM + 60
        ));
        assert_eq!(
            again.hand_to(again_bag, again_operator, ROBOT_MM),
            Err(ArmError::Blocked)
        );
        assert_eq!(snap(&again), tight);
        again.set_handoff_fault(true);
        assert_eq!(
            again.hand_to(again_bag, again_operator, ROBOT_MM),
            Err(ArmError::Blocked)
        );
        assert!(again.handoff_fault());
        again.set_obstacles(Vec::new());
        assert_eq!(
            again.hand_to(again_bag, again_operator, ROBOT_MM),
            Err(ArmError::HandoffFailed)
        );
        assert_eq!(again.carried(), Some(again_bag));
        let mut far = SimWorld::from_seed(5);
        let far_bag = far.observe().bags[0].id;
        let far_operator = far.observe().operator.id;
        far.pick(far_bag, 2500).unwrap();
        far.place_operator(point(0, 1_000_000));
        far.set_obstacles(vec![disk_obstacle(ObstacleClass::HardToSee, 0, 500_000, 1)]);
        let far_before = snap(&far);
        assert_eq!(
            far.hand_to(far_bag, far_operator, ROBOT_MM),
            Err(ArmError::Blocked)
        );
        assert_eq!(snap(&far), far_before);
        let _ = tangent;
        assert_eq!(
            world.hand_to(bag, Id::new(8).unwrap(), ROBOT_MM),
            Err(ArmError::WrongRecipient)
        );
        let mut empty = SimWorld::from_seed(2);
        let empty_bag = empty.observe().bags[0].id;
        let empty_operator = empty.observe().operator.id;
        empty.set_obstacles(vec![disk_obstacle(ObstacleClass::HardToSee, 0, 1060, 60)]);
        assert_eq!(
            empty.hand_to(empty_bag, empty_operator, ROBOT_MM),
            Err(ArmError::Empty)
        );
    }

    #[test]
    fn a_varied_car_stays_out_of_the_observation() {
        let mut world = SimWorld::from_seed(3);
        let before = world.observe();
        let events = world.events().to_vec();
        let moved = point(12_000, 4_000);
        world.set_car(moved);
        assert_eq!(world.truth().car(), moved);
        assert_eq!(world.observe(), before);
        assert_eq!(world.events(), events.as_slice());
        assert_eq!(world.revealed_car(), None);
        assert!(before.points().all(|item| item != moved));
        world.set_car(point(2_600, -4_000));
        assert_eq!(world.revealed_car(), None);
        assert_eq!(world.observe().robot, before.robot);
    }

    fn legacy_planner_list(fault: Option<ObstacleClass>, obstacles: &[Obstacle]) -> Vec<Obstacle> {
        match fault {
            Some(class) => obstacles
                .iter()
                .filter(|obstacle| obstacle.class != class)
                .cloned()
                .collect(),
            None => obstacles.to_vec(),
        }
    }

    #[test]
    fn a_collision_fault_omits_one_class_from_the_planner_and_the_slide() {
        let mut world = SimWorld::from_seed(1);
        assert_eq!(world.collision_fault(), None);
        let seeded = world.observe().obstacles;
        assert_eq!(
            world.obstacles_for_planner(),
            legacy_planner_list(None, &seeded)
        );
        assert_eq!(world.obstacles_for_planner(), seeded);

        world.set_obstacles(vec![
            disk_obstacle(ObstacleClass::SmallGround, 500, 0, 100),
            disk_obstacle(ObstacleClass::Crowd, 1_000, 400, 100),
        ]);
        let full = world.observe().obstacles;
        assert_eq!(world.obstacles_for_planner(), full);
        assert_eq!(
            world.try_slide(point(1_000, 0), 50),
            Err(SlideError::Blocked)
        );
        assert_eq!(world.observe().robot.pose.point(), point(0, 0));

        world.set_collision_fault(Some(ObstacleClass::SmallGround));
        assert_eq!(world.observe().obstacles, full);
        let planned = world.obstacles_for_planner();
        assert_eq!(
            planned,
            legacy_planner_list(Some(ObstacleClass::SmallGround), &full)
        );
        assert!(
            planned
                .iter()
                .all(|obstacle| obstacle.class != ObstacleClass::SmallGround)
        );
        assert!(
            planned
                .iter()
                .any(|obstacle| obstacle.class == ObstacleClass::Crowd)
        );
        world.try_slide(point(1_000, 0), 50).unwrap();
        assert_eq!(world.observe().robot.pose.point(), point(1_000, 0));
        assert_eq!(world.collision_fault(), Some(ObstacleClass::SmallGround));
        assert_eq!(
            world.try_slide(point(1_000, 1_000), 50),
            Err(SlideError::Blocked)
        );
        assert_eq!(world.observe().robot.pose.point(), point(1_000, 0));
        assert!(
            world
                .observe()
                .obstacles
                .iter()
                .any(|obstacle| obstacle.class == ObstacleClass::SmallGround)
        );

        world.set_collision_fault(None);
        assert_eq!(world.obstacles_for_planner(), world.observe().obstacles);
        assert_eq!(world.try_slide(point(0, 0), 50), Err(SlideError::Blocked));
        assert_eq!(world.observe().robot.pose.point(), point(1_000, 0));
    }
}
