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

fn draw_oblique(scene: &Scene, hits: &[Vec3], other_hits: &[Vec3]) -> String {
    let map = |point: Vec3| {
        let (x, y) = project_oblique(point);
        // `project_oblique` already uses a downward y, so z increases up the page.
        (220.0 + x * 150.0, 300.0 + y * 150.0)
    };
    let mut body =
        svg_header("Oblique projection of the 3D coordinates. Two mount yaws. Not a robot mesh.");
    body.push_str(&world_axes(&map));
    body.push_str(&edges(scene, map));
    body.push_str(&body_labels(scene, map));
    body.push_str(&rays(scene.mount.translation_m, hits, map, "#c0392b"));
    body.push_str(&rays(scene.mount.translation_m, other_hits, map, "#1f4e79"));
    body.push_str(&marker(map(scene.mount.translation_m), scene.mount.name));
    body.push_str(&hit_marks(hits, map));
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">Red rays: named mount. Blue rays: yaw 0 at the same translation. Axes start at the synthetic world origin.</text>");
    body.push_str(&dimension_note(scene));
    body.push_str("</svg>");
    body
}

fn draw_top(scene: &Scene, hits: &[Vec3], other_hits: &[Vec3]) -> String {
    let map = |point: Vec3| place(point.x, point.y, 220.0, 460.0, 150.0);
    let mut body =
        svg_header("World top-down, metres. Origin is the synthetic base, not the sensor.");
    body.push_str(&axis_pair(
        &map,
        Vec3 {
            x: 2.6,
            y: 0.0,
            z: 0.0,
        },
        "world x",
        Vec3 {
            x: 0.0,
            y: 2.0,
            z: 0.0,
        },
        "world y",
    ));
    body.push_str(&edges(scene, map));
    body.push_str(&body_labels(scene, map));
    body.push_str(&rays(scene.mount.translation_m, hits, map, "#c0392b"));
    body.push_str(&rays(scene.mount.translation_m, other_hits, map, "#1f4e79"));
    body.push_str(&marker(map(scene.mount.translation_m), scene.mount.name));
    body.push_str(&hit_marks(hits, map));
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">The named dot is the synthetic sensor. It is not the base origin and not a 0.175 m installed arm.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"13\">Rays cross the plank footprint in this view. The side view shows those rays pass above the plank.</text>");
    body.push_str("</svg>");
    body
}

fn draw_side(scene: &Scene, hits: &[Vec3]) -> String {
    let map = |point: Vec3| place(point.x, point.z, 160.0, 520.0, 170.0);
    let mut body = svg_header(
        "World side view, x and z. The horizontal beam misses the low plank and meets the pillar.",
    );
    body.push_str(&axis_pair(
        &map,
        Vec3 {
            x: 2.6,
            y: 0.0,
            z: 0.0,
        },
        "world x",
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.2,
        },
        "world z",
    ));
    body.push_str(&edges(scene, map));
    body.push_str(&body_labels(scene, map));
    body.push_str(&rays(scene.mount.translation_m, hits, map, "#c0392b"));
    body.push_str(&marker(
        map(scene.mount.translation_m),
        &format!(
            "{} z {:.2} m",
            scene.mount.name, scene.mount.translation_m.z
        ),
    ));
    body.push_str(&hit_marks(hits, map));
    body.push_str("</svg>");
    body
}

fn draw_scan(scene: &Scene, profile: &LidarProfile, points: &[(f64, f64)]) -> String {
    let mut body = svg_header(&format!(
        "Scan frame of {}. Origin is the sensor, not the chassis. Front {}.",
        scene.mount.name, profile.id
    ));
    let map = |x: f64, y: f64| place(x, y, 280.0, 560.0, 140.0);
    let origin = map(0.0, 0.0);
    let x_end = map(2.0, 0.0);
    let y_end = map(0.0, 2.0);
    body.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">scan x, sensor forward</text>",
        origin.0, origin.1, x_end.0, x_end.1, x_end.0 + 6.0, x_end.1
    ));
    body.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">scan y, sensor left</text>",
        origin.0, origin.1, y_end.0, y_end.1, y_end.0 + 8.0, y_end.1 + 14.0
    ));
    let disk = 0.25 * 140.0;
    body.push_str(&format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{disk:.1}\" fill=\"none\" stroke=\"#888888\" stroke-dasharray=\"4 3\"/>",
        origin.0, origin.1
    ));
    for point in points {
        let placed = map(point.0, point.1);
        body.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"2.2\" fill=\"#c0392b\"/>",
            placed.0, placed.1
        ));
    }
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"13\">Dashed 250 mm circle: planner assumption around the posted scan origin. It is not a measured chassis.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"13\">It does not certify clearance. Points are frame_sample returns, not world coordinates and not a measured obstacle radius.</text>");
    body.push_str("</svg>");
    body
}

fn svg_header(title: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1040\" height=\"680\" viewBox=\"0 0 1040 680\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><text x=\"24\" y=\"32\" font-size=\"16\">{title}</text><text x=\"24\" y=\"54\" font-size=\"12\">Axes are metres from the 3D coordinates. This is not a photograph and not a chassis clearance.</text><text x=\"24\" y=\"74\" font-size=\"12\">World and base coincide only in this synthetic fixture. The installed lever arm stays unknown.</text>"
    )
}

fn world_axes(map: &impl Fn(Vec3) -> (f64, f64)) -> String {
    let mut body = axis_pair(
        map,
        Vec3 {
            x: 2.6,
            y: 0.0,
            z: 0.0,
        },
        "world x",
        Vec3 {
            x: 0.0,
            y: 1.5,
            z: 0.0,
        },
        "world y",
    );
    let origin = map(Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let z_end = map(Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.2,
    });
    body.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">world z</text>",
        origin.0,
        origin.1,
        z_end.0,
        z_end.1,
        z_end.0 + 4.0,
        z_end.1
    ));
    body
}

fn axis_pair(
    map: &impl Fn(Vec3) -> (f64, f64),
    x_end: Vec3,
    x_name: &str,
    y_end: Vec3,
    y_name: &str,
) -> String {
    let origin = map(Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let x = map(x_end);
    let y = map(y_end);
    format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{x_name}</text><line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{y_name}</text>",
        origin.0,
        origin.1,
        x.0,
        x.1,
        x.0 + 4.0,
        x.1,
        origin.0,
        origin.1,
        y.0,
        y.1,
        y.0 + 4.0,
        y.1
    )
}

fn body_labels(scene: &Scene, map: impl Fn(Vec3) -> (f64, f64)) -> String {
    let mut body = String::new();
    for aabb in &scene.bodies {
        let center = Vec3 {
            x: (aabb.min.x + aabb.max.x) / 2.0,
            y: (aabb.min.y + aabb.max.y) / 2.0,
            z: (aabb.min.z + aabb.max.z) / 2.0,
        };
        let placed = map(center);
        body.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{}</text>",
            placed.0, placed.1, aabb.name
        ));
    }
    body
}

fn dimension_note(scene: &Scene) -> String {
    let mut body = String::from("<text x=\"24\" y=\"640\" font-size=\"12\">");
    for aabb in &scene.bodies {
        body.push_str(&format!(
            "{} x {:.2}–{:.2} m, y {:.2}–{:.2} m, z {:.2}–{:.2} m. ",
            aabb.name, aabb.min.x, aabb.max.x, aabb.min.y, aabb.max.y, aabb.min.z, aabb.max.z
        ));
    }
    body.push_str("</text>");
    body
}

fn marker(at: (f64, f64), label: &str) -> String {
    format!(
        "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" fill=\"#222222\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{label}</text>",
        at.0,
        at.1,
        at.0 + 8.0,
        at.1 - 8.0
    )
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

fn place(x: f64, y: f64, origin_x: f64, origin_y: f64, scale: f64) -> (f64, f64) {
    (origin_x + x * scale, origin_y - y * scale)
}

fn edges(scene: &Scene, map: impl Fn(Vec3) -> (f64, f64)) -> String {
    let mut body = String::new();
    for aabb in &scene.bodies {
        let corners = corners(aabb);
        for (start, end) in BOX_EDGES {
            let a = map(corners[start]);
            let b = map(corners[end]);
            body.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#333333\" stroke-width=\"1.2\"/>",
                a.0, a.1, b.0, b.1
            ));
        }
    }
    body
}

fn rays(origin: Vec3, hits: &[Vec3], map: impl Fn(Vec3) -> (f64, f64), color: &str) -> String {
    let start = map(origin);
    let mut body = String::new();
    for hit in hits {
        let end = map(*hit);
        body.push_str(&format!(
            "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{color}\" stroke-width=\"1.1\"/>",
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
        Aabb, Cast, Mount, Scene, SceneError, Vec3, cast_bearing, demonstration, inverse_bearing,
        packet_revolution, project_oblique, render_views, sensor_samples,
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
        assert!((point.y - 1.20).abs() < 1e-9);
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
        let (dx, dy, dz) = independent_dir(yaw, pitch, bearing);
        let t = (2.40 - origin.x) / dx;
        let expected = v(origin.x + t * dx, origin.y + t * dy, origin.z + t * dz);
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
        assert!((range_m - t).abs() < 1e-9, "{range_m} {t}");
        assert!((point.x - expected.x).abs() < 1e-9);
        assert!((point.y - expected.y).abs() < 1e-9);
        assert!((point.z - expected.z).abs() < 1e-9);
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

    fn independent_dir(yaw_deg: f64, pitch_down_deg: f64, bearing_deg: f64) -> (f64, f64, f64) {
        let yaw = yaw_deg.to_radians();
        let pitch = pitch_down_deg.to_radians();
        let bearing = bearing_deg.to_radians();
        let x1 = pitch.cos() * bearing.cos();
        let y1 = bearing.sin();
        let z1 = -pitch.sin() * bearing.cos();
        let x = yaw.cos() * x1 - yaw.sin() * y1;
        let y = yaw.sin() * x1 + yaw.cos() * y1;
        (x, y, z1)
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

    fn line_attr(svg_prefix: &str, name: &str) -> f64 {
        let key = format!("{name}=\"");
        let start = svg_prefix.rfind(&key).expect(name) + key.len();
        let end = svg_prefix[start..].find('"').expect("quote") + start;
        svg_prefix[start..end].parse().expect("number")
    }
}
