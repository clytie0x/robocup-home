//! Single-gripper decisions for pick, drop, and handoff.
//!
//! [`decide`] reads a scene and returns an action or a refusal. It does not
//! change the scene, it does not confirm a task event, and it does not plan a
//! motion. The numbers in [`assume`] are not printed arm measurements.
//!
//! ```
//! use domain::{Bag, BagKind, Custody, Id, Mm, Point, Pose};
//! use manipulation::{Act, Command, GripError, Scene, decide};
//!
//! let operator = Id::new(2).unwrap();
//! let bag_id = Id::new(3).unwrap();
//! let near = Bag {
//!     id: bag_id,
//!     pose: Pose::new(Point::new(Mm::new(1000), Mm::new(0))),
//!     kind: BagKind::LightweightStiffVerticalHandles,
//!     visible: true,
//!     custody: Custody::Free,
//! };
//! let scene = Scene {
//!     robot: Point::new(Mm::new(0), Mm::new(0)),
//!     operator,
//!     carried: None,
//!     bags: std::slice::from_ref(&near),
//!     grasp_fails: false,
//!     handoff_fails: false,
//! };
//! assert_eq!(
//!     decide(&scene, Command::Pick { bag: bag_id }),
//!     Ok(Act::Grasped { bag: bag_id })
//! );
//! let far = Bag {
//!     pose: Pose::new(Point::new(Mm::new(2501), Mm::new(0))),
//!     ..near
//! };
//! let far_scene = Scene {
//!     bags: std::slice::from_ref(&far),
//!     ..scene
//! };
//! assert_eq!(
//!     decide(&far_scene, Command::Pick { bag: bag_id }),
//!     Err(GripError::OutOfReach)
//! );
//! ```

use domain::{Bag, Custody, Id, Point, within_reach};

pub mod assume {
    //! Gripper stand-ins. No constant here is a printed measurement.

    /// Inclusive planar reach from the robot base, in millimeters.
    ///
    /// This covers the seeded bags. They sit on the horizontal through the
    /// operator at `(0, 1000)`, with `|x|` from 400 through 2000, and those
    /// points are inside 2500 mm of the robot at the origin. It does not cover
    /// every bag inside the printed 2 m of the operator. A bag at `(0, 3000)`
    /// is 2000 mm from that operator and 3000 mm from the robot, so it is
    /// outside this disk. Picking it means moving the base, which is later
    /// navigation. This is not an arm length.
    pub const REACH_MM: i32 = 2500;

    /// A dropped or held bag is recorded at the robot base. There is no gripper frame.
    pub const SITE: &str = "robot-base";
}

/// What the gripper was asked to do. Asking is not execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Pick { bag: Id },
    Drop { bag: Id },
    Release { bag: Id, to: Id },
}

/// A decision. Applying it is a later step, and it is not task confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Grasped { bag: Id },
    GraspFailed { bag: Id },
    Dropped { bag: Id, at: Point },
    Released { bag: Id, to: Id },
    HandoffFailed { bag: Id },
}

/// Why no action was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripError {
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
}

impl std::fmt::Display for GripError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GripError::Missing => "missing bag",
            GripError::Invisible => "invisible bag",
            GripError::OutOfReach => "out of reach",
            GripError::Occupied => "gripper occupied",
            GripError::Empty => "gripper empty",
            GripError::WrongId => "wrong id",
            GripError::Role => "id role",
            GripError::NotFree => "bag not free",
            GripError::WrongRecipient => "wrong recipient",
            GripError::Duplicate => "duplicate bag",
            GripError::FailedGrasp => "grasp failed",
            GripError::HandoffFailed => "handoff failed",
        })
    }
}

impl std::error::Error for GripError {}

/// The facts a decision may read. The slice is not modified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene<'a> {
    pub robot: Point,
    pub operator: Id,
    pub carried: Option<Id>,
    pub bags: &'a [Bag],
    pub grasp_fails: bool,
    pub handoff_fails: bool,
}

/// Chooses one gripper action, or refuses it.
///
/// `Ok` is not a world change. An injected failure is an [`Act`], because the
/// attempt was legal and did not succeed. A missing, hidden, occupied, or
/// unreachable bag is a refusal and comes before that injected failure.
pub fn decide(scene: &Scene<'_>, command: Command) -> Result<Act, GripError> {
    match command {
        Command::Pick { bag } => pick(scene, bag),
        Command::Drop { bag } => drop_held(scene, bag),
        Command::Release { bag, to } => release(scene, bag, to),
    }
}

fn locate(bags: &[Bag], id: Id) -> Result<usize, GripError> {
    let mut found = None;
    for (index, bag) in bags.iter().enumerate() {
        if bag.id == id {
            if found.is_some() {
                return Err(GripError::Duplicate);
            }
            found = Some(index);
        }
    }
    found.ok_or(GripError::Missing)
}

fn pick(scene: &Scene<'_>, bag: Id) -> Result<Act, GripError> {
    if bag == scene.operator {
        return Err(GripError::Role);
    }
    if scene.carried.is_some() {
        return Err(GripError::Occupied);
    }
    let item = &scene.bags[locate(scene.bags, bag)?];
    if item.custody != Custody::Free {
        return Err(GripError::NotFree);
    }
    if !item.visible {
        return Err(GripError::Invisible);
    }
    if !within_reach(scene.robot, item.pose.point(), assume::REACH_MM) {
        return Err(GripError::OutOfReach);
    }
    if scene
        .bags
        .iter()
        .any(|other| other.custody == Custody::Gripper)
    {
        return Err(GripError::Occupied);
    }
    if scene.grasp_fails {
        return Ok(Act::GraspFailed { bag });
    }
    Ok(Act::Grasped { bag })
}

fn drop_held(scene: &Scene<'_>, bag: Id) -> Result<Act, GripError> {
    if bag == scene.operator {
        return Err(GripError::Role);
    }
    let Some(carried) = scene.carried else {
        return Err(GripError::Empty);
    };
    if carried != bag {
        return Err(GripError::WrongId);
    }
    let item = &scene.bags[locate(scene.bags, bag)?];
    if item.custody != Custody::Gripper {
        return Err(GripError::NotFree);
    }
    Ok(Act::Dropped {
        bag,
        at: scene.robot,
    })
}

fn release(scene: &Scene<'_>, bag: Id, to: Id) -> Result<Act, GripError> {
    if to == bag || scene.bags.iter().any(|item| item.id == to) {
        return Err(GripError::Role);
    }
    if to != scene.operator {
        return Err(GripError::WrongRecipient);
    }
    let Some(carried) = scene.carried else {
        return Err(GripError::Empty);
    };
    if carried != bag {
        return Err(GripError::WrongId);
    }
    let item = &scene.bags[locate(scene.bags, bag)?];
    if item.custody != Custody::Gripper {
        return Err(GripError::NotFree);
    }
    if scene.handoff_fails {
        return Ok(Act::HandoffFailed { bag });
    }
    Ok(Act::Released { bag, to })
}

#[cfg(test)]
mod tests {
    use super::{Act, Command, GripError, Scene, assume, decide};
    use domain::{Bag, BagKind, Custody, Id, Mm, Point, Pose, within_reach};

    fn id(raw: u32) -> Id {
        Id::new(raw).expect("id")
    }

    fn bag(raw: u32, x: i32, y: i32, visible: bool, custody: Custody) -> Bag {
        Bag {
            id: id(raw),
            pose: Pose::new(Point::new(Mm::new(x), Mm::new(y))),
            kind: BagKind::LightweightStiffVerticalHandles,
            visible,
            custody,
        }
    }

    fn open<'a>(bags: &'a [Bag]) -> Scene<'a> {
        Scene {
            robot: Point::new(Mm::new(0), Mm::new(0)),
            operator: id(2),
            carried: None,
            bags,
            grasp_fails: false,
            handoff_fails: false,
        }
    }

    #[test]
    fn a_reachable_pick_is_a_decision_and_leaves_the_bag_free() {
        let bags = [bag(3, 1000, 0, true, Custody::Free)];
        let before = bags.clone();
        let scene = open(&bags);
        assert_eq!(assume::REACH_MM, 2500);
        assert_eq!(assume::SITE, "robot-base");
        assert_eq!(
            decide(&scene, Command::Pick { bag: id(3) }),
            Ok(Act::Grasped { bag: id(3) })
        );
        assert_eq!(bags, before);
        assert_eq!(bags[0].custody, Custody::Free);
        assert_ne!(
            decide(&scene, Command::Pick { bag: id(4) }),
            Ok(Act::Grasped { bag: id(3) })
        );
    }

    #[test]
    fn the_reach_boundary_is_inclusive_and_one_millimeter_outside_is_refused() {
        let origin = Point::new(Mm::new(0), Mm::new(0));
        let cases = [
            (2500, 0, true),
            (2501, 0, false),
            (-2500, 0, true),
            (0, -2500, true),
            (0, 2501, false),
            (1500, 2000, true),
            (1500, 2001, false),
            (-1500, -2000, true),
            (-1500, -2001, false),
        ];
        for (x, y, inside) in cases {
            let bags = [bag(3, x, y, true, Custody::Free)];
            let scene = open(&bags);
            assert_eq!(
                within_reach(origin, bags[0].pose.point(), assume::REACH_MM),
                inside,
                "{x},{y}"
            );
            let result = decide(&scene, Command::Pick { bag: id(3) });
            if inside {
                assert_eq!(result, Ok(Act::Grasped { bag: id(3) }), "{x},{y}");
            } else {
                assert_eq!(result, Err(GripError::OutOfReach), "{x},{y}");
            }
            assert_eq!(bags[0].custody, Custody::Free);
        }
    }

    #[test]
    fn a_bounded_grid_matches_the_disk() {
        let origin = Point::new(Mm::new(0), Mm::new(0));
        let xs = [-2501, -2500, -1, 0, 1, 2500, 2501];
        let ys = [-2501, -2500, 0, 2500, 2501];
        for x in xs {
            for y in ys {
                let bags = [bag(4, x, y, true, Custody::Free)];
                let inside = within_reach(origin, bags[0].pose.point(), assume::REACH_MM);
                let result = decide(&open(&bags), Command::Pick { bag: id(4) });
                if inside {
                    assert_eq!(result, Ok(Act::Grasped { bag: id(4) }), "{x},{y}");
                } else {
                    assert_eq!(result, Err(GripError::OutOfReach), "{x},{y}");
                }
            }
        }
        let extreme = [bag(4, i32::MAX, i32::MIN, true, Custody::Free)];
        assert_eq!(
            decide(&open(&extreme), Command::Pick { bag: id(4) }),
            Err(GripError::OutOfReach)
        );
    }

    #[test]
    fn hidden_missing_role_occupied_and_not_free_are_refusals() {
        let hidden = [bag(3, 1000, 0, false, Custody::Free)];
        assert_eq!(
            decide(&open(&hidden), Command::Pick { bag: id(3) }),
            Err(GripError::Invisible)
        );
        let visible = [bag(3, 1000, 0, true, Custody::Free)];
        assert_eq!(
            decide(&open(&visible), Command::Pick { bag: id(9) }),
            Err(GripError::Missing)
        );
        assert_eq!(
            decide(&open(&visible), Command::Pick { bag: id(2) }),
            Err(GripError::Role)
        );
        let mut occupied = open(&visible);
        occupied.carried = Some(id(3));
        assert_eq!(
            decide(&occupied, Command::Pick { bag: id(3) }),
            Err(GripError::Occupied)
        );
        let held = [bag(3, 0, 0, true, Custody::Gripper)];
        assert_eq!(
            decide(&open(&held), Command::Pick { bag: id(3) }),
            Err(GripError::NotFree)
        );
        let owned = [bag(3, 0, 1000, true, Custody::Operator)];
        assert_eq!(
            decide(&open(&owned), Command::Pick { bag: id(3) }),
            Err(GripError::NotFree)
        );
    }

    #[test]
    fn an_injected_grasp_failure_does_not_replace_an_earlier_refusal() {
        let bags = [bag(3, 800, 0, true, Custody::Free)];
        let mut scene = open(&bags);
        scene.grasp_fails = true;
        assert_eq!(
            decide(&scene, Command::Pick { bag: id(3) }),
            Ok(Act::GraspFailed { bag: id(3) })
        );
        assert_ne!(
            decide(&scene, Command::Pick { bag: id(3) }),
            Ok(Act::Grasped { bag: id(3) })
        );
        let far = [bag(3, 2501, 0, true, Custody::Free)];
        let mut far_scene = open(&far);
        far_scene.grasp_fails = true;
        assert_eq!(
            decide(&far_scene, Command::Pick { bag: id(3) }),
            Err(GripError::OutOfReach)
        );
        let hidden = [bag(3, 800, 0, false, Custody::Free)];
        let mut hidden_scene = open(&hidden);
        hidden_scene.grasp_fails = true;
        assert_eq!(
            decide(&hidden_scene, Command::Pick { bag: id(3) }),
            Err(GripError::Invisible)
        );
    }

    #[test]
    fn duplicate_ids_are_not_a_grasp() {
        let bags = [
            bag(3, 500, 0, true, Custody::Free),
            bag(3, 700, 0, true, Custody::Free),
        ];
        assert_eq!(
            decide(&open(&bags), Command::Pick { bag: id(3) }),
            Err(GripError::Duplicate)
        );
    }

    #[test]
    fn drop_names_the_robot_point_only_for_the_held_bag() {
        let bags = [bag(3, 0, 0, true, Custody::Gripper)];
        let mut scene = open(&bags);
        scene.carried = Some(id(3));
        scene.robot = Point::new(Mm::new(0), Mm::new(0));
        assert_eq!(
            decide(&scene, Command::Drop { bag: id(3) }),
            Ok(Act::Dropped {
                bag: id(3),
                at: scene.robot
            })
        );
        let empty = open(&bags);
        assert_eq!(
            decide(&empty, Command::Drop { bag: id(3) }),
            Err(GripError::Empty)
        );
        scene.carried = Some(id(4));
        assert_eq!(
            decide(&scene, Command::Drop { bag: id(3) }),
            Err(GripError::WrongId)
        );
        scene.carried = Some(id(3));
        assert_eq!(
            decide(&scene, Command::Drop { bag: id(2) }),
            Err(GripError::Role)
        );
    }

    #[test]
    fn release_requires_the_operator_and_the_held_bag() {
        let bags = [bag(3, 0, 0, true, Custody::Gripper)];
        let mut scene = open(&bags);
        scene.carried = Some(id(3));
        assert_eq!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(2)
                }
            ),
            Ok(Act::Released {
                bag: id(3),
                to: id(2)
            })
        );
        assert_eq!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(3)
                }
            ),
            Err(GripError::Role)
        );
        assert_eq!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(8)
                }
            ),
            Err(GripError::WrongRecipient)
        );
        scene.carried = None;
        assert_eq!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(2)
                }
            ),
            Err(GripError::Empty)
        );
        scene.carried = Some(id(3));
        scene.handoff_fails = true;
        assert_eq!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(2)
                }
            ),
            Ok(Act::HandoffFailed { bag: id(3) })
        );
        assert_ne!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(2)
                }
            ),
            Ok(Act::Released {
                bag: id(3),
                to: id(2)
            })
        );
        scene.handoff_fails = true;
        assert_eq!(
            decide(
                &scene,
                Command::Release {
                    bag: id(3),
                    to: id(8)
                }
            ),
            Err(GripError::WrongRecipient)
        );
    }
}
