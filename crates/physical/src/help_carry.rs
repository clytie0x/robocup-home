//! One deterministic Help Me Carry walk.
//!
//! The operator script knows the car. `follow_intent` does not. Motion uses
//! `base::integrate` with no obstacle boxes. The pick is a state flag, not a grasp.
//! The return walks the robot's own accepted poses backwards.

use crate::base::{BasePose, integrate};
use crate::profile::wrap_signed_deg;
use crate::still::{STILL_HEIGHT, STILL_WIDTH};
use domain::{Actor, Bag, BagKind, Custody, Id, Mm, Obstacle, Point, Pose};
use navigation::{
    Follower, Intent, Posted, SampleHistory, SensorView, Taken, accept, cue_from, plan_follow,
    sense,
};
use perception::{Candidate, select};
use task_5_1::{Facts, Sheet, score};

const ARENA_X_M: f64 = 0.0;
const START_X_M: f64 = -2.0;
const START_Y_M: f64 = 0.0;
const START_YAW_DEG: f64 = 180.0;
const OPERATOR_X_M: f64 = -2.8;
const STEP_M: f64 = 0.2;
const AT_CAR_M: f64 = 0.05;
const AT_CRUMB_M: f64 = 0.05;
const BAG_OFFSET_M: f64 = 0.8;
const MAX_TICKS: u32 = 4_000;

/// Script for one run. The car fields are the operator's goal, not a planner input.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    pub car_x_m: f64,
    pub car_y_m: f64,
    pub assigned: u32,
    pub point_at: u32,
    pub budget_ms: u64,
    pub dt_ms: u64,
    pub drop_after_pick: bool,
    pub stall_after_pick_ms: u64,
    pub skip_return: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub t_ms: u64,
    pub kind: &'static str,
    pub x_m: f64,
    pub y_m: f64,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub marks: Vec<Mark>,
    pub operator_path: Vec<(f64, f64)>,
    pub robot_path: Vec<(f64, f64)>,
    pub start: (f64, f64),
    pub car: (f64, f64),
    pub bags: [(u32, f64, f64); 2],
    pub chosen: Option<u32>,
    pub facts: Facts,
    pub sheet: Sheet,
    pub elapsed_ms: u64,
    pub ticks: u32,
}

/// The slice-1 success script. The start is outside the arena half-plane `x >= 0`.
pub fn success_setup() -> Setup {
    Setup {
        car_x_m: -6.4,
        car_y_m: 1.6,
        assigned: 2,
        point_at: 2,
        budget_ms: task_5_1::LIMIT_MS,
        dt_ms: 500,
        drop_after_pick: false,
        stall_after_pick_ms: 0,
        skip_return: false,
    }
}

pub fn run(setup: &Setup) -> Run {
    let start = BasePose {
        x_m: START_X_M,
        y_m: START_Y_M,
        yaw_deg: START_YAW_DEG,
    };
    let car = (setup.car_x_m, setup.car_y_m);
    let bags = [
        (2, car.0, car.1 + BAG_OFFSET_M),
        (3, car.0, car.1 - BAG_OFFSET_M),
    ];
    let pointed = id(setup.point_at);
    let mut pose = start;
    let mut crumbs = vec![(pose.x_m, pose.y_m)];
    let mut operator = (OPERATOR_X_M, START_Y_M);
    let op_path = operator_course(car);
    let mut op_index = 0usize;
    let mut follower = Follower::follow(id(1));
    let mut history = SampleHistory::new();
    let mut marks = Vec::new();
    let mut operator_path = Vec::new();
    let mut robot_path = vec![(pose.x_m, pose.y_m)];
    let mut now = 0u64;
    let mut ticks = 0u32;
    let mut followed = false;
    let mut perceived = None;
    let mut picked = None;
    let mut carried = None;
    let mut dropped = false;
    let mut reached = false;
    let mut timed_out = false;
    marks.push(mark(now, "ready-to-follow", pose, "ready-to-follow"));
    while ticks < MAX_TICKS {
        if !has_time(now, setup.dt_ms, setup.budget_ms) {
            timed_out = true;
            marks.push(mark(now, "time-up", pose, "budget"));
            break;
        }
        ticks += 1;
        if op_index < op_path.len() {
            let goal = op_path[op_index];
            let (next, arrived) = slide(operator, goal, STEP_M);
            operator = next;
            if arrived {
                op_index += 1;
            }
        }
        operator_path.push(operator);
        let at_car = dist(operator, car) <= AT_CAR_M;
        let view = observe(pose, operator, &bags, at_car, now);
        let (next_history, taken, _) = accept(history, &Posted::ready(view, now), now);
        let Some(view) = acted(taken) else {
            now += setup.dt_ms;
            continue;
        };
        history = next_history;
        let (next_follower, intent) = follow_intent(&follower, &view);
        follower = next_follower;
        if matches!(intent, Intent::Arrived) && at_car {
            followed = true;
            marks.push(mark(now, "at-car", pose, "operator script at car"));
            let ray = scripted_ray(operator, &bags, setup.point_at);
            let candidates = candidates_from(&view);
            if let Some((dx, dy)) = ray
                && let Ok(chosen) = select(Some(point_m(operator)), Some((dx, dy)), &candidates)
            {
                perceived = Some(chosen);
                marks.push(Mark {
                    t_ms: now,
                    kind: "perceive",
                    x_m: pose.x_m,
                    y_m: pose.y_m,
                    note: format!("perceived bag id {}", chosen.get()),
                });
                picked = Some(chosen);
                carried = Some(chosen);
                marks.push(Mark {
                    t_ms: now,
                    kind: "abstract-pick",
                    x_m: pose.x_m,
                    y_m: pose.y_m,
                    note: format!("abstract-pick bag id {}", chosen.get()),
                });
            }
            break;
        }
        if let Intent::Move(to) = intent {
            pose = take_step(pose, meters(to), &mut crumbs, &mut robot_path, true);
            marks.push(mark(now, "robot", pose, "step"));
        }
        now += setup.dt_ms;
    }
    if followed && setup.drop_after_pick {
        dropped = true;
        carried = None;
        marks.push(mark(now, "drop", pose, "bag dropped"));
    }
    if followed && setup.stall_after_pick_ms > 0 {
        if !has_time(now, setup.stall_after_pick_ms, setup.budget_ms) {
            timed_out = true;
            now = setup.budget_ms;
            marks.push(mark(now, "time-up", pose, "stall"));
        } else {
            now += setup.stall_after_pick_ms;
        }
    }
    if followed && !timed_out && !setup.skip_return {
        let mut goal_i = crumbs.len();
        while goal_i > 1 && ticks < MAX_TICKS {
            if !has_time(now, setup.dt_ms, setup.budget_ms) {
                timed_out = true;
                marks.push(mark(now, "time-up", pose, "return"));
                break;
            }
            let goal = crumbs[goal_i - 2];
            if dist((pose.x_m, pose.y_m), goal) <= AT_CRUMB_M {
                goal_i -= 1;
                continue;
            }
            ticks += 1;
            pose = take_step(pose, goal, &mut crumbs, &mut robot_path, false);
            marks.push(mark(now, "robot", pose, "return"));
            now += setup.dt_ms;
        }
        if dist((pose.x_m, pose.y_m), crumbs[0]) <= AT_CRUMB_M {
            reached = true;
            if carried.is_some() {
                marks.push(mark(now, "place-on-floor", pose, "bag on the floor"));
            }
        }
    }
    let facts = Facts {
        followed_to_car: followed,
        perceived,
        pointed,
        assigned: id(setup.assigned),
        picked,
        reached_start: reached,
        dropped,
        timed_out,
    };
    let sheet = score(&facts);
    Run {
        marks,
        operator_path,
        robot_path,
        start: (START_X_M, START_Y_M),
        car,
        bags,
        chosen: perceived.map(Id::get),
        facts,
        sheet,
        elapsed_ms: now,
        ticks,
    }
}

/// One follow decision. There is no car argument.
pub fn follow_intent(follower: &Follower, view: &SensorView) -> (Follower, Intent) {
    plan_follow(follower, &view.sight(), cue_from(view))
}

/// One kinematic step toward a recorded pose. The car is not an argument.
pub fn return_step(pose: BasePose, crumb: (f64, f64)) -> BasePose {
    match step_base(pose, crumb) {
        Some(next) => next,
        None => pose,
    }
}

pub fn timeline_csv(run: &Run) -> String {
    let mut out = String::from("# Help Me Carry 2025.1 slice 1.\n");
    out.push_str(
        "# Start is outside the arena half-plane x>=0. The car belongs to the operator script.\n",
    );
    out.push_str("# The follow call is not given that point.\n");
    out.push_str("t_ms,kind,x_m,y_m,note\n");
    for mark in &run.marks {
        out.push_str(&format!(
            "{},{},{:.4},{:.4},{}\n",
            mark.t_ms, mark.kind, mark.x_m, mark.y_m, mark.note
        ));
    }
    out
}

pub fn score_text(run: &Run) -> String {
    let mut out = String::from("Help Me Carry 2025.1 slice 1\n");
    out.push_str(&format!(
        "elapsed_ms {}\ntimed_out {}\nmeasured_subtotal {}\nprinted_total {}\n",
        run.elapsed_ms,
        if run.facts.timed_out { "yes" } else { "no" },
        run.sheet.measured_subtotal(),
        run.sheet.printed_total()
    ));
    out.push_str("id catalog status awarded action\n");
    for row in run.sheet.rows() {
        out.push_str(&format!(
            "{} {} {} {} {}\n",
            row.id,
            row.catalog,
            status_name(row.status),
            row.awarded,
            row.action
        ));
    }
    out
}

pub fn trajectory_png(run: &Run) -> Vec<u8> {
    let mut points = run.robot_path.clone();
    points.extend(run.operator_path.iter().copied());
    points.push(run.start);
    points.push(run.car);
    for bag in run.bags {
        points.push((bag.1, bag.2));
    }
    let (min_x, max_x, min_y, max_y) = bounds(&points);
    let mut rgb = vec![255u8; STILL_WIDTH * STILL_HEIGHT * 3];
    let paint = |rgb: &mut [u8], points: &[(f64, f64)], color: [u8; 3]| {
        for point in points {
            dot(rgb, project(*point, min_x, max_x, min_y, max_y), color);
        }
    };
    paint(&mut rgb, &run.operator_path, [30, 90, 180]);
    paint(&mut rgb, &run.robot_path, [20, 140, 60]);
    dot(&mut rgb, project(run.start, min_x, max_x, min_y, max_y), [
        0, 0, 0,
    ]);
    dot(&mut rgb, project(run.car, min_x, max_x, min_y, max_y), [
        90, 90, 90,
    ]);
    for bag in run.bags {
        let color = if run.chosen == Some(bag.0) {
            [200, 30, 30]
        } else {
            [230, 160, 40]
        };
        dot(
            &mut rgb,
            project((bag.1, bag.2), min_x, max_x, min_y, max_y),
            color,
        );
    }
    encode_png(&rgb)
}

fn operator_course(car: (f64, f64)) -> Vec<(f64, f64)> {
    let start = (OPERATOR_X_M, START_Y_M);
    let mid = ((start.0 + car.0) * 0.5, start.1);
    vec![mid, car]
}

fn observe(
    pose: BasePose,
    operator: (f64, f64),
    bags: &[(u32, f64, f64); 2],
    gesture: bool,
    now: u64,
) -> SensorView {
    let robot = point_m((pose.x_m, pose.y_m));
    let actor = Actor {
        id: id(1),
        pose: Pose::new(point_m(operator)),
    };
    let bags: Vec<Bag> = bags
        .iter()
        .map(|bag| Bag {
            id: id(bag.0),
            pose: Pose::new(point_m((bag.1, bag.2))),
            kind: BagKind::LightweightStiffVerticalHandles,
            visible: true,
            custody: Custody::Free,
        })
        .collect();
    let obstacles: &[Obstacle] = &[];
    sense(robot, actor, &bags, obstacles, gesture, now)
}

fn acted(taken: Taken) -> Option<SensorView> {
    match taken {
        Taken::Act(view) => Some(view),
        Taken::Rejected | Taken::NeedFresh => None,
    }
}

fn candidates_from(view: &SensorView) -> Vec<Candidate> {
    view.bags
        .iter()
        .map(|bag| Candidate {
            id: bag.id,
            at: bag.pose.point(),
            visible: bag.visible,
        })
        .collect()
}

fn scripted_ray(
    operator: (f64, f64),
    bags: &[(u32, f64, f64); 2],
    point_at: u32,
) -> Option<(i32, i32)> {
    let bag = bags.iter().find(|bag| bag.0 == point_at)?;
    let dx = Mm::try_from_meters(bag.1 - operator.0).ok()?.get();
    let dy = Mm::try_from_meters(bag.2 - operator.1).ok()?.get();
    if dx == 0 && dy == 0 {
        None
    } else {
        Some((dx, dy))
    }
}

fn take_step(
    pose: BasePose,
    goal: (f64, f64),
    crumbs: &mut Vec<(f64, f64)>,
    robot_path: &mut Vec<(f64, f64)>,
    record_crumb: bool,
) -> BasePose {
    let Some(next) = step_base(pose, goal) else {
        return pose;
    };
    if record_crumb && ((next.x_m - pose.x_m).abs() > 1e-9 || (next.y_m - pose.y_m).abs() > 1e-9) {
        crumbs.push((next.x_m, next.y_m));
    }
    robot_path.push((next.x_m, next.y_m));
    next
}

fn step_base(pose: BasePose, goal: (f64, f64)) -> Option<BasePose> {
    let dx = goal.0 - pose.x_m;
    let dy = goal.1 - pose.y_m;
    let distance = dx.hypot(dy);
    let step = distance.min(STEP_M);
    let (sx, sy) = if distance < 1e-9 {
        (0.0, 0.0)
    } else {
        (dx / distance * step, dy / distance * step)
    };
    let yaw = pose.yaw_deg.to_radians();
    let dx_base = yaw.cos() * sx + yaw.sin() * sy;
    let dy_base = -yaw.sin() * sx + yaw.cos() * sy;
    let face = if distance < 1e-9 {
        0.0
    } else {
        wrap_signed_deg(dy.atan2(dx).to_degrees() - pose.yaw_deg).clamp(-90.0, 90.0)
    };
    let attempt = integrate(pose, dx_base, dy_base, face, &[]);
    match attempt.hit {
        crate::base::BodyHit::Clear => Some(attempt.pose),
        _ => None,
    }
}

fn slide(from: (f64, f64), to: (f64, f64), step: f64) -> ((f64, f64), bool) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let distance = dx.hypot(dy);
    if distance <= step {
        (to, true)
    } else {
        (
            (from.0 + dx / distance * step, from.1 + dy / distance * step),
            false,
        )
    }
}

fn has_time(now: u64, dt: u64, budget: u64) -> bool {
    now < budget && budget - now >= dt
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn id(raw: u32) -> Id {
    Id::new(raw).expect("id")
}

fn point_m(point: (f64, f64)) -> Point {
    Point::new(
        Mm::try_from_meters(point.0).expect("x"),
        Mm::try_from_meters(point.1).expect("y"),
    )
}

fn meters(point: Point) -> (f64, f64) {
    (
        f64::from(point.x().get()) / 1000.0,
        f64::from(point.y().get()) / 1000.0,
    )
}

fn mark(t_ms: u64, kind: &'static str, pose: BasePose, note: &str) -> Mark {
    Mark {
        t_ms,
        kind,
        x_m: pose.x_m,
        y_m: pose.y_m,
        note: note.to_string(),
    }
}

fn status_name(status: task_5_1::Status) -> &'static str {
    match status {
        task_5_1::Status::Measured => "implemented-in-slice1",
        task_5_1::Status::NotRun => "not-run",
        task_5_1::Status::OutOfSimScope => "out-of-sim-scope",
    }
}

fn bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for point in points {
        min_x = min_x.min(point.0);
        max_x = max_x.max(point.0);
        min_y = min_y.min(point.1);
        max_y = max_y.max(point.1);
    }
    if min_x >= max_x {
        max_x = min_x + 1.0;
    }
    if min_y >= max_y {
        max_y = min_y + 1.0;
    }
    (min_x, max_x, min_y, max_y)
}

fn project(point: (f64, f64), min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> (i32, i32) {
    let span_x = (max_x - min_x).max(0.001);
    let span_y = (max_y - min_y).max(0.001);
    let scale = ((STILL_WIDTH as f64) - 32.0) / span_x;
    let scale = scale.min(((STILL_HEIGHT as f64) - 32.0) / span_y);
    let x = 16.0 + (point.0 - min_x) * scale;
    let y = 16.0 + (max_y - point.1) * scale;
    (x.round() as i32, y.round() as i32)
}

fn dot(rgb: &mut [u8], at: (i32, i32), color: [u8; 3]) {
    for dy in -2..=2 {
        for dx in -2..=2 {
            let x = at.0 + dx;
            let y = at.1 + dy;
            if x < 0 || y < 0 || x >= STILL_WIDTH as i32 || y >= STILL_HEIGHT as i32 {
                continue;
            }
            let index = (y as usize * STILL_WIDTH + x as usize) * 3;
            rgb[index] = color[0];
            rgb[index + 1] = color[1];
            rgb[index + 2] = color[2];
        }
    }
}

fn encode_png(rgb: &[u8]) -> Vec<u8> {
    let mut scan = Vec::with_capacity(STILL_HEIGHT * (1 + STILL_WIDTH * 3));
    for row in 0..STILL_HEIGHT {
        scan.push(0);
        let start = row * STILL_WIDTH * 3;
        scan.extend_from_slice(&rgb[start..start + STILL_WIDTH * 3]);
    }
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = [0u8; 13];
    ihdr[0..4].copy_from_slice(&(STILL_WIDTH as u32).to_be_bytes());
    ihdr[4..8].copy_from_slice(&(STILL_HEIGHT as u32).to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = 2;
    png.extend(chunk(b"IHDR", &ihdr));
    png.extend(chunk(b"IDAT", &stored_zlib(&scan)));
    png.extend(chunk(b"IEND", &[]));
    png
}

fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(tag);
    body.extend_from_slice(data);
    let crc = crc32(&body);
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(data.len()).expect("len").to_be_bytes());
    out.extend(body);
    out.extend_from_slice(&crc.to_be_bytes());
    out
}

fn stored_zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut index = 0;
    loop {
        let n = (data.len() - index).min(65_535);
        let last = index + n == data.len();
        out.push(if last { 1 } else { 0 });
        let len = u16::try_from(n).expect("block");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(&data[index..index + n]);
        index += n;
        if last {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a += u32::from(byte);
        if a >= 65_521 {
            a -= 65_521;
        }
        b += a;
        if b >= 65_521 {
            b -= 65_521;
        }
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::still::read_still_png;
    use task_5_1::Status;

    fn note<'a>(run: &'a Run, kind: &str) -> Option<&'a str> {
        run.marks
            .iter()
            .find(|mark| mark.kind == kind)
            .map(|mark| mark.note.as_str())
    }

    #[test]
    fn the_success_run_follows_picks_and_returns() {
        let run = run(&success_setup());
        assert!(run.start.0 < ARENA_X_M);
        assert!(run.facts.followed_to_car);
        assert_eq!(run.chosen, Some(2));
        assert_eq!(note(&run, "ready-to-follow"), Some("ready-to-follow"));
        assert_eq!(note(&run, "perceive"), Some("perceived bag id 2"));
        assert_eq!(note(&run, "abstract-pick"), Some("abstract-pick bag id 2"));
        assert_eq!(note(&run, "place-on-floor"), Some("bag on the floor"));
        assert!(!run.facts.timed_out);
        assert!(run.facts.reached_start);
        assert!(run.elapsed_ms <= task_5_1::LIMIT_MS);
        assert!(run.ticks < MAX_TICKS);
        let sheet = &run.sheet;
        assert_eq!(sheet.row("HMC-FOLLOW").expect("row").awarded, 300);
        assert_eq!(sheet.row("HMC-PERCEIVE").expect("row").awarded, 15);
        assert_eq!(sheet.row("HMC-PICK").expect("row").awarded, 100);
        assert_eq!(sheet.row("HMC-RETURN").expect("row").awarded, 200);
        assert_eq!(sheet.row("HMC-DROP").expect("row").awarded, 0);
        assert_eq!(sheet.measured_subtotal(), 615);
        assert_eq!(sheet.row("HMC-CROWD").expect("row").status, Status::NotRun);
        assert_eq!(sheet.row("HMC-QUEUE").expect("row").status, Status::NotRun);
        let png = trajectory_png(&run);
        let rgb = read_still_png(&png).expect("png");
        assert!(rgb.chunks(3).any(|pixel| pixel == [200, 30, 30]));
        assert!(rgb.chunks(3).any(|pixel| pixel == [20, 140, 60]));
        assert!(timeline_csv(&run).contains("ready-to-follow"));
        assert!(!score_text(&run).contains("measured_subtotal 1115"));
    }

    #[test]
    fn a_second_car_is_not_a_hardcoded_robot_goal() {
        let mut setup = success_setup();
        setup.car_x_m = -8.0;
        setup.car_y_m = -1.2;
        let run = run(&setup);
        assert!(run.facts.followed_to_car);
        assert_eq!(run.car, (-8.0, -1.2));
        assert_eq!(run.sheet.row("HMC-FOLLOW").expect("row").awarded, 300);
        assert_ne!(run.car, (success_setup().car_x_m, success_setup().car_y_m));
    }

    #[test]
    fn the_follow_call_has_no_car_argument() {
        let run = run(&success_setup());
        let pose = BasePose {
            x_m: run.start.0,
            y_m: run.start.1,
            yaw_deg: START_YAW_DEG,
        };
        let view = observe(pose, (OPERATOR_X_M, START_Y_M), &run.bags, false, 0);
        let (_follower, intent) = follow_intent(&Follower::follow(id(1)), &view);
        assert_ne!(intent, Intent::Arrived);
        let closer = return_step(pose, (pose.x_m - STEP_M, pose.y_m));
        assert!(closer.x_m < pose.x_m);
    }

    #[test]
    fn the_wrong_pointed_bag_scores_zero_for_perceive_and_pick() {
        let mut setup = success_setup();
        setup.point_at = 3;
        let run = run(&setup);
        assert!(run.facts.followed_to_car);
        assert_eq!(note(&run, "perceive"), Some("perceived bag id 3"));
        assert_eq!(run.sheet.row("HMC-PERCEIVE").expect("row").awarded, 0);
        assert_eq!(run.sheet.row("HMC-PICK").expect("row").awarded, 0);
    }

    #[test]
    fn a_timeout_stops_and_keeps_only_earned_rows() {
        let mut setup = success_setup();
        setup.stall_after_pick_ms = task_5_1::LIMIT_MS;
        let run = run(&setup);
        assert!(run.facts.timed_out);
        assert!(run.facts.followed_to_car);
        assert!(!run.facts.reached_start);
        assert_eq!(run.sheet.row("HMC-FOLLOW").expect("row").awarded, 300);
        assert_eq!(run.sheet.row("HMC-PERCEIVE").expect("row").awarded, 15);
        assert_eq!(run.sheet.row("HMC-PICK").expect("row").awarded, 100);
        assert_eq!(run.sheet.row("HMC-RETURN").expect("row").awarded, 0);
        assert!(run.elapsed_ms <= task_5_1::LIMIT_MS);
    }

    #[test]
    fn a_dropped_bag_scores_minus_50() {
        let mut setup = success_setup();
        setup.drop_after_pick = true;
        let run = run(&setup);
        assert!(run.facts.dropped);
        assert_eq!(run.sheet.row("HMC-DROP").expect("row").awarded, -50);
        assert_eq!(run.sheet.row("HMC-PICK").expect("row").awarded, 100);
        assert!(note(&run, "place-on-floor").is_none());
    }

    #[test]
    fn a_return_that_does_not_finish_scores_no_200() {
        let mut setup = success_setup();
        setup.skip_return = true;
        let run = run(&setup);
        assert!(run.facts.followed_to_car);
        assert!(!run.facts.reached_start);
        assert!(!run.facts.timed_out);
        assert_eq!(run.sheet.row("HMC-RETURN").expect("row").awarded, 0);
        assert_eq!(run.sheet.row("HMC-FOLLOW").expect("row").awarded, 300);
    }
}
