//! Three sourced base plates, a kinematic step, and a lidar mounted on that base.
//!
//! Plate plan views come from the configured footprint. Plate heights come from
//! the taped gaps plus the assumed thicknesses. The lidar xyz is the configured
//! pose, not an installed lever arm. A step is a rigid translation and yaw. It
//! is not a motor model. `sent` stays false.

use domain::{Mm, Point};
use navigation::{Intent, SampleHistory};

use crate::kind::Kind;
use crate::lidar::{encode_packet, parse_lidar};
use crate::navpost::{ClockLabel, PostClock, PostOutcome, post_samples};
use crate::profile::{
    FOOTPRINT_XY_M, LIDAR_X_M, LIDAR_Z_M, lidar_profiles, wrap_full_deg, yaw_disagreement_deg,
};
use crate::scene::{
    Aabb, Mount, Plane, SceneError, ScreenSeg, Vec3, enter_box, fit_plane, place_ticks, plane_axes,
    project_oblique, range_word, scan_direction, svg_header, swatches, tick_values, world_axis,
};

pub const MAX_STEP_M: f64 = 1.0;
pub const MAX_YAW_DEG: f64 = 90.0;
const YAW_SAMPLE_DEG: f64 = 5.0;
const ROTATION_PAD_M: f64 = 0.0005;
const CONTACT_EPS_M: f64 = 1e-9;

const TOP_XY: [(f64, f64); 8] = [
    (0.20, -0.15),
    (0.20, 0.15),
    (0.15, 0.20),
    (-0.15, 0.20),
    (-0.20, 0.15),
    (-0.20, -0.15),
    (-0.15, -0.20),
    (0.15, -0.20),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plate {
    pub name: &'static str,
    pub polygon_xy_m: &'static [(f64, f64)],
    pub z_min_m: f64,
    pub z_max_m: f64,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placeholder {
    pub name: &'static str,
    pub kind: Kind,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasePose {
    pub x_m: f64,
    pub y_m: f64,
    pub yaw_deg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    Overlap,
    Contact,
    Sweep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepFault {
    NonFinite,
    Oversize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyHit {
    Clear,
    Blocked {
        reason: BlockReason,
        plate: &'static str,
        obstacle: &'static str,
    },
    Rejected {
        fault: StepFault,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attempt {
    pub pose: BasePose,
    pub hit: BodyHit,
    pub sent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub range_m: f64,
    pub point: Vec3,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyCycle {
    pub hits: usize,
    pub posted_points: usize,
    pub intent_name: &'static str,
    pub sent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceRow {
    pub index: u32,
    pub sensor_hits: usize,
    pub policy: &'static str,
    pub attempted_x_m: f64,
    pub attempted_y_m: f64,
    pub attempted_yaw_deg: f64,
    pub accepted_x_m: f64,
    pub accepted_y_m: f64,
    pub accepted_yaw_deg: f64,
    pub outcome: &'static str,
    pub sent: bool,
}

pub struct BaseViews {
    pub model_csv: String,
    pub trace_csv: String,
    pub top_svg: String,
    pub side_svg: String,
    pub oblique_svg: String,
    pub pose_svg: String,
}

struct MountSpec {
    id: &'static str,
    ray_yaw_deg: f64,
    provenance: &'static str,
}

pub fn collision_plates() -> [Plate; 3] {
    let underside = 0.030;
    let aluminium = 0.002;
    let taped_below_top = 0.120;
    let taped_above_layer1 = 0.210;
    let acrylic = 0.003;
    let layer3_under = underside + aluminium + taped_above_layer1;
    let layer2_top = layer3_under - taped_below_top;
    [
        Plate {
            name: "chassis_bottom",
            polygon_xy_m: &FOOTPRINT_XY_M,
            z_min_m: underside,
            z_max_m: underside + aluminium,
            source: "plan from pi robot.json footprint; underside 0.030 m taped; thickness 0.002 m assumed",
        },
        Plate {
            name: "chassis_middle",
            polygon_xy_m: &FOOTPRINT_XY_M,
            z_min_m: layer2_top - aluminium,
            z_max_m: layer2_top,
            source: "same footprint; top is the layer-3 underside minus the 0.12 m taped gap; thickness assumed",
        },
        Plate {
            name: "top_plate",
            polygon_xy_m: &TOP_XY,
            z_min_m: layer3_under,
            z_max_m: layer3_under + acrylic,
            source: "underside is layer-1 top plus the 0.21 m taped gap; 0.05 m corner cut and 0.003 m thickness are from photos",
        },
    ]
}

pub fn visual_placeholders() -> [Placeholder; 1] {
    [Placeholder {
        name: "lidar_head",
        kind: Kind::Assumed,
        source: "robot3d unknown list: cylinder radius 0.028 m and height 0.04 m from photos. Not a collision solid.",
    }]
}

pub fn resting_contact(pose: BasePose, obstacles: &[Aabb]) -> BodyHit {
    if !pose_finite(pose)
        || obstacles
            .iter()
            .any(|body| !body.min.finite() || !body.max.finite())
    {
        return BodyHit::Rejected {
            fault: StepFault::NonFinite,
        };
    }
    let mut graze: Option<(&'static str, &'static str)> = None;
    for plate in &collision_plates() {
        let polygon = placed(pose, plate.polygon_xy_m);
        for obstacle in obstacles {
            match solid_contact(plate.z_min_m, plate.z_max_m, &polygon, obstacle, 0.0) {
                Touch::Overlap => {
                    return BodyHit::Blocked {
                        reason: BlockReason::Overlap,
                        plate: plate.name,
                        obstacle: obstacle.name,
                    };
                }
                Touch::Graze => graze = Some((plate.name, obstacle.name)),
                Touch::Clear => {}
            }
        }
    }
    if let Some((plate, obstacle)) = graze {
        BodyHit::Blocked {
            reason: BlockReason::Contact,
            plate,
            obstacle,
        }
    } else {
        BodyHit::Clear
    }
}

pub fn integrate(
    pose: BasePose,
    dx_base_m: f64,
    dy_base_m: f64,
    dyaw_deg: f64,
    obstacles: &[Aabb],
) -> Attempt {
    if !pose_finite(pose)
        || !dx_base_m.is_finite()
        || !dy_base_m.is_finite()
        || !dyaw_deg.is_finite()
        || obstacles
            .iter()
            .any(|body| !body.min.finite() || !body.max.finite())
    {
        return stay(
            pose,
            BodyHit::Rejected {
                fault: StepFault::NonFinite,
            },
        );
    }
    let chord = dx_base_m.hypot(dy_base_m);
    if chord > MAX_STEP_M || dyaw_deg.abs() > MAX_YAW_DEG {
        return stay(
            pose,
            BodyHit::Rejected {
                fault: StepFault::Oversize,
            },
        );
    }
    let start = resting_contact(pose, obstacles);
    if !matches!(start, BodyHit::Clear) {
        return stay(pose, start);
    }
    let end = shifted(pose, dx_base_m, dy_base_m, dyaw_deg);
    if dyaw_deg.abs() <= CONTACT_EPS_M {
        if chord <= CONTACT_EPS_M {
            return Attempt {
                pose: end,
                hit: BodyHit::Clear,
                sent: false,
            };
        }
        for plate in &collision_plates() {
            let mut points = placed(pose, plate.polygon_xy_m);
            points.extend(placed(end, plate.polygon_xy_m));
            let swept = hull(&points);
            if let Some(obstacle) = first_touch(plate, &swept, obstacles, 0.0) {
                return stay(
                    pose,
                    BodyHit::Blocked {
                        reason: BlockReason::Sweep,
                        plate: plate.name,
                        obstacle,
                    },
                );
            }
        }
        return Attempt {
            pose: end,
            hit: BodyHit::Clear,
            sent: false,
        };
    }
    let steps = (dyaw_deg.abs() / YAW_SAMPLE_DEG).ceil().max(1.0) as i32;
    for step in 0..=steps {
        let fraction = f64::from(step) / f64::from(steps);
        let sample = shifted(
            pose,
            dx_base_m * fraction,
            dy_base_m * fraction,
            dyaw_deg * fraction,
        );
        for plate in &collision_plates() {
            let polygon = placed(sample, plate.polygon_xy_m);
            if let Some(obstacle) = first_touch(plate, &polygon, obstacles, ROTATION_PAD_M) {
                return stay(
                    pose,
                    BodyHit::Blocked {
                        reason: BlockReason::Sweep,
                        plate: plate.name,
                        obstacle,
                    },
                );
            }
        }
    }
    Attempt {
        pose: end,
        hit: BodyHit::Clear,
        sent: false,
    }
}

pub fn cast_sensor(
    pose: &BasePose,
    profile_id: &str,
    pitch_down_deg: f64,
    bearing_deg: f64,
    obstacles: &[Aabb],
) -> Result<Option<RayHit>, SceneError> {
    let spec = mount_spec(profile_id)?;
    if !pose_finite(*pose) || !pitch_down_deg.is_finite() {
        return Err(SceneError::NonFinite);
    }
    let origin = base_to_world(
        pose,
        Vec3 {
            x: LIDAR_X_M,
            y: 0.0,
            z: LIDAR_Z_M,
        },
    );
    let mount = Mount {
        name: spec.id,
        translation_m: origin,
        yaw_deg: pose.yaw_deg + spec.ray_yaw_deg,
        pitch_down_deg,
        provenance: spec.provenance,
    };
    let direction = scan_direction(&mount, bearing_deg)?;
    let mut best: Option<(f64, Vec3, &'static str)> = None;
    for obstacle in obstacles {
        if !obstacle.min.finite() || !obstacle.max.finite() {
            return Err(SceneError::NonFinite);
        }
        if let Some(range) = enter_box(origin, direction, obstacle) {
            keep(
                &mut best,
                range,
                origin.add(direction.scale(range)),
                obstacle.name,
            );
        }
    }
    let origin_base = world_to_base(pose, origin);
    let direction_base = rotate_to_base(pose, direction);
    for plate in &collision_plates() {
        if let Some(range) = enter_prism(origin_base, direction_base, plate) {
            keep(
                &mut best,
                range,
                origin.add(direction.scale(range)),
                plate.name,
            );
        }
    }
    Ok(best.map(|(range_m, point, name)| RayHit {
        range_m,
        point,
        name,
    }))
}

pub fn scan_step_m(from: Point, to: Point, profile_id: &str) -> Result<(f64, f64), SceneError> {
    let spec = mount_spec(profile_id)?;
    let dx = (i64::from(to.x().get()) - i64::from(from.x().get())) as f64 / 1000.0;
    let dy = (i64::from(to.y().get()) - i64::from(from.y().get())) as f64 / 1000.0;
    let rad = spec.ray_yaw_deg.to_radians();
    Ok((
        dx * rad.cos() - dy * rad.sin(),
        dx * rad.sin() + dy * rad.cos(),
    ))
}

pub fn sense_policy(
    pose: &BasePose,
    profile_id: &str,
    obstacles: &[Aabb],
    bearings_deg: &[f64],
) -> Result<PolicyCycle, SceneError> {
    mount_spec(profile_id)?;
    let profile = lidar_profiles()
        .into_iter()
        .find(|item| item.id == profile_id)
        .ok_or(SceneError::Uncodable)?;
    if bearings_deg.is_empty() {
        return Err(SceneError::Uncodable);
    }
    let mut triples = Vec::new();
    let mut hits = 0;
    for bearing in bearings_deg {
        let range = match cast_sensor(pose, profile_id, 0.0, *bearing, obstacles)? {
            Some(hit) => {
                hits += 1;
                hit.range_m
            }
            None => 0.0,
        };
        triples.push((
            wrap_full_deg(profile.front_raw_deg - bearing),
            range_word(range)?,
            0_u8,
        ));
    }
    let mut bytes = Vec::new();
    for group in triples.chunks(40) {
        bytes.extend(encode_packet(0, group).map_err(|_| SceneError::Uncodable)?);
    }
    let parsed = parse_lidar(&bytes);
    if !parsed.rejects.is_empty() {
        return Err(SceneError::Uncodable);
    }
    let clock = PostClock {
        captured_ms: Some(2_500),
        delivered_ms: 2_500,
        decision_ms: 2_500,
        label: ClockLabel::InjectedReplay,
    };
    let (_history, posted) = post_samples(&profile, &parsed.samples, SampleHistory::new(), clock);
    let intent = match posted.outcome {
        PostOutcome::Acted { intent } => intent,
        PostOutcome::Unusable { .. } => return Err(SceneError::Uncodable),
    };
    Ok(PolicyCycle {
        hits,
        posted_points: posted.view_points.len(),
        intent_name: intent_name(intent),
        sent: posted.sent,
    })
}

pub fn demo_low_box() -> Aabb {
    Aabb {
        name: "low-box",
        min: Vec3 {
            x: 0.30,
            y: -0.15,
            z: 0.0,
        },
        max: Vec3 {
            x: 0.50,
            y: 0.15,
            z: 0.10,
        },
    }
}

pub fn demo_views() -> Result<BaseViews, SceneError> {
    let start = BasePose {
        x_m: 0.0,
        y_m: 0.0,
        yaw_deg: 0.0,
    };
    let low = demo_low_box();
    let bearings = fan();
    let sensed = sense_policy(&start, "measured-front", &[low], &bearings)?;
    let from = Point::new(Mm::new(0), Mm::new(0));
    let to = Point::new(Mm::new(200), Mm::new(0));
    let (dx, dy) = scan_step_m(from, to, "measured-front")?;
    let blocked = integrate(start, dx, dy, 0.0, &[low]);
    let opened = integrate(start, dx, dy, 0.0, &[]);
    let rows = [
        row(0, sensed.hits, sensed.intent_name, start, start, "lost"),
        row(
            1,
            sensed.hits,
            "Move",
            shifted(start, dx, dy, 0.0),
            blocked.pose,
            outcome_name(blocked.hit),
        ),
        row(
            2,
            0,
            "Move",
            shifted(start, dx, dy, 0.0),
            opened.pose,
            outcome_name(opened.hit),
        ),
    ];
    Ok(BaseViews {
        model_csv: model_csv(),
        trace_csv: trace_csv(&rows),
        top_svg: draw_top(&low, dx),
        side_svg: draw_side(&low),
        oblique_svg: draw_oblique(&low),
        pose_svg: draw_pose(&rows),
    })
}

fn row(
    index: u32,
    hits: usize,
    policy: &'static str,
    attempted: BasePose,
    accepted: BasePose,
    outcome: &'static str,
) -> TraceRow {
    TraceRow {
        index,
        sensor_hits: hits,
        policy,
        attempted_x_m: attempted.x_m,
        attempted_y_m: attempted.y_m,
        attempted_yaw_deg: attempted.yaw_deg,
        accepted_x_m: accepted.x_m,
        accepted_y_m: accepted.y_m,
        accepted_yaw_deg: accepted.yaw_deg,
        outcome,
        sent: false,
    }
}

fn outcome_name(hit: BodyHit) -> &'static str {
    match hit {
        BodyHit::Clear => "accepted",
        BodyHit::Blocked {
            reason: BlockReason::Overlap,
            ..
        } => "overlap",
        BodyHit::Blocked {
            reason: BlockReason::Contact,
            ..
        } => "contact",
        BodyHit::Blocked {
            reason: BlockReason::Sweep,
            ..
        } => "sweep",
        BodyHit::Rejected {
            fault: StepFault::NonFinite,
        } => "non-finite",
        BodyHit::Rejected {
            fault: StepFault::Oversize,
        } => "oversize",
    }
}

fn fan() -> Vec<f64> {
    let mut bearings = Vec::new();
    let mut bearing = -20.0;
    while bearing <= 20.0 + 1e-9 {
        bearings.push(bearing);
        bearing += 2.0;
    }
    bearings
}

fn mount_spec(profile_id: &str) -> Result<MountSpec, SceneError> {
    let measured = lidar_profiles()
        .into_iter()
        .find(|item| item.id == profile_id)
        .ok_or(SceneError::Uncodable)?;
    let ray_yaw_deg = if profile_id == "measured-front" {
        0.0
    } else if profile_id == "configured-front" {
        yaw_disagreement_deg()
    } else {
        return Err(SceneError::Uncodable);
    };
    Ok(MountSpec {
        id: measured.id,
        ray_yaw_deg,
        provenance: "xyz is pi robot.json lidar x_m y_m z_m, configured, not an installed lever. Bearing 0 for the measured profile is base +x. The configured profile adds the 4.8 deg disagreement. The URDF yaw is the raw-angle zero and is not applied again.",
    })
}

fn pose_finite(pose: BasePose) -> bool {
    pose.x_m.is_finite() && pose.y_m.is_finite() && pose.yaw_deg.is_finite()
}

fn stay(pose: BasePose, hit: BodyHit) -> Attempt {
    Attempt {
        pose,
        hit,
        sent: false,
    }
}

fn shifted(pose: BasePose, dx: f64, dy: f64, dyaw: f64) -> BasePose {
    let yaw = pose.yaw_deg.to_radians();
    BasePose {
        x_m: pose.x_m + dx * yaw.cos() - dy * yaw.sin(),
        y_m: pose.y_m + dx * yaw.sin() + dy * yaw.cos(),
        yaw_deg: pose.yaw_deg + dyaw,
    }
}

fn base_to_world(pose: &BasePose, point: Vec3) -> Vec3 {
    let yaw = pose.yaw_deg.to_radians();
    Vec3 {
        x: pose.x_m + point.x * yaw.cos() - point.y * yaw.sin(),
        y: pose.y_m + point.x * yaw.sin() + point.y * yaw.cos(),
        z: point.z,
    }
}

fn world_to_base(pose: &BasePose, point: Vec3) -> Vec3 {
    let yaw = pose.yaw_deg.to_radians();
    let dx = point.x - pose.x_m;
    let dy = point.y - pose.y_m;
    Vec3 {
        x: yaw.cos() * dx + yaw.sin() * dy,
        y: -yaw.sin() * dx + yaw.cos() * dy,
        z: point.z,
    }
}

fn rotate_to_base(pose: &BasePose, direction: Vec3) -> Vec3 {
    let yaw = pose.yaw_deg.to_radians();
    Vec3 {
        x: yaw.cos() * direction.x + yaw.sin() * direction.y,
        y: -yaw.sin() * direction.x + yaw.cos() * direction.y,
        z: direction.z,
    }
}

fn placed(pose: BasePose, polygon: &[(f64, f64)]) -> Vec<(f64, f64)> {
    polygon
        .iter()
        .map(|(x, y)| {
            let point = base_to_world(
                &pose,
                Vec3 {
                    x: *x,
                    y: *y,
                    z: 0.0,
                },
            );
            (point.x, point.y)
        })
        .collect()
}

enum Touch {
    Clear,
    Graze,
    Overlap,
}

fn solid_contact(z0: f64, z1: f64, polygon: &[(f64, f64)], obstacle: &Aabb, pad_m: f64) -> Touch {
    let z_pen = z1.min(obstacle.max.z) - z0.max(obstacle.min.z);
    let rect = (
        obstacle.min.x - pad_m,
        obstacle.min.y - pad_m,
        obstacle.max.x + pad_m,
        obstacle.max.y + pad_m,
    );
    let xy_pen = xy_penetration(polygon, rect);
    match (class_pen(z_pen), class_pen(xy_pen)) {
        (Touch::Clear, _) | (_, Touch::Clear) => Touch::Clear,
        (Touch::Graze, _) | (_, Touch::Graze) => Touch::Graze,
        _ => Touch::Overlap,
    }
}

fn class_pen(penetration: f64) -> Touch {
    if penetration < -CONTACT_EPS_M {
        Touch::Clear
    } else if penetration <= CONTACT_EPS_M {
        Touch::Graze
    } else {
        Touch::Overlap
    }
}

fn first_touch(
    plate: &Plate,
    polygon: &[(f64, f64)],
    obstacles: &[Aabb],
    pad_m: f64,
) -> Option<&'static str> {
    for obstacle in obstacles {
        if !matches!(
            solid_contact(plate.z_min_m, plate.z_max_m, polygon, obstacle, pad_m),
            Touch::Clear
        ) {
            return Some(obstacle.name);
        }
    }
    None
}

fn xy_penetration(polygon: &[(f64, f64)], rect: (f64, f64, f64, f64)) -> f64 {
    let mut best = f64::INFINITY;
    let mut axes = vec![(1.0, 0.0), (0.0, 1.0)];
    let count = polygon.len();
    for index in 0..count {
        let a = polygon[index];
        let b = polygon[(index + 1) % count];
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let length = dx.hypot(dy);
        if length > 1e-12 {
            axes.push((-dy / length, dx / length));
        }
    }
    for (nx, ny) in axes {
        let (p0, p1) = project_poly(polygon, nx, ny);
        let (r0, r1) = project_rect(rect, nx, ny);
        best = best.min(p1.min(r1) - p0.max(r0));
    }
    best
}

fn project_poly(polygon: &[(f64, f64)], nx: f64, ny: f64) -> (f64, f64) {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for (x, y) in polygon {
        let value = x * nx + y * ny;
        low = low.min(value);
        high = high.max(value);
    }
    (low, high)
}

fn project_rect(rect: (f64, f64, f64, f64), nx: f64, ny: f64) -> (f64, f64) {
    let corners = [
        (rect.0, rect.1),
        (rect.2, rect.1),
        (rect.0, rect.3),
        (rect.2, rect.3),
    ];
    project_poly(&corners, nx, ny)
}

fn hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    sorted.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12);
    if sorted.len() <= 2 {
        return sorted;
    }
    let mut lower = Vec::new();
    for point in &sorted {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::new();
    for point in sorted.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross(origin: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - origin.0) * (b.1 - origin.1) - (a.1 - origin.1) * (b.0 - origin.0)
}

fn keep(best: &mut Option<(f64, Vec3, &'static str)>, range: f64, point: Vec3, name: &'static str) {
    if best.map(|(kept, _, _)| range < kept).unwrap_or(true) {
        *best = Some((range, point, name));
    }
}

fn enter_prism(origin: Vec3, direction: Vec3, plate: &Plate) -> Option<f64> {
    let mut near = 0.0;
    let mut far = f64::INFINITY;
    if !clip_slab(
        origin.z,
        direction.z,
        plate.z_min_m,
        plate.z_max_m,
        &mut near,
        &mut far,
    ) {
        return None;
    }
    let count = plate.polygon_xy_m.len();
    for index in 0..count {
        let a = plate.polygon_xy_m[index];
        let b = plate.polygon_xy_m[(index + 1) % count];
        let nx = -(b.1 - a.1);
        let ny = b.0 - a.0;
        let offset = nx * (origin.x - a.0) + ny * (origin.y - a.1);
        let denom = nx * direction.x + ny * direction.y;
        if !clip_half(offset, denom, &mut near, &mut far) {
            return None;
        }
    }
    (near > 1e-9 && near <= far).then_some(near)
}

fn clip_slab(
    origin: f64,
    direction: f64,
    min_axis: f64,
    max_axis: f64,
    near: &mut f64,
    far: &mut f64,
) -> bool {
    if direction.abs() < 1e-12 {
        return origin >= min_axis - CONTACT_EPS_M && origin <= max_axis + CONTACT_EPS_M;
    }
    let mut t1 = (min_axis - origin) / direction;
    let mut t2 = (max_axis - origin) / direction;
    if t1 > t2 {
        std::mem::swap(&mut t1, &mut t2);
    }
    *near = near.max(t1);
    *far = far.min(t2);
    *near <= *far
}

fn clip_half(offset: f64, denom: f64, near: &mut f64, far: &mut f64) -> bool {
    if denom.abs() < 1e-12 {
        return offset >= -1e-9;
    }
    let t = -offset / denom;
    if denom > 0.0 {
        *near = near.max(t);
    } else {
        *far = far.min(t);
    }
    *near <= *far
}

fn intent_name(intent: Intent) -> &'static str {
    match intent {
        Intent::Move(_) => "Move",
        Intent::Stay => "Stay",
        Intent::Blocked => "Blocked",
        Intent::Stuck => "Stuck",
        Intent::Lost => "Lost",
        Intent::WrongPerson => "WrongPerson",
        Intent::Arrived => "Arrived",
        Intent::InReach => "InReach",
        Intent::Cancelled => "Cancelled",
    }
}

fn model_csv() -> String {
    let mut body = String::from("name,role,z_min_m,z_max_m,source\n");
    for plate in collision_plates() {
        body.push_str(&format!(
            "{},collision,{:.4},{:.4},{}\n",
            plate.name, plate.z_min_m, plate.z_max_m, plate.source
        ));
    }
    for item in visual_placeholders() {
        body.push_str(&format!("{},visual,,,{}\n", item.name, item.source));
    }
    body.push_str(&format!(
        "lidar_xyz,configured mount,{LIDAR_Z_M:.4},,{LIDAR_X_M:.3} 0 {LIDAR_Z_M:.2} m from pi robot.json. Not an installed lever.\n"
    ));
    body
}

fn trace_csv(rows: &[TraceRow]) -> String {
    let mut body = String::from(
        "index,sensor_hits,policy,attempted_x_m,attempted_y_m,attempted_yaw_deg,accepted_x_m,accepted_y_m,accepted_yaw_deg,outcome,sent\n",
    );
    for row in rows {
        body.push_str(&format!(
            "{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{},{}\n",
            row.index,
            row.sensor_hits,
            row.policy,
            row.attempted_x_m,
            row.attempted_y_m,
            row.attempted_yaw_deg,
            row.accepted_x_m,
            row.accepted_y_m,
            row.accepted_yaw_deg,
            row.outcome,
            row.sent
        ));
    }
    body
}

fn draw_top(low: &Aabb, dx: f64) -> String {
    let start = BasePose {
        x_m: 0.0,
        y_m: 0.0,
        yaw_deg: 0.0,
    };
    let attempted = shifted(start, dx, 0.0, 0.0);
    let mut cloud = Vec::new();
    for plate in collision_plates() {
        cloud.extend(placed(start, plate.polygon_xy_m));
        cloud.extend(placed(attempted, plate.polygon_xy_m));
    }
    cloud.push((low.min.x, low.min.y));
    cloud.push((low.max.x, low.max.y));
    cloud.push((LIDAR_X_M, 0.0));
    cloud.push((LIDAR_X_M + 1.0, 0.0));
    let plane = fit_plane(&cloud, 70.0, 168.0, 900.0, 450.0);
    let mut geometry = Vec::new();
    geometry.extend(ring(&plane, &placed(start, &FOOTPRINT_XY_M)));
    geometry.extend(ring(&plane, &placed(start, &TOP_XY)));
    geometry.extend(ring(&plane, &placed(attempted, &FOOTPRINT_XY_M)));
    geometry.extend(box_ring(&plane, low.min.x, low.max.x, low.min.y, low.max.y));
    let (axes, _) = plane_axes(
        &plane,
        ("world x", "world y", "x", "y"),
        &cloud,
        0.0,
        &geometry,
    );
    let mut body =
        svg_header("Base top-down, metres. Origin is base_link, not the sensor. Equal scale.");
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"12\">Plates are the configured octagon. Layer 3 has the 0.05 m photo corner cut. The dashed head is assumed and is not collision.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"12\">The red rays cross the low-box footprint. The side view shows those rays above the box. The blue outline is the attempted step, which was not accepted.</text>");
    body.push_str("<text x=\"24\" y=\"136\" font-size=\"12\">Lidar xyz (0.175, 0, 0.22) m is configured. It is not an installed lever arm. The 250 mm disk is not drawn as chassis clearance.</text>");
    body.push_str(&axes);
    body.push_str(&paint_ring(
        &plane,
        &placed(start, &FOOTPRINT_XY_M),
        "#222222",
        "",
    ));
    body.push_str(&paint_ring(&plane, &placed(start, &TOP_XY), "#555555", ""));
    body.push_str(&paint_ring(
        &plane,
        &placed(attempted, &FOOTPRINT_XY_M),
        "#1f4e79",
        "stroke-dasharray=\"5 4\"",
    ));
    body.push_str(&paint_ring_rect(&plane, low, "#c0392b"));
    body.push_str(&miss_rays(&plane, 0.0));
    body.push_str(&sensor_mark(&plane));
    body.push_str(&swatches(
        &[
            ("#222222", "layer 1 and 2 plan"),
            ("#555555", "layer 3 plan"),
            ("#1f4e79", "attempted, not accepted"),
            ("#c0392b", "low box and miss rays"),
        ],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn draw_side(low: &Aabb) -> String {
    let mut cloud = vec![
        (-0.6, 0.0),
        (1.2, 0.0),
        (-0.6, 0.40),
        (-0.2, 0.245),
        (LIDAR_X_M, LIDAR_Z_M),
        (low.min.x, low.min.z),
        (low.max.x, low.max.z),
    ];
    cloud.push((LIDAR_X_M + 1.0, LIDAR_Z_M));
    let plane = fit_plane(&cloud, 70.0, 168.0, 900.0, 450.0);
    let mut geometry = Vec::new();
    for plate in collision_plates() {
        geometry.extend(box_ring_xz(&plane, -0.2, 0.2, plate.z_min_m, plate.z_max_m));
    }
    geometry.extend(box_ring_xz(
        &plane, low.min.x, low.max.x, low.min.z, low.max.z,
    ));
    let (axes, _) = plane_axes(
        &plane,
        ("world x", "world z", "x", "z"),
        &cloud,
        0.0,
        &geometry,
    );
    let mut body =
        svg_header("Base side, x and z, equal metres. The horizontal rays pass above the low box.");
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"12\">Layer slabs are z 0.030–0.032, 0.120–0.122, and 0.242–0.245 m. Thicknesses are assumed. The gaps are the taped values.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"12\">A ray at the configured lidar height misses the low box. The body still meets that box if it translates into it. The dashed head is not collision.</text>");
    body.push_str(&axes);
    for plate in collision_plates() {
        body.push_str(&paint_rect_xz(
            &plane,
            -0.2,
            0.2,
            plate.z_min_m,
            plate.z_max_m,
            "#222222",
            "",
        ));
    }
    body.push_str(&paint_rect_xz(
        &plane, low.min.x, low.max.x, low.min.z, low.max.z, "#c0392b", "",
    ));
    body.push_str(&paint_rect_xz(
        &plane,
        LIDAR_X_M - 0.028,
        LIDAR_X_M + 0.028,
        LIDAR_Z_M - 0.02,
        LIDAR_Z_M + 0.02,
        "#888888",
        "stroke-dasharray=\"4 3\"",
    ));
    let sensor = plane.screen(LIDAR_X_M, LIDAR_Z_M);
    let far = plane.screen(LIDAR_X_M + 1.0, LIDAR_Z_M);
    body.push_str(&format!(
        "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#c0392b\" stroke-width=\"1.4\"/>",
        sensor.0, sensor.1, far.0, far.1
    ));
    body.push_str(&format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.5\" fill=\"#111111\"/>",
        sensor.0, sensor.1
    ));
    let label = plane.screen(-0.55, 0.38);
    body.push_str(&format!(
        "<line data-leader=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#888888\" stroke-width=\"0.8\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"end\">configured lidar</text>",
        sensor.0, sensor.1, label.0, label.1, label.0, label.1
    ));
    body.push_str(&swatches(
        &[
            ("#222222", "collision plates"),
            ("#c0392b", "low box and miss ray"),
            ("#888888", "assumed head, not collision"),
        ],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn draw_oblique(low: &Aabb) -> String {
    let mut cloud = Vec::new();
    let mut corners = Vec::new();
    for plate in collision_plates() {
        for (x, y) in plate.polygon_xy_m {
            corners.push(Vec3 {
                x: *x,
                y: *y,
                z: plate.z_min_m,
            });
            corners.push(Vec3 {
                x: *x,
                y: *y,
                z: plate.z_max_m,
            });
        }
    }
    corners.push(low.min);
    corners.push(low.max);
    corners.push(Vec3 {
        x: LIDAR_X_M,
        y: 0.0,
        z: LIDAR_Z_M,
    });
    corners.push(Vec3 {
        x: LIDAR_X_M + 1.0,
        y: 0.0,
        z: LIDAR_Z_M,
    });
    let project = |point: Vec3| {
        let (right, down) = project_oblique(point);
        (right, -down)
    };
    for corner in &corners {
        cloud.push(project(*corner));
    }
    let plane = fit_plane(&cloud, 80.0, 168.0, 880.0, 450.0);
    let map = |point: Vec3| {
        let (right, up) = project(point);
        plane.screen(right, up)
    };
    let mut pending = Vec::new();
    let mut body = svg_header("Oblique view of the base plates and one low box. Not a mesh.");
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"12\">Ticks are metres along each world axis. This cabinet projection does not use one screen scale for every axis.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"12\">The low box is below the lidar plane. A level ray does not meet it. The plates are the only collision solids.</text>");
    let (axis, _) = world_axis(
        &map,
        "z",
        "world z",
        &tick_values(0.0, 0.5),
        |metres| Vec3 {
            x: 0.0,
            y: 0.0,
            z: metres,
        },
        ((16.0, 0.0), (-64.0, 14.0)),
        &mut pending,
    );
    body.push_str(&axis);
    let (axis, _) = world_axis(
        &map,
        "x",
        "world x",
        &tick_values(-0.5, 1.0),
        |metres| Vec3 {
            x: metres,
            y: 0.0,
            z: 0.0,
        },
        ((0.0, 16.0), (12.0, -16.0)),
        &mut pending,
    );
    body.push_str(&axis);
    let (axis, _) = world_axis(
        &map,
        "y",
        "world y",
        &tick_values(-0.5, 0.5),
        |metres| Vec3 {
            x: 0.0,
            y: metres,
            z: 0.0,
        },
        ((-16.0, 8.0), (14.0, 18.0)),
        &mut pending,
    );
    body.push_str(&axis);
    let mut geometry = Vec::new();
    for plate in collision_plates() {
        body.push_str(&prism_edges(&plate, map, &mut geometry));
    }
    body.push_str(&aabb_edges(low, map, &mut geometry));
    let sensor = Vec3 {
        x: LIDAR_X_M,
        y: 0.0,
        z: LIDAR_Z_M,
    };
    let far = Vec3 {
        x: LIDAR_X_M + 1.0,
        y: 0.0,
        z: LIDAR_Z_M,
    };
    let a = map(sensor);
    let b = map(far);
    body.push_str(&format!(
        "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#c0392b\" stroke-width=\"1.4\"/>",
        a.0, a.1, b.0, b.1
    ));
    geometry.push((a, b));
    let (ticks, _) = place_ticks(&pending, &geometry);
    body.push_str(&ticks);
    body.push_str(&swatches(
        &[("#222222", "collision plates"), ("#c0392b", "miss ray")],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn draw_pose(rows: &[TraceRow]) -> String {
    let mut cloud = Vec::new();
    for row in rows {
        cloud.push((f64::from(row.index), row.attempted_x_m));
        cloud.push((f64::from(row.index), row.accepted_x_m));
    }
    let plane = fit_plane(&cloud, 120.0, 180.0, 760.0, 420.0);
    let (axes, _) = plane_axes(
        &plane,
        ("step index", "x metres", "step", "x"),
        &cloud,
        0.0,
        &[],
    );
    let mut body = svg_header(
        "Attempted and accepted base x. The index is a kinematic step, not a measured clock.",
    );
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"12\">Step 0 is the sensor cycle: no person, so the policy is Lost and the base stays. Step 1 attempts a 0.20 m Move into the low box and is rejected.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"12\">Step 2 is the same Move after the box is removed. The cap of 1 m and 90 deg is not the configured 0.15 m/s motor limit. sent stays false.</text>");
    body.push_str(&axes);
    body.push_str(&series(&plane, rows, true, "#1f4e79"));
    body.push_str(&series(&plane, rows, false, "#c0392b"));
    body.push_str(&swatches(
        &[("#1f4e79", "attempted x"), ("#c0392b", "accepted x")],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn series(plane: &Plane, rows: &[TraceRow], attempted: bool, color: &str) -> String {
    let mut body = String::new();
    let mut previous: Option<(f64, f64)> = None;
    for row in rows {
        let x = if attempted {
            row.attempted_x_m
        } else {
            row.accepted_x_m
        };
        let point = plane.screen(f64::from(row.index), x);
        if let Some(from) = previous {
            body.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{color}\" stroke-width=\"1.6\"/>",
                from.0, from.1, point.0, point.1
            ));
        }
        body.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.5\" fill=\"{color}\"/>",
            point.0, point.1
        ));
        previous = Some(point);
    }
    body
}

fn miss_rays(plane: &Plane, z_unused: f64) -> String {
    let _ = z_unused;
    let mut body = String::new();
    let mut bearing = -16.0_f64;
    while bearing <= 16.0 + 1e-9 {
        let rad = bearing.to_radians();
        let end = (LIDAR_X_M + rad.cos(), rad.sin());
        let a = plane.screen(LIDAR_X_M, 0.0);
        let b = plane.screen(end.0, end.1);
        body.push_str(&format!(
            "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#c0392b\" stroke-width=\"1.1\"/>",
            a.0, a.1, b.0, b.1
        ));
        bearing += 8.0;
    }
    body
}

fn sensor_mark(plane: &Plane) -> String {
    let sensor = plane.screen(LIDAR_X_M, 0.0);
    let label = (sensor.0, sensor.1 + 28.0);
    let head = plane.screen(LIDAR_X_M + 0.028, 0.0);
    let radius = (head.0 - sensor.0).abs();
    format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{radius:.1}\" fill=\"none\" stroke=\"#888888\" stroke-dasharray=\"3 3\"/><circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.5\" fill=\"#111111\"/><line data-leader=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#888888\" stroke-width=\"0.8\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"middle\">configured lidar</text>",
        sensor.0,
        sensor.1,
        sensor.0,
        sensor.1,
        sensor.0,
        sensor.1,
        label.0,
        label.1,
        label.0,
        label.1
    )
}

fn ring(plane: &Plane, polygon: &[(f64, f64)]) -> Vec<ScreenSeg> {
    let screen: Vec<_> = polygon.iter().map(|(x, y)| plane.screen(*x, *y)).collect();
    let mut lines = Vec::new();
    for index in 0..screen.len() {
        lines.push((screen[index], screen[(index + 1) % screen.len()]));
    }
    lines
}

fn box_ring(plane: &Plane, x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<ScreenSeg> {
    ring(plane, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

fn box_ring_xz(plane: &Plane, x0: f64, x1: f64, z0: f64, z1: f64) -> Vec<ScreenSeg> {
    ring(plane, &[(x0, z0), (x1, z0), (x1, z1), (x0, z1)])
}

fn paint_ring(plane: &Plane, polygon: &[(f64, f64)], color: &str, extra: &str) -> String {
    let mut body = String::new();
    let screen: Vec<_> = polygon.iter().map(|(x, y)| plane.screen(*x, *y)).collect();
    for index in 0..screen.len() {
        let a = screen[index];
        let b = screen[(index + 1) % screen.len()];
        body.push_str(&format!(
            "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{color}\" stroke-width=\"1.4\" {extra}/>",
            a.0, a.1, b.0, b.1
        ));
    }
    body
}

fn paint_ring_rect(plane: &Plane, body_box: &Aabb, color: &str) -> String {
    paint_ring(
        plane,
        &[
            (body_box.min.x, body_box.min.y),
            (body_box.max.x, body_box.min.y),
            (body_box.max.x, body_box.max.y),
            (body_box.min.x, body_box.max.y),
        ],
        color,
        "",
    )
}

fn paint_rect_xz(
    plane: &Plane,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    color: &str,
    extra: &str,
) -> String {
    paint_ring(
        plane,
        &[(x0, z0), (x1, z0), (x1, z1), (x0, z1)],
        color,
        extra,
    )
}

fn prism_edges(
    plate: &Plate,
    map: impl Fn(Vec3) -> (f64, f64),
    geometry: &mut Vec<ScreenSeg>,
) -> String {
    let mut body = String::new();
    let count = plate.polygon_xy_m.len();
    for index in 0..count {
        let (x, y) = plate.polygon_xy_m[index];
        let (nx, ny) = plate.polygon_xy_m[(index + 1) % count];
        for z in [plate.z_min_m, plate.z_max_m] {
            body.push_str(&edge(
                map(Vec3 { x, y, z }),
                map(Vec3 { x: nx, y: ny, z }),
                geometry,
            ));
        }
        body.push_str(&edge(
            map(Vec3 {
                x,
                y,
                z: plate.z_min_m,
            }),
            map(Vec3 {
                x,
                y,
                z: plate.z_max_m,
            }),
            geometry,
        ));
    }
    body
}

fn aabb_edges(
    body_box: &Aabb,
    map: impl Fn(Vec3) -> (f64, f64),
    geometry: &mut Vec<ScreenSeg>,
) -> String {
    let min = body_box.min;
    let max = body_box.max;
    let corners = [
        Vec3 {
            x: min.x,
            y: min.y,
            z: min.z,
        },
        Vec3 {
            x: max.x,
            y: min.y,
            z: min.z,
        },
        Vec3 {
            x: max.x,
            y: max.y,
            z: min.z,
        },
        Vec3 {
            x: min.x,
            y: max.y,
            z: min.z,
        },
        Vec3 {
            x: min.x,
            y: min.y,
            z: max.z,
        },
        Vec3 {
            x: max.x,
            y: min.y,
            z: max.z,
        },
        Vec3 {
            x: max.x,
            y: max.y,
            z: max.z,
        },
        Vec3 {
            x: min.x,
            y: max.y,
            z: max.z,
        },
    ];
    let mut body = String::new();
    for index in 0..4 {
        body.push_str(&edge(
            map(corners[index]),
            map(corners[(index + 1) % 4]),
            geometry,
        ));
        body.push_str(&edge(
            map(corners[index + 4]),
            map(corners[(index + 1) % 4 + 4]),
            geometry,
        ));
        body.push_str(&edge(
            map(corners[index]),
            map(corners[index + 4]),
            geometry,
        ));
    }
    body
}

fn edge(a: (f64, f64), b: (f64, f64), geometry: &mut Vec<ScreenSeg>) -> String {
    geometry.push((a, b));
    format!(
        "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.2\"/>",
        a.0, a.1, b.0, b.1
    )
}

#[cfg(test)]
mod tests {
    use domain::{Mm, Point};

    use super::{
        BlockReason, BodyHit, StepFault, cast_sensor, collision_plates, integrate, resting_contact,
        scan_step_m, sense_policy, visual_placeholders,
    };
    use crate::kind::Kind;
    use crate::scene::{Aabb, Vec3};

    fn pose(x: f64, y: f64, yaw: f64) -> super::BasePose {
        super::BasePose {
            x_m: x,
            y_m: y,
            yaw_deg: yaw,
        }
    }

    fn solid(name: &'static str, min: Vec3, max: Vec3) -> Aabb {
        Aabb { name, min, max }
    }

    fn wall_at_one() -> Aabb {
        solid(
            "wall",
            Vec3 {
                x: 1.0,
                y: -0.5,
                z: 0.0,
            },
            Vec3 {
                x: 1.05,
                y: 0.5,
                z: 1.0,
            },
        )
    }

    #[test]
    fn plates_use_the_taped_gaps_and_the_assumed_thickness() {
        let plates = collision_plates();
        let bottom = plates
            .iter()
            .find(|plate| plate.name == "chassis_bottom")
            .unwrap();
        let middle = plates
            .iter()
            .find(|plate| plate.name == "chassis_middle")
            .unwrap();
        let top = plates
            .iter()
            .find(|plate| plate.name == "top_plate")
            .unwrap();
        assert!((bottom.z_min_m - 0.030).abs() < 1e-12);
        assert!((bottom.z_max_m - (0.030 + 0.002)).abs() < 1e-12);
        assert!((middle.z_max_m - (0.242 - 0.120)).abs() < 1e-12);
        assert!((middle.z_min_m - (middle.z_max_m - 0.002)).abs() < 1e-12);
        assert!((top.z_min_m - (0.030 + 0.002 + 0.210)).abs() < 1e-12);
        assert!((top.z_max_m - (top.z_min_m + 0.003)).abs() < 1e-12);
        let front: Vec<_> = top
            .polygon_xy_m
            .iter()
            .filter(|point| (point.0 - 0.20).abs() < 1e-12)
            .collect();
        assert_eq!(front.len(), 2);
        let mut ys: Vec<f64> = front.iter().map(|point| point.1).collect();
        ys.sort_by(f64::total_cmp);
        assert!((ys[0] + 0.15).abs() < 1e-12);
        assert!((ys[1] - 0.15).abs() < 1e-12);
        assert!(
            visual_placeholders()
                .iter()
                .any(|item| item.kind == Kind::Assumed)
        );
        assert!(plates.iter().all(|plate| plate.name != "lidar_head"));
    }

    #[test]
    fn a_measured_forward_ray_hits_the_wall_at_the_configured_lever() {
        let hit = cast_sensor(
            &pose(0.0, 0.0, 0.0),
            "measured-front",
            0.0,
            0.0,
            &[wall_at_one()],
        )
        .unwrap()
        .expect("hit");
        assert!(
            (hit.range_m - (1.0 - 0.175)).abs() < 1e-9,
            "{}",
            hit.range_m
        );
        assert!((hit.point.x - 1.0).abs() < 1e-6);
        assert!(hit.point.y.abs() < 1e-6);
        assert!((hit.point.z - 0.22).abs() < 1e-6);
        assert_eq!(hit.name, "wall");
    }

    #[test]
    fn a_configured_front_is_not_the_measured_ray() {
        let hit = cast_sensor(
            &pose(0.0, 0.0, 0.0),
            "configured-front",
            0.0,
            0.0,
            &[wall_at_one()],
        )
        .unwrap()
        .expect("hit");
        let rad = (183.5 - 178.7) * std::f64::consts::PI / 180.0;
        let range = (1.0 - 0.175) / rad.cos();
        assert!((hit.range_m - range).abs() < 1e-9, "{}", hit.range_m);
        assert!((hit.point.y - range * rad.sin()).abs() < 1e-6);
        assert!(hit.point.y.abs() > 0.05);
    }

    #[test]
    fn a_level_ray_misses_the_plates_and_a_downward_ray_hits_one() {
        assert!(
            cast_sensor(&pose(0.0, 0.0, 0.0), "measured-front", 0.0, 0.0, &[])
                .unwrap()
                .is_none()
        );
        let hit = cast_sensor(&pose(0.0, 0.0, 0.0), "measured-front", 90.0, 0.0, &[])
            .unwrap()
            .expect("plate");
        assert_eq!(hit.name, "chassis_middle");
        assert!(
            (hit.range_m - (0.22 - 0.122)).abs() < 1e-9,
            "{}",
            hit.range_m
        );
    }

    #[test]
    fn a_gap_solid_misses_and_the_layer_slab_hits() {
        let gap = solid(
            "gap",
            Vec3 {
                x: -0.05,
                y: -0.05,
                z: 0.05,
            },
            Vec3 {
                x: 0.05,
                y: 0.05,
                z: 0.10,
            },
        );
        let slab = solid(
            "slab",
            Vec3 {
                x: -0.05,
                y: -0.05,
                z: 0.025,
            },
            Vec3 {
                x: 0.05,
                y: 0.05,
                z: 0.035,
            },
        );
        assert!(matches!(
            resting_contact(pose(0.0, 0.0, 0.0), &[gap]),
            BodyHit::Clear
        ));
        match resting_contact(pose(0.0, 0.0, 0.0), &[slab]) {
            BodyHit::Blocked {
                reason,
                plate,
                obstacle,
            } => {
                assert_eq!(reason, BlockReason::Overlap);
                assert_eq!(plate, "chassis_bottom");
                assert_eq!(obstacle, "slab");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_rear_solid_outside_the_sensor_disk_still_blocks_the_body() {
        let rear = solid(
            "rear",
            Vec3 {
                x: -0.30,
                y: -0.05,
                z: 0.025,
            },
            Vec3 {
                x: -0.25,
                y: 0.05,
                z: 0.035,
            },
        );
        let disk_rear = 0.175 - 0.250;
        assert!(rear.max.x < disk_rear);
        let step = integrate(pose(0.0, 0.0, 0.0), -0.08, 0.0, 0.0, &[rear]);
        assert!(step.pose.x_m.abs() < 1e-12);
        assert!(matches!(
            step.hit,
            BodyHit::Blocked {
                reason: BlockReason::Sweep,
                ..
            }
        ));
        assert!(!step.sent);
    }

    #[test]
    fn a_step_over_a_thin_wall_is_blocked_until_the_wall_is_removed() {
        let wall = solid(
            "thin",
            Vec3 {
                x: 0.30,
                y: -0.05,
                z: 0.025,
            },
            Vec3 {
                x: 0.34,
                y: 0.05,
                z: 0.035,
            },
        );
        let blocked = integrate(pose(0.0, 0.0, 0.0), 0.80, 0.0, 0.0, &[wall]);
        assert!(blocked.pose.x_m.abs() < 1e-12);
        assert!(matches!(
            blocked.hit,
            BodyHit::Blocked {
                reason: BlockReason::Sweep,
                ..
            }
        ));
        let open = integrate(pose(0.0, 0.0, 0.0), 0.80, 0.0, 0.0, &[]);
        assert!((open.pose.x_m - 0.80).abs() < 1e-12);
        assert!(matches!(open.hit, BodyHit::Clear));
    }

    #[test]
    fn a_yaw_sweep_hits_a_corner_the_end_poses_miss() {
        let wall = solid(
            "corner",
            Vec3 {
                x: 0.25,
                y: -0.02,
                z: 0.025,
            },
            Vec3 {
                x: 0.40,
                y: 0.02,
                z: 0.035,
            },
        );
        assert!(matches!(
            resting_contact(pose(0.0, 0.0, 0.0), &[wall]),
            BodyHit::Clear
        ));
        assert!(matches!(
            resting_contact(pose(0.0, 0.0, 90.0), &[wall]),
            BodyHit::Clear
        ));
        let turned = integrate(pose(0.0, 0.0, 0.0), 0.0, 0.0, 90.0, &[wall]);
        assert!(turned.pose.yaw_deg.abs() < 1e-12);
        assert!(matches!(
            turned.hit,
            BodyHit::Blocked {
                reason: BlockReason::Sweep,
                ..
            }
        ));
    }

    #[test]
    fn an_oversize_or_non_finite_step_is_rejected() {
        let far = integrate(pose(0.0, 0.0, 0.0), 1.5, 0.0, 0.0, &[]);
        assert!(matches!(
            far.hit,
            BodyHit::Rejected {
                fault: StepFault::Oversize
            }
        ));
        assert!(far.pose.x_m.abs() < 1e-12);
        let spin = integrate(pose(0.0, 0.0, 0.0), 0.0, 0.0, 120.0, &[]);
        assert!(matches!(
            spin.hit,
            BodyHit::Rejected {
                fault: StepFault::Oversize
            }
        ));
        let bad = integrate(pose(0.0, 0.0, 0.0), f64::NAN, 0.0, 0.0, &[]);
        assert!(matches!(
            bad.hit,
            BodyHit::Rejected {
                fault: StepFault::NonFinite
            }
        ));
    }

    #[test]
    fn grazing_the_front_face_is_contact() {
        let face = solid(
            "face",
            Vec3 {
                x: 0.20,
                y: -0.05,
                z: 0.025,
            },
            Vec3 {
                x: 0.30,
                y: 0.05,
                z: 0.035,
            },
        );
        match resting_contact(pose(0.0, 0.0, 0.0), &[face]) {
            BodyHit::Blocked { reason, .. } => assert_eq!(reason, BlockReason::Contact),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_low_solid_is_absent_from_the_posted_view_while_the_body_blocks() {
        let low = solid(
            "low",
            Vec3 {
                x: 0.40,
                y: -0.10,
                z: 0.0,
            },
            Vec3 {
                x: 0.60,
                y: 0.10,
                z: 0.10,
            },
        );
        let sensed = sense_policy(&pose(0.0, 0.0, 0.0), "measured-front", &[low], &[0.0]).unwrap();
        assert_eq!(sensed.hits, 0);
        assert_eq!(sensed.posted_points, 0);
        assert_eq!(sensed.intent_name, "Lost");
        assert!(!sensed.sent);
        let step = integrate(pose(0.0, 0.0, 0.0), 0.25, 0.0, 0.0, &[low]);
        assert!(matches!(
            step.hit,
            BodyHit::Blocked {
                reason: BlockReason::Sweep,
                ..
            }
        ));
        assert!(step.pose.x_m.abs() < 1e-12);
    }

    #[test]
    fn a_move_does_not_add_the_lever_arm() {
        let from = Point::new(Mm::new(0), Mm::new(0));
        let to = Point::new(Mm::new(200), Mm::new(0));
        let (dx, dy) = scan_step_m(from, to, "measured-front").unwrap();
        assert!((dx - 0.2).abs() < 1e-12);
        assert!(dy.abs() < 1e-12);
        let (cx, cy) = scan_step_m(from, to, "configured-front").unwrap();
        let rad = (183.5 - 178.7) * std::f64::consts::PI / 180.0;
        assert!((cx - 0.2 * rad.cos()).abs() < 1e-12);
        assert!((cy - 0.2 * rad.sin()).abs() < 1e-12);
        assert!((cx - 0.2).abs() > 1e-4);
        assert!((cx - (0.2 - 0.175)).abs() > 1e-3);
    }

    #[test]
    fn the_demo_trace_rejects_the_low_box_and_accepts_after_removal() {
        let views = super::demo_views().unwrap();
        assert!(views.trace_csv.contains(",Lost,"));
        assert!(views.trace_csv.contains(",sweep,"));
        assert!(views.trace_csv.contains(",accepted,"));
        assert!(views.trace_csv.contains(",false\n"));
        assert!(views.model_csv.contains("lidar_head,visual"));
        assert!(views.top_svg.contains("data-tick-stroke"));
        assert!(views.side_svg.contains("data-axis-line"));
        assert!(views.oblique_svg.contains("world z"));
        assert!(views.pose_svg.contains("data-axis-tick"));
    }
}
