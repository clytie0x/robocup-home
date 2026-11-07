//! Ideal pinhole depth from the sourced mounts. Not a calibrated camera.
//!
//! Axial depth is the hit's component along the optical axis. The Euclidean
//! range is longer off that axis. Pixel indexes are pixel centers. Columns 0
//! and 319 are the cited horizontal field, and rows 0 and 239 are the cited
//! vertical field. The recorded optical row 153.6 is not applied.

use crate::base::{
    BasePose, base_to_world, collision_plates, enter_prism, rotate_to_base, world_to_base,
};
use crate::kind::Kind;
use crate::observe::{DepthClass, DepthView, Freshness, classify_depth_mm};
use crate::profile::{CameraProfile, DEPTH_GATE_M, HFOV_DEG, VFOV_DEG};
use crate::scene::{
    Aabb, Plane, SceneError, ScreenSeg, Vec3, enter_box, fit_plane, plane_axes, svg_header,
    swatches,
};

pub const IMAGE_WIDTH: u32 = 320;
pub const IMAGE_HEIGHT: u32 = 240;
const STRIDE: u32 = 16;

#[derive(Clone, Copy, Debug)]
pub struct CastQuery<'a> {
    pub pose: BasePose,
    pub profile: CameraProfile,
    pub obstacles: &'a [Aabb],
    pub plates: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepthRay {
    pub col: Option<u32>,
    pub row: Option<u32>,
    pub axial_m: Option<f64>,
    pub range_m: Option<f64>,
    pub axial_mm: u16,
    pub direction: Vec3,
    pub name: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraViews {
    pub model_csv: String,
    pub trace_csv: String,
    pub fault_csv: String,
    pub side_svg: String,
    pub row_svg: String,
    pub grid_svg: String,
}

pub fn cast_pixel(query: CastQuery<'_>, col: u32, row: u32) -> Result<DepthRay, SceneError> {
    checked(query, Some(col), Some(row))?;
    let (image_right, image_down) = tangents(col, row);
    cast_tangent(query, image_right, image_down, Some(col), Some(row))
}

pub fn cast_axis(query: CastQuery<'_>) -> Result<DepthRay, SceneError> {
    checked(query, None, None)?;
    cast_tangent(query, 0.0, 0.0, None, None)
}

pub fn pixel_for_tangent(image_right: f64, image_down: f64) -> Option<(u32, u32)> {
    if !image_right.is_finite() || !image_down.is_finite() {
        return None;
    }
    let (cx, cy, half_h, half_v) = centers();
    if half_h <= 0.0 || half_v <= 0.0 {
        return None;
    }
    let col = cx + image_right / half_h * cx;
    let row = cy + image_down / half_v * cy;
    let max_col = f64::from(IMAGE_WIDTH - 1);
    let max_row = f64::from(IMAGE_HEIGHT - 1);
    if col < -1e-9 || row < -1e-9 || col > max_col + 1e-9 || row > max_row + 1e-9 {
        return None;
    }
    Some((col.round() as u32, row.round() as u32))
}

pub fn apply_dropout(samples: &mut [DepthRay], indexes: &[usize]) {
    for index in indexes {
        if let Some(sample) = samples.get_mut(*index) {
            sample.axial_mm = 0;
        }
    }
}

pub fn post_depth(samples: &[DepthRay], freshness: Freshness) -> DepthView {
    DepthView {
        freshness,
        width: samples.len(),
        height: 1,
        mm: samples.iter().map(|sample| sample.axial_mm).collect(),
        gate_mm: (DEPTH_GATE_M * 1000.0).round() as u16,
        gate_kind: Kind::Configured,
    }
}

pub fn demonstration_bodies() -> Vec<Aabb> {
    vec![
        Aabb {
            name: "low-box",
            min: Vec3 {
                x: 0.55,
                y: -0.35,
                z: 0.0,
            },
            max: Vec3 {
                x: 1.00,
                y: 0.35,
                z: 0.12,
            },
        },
        Aabb {
            name: "near-post",
            min: Vec3 {
                x: 0.48,
                y: 0.04,
                z: 0.0,
            },
            max: Vec3 {
                x: 0.56,
                y: 0.20,
                z: 0.55,
            },
        },
        Aabb {
            name: "occluder",
            min: Vec3 {
                x: 1.15,
                y: -0.45,
                z: 0.0,
            },
            max: Vec3 {
                x: 1.28,
                y: -0.02,
                z: 0.90,
            },
        },
        Aabb {
            name: "far-wall",
            min: Vec3 {
                x: 2.00,
                y: -1.20,
                z: 0.0,
            },
            max: Vec3 {
                x: 2.06,
                y: 1.20,
                z: 0.80,
            },
        },
    ]
}

pub fn demo_camera() -> Result<CameraViews, SceneError> {
    let profile = measured();
    let pose = BasePose {
        x_m: 0.0,
        y_m: 0.0,
        yaw_deg: 0.0,
    };
    let bodies = demonstration_bodies();
    let mut samples = Vec::new();
    for col in 0..IMAGE_WIDTH {
        samples.push(cast_pixel(
            CastQuery {
                pose,
                profile,
                obstacles: &bodies,
                plates: true,
            },
            col,
            120,
        )?);
    }
    samples.push(cast_pixel(
        CastQuery {
            pose,
            profile,
            obstacles: &bodies,
            plates: true,
        },
        160,
        239,
    )?);
    let mut grid = Vec::new();
    let mut row = 0;
    while row < IMAGE_HEIGHT {
        let mut col = 0;
        while col < IMAGE_WIDTH {
            grid.push(cast_pixel(
                CastQuery {
                    pose,
                    profile,
                    obstacles: &bodies,
                    plates: true,
                },
                col,
                row,
            )?);
            col += STRIDE;
        }
        row += STRIDE;
    }
    let mut fault = samples[..8].to_vec();
    apply_dropout(&mut fault, &[0]);
    Ok(CameraViews {
        model_csv: model_csv(),
        trace_csv: trace_csv(&samples, "fresh"),
        fault_csv: trace_csv(&fault, "stale"),
        side_svg: draw_side(pose, profile, &bodies)?,
        row_svg: draw_row(&samples),
        grid_svg: draw_grid(&grid),
    })
}

fn measured() -> CameraProfile {
    crate::profile::camera_profiles()
        .into_iter()
        .find(|profile| profile.id == "measured-mount")
        .expect("measured mount")
}

fn cast_tangent(
    query: CastQuery<'_>,
    image_right: f64,
    image_down: f64,
    col: Option<u32>,
    row: Option<u32>,
) -> Result<DepthRay, SceneError> {
    let origin = base_to_world(
        &query.pose,
        Vec3 {
            x: query.profile.x_m,
            y: 0.0,
            z: query.profile.z_m,
        },
    );
    let direction = ray_direction(
        &query.pose,
        query.profile.pitch_down_deg,
        image_right,
        image_down,
    )?;
    let axis = ray_direction(&query.pose, query.profile.pitch_down_deg, 0.0, 0.0)?;
    let mut best: Option<(f64, &'static str)> = None;
    if query.plates {
        let origin_base = world_to_base(&query.pose, origin);
        let direction_base = rotate_to_base(&query.pose, direction);
        for plate in &collision_plates() {
            if let Some(range) = enter_prism(origin_base, direction_base, plate) {
                keep(&mut best, range, plate.name);
            }
        }
    }
    for body in query.obstacles {
        if let Some(range) = enter_box(origin, direction, body) {
            keep(&mut best, range, body.name);
        }
    }
    let Some((range_m, name)) = best else {
        return Ok(empty(col, row, direction));
    };
    let forward = dot(direction, axis);
    if forward <= 1e-9 {
        return Ok(empty(col, row, direction));
    }
    let axial_m = range_m * forward;
    Ok(DepthRay {
        col,
        row,
        axial_m: Some(axial_m),
        range_m: Some(range_m),
        axial_mm: axial_word(axial_m),
        direction,
        name: Some(name),
    })
}

fn ray_direction(
    pose: &BasePose,
    pitch_down_deg: f64,
    image_right: f64,
    image_down: f64,
) -> Result<Vec3, SceneError> {
    let pitch = pitch_down_deg.to_radians();
    let yaw = pose.yaw_deg.to_radians();
    if !pitch.is_finite() || !yaw.is_finite() {
        return Err(SceneError::NonFinite);
    }
    let length = (1.0 + image_right * image_right + image_down * image_down).sqrt();
    let forward = 1.0 / length;
    let left = image_right / length;
    let up = -image_down / length;
    let pitched_x = pitch.cos() * forward + pitch.sin() * up;
    let pitched_z = -pitch.sin() * forward + pitch.cos() * up;
    Ok(Vec3 {
        x: yaw.cos() * pitched_x - yaw.sin() * left,
        y: yaw.sin() * pitched_x + yaw.cos() * left,
        z: pitched_z,
    })
}

fn tangents(col: u32, row: u32) -> (f64, f64) {
    let (cx, cy, half_h, half_v) = centers();
    let image_right = (f64::from(col) - cx) / cx * half_h;
    let image_down = (f64::from(row) - cy) / cy * half_v;
    (image_right, image_down)
}

fn centers() -> (f64, f64, f64, f64) {
    let cx = f64::from(IMAGE_WIDTH - 1) / 2.0;
    let cy = f64::from(IMAGE_HEIGHT - 1) / 2.0;
    let half_h = (HFOV_DEG.to_radians() / 2.0).tan();
    let half_v = (VFOV_DEG.to_radians() / 2.0).tan();
    (cx, cy, half_h, half_v)
}

fn keep(best: &mut Option<(f64, &'static str)>, range: f64, name: &'static str) {
    if best.map(|(kept, _)| range < kept).unwrap_or(true) {
        *best = Some((range, name));
    }
}

fn dot(left: Vec3, right: Vec3) -> f64 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn axial_word(axial_m: f64) -> u16 {
    if !axial_m.is_finite() || axial_m <= 0.0 {
        return 0;
    }
    let mm = (axial_m * 1000.0).round();
    if (1.0..=f64::from(u16::MAX)).contains(&mm) {
        mm as u16
    } else {
        0
    }
}

fn empty(col: Option<u32>, row: Option<u32>, direction: Vec3) -> DepthRay {
    DepthRay {
        col,
        row,
        axial_m: None,
        range_m: None,
        axial_mm: 0,
        direction,
        name: None,
    }
}

fn checked(query: CastQuery<'_>, col: Option<u32>, row: Option<u32>) -> Result<(), SceneError> {
    if !query.pose.x_m.is_finite()
        || !query.pose.y_m.is_finite()
        || !query.pose.yaw_deg.is_finite()
        || !query.profile.x_m.is_finite()
        || !query.profile.z_m.is_finite()
        || !query.profile.pitch_down_deg.is_finite()
    {
        return Err(SceneError::NonFinite);
    }
    if query
        .obstacles
        .iter()
        .any(|body| !body.min.finite() || !body.max.finite())
    {
        return Err(SceneError::NonFinite);
    }
    if col.is_some_and(|col| col >= IMAGE_WIDTH) || row.is_some_and(|row| row >= IMAGE_HEIGHT) {
        return Err(SceneError::OutOfImage);
    }
    Ok(())
}

fn model_csv() -> String {
    "\
item,value,kind,source
image,320x240,cited device class,sensors.json model string. Not measured from a decoded PNG
horizontal_fov_deg,58.59,cited OpenNI property,sensors.json and HFOV_DEG. Not remeasured
vertical_fov_deg,45.64,cited and rounded,sensors.json. The unrounded build.py constant is not used
mirror,image right is robot left,cited,session README. Not remeasured
principal_point,pixel centers at (width-1)/2 and (height-1)/2,assumption,same center as pixel_bearing_deg. The recorded row 153.6 is not applied
depth,axial millimetres along the optical axis,cited,session README. 0 is no return. Not the Euclidean range
near_gate_m,0.5,cited OpenNI property,sensors.json depth_valid_from_m. A positive sample under the gate stays BelowGate
measured_mount,\"x 0.14 m, z 0.282 m, pitch 6.7 deg down\",floor-plane fit,sensors.json and the URDF camera_link origin. This run does not refit the plane
configured_mount,\"x 0.14 m, z 0.27 m, pitch 0\",configured,pi robot.json. mount_confirmed is a flag
optical_frame,\"rpy -pi/2 0 -pi/2 from camera_link\",model convention,URDF camera_optical_joint. Not a second measured extrinsics
focal_length,not stored,derived from the field and the image,no calibrated fx or fy is in the files
distortion,unknown,unknown,no model is added
device_maximum_range,unknown,unknown,no far cutoff is added
camera_body,not an occluder,assumed product size,the 35 by 178 by 45 mm box is not a collision solid
calibration,not calibrated and not installed,limit,neither mount is chosen as the installed camera
demo_pixels,one row of 320 plus a stride of 16,performance cap,not a full 320 by 240 frame and not a timing benchmark
"
    .to_string()
}

fn trace_csv(samples: &[DepthRay], fresh: &str) -> String {
    let mut csv = String::from("col,row,axial_mm,range_m,depth_class,fresh,sent\n");
    let gate = (DEPTH_GATE_M * 1000.0).round() as u16;
    for sample in samples {
        let range = sample
            .range_m
            .map(|range| format!("{range:.4}"))
            .unwrap_or_default();
        let class = match classify_depth_mm(sample.axial_mm, gate) {
            DepthClass::NoReturn => "no-return",
            DepthClass::BelowGate => "below-gate",
            DepthClass::Valid => "valid",
        };
        let col = sample.col.map(|col| col.to_string()).unwrap_or_default();
        let row = sample.row.map(|row| row.to_string()).unwrap_or_default();
        csv.push_str(&format!(
            "{col},{row},{},{range},{class},{fresh},false\n",
            sample.axial_mm
        ));
    }
    csv
}

fn draw_side(
    pose: BasePose,
    profile: CameraProfile,
    bodies: &[Aabb],
) -> Result<String, SceneError> {
    let mut cloud = vec![(0.0, 0.0), (-0.25, 0.0), (2.2, 0.9)];
    for body in bodies {
        cloud.push((body.min.x, body.min.z));
        cloud.push((body.max.x, body.max.z));
    }
    let plane = fit_plane(&cloud, 70.0, 150.0, 900.0, 520.0);
    let mut geometry = Vec::new();
    let mut body = svg_header("Measured-mount camera and the level lidar, side view");
    body.push_str("<text x=\"24\" y=\"96\" font-size=\"12\">Fixture geometry. Labels here are not posted to the controller. The camera housing is not a solid.</text>");
    body.push_str("<text x=\"24\" y=\"116\" font-size=\"12\">Equal scale in metres. This view collapses y, so a ray may cross a box that stands beside it. The lidar ray stays at 0.22 m.</text>");
    for (index, plate) in collision_plates().iter().enumerate() {
        let x0 = plate
            .polygon_xy_m
            .iter()
            .map(|point| point.0)
            .fold(f64::MAX, f64::min);
        let x1 = plate
            .polygon_xy_m
            .iter()
            .map(|point| point.0)
            .fold(f64::MIN, f64::max);
        body.push_str(&rect(
            &plane,
            Span {
                x0,
                z0: plate.z_min_m,
                x1,
                z1: plate.z_max_m,
            },
            "#8fb8d6",
            &format!("plate-{index}"),
            &mut geometry,
        ));
    }
    for obstacle in bodies {
        let color = match obstacle.name {
            "low-box" => "#d35f3f",
            "near-post" => "#d4a017",
            "occluder" => "#5f6b7a",
            _ => "#b7c4b1",
        };
        body.push_str(&rect(
            &plane,
            Span {
                x0: obstacle.min.x,
                z0: obstacle.min.z,
                x1: obstacle.max.x,
                z1: obstacle.max.z,
            },
            color,
            obstacle.name,
            &mut geometry,
        ));
    }
    let query = CastQuery {
        pose,
        profile,
        obstacles: bodies,
        plates: true,
    };
    body.push_str(&ray_line(
        &plane,
        &query,
        160,
        120,
        "#1d4e89",
        "optical-row",
        &mut geometry,
    )?);
    body.push_str(&ray_line(
        &plane,
        &query,
        160,
        239,
        "#9b2226",
        "low-ray",
        &mut geometry,
    )?);
    let lidar = crate::base::cast_sensor(&pose, "measured-front", 0.0, 0.0, bodies)?;
    let lidar_origin = base_to_world(
        &pose,
        Vec3 {
            x: crate::profile::LIDAR_X_M,
            y: 0.0,
            z: crate::profile::LIDAR_Z_M,
        },
    );
    let lidar_end = lidar.map(|hit| hit.point).unwrap_or(Vec3 {
        x: lidar_origin.x + 1.5,
        y: 0.0,
        z: lidar_origin.z,
    });
    body.push_str(&segment(
        &plane,
        Span {
            x0: lidar_origin.x,
            z0: lidar_origin.z,
            x1: lidar_end.x,
            z1: lidar_end.z,
        },
        "#2a6f4e",
        "lidar",
        &mut geometry,
    ));
    let camera = base_to_world(
        &pose,
        Vec3 {
            x: profile.x_m,
            y: 0.0,
            z: profile.z_m,
        },
    );
    let mark = plane.screen(camera.x, camera.z);
    body.push_str(&format!(
        "<circle data-mount=\"measured\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"4.5\" fill=\"#1d4e89\"/>",
        mark.0, mark.1
    ));
    let (axes, _) = plane_axes(&plane, ("x, m", "z, m", "x", "z"), &cloud, 0.08, &geometry);
    body.push_str(&axes);
    body.push_str(&swatches(
        &[
            ("#8fb8d6", "plate x extent"),
            ("#d35f3f", "low box"),
            ("#d4a017", "near post"),
            ("#5f6b7a", "occluder"),
        ],
        700.0,
    ));
    body.push_str(&swatches(
        &[
            ("#1d4e89", "camera row 120"),
            ("#9b2226", "camera row 239"),
            ("#2a6f4e", "level lidar"),
        ],
        728.0,
    ));
    body.push_str("</svg>");
    Ok(body)
}

fn ray_line(
    plane: &Plane,
    query: &CastQuery<'_>,
    col: u32,
    row: u32,
    color: &str,
    name: &str,
    geometry: &mut Vec<ScreenSeg>,
) -> Result<String, SceneError> {
    let sample = cast_pixel(*query, col, row)?;
    let origin = base_to_world(
        &query.pose,
        Vec3 {
            x: query.profile.x_m,
            y: 0.0,
            z: query.profile.z_m,
        },
    );
    let end = sample.range_m.map(|range| Vec3 {
        x: origin.x + sample.direction.x * range,
        y: origin.y + sample.direction.y * range,
        z: origin.z + sample.direction.z * range,
    });
    let Some(end) = end else {
        return Ok(String::new());
    };
    Ok(segment(
        plane,
        Span {
            x0: origin.x,
            z0: origin.z,
            x1: end.x,
            z1: end.z,
        },
        color,
        name,
        geometry,
    ))
}

struct Span {
    x0: f64,
    z0: f64,
    x1: f64,
    z1: f64,
}

fn rect(
    plane: &Plane,
    span: Span,
    color: &str,
    name: &str,
    geometry: &mut Vec<ScreenSeg>,
) -> String {
    let a = plane.screen(span.x0, span.z0);
    let b = plane.screen(span.x1, span.z0);
    let c = plane.screen(span.x1, span.z1);
    let d = plane.screen(span.x0, span.z1);
    geometry.push((a, b));
    geometry.push((b, c));
    geometry.push((c, d));
    geometry.push((d, a));
    format!(
        "<polygon data-body=\"{name}\" points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"{color}\" fill-opacity=\"0.85\" stroke=\"#222222\" stroke-width=\"1\"/>",
        a.0, a.1, b.0, b.1, c.0, c.1, d.0, d.1
    )
}

fn segment(
    plane: &Plane,
    span: Span,
    color: &str,
    name: &str,
    geometry: &mut Vec<ScreenSeg>,
) -> String {
    let a = plane.screen(span.x0, span.z0);
    let b = plane.screen(span.x1, span.z1);
    geometry.push((a, b));
    format!(
        "<line data-ray=\"{name}\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{color}\" stroke-width=\"1.6\"/>",
        a.0, a.1, b.0, b.1
    )
}

fn draw_row(samples: &[DepthRay]) -> String {
    let plot = Plot {
        left: 90.0,
        right: 980.0,
        top: 150.0,
        bottom: 640.0,
    };
    let mut body = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1040\" height=\"760\" viewBox=\"0 0 1040 760\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>",
    );
    body.push_str(
        "<text x=\"24\" y=\"32\" font-size=\"16\">Posted axial depth on image row 120</text>",
    );
    body.push_str("<text x=\"24\" y=\"54\" font-size=\"12\">Horizontal ticks are column indexes, not metres. Vertical ticks are axial metres along the optical axis.</text>");
    body.push_str("<text x=\"24\" y=\"74\" font-size=\"12\">A no-return is omitted. It is not drawn as a depth of 0 m. This is not a photograph and not a detector.</text>");
    body.push_str(&axis_line(
        plot.left,
        plot.bottom,
        plot.right,
        plot.bottom,
        "column",
    ));
    body.push_str(&axis_line(
        plot.left,
        plot.bottom,
        plot.left,
        plot.top,
        "axial",
    ));
    for column in [0, 80, 160, 240, 319] {
        let x = plot.x(f64::from(column), 319.0);
        body.push_str(&tick(
            x,
            plot.bottom,
            0.0,
            16.0,
            "column",
            &column.to_string(),
        ));
    }
    for metres in [0.0, 0.5, 1.0, 1.5, 2.0, 2.5] {
        let y = plot.y(metres, 2.5);
        body.push_str(&tick(
            plot.left,
            y,
            -36.0,
            4.0,
            "axial-m",
            &format!("{metres:.1}"),
        ));
    }
    body.push_str("<text x=\"500\" y=\"680\" font-size=\"12\">column index</text>");
    body.push_str("<text x=\"24\" y=\"390\" font-size=\"12\" transform=\"rotate(-90 24 390)\">axial depth, m</text>");
    for sample in samples.iter().filter(|sample| sample.row == Some(120)) {
        let Some(axial) = sample.axial_m else {
            continue;
        };
        let x = plot.x(f64::from(sample.col.unwrap_or(0)), 319.0);
        let y = plot.y(axial.min(2.5), 2.5);
        body.push_str(&format!(
            "<circle data-depth-mark=\"1\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"2.2\" fill=\"#1d4e89\"/>"
        ));
    }
    body.push_str(&swatches(&[("#1d4e89", "axial depth, row 120")], 720.0));
    body.push_str("</svg>");
    body
}

fn draw_grid(samples: &[DepthRay]) -> String {
    let mut body = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1040\" height=\"760\" viewBox=\"0 0 1040 760\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>",
    );
    body.push_str("<text x=\"24\" y=\"32\" font-size=\"16\">Posted axial depth, stride 16</text>");
    body.push_str("<text x=\"24\" y=\"54\" font-size=\"12\">Each cell is one cast at that pixel center. The stride does not interpolate. Colour is axial metres, not a fixture identity.</text>");
    body.push_str("<text x=\"24\" y=\"74\" font-size=\"12\">Image row 0 is the top. Column ticks are indexes. This is not a photograph, not a calibration, and not a detector.</text>");
    let origin_x = 90.0;
    let origin_y = 120.0;
    let cell = 28.0;
    for sample in samples {
        let col = sample.col.unwrap_or(0) / STRIDE;
        let row = sample.row.unwrap_or(0) / STRIDE;
        let x = origin_x + f64::from(col) * cell;
        let y = origin_y + f64::from(row) * cell;
        let (fill, mm) = match sample.axial_m {
            Some(axial) => (depth_fill(axial), sample.axial_mm.to_string()),
            None => ("#d9d9d9".to_string(), "0".to_string()),
        };
        body.push_str(&format!(
            "<rect data-cell=\"1\" data-axial-mm=\"{mm}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"26\" height=\"26\" fill=\"{fill}\"/>"
        ));
    }
    for column in [0, 160, 304] {
        let x = origin_x + f64::from(column / STRIDE) * cell;
        body.push_str(&tick(
            x,
            origin_y + 15.0 * cell,
            0.0,
            18.0,
            "column",
            &column.to_string(),
        ));
    }
    body.push_str("<text x=\"90\" y=\"620\" font-size=\"12\">column index, stride 16</text>");
    let legend_x = 760.0;
    for (index, metres) in [0.25, 0.5, 1.0, 2.0].into_iter().enumerate() {
        let y = 140.0 + index as f64 * 36.0;
        body.push_str(&format!(
            "<rect data-legend=\"scale\" x=\"{legend_x:.1}\" y=\"{y:.1}\" width=\"28\" height=\"18\" fill=\"{}\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{metres:.2} m axial</text>",
            depth_fill(metres),
            legend_x + 36.0,
            y + 14.0
        ));
    }
    body.push_str(&format!(
        "<rect data-legend=\"swatch\" x=\"{legend_x:.1}\" y=\"290\" width=\"28\" height=\"18\" fill=\"#d9d9d9\"/><text x=\"{:.1}\" y=\"304\" font-size=\"12\">no return</text>",
        legend_x + 36.0
    ));
    body.push_str("</svg>");
    body
}

fn depth_fill(axial_m: f64) -> String {
    let t = (axial_m / 2.0).clamp(0.0, 1.0);
    let red = (24.0 + 216.0 * t) as u8;
    let green = (64.0 + 150.0 * (1.0 - (t - 0.45).abs()).clamp(0.0, 1.0)) as u8;
    let blue = (210.0 * (1.0 - t)) as u8;
    format!("#{red:02x}{green:02x}{blue:02x}")
}

struct Plot {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Plot {
    fn x(&self, value: f64, span: f64) -> f64 {
        self.left + (self.right - self.left) * value / span
    }

    fn y(&self, value: f64, span: f64) -> f64 {
        self.bottom - (self.bottom - self.top) * value / span
    }
}

fn axis_line(x1: f64, y1: f64, x2: f64, y2: f64, name: &str) -> String {
    format!(
        "<line data-axis-line=\"{name}\" x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" stroke=\"#222222\" stroke-width=\"1.4\"/>"
    )
}

fn tick(x: f64, y: f64, dx: f64, dy: f64, axis: &str, label: &str) -> String {
    format!(
        "<g data-axis-tick=\"{axis}\"><line data-tick-stroke=\"1\" x1=\"{x:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222222\" stroke-width=\"1.2\"/><text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\">{label}</text></g>",
        x + dx.signum() * 8.0,
        y + dy.signum() * 8.0,
        x + dx,
        y + dy
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CastQuery, apply_dropout, cast_axis, cast_pixel, demo_camera, pixel_for_tangent, post_depth,
    };
    use crate::base::{BasePose, cast_sensor};
    use crate::observe::{
        Actuation, DepthClass, Freshness, PersonPost, PostedScan, classify_depth_mm,
        controller_note, pixel_bearing_deg,
    };
    use crate::profile::{
        CameraProfile, DEPTH_GATE_M, HFOV_DEG, VFOV_DEG, camera_profiles, floor_hit,
    };
    use crate::scene::{Aabb, SceneError, Vec3};

    fn origin() -> BasePose {
        BasePose {
            x_m: 0.0,
            y_m: 0.0,
            yaw_deg: 0.0,
        }
    }

    fn profile(id: &str) -> CameraProfile {
        camera_profiles()
            .into_iter()
            .find(|profile| profile.id == id)
            .unwrap()
    }

    fn wall(name: &'static str, x0: f64, x1: f64) -> Aabb {
        Aabb {
            name,
            min: Vec3 {
                x: x0,
                y: -2.0,
                z: -1.0,
            },
            max: Vec3 {
                x: x1,
                y: 2.0,
                z: 2.0,
            },
        }
    }

    fn query<'a>(
        pose: BasePose,
        profile: CameraProfile,
        obstacles: &'a [Aabb],
        plates: bool,
    ) -> CastQuery<'a> {
        CastQuery {
            pose,
            profile,
            obstacles,
            plates,
        }
    }

    #[test]
    fn a_frontal_wall_keeps_axial_depth_and_lengthens_the_range() {
        let profile = profile("configured-mount");
        let face = profile.x_m + 1.0;
        let bodies = [wall("wall", face, face + 0.2)];
        let edge = cast_pixel(query(origin(), profile, &bodies, false), 0, 120).unwrap();
        let center = cast_pixel(query(origin(), profile, &bodies, false), 160, 120).unwrap();
        let far = cast_pixel(query(origin(), profile, &bodies, false), 319, 120).unwrap();
        assert!((edge.axial_m.unwrap() - 1.0).abs() < 1e-6);
        assert!((center.axial_m.unwrap() - 1.0).abs() < 1e-6);
        assert!((far.axial_m.unwrap() - 1.0).abs() < 1e-6);
        assert!(edge.range_m.unwrap() > edge.axial_m.unwrap() + 0.05);
        assert!(center.range_m.unwrap() > center.axial_m.unwrap());
        assert!(edge.range_m.unwrap() > center.range_m.unwrap());
        let bearing = edge.direction.y.atan2(edge.direction.x).to_degrees();
        let other = far.direction.y.atan2(far.direction.x).to_degrees();
        assert!((bearing + HFOV_DEG / 2.0).abs() < 1e-6, "{bearing}");
        assert!((other - HFOV_DEG / 2.0).abs() < 1e-6, "{other}");
        let helper = pixel_bearing_deg(0.0, 320.0, HFOV_DEG, true).unwrap();
        assert!((bearing - helper).abs() < 1e-6);
        assert_eq!(edge.axial_mm, 1000);
        assert!(classify_depth_mm(edge.axial_mm, 500) == DepthClass::Valid);
    }

    #[test]
    fn the_optical_axis_depth_is_not_the_horizontal_ground_range() {
        let measured = profile("measured-mount");
        let floor = Aabb {
            name: "floor",
            min: Vec3 {
                x: -4.0,
                y: -4.0,
                z: -0.05,
            },
            max: Vec3 {
                x: 8.0,
                y: 4.0,
                z: 0.0,
            },
        };
        let hit = cast_axis(query(origin(), measured, &[floor], false)).unwrap();
        let pitch = measured.pitch_down_deg.to_radians();
        let along = measured.z_m / pitch.sin();
        assert!((hit.axial_m.unwrap() - along).abs() < 1e-6);
        assert!((hit.range_m.unwrap() - along).abs() < 1e-6);
        let horizontal = floor_hit(measured).optical_ground_range_m.unwrap();
        assert!((horizontal - measured.z_m / pitch.tan()).abs() < 1e-9);
        assert!((hit.axial_m.unwrap() - horizontal).abs() > 0.01);
        let configured = profile("configured-mount");
        let level = cast_axis(query(origin(), configured, &[floor], false)).unwrap();
        assert!(level.axial_m.is_none());
        assert!(floor_hit(configured).optical_ground_range_m.is_none());
    }

    #[test]
    fn a_low_box_missed_by_the_level_lidar_is_visible_to_the_camera() {
        let measured = profile("measured-mount");
        let low = Aabb {
            name: "low-box",
            min: Vec3 {
                x: 0.40,
                y: -0.80,
                z: 0.0,
            },
            max: Vec3 {
                x: 1.20,
                y: 0.80,
                z: 0.15,
            },
        };
        let lidar = cast_sensor(&origin(), "measured-front", 0.0, 0.0, &[low]).unwrap();
        assert!(lidar.is_none());
        let seen = cast_pixel(query(origin(), measured, &[low], true), 160, 239).unwrap();
        assert_eq!(seen.name, Some("low-box"));
        assert!(seen.range_m.unwrap() > seen.axial_m.unwrap());
        let point_z = measured.z_m + seen.range_m.unwrap() * seen.direction.z;
        assert!((-1e-6..0.15 + 1e-6).contains(&point_z), "{point_z}");
        let short = Aabb {
            name: "low-box",
            min: Vec3 {
                x: 0.40,
                y: -0.80,
                z: 0.0,
            },
            max: Vec3 {
                x: 0.55,
                y: 0.80,
                z: 0.03,
            },
        };
        let missed = cast_pixel(query(origin(), measured, &[short], false), 160, 239).unwrap();
        assert!(missed.axial_m.is_none());
        assert_eq!(missed.axial_mm, 0);
    }

    #[test]
    fn a_near_wall_hides_a_far_wall_until_it_is_removed() {
        let profile = profile("configured-mount");
        let near = wall("near", 1.0, 1.05);
        let far = wall("far", 2.0, 2.05);
        let blocked = cast_pixel(query(origin(), profile, &[near, far], false), 160, 120).unwrap();
        assert_eq!(blocked.name, Some("near"));
        assert!((blocked.point_x() - 1.0).abs() < 1e-6);
        let open = cast_pixel(query(origin(), profile, &[far], false), 160, 120).unwrap();
        assert_eq!(open.name, Some("far"));
        assert!((open.point_x() - 2.0).abs() < 1e-6);
        assert!(open.axial_m.unwrap() > blocked.axial_m.unwrap() + 0.5);
    }

    #[test]
    fn a_box_behind_the_camera_is_a_miss() {
        let profile = profile("configured-mount");
        let behind = wall("behind", -1.2, -0.4);
        let ray = cast_pixel(query(origin(), profile, &[behind], false), 160, 120).unwrap();
        assert!(ray.axial_m.is_none());
        assert_eq!(ray.axial_mm, 0);
        assert!(classify_depth_mm(ray.axial_mm, 500) == DepthClass::NoReturn);
    }

    #[test]
    fn a_near_hit_stays_below_the_gate_and_a_miss_is_zero() {
        let profile = profile("configured-mount");
        let face = profile.x_m + 0.30;
        let near = wall("near", face, face + 0.08);
        let ray = cast_pixel(query(origin(), profile, &[near], false), 160, 120).unwrap();
        assert!((ray.axial_m.unwrap() - 0.30).abs() < 1e-6);
        assert_eq!(ray.axial_mm, 300);
        assert!(ray.axial_mm < (DEPTH_GATE_M * 1000.0).round() as u16);
        assert_eq!(classify_depth_mm(ray.axial_mm, 500), DepthClass::BelowGate);
        let miss = cast_pixel(query(origin(), profile, &[], false), 160, 120).unwrap();
        assert_eq!(miss.axial_mm, 0);
        assert_eq!(classify_depth_mm(miss.axial_mm, 500), DepthClass::NoReturn);
    }

    #[test]
    fn a_yawed_base_looks_along_its_forward_axis() {
        let profile = profile("configured-mount");
        let pose = BasePose {
            x_m: 0.0,
            y_m: 0.0,
            yaw_deg: 90.0,
        };
        let ahead = Aabb {
            name: "ahead",
            min: Vec3 {
                x: -1.0,
                y: profile.x_m + 1.0,
                z: -1.0,
            },
            max: Vec3 {
                x: 1.0,
                y: profile.x_m + 1.2,
                z: 2.0,
            },
        };
        let aside = wall("aside", profile.x_m + 1.0, profile.x_m + 1.2);
        let ray = cast_pixel(query(pose, profile, &[ahead, aside], false), 160, 120).unwrap();
        assert_eq!(ray.name, Some("ahead"));
        assert!((ray.axial_m.unwrap() - 1.0).abs() < 1e-6);
        let only_aside = cast_pixel(query(pose, profile, &[aside], false), 160, 120).unwrap();
        assert!(only_aside.axial_m.is_none());
    }

    #[test]
    fn the_top_plate_occludes_a_wall_when_the_ray_meets_it() {
        let profile = profile("configured-mount");
        let far = wall("far", 1.5, 1.55);
        let blocked = cast_pixel(query(origin(), profile, &[far], true), 160, 239).unwrap();
        assert!(blocked.range_m.unwrap() < 0.25);
        assert_eq!(blocked.name, Some("top_plate"));
        let open = cast_pixel(query(origin(), profile, &[far], false), 160, 239).unwrap();
        assert_eq!(open.name, Some("far"));
        assert!(open.range_m.unwrap() > 1.0);
    }

    #[test]
    fn a_tangent_past_the_field_is_not_a_pixel() {
        let half_h = (HFOV_DEG.to_radians() / 2.0).tan();
        let half_v = (VFOV_DEG.to_radians() / 2.0).tan();
        let edge = pixel_for_tangent(-half_h, 0.0).unwrap();
        assert_eq!(edge.0, 0);
        let bottom = pixel_for_tangent(0.0, half_v).unwrap();
        assert_eq!(bottom.1, super::IMAGE_HEIGHT - 1);
        assert!(pixel_for_tangent(-half_h * 1.02, 0.0).is_none());
        assert!(pixel_for_tangent(0.0, half_v * 1.02).is_none());
    }

    #[test]
    fn dropout_zeros_the_posted_sample_and_leaves_the_cast() {
        let profile = profile("configured-mount");
        let face = profile.x_m + 1.0;
        let bodies = [wall("wall", face, face + 0.2)];
        let mut samples = vec![
            cast_pixel(query(origin(), profile, &bodies, false), 160, 120).unwrap(),
            cast_pixel(query(origin(), profile, &bodies, false), 200, 120).unwrap(),
        ];
        assert!(samples[0].axial_m.is_some());
        let kept = samples[0].axial_m;
        apply_dropout(&mut samples, &[0]);
        assert_eq!(samples[0].axial_mm, 0);
        assert_eq!(samples[0].axial_m, kept);
        assert_eq!(samples[1].axial_mm, 1000);
        let fresh = post_depth(&samples, Freshness::Fresh);
        assert_eq!(fresh.mm, vec![0, 1000]);
    }

    #[test]
    fn a_stale_depth_view_is_not_a_scene_edit() {
        let profile = profile("configured-mount");
        let face = profile.x_m + 1.0;
        let bodies = [wall("wall", face, face + 0.2)];
        let sample = cast_pixel(query(origin(), profile, &bodies, false), 160, 120).unwrap();
        let stale = post_depth(&[sample], Freshness::Stale { age_s: 2.0 });
        assert_eq!(stale.mm, vec![1000]);
        let scan = PostedScan {
            freshness: Freshness::Fresh,
            samples: Vec::new(),
            source_name: "empty".to_string(),
        };
        let note = controller_note(&scan, Some(&stale));
        assert_eq!(note.fresh_valid_depth, 0);
        assert_eq!(note.retained_depth, 1);
        assert_eq!(note.person, PersonPost::NotInPostedView);
        assert_eq!(note.actuation, Actuation::NotSent);
        let views = demo_camera().unwrap();
        assert!(views.trace_csv.lines().any(|line| line.ends_with(",false")));
        assert!(!views.trace_csv.contains("low-box"));
        assert!(!views.trace_csv.contains("chassis"));
    }

    #[test]
    fn a_pixel_outside_the_image_or_a_non_finite_pose_is_rejected() {
        let profile = profile("configured-mount");
        let bad = BasePose {
            x_m: f64::NAN,
            y_m: 0.0,
            yaw_deg: 0.0,
        };
        assert_eq!(
            cast_pixel(query(bad, profile, &[], false), 0, 0),
            Err(SceneError::NonFinite)
        );
        assert_eq!(
            cast_pixel(query(origin(), profile, &[], false), 320, 0),
            Err(SceneError::OutOfImage)
        );
        assert_eq!(
            cast_pixel(query(origin(), profile, &[], false), 0, 240),
            Err(SceneError::OutOfImage)
        );
    }

    #[test]
    fn the_demo_trace_is_the_cast_and_the_pictures_have_scales() {
        let views = demo_camera().unwrap();
        let bodies = super::demonstration_bodies();
        let profile = profile("measured-mount");
        let low = cast_pixel(query(origin(), profile, &bodies, true), 160, 239).unwrap();
        assert_eq!(low.name, Some("low-box"));
        assert!(views.trace_csv.contains(&format!(",239,{},", low.axial_mm)));
        assert!(
            views
                .trace_csv
                .starts_with("col,row,axial_mm,range_m,depth_class,fresh,sent\n")
        );
        assert!(
            views
                .trace_csv
                .lines()
                .skip(1)
                .all(|line| line.ends_with(",false"))
        );
        assert!(views.trace_csv.contains("below-gate") || views.trace_csv.contains("valid"));
        assert!(!views.trace_csv.contains("low-box"));
        assert!(!views.trace_csv.contains("chassis"));
        assert!(views.fault_csv.contains("stale"));
        assert!(
            views
                .fault_csv
                .lines()
                .skip(1)
                .all(|line| line.ends_with(",false"))
        );
        assert!(views.model_csv.contains("not calibrated"));
        assert!(views.side_svg.contains("data-tick-stroke"));
        assert!(views.side_svg.contains("data-body=\"low-box\""));
        assert!(views.side_svg.contains("data-ray=\"lidar\""));
        assert!(views.row_svg.contains("column index"));
        assert!(views.row_svg.contains("data-axis-tick=\"axial-m\""));
        assert!(views.grid_svg.contains("stride 16"));
        assert!(views.grid_svg.contains("data-legend=\"scale\""));
        let fresh = post_depth(&[low], Freshness::Fresh);
        let scan = PostedScan {
            freshness: Freshness::Fresh,
            samples: Vec::new(),
            source_name: "empty".to_string(),
        };
        let note = controller_note(&scan, Some(&fresh));
        assert!(note.fresh_valid_depth + note.retained_depth > 0);
        assert_eq!(note.person, PersonPost::NotInPostedView);
        assert_eq!(note.actuation, Actuation::NotSent);
        assert!(!views.trace_csv.contains("person"));
    }

    impl super::DepthRay {
        fn point_x(&self) -> f64 {
            // The stub has no hit point. The real ray stores it on the direction scale.
            self.range_m.unwrap_or(0.0) * self.direction.x + profile("configured-mount").x_m
        }
    }
}
