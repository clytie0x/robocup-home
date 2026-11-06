//! One observed step toward a person or a bag standoff.
//!
//! The car coordinate is not an input. Lengths in [`assume`] are not printed
//! measurements. A plan is not motion and not task confirmation.
//!
//! ```
//! use domain::{Id, Mm, Point};
//! use navigation::{Cue, Follower, Intent, SeenPerson, Sight, plan_follow};
//! let person = Id::new(2).unwrap();
//! let follower = Follower::follow(person);
//! let people = [SeenPerson {
//!     id: person,
//!     at: Point::new(Mm::new(0), Mm::new(1000)),
//! }];
//! let sight = Sight {
//!     robot: Point::new(Mm::new(0), Mm::new(0)),
//!     people: &people,
//!     bodies: &[],
//! };
//! let (_next, intent) = plan_follow(&follower, &sight, Cue::None);
//! assert_ne!(intent, Intent::Arrived);
//! ```

use domain::{Actor, Id, Mm, Obstacle, ObstacleClass, Point, segment_within, within_reach};

pub mod assume {
    //! Follow stand-ins. No constant here is a printed speed, radius, or sensor range.

    /// Disk carried around the base. Not a measured robot.
    pub const ROBOT_RADIUS_MM: i32 = 250;
    /// Preferred gap from the operator, on the side opposite their last step.
    pub const FOLLOW_MM: i32 = 800;
    /// Proximity for a component take-bag cue. Not a printed distance and not the car.
    ///
    /// Leaving the start poses and a clear observed segment are fixture gates.
    /// They can still set the trip flag after one short step. That flag is not
    /// arrival at the unknown car. The episode runner checks the operator's
    /// designated place separately and does not pass that place here.
    pub const ARRIVE_MM: i32 = 1200;
    /// Longest geometric step one plan may propose.
    pub const STEP_MM: i32 = 200;
    /// Hold still when the standoff is already this close.
    pub const HOLD_MM: i32 = 40;
    /// Local sensor. Obstacles farther than this are absent from the plan.
    pub const SENSOR_MM: i32 = 2500;
    /// Shorter range for the hard-to-see class. Still a model, not a contrast measure.
    pub const HARD_SENSOR_MM: i32 = 800;
    /// Consecutive blocked plans before the follower reports stuck.
    pub const STUCK_LIMIT: u8 = 8;
    /// Stop this far inside the gripper reach so the pick disk still contains the bag.
    pub const APPROACH_MARGIN_MM: i32 = 100;
    /// Sixteen headings. Not a search framework.
    pub const MODEL: &str = "standoff-headings-v1";
}

/// Observed cue. `TakeBag` is an explicit operator signal, not a car coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    None,
    TakeBag,
}

/// A person the sensor reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeenPerson {
    pub id: Id,
    pub at: Point,
}

/// A disk the sensor reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeenBody {
    pub center: Point,
    pub radius_mm: i32,
}

/// Local view. There is no route and no car.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sight<'a> {
    pub robot: Point,
    pub people: &'a [SeenPerson],
    pub bodies: &'a [SeenBody],
}

/// Memory for one followed id. It does not store a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Follower {
    person: Id,
    previous: Option<Point>,
    blocked_run: u8,
    cancelled: bool,
    /// Which way the last detour turned. Zero means the straight step was free.
    side: i8,
    /// Base pose on the first sight of the person. Arrival requires leaving it.
    home_robot: Option<Point>,
    /// That person's pose on the same sight. Arrival requires a later pose.
    home_person: Option<Point>,
}

impl Follower {
    pub const fn follow(person: Id) -> Self {
        Self {
            person,
            previous: None,
            blocked_run: 0,
            cancelled: false,
            side: 0,
            home_robot: None,
            home_person: None,
        }
    }

    pub const fn cancel(mut self) -> Self {
        self.cancelled = true;
        self
    }

    pub const fn person(self) -> Id {
        self.person
    }

    pub const fn blocked_run(self) -> u8 {
        self.blocked_run
    }

    pub const fn cancelled(self) -> bool {
        self.cancelled
    }

    /// Counts one blocked physical step. The pose is the caller's.
    pub const fn note_blocked(mut self) -> (Self, Intent) {
        self.blocked_run = self.blocked_run.saturating_add(1);
        if self.blocked_run >= assume::STUCK_LIMIT {
            (self, Intent::Stuck)
        } else {
            (self, Intent::Blocked)
        }
    }
}

/// What the next geometric step would be. `Move` is a proposal, not a teleport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Move(Point),
    Stay,
    Blocked,
    Stuck,
    Lost,
    WrongPerson,
    Arrived,
    InReach,
    Cancelled,
}

const HEADINGS: [(i32, i32); 16] = [
    (1024, 0),
    (946, 392),
    (724, 724),
    (392, 946),
    (0, 1024),
    (-392, 946),
    (-724, 724),
    (-946, 392),
    (-1024, 0),
    (-946, -392),
    (-724, -724),
    (-392, -946),
    (0, -1024),
    (392, -946),
    (724, -724),
    (946, -392),
];

/// Plans one follow step from `sight`.
pub fn plan_follow(state: &Follower, sight: &Sight<'_>, cue: Cue) -> (Follower, Intent) {
    if state.cancelled {
        return (*state, Intent::Cancelled);
    }
    let Some(target) = sight.people.iter().find(|person| person.id == state.person) else {
        let mut next = *state;
        next.blocked_run = 0;
        let intent = if sight.people.is_empty() {
            Intent::Lost
        } else {
            Intent::WrongPerson
        };
        return (next, intent);
    };
    let mut next = *state;
    if next.home_robot.is_none() {
        next.home_robot = Some(sight.robot);
    }
    if next.home_person.is_none() {
        next.home_person = Some(target.at);
    }
    let left_home = sight.robot != next.home_robot.unwrap_or(sight.robot);
    let led_away = target.at != next.home_person.unwrap_or(target.at);
    let clear_to_person = segment_clear(sight.robot, target.at, sight.bodies);
    if cue == Cue::TakeBag
        && left_home
        && led_away
        && clear_to_person
        && within_reach(sight.robot, target.at, assume::ARRIVE_MM)
    {
        next.previous = Some(target.at);
        next.blocked_run = 0;
        return (next, Intent::Arrived);
    }
    let goal = behind_point(sight.robot, target.at, state.previous);
    next.previous = Some(target.at);
    match choose_step(sight.robot, goal, sight.bodies, state.side) {
        Some((to, side)) if to == sight.robot => {
            next.blocked_run = 0;
            next.side = side;
            (next, Intent::Stay)
        }
        Some((to, side)) => {
            next.blocked_run = 0;
            next.side = side;
            (next, Intent::Move(to))
        }
        None => {
            next.blocked_run = state.blocked_run.saturating_add(1);
            next.side = state.side;
            if next.blocked_run >= assume::STUCK_LIMIT {
                (next, Intent::Stuck)
            } else {
                (next, Intent::Blocked)
            }
        }
    }
}

/// Plans one step that brings `robot` into `reach_mm` of `bag` without enlarging that reach.
pub fn plan_approach(robot: Point, bag: Point, bodies: &[SeenBody], reach_mm: i32) -> Intent {
    if within_reach(robot, bag, reach_mm) {
        return Intent::InReach;
    }
    let standoff = reach_mm.saturating_sub(assume::APPROACH_MARGIN_MM).max(0);
    let dx = i128::from(robot.x().get()) - i128::from(bag.x().get());
    let dy = i128::from(robot.y().get()) - i128::from(bag.y().get());
    let Some(goal) = scale_from(bag, dx, dy, standoff) else {
        return Intent::Blocked;
    };
    match choose_step(robot, goal, bodies, 0) {
        Some((to, _)) if to == robot => Intent::Stay,
        Some((to, _)) => Intent::Move(to),
        None => Intent::Blocked,
    }
}

/// Filters `obstacles` to the local sensor. The hard-to-see class uses the shorter range.
pub fn look(
    robot: Point,
    operator: Actor,
    obstacles: &[Obstacle],
) -> (Vec<SeenPerson>, Vec<SeenBody>) {
    let mut people = Vec::new();
    if within_reach(robot, operator.pose.point(), assume::SENSOR_MM) {
        people.push(SeenPerson {
            id: operator.id,
            at: operator.pose.point(),
        });
    }
    let mut bodies = Vec::new();
    for obstacle in obstacles {
        let range = if obstacle.class == ObstacleClass::HardToSee {
            assume::HARD_SENSOR_MM
        } else {
            assume::SENSOR_MM
        };
        for disk in &obstacle.bodies {
            if !within_reach(robot, disk.center, range) {
                continue;
            }
            bodies.push(SeenBody {
                center: disk.center,
                radius_mm: disk.radius.get(),
            });
            if obstacle.class == ObstacleClass::Crowd {
                people.push(SeenPerson {
                    id: disk.id,
                    at: disk.center,
                });
            }
        }
    }
    (people, bodies)
}

fn behind_point(robot: Point, operator: Point, previous: Option<Point>) -> Point {
    let (dx, dy) = match previous {
        Some(prev) if prev != operator => {
            let hx = i128::from(operator.x().get()) - i128::from(prev.x().get());
            let hy = i128::from(operator.y().get()) - i128::from(prev.y().get());
            (-hx, -hy)
        }
        _ => {
            let hx = i128::from(robot.x().get()) - i128::from(operator.x().get());
            let hy = i128::from(robot.y().get()) - i128::from(operator.y().get());
            if hx == 0 && hy == 0 {
                (0, -1)
            } else {
                (hx, hy)
            }
        }
    };
    scale_from(operator, dx, dy, assume::FOLLOW_MM).unwrap_or(operator)
}

fn choose_step(from: Point, goal: Point, bodies: &[SeenBody], side: i8) -> Option<(Point, i8)> {
    let hold = i128::from(assume::HOLD_MM);
    if dist_sq(from, goal) <= hold * hold {
        return Some((from, 0));
    }
    if let Some(direct) = headed_step(from, goal, 1024, 0)
        && direct != from
        && segment_clear(from, direct, bodies)
    {
        return Some((direct, 0));
    }
    let mut best: Option<(i128, usize, Point, i128)> = None;
    let mut index = 0usize;
    for disk in bodies {
        let limit = assume::ROBOT_RADIUS_MM.saturating_add(disk.radius_mm);
        for sign in [1, -1] {
            index += 1;
            let Some(candidate) = tangent_step(from, disk.center, limit, sign) else {
                continue;
            };
            consider(from, goal, bodies, candidate, index, side, &mut best);
        }
    }
    for (cos, sin) in HEADINGS {
        index += 1;
        let Some(candidate) = headed_step(from, goal, cos, sin) else {
            continue;
        };
        consider(from, goal, bodies, candidate, index, side, &mut best);
    }
    best.map(|(_, _, point, cross)| {
        let chosen = if side == 0 {
            sign_of(cross).unwrap_or(1)
        } else {
            side
        };
        (point, chosen)
    })
}

fn consider(
    from: Point,
    goal: Point,
    bodies: &[SeenBody],
    candidate: Point,
    index: usize,
    side: i8,
    best: &mut Option<(i128, usize, Point, i128)>,
) {
    if candidate == from || !segment_clear(from, candidate, bodies) {
        return;
    }
    let cap = i128::from(assume::STEP_MM);
    if dist_sq(from, candidate) > cap * cap {
        return;
    }
    let cross = cross_turn(from, goal, candidate);
    if side != 0 && cross != 0 && sign_of(cross) != Some(side) {
        return;
    }
    let score = dist_sq(candidate, goal);
    let replace = best
        .map(|(best_score, best_index, _, _)| (score, index) < (best_score, best_index))
        .unwrap_or(true);
    if replace {
        *best = Some((score, index, candidate, cross));
    }
}

fn cross_turn(from: Point, goal: Point, candidate: Point) -> i128 {
    let gx = i128::from(goal.x().get()) - i128::from(from.x().get());
    let gy = i128::from(goal.y().get()) - i128::from(from.y().get());
    let sx = i128::from(candidate.x().get()) - i128::from(from.x().get());
    let sy = i128::from(candidate.y().get()) - i128::from(from.y().get());
    gx * sy - gy * sx
}

fn sign_of(value: i128) -> Option<i8> {
    match value.cmp(&0) {
        std::cmp::Ordering::Greater => Some(1),
        std::cmp::Ordering::Less => Some(-1),
        std::cmp::Ordering::Equal => None,
    }
}

/// One step along a tangent of the grown disk. `sign` picks a side.
fn tangent_step(from: Point, center: Point, radius: i32, sign: i32) -> Option<Point> {
    let dx = i128::from(center.x().get()) - i128::from(from.x().get());
    let dy = i128::from(center.y().get()) - i128::from(from.y().get());
    let l2 = dx * dx + dy * dy;
    if l2 == 0 || radius <= 0 {
        return None;
    }
    let radius = i128::from(radius);
    let r2 = radius * radius;
    if l2 <= r2 {
        return step_along(from, -dx, -dy, assume::STEP_MM);
    }
    let h = i128::try_from(isqrt(u128::try_from(l2 - r2).ok()?)).ok()?;
    let s = i128::from(sign);
    let tx = dx * h - s * radius * dy;
    let ty = dy * h + s * radius * dx;
    step_along(from, tx, ty, assume::STEP_MM)
}

fn headed_step(from: Point, goal: Point, cos: i32, sin: i32) -> Option<Point> {
    let dx = i128::from(goal.x().get()) - i128::from(from.x().get());
    let dy = i128::from(goal.y().get()) - i128::from(from.y().get());
    if dx == 0 && dy == 0 {
        return Some(from);
    }
    if cos == 1024 && sin == 0 {
        let len = isqrt(square_len(dx, dy)?);
        let travel = i32::try_from(len).unwrap_or(i32::MAX).min(assume::STEP_MM);
        return step_along(from, dx, dy, travel);
    }
    let seed = step_along(from, dx, dy, 1024)?;
    let sx = i128::from(seed.x().get()) - i128::from(from.x().get());
    let sy = i128::from(seed.y().get()) - i128::from(from.y().get());
    let rx = sx * i128::from(cos) - sy * i128::from(sin);
    let ry = sx * i128::from(sin) + sy * i128::from(cos);
    step_along(from, rx, ry, assume::STEP_MM)
}

/// A step of at most `travel` mm along `(dx, dy)`. The length is never longer than `travel`.
fn step_along(from: Point, mut dx: i128, mut dy: i128, travel: i32) -> Option<Point> {
    if travel == 0 {
        return Some(from);
    }
    if travel < 0 {
        return None;
    }
    if dx == 0 && dy == 0 {
        return Some(from);
    }
    let bound = 1u128 << 62;
    while dx.unsigned_abs() > bound || dy.unsigned_abs() > bound {
        dx /= 2;
        dy /= 2;
        if dx == 0 && dy == 0 {
            return None;
        }
    }
    let len2 = square_len(dx, dy)?;
    let len = isqrt(len2);
    if len == 0 {
        return None;
    }
    let ceil = if len.saturating_mul(len) == len2 {
        len
    } else {
        len.saturating_add(1)
    };
    let ceil = i128::try_from(ceil).ok()?;
    let span = i128::from(travel);
    let ox = dx.checked_mul(span)?.checked_div(ceil)?;
    let oy = dy.checked_mul(span)?.checked_div(ceil)?;
    let to = from
        .checked_offset(
            Mm::new(i32::try_from(ox).ok()?),
            Mm::new(i32::try_from(oy).ok()?),
        )
        .ok()?;
    let cap = span * span;
    (dist_sq(from, to) <= cap).then_some(to)
}

fn square_len(dx: i128, dy: i128) -> Option<u128> {
    let x = dx.unsigned_abs();
    let y = dy.unsigned_abs();
    x.checked_mul(x)?.checked_add(y.checked_mul(y)?)
}

fn segment_clear(from: Point, to: Point, bodies: &[SeenBody]) -> bool {
    bodies.iter().all(|disk| {
        let limit = assume::ROBOT_RADIUS_MM.saturating_add(disk.radius_mm);
        !segment_within(from, to, disk.center, limit)
    })
}

fn scale_from(origin: Point, dx: i128, dy: i128, dist: i32) -> Option<Point> {
    step_along(origin, dx, dy, dist)
}

fn dist_sq(a: Point, b: Point) -> i128 {
    let dx = i128::from(a.x().get()) - i128::from(b.x().get());
    let dy = i128::from(a.y().get()) - i128::from(b.y().get());
    dx * dx + dy * dy
}

fn isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let shift = (127 - n.leading_zeros()) / 2 + 1;
    let mut x = 1u128 << shift;
    loop {
        let next = (x + n / x) >> 1;
        if next >= x {
            return x;
        }
        x = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Disk, Id, Mm, Obstacle, ObstacleClass, Point, Pose, segment_within};

    fn id(raw: u32) -> Id {
        Id::new(raw).expect("id")
    }

    fn at(x: i32, y: i32) -> Point {
        Point::new(Mm::new(x), Mm::new(y))
    }

    fn person(raw: u32, x: i32, y: i32) -> SeenPerson {
        SeenPerson {
            id: id(raw),
            at: at(x, y),
        }
    }

    fn body(x: i32, y: i32, radius: i32) -> SeenBody {
        SeenBody {
            center: at(x, y),
            radius_mm: radius,
        }
    }

    fn clear(from: Point, to: Point, bodies: &[SeenBody]) -> bool {
        let limit_gap = |disk: &SeenBody| assume::ROBOT_RADIUS_MM.saturating_add(disk.radius_mm);
        bodies
            .iter()
            .all(|disk| !segment_within(from, to, disk.center, limit_gap(disk)))
    }

    fn moved(intent: Intent) -> Point {
        match intent {
            Intent::Move(to) => to,
            other => panic!("expected a step, got {other:?}"),
        }
    }

    #[test]
    fn a_follow_plan_does_not_arrive_without_the_cue() {
        let follower = Follower::follow(id(2));
        let people = [person(2, 0, 1000)];
        let sight = Sight {
            robot: at(0, 0),
            people: &people,
            bodies: &[],
        };
        let (_next, intent) = plan_follow(&follower, &sight, Cue::None);
        assert_ne!(intent, Intent::Arrived);
        assert_eq!(assume::MODEL, "standoff-headings-v1");
        assert_eq!(assume::ROBOT_RADIUS_MM, 250);
        assert_eq!(assume::FOLLOW_MM, 800);
        assert_eq!(assume::ARRIVE_MM, 1200);
        assert_eq!(assume::STEP_MM, 200);
        let to = moved(intent);
        let step = dist_sq(at(0, 0), to);
        let cap = i128::from(assume::STEP_MM);
        assert!(step <= cap * cap);
        assert!(to.y().get() > 0);
    }

    #[test]
    fn the_followed_id_wins_over_a_nearer_stranger() {
        let people = [person(5, -400, 0), person(2, 0, 3000)];
        let sight = Sight {
            robot: at(0, 0),
            people: &people,
            bodies: &[],
        };
        let (_next, intent) = plan_follow(&Follower::follow(id(2)), &sight, Cue::None);
        let to = moved(intent);
        assert_eq!(to.x().get(), 0);
        assert!(to.y().get() > 0);
        assert_ne!(to, people[0].at);
    }

    #[test]
    fn a_missing_operator_is_lost_and_another_person_is_not_followed() {
        let nobody = Sight {
            robot: at(0, 0),
            people: &[],
            bodies: &[],
        };
        assert_eq!(
            plan_follow(&Follower::follow(id(2)), &nobody, Cue::TakeBag).1,
            Intent::Lost
        );
        let stranger = [person(5, 100, 0)];
        let wrong = Sight {
            robot: at(0, 0),
            people: &stranger,
            bodies: &[],
        };
        let (next, intent) = plan_follow(&Follower::follow(id(2)), &wrong, Cue::TakeBag);
        assert_eq!(intent, Intent::WrongPerson);
        assert_eq!(next.person(), id(2));
        assert_ne!(intent, Intent::Move(at(100, 0)));
    }

    #[test]
    fn the_cue_arrives_only_when_the_same_person_is_near() {
        let near = [person(2, 0, 1000)];
        let sight = Sight {
            robot: at(0, 0),
            people: &near,
            bodies: &[],
        };
        let (armed, intent) = plan_follow(&Follower::follow(id(2)), &sight, Cue::TakeBag);
        assert_ne!(intent, Intent::Arrived);
        let (still, intent) = plan_follow(&armed, &sight, Cue::TakeBag);
        assert_ne!(intent, Intent::Arrived);
        let walked_person = [person(2, 0, 1600)];
        let walked = Sight {
            robot: at(0, 400),
            people: &walked_person,
            bodies: &[],
        };
        assert_eq!(
            plan_follow(&still, &walked, Cue::TakeBag).1,
            Intent::Arrived
        );
        let origin_person = [person(2, -400, 1000)];
        let origin = Sight {
            robot: at(-400, 0),
            people: &origin_person,
            bodies: &[],
        };
        let (moved_home, _) = plan_follow(&Follower::follow(id(2)), &origin, Cue::None);
        let blocked = [body(0, 500, 40)];
        let blocked_person = [person(2, 0, 1000)];
        let walled = Sight {
            robot: at(0, 0),
            people: &blocked_person,
            bodies: &blocked,
        };
        assert_ne!(
            plan_follow(&moved_home, &walled, Cue::TakeBag).1,
            Intent::Arrived
        );
        let far = [person(2, 0, 5000)];
        let distant = Sight {
            robot: at(0, 0),
            people: &far,
            bodies: &[],
        };
        assert_ne!(
            plan_follow(&Follower::follow(id(2)), &distant, Cue::TakeBag).1,
            Intent::Arrived
        );
        let cancelled = Follower::follow(id(2)).cancel();
        assert_eq!(
            plan_follow(&cancelled, &sight, Cue::TakeBag).1,
            Intent::Cancelled
        );
    }

    #[test]
    fn a_straight_step_cannot_tunnel_between_clear_ends() {
        let people = [person(2, 5000, 0)];
        let bodies = [body(100, 269, 20)];
        let from = at(0, 0);
        let sight = Sight {
            robot: from,
            people: &people,
            bodies: &bodies,
        };
        let straight = at(assume::STEP_MM, 0);
        assert!(
            !clear(from, straight, &bodies),
            "the fixture segment must be a real hit"
        );
        let (_next, intent) = plan_follow(&Follower::follow(id(2)), &sight, Cue::None);
        match intent {
            Intent::Move(to) => {
                assert!(clear(from, to, &bodies));
                let cap = i128::from(assume::STEP_MM);
                assert!(dist_sq(from, to) <= cap * cap);
                assert_ne!(to, people[0].at);
            }
            Intent::Blocked | Intent::Stuck => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn blocked_headings_count_to_stuck_and_do_not_loop_inside_one_plan() {
        let mut bodies = Vec::new();
        for (cos, sin) in HEADINGS {
            let x = i32::try_from(i64::from(450) * i64::from(cos) / 1024).expect("x");
            let y = i32::try_from(i64::from(450) * i64::from(sin) / 1024).expect("y");
            bodies.push(body(x, y, 180));
        }
        let people = [person(2, 5000, 0)];
        let mut follower = Follower::follow(id(2));
        let started = std::time::Instant::now();
        for expected in 1..assume::STUCK_LIMIT {
            let sight = Sight {
                robot: at(0, 0),
                people: &people,
                bodies: &bodies,
            };
            let (next, intent) = plan_follow(&follower, &sight, Cue::None);
            assert_eq!(intent, Intent::Blocked, "{expected}");
            assert_eq!(next.blocked_run(), expected);
            follower = next;
        }
        let sight = Sight {
            robot: at(0, 0),
            people: &people,
            bodies: &bodies,
        };
        let (next, intent) = plan_follow(&follower, &sight, Cue::None);
        assert_eq!(intent, Intent::Stuck);
        assert_eq!(next.blocked_run(), assume::STUCK_LIMIT);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        let mut memory = Follower::follow(id(2));
        for _ in 1..assume::STUCK_LIMIT {
            let (next, intent) = memory.note_blocked();
            assert_eq!(intent, Intent::Blocked);
            memory = next;
        }
        assert_eq!(memory.note_blocked().1, Intent::Stuck);
    }

    fn walk(bodies: &[SeenBody], steps: i32) -> Point {
        let mut robot = at(0, 0);
        let mut follower = Follower::follow(id(2));
        for step in 1..=steps {
            let people = [person(2, 800 + step * 200, 0)];
            let sight = Sight {
                robot,
                people: &people,
                bodies,
            };
            let (next, intent) = plan_follow(&follower, &sight, Cue::None);
            assert_ne!(intent, Intent::Arrived, "step {step}");
            assert_ne!(intent, Intent::Lost, "step {step}");
            assert_ne!(intent, Intent::WrongPerson, "step {step}");
            assert_ne!(intent, Intent::Stuck, "step {step}");
            if let Intent::Move(to) = intent {
                assert!(clear(robot, to, bodies), "{robot:?} -> {to:?}");
                let cap = i128::from(assume::STEP_MM);
                assert!(dist_sq(robot, to) <= cap * cap);
                robot = to;
            }
            follower = next;
        }
        robot
    }

    fn apart(robot: Point, bodies: &[SeenBody]) {
        for disk in bodies {
            let limit = i128::from(assume::ROBOT_RADIUS_MM + disk.radius_mm);
            assert!(
                dist_sq(robot, disk.center) >= limit * limit,
                "robot {robot:?} disk {disk:?}"
            );
        }
    }

    #[test]
    fn each_obstacle_class_is_skirted_and_order_does_not_open_a_tunnel() {
        let small = [body(1600, 0, 80)];
        let hard = [body(1600, 0, 60)];
        let barrier = [body(1800, 0, 350)];
        let crowd = [
            body(1800, -600, 200),
            body(1800, 0, 200),
            body(1800, 600, 200),
        ];
        let crowd_four = [
            body(1800, -900, 200),
            body(1800, -300, 200),
            body(1800, 300, 200),
            body(1800, 900, 200),
        ];
        for bodies in [
            &small[..],
            &hard[..],
            &barrier[..],
            &crowd[..],
            &crowd_four[..],
        ] {
            let robot = walk(bodies, 40);
            apart(robot, bodies);
            assert!(
                robot.x().get() > 2200,
                "did not follow past the obstacle, ended at {robot:?}"
            );
        }
        let mut combined = vec![
            body(1400, 0, 80),
            body(2200, 40, 60),
            body(3200, 0, 350),
            body(4600, -600, 200),
            body(4600, 0, 200),
            body(4600, 600, 200),
        ];
        let forward = walk(&combined, 50);
        apart(forward, &combined);
        assert!(forward.x().get() > 5000, "{forward:?}");
        combined.reverse();
        let backward = walk(&combined, 50);
        apart(backward, &combined);
        assert!(backward.x().get() > 5000, "{backward:?}");
    }

    #[test]
    fn approach_stops_inside_the_given_reach_and_does_not_enlarge_it() {
        let bag = at(0, 3000);
        let reach = 2500;
        assert!(!within_reach(at(0, 0), bag, reach));
        assert_eq!(
            plan_approach(at(0, 0), bag, &[], reach),
            Intent::Move(at(0, assume::STEP_MM))
        );
        let mut robot = at(0, 0);
        for _ in 0..20 {
            match plan_approach(robot, bag, &[], reach) {
                Intent::InReach => break,
                Intent::Move(to) => {
                    assert!(to.y().get() > robot.y().get());
                    assert!(to.y().get() < bag.y().get());
                    let cap = i128::from(assume::STEP_MM);
                    assert!(dist_sq(robot, to) <= cap * cap);
                    robot = to;
                }
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(plan_approach(robot, bag, &[], reach), Intent::InReach);
        assert!(within_reach(robot, bag, reach));
        assert!(!within_reach(at(0, 0), bag, reach));
        assert!(!within_reach(robot, bag, 0));
        let wall = [body(0, 400, 80)];
        let intent = plan_approach(at(0, 0), bag, &wall, reach);
        match intent {
            Intent::Move(to) => assert!(clear(at(0, 0), to, &wall)),
            Intent::Blocked | Intent::Stuck => {}
            other => panic!("{other:?}"),
        }
    }

    fn step_len_sq(from: Point, to: Point) -> i128 {
        dist_sq(from, to)
    }

    fn assert_capped(from: Point, to: Point) {
        let cap = i128::from(assume::STEP_MM);
        assert!(
            step_len_sq(from, to) <= cap * cap,
            "{from:?} -> {to:?} exceeds {cap}"
        );
    }

    #[test]
    fn approach_toward_a_far_bag_stays_on_axis_and_inside_the_step() {
        let cap = i128::from(assume::STEP_MM);
        for x in [2_968_220, 2_968_221, 3_000_000, 5_000_000, i32::MAX] {
            let intent = plan_approach(at(0, 0), at(x, 0), &[], 2500);
            let Intent::Move(to) = intent else {
                panic!("{x} {intent:?}");
            };
            assert!(to.x().get() > 0, "{x} {to:?}");
            assert_eq!(to.y().get(), 0, "{x} {to:?}");
            assert!(step_len_sq(at(0, 0), to) <= cap * cap, "{x} {to:?}");
        }
        let intent = plan_approach(at(0, 0), at(i32::MIN, 0), &[], 2500);
        let Intent::Move(to) = intent else {
            panic!("{intent:?}");
        };
        assert!(to.x().get() < 0, "{to:?}");
        assert_eq!(to.y().get(), 0, "{to:?}");
        assert_capped(at(0, 0), to);
    }

    #[test]
    fn a_direct_follow_step_at_extreme_coordinates_stays_capped() {
        let robot = at(-411_794_476, -373_727_457);
        let people = [person(2, 239_722_261, 173_842_438)];
        let bodies = [body(175_638_116, -246_822_830, 389)];
        let sight = Sight {
            robot,
            people: &people,
            bodies: &bodies,
        };
        let (_next, intent) = plan_follow(&Follower::follow(id(2)), &sight, Cue::None);
        match intent {
            Intent::Move(to) => {
                assert_capped(robot, to);
                assert!(clear(robot, to, &bodies), "{to:?}");
            }
            Intent::Blocked | Intent::Stuck | Intent::Stay => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn random_clear_steps_match_an_independent_short_step() {
        let mut state = 0x1234_5678_9abc_def0u64;
        for _ in 0..200 {
            state = state
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .wrapping_add(0x6a09_e667);
            let x0 = state as i32;
            state = state.wrapping_mul(0xbf58_476d_1ce4_e5b9);
            let y0 = state as i32;
            state = state.wrapping_mul(0x94d0_49bb_1331_11eb);
            let x1 = state as i32;
            state = state.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let y1 = state as i32;
            let from = at(x0, y0);
            let goal = at(x1, y1);
            let intent = plan_approach(from, goal, &[], 0);
            if let Intent::Move(to) = intent {
                assert_capped(from, to);
                let dx = i128::from(goal.x().get()) - i128::from(from.x().get());
                let dy = i128::from(goal.y().get()) - i128::from(from.y().get());
                let sx = i128::from(to.x().get()) - i128::from(from.x().get());
                let sy = i128::from(to.y().get()) - i128::from(from.y().get());
                let cross = dx * sy - dy * sx;
                let len2 =
                    dx.unsigned_abs() * dx.unsigned_abs() + dy.unsigned_abs() * dy.unsigned_abs();
                let len = isqrt(len2);
                assert!(
                    cross.unsigned_abs() <= len.max(1),
                    "{from:?} {goal:?} {to:?} cross {cross}"
                );
                assert!(dx * sx + dy * sy > 0, "{from:?} {goal:?} {to:?}");
            }
        }
    }

    #[test]
    fn the_sensor_keeps_a_far_hard_object_out_and_keeps_a_crowd_of_people() {
        let operator = Actor {
            id: id(2),
            pose: Pose::new(at(0, 1000)),
        };
        let hard = Obstacle {
            id: id(6),
            pose: Pose::new(at(2000, 0)),
            class: ObstacleClass::HardToSee,
            clearance: Mm::new(400),
            bodies: vec![Disk {
                id: id(6),
                center: at(2000, 0),
                radius: Mm::new(60),
            }],
        };
        let (people, bodies) = look(at(0, 0), operator, std::slice::from_ref(&hard));
        assert_eq!(people.len(), 1);
        assert!(bodies.is_empty());
        let (_people, near) = look(at(1400, 0), operator, &[hard]);
        assert_eq!(near.len(), 1);
        let crowd = Obstacle {
            id: id(7),
            pose: Pose::new(at(1000, 0)),
            class: ObstacleClass::Crowd,
            clearance: Mm::new(400),
            bodies: (0u32..3)
                .map(|index| Disk {
                    id: id(10 + index),
                    center: at(1000, i32::try_from(index).expect("index") * 600 - 600),
                    radius: Mm::new(200),
                })
                .collect(),
        };
        let (people, bodies) = look(at(0, 0), operator, &[crowd]);
        assert_eq!(bodies.len(), 3);
        assert_eq!(people.len(), 4);
        assert!(people.iter().any(|seen| seen.id == id(2)));
        assert!(people.iter().any(|seen| seen.id == id(10)));
        let small = Obstacle {
            id: id(5),
            pose: Pose::new(at(2000, 0)),
            class: ObstacleClass::SmallGround,
            clearance: Mm::new(400),
            bodies: vec![Disk {
                id: id(5),
                center: at(2000, 0),
                radius: Mm::new(80),
            }],
        };
        let (_people, seen) = look(at(0, 0), operator, &[small]);
        assert_eq!(seen.len(), 1);
    }
}
