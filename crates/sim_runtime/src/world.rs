use domain::{
    Actor, Arm, Bag, BagKind, Base, Event, EventKind, Id, Mm, Observation, Obstacle, ObstacleClass,
    Point, Pose, Step, TimeError, World,
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
    car: Point,
    car_id: Id,
    events: Vec<Event>,
}

fn id(raw: u32) -> Id {
    Id::new(raw).expect("nonzero id")
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
            },
            Bag {
                id: id(4),
                pose: Pose::new(Point::new(Mm::new(-west), Mm::new(assume::OPERATOR_Y_MM))),
                kind: BagKind::LightweightStiffVerticalHandles,
                visible: true,
            },
        ];
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
}

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
}
