//! A full synthetic frame from the reviewed pinhole.
//!
//! Every pixel calls `cast_pixel`. The colour is an assumed flat fixture
//! colour, not a photograph and not a depth class. Depth and colour are
//! aligned because they are the same ray. That is not a calibration of a
//! real colour camera against a real depth camera.

use std::time::Instant;

use crate::camera::{CastQuery, IMAGE_HEIGHT, IMAGE_WIDTH, cast_pixel, demonstration_bodies};
use crate::kind::Kind;
use crate::observe::{DepthClass, DepthView, Freshness, classify_depth_mm};
use crate::profile::{DEPTH_GATE_M, camera_profiles};
use crate::scene::SceneError;

pub const FRAME_PIXELS: usize = (IMAGE_WIDTH as usize) * (IMAGE_HEIGHT as usize);
pub const SYNTHETIC_STAMP_MS: u64 = 0;
const HEADER_LEN: usize = 32;
const LIMIT_LINE: &str = "No decoded camera recording. Not a photograph. Not a full photoreal frame. Stage 4 is not complete.";

#[derive(Clone, Debug, PartialEq)]
pub struct CameraFrame {
    pub width: u32,
    pub height: u32,
    pub depth_mm: Vec<u16>,
    pub rgb: Vec<u8>,
    pub depth_fresh: Freshness,
    pub rgb_fresh: Freshness,
    pub stamp_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameFault {
    pub depth_indexes: Vec<usize>,
    pub rgb_indexes: Vec<usize>,
    pub depth_fresh: Freshness,
    pub rgb_fresh: Freshness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameFaultError {
    OutOfRange,
    BothChannels,
}

#[derive(Clone, Debug)]
pub struct FrameViews {
    pub measured: CameraFrame,
    pub configured: CameraFrame,
    pub faulted: CameraFrame,
    pub fault: FrameFault,
    pub measured_ms: u128,
    pub configured_ms: u128,
    pub model_csv: String,
    pub legend_svg: String,
    pub fault_svg: String,
}

pub fn frame_index(col: u32, row: u32) -> Option<usize> {
    if col >= IMAGE_WIDTH || row >= IMAGE_HEIGHT {
        return None;
    }
    Some(row as usize * IMAGE_WIDTH as usize + col as usize)
}

pub fn material_rgb(name: Option<&str>) -> [u8; 3] {
    match name {
        None => [0, 0, 0],
        Some("low-box") => [176, 74, 58],
        Some("near-post") => [196, 154, 48],
        Some("occluder") => [62, 122, 114],
        Some("far-wall") => [206, 198, 176],
        Some("chassis_bottom") => [96, 108, 122],
        Some("chassis_middle") => [132, 144, 156],
        Some("top_plate") => [176, 188, 198],
        Some("floor") => [150, 124, 86],
        Some("near") => [188, 72, 72],
        Some("far") => [64, 148, 112],
        Some("wall") => [156, 156, 188],
        Some("ahead") => [188, 112, 56],
        Some("aside") => [72, 84, 156],
        Some("behind") => [120, 72, 112],
        Some(_) => [148, 148, 148],
    }
}

pub fn class_rgb(class: DepthClass) -> [u8; 3] {
    match class {
        DepthClass::Valid => [0x1d, 0x4e, 0x89],
        DepthClass::BelowGate => [0xd9, 0x77, 0x06],
        DepthClass::NoReturn => [0xbd, 0xbd, 0xbd],
    }
}

pub fn cast_frame(query: CastQuery<'_>) -> Result<CameraFrame, SceneError> {
    let mut depth_mm = vec![0; FRAME_PIXELS];
    let mut rgb = vec![0; FRAME_PIXELS * 3];
    for row in 0..IMAGE_HEIGHT {
        for col in 0..IMAGE_WIDTH {
            let ray = cast_pixel(query, col, row)?;
            let index = frame_index(col, row).expect("pixel");
            depth_mm[index] = ray.axial_mm;
            let colour = material_rgb(ray.name);
            let at = index * 3;
            rgb[at..at + 3].copy_from_slice(&colour);
        }
    }
    Ok(CameraFrame {
        width: IMAGE_WIDTH,
        height: IMAGE_HEIGHT,
        depth_mm,
        rgb,
        depth_fresh: Freshness::Fresh,
        rgb_fresh: Freshness::Fresh,
        stamp_ms: SYNTHETIC_STAMP_MS,
    })
}

pub fn apply_frame_fault(
    frame: &mut CameraFrame,
    fault: &FrameFault,
) -> Result<(), FrameFaultError> {
    for index in fault.depth_indexes.iter().chain(fault.rgb_indexes.iter()) {
        if *index >= FRAME_PIXELS {
            return Err(FrameFaultError::OutOfRange);
        }
    }
    for index in &fault.depth_indexes {
        if fault.rgb_indexes.contains(index) {
            return Err(FrameFaultError::BothChannels);
        }
    }
    for index in &fault.depth_indexes {
        frame.depth_mm[*index] = 0;
    }
    for index in &fault.rgb_indexes {
        let at = index * 3;
        frame.rgb[at..at + 3].fill(0);
    }
    frame.depth_fresh = fault.depth_fresh.clone();
    frame.rgb_fresh = fault.rgb_fresh.clone();
    Ok(())
}

pub fn depth_view(frame: &CameraFrame) -> DepthView {
    DepthView {
        freshness: frame.depth_fresh.clone(),
        width: frame.width as usize,
        height: frame.height as usize,
        mm: frame.depth_mm.clone(),
        gate_mm: gate_mm(),
        gate_kind: Kind::Configured,
    }
}

pub fn encode_frame(frame: &CameraFrame) -> Result<Vec<u8>, &'static str> {
    if frame.width != IMAGE_WIDTH
        || frame.height != IMAGE_HEIGHT
        || frame.depth_mm.len() != FRAME_PIXELS
        || frame.rgb.len() != FRAME_PIXELS * 3
    {
        return Err("dimensions");
    }
    let mut bytes = Vec::with_capacity(HEADER_LEN + FRAME_PIXELS * 5);
    bytes.extend_from_slice(b"RCF1");
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&frame.width.to_le_bytes());
    bytes.extend_from_slice(&frame.height.to_le_bytes());
    bytes.push(fresh_code(&frame.depth_fresh));
    bytes.push(fresh_code(&frame.rgb_fresh));
    bytes.extend_from_slice(&age_ms(&frame.depth_fresh).to_le_bytes());
    bytes.extend_from_slice(&age_ms(&frame.rgb_fresh).to_le_bytes());
    bytes.extend_from_slice(&frame.stamp_ms.to_le_bytes());
    for value in &frame.depth_mm {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&frame.rgb);
    Ok(bytes)
}

pub fn decode_frame(bytes: &[u8]) -> Result<CameraFrame, &'static str> {
    let need = HEADER_LEN + FRAME_PIXELS * 2 + FRAME_PIXELS * 3;
    if bytes.len() != need || bytes.get(..4) != Some(b"RCF1") {
        return Err("not a frame");
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != 1 {
        return Err("version");
    }
    let width = u32::from_le_bytes(bytes[6..10].try_into().map_err(|_| "header")?);
    let height = u32::from_le_bytes(bytes[10..14].try_into().map_err(|_| "header")?);
    if width != IMAGE_WIDTH || height != IMAGE_HEIGHT {
        return Err("dimensions");
    }
    let depth_fresh = fresh_from(bytes[14], read_u32(bytes, 16)?)?;
    let rgb_fresh = fresh_from(bytes[15], read_u32(bytes, 20)?)?;
    let stamp_ms = u64::from_le_bytes(bytes[24..32].try_into().map_err(|_| "header")?);
    let depth_end = HEADER_LEN + FRAME_PIXELS * 2;
    let (pairs, rest) = bytes[HEADER_LEN..depth_end].as_chunks::<2>();
    if !rest.is_empty() || pairs.len() != FRAME_PIXELS {
        return Err("depth");
    }
    let depth_mm = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    Ok(CameraFrame {
        width,
        height,
        depth_mm,
        rgb: bytes[depth_end..].to_vec(),
        depth_fresh,
        rgb_fresh,
        stamp_ms,
    })
}

pub fn rgb_ppm(frame: &CameraFrame) -> Vec<u8> {
    let mut bytes = ppm_header(frame.width, frame.height);
    bytes.extend_from_slice(&frame.rgb);
    bytes
}

pub fn class_ppm(frame: &CameraFrame) -> Vec<u8> {
    let mut body = Vec::with_capacity(FRAME_PIXELS * 3);
    for value in &frame.depth_mm {
        body.extend_from_slice(&class_rgb(classify_depth_mm(*value, gate_mm())));
    }
    let mut bytes = ppm_header(frame.width, frame.height);
    bytes.extend_from_slice(&body);
    bytes
}

pub fn fault_csv(frame: &CameraFrame, fault: &FrameFault) -> String {
    let mut csv = String::from("index,channel,posted_mm,r,g,b,reason,depth_fresh,rgb_fresh\n");
    let depth_word = fresh_word(&frame.depth_fresh);
    let rgb_word = fresh_word(&frame.rgb_fresh);
    for index in &fault.depth_indexes {
        push_fault(
            &mut csv,
            frame,
            *index,
            "depth",
            "depth-dropout",
            depth_word,
            rgb_word,
        );
    }
    for index in &fault.rgb_indexes {
        push_fault(
            &mut csv,
            frame,
            *index,
            "rgb",
            "rgb-dropout",
            depth_word,
            rgb_word,
        );
    }
    csv
}

pub fn demo_frames() -> Result<FrameViews, SceneError> {
    let bodies = demonstration_bodies();
    let pose = crate::base::BasePose {
        x_m: 0.0,
        y_m: 0.0,
        yaw_deg: 0.0,
    };
    let measured_profile = profile("measured-mount");
    let configured_profile = profile("configured-mount");
    let started = Instant::now();
    let measured = cast_frame(CastQuery {
        pose,
        profile: measured_profile,
        obstacles: &bodies,
        plates: true,
    })?;
    let measured_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let configured = cast_frame(CastQuery {
        pose,
        profile: configured_profile,
        obstacles: &bodies,
        plates: true,
    })?;
    let configured_ms = started.elapsed().as_millis();
    let depth_index = frame_index(0, 120).expect("pixel");
    let rgb_index = frame_index(319, 120).expect("pixel");
    let fault = FrameFault {
        depth_indexes: vec![depth_index],
        rgb_indexes: vec![rgb_index],
        depth_fresh: Freshness::Stale { age_s: 2.0 },
        rgb_fresh: Freshness::Fresh,
    };
    let mut faulted = measured.clone();
    apply_frame_fault(&mut faulted, &fault).expect("demo fault");
    let fault_svg = draw_fault(&measured, &faulted, &fault);
    Ok(FrameViews {
        measured,
        configured,
        faulted,
        fault,
        measured_ms,
        configured_ms,
        model_csv: model_csv(),
        legend_svg: class_legend_svg(),
        fault_svg,
    })
}

fn profile(id: &str) -> crate::profile::CameraProfile {
    camera_profiles()
        .into_iter()
        .find(|profile| profile.id == id)
        .expect("camera profile")
}

fn gate_mm() -> u16 {
    (DEPTH_GATE_M * 1000.0).round() as u16
}

fn ppm_header(width: u32, height: u32) -> Vec<u8> {
    format!("P6\n{width} {height}\n255\n").into_bytes()
}

fn fresh_code(fresh: &Freshness) -> u8 {
    match fresh {
        Freshness::Fresh => 0,
        Freshness::Stale { .. } => 1,
        Freshness::Missing => 2,
        Freshness::Malformed => 3,
        Freshness::Reversed => 4,
    }
}

fn age_ms(fresh: &Freshness) -> u32 {
    match fresh {
        Freshness::Stale { age_s } if age_s.is_finite() && *age_s >= 0.0 => {
            let ms = age_s * 1000.0;
            if ms >= f64::from(u32::MAX) {
                u32::MAX
            } else {
                ms.round() as u32
            }
        }
        _ => 0,
    }
}

fn fresh_from(code: u8, age: u32) -> Result<Freshness, &'static str> {
    match code {
        0 if age == 0 => Ok(Freshness::Fresh),
        1 => Ok(Freshness::Stale {
            age_s: f64::from(age) / 1000.0,
        }),
        2 if age == 0 => Ok(Freshness::Missing),
        3 if age == 0 => Ok(Freshness::Malformed),
        4 if age == 0 => Ok(Freshness::Reversed),
        _ => Err("freshness"),
    }
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, &'static str> {
    let end = at + 4;
    Ok(u32::from_le_bytes(
        bytes
            .get(at..end)
            .ok_or("header")?
            .try_into()
            .map_err(|_| "header")?,
    ))
}

fn fresh_word(fresh: &Freshness) -> &'static str {
    match fresh {
        Freshness::Fresh => "fresh",
        Freshness::Stale { .. } => "stale",
        Freshness::Missing => "missing",
        Freshness::Malformed => "malformed",
        Freshness::Reversed => "reversed",
    }
}

fn push_fault(
    csv: &mut String,
    frame: &CameraFrame,
    index: usize,
    channel: &str,
    reason: &str,
    depth_fresh: &str,
    rgb_fresh: &str,
) {
    let at = index * 3;
    csv.push_str(&format!(
        "{index},{channel},{},{},{},{},{reason},{depth_fresh},{rgb_fresh}\n",
        frame.depth_mm[index],
        frame.rgb[at],
        frame.rgb[at + 1],
        frame.rgb[at + 2]
    ));
}

fn model_csv() -> String {
    "\
item,value,kind,source
width,320,cited device class,sensors.json model string. Not measured from a decoded PNG
height,240,cited device class,sensors.json model string and camera/index.json. Not measured from a decoded PNG
layout,row-major index = row * 320 + col,model convention,row 0 is the top row already used by cast_pixel. Not verified by opening camera.npz
pixel,center of the pixel,assumption,same center as the depth slice. The recorded optical row 153.6 is not applied
mirror,image right is robot left,cited,session README. Already applied inside cast_pixel
depth,little-endian u16 millimetres,this file,0 is no return and not a distance of 0 m. 1 to 499 is BelowGate. 500 and above is Valid
rgb,\"three bytes R G B, same index as depth\",assumed fixture palette,flat colour of the nearest solid. Not a photograph and not a class label
registration,same ray,construction,not a measured extrinsics calibration and not a verified shutter pair
stamp_ms,0,synthetic,not a capture clock. Freshness is a separate byte for each buffer
file,RCF1 version 1 little-endian,this file,not an npy and not a camera PNG
demo,both mounts and one fault example,limit,stage 4 is not complete and stage 5 is not this file
"
    .to_string()
}

fn class_legend_svg() -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1040\" height=\"220\" viewBox=\"0 0 1040 220\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><text x=\"24\" y=\"32\" font-size=\"16\">Posted class of the 320 by 240 depth buffer</text><text x=\"24\" y=\"58\" font-size=\"12\">The class picture is 320 by 240. This legend is outside that buffer. Colour here is the posted class, not metres and not the fixture colour.</text><text x=\"24\" y=\"80\" font-size=\"12\">Posted 0 is no return. It is not a distance of 0 m. Below gate is 1 to 499 mm and is not usable. Valid is at or above 500 mm.</text><text x=\"24\" y=\"102\" font-size=\"12\">{LIMIT_LINE}</text><rect data-legend=\"valid\" x=\"24\" y=\"130\" width=\"18\" height=\"14\" fill=\"#1d4e89\"/><text x=\"50\" y=\"142\" font-size=\"12\">valid, at or above 500 mm</text><rect data-legend=\"below-gate\" x=\"280\" y=\"130\" width=\"18\" height=\"14\" fill=\"#d97706\"/><text x=\"306\" y=\"142\" font-size=\"12\">below gate, 1 to 499 mm</text><rect data-legend=\"no-return\" x=\"560\" y=\"130\" width=\"18\" height=\"14\" fill=\"#bdbdbd\"/><text x=\"586\" y=\"142\" font-size=\"12\">no return, posted 0</text></svg>"
    )
}

fn draw_fault(before: &CameraFrame, after: &CameraFrame, fault: &FrameFault) -> String {
    let mut body = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1040\" height=\"280\" viewBox=\"0 0 1040 280\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><text x=\"24\" y=\"32\" font-size=\"16\">Fault on the synthetic frame</text><text x=\"24\" y=\"58\" font-size=\"12\">Depth dropout zeros the posted word and leaves the colour. RGB dropout zeros the colour and leaves the posted word. The same pixel is not on both lists.</text><text x=\"24\" y=\"80\" font-size=\"12\">The depth buffer is stale, so fresh valid depth is 0. The RGB buffer stays fresh. The stamp stays 0. This is not a device shutter pair.</text><text x=\"24\" y=\"102\" font-size=\"12\">",
    );
    body.push_str(LIMIT_LINE);
    body.push_str("</text>");
    body.push_str("<text x=\"24\" y=\"140\" font-size=\"12\">index</text><text x=\"110\" y=\"140\" font-size=\"12\">channel</text><text x=\"210\" y=\"140\" font-size=\"12\">posted mm</text><text x=\"330\" y=\"140\" font-size=\"12\">colour</text><text x=\"460\" y=\"140\" font-size=\"12\">reason</text>");
    let mut y = 168.0;
    for index in &fault.depth_indexes {
        y = push_fault_row(
            &mut body,
            before,
            after,
            *index,
            "depth",
            "depth-dropout",
            y,
        );
    }
    for index in &fault.rgb_indexes {
        y = push_fault_row(&mut body, before, after, *index, "rgb", "rgb-dropout", y);
    }
    body.push_str("</svg>");
    body
}

fn push_fault_row(
    body: &mut String,
    before: &CameraFrame,
    after: &CameraFrame,
    index: usize,
    channel: &str,
    reason: &str,
    y: f64,
) -> f64 {
    let posted = after.depth_mm[index];
    let ideal = before.depth_mm[index];
    let at = index * 3;
    let rgb = format!(
        "{},{},{}",
        after.rgb[at],
        after.rgb[at + 1],
        after.rgb[at + 2]
    );
    let ideal_posted = if channel == "depth" { "false" } else { "true" };
    body.push_str(&format!(
        "<g data-fault-index=\"{index}\" data-channel=\"{channel}\" data-posted-mm=\"{posted}\" data-ideal-mm=\"{ideal}\" data-ideal-posted=\"{ideal_posted}\" data-rgb=\"{rgb}\" data-reason=\"{reason}\" data-depth-fresh=\"stale\" data-rgb-fresh=\"fresh\"><text x=\"24\" y=\"{y:.1}\" font-size=\"12\">{index}</text><text x=\"110\" y=\"{y:.1}\" font-size=\"12\">{channel}</text><text x=\"210\" y=\"{y:.1}\" font-size=\"12\">{posted}</text><text x=\"330\" y=\"{y:.1}\" font-size=\"12\">{rgb}</text><text x=\"460\" y=\"{y:.1}\" font-size=\"12\">{reason}</text></g>"
    ));
    if channel == "depth" {
        body.push_str(&format!(
            "<text x=\"620\" y=\"{y:.1}\" font-size=\"12\">ideal {ideal} mm, not posted</text>"
        ));
    }
    y + 28.0
}

#[cfg(test)]
mod tests {
    use super::{
        FRAME_PIXELS, HEADER_LEN, LIMIT_LINE, apply_frame_fault, cast_frame, class_ppm, class_rgb,
        decode_frame, demo_frames, encode_frame, fault_csv, frame_index, material_rgb, rgb_ppm,
    };
    use crate::base::{BasePose, cast_sensor};
    use crate::camera::{CastQuery, IMAGE_HEIGHT, IMAGE_WIDTH, cast_pixel, demonstration_bodies};
    use crate::frame::{CameraFrame, FrameFault, FrameFaultError, depth_view};
    use crate::observe::{
        Actuation, DepthClass, Freshness, PersonPost, PostedScan, classify_depth_mm,
        controller_note,
    };
    use crate::profile::{CameraProfile, camera_profiles};
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

    fn sample(frame: &CameraFrame, col: u32, row: u32) -> (u16, [u8; 3]) {
        let index = frame_index(col, row).unwrap();
        let at = index * 3;
        (
            frame.depth_mm[index],
            [frame.rgb[at], frame.rgb[at + 1], frame.rgb[at + 2]],
        )
    }

    #[test]
    fn the_frame_matches_the_cast_at_center_edge_and_off_axis() {
        let profile = profile("configured-mount");
        let face = profile.x_m + 1.0;
        let bodies = [wall("wall", face, face + 0.2)];
        let asked = query(origin(), profile, &bodies, false);
        let frame = cast_frame(asked).unwrap();
        assert_eq!(frame.width, 320);
        assert_eq!(frame.height, 240);
        assert_eq!(frame.depth_mm.len(), FRAME_PIXELS);
        assert_eq!(frame.rgb.len(), FRAME_PIXELS * 3);
        for (col, row) in [(160, 120), (0, 120), (319, 120), (159, 0), (159, 239)] {
            let ray = cast_pixel(asked, col, row).unwrap();
            let (mm, rgb) = sample(&frame, col, row);
            assert_eq!(mm, ray.axial_mm);
            assert_eq!(rgb, material_rgb(ray.name));
        }
        let edge = cast_pixel(asked, 0, 120).unwrap();
        let range_mm = (edge.range_m.unwrap() * 1000.0).round() as u16;
        assert_ne!(range_mm, edge.axial_mm);
        assert_eq!(sample(&frame, 0, 120).0, edge.axial_mm);
        assert!((edge.axial_m.unwrap() - 1.0).abs() < 1e-6);
        let near_face = profile.x_m + 0.30;
        let near = [wall("near", near_face, near_face + 0.08)];
        let near_query = query(origin(), profile, &near, false);
        let near_frame = cast_frame(near_query).unwrap();
        let near_ray = cast_pixel(near_query, 160, 120).unwrap();
        assert_eq!(sample(&near_frame, 160, 120).0, near_ray.axial_mm);
        assert_eq!(near_ray.axial_mm, 300);
        assert_eq!(
            classify_depth_mm(near_ray.axial_mm, 500),
            DepthClass::BelowGate
        );
        assert_ne!(sample(&near_frame, 160, 120).1, [0, 0, 0]);
        assert_ne!(
            sample(&near_frame, 160, 120).1,
            class_rgb(DepthClass::BelowGate)
        );
        let gate_face = profile.x_m + 0.50;
        let gate = [wall("wall", gate_face, gate_face + 0.08)];
        let gate_query = query(origin(), profile, &gate, false);
        let gate_ray = cast_pixel(gate_query, 160, 120).unwrap();
        assert_eq!(gate_ray.axial_mm, 500);
        assert_eq!(
            sample(&cast_frame(gate_query).unwrap(), 160, 120).0,
            gate_ray.axial_mm
        );
        let miss = cast_frame(query(origin(), profile, &[], false)).unwrap();
        let missed = cast_pixel(query(origin(), profile, &[], false), 160, 120).unwrap();
        assert!(missed.axial_m.is_none());
        assert_eq!(sample(&miss, 160, 120), (0, [0, 0, 0]));
    }

    #[test]
    fn a_yawed_frame_matches_the_forward_cast() {
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
        let bodies = [ahead, aside];
        let asked = query(pose, profile, &bodies, false);
        let frame = cast_frame(asked).unwrap();
        let ray = cast_pixel(asked, 160, 120).unwrap();
        assert_eq!(ray.name, Some("ahead"));
        assert_eq!(sample(&frame, 160, 120).0, ray.axial_mm);
        assert_eq!(sample(&frame, 160, 120).1, material_rgb(Some("ahead")));
        let aside_only = [aside];
        let only = query(pose, profile, &aside_only, false);
        let missed = cast_frame(only).unwrap();
        assert!(cast_pixel(only, 160, 120).unwrap().axial_m.is_none());
        assert_eq!(sample(&missed, 160, 120).0, 0);
    }

    #[test]
    fn the_frame_keeps_occlusion_plates_and_the_low_box() {
        let configured = profile("configured-mount");
        let measured_mount = profile("measured-mount");
        let near = wall("near", 1.0, 1.05);
        let far = wall("far", 2.0, 2.05);
        let both = [near, far];
        let blocked_query = query(origin(), configured, &both, false);
        let blocked = cast_frame(blocked_query).unwrap();
        let blocked_ray = cast_pixel(blocked_query, 160, 120).unwrap();
        assert_eq!(blocked_ray.name, Some("near"));
        assert_eq!(sample(&blocked, 160, 120).0, blocked_ray.axial_mm);
        assert_eq!(sample(&blocked, 160, 120).1, material_rgb(Some("near")));
        let far_only = [far];
        let open_query = query(origin(), configured, &far_only, false);
        let open_ray = cast_pixel(open_query, 160, 120).unwrap();
        assert_eq!(open_ray.name, Some("far"));
        assert_ne!(open_ray.axial_mm, blocked_ray.axial_mm);
        assert_eq!(
            sample(&cast_frame(open_query).unwrap(), 160, 120).0,
            open_ray.axial_mm
        );
        let plated = query(origin(), configured, &far_only, true);
        let plated_ray = cast_pixel(plated, 160, 239).unwrap();
        assert_eq!(plated_ray.name, Some("top_plate"));
        let plated_frame = cast_frame(plated).unwrap();
        assert_eq!(sample(&plated_frame, 160, 239).0, plated_ray.axial_mm);
        assert_eq!(
            sample(&plated_frame, 160, 239).1,
            material_rgb(Some("top_plate"))
        );
        let bare = query(origin(), configured, &far_only, false);
        let bare_ray = cast_pixel(bare, 160, 239).unwrap();
        assert_eq!(bare_ray.name, Some("far"));
        assert_ne!(
            sample(&cast_frame(bare).unwrap(), 160, 239).1,
            material_rgb(Some("top_plate"))
        );
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
        assert!(
            cast_sensor(&origin(), "measured-front", 0.0, 0.0, &[low])
                .unwrap()
                .is_none()
        );
        let low_only = [low];
        let seen_query = query(origin(), measured_mount, &low_only, true);
        let seen = cast_pixel(seen_query, 160, 239).unwrap();
        assert_eq!(seen.name, Some("low-box"));
        let seen_frame = cast_frame(seen_query).unwrap();
        assert_eq!(sample(&seen_frame, 160, 239).0, seen.axial_mm);
        assert_eq!(
            sample(&seen_frame, 160, 239).1,
            material_rgb(Some("low-box"))
        );
        let behind = [wall("behind", -1.2, -0.4)];
        let hidden = query(origin(), configured, &behind, false);
        assert!(cast_pixel(hidden, 160, 120).unwrap().axial_m.is_none());
        assert_eq!(
            sample(&cast_frame(hidden).unwrap(), 160, 120),
            (0, [0, 0, 0])
        );
    }

    #[test]
    fn measured_and_configured_frames_follow_their_own_casts() {
        let bodies = demonstration_bodies();
        let pose = origin();
        let measured = profile("measured-mount");
        let configured = profile("configured-mount");
        let measured_query = query(pose, measured, &bodies, true);
        let configured_query = query(pose, configured, &bodies, true);
        let measured_frame = cast_frame(measured_query).unwrap();
        let configured_frame = cast_frame(configured_query).unwrap();
        let measured_ray = cast_pixel(measured_query, 160, 239).unwrap();
        let configured_ray = cast_pixel(configured_query, 160, 239).unwrap();
        assert_ne!(measured_ray.axial_mm, configured_ray.axial_mm);
        assert_eq!(sample(&measured_frame, 160, 239).0, measured_ray.axial_mm);
        assert_eq!(
            sample(&configured_frame, 160, 239).0,
            configured_ray.axial_mm
        );
        assert_eq!(
            sample(&measured_frame, 160, 239).1,
            material_rgb(measured_ray.name)
        );
        assert_eq!(
            sample(&configured_frame, 160, 239).1,
            material_rgb(configured_ray.name)
        );
        let bad = BasePose {
            x_m: f64::NAN,
            y_m: 0.0,
            yaw_deg: 0.0,
        };
        assert_eq!(
            cast_frame(query(bad, configured, &[], false)),
            Err(SceneError::NonFinite)
        );
    }

    #[test]
    fn the_buffer_is_row_major_and_not_a_stride_or_a_repeated_row() {
        let bodies = demonstration_bodies();
        let asked = query(origin(), profile("measured-mount"), &bodies, true);
        let frame = cast_frame(asked).unwrap();
        let left = cast_pixel(asked, 0, 120).unwrap();
        let right = cast_pixel(asked, 319, 120).unwrap();
        let down = cast_pixel(asked, 120, 0).unwrap();
        assert_ne!(left.axial_mm, right.axial_mm);
        assert_ne!(left.axial_mm, down.axial_mm);
        assert_eq!(frame.depth_mm[frame_index(0, 120).unwrap()], left.axial_mm);
        assert_eq!(
            frame.depth_mm[frame_index(319, 120).unwrap()],
            right.axial_mm
        );
        assert_eq!(frame.depth_mm[frame_index(120, 0).unwrap()], down.axial_mm);
        assert_eq!(frame_index(0, 1).unwrap(), IMAGE_WIDTH as usize);
        assert_eq!(frame_index(1, 0).unwrap(), 1);
        let row0 = cast_pixel(asked, 0, 0).unwrap();
        let row1 = cast_pixel(asked, 0, 1).unwrap();
        assert_eq!(frame.depth_mm[0], row0.axial_mm);
        assert_eq!(frame.depth_mm[IMAGE_WIDTH as usize], row1.axial_mm);
        assert_ne!(
            frame.depth_mm[0],
            frame.depth_mm[frame_index(0, 120).unwrap()]
        );
        let mut separated = false;
        for row in [1, 17, 33, 121] {
            for col in [1, 17, 33, 161] {
                let exact = cast_pixel(asked, col, row).unwrap();
                let coarse = cast_pixel(asked, col - col % 16, row - row % 16).unwrap();
                if exact.axial_mm != coarse.axial_mm {
                    assert_eq!(
                        frame.depth_mm[frame_index(col, row).unwrap()],
                        exact.axial_mm
                    );
                    separated = true;
                }
            }
        }
        assert!(separated);
        assert_eq!(sample(&frame, 0, 120).1, material_rgb(left.name));
        assert_ne!(sample(&frame, 0, 120).1, sample(&frame, 319, 120).1);
    }

    #[test]
    fn colour_follows_the_same_ray_as_the_depth_word() {
        let bodies = demonstration_bodies();
        let asked = query(origin(), profile("measured-mount"), &bodies, true);
        let frame = cast_frame(asked).unwrap();
        let ray = cast_pixel(asked, 0, 120).unwrap();
        let next = cast_pixel(asked, 319, 120).unwrap();
        assert_ne!(
            (ray.axial_mm, material_rgb(ray.name)),
            (next.axial_mm, material_rgb(next.name))
        );
        assert_eq!(sample(&frame, 0, 120).0, ray.axial_mm);
        assert_eq!(sample(&frame, 0, 120).1, material_rgb(ray.name));
        assert_eq!(sample(&frame, 319, 120).0, next.axial_mm);
        assert_eq!(sample(&frame, 319, 120).1, material_rgb(next.name));
        assert_ne!(material_rgb(Some("low-box")), class_rgb(DepthClass::Valid));
        assert_ne!(
            material_rgb(Some("low-box")),
            class_rgb(DepthClass::NoReturn)
        );
        let bytes = encode_frame(&frame).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("low-box"));
        assert!(!text.contains("top_plate"));
        assert!(!text.contains("far-wall"));
    }

    #[test]
    fn dropout_channels_stay_apart_and_the_posted_file_has_no_names() {
        let profile = profile("configured-mount");
        let face = profile.x_m + 1.0;
        let bodies = [wall("wall", face, face + 0.2)];
        let mut frame = cast_frame(query(origin(), profile, &bodies, false)).unwrap();
        let depth_index = frame_index(0, 120).unwrap();
        let rgb_index = frame_index(319, 120).unwrap();
        let before_depth = frame.depth_mm[depth_index];
        let before_rgb = sample(&frame, 319, 120).1;
        assert_ne!(before_depth, 0);
        assert_ne!(before_rgb, [0, 0, 0]);
        let fault = FrameFault {
            depth_indexes: vec![depth_index],
            rgb_indexes: vec![rgb_index],
            depth_fresh: Freshness::Fresh,
            rgb_fresh: Freshness::Fresh,
        };
        apply_frame_fault(&mut frame, &fault).unwrap();
        assert_eq!(frame.depth_mm[depth_index], 0);
        assert_eq!(sample(&frame, 0, 120).1, material_rgb(Some("wall")));
        assert_eq!(frame.depth_mm[rgb_index], before_depth);
        assert_eq!(sample(&frame, 319, 120).1, [0, 0, 0]);
        assert_eq!(frame.stamp_ms, 0);
        let csv = fault_csv(&frame, &fault);
        assert!(!csv.contains("wall"));
        assert!(!csv.contains("ideal"));
        assert!(csv.contains("depth-dropout"));
        assert!(csv.contains("rgb-dropout"));
        assert!(csv.contains(&format!("{depth_index},depth,0,")));
        assert!(csv.contains(&format!("{rgb_index},rgb,{before_depth},0,0,0,")));
        let mut overlap = frame.clone();
        let both = FrameFault {
            depth_indexes: vec![depth_index],
            rgb_indexes: vec![depth_index],
            depth_fresh: Freshness::Fresh,
            rgb_fresh: Freshness::Fresh,
        };
        assert_eq!(
            apply_frame_fault(&mut overlap, &both),
            Err(FrameFaultError::BothChannels)
        );
        assert_eq!(overlap.depth_mm[depth_index], frame.depth_mm[depth_index]);
        let mut wide = frame.clone();
        let past = FrameFault {
            depth_indexes: vec![FRAME_PIXELS],
            rgb_indexes: Vec::new(),
            depth_fresh: Freshness::Fresh,
            rgb_fresh: Freshness::Fresh,
        };
        assert_eq!(
            apply_frame_fault(&mut wide, &past),
            Err(FrameFaultError::OutOfRange)
        );
        let bytes = encode_frame(&frame).unwrap();
        assert_eq!(&bytes[0..4], b"RCF1");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 1);
        assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 320);
        assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 240);
        let pixel = HEADER_LEN + frame_index(160, 120).unwrap() * 2;
        assert_eq!(u16::from_le_bytes([bytes[pixel], bytes[pixel + 1]]), 1000);
        assert_eq!(bytes[pixel], 0xe8);
        assert_eq!(bytes[pixel + 1], 0x03);
        let decoded = decode_frame(&bytes).unwrap();
        assert_eq!(decoded, frame);
        let mut lied = bytes.clone();
        lied[6] = 1;
        lied[7] = 0;
        lied[8] = 0;
        lied[9] = 0;
        assert!(decode_frame(&lied).is_err());
        assert!(decode_frame(&bytes[..bytes.len() - 1]).is_err());
        let mut narrow = frame.clone();
        narrow.width = 16;
        assert!(encode_frame(&narrow).is_err());
        assert_eq!(IMAGE_HEIGHT, 240);
        assert_eq!(IMAGE_WIDTH, 320);
    }

    #[test]
    fn a_stale_depth_buffer_is_not_usable_and_the_stamp_stays() {
        let views = demo_frames().unwrap();
        let scan = PostedScan {
            freshness: Freshness::Fresh,
            samples: Vec::new(),
            source_name: "empty".to_string(),
        };
        let fresh = controller_note(&scan, Some(&depth_view(&views.measured)));
        assert!(fresh.fresh_valid_depth > 0);
        assert_eq!(fresh.retained_depth, FRAME_PIXELS);
        assert_eq!(fresh.person, PersonPost::NotInPostedView);
        assert_eq!(fresh.actuation, Actuation::NotSent);
        let stale = controller_note(&scan, Some(&depth_view(&views.faulted)));
        assert_eq!(stale.fresh_valid_depth, 0);
        assert_eq!(stale.retained_depth, FRAME_PIXELS);
        assert!(
            views
                .faulted
                .depth_mm
                .iter()
                .any(|value| classify_depth_mm(*value, 500) == DepthClass::Valid)
        );
        assert_eq!(views.faulted.stamp_ms, views.measured.stamp_ms);
        assert_eq!(views.faulted.stamp_ms, 0);
        assert_ne!(views.faulted.rgb, vec![0; FRAME_PIXELS * 3]);
        let mut rgb_stale = views.measured.clone();
        apply_frame_fault(
            &mut rgb_stale,
            &FrameFault {
                depth_indexes: Vec::new(),
                rgb_indexes: Vec::new(),
                depth_fresh: Freshness::Fresh,
                rgb_fresh: Freshness::Stale { age_s: 2.0 },
            },
        )
        .unwrap();
        let note = controller_note(&scan, Some(&depth_view(&rgb_stale)));
        assert_eq!(note.fresh_valid_depth, fresh.fresh_valid_depth);
        assert_eq!(rgb_stale.rgb, views.measured.rgb);
        assert_eq!(rgb_stale.stamp_ms, 0);
        let bytes = encode_frame(&views.faulted).unwrap();
        assert_eq!(bytes[14], 1);
        assert_eq!(bytes[15], 0);
        assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 2000);
        assert_eq!(
            decode_frame(&bytes).unwrap().depth_fresh,
            views.faulted.depth_fresh
        );
    }

    #[test]
    fn the_class_picture_uses_the_posted_class_and_stays_the_frame_size() {
        let views = demo_frames().unwrap();
        let header = b"P6\n320 240\n255\n";
        let classed = class_ppm(&views.measured);
        let coloured = rgb_ppm(&views.measured);
        assert!(classed.starts_with(header));
        assert!(coloured.starts_with(header));
        assert_eq!(classed.len(), header.len() + FRAME_PIXELS * 3);
        assert_eq!(coloured.len(), header.len() + FRAME_PIXELS * 3);
        let pixels = &classed[header.len()..];
        let mut seen = [false; 3];
        for (index, value) in views.measured.depth_mm.iter().enumerate() {
            let class = classify_depth_mm(*value, 500);
            let colour = match class {
                DepthClass::Valid => [0x1d, 0x4e, 0x89],
                DepthClass::BelowGate => [0xd9, 0x77, 0x06],
                DepthClass::NoReturn => [0xbd, 0xbd, 0xbd],
            };
            assert_eq!(&pixels[index * 3..index * 3 + 3], &colour);
            seen[match class {
                DepthClass::Valid => 0,
                DepthClass::BelowGate => 1,
                DepthClass::NoReturn => 2,
            }] = true;
        }
        assert_eq!(seen, [true, true, true]);
        assert_ne!(&coloured[header.len()..], pixels);
        assert!(views.legend_svg.contains(LIMIT_LINE));
        assert!(views.legend_svg.contains("data-legend=\"valid\""));
        assert!(views.legend_svg.contains("data-legend=\"below-gate\""));
        assert!(views.legend_svg.contains("data-legend=\"no-return\""));
        assert!(views.legend_svg.contains("not a distance of 0 m"));
        assert!(views.legend_svg.len() < 2000);
        assert!(!views.model_csv.contains("low-box"));
        assert!(views.model_csv.contains("row-major"));
        assert!(views.model_csv.contains("little-endian"));
        assert!(views.fault_svg.contains("data-ideal-posted=\"false\""));
        assert!(views.fault_svg.contains(LIMIT_LINE));
        assert!(!views.fault_svg.contains("low-box"));
        assert!(!views.fault_svg.contains("top_plate"));
        let depth_index = frame_index(0, 120).unwrap();
        assert!(views.fault_svg.contains(&format!(
            "data-fault-index=\"{depth_index}\" data-channel=\"depth\" data-posted-mm=\"0\""
        )));
        assert!(views.fault_svg.contains(&format!(
            "data-ideal-mm=\"{}\"",
            views.measured.depth_mm[depth_index]
        )));
        let csv = fault_csv(&views.faulted, &views.fault);
        assert!(!csv.contains("ideal"));
        assert!(!csv.contains(&views.measured.depth_mm[depth_index].to_string()));
    }
}
