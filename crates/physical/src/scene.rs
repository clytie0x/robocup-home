//! Static boxes and one named lidar mount. Hits are ray intersections.
//!
//! The mount in this module is a synthetic fixture. It is not `LIDAR_X_M`
//! and it is not an installed measurement. A returned range is the distance
//! along the 3D ray. `frame_sample` then places that range in the scan plane.

use crate::lidar::{RawSample, encode_packet};
use crate::navpost::frame_sample;
use crate::profile::{LidarProfile, RANGE_DIVISOR, wrap_full_deg, wrap_signed_deg};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    pub name: &'static str,
    pub translation_m: Vec3,
    pub yaw_deg: f64,
    pub pitch_down_deg: f64,
    pub provenance: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub name: &'static str,
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub mount: Mount,
    pub bodies: Vec<Aabb>,
    pub max_range_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cast {
    Hit { range_m: f64, point: Vec3 },
    Miss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneError {
    NonFinite,
    NegativeRange,
    Uncodable,
}

pub struct PacketScene {
    pub bytes: Vec<u8>,
    pub bearings_deg: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneViews {
    pub mount_csv: String,
    pub rays_csv: String,
    pub oblique_svg: String,
    pub top_svg: String,
    pub side_svg: String,
    pub scan_svg: String,
}

/// Cabinet projection. Screen y grows downward, so a larger z is higher on the page.
pub fn project_oblique(point: Vec3) -> (f64, f64) {
    (point.x + 0.5 * point.y, -point.z + 0.25 * point.y)
}

pub fn demonstration() -> Scene {
    Scene {
        mount: Mount {
            name: "synthetic-lidar",
            translation_m: Vec3 {
                x: 0.40,
                y: -0.12,
                z: 0.50,
            },
            yaw_deg: 15.0,
            pitch_down_deg: 0.0,
            provenance: "synthetic fixture for this slice; not the configured 0.175 m lever and not an installed measurement",
        },
        bodies: vec![
            Aabb {
                name: "pillar",
                min: Vec3 {
                    x: 1.60,
                    y: -0.25,
                    z: 0.0,
                },
                max: Vec3 {
                    x: 1.90,
                    y: 0.25,
                    z: 1.20,
                },
            },
            Aabb {
                name: "low-plank",
                min: Vec3 {
                    x: 1.10,
                    y: -0.40,
                    z: 0.0,
                },
                max: Vec3 {
                    x: 1.30,
                    y: 0.40,
                    z: 0.20,
                },
            },
        ],
        max_range_m: 8.0,
    }
}

pub fn cast_bearing(scene: &Scene, bearing_deg: f64) -> Result<Cast, SceneError> {
    if scene.max_range_m < 0.0 {
        return Err(SceneError::NegativeRange);
    }
    if !scene.max_range_m.is_finite()
        || !scene.mount.translation_m.finite()
        || !scene.mount.yaw_deg.is_finite()
        || !scene.mount.pitch_down_deg.is_finite()
        || !bearing_deg.is_finite()
    {
        return Err(SceneError::NonFinite);
    }
    let direction = scan_direction(&scene.mount, bearing_deg)?;
    let mut best: Option<(f64, Vec3)> = None;
    for body in &scene.bodies {
        if !body.min.finite() || !body.max.finite() {
            return Err(SceneError::NonFinite);
        }
        let Some(range) = enter_box(scene.mount.translation_m, direction, body) else {
            continue;
        };
        if range > scene.max_range_m {
            continue;
        }
        if best.map(|(kept, _)| range < kept).unwrap_or(true) {
            best = Some((range, scene.mount.translation_m.add(direction.scale(range))));
        }
    }
    Ok(match best {
        Some((range_m, point)) => Cast::Hit { range_m, point },
        None => Cast::Miss,
    })
}

pub fn inverse_bearing(mount: &Mount, point: Vec3) -> Result<f64, SceneError> {
    if !mount.translation_m.finite()
        || !mount.yaw_deg.is_finite()
        || !mount.pitch_down_deg.is_finite()
        || !point.finite()
    {
        return Err(SceneError::NonFinite);
    }
    let local = world_to_scan(mount, point);
    Ok(wrap_signed_deg(local.y.atan2(local.x).to_degrees()))
}

pub fn sensor_samples(
    scene: &Scene,
    bearings_deg: &[f64],
    front_raw_deg: f64,
) -> Result<Vec<RawSample>, SceneError> {
    if !front_raw_deg.is_finite() {
        return Err(SceneError::NonFinite);
    }
    let mut samples = Vec::with_capacity(bearings_deg.len());
    for (index, bearing) in bearings_deg.iter().copied().enumerate() {
        let range_m = match cast_bearing(scene, bearing)? {
            Cast::Hit { range_m, .. } => range_m,
            Cast::Miss => 0.0,
        };
        samples.push(RawSample {
            at: index,
            raw_deg: wrap_full_deg(front_raw_deg - bearing),
            range_m,
            intensity: 0,
        });
    }
    Ok(samples)
}

/// Evenly spaced raw angles, starting at 0, encoded with ranges from the ray cast.
///
/// A packet stores the first and last angle and the parser fills the steps between
/// them, so `step_deg` has to divide the packet evenly. A miss is a zero range.
pub fn packet_revolution(
    scene: &Scene,
    front_raw_deg: f64,
    step_deg: f64,
) -> Result<PacketScene, SceneError> {
    if !front_raw_deg.is_finite() || !step_deg.is_finite() || step_deg <= 0.0 || step_deg >= 360.0 {
        return Err(SceneError::Uncodable);
    }
    let mut bearings = Vec::new();
    let mut triples = Vec::new();
    let mut raw = 0.0;
    while raw < 360.0 - step_deg * 0.5 {
        let bearing = wrap_signed_deg(front_raw_deg - raw);
        let range = match cast_bearing(scene, bearing)? {
            Cast::Hit { range_m, .. } => range_m,
            Cast::Miss => 0.0,
        };
        triples.push((raw, range_word(range)?, 0_u8));
        bearings.push(bearing);
        raw += step_deg;
    }
    if triples.is_empty() {
        return Err(SceneError::Uncodable);
    }
    let mut bytes = Vec::new();
    for group in triples.chunks(40) {
        let packet = encode_packet(0, group).map_err(|_| SceneError::Uncodable)?;
        bytes.extend(packet);
    }
    Ok(PacketScene {
        bytes,
        bearings_deg: bearings,
    })
}

pub fn render_views(
    scene: &Scene,
    bearings_deg: &[f64],
    profile: &LidarProfile,
    captured_ms: u64,
) -> Result<SceneViews, SceneError> {
    let mut rays = String::from(
        "pose,bearing_deg,range_m,hit_x_m,hit_y_m,hit_z_m,scan_x_m,scan_y_m,outcome\n",
    );
    let mut scan_points = Vec::new();
    let mut hits = Vec::new();
    for bearing in bearings_deg {
        let cast = cast_bearing(scene, *bearing)?;
        let (range, hit, outcome) = match cast {
            Cast::Hit { range_m, point } => (range_m, Some(point), "hit"),
            Cast::Miss => (0.0, None, "miss"),
        };
        let sample = RawSample {
            at: 0,
            raw_deg: wrap_full_deg(profile.front_raw_deg - bearing),
            range_m: range,
            intensity: 0,
        };
        let framed = frame_sample(profile, sample.raw_deg, sample.range_m);
        let scan = match framed {
            crate::navpost::SampleUse::Body { x_mm, y_mm } => {
                let point = (f64::from(x_mm) / 1000.0, f64::from(y_mm) / 1000.0);
                scan_points.push(point);
                Some(point)
            }
            _ => None,
        };
        if let Some(point) = hit {
            hits.push(point);
        }
        rays.push_str(&format!(
            "{},{:.1},{range:.4},{},{},{},{},{},{outcome}\n",
            scene.mount.name,
            bearing,
            csv_num(hit.map(|point| point.x)),
            csv_num(hit.map(|point| point.y)),
            csv_num(hit.map(|point| point.z)),
            csv_num(scan.map(|point| point.0)),
            csv_num(scan.map(|point| point.1)),
        ));
    }
    let yawed = Scene {
        mount: Mount {
            yaw_deg: 0.0,
            ..scene.mount
        },
        ..scene.clone()
    };
    let mut compare_hits = Vec::new();
    for bearing in bearings_deg {
        if let Cast::Hit { point, .. } = cast_bearing(&yawed, *bearing)? {
            compare_hits.push(point);
        }
    }
    Ok(SceneViews {
        mount_csv: mount_csv(&scene.mount, captured_ms),
        rays_csv: rays,
        oblique_svg: draw_oblique(scene, &hits, &compare_hits),
        top_svg: draw_top(scene, &hits, &compare_hits),
        side_svg: draw_side(scene, &hits),
        scan_svg: draw_scan(scene, profile, &scan_points),
    })
}

fn mount_csv(mount: &Mount, captured_ms: u64) -> String {
    format!(
        "field,value\nname,{}\nframe,scan origin in the synthetic base\ntx_m,{:.4}\nty_m,{:.4}\ntz_m,{:.4}\nyaw_deg,{:.4}\npitch_down_deg,{:.4}\nprovenance,{}\ncaptured_ms,{captured_ms}\nclock,injected-replay\ninstalled_lever_arm,unknown\n",
        mount.name,
        mount.translation_m.x,
        mount.translation_m.y,
        mount.translation_m.z,
        mount.yaw_deg,
        mount.pitch_down_deg,
        mount.provenance,
    )
}

fn scan_direction(mount: &Mount, bearing_deg: f64) -> Result<Vec3, SceneError> {
    let yaw = mount.yaw_deg.to_radians();
    let pitch = mount.pitch_down_deg.to_radians();
    let bearing = bearing_deg.to_radians();
    if !yaw.is_finite() || !pitch.is_finite() || !bearing.is_finite() {
        return Err(SceneError::NonFinite);
    }
    let x1 = pitch.cos() * bearing.cos();
    let y1 = bearing.sin();
    let z1 = -pitch.sin() * bearing.cos();
    Ok(Vec3 {
        x: yaw.cos() * x1 - yaw.sin() * y1,
        y: yaw.sin() * x1 + yaw.cos() * y1,
        z: z1,
    })
}

fn world_to_scan(mount: &Mount, point: Vec3) -> Vec3 {
    let yaw = mount.yaw_deg.to_radians();
    let pitch = mount.pitch_down_deg.to_radians();
    let delta = point.sub(mount.translation_m);
    let x1 = yaw.cos() * delta.x + yaw.sin() * delta.y;
    let y1 = -yaw.sin() * delta.x + yaw.cos() * delta.y;
    let z1 = delta.z;
    Vec3 {
        x: pitch.cos() * x1 - pitch.sin() * z1,
        y: y1,
        z: pitch.sin() * x1 + pitch.cos() * z1,
    }
}

fn enter_box(origin: Vec3, direction: Vec3, body: &Aabb) -> Option<f64> {
    let mut near = 0.0_f64;
    let mut far = f64::INFINITY;
    let axes = [
        (origin.x, direction.x, body.min.x, body.max.x),
        (origin.y, direction.y, body.min.y, body.max.y),
        (origin.z, direction.z, body.min.z, body.max.z),
    ];
    for (origin_axis, direction_axis, min_axis, max_axis) in axes {
        if direction_axis.abs() < 1e-12 {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }
        let mut t1 = (min_axis - origin_axis) / direction_axis;
        let mut t2 = (max_axis - origin_axis) / direction_axis;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        near = near.max(t1);
        far = far.min(t2);
        if near > far {
            return None;
        }
    }
    (near > 1e-9).then_some(near)
}

fn range_word(range_m: f64) -> Result<u16, SceneError> {
    if !range_m.is_finite() || range_m < 0.0 {
        return Err(SceneError::NonFinite);
    }
    let word = (range_m * RANGE_DIVISOR).round();
    if !(0.0..=f64::from(u16::MAX)).contains(&word) {
        return Err(SceneError::Uncodable);
    }
    Ok(word as u16)
}

fn csv_num(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.4}"))
        .unwrap_or_default()
}

type ScreenSeg = ((f64, f64), (f64, f64));

struct Plane {
    origin_x: f64,
    origin_y: f64,
    scale: f64,
}

impl Plane {
    fn screen(&self, right: f64, up: f64) -> (f64, f64) {
        (
            self.origin_x + right * self.scale,
            self.origin_y - up * self.scale,
        )
    }
}

fn draw_oblique(scene: &Scene, hits: &[Vec3], other_hits: &[Vec3]) -> String {
    let project = |point: Vec3| {
        let (right, down) = project_oblique(point);
        (right, -down)
    };
    let mut cloud = Vec::new();
    cloud.push(project(scene.mount.translation_m));
    cloud.push((0.0, 0.0));
    for body in &scene.bodies {
        for corner in corners(body) {
            cloud.push(project(corner));
        }
    }
    for hit in hits.iter().chain(other_hits) {
        cloud.push(project(*hit));
    }
    let plane = fit_plane(&cloud, 120.0, 168.0, 860.0, 430.0);
    let map = |point: Vec3| {
        let (right, up) = project(point);
        plane.screen(right, up)
    };
    let mut geometry = Vec::new();
    let mut body =
        svg_header("Oblique projection of the 3D coordinates. Two mount yaws. Not a robot mesh.");
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">Ticks are metres along each world axis. This cabinet projection does not use one screen scale for every axis.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"13\">Red is the named mount. Blue is yaw 0 at the same translation. Labels sit off the boxes.</text>");
    let mut pending = Vec::new();
    let mut axis_keep = Vec::new();
    let (axis, keepout) = world_axis(
        &map,
        "z",
        "world z",
        &tick_values(0.0, 1.2),
        |metres| Vec3 {
            x: 0.0,
            y: 0.0,
            z: metres,
        },
        ((16.0, 0.0), (-64.0, 14.0)),
        &mut pending,
    );
    body.push_str(&axis);
    axis_keep.extend(keepout);
    let (axis, keepout) = world_axis(
        &map,
        "x",
        "world x",
        &tick_values(0.0, 2.0),
        |metres| Vec3 {
            x: metres,
            y: 0.0,
            z: 0.0,
        },
        ((0.0, 16.0), (12.0, -16.0)),
        &mut pending,
    );
    body.push_str(&axis);
    axis_keep.extend(keepout);
    let (axis, keepout) = world_axis(
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
    axis_keep.extend(keepout);
    body.push_str(&edges(scene, map, &mut geometry));
    body.push_str(&rays(
        scene.mount.translation_m,
        other_hits,
        map,
        "#1f4e79",
        &mut geometry,
    ));
    body.push_str(&rays(
        scene.mount.translation_m,
        hits,
        map,
        "#c0392b",
        &mut geometry,
    ));
    body.push_str(&hit_marks(hits, map));
    let (ticks, tick_keep) = place_ticks(&pending, &geometry);
    body.push_str(&ticks);
    geometry.extend(axis_keep);
    geometry.extend(tick_keep);
    body.push_str(&name_labels(scene, map, &geometry));
    body.push_str(&swatches(
        &[
            ("#c0392b", "named-mount yaw"),
            ("#1f4e79", "yaw 0, same translation"),
        ],
        676.0,
    ));
    body.push_str(&dimension_note(scene));
    body.push_str("</svg>");
    body
}

fn fit_plane(cloud: &[(f64, f64)], left: f64, top: f64, width: f64, height: f64) -> Plane {
    let (mut min_u, mut max_u, mut min_v, mut max_v) = extents(cloud);
    let pad_u = ((max_u - min_u) * 0.12).max(0.3);
    let pad_v = ((max_v - min_v) * 0.16).max(0.3);
    min_u -= pad_u;
    max_u += pad_u;
    min_v -= pad_v;
    max_v += pad_v;
    let span_u = (max_u - min_u).max(1.0);
    let span_v = (max_v - min_v).max(1.0);
    let scale = (width / span_u).min(height / span_v);
    let used_w = span_u * scale;
    let used_h = span_v * scale;
    Plane {
        origin_x: left + (width - used_w) / 2.0 - min_u * scale,
        origin_y: top + (height - used_h) / 2.0 + max_v * scale,
        scale,
    }
}

fn extents(cloud: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let mut min_u = f64::MAX;
    let mut max_u = f64::MIN;
    let mut min_v = f64::MAX;
    let mut max_v = f64::MIN;
    for (u, v) in cloud {
        min_u = min_u.min(*u);
        max_u = max_u.max(*u);
        min_v = min_v.min(*v);
        max_v = max_v.max(*v);
    }
    if cloud.is_empty() {
        return (-1.0, 1.0, -1.0, 1.0);
    }
    (min_u, max_u, min_v, max_v)
}

fn tick_values(min: f64, max: f64) -> Vec<f64> {
    let span = (max - min).abs().max(0.5);
    let step = if span > 6.0 { 1.0 } else { 0.5 };
    let mut value = (min / step).floor() * step;
    let mut ticks = Vec::new();
    loop {
        let snapped = (value / step).round() * step;
        let snapped = if snapped.abs() < 1e-9 { 0.0 } else { snapped };
        ticks.push(snapped);
        if snapped + 1e-9 >= max || ticks.len() > 48 {
            break;
        }
        value += step;
    }
    ticks
}

fn metres_attr(value: f64) -> String {
    if (value - value.round()).abs() < 1e-6 {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.1}")
    }
}

fn draw_top(scene: &Scene, hits: &[Vec3], other_hits: &[Vec3]) -> String {
    let mut cloud = vec![
        (0.0, 0.0),
        (scene.mount.translation_m.x, scene.mount.translation_m.y),
    ];
    for body in &scene.bodies {
        cloud.push((body.min.x, body.min.y));
        cloud.push((body.max.x, body.max.y));
    }
    for hit in hits.iter().chain(other_hits) {
        cloud.push((hit.x, hit.y));
    }
    let plane = fit_plane(&cloud, 120.0, 168.0, 860.0, 430.0);
    let map = |point: Vec3| plane.screen(point.x, point.y);
    let mut geometry = Vec::new();
    let mut body = svg_header(
        "World top-down, metres. Origin is the synthetic base, not the sensor. Equal scale.",
    );
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">The named dot is the synthetic sensor. It is not the base origin and not a 0.175 m installed arm.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"13\">Rays cross the plank footprint in this view. The side view shows those rays pass above the plank.</text>");
    body.push_str(&edges(scene, map, &mut geometry));
    body.push_str(&rays(
        scene.mount.translation_m,
        other_hits,
        map,
        "#1f4e79",
        &mut geometry,
    ));
    body.push_str(&rays(
        scene.mount.translation_m,
        hits,
        map,
        "#c0392b",
        &mut geometry,
    ));
    body.push_str(&hit_marks(hits, map));
    let (axes, keepout) = plane_axes(
        &plane,
        ("world x", "world y", "x", "y"),
        &cloud,
        0.0,
        &geometry,
    );
    body.push_str(&axes);
    geometry.extend(keepout);
    body.push_str(&name_labels(scene, map, &geometry));
    body.push_str(&swatches(
        &[
            ("#c0392b", "named-mount yaw"),
            ("#1f4e79", "yaw 0, same translation"),
        ],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn draw_side(scene: &Scene, hits: &[Vec3]) -> String {
    let mut cloud = vec![
        (0.0, 0.0),
        (scene.mount.translation_m.x, scene.mount.translation_m.z),
    ];
    for body in &scene.bodies {
        cloud.push((body.min.x, body.min.z));
        cloud.push((body.max.x, body.max.z));
    }
    for hit in hits {
        cloud.push((hit.x, hit.z));
    }
    let plane = fit_plane(&cloud, 120.0, 168.0, 860.0, 430.0);
    let map = |point: Vec3| plane.screen(point.x, point.z);
    let mut geometry = Vec::new();
    let mut body = svg_header(
        "World side view, x and z, equal metres. The horizontal beam misses the low plank and meets the pillar.",
    );
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">The beam height is the sensor z. A hit on this picture is a physical intersection, not a footprint overlap.</text>");
    body.push_str(&edges(scene, map, &mut geometry));
    body.push_str(&rays(
        scene.mount.translation_m,
        hits,
        map,
        "#c0392b",
        &mut geometry,
    ));
    body.push_str(&hit_marks(hits, map));
    let (axes, keepout) = plane_axes(
        &plane,
        ("world x", "world z", "x", "z"),
        &cloud,
        0.0,
        &geometry,
    );
    body.push_str(&axes);
    geometry.extend(keepout);
    body.push_str(&name_labels(scene, map, &geometry));
    body.push_str(&swatches(
        &[("#c0392b", "named-mount ray at the beam height")],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn draw_scan(scene: &Scene, profile: &LidarProfile, points: &[(f64, f64)]) -> String {
    let mut cloud = vec![(0.0, 0.0), (0.25, 0.25), (-0.25, -0.25)];
    cloud.extend(points.iter().copied());
    let plane = fit_plane(&cloud, 120.0, 168.0, 860.0, 430.0);
    let mut body = svg_header(&format!(
        "Scan frame of {}. Origin is the sensor, not the chassis. Front {}. Equal scale.",
        scene.mount.name, profile.id
    ));
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">Dashed circle: radius 250 mm around the sensor origin. It is not a diameter and not a measured chassis.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"13\">It does not certify clearance. Points are frame_sample returns, not world coordinates and not a measured obstacle radius.</text>");
    let (axes, _) = plane_axes(
        &plane,
        ("scan x, sensor forward", "scan y, sensor left", "x", "y"),
        &cloud,
        0.25,
        &[],
    );
    body.push_str(&axes);
    let origin = plane.screen(0.0, 0.0);
    let radius = 0.25 * plane.scale;
    body.push_str(&format!(
        "<circle data-radius-m=\"0.25\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"{radius:.1}\" fill=\"none\" stroke=\"#888888\" stroke-dasharray=\"4 3\"/>",
        origin.0, origin.1
    ));
    for point in points {
        let placed = plane.screen(point.0, point.1);
        body.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.4\" fill=\"#c0392b\"/>",
            placed.0, placed.1
        ));
    }
    body.push_str(&swatches(
        &[
            ("#c0392b", "scan-frame return"),
            ("#888888", "radius 250 mm, sensor origin"),
        ],
        676.0,
    ));
    body.push_str("</svg>");
    body
}

fn svg_header(title: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1040\" height=\"760\" viewBox=\"0 0 1040 760\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><text x=\"24\" y=\"32\" font-size=\"16\">{title}</text><text x=\"24\" y=\"54\" font-size=\"12\">Numeric ticks are metres. This is not a photograph and not a chassis clearance.</text><text x=\"24\" y=\"74\" font-size=\"12\">World and base coincide only in this synthetic fixture. The installed lever arm stays unknown.</text>"
    )
}

struct TickMark {
    axis: &'static str,
    metres: f64,
    at: (f64, f64),
    offset: (f64, f64),
}

fn world_axis(
    map: &impl Fn(Vec3) -> (f64, f64),
    axis: &'static str,
    name: &str,
    ticks: &[f64],
    at: impl Fn(f64) -> Vec3,
    offsets: ((f64, f64), (f64, f64)),
    pending: &mut Vec<TickMark>,
) -> (String, Vec<ScreenSeg>) {
    let (tick_offset, name_offset) = offsets;
    let start = map(at(ticks[0]));
    let end = map(at(*ticks.last().unwrap_or(&ticks[0])));
    let name_at = (end.0 + name_offset.0, end.1 + name_offset.1);
    let body = format!(
        "<line data-axis-line=\"{axis}\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{name}</text>",
        start.0, start.1, end.0, end.1, name_at.0, name_at.1
    );
    for metres in ticks {
        pending.push(TickMark {
            axis,
            metres: *metres,
            at: map(at(*metres)),
            offset: tick_offset,
        });
    }
    (body, vec![(start, end), (name_at, name_at)])
}

fn place_ticks(pending: &[TickMark], geometry: &[ScreenSeg]) -> (String, Vec<ScreenSeg>) {
    let mut body = String::new();
    let mut keepout = Vec::new();
    for tick in pending {
        let (svg, label) = emit_tick(
            tick.axis,
            tick.metres,
            tick.at,
            tick.offset,
            geometry,
            tick.at,
            0.0,
        );
        body.push_str(&svg);
        if let Some(label) = label {
            keepout.push((label, label));
        }
    }
    (body, keepout)
}

fn plane_axes(
    plane: &Plane,
    names: (&str, &str, &str, &str),
    cloud: &[(f64, f64)],
    protect_m: f64,
    geometry: &[ScreenSeg],
) -> (String, Vec<ScreenSeg>) {
    let (x_name, y_name, x_attr, y_attr) = names;
    let (min_u, max_u, min_v, max_v) = extents(cloud);
    let u_ticks = tick_values(min_u, max_u);
    let v_ticks = tick_values(min_v, max_v);
    let x0 = plane.screen(u_ticks[0], 0.0);
    let x1 = plane.screen(*u_ticks.last().unwrap_or(&u_ticks[0]), 0.0);
    let y0 = plane.screen(0.0, v_ticks[0]);
    let y1 = plane.screen(0.0, *v_ticks.last().unwrap_or(&v_ticks[0]));
    let mut body = format!(
        "<line data-axis-line=\"{x_attr}\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/><line data-axis-line=\"{y_attr}\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/>",
        x0.0, x0.1, x1.0, x1.1, y0.0, y0.1, y1.0, y1.1
    );
    let origin = plane.screen(0.0, 0.0);
    let protect = protect_m * plane.scale;
    let mut keepout = vec![(x0, x1), (y0, y1)];
    for metres in &u_ticks {
        let point = plane.screen(*metres, 0.0);
        let (svg, label) = emit_tick(
            x_attr,
            *metres,
            point,
            (0.0, -16.0),
            geometry,
            origin,
            protect,
        );
        body.push_str(&svg);
        if let Some(label) = label {
            keepout.push((label, label));
        }
    }
    for metres in &v_ticks {
        let point = plane.screen(0.0, *metres);
        let (svg, label) = emit_tick(
            y_attr,
            *metres,
            point,
            (-16.0, 0.0),
            geometry,
            origin,
            protect,
        );
        body.push_str(&svg);
        if let Some(label) = label {
            keepout.push((label, label));
        }
    }
    let name_px = 6.6 * x_name.chars().count() as f64;
    let x_past_end = x1.0 + 12.0 + name_px > 1024.0;
    let x_label = if x_past_end {
        (1020.0, x1.1 + 16.0)
    } else {
        (x1.0 + 12.0, x1.1 + 16.0)
    };
    let x_anchor = if x_past_end { "end" } else { "start" };
    let y_label = (y1.0 + 10.0, y1.1 + 4.0);
    body.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"{x_anchor}\">{x_name}</text><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{y_name}</text>",
        x_label.0, x_label.1, y_label.0, y_label.1
    ));
    keepout.push((x_label, x_label));
    keepout.push((y_label, y_label));
    (body, keepout)
}

fn emit_tick(
    axis: &str,
    metres: f64,
    at: (f64, f64),
    mut label_offset: (f64, f64),
    geometry: &[ScreenSeg],
    origin: (f64, f64),
    protect_px: f64,
) -> (String, Option<(f64, f64)>) {
    let zero = metres.abs() < 1e-6;
    let draw_number = !zero || axis == "x";
    if zero && draw_number {
        label_offset = (-12.0, -12.0);
    }
    let span = label_offset.0.hypot(label_offset.1).max(1.0);
    let dir = (label_offset.0 / span, label_offset.1 / span);
    let stroke = (at.0 + dir.0 * 8.0, at.1 + dir.1 * 8.0);
    let preferred = settle_label(at, label_offset, dir, geometry, origin, protect_px);
    let flipped = settle_label(
        at,
        (-label_offset.0, -label_offset.1),
        (-dir.0, -dir.1),
        geometry,
        origin,
        protect_px,
    );
    let label = if !geometry.is_empty()
        && gap_to_geometry(flipped, geometry) >= 12.0
        && point_gap(flipped, at) + 4.0 < point_gap(preferred, at)
    {
        flipped
    } else {
        preferred
    };
    let metres_s = metres_attr(metres);
    let mut body = format!(
        "<g data-axis-tick=\"{axis}\" data-metres=\"{metres_s}\"><line data-tick-stroke=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.2\"/>",
        at.0, at.1, stroke.0, stroke.1
    );
    if draw_number {
        if point_gap(label, at) > 22.0 {
            body.push_str(&format!(
                "<line data-tick-leader=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#888888\" stroke-width=\"0.7\"/>",
                at.0, at.1, label.0, label.1
            ));
        }
        let anchor = if label.0 < at.0 - 4.0 {
            "end"
        } else {
            "middle"
        };
        body.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" text-anchor=\"{anchor}\">{metres_s}</text>",
            label.0, label.1
        ));
    }
    body.push_str("</g>");
    (body, draw_number.then_some(label))
}

fn settle_label(
    at: (f64, f64),
    label_offset: (f64, f64),
    dir: (f64, f64),
    geometry: &[ScreenSeg],
    origin: (f64, f64),
    protect_px: f64,
) -> (f64, f64) {
    let mut label = (at.0 + label_offset.0, at.1 + label_offset.1);
    if protect_px > 0.0 {
        let dx = label.0 - origin.0;
        let dy = label.1 - origin.1;
        let distance = dx.hypot(dy);
        let need = protect_px + 18.0;
        if distance < need && distance > 0.5 {
            let scale = need / distance;
            label = (origin.0 + dx * scale, origin.1 + dy * scale);
        }
    }
    for _ in 0..6 {
        if gap_to_geometry(label, geometry) >= 12.0 {
            break;
        }
        label.0 += dir.0 * 12.0;
        label.1 += dir.1 * 12.0;
    }
    label
}

fn point_gap(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn swatches(items: &[(&str, &str)], y: f64) -> String {
    let mut body = String::new();
    let mut x = 24.0;
    for (color, label) in items {
        body.push_str(&format!(
            "<rect data-legend=\"swatch\" x=\"{x:.1}\" y=\"{:.1}\" width=\"14\" height=\"10\" fill=\"{color}\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{label}</text>",
            y - 9.0,
            x + 20.0,
            y
        ));
        x += 240.0;
    }
    body
}

fn name_labels(scene: &Scene, map: impl Fn(Vec3) -> (f64, f64), geometry: &[ScreenSeg]) -> String {
    let mut body = String::new();
    for aabb in &scene.bodies {
        let points: Vec<(f64, f64)> = corners(aabb).into_iter().map(&map).collect();
        let (min_x, max_x, min_y, max_y) = extents(&points);
        let anchor = map(Vec3 {
            x: (aabb.min.x + aabb.max.x) / 2.0,
            y: (aabb.min.y + aabb.max.y) / 2.0,
            z: (aabb.min.z + aabb.max.z) / 2.0,
        });
        let half = 3.7 * aabb.name.chars().count() as f64;
        let mid_x = (min_x + max_x) / 2.0;
        let mid_y = (min_y + max_y) / 2.0;
        let candidates = [
            (mid_x, min_y - 20.0),
            (mid_x, max_y + 18.0),
            (min_x - half - 14.0, mid_y),
            (max_x + half + 14.0, mid_y),
        ];
        let placed = clearest(candidates, aabb.name.chars().count(), geometry);
        body.push_str(&leader(placed, anchor));
        body.push_str(&format!(
            "<text data-label=\"{}\" x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"middle\">{}</text>",
            aabb.name, placed.0, placed.1, aabb.name
        ));
    }
    let sensor = map(scene.mount.translation_m);
    let half = 3.7 * scene.mount.name.chars().count() as f64;
    let candidates = [
        (sensor.0, sensor.1 - 28.0),
        (sensor.0, sensor.1 + 24.0),
        (sensor.0 - half - 16.0, sensor.1),
        (sensor.0 + half + 16.0, sensor.1),
    ];
    let placed = clearest(candidates, scene.mount.name.chars().count(), geometry);
    body.push_str(&format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" fill=\"#222222\"/>",
        sensor.0, sensor.1
    ));
    body.push_str(&leader(placed, sensor));
    body.push_str(&format!(
        "<text data-label=\"{}\" x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"middle\">{}</text>",
        scene.mount.name, placed.0, placed.1, scene.mount.name
    ));
    body
}

fn clearest(candidates: [(f64, f64); 4], chars: usize, geometry: &[ScreenSeg]) -> (f64, f64) {
    let mut best = nudge_text(candidates[0], chars, geometry);
    let mut best_gap = text_gap(best, chars, geometry);
    for candidate in candidates.into_iter().skip(1) {
        let placed = nudge_text(candidate, chars, geometry);
        let gap = text_gap(placed, chars, geometry);
        if gap > best_gap {
            best = placed;
            best_gap = gap;
        }
    }
    best
}

fn nudge_text(mut point: (f64, f64), chars: usize, geometry: &[ScreenSeg]) -> (f64, f64) {
    for _ in 0..12 {
        if text_gap(point, chars, geometry) >= 8.0 {
            break;
        }
        let away = nearest_push(point, geometry)
            .map(|(_, away)| away)
            .unwrap_or((0.0, -1.0));
        point.0 += away.0 * 12.0;
        point.1 += away.1 * 12.0;
    }
    (point.0.clamp(18.0, 1000.0), point.1.clamp(140.0, 640.0))
}

fn text_gap(anchor: (f64, f64), chars: usize, geometry: &[ScreenSeg]) -> f64 {
    if geometry.is_empty() {
        return f64::INFINITY;
    }
    let half = 3.7 * chars as f64;
    let rect = (
        anchor.0 - half,
        anchor.1 - 12.0,
        anchor.0 + half,
        anchor.1 + 3.0,
    );
    let mut gap = f64::INFINITY;
    let steps = 8;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let samples = [
            (rect.0 + (rect.2 - rect.0) * t, rect.1),
            (rect.0 + (rect.2 - rect.0) * t, rect.3),
            (rect.0, rect.1 + (rect.3 - rect.1) * t),
            (rect.2, rect.1 + (rect.3 - rect.1) * t),
        ];
        for sample in samples {
            gap = gap.min(gap_to_geometry(sample, geometry));
        }
    }
    gap
}

fn nearest_push(point: (f64, f64), geometry: &[ScreenSeg]) -> Option<(f64, (f64, f64))> {
    let mut best_gap = f64::INFINITY;
    let mut best_away = (0.0, -1.0);
    for segment in geometry {
        let closest = closest_point(point, *segment);
        let dx = point.0 - closest.0;
        let dy = point.1 - closest.1;
        let gap = dx.hypot(dy);
        if gap < best_gap {
            best_gap = gap;
            best_away = if gap < 0.5 {
                (0.0, -1.0)
            } else {
                (dx / gap, dy / gap)
            };
        }
    }
    if best_gap.is_infinite() {
        None
    } else {
        Some((best_gap, best_away))
    }
}

fn gap_to_geometry(point: (f64, f64), geometry: &[ScreenSeg]) -> f64 {
    nearest_push(point, geometry)
        .map(|(gap, _)| gap)
        .unwrap_or(f64::INFINITY)
}

fn closest_point(point: (f64, f64), segment: ((f64, f64), (f64, f64))) -> (f64, f64) {
    let (start, end) = segment;
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length_sq = dx * dx + dy * dy;
    if length_sq < 1e-9 {
        return start;
    }
    let t = ((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);
    (start.0 + dx * t, start.1 + dy * t)
}

fn leader(from: (f64, f64), to: (f64, f64)) -> String {
    format!(
        "<line data-leader=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#888888\" stroke-width=\"0.8\"/>",
        from.0, from.1, to.0, to.1
    )
}

fn dimension_note(scene: &Scene) -> String {
    let mut body = String::from("<text x=\"24\" y=\"712\" font-size=\"12\">");
    for aabb in &scene.bodies {
        body.push_str(&format!(
            "{} x {:.2}–{:.2} m, y {:.2}–{:.2} m, z {:.2}–{:.2} m. ",
            aabb.name, aabb.min.x, aabb.max.x, aabb.min.y, aabb.max.y, aabb.min.z, aabb.max.z
        ));
    }
    body.push_str("</text>");
    body
}

fn hit_marks(hits: &[Vec3], map: impl Fn(Vec3) -> (f64, f64)) -> String {
    let mut body = String::new();
    for hit in hits {
        let placed = map(*hit);
        body.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.4\" fill=\"#c0392b\"/>",
            placed.0, placed.1
        ));
    }
    body
}

fn edges(scene: &Scene, map: impl Fn(Vec3) -> (f64, f64), geometry: &mut Vec<ScreenSeg>) -> String {
    let mut body = String::new();
    for aabb in &scene.bodies {
        let corners = corners(aabb);
        for (start, end) in BOX_EDGES {
            let a = map(corners[start]);
            let b = map(corners[end]);
            geometry.push((a, b));
            body.push_str(&format!(
                "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#333333\" stroke-width=\"1.2\"/>",
                a.0, a.1, b.0, b.1
            ));
        }
    }
    body
}

fn rays(
    origin: Vec3,
    hits: &[Vec3],
    map: impl Fn(Vec3) -> (f64, f64),
    color: &str,
    geometry: &mut Vec<ScreenSeg>,
) -> String {
    let start = map(origin);
    let mut body = String::new();
    for hit in hits {
        let end = map(*hit);
        geometry.push((start, end));
        body.push_str(&format!(
            "<line data-geom=\"1\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{color}\" stroke-width=\"1.1\"/>",
            start.0, start.1, end.0, end.1
        ));
    }
    body
}

fn corners(aabb: &Aabb) -> [Vec3; 8] {
    let (a, b) = (aabb.min, aabb.max);
    [
        Vec3 {
            x: a.x,
            y: a.y,
            z: a.z,
        },
        Vec3 {
            x: b.x,
            y: a.y,
            z: a.z,
        },
        Vec3 {
            x: b.x,
            y: b.y,
            z: a.z,
        },
        Vec3 {
            x: a.x,
            y: b.y,
            z: a.z,
        },
        Vec3 {
            x: a.x,
            y: a.y,
            z: b.z,
        },
        Vec3 {
            x: b.x,
            y: a.y,
            z: b.z,
        },
        Vec3 {
            x: b.x,
            y: b.y,
            z: b.z,
        },
        Vec3 {
            x: a.x,
            y: b.y,
            z: b.z,
        },
    ]
}

const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

#[cfg(test)]
mod tests {
    use super::{
        Aabb, Cast, Mount, Scene, SceneError, ScreenSeg, Vec3, cast_bearing, demonstration,
        inverse_bearing, packet_revolution, project_oblique, render_views, sensor_samples,
    };
    use crate::kind::Kind;
    use crate::navpost::{ClockLabel, PostClock, PostOutcome, UnusableKind, post_samples};
    use crate::profile::{LIDAR_X_M, lidar_profiles, wrap_signed_deg};
    use domain::{Id, Mm, Point};
    use navigation::{
        Cue, Follower, Intent, SampleHistory, SeenBody, SeenPerson, Sight, plan_follow,
    };

    fn v(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    fn mount(t: Vec3, yaw: f64, pitch: f64) -> Mount {
        Mount {
            name: "synthetic-lidar",
            translation_m: t,
            yaw_deg: yaw,
            pitch_down_deg: pitch,
            provenance: "synthetic fixture, not an installed lever arm",
        }
    }

    fn scene(mount: Mount, bodies: Vec<Aabb>) -> Scene {
        Scene {
            mount,
            bodies,
            max_range_m: 20.0,
        }
    }

    fn box_at(name: &'static str, min: Vec3, max: Vec3) -> Aabb {
        Aabb { name, min, max }
    }

    fn measured_front() -> f64 {
        let profile = lidar_profiles()[0];
        assert_eq!(profile.id, "measured-front");
        assert_eq!(profile.kind, Kind::Measured);
        profile.front_raw_deg
    }

    #[test]
    fn a_forward_wall_is_the_face_distance_and_not_the_configured_lever() {
        let fixture = scene(
            mount(v(0.40, 0.10, 0.50), 0.0, 0.0),
            vec![box_at("wall", v(2.40, -1.0, 0.0), v(2.50, 1.0, 1.0))],
        );
        assert_ne!(fixture.mount.translation_m.x, 0.0);
        assert!((fixture.mount.translation_m.x - LIDAR_X_M).abs() > 0.01);
        let Cast::Hit { range_m, point } = cast_bearing(&fixture, 0.0).unwrap() else {
            panic!("miss");
        };
        assert!((range_m - 2.0).abs() < 1e-9, "{range_m}");
        assert!((point.x - 2.40).abs() < 1e-9);
        assert!((point.y - 0.10).abs() < 1e-9);
        assert!((point.z - 0.50).abs() < 1e-9);
        assert!((inverse_bearing(&fixture.mount, point).unwrap() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn yaw_turns_forward_to_world_left_and_sensor_left_to_world_back() {
        let forward = scene(
            mount(v(0.30, 0.20, 0.50), 90.0, 0.0),
            vec![box_at("side-wall", v(-1.0, 1.20, 0.0), v(1.0, 1.30, 1.0))],
        );
        let Cast::Hit { range_m, point } = cast_bearing(&forward, 0.0).unwrap() else {
            panic!("miss");
        };
        assert!((range_m - 1.0).abs() < 1e-9, "{range_m}");
        assert!((point.x - 0.30).abs() < 1e-9, "{}", point.x);
        assert!((point.y - 1.20).abs() < 1e-9);
        assert!((point.z - 0.50).abs() < 1e-9, "{}", point.z);
        let left = scene(
            mount(v(0.30, 0.20, 0.50), 90.0, 0.0),
            vec![box_at("rear", v(-0.80, -1.0, 0.0), v(-0.70, 1.0, 1.0))],
        );
        let Cast::Hit { range_m, point } = cast_bearing(&left, 90.0).unwrap() else {
            panic!("sensor left missed world -x");
        };
        assert!((range_m - 1.0).abs() < 1e-9, "{range_m}");
        assert!(point.x < 0.0);
        assert!((inverse_bearing(&left.mount, point).unwrap() - 90.0).abs() < 1e-6);
    }

    #[test]
    fn a_pitched_ray_matches_an_independent_matrix_and_round_trips() {
        let yaw = 20.0_f64;
        let pitch = 10.0_f64;
        let bearing = 15.0_f64;
        let origin = v(0.40, -0.12, 0.55);
        let fixture = scene(
            mount(origin, yaw, pitch),
            vec![box_at(
                "far-wall",
                v(2.40, -20.0, -20.0),
                v(2.50, 20.0, 20.0),
            )],
        );
        let Cast::Hit { range_m, point } = cast_bearing(&fixture, bearing).unwrap() else {
            panic!("miss");
        };
        assert!((point.x - 2.40).abs() < 1e-9, "{}", point.x);
        assert!(point.z < origin.z, "pitch down raised the hit: {}", point.z);
        assert!(
            range_m > 2.0,
            "slant range was the horizontal run: {range_m}"
        );
        let back = inverse_bearing(&fixture.mount, point).unwrap();
        assert!((wrap_signed_deg(back - bearing)).abs() < 1e-6, "{back}");
        let flat = project_oblique(v(1.0, 0.0, 0.0));
        let raised = project_oblique(v(1.0, 0.0, 1.0));
        assert_ne!(flat, raised);
        let views = render_views(&fixture, &[0.0, 15.0], &lidar_profiles()[0], 2_500).unwrap();
        assert!(views.oblique_svg.contains("<line "));
        assert!(views.top_svg.contains("not the sensor"));
        assert!(views.side_svg.contains("<line "));
        assert!(views.scan_svg.contains("Scan frame"));
        assert!(views.mount_csv.contains("installed_lever_arm,unknown"));
        assert!(views.rays_csv.contains("hit") || views.rays_csv.contains("miss"));
        assert!(!views.mount_csv.contains("0.1750"));
    }

    #[test]
    fn a_45_degree_pitch_drops_one_metre_for_one_metre_of_run() {
        let fixture = scene(
            mount(v(0.0, 0.0, 1.0), 0.0, 45.0),
            vec![box_at("wall", v(1.0, -2.0, -2.0), v(1.2, 2.0, 3.0))],
        );
        let Cast::Hit { range_m, point } = cast_bearing(&fixture, 0.0).unwrap() else {
            panic!("miss");
        };
        let slant = 2.0_f64.sqrt();
        assert!((range_m - slant).abs() < 1e-9, "{range_m}");
        assert!((range_m - 1.0).abs() > 0.1, "horizontal run was reported");
        assert!((point.x - 1.0).abs() < 1e-9, "{}", point.x);
        assert!(point.y.abs() < 1e-9, "{}", point.y);
        assert!(point.z.abs() < 1e-9, "{}", point.z);
        assert!(inverse_bearing(&fixture.mount, point).unwrap().abs() < 1e-6);
    }

    #[test]
    fn the_near_box_hides_the_far_box_until_it_is_removed() {
        let near = box_at("near", v(1.00, -0.2, 0.0), v(1.10, 0.2, 1.0));
        let far = box_at("far", v(2.00, -0.2, 0.0), v(2.20, 0.2, 1.0));
        let origin = mount(v(0.40, 0.0, 0.50), 0.0, 0.0);
        let both = scene(origin, vec![far, near]);
        let Cast::Hit { range_m, .. } = cast_bearing(&both, 0.0).unwrap() else {
            panic!("miss");
        };
        assert!((range_m - 0.60).abs() < 1e-9, "{range_m}");
        assert!((range_m - 0.65).abs() > 0.01, "box center was used");
        let only_far = scene(origin, vec![far]);
        let Cast::Hit { range_m, .. } = cast_bearing(&only_far, 0.0).unwrap() else {
            panic!("far miss");
        };
        assert!((range_m - 1.60).abs() < 1e-9, "{range_m}");
    }

    #[test]
    fn a_low_box_misses_the_beam_until_its_height_reaches_it() {
        let origin = mount(v(0.40, 0.0, 0.50), 0.0, 0.0);
        let low = scene(
            origin,
            vec![box_at("low", v(1.40, -0.2, 0.0), v(1.60, 0.2, 0.20))],
        );
        assert!(matches!(cast_bearing(&low, 0.0).unwrap(), Cast::Miss));
        let tall = scene(
            origin,
            vec![box_at("tall", v(1.40, -0.2, 0.0), v(1.60, 0.2, 1.00))],
        );
        let Cast::Hit { range_m, .. } = cast_bearing(&tall, 0.0).unwrap() else {
            panic!("tall miss");
        };
        assert!((range_m - 1.0).abs() < 1e-9, "{range_m}");
        let raised = scene(
            origin,
            vec![box_at("raised", v(1.40, -0.2, 0.0), v(1.60, 0.2, 0.80))],
        );
        assert!(matches!(
            cast_bearing(&raised, 0.0).unwrap(),
            Cast::Hit { .. }
        ));
        assert!(matches!(
            cast_bearing(&scene(origin, vec![]), 0.0).unwrap(),
            Cast::Miss
        ));
    }

    #[test]
    fn bad_geometry_and_a_bad_clock_do_not_become_a_view() {
        let mut bad = scene(mount(v(0.40, 0.10, f64::NAN), 0.0, 0.0), vec![]);
        assert_eq!(cast_bearing(&bad, 0.0), Err(SceneError::NonFinite));
        bad.mount.translation_m.z = 0.50;
        bad.max_range_m = -1.0;
        assert_eq!(cast_bearing(&bad, 0.0), Err(SceneError::NegativeRange));
        bad.max_range_m = 20.0;
        assert_eq!(cast_bearing(&bad, f64::NAN), Err(SceneError::NonFinite));
        let fixture = scene(
            mount(v(0.40, 0.10, 0.50), 0.0, 0.0),
            vec![box_at("wall", v(2.40, -1.0, 0.0), v(2.50, 1.0, 1.0))],
        );
        let measured = sensor_samples(&fixture, &[0.0], measured_front()).unwrap();
        let configured =
            sensor_samples(&fixture, &[0.0], lidar_profiles()[1].front_raw_deg).unwrap();
        let profile_m = lidar_profiles()[0];
        let profile_c = lidar_profiles()[1];
        let decoded_m =
            crate::navpost::frame_sample(&profile_m, measured[0].raw_deg, measured[0].range_m);
        let decoded_c =
            crate::navpost::frame_sample(&profile_c, configured[0].raw_deg, configured[0].range_m);
        let crossed =
            crate::navpost::frame_sample(&profile_c, measured[0].raw_deg, measured[0].range_m);
        assert_eq!(decoded_m, decoded_c);
        assert_ne!(decoded_m, crossed);
        let clock = PostClock {
            captured_ms: Some(2_500),
            delivered_ms: 2_500,
            decision_ms: 2_500,
            label: ClockLabel::InjectedReplay,
        };
        let (_history, fresh) = post_samples(&profile_m, &measured, SampleHistory::new(), clock);
        assert!(matches!(
            fresh.outcome,
            PostOutcome::Acted {
                intent: Intent::Lost
            }
        ));
        assert!(!fresh.sent);
        assert!(fresh.command_text.is_empty());
        assert_eq!(fresh.people, 0);
        let stale = PostClock {
            decision_ms: 3_501,
            ..clock
        };
        let (_history, stale) = post_samples(&profile_m, &measured, SampleHistory::new(), stale);
        assert_eq!(
            stale.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Stale
            }
        );
        let missing = PostClock {
            captured_ms: None,
            ..clock
        };
        let (history, _first) = post_samples(&profile_m, &measured, SampleHistory::new(), clock);
        let (_history, missing) =
            post_samples(&profile_m, &measured, SampleHistory::new(), missing);
        assert_eq!(
            missing.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Missing
            }
        );
        let older = PostClock {
            captured_ms: Some(1_000),
            delivered_ms: 3_000,
            decision_ms: 3_000,
            label: ClockLabel::InjectedReplay,
        };
        let (_history, reversed) = post_samples(&profile_m, &measured, history, older);
        assert_eq!(
            reversed.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Reversed
            }
        );
    }

    #[test]
    fn the_scan_view_is_not_the_world_point_and_it_changes_the_step() {
        let fixture = scene(
            mount(v(0.40, 0.10, 0.50), 0.0, 0.0),
            vec![box_at("close", v(0.55, -0.05, 0.0), v(0.70, 0.25, 1.0))],
        );
        let samples = sensor_samples(&fixture, &[0.0], measured_front()).unwrap();
        let profile = lidar_profiles()[0];
        let clock = PostClock {
            captured_ms: Some(2_500),
            delivered_ms: 2_500,
            decision_ms: 2_500,
            label: ClockLabel::InjectedReplay,
        };
        let (_history, post) = post_samples(&profile, &samples, SampleHistory::new(), clock);
        assert_eq!(post.view_points, vec![(150, 0)]);
        assert_ne!(post.view_points, vec![(550, 100)]);
        let bodies: Vec<SeenBody> = post
            .view_points
            .iter()
            .map(|(x, y)| SeenBody {
                center: Point::new(Mm::new(*x), Mm::new(*y)),
                radius_mm: 0,
            })
            .collect();
        let person = SeenPerson {
            id: Id::new(2).expect("id"),
            at: Point::new(Mm::new(3_000), Mm::new(0)),
        };
        let robot = Point::new(Mm::new(0), Mm::new(0));
        let with = Sight {
            robot,
            people: std::slice::from_ref(&person),
            bodies: &bodies,
        };
        let without = Sight {
            robot,
            people: std::slice::from_ref(&person),
            bodies: &[],
        };
        let follower = Follower::follow(person.id);
        let blocked = plan_follow(&follower, &with, Cue::None).1;
        let open = plan_follow(&follower, &without, Cue::None).1;
        match open {
            Intent::Move(to) => {
                assert_eq!(to.y().get(), 0);
                assert!(to.x().get() > 0);
            }
            other => panic!("{other:?}"),
        }
        match blocked {
            Intent::Blocked | Intent::Stuck => {}
            Intent::Move(to) => assert_ne!(to.y().get(), 0, "{to:?}"),
            other => panic!("{other:?}"),
        }
        assert_ne!(blocked, open);
    }

    #[test]
    fn a_parsed_packet_blocks_the_step_until_the_body_is_removed() {
        let front = measured_front();
        let profile = lidar_profiles()[0];
        let clock = PostClock {
            captured_ms: Some(2_500),
            delivered_ms: 2_500,
            decision_ms: 2_500,
            label: ClockLabel::InjectedReplay,
        };
        let blocked_scene = scene(
            mount(v(0.40, 0.10, 0.50), 0.0, 0.0),
            vec![box_at("close", v(0.55, -0.05, 0.0), v(0.70, 0.25, 1.0))],
        );
        let open_scene = scene(mount(v(0.40, 0.10, 0.50), 0.0, 0.0), vec![]);
        let blocked_post = post_one_packet(&blocked_scene, front, &profile, clock);
        let open_post = post_one_packet(&open_scene, front, &profile, clock);
        assert_eq!(blocked_post.view_points, vec![(150, 0)]);
        assert_ne!(blocked_post.view_points, vec![(550, 100)]);
        assert!(open_post.view_points.is_empty());
        let person = SeenPerson {
            id: Id::new(2).expect("id"),
            at: Point::new(Mm::new(3_000), Mm::new(0)),
        };
        let robot = Point::new(Mm::new(0), Mm::new(0));
        let follower = Follower::follow(person.id);
        let blocked = plan_of(&blocked_post.view_points, &person, robot, follower);
        let open = plan_of(&open_post.view_points, &person, robot, follower);
        match open {
            Intent::Move(to) => {
                assert_eq!(to.y().get(), 0);
                assert!(to.x().get() > 0);
            }
            other => panic!("{other:?}"),
        }
        match blocked {
            Intent::Blocked | Intent::Stuck => {}
            Intent::Move(to) => assert_ne!(to.y().get(), 0, "{to:?}"),
            other => panic!("{other:?}"),
        }
        assert_ne!(blocked, open);
    }

    fn post_one_packet(
        fixture: &Scene,
        front: f64,
        profile: &crate::profile::LidarProfile,
        clock: PostClock,
    ) -> crate::navpost::ScanPost {
        let generated = sensor_samples(fixture, &[0.0], front).unwrap();
        let word = (generated[0].range_m * 4_000.0).round() as u16;
        let bytes = crate::lidar::encode_packet(0, &[(generated[0].raw_deg, word, 0)]).unwrap();
        let parsed = crate::lidar::parse_lidar(&bytes);
        assert!(parsed.rejects.is_empty());
        assert_eq!(parsed.samples.len(), 1);
        if generated[0].range_m > 0.0 {
            assert!((parsed.samples[0].range_m - 0.15).abs() < 1e-9);
        } else {
            assert_eq!(parsed.samples[0].range_m, 0.0);
        }
        let (_history, post) = post_samples(profile, &parsed.samples, SampleHistory::new(), clock);
        assert!(!post.sent);
        post
    }

    fn plan_of(
        points: &[(i32, i32)],
        person: &SeenPerson,
        robot: Point,
        follower: Follower,
    ) -> Intent {
        let bodies: Vec<SeenBody> = points
            .iter()
            .map(|(x, y)| SeenBody {
                center: Point::new(Mm::new(*x), Mm::new(*y)),
                radius_mm: 0,
            })
            .collect();
        plan_follow(
            &follower,
            &Sight {
                robot,
                people: std::slice::from_ref(person),
                bodies: &bodies,
            },
            Cue::None,
        )
        .1
    }

    #[test]
    fn a_packet_carries_the_ray_range_and_not_a_written_hit() {
        let fixture = scene(
            mount(v(0.40, 0.10, 0.50), 0.0, 0.0),
            vec![box_at("wall", v(2.40, -1.0, 0.0), v(2.50, 1.0, 1.0))],
        );
        let front = measured_front();
        let packet = packet_revolution(&fixture, front, 2.0).unwrap();
        let parsed = crate::lidar::parse_lidar(&packet.bytes);
        assert!(parsed.rejects.is_empty());
        assert_eq!(parsed.packets_ok, 5);
        assert_eq!(parsed.samples.len(), 180);
        let sample = parsed
            .samples
            .iter()
            .find(|sample| (sample.raw_deg - 178.0).abs() < 1e-6)
            .expect("raw 178");
        let bearing = wrap_signed_deg(front - 178.0);
        let Cast::Hit { range_m, .. } = cast_bearing(&fixture, bearing).unwrap() else {
            panic!("miss");
        };
        let expected = (range_m * 4_000.0).round() / 4_000.0;
        assert!(
            (sample.range_m - expected).abs() < 1e-12,
            "{}",
            sample.range_m
        );
        assert!(
            sample.range_m > 2.0 && sample.range_m < 2.05,
            "{}",
            sample.range_m
        );
        let clock = PostClock {
            captured_ms: Some(2_500),
            delivered_ms: 2_500,
            decision_ms: 2_500,
            label: ClockLabel::InjectedReplay,
        };
        let (_history, post) = post_samples(
            &lidar_profiles()[0],
            &parsed.samples,
            SampleHistory::new(),
            clock,
        );
        assert!(matches!(
            post.outcome,
            PostOutcome::Acted {
                intent: Intent::Lost
            }
        ));
        assert!(!post.sent);
        assert_eq!(post.people, 0);
        assert!(!post.view_points.is_empty());
    }

    #[test]
    fn the_oblique_picture_lifts_z_up_the_page() {
        let views = render_views(&demonstration(), &[0.0], &lidar_profiles()[0], 2_500).unwrap();
        let svg = &views.oblique_svg;
        let z_label = svg.find("world z").expect("z axis");
        let line = &svg[..z_label];
        let y1 = line_attr(line, "y1");
        let y2 = line_attr(line, "y2");
        assert!(y2 < y1, "z axis went down the page: {y1} to {y2}");
        assert!(svg.contains("world x") && svg.contains("world y"));
        assert!(views.side_svg.contains("world z"));
        assert!(views.scan_svg.contains("250 mm"));
    }

    #[test]
    fn the_pictures_have_ticks_swatches_and_a_signed_scan_frame() {
        let scene = demonstration();
        let profile = lidar_profiles()[0];
        let packet = packet_revolution(&scene, profile.front_raw_deg, 2.0).unwrap();
        let views = render_views(&scene, &packet.bearings_deg, &profile, 2_500).unwrap();
        for svg in [
            &views.oblique_svg,
            &views.top_svg,
            &views.side_svg,
            &views.scan_svg,
        ] {
            assert!(svg.contains("data-axis-tick"), "missing ticks");
            assert!(svg.contains("data-legend=\"swatch\""), "missing swatch");
        }
        assert!(views.top_svg.contains("data-legend=\"swatch\""));
        assert!(views.oblique_svg.matches("data-legend=\"swatch\"").count() >= 2);
        let scan = &views.scan_svg;
        assert!(scan.contains("radius 250 mm"));
        assert!(scan.contains("not a diameter"));
        assert!(scan.contains("data-radius-m=\"0.25\""));
        let y_ticks = tick_metres(scan, "y");
        assert!(
            y_ticks.iter().any(|metres| *metres <= -0.398),
            "scan y does not cover the returns: {y_ticks:?}"
        );
        assert!(
            y_ticks.iter().any(|metres| *metres >= 0.25),
            "scan y does not cover the radius: {y_ticks:?}"
        );
        let marks = anchored_ticks(scan);
        let x0 = tick_mark(&marks, "x", 0.0);
        let x1 = tick_mark(&marks, "x", 1.0);
        let y0 = tick_mark(&marks, "y", 0.0);
        let y_half = tick_mark(&marks, "y", -0.5);
        let dx = dist(x0, x1);
        let dy = dist(y0, y_half) * 2.0;
        assert!((dx - dy).abs() < 1.5, "unequal scale {dx} {dy}");
        let radius_px = circle_radius(scan);
        assert!(
            (radius_px - 0.25 * dx).abs() < 1.5,
            "radius px {radius_px} scale {dx}"
        );
        let scan_faults = tick_anchor_faults(scan, radius_px);
        assert!(scan_faults.is_empty(), "scan ticks: {scan_faults:?}");
        for svg in [&views.top_svg, &views.side_svg, &views.oblique_svg] {
            let faults = tick_anchor_faults(svg, 0.0);
            assert!(faults.is_empty(), "ticks: {faults:?}");
        }
        for svg in [&views.top_svg, &views.side_svg, &views.oblique_svg] {
            for name in ["pillar", "low-plank", "synthetic-lidar"] {
                let label = text_xy(svg, name);
                let gap = geom_lines(svg)
                    .into_iter()
                    .map(|segment| dist_seg(label, segment))
                    .fold(f64::MAX, f64::min);
                assert!(gap >= 8.0, "{name} is {gap} px from geometry");
            }
        }
    }

    fn tick_metres(svg: &str, axis: &str) -> Vec<f64> {
        let key = format!("data-axis-tick=\"{axis}\" data-metres=\"");
        let mut values = Vec::new();
        let mut rest = svg;
        while let Some(at) = rest.find(&key) {
            let start = at + key.len();
            let end = rest[start..].find('"').expect("metres") + start;
            values.push(rest[start..end].parse().expect("tick"));
            rest = &rest[end..];
        }
        values
    }

    #[test]
    fn a_tick_label_lifted_clear_of_its_mark_fails() {
        let ok = sample_ticks(16.0, 200.0, false);
        assert!(
            tick_anchor_faults(&ok, 0.0).is_empty(),
            "{:?}",
            tick_anchor_faults(&ok, 0.0)
        );
        let lifted = sample_ticks(216.0, 200.0, true);
        assert!(
            !tick_anchor_faults(&lifted, 0.0).is_empty(),
            "a 200 px lift still counted as anchored"
        );
        let moved = sample_ticks(16.0, 260.0, false);
        assert!(
            !tick_anchor_faults(&moved, 0.0).is_empty(),
            "a mark off the axis still counted"
        );
    }

    fn sample_ticks(label_lift: f64, mark_y: f64, leader: bool) -> String {
        let leader_tag = if leader {
            format!(
                "<line data-tick-leader=\"1\" x1=\"300\" y1=\"200\" x2=\"300\" y2=\"{label_y}\"/>",
                label_y = 200.0 - label_lift
            )
        } else {
            String::new()
        };
        format!(
            "<svg><line data-axis-line=\"x\" x1=\"0\" y1=\"200\" x2=\"400\" y2=\"200\"/><g data-axis-tick=\"x\" data-metres=\"0\"><line data-tick-stroke=\"1\" x1=\"100\" y1=\"{mark_y}\" x2=\"100\" y2=\"192\"/><text x=\"88\" y=\"188\">0</text></g><g data-axis-tick=\"x\" data-metres=\"1\"><line data-tick-stroke=\"1\" x1=\"300\" y1=\"200\" x2=\"300\" y2=\"192\"/>{leader_tag}<text x=\"300\" y=\"{label_y}\">1</text></g></svg>",
            label_y = 200.0 - label_lift
        )
    }

    struct AnchoredTick {
        axis: String,
        metres: f64,
        mark: (f64, f64),
        label: Option<(f64, f64)>,
        leader: bool,
    }

    fn tick_mark(ticks: &[AnchoredTick], axis: &str, metres: f64) -> (f64, f64) {
        ticks
            .iter()
            .find(|tick| tick.axis == axis && (tick.metres - metres).abs() < 1e-6)
            .unwrap_or_else(|| panic!("mark {axis} {metres}"))
            .mark
    }

    fn tick_anchor_faults(svg: &str, protect_px: f64) -> Vec<String> {
        let mut faults = Vec::new();
        let ticks = anchored_ticks(svg);
        if ticks.is_empty() {
            faults.push("no tick strokes".to_string());
            return faults;
        }
        let lines = axis_lines(svg);
        let origin = ticks
            .iter()
            .find(|tick| tick.metres.abs() <= 1e-6)
            .map(|tick| tick.mark)
            .unwrap_or((0.0, 0.0));
        let mut origin_labels = 0;
        for tick in &ticks {
            let Some((_, segment)) = lines.iter().find(|(axis, _)| axis == &tick.axis) else {
                faults.push(format!("no axis for {}", tick.axis));
                continue;
            };
            let gap = dist_seg(tick.mark, *segment);
            if gap > 1.5 {
                faults.push(format!(
                    "{} {:.1} mark is {gap:.1} px off its axis",
                    tick.axis, tick.metres
                ));
            }
            let Some(label) = tick.label else {
                if tick.metres.abs() > 1e-6 {
                    faults.push(format!("missing number {} {:.1}", tick.axis, tick.metres));
                }
                continue;
            };
            if tick.metres.abs() <= 1e-6 {
                origin_labels += 1;
            }
            let separation = dist(label, tick.mark);
            let near_origin = dist(tick.mark, origin) <= protect_px + 8.0;
            let cap = if near_origin {
                (protect_px + 30.0).max(26.0)
            } else {
                96.0
            };
            if separation > 26.0 && !tick.leader {
                faults.push(format!(
                    "{} {:.1} label is {separation:.0} px away without a leader",
                    tick.axis, tick.metres
                ));
            }
            if separation > cap {
                faults.push(format!(
                    "{} {:.1} label is {separation:.0} px from its mark, cap {cap:.0}",
                    tick.axis, tick.metres
                ));
            }
        }
        if origin_labels != 1 {
            faults.push(format!("origin labels {origin_labels}, want 1"));
        }
        faults
    }

    fn anchored_ticks(svg: &str) -> Vec<AnchoredTick> {
        let mut ticks = Vec::new();
        let mut rest = svg;
        let key = "<g data-axis-tick=\"";
        while let Some(at) = rest.find(key) {
            let open_end = rest[at..].find('>').unwrap() + at;
            let open = &rest[at..open_end];
            let close = rest[open_end..].find("</g>").unwrap() + open_end;
            let inner = &rest[open_end + 1..close];
            let axis = quoted_attr(open, "data-axis-tick");
            let metres = quoted_attr(open, "data-metres").parse().unwrap_or(0.0);
            let stroke_at = inner.find("data-tick-stroke=\"1\"").expect("stroke");
            let stroke_start = inner[..stroke_at].rfind("<line").unwrap();
            let stroke_end = inner[stroke_at..].find('>').unwrap() + stroke_at;
            let stroke = &inner[stroke_start..stroke_end];
            let mark = (attr_in(stroke, "x1"), attr_in(stroke, "y1"));
            let label = inner.find("<text ").map(|text_at| {
                let text_end = inner[text_at..].find('>').unwrap() + text_at;
                let text = &inner[text_at..text_end];
                (attr_in(text, "x"), attr_in(text, "y"))
            });
            let leader = label.is_some_and(|label| leader_joins(inner, mark, label));
            ticks.push(AnchoredTick {
                axis,
                metres,
                mark,
                label,
                leader,
            });
            rest = &rest[close + 4..];
        }
        ticks
    }

    fn leader_joins(inner: &str, mark: (f64, f64), label: (f64, f64)) -> bool {
        let Some(at) = inner.find("data-tick-leader=\"1\"") else {
            return false;
        };
        let start = inner[..at].rfind("<line").unwrap();
        let end = inner[at..].find('>').unwrap() + at;
        let tag = &inner[start..end];
        let a = (attr_in(tag, "x1"), attr_in(tag, "y1"));
        let b = (attr_in(tag, "x2"), attr_in(tag, "y2"));
        (dist(a, mark) < 4.0 || dist(b, mark) < 4.0)
            && (dist(a, label) < 8.0 || dist(b, label) < 8.0)
    }

    fn axis_lines(svg: &str) -> Vec<(String, ScreenSeg)> {
        let mut lines = Vec::new();
        let mut rest = svg;
        let key = "data-axis-line=\"";
        while let Some(at) = rest.find(key) {
            let start = rest[..at].rfind("<line").unwrap();
            let end = rest[at..].find('>').unwrap() + at;
            let tag = &rest[start..end];
            lines.push((
                quoted_attr(tag, "data-axis-line"),
                (
                    (attr_in(tag, "x1"), attr_in(tag, "y1")),
                    (attr_in(tag, "x2"), attr_in(tag, "y2")),
                ),
            ));
            rest = &rest[end..];
        }
        lines
    }

    fn quoted_attr(tag: &str, name: &str) -> String {
        let key = format!("{name}=\"");
        let start = tag.find(&key).unwrap_or_else(|| panic!("{name}")) + key.len();
        let end = tag[start..].find('"').unwrap() + start;
        tag[start..end].to_string()
    }

    fn text_xy(svg: &str, name: &str) -> (f64, f64) {
        let key = format!("data-label=\"{name}\"");
        let at = svg.find(&key).unwrap_or_else(|| panic!("label {name}"));
        let tag = &svg[at..svg[at..].find('>').unwrap() + at];
        (attr_in(tag, "x"), attr_in(tag, "y"))
    }

    fn circle_radius(svg: &str) -> f64 {
        let key = "data-radius-m=\"0.25\"";
        let at = svg.find(key).expect("radius");
        let tag = &svg[at..svg[at..].find('>').unwrap() + at];
        attr_in(tag, "r")
    }

    fn attr_in(tag: &str, name: &str) -> f64 {
        let key = format!("{name}=\"");
        let start = tag.find(&key).unwrap_or_else(|| panic!("{name}")) + key.len();
        let end = tag[start..].find('"').unwrap() + start;
        tag[start..end].parse().expect(name)
    }

    fn geom_lines(svg: &str) -> Vec<((f64, f64), (f64, f64))> {
        let mut lines = Vec::new();
        let mut rest = svg;
        while let Some(at) = rest.find("<line ") {
            let tag_end = rest[at..].find('>').unwrap() + at;
            let tag = &rest[at..tag_end];
            if tag.contains("data-geom=\"1\"") {
                lines.push((
                    (attr_in(tag, "x1"), attr_in(tag, "y1")),
                    (attr_in(tag, "x2"), attr_in(tag, "y2")),
                ));
            }
            rest = &rest[tag_end..];
        }
        lines
    }

    fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        dx.hypot(dy)
    }

    fn dist_seg(point: (f64, f64), segment: ((f64, f64), (f64, f64))) -> f64 {
        let (a, b) = segment;
        let abx = b.0 - a.0;
        let aby = b.1 - a.1;
        let len2 = abx * abx + aby * aby;
        if len2 < 1e-9 {
            return dist(point, a);
        }
        let t = ((point.0 - a.0) * abx + (point.1 - a.1) * aby) / len2;
        let t = t.clamp(0.0, 1.0);
        dist(point, (a.0 + abx * t, a.1 + aby * t))
    }

    fn line_attr(svg_prefix: &str, name: &str) -> f64 {
        let key = format!("{name}=\"");
        let start = svg_prefix.rfind(&key).expect(name) + key.len();
        let end = svg_prefix[start..].find('"').expect("quote") + start;
        svg_prefix[start..end].parse().expect("number")
    }
}
