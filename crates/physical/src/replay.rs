//! Headless replay. It reads a capture directory and writes a report. It sends nothing.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::arm::{self, identity};
use crate::chunk::parse_chunks;
use crate::command::{self, SCALE_SOURCE, format_command, wire_preview};
use crate::json::{self, Value};
use crate::kind::Kind;
use crate::lidar::{self, RawSample};
use crate::npy::{self, load_depth_u16};
use crate::observe::{DepthClass, Freshness, classify_depth_mm, sanitize_line, stamp_order};
use crate::plot::{self, Bar, XySeries};
use crate::profile::{
    self, DEPTH_GATE_M, DEPTH_GATE_SOURCE, FOV_SOURCE, LIDAR_XYZ_SOURCE, LIDAR_Z_M, MIRROR_SOURCE,
    MOTION_SOURCE, RANGE_SOURCE, SCENARIO_GAP_M, SCENARIO_GAP_SOURCE, base_xy, camera_profiles,
    floor_hit, footprint_width_m, front_face_width_m, lidar_misses_below, lidar_profiles,
    opening_blocks_chassis, yaw_disagreement_deg,
};
use crate::speed::{self, GATES, Projection, odom_delta, scan_forward, shift_angles};
use crate::zipnote::zip_entries;

const EARLY_PROBE_S: f64 = 2.0;
const LOW_EXAMPLE_Z_M: f64 = 0.10;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Message(text) => f.write_str(text),
            Error::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

pub fn write_replay(session: &Path, out: &Path, arm_urdf: Option<&Path>) -> Result<(), Error> {
    if !session.is_dir() {
        return Err(Error::Message("session directory is missing".to_string()));
    }
    std::fs::create_dir_all(out)?;
    let started = Instant::now();
    let mut report = String::new();
    report.push_str("# Physical foundation replay\n\n");
    report.push_str("Section 3.1 Carry My Luggage. This file is a posted-capture report, not a score and not a hardware result.\n");
    report.push_str(&format!(
        "Elapsed milliseconds are recorded at the end. Unix start {}.\n",
        unix_now()
    ));
    push_spec(&mut report);
    push_arm(&mut report, out, arm_urdf)?;
    push_robot_json(&mut report, &session.join("pi_config/robot.json"));
    let mut zero_bars = Vec::new();
    push_lidar_dir(
        &mut report,
        out,
        &session.join("raw_lidar"),
        "raw_lidar",
        &mut zero_bars,
    )?;
    push_depth(&mut report, out, &session.join("camera"))?;
    push_speed(&mut report, out, session)?;
    push_static(
        &mut report,
        out,
        &session.join("static_recording_pose9"),
        &mut zero_bars,
    )?;
    if !zero_bars.is_empty() {
        let svg = plot::bar_groups(
            "Zero-return fraction by file. Dataset: capture files named below. Unit: fraction of checksum-valid samples. Kind: measured.",
            "A zero range is kept in the count. It is not a dropped sample and not a filled range.",
            "zero fraction",
            &zero_bars
                .into_iter()
                .map(|bar| vec![bar])
                .collect::<Vec<_>>(),
        );
        std::fs::write(out.join("zero-fraction.svg"), svg)?;
        report.push_str("\nWrote zero-fraction.svg.\n");
    }
    push_logs(&mut report, out, &session.join("logs"))?;
    push_prims(&mut report, &session.join("experiments/prims6.json"));
    push_maps(&mut report, &session.join("maps"));
    push_npz(
        &mut report,
        &session.join("static_recording_pose9/camera.npz"),
    );
    report.push_str("\n## Limitations\n\n");
    report.push_str("This foundation does not render a 3D body, does not raycast a scene, does not run the section 3.1 controller on these scans, and does not send wheel or arm commands. PNG inventory scope is `camera/` files whose names end in `.png`. Those pixels are not decoded. `figures/` is outside this inventory and was not read. Deflate NPZ frames are blocked. Wheel radius and robot mass are unknown, so no rolling or contact claim is made. The SO-ARM100 mount, joint calibration, payload, and grasp are not validated. The two lidar fronts and the two camera mounts are both reported. Disagreeing reference-code scales are not adopted.\n");
    report.push_str(&format!(
        "\nElapsed milliseconds: {}.\n",
        started.elapsed().as_millis()
    ));
    std::fs::write(out.join("report.md"), report)?;
    Ok(())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn push_spec(report: &mut String) {
    report.push_str("\n## Frozen profiles\n\n");
    report.push_str("These profiles are not merged. Replay of `pi robot.json` below only checks the configured numbers against that file.\n\n");
    for profile in lidar_profiles() {
        report.push_str(&format!(
            "- lidar `{}` front raw {:.1} deg, {}, source: {}\n",
            profile.id,
            profile.front_raw_deg,
            profile.kind.as_str(),
            profile.source
        ));
    }
    report.push_str(&format!(
        "- lidar xyz ({:.3}, 0, {LIDAR_Z_M:.2}) m. {LIDAR_XYZ_SOURCE}\n",
        profile::LIDAR_X_M
    ));
    report.push_str(&format!("- range divisor {RANGE_SOURCE}\n"));
    report.push_str(&format!(
        "- yaw disagreement {:.4} deg, derived from the two fronts. Lateral separation at 1 m is {:.4} m, derived, formula range*sin(disagreement).\n",
        yaw_disagreement_deg(),
        profile::lateral_from_yaw_disagreement(1.0)
    ));
    for profile in camera_profiles() {
        let hit = floor_hit(profile);
        report.push_str(&format!(
            "- camera `{}` x {:.3} m z {:.3} m pitch down {:.1} deg, {}. Lower-ray floor range {}. Optical-axis floor range {}. Source: {}\n",
            profile.id,
            profile.x_m,
            profile.z_m,
            profile.pitch_down_deg,
            profile.kind.as_str(),
            opt_metres(hit.ground_range_m),
            opt_metres(hit.optical_ground_range_m),
            profile.source
        ));
    }
    report.push_str(&format!(
        "- horizontal fov {} deg, vertical fov {} deg. {FOV_SOURCE}\n",
        profile::HFOV_DEG,
        profile::VFOV_DEG
    ));
    report.push_str(&format!("- mirror: {MIRROR_SOURCE}\n"));
    report.push_str(&format!(
        "- depth gate {DEPTH_GATE_M} m. {DEPTH_GATE_SOURCE}\n"
    ));
    report.push_str(&format!(
        "- motion caps: {MOTION_SOURCE} Values {} m/s and {} deg/s.\n",
        profile::CONFIG_LINEAR_M_S,
        profile::CONFIG_ANGULAR_DEG_S
    ));
    let width = footprint_width_m();
    let front = front_face_width_m();
    report.push_str(&format!(
        "- footprint width {:.3} m and front-face width {:.3} m, derived from footprint.json. Opening {SCENARIO_GAP_M} m ({SCENARIO_GAP_SOURCE}) blocks the chassis: {} and blocks the front face: {}.\n",
        width,
        front,
        opening_blocks_chassis(SCENARIO_GAP_M, width),
        opening_blocks_chassis(SCENARIO_GAP_M, front)
    ));
    report.push_str(&format!(
        "- a top at {LOW_EXAMPLE_Z_M} m is a specified-scenario example, not a measured box. Lidar plane {LIDAR_Z_M} m misses it: {}.\n",
        lidar_misses_below(LOW_EXAMPLE_Z_M, LIDAR_Z_M)
    ));
    report.push_str("- wheel radius: unknown. robot mass: unknown. Wheel half-track 0.155 m is only a sentence in the private robot3d README; the driver source is not in the inspected packet, so it is not used.\n");
    report.push_str(&format!(
        "- reference-code scales not adopted: linear {}, angular {}, yaw {}, distance {}. {SCALE_SOURCE}\n",
        command::LINEAR_SCALE_NOT_ADOPTED,
        command::ANGULAR_SCALE_NOT_ADOPTED,
        command::YAW_SCALE_NOT_ADOPTED,
        command::DIST_SCALE_NOT_ADOPTED
    ));
}

fn push_arm(report: &mut String, out: &Path, arm_urdf: Option<&Path>) -> Result<(), Error> {
    let arm = identity();
    report.push_str("\n## Arm\n\n");
    report.push_str(&format!(
        "- identity {} from {}, commit `{}`. Geometry: {}. License: {}.\n",
        arm.name, arm.vendor, arm.commit, arm.geometry, arm.license
    ));
    report.push_str(&format!(
        "- mount to this robot base: {}. Joint limits when read from the URDF: {}. Grasp: not validated. Simulated gripper is this arm: {}.\n",
        arm.mount_to_base.as_str(),
        arm.joint_limits.as_str(),
        arm.simulated_gripper_is_this_arm
    ));
    report.push_str(&format!("Recorded URDF sha256 `{}`.\n", arm::URDF_SHA256));
    let Some(path) = arm_urdf else {
        report.push_str("No URDF path was passed. Joint table not loaded. Meshes stay out of this repository.\n");
        return Ok(());
    };
    match std::fs::read(path) {
        Ok(bytes) => {
            let read = arm::read_urdf(&bytes);
            report.push_str(&format!(
                "Loaded URDF bytes {}. Hash matches the recorded SO-ARM100 file: {}. Approximate-inertia phrase: {}. Link mass tags: {}. Mass status: {}.\n",
                bytes.len(),
                read.hash_matches_recorded,
                read.mentions_approximate_inertia,
                read.link_mass_count,
                read.mass_status
            ));
            let mut csv = String::from("name,lower,upper,effort,velocity,kind\n");
            for joint in &read.joints {
                csv.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    joint.name,
                    joint.lower,
                    joint.upper,
                    joint.effort,
                    joint.velocity,
                    joint.kind.as_str()
                ));
            }
            std::fs::write(out.join("arm-joints.csv"), csv)?;
            report.push_str("Wrote arm-joints.csv. Limits are upstream-nominal, not a safe real calibration. Grasp remains not validated.\n");
        }
        Err(err) => report.push_str(&format!("URDF unreadable: {err}.\n")),
    }
    Ok(())
}

fn push_robot_json(report: &mut String, path: &Path) {
    report.push_str("\n## Config file check\n\n");
    if !path.is_file() {
        report.push_str(
            "pi_config/robot.json is missing. Frozen configured numbers were not re-read.\n",
        );
        return;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        report.push_str("pi_config/robot.json is unreadable.\n");
        return;
    };
    match robot_numbers(&text) {
        Ok(numbers) => {
            report.push_str(&render_robot(&numbers));
            report.push_str("String fields in that file are not copied into this report.\n");
        }
        Err(err) => report.push_str(&format!("pi_config/robot.json malformed: {err}.\n")),
    }
}

struct RobotNumbers {
    lidar_yaw: f64,
    lidar_z: f64,
    camera_z: f64,
    camera_pitch: f64,
    half_width: f64,
    manual_linear: f64,
}

fn robot_numbers(text: &str) -> Result<RobotNumbers, String> {
    let value = json::parse(text)?;
    Ok(RobotNumbers {
        lidar_yaw: nested(&value, &["lidar", "yaw_deg"])?,
        lidar_z: nested(&value, &["lidar", "z_m"])?,
        camera_z: nested(&value, &["camera", "z_m"])?,
        camera_pitch: nested(&value, &["camera", "pitch_deg"])?,
        half_width: nested(&value, &["footprint", "half_width_m"])?,
        manual_linear: nested(&value, &["motion", "manual_linear_mps"])?,
    })
}

fn nested(value: &Value, path: &[&str]) -> Result<f64, String> {
    let mut current = value;
    for key in path {
        current = current.get(key).ok_or_else(|| format!("missing {key}"))?;
    }
    current
        .as_f64()
        .ok_or_else(|| format!("missing {}", path.last().unwrap_or(&"value")))
}

fn render_robot(numbers: &RobotNumbers) -> String {
    let configured = lidar_profiles()[1].front_raw_deg;
    let measured = lidar_profiles()[0].front_raw_deg;
    format!(
        "File lidar yaw {yaw:.4} deg matches configured {configured:.4}: {cy}. Matches measured {measured:.4}: {my}. Neither result selects a profile.\nFile lidar z {z:.4} m, camera z {cz:.4} m, camera pitch {pitch:.4} deg, footprint half width {half:.4} m, manual linear cap {linear:.4} m/s. Kind: configured, re-read from the file.\n",
        yaw = numbers.lidar_yaw,
        cy = (numbers.lidar_yaw - configured).abs() < 1e-9,
        my = (numbers.lidar_yaw - measured).abs() < 1e-6,
        z = numbers.lidar_z,
        cz = numbers.camera_z,
        pitch = numbers.camera_pitch,
        half = numbers.half_width,
        linear = numbers.manual_linear,
    )
}

struct LidarSummary {
    bytes: usize,
    samples: usize,
    zeros: usize,
    packets_ok: usize,
    rejects: usize,
    tail: usize,
    revolutions: usize,
}

fn push_lidar_dir(
    report: &mut String,
    out: &Path,
    dir: &Path,
    label: &str,
    bars: &mut Vec<Bar>,
) -> Result<(), Error> {
    report.push_str(&format!("\n## Lidar directory `{label}`\n\n"));
    if !dir.is_dir() {
        report.push_str("Directory missing.\n");
        return Ok(());
    }
    let mut files = list_files(dir, ".bin")?;
    files.retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("bin"));
    if files.is_empty() {
        report.push_str("No .bin files.\n");
        return Ok(());
    }
    let mut summaries = Vec::new();
    let mut plot_name = String::new();
    let mut plot_samples: Vec<RawSample> = Vec::new();
    let mut plot_count = 0_usize;
    let mut locked = false;
    let mut csv = String::from(
        "file,bytes,samples,zero_returns,packets_ok,rejects,unparsed_tail,revolutions\n",
    );
    for path in files {
        let name = file_name(&path);
        let bytes = std::fs::read(&path)?;
        let parsed = lidar::parse_lidar(&bytes);
        let zeros = parsed
            .samples
            .iter()
            .filter(|sample| sample.range_m == 0.0)
            .count();
        let revolutions = lidar::split_revolutions(&parsed.samples);
        let summary = LidarSummary {
            bytes: bytes.len(),
            samples: parsed.samples.len(),
            zeros,
            packets_ok: parsed.packets_ok,
            rejects: parsed.rejects.len(),
            tail: parsed.unparsed_tail,
            revolutions: revolutions.len(),
        };
        csv.push_str(&format!(
            "{name},{},{},{zeros},{},{},{},{}\n",
            summary.bytes,
            summary.samples,
            summary.packets_ok,
            summary.rejects,
            summary.tail,
            summary.revolutions
        ));
        if summary.samples > 0 {
            bars.push(Bar {
                label: name.clone(),
                value: zeros as f64 / summary.samples as f64,
                whisker: 0.0,
                color: "#1f4e79",
                kind: "measured",
            });
        }
        let longest = revolutions.iter().max_by_key(|rev| rev.len()).copied();
        let take = name == "course_lidar.bin" || (!locked && summary.samples > plot_count);
        if take {
            if let Some(longest) = longest {
                plot_name = name.clone();
                plot_samples = longest.to_vec();
                plot_count = summary.samples;
            }
            locked = name == "course_lidar.bin";
        }
        report.push_str(&format!(
            "- `{name}` bytes {} samples {} zeros {} packets {} rejects {} tail {} revolutions {}\n",
            summary.bytes, summary.samples, summary.zeros, summary.packets_ok, summary.rejects, summary.tail, summary.revolutions
        ));
        summaries.push(summary);
    }
    std::fs::write(out.join(format!("{label}-counts.csv")), csv)?;
    if !plot_samples.is_empty() {
        write_scan_plot(report, out, &plot_name, &plot_samples)?;
    }
    let _ = summaries;
    Ok(())
}

fn write_scan_plot(
    report: &mut String,
    out: &Path,
    name: &str,
    samples: &[RawSample],
) -> Result<(), Error> {
    let measured = lidar_profiles()[0].front_raw_deg;
    let configured = lidar_profiles()[1].front_raw_deg;
    let mut csv = String::from(
        "angle_raw_deg,range_m_protocol,x_m_measured_front,y_m_measured_front,x_m_configured_front,y_m_configured_front\n",
    );
    let mut measured_pts = Vec::new();
    let mut configured_pts = Vec::new();
    let mut zeros = 0;
    for sample in samples {
        let (xm, ym) = base_xy(sample.raw_deg, sample.range_m, measured);
        let (xc, yc) = base_xy(sample.raw_deg, sample.range_m, configured);
        csv.push_str(&format!(
            "{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
            sample.raw_deg, sample.range_m, xm, ym, xc, yc
        ));
        if sample.range_m == 0.0 {
            zeros += 1;
        } else {
            measured_pts.push((xm, ym));
            configured_pts.push((xc, yc));
        }
    }
    std::fs::write(out.join("scan-longest.csv"), csv)?;
    let svg = plot::robot_plan(
        &format!("Longest revolution in `{name}`. Dataset: that file. x forward, y left, metres."),
        &format!(
            "Red is the measured front {measured} deg. Blue is the configured front {configured} deg. Ranges use the protocol divisor and are not tape-calibrated. Zero returns in this revolution: {zeros}. They are in the CSV and are not drawn as hits."
        ),
        &[
            XySeries {
                name: format!("measured front {measured} deg"),
                kind: "measured-front",
                color: "#c0392b",
                points: measured_pts,
            },
            XySeries {
                name: format!("configured front {configured} deg"),
                kind: "configured-front",
                color: "#2471a3",
                points: configured_pts,
            },
        ],
    );
    std::fs::write(out.join("scan-longest.svg"), svg)?;
    report.push_str(&format!(
        "Drawing uses the longest revolution of `{name}` when that rule selects it. Wrote scan-longest.csv and scan-longest.svg. Counts above use every sample, not only the drawn revolution.\n"
    ));
    Ok(())
}

fn push_depth(report: &mut String, out: &Path, dir: &Path) -> Result<(), Error> {
    report.push_str("\n## Depth frames\n\n");
    let gate_mm = (DEPTH_GATE_M * 1000.0).round() as u16;
    if !dir.is_dir() {
        report.push_str("camera directory missing.\n");
    } else {
        let mut csv = String::from(
            "file,width,height,no_return,below_gate,valid,min_valid_mm,max_valid_mm,gate_mm,gate_kind\n",
        );
        let mut groups = Vec::new();
        for path in list_files(dir, "_depth_mm.npy")? {
            let name = file_name(&path);
            let bytes = std::fs::read(&path)?;
            match load_depth_u16(&bytes) {
                Ok(frame) => {
                    let mut no_return = 0;
                    let mut below = 0;
                    let mut valid = 0;
                    let mut min_valid = u16::MAX;
                    let mut max_valid = 0_u16;
                    for mm in &frame.mm {
                        match classify_depth_mm(*mm, gate_mm) {
                            DepthClass::NoReturn => no_return += 1,
                            DepthClass::BelowGate => below += 1,
                            DepthClass::Valid => {
                                valid += 1;
                                min_valid = min_valid.min(*mm);
                                max_valid = max_valid.max(*mm);
                            }
                        }
                    }
                    let min_text = if valid == 0 {
                        String::new()
                    } else {
                        min_valid.to_string()
                    };
                    let max_text = if valid == 0 {
                        String::new()
                    } else {
                        max_valid.to_string()
                    };
                    csv.push_str(&format!(
                    "{name},{},{},{no_return},{below},{valid},{min_text},{max_text},{gate_mm},{}\n",
                    frame.info.shape[1],
                    frame.info.shape[0],
                    Kind::Configured.as_str()
                ));
                    let total = frame.mm.len().max(1) as f64;
                    groups.push(vec![
                        Bar {
                            label: name.clone(),
                            value: no_return as f64 / total,
                            whisker: 0.0,
                            color: "#7f8c8d",
                            kind: "measured no-return",
                        },
                        Bar {
                            label: name.clone(),
                            value: below as f64 / total,
                            whisker: 0.0,
                            color: "#b9770e",
                            kind: "measured below configured gate",
                        },
                    ]);
                    report.push_str(&format!(
                    "- `{name}` {width}x{height} no-return {no_return} below-gate {below} valid {valid}. Zeros stay in the frame.\n",
                    width = frame.info.shape[1],
                    height = frame.info.shape[0]
                ));
                }
                Err(err) => report.push_str(&format!("- `{name}` not loaded: {err}.\n")),
            }
        }
        std::fs::write(out.join("depth.csv"), csv)?;
        if !groups.is_empty() {
            let svg = plot::bar_groups(
                "Depth validity by file. Dataset: *_depth_mm.npy. Unit: fraction of pixels. Grey no-return is measured. Amber below-gate uses the configured 500 mm gate.",
                "Every pixel is retained. No-return and below-gate are different classes.",
                "fraction of pixels",
                &groups,
            );
            std::fs::write(out.join("depth.svg"), svg)?;
        }
        for path in list_files(dir, ".png")? {
            let bytes = std::fs::read(&path)?;
            report.push_str(&format!(
                "- `{}` PNG {} bytes. Pixel decode is blocked in this milestone.\n",
                file_name(&path),
                bytes.len()
            ));
        }
    }
    report.push_str("PNG inventory scope is `camera/` files whose names end in `.png`. Those bytes are counted and pixels are not decoded. `figures/` is outside this inventory and was not read.\n");
    let mut notes = Vec::new();
    for profile in camera_profiles() {
        let hit = floor_hit(profile);
        notes.push(format!(
            "{} lower-ray {} optical {}",
            profile.id,
            opt_metres(hit.ground_range_m),
            opt_metres(hit.optical_ground_range_m)
        ));
        notes.push(format!(
            "{} z={:.3} m pitch down {:.1} deg",
            profile.id, profile.z_m, profile.pitch_down_deg
        ));
    }
    notes.push(format!(
        "lidar plane {LIDAR_Z_M:.2} m misses a {LOW_EXAMPLE_Z_M:.2} m top: {}",
        lidar_misses_below(LOW_EXAMPLE_Z_M, LIDAR_Z_M)
    ));
    notes.push(format!(
        "opening {SCENARIO_GAP_M:.2} m blocks chassis width {:.3} m: {}",
        footprint_width_m(),
        opening_blocks_chassis(SCENARIO_GAP_M, footprint_width_m())
    ));
    notes.push("not a photograph and not a mesh".to_string());
    let svg = plot::frame_schematic(
        "Frame inequalities. Dataset: frozen profiles. Units: metres. Kinds are on each line.",
        "Configured and measured mounts are both shown. The 0.28 m opening is a specified scenario, not a measured gap.",
        &notes,
    );
    std::fs::write(out.join("frame-geometry.svg"), svg)?;
    report.push_str("Wrote depth.csv, depth.svg when frames loaded, and frame-geometry.svg.\n");
    Ok(())
}

fn push_speed(report: &mut String, out: &Path, session: &Path) -> Result<(), Error> {
    report.push_str("\n## Speed runs\n\n");
    report.push_str("JSON angles are already wrap(measured front 178.7 deg - raw clockwise). Adding the frozen yaw disagreement reconstructs configured-front angles for the same ranges. That reconstruction is unselected. The shifted column uses it and is not an adopted speed estimate, not a correction, and not a change to the stored angles or to the reference scales. Odometry units are not named in the file. The reference code treats the third value as radians and hypot(dx, dy) as a length. DIST_SCALE is not applied to the main columns.\n\n");
    report.push_str("Two scan models are both derived. Ray projection uses (range before - range after) / cos(angle). It matches a small move when the same bearing still sees the same point. Frontal plane uses (range before - range after) * cos(angle). It matches a wall perpendicular to the forward axis. The median absolute deviation does not include the disagreement between the models. Neither model is selected.\n\n");
    let mut csv = String::from(
        "file,index,request_m_s,duration_s,commanded_distance_m,odom_dx,odom_dy,odom_dyaw,odom_chord,odom_forward,odom_left,chord_if_dist_scale_not_adopted,wire_not_sent,scan_gate,projection,scan_median_m,scan_mad_m,scan_pairs,unselected_configured_front_mid_median_m\n",
    );
    let mut groups = Vec::new();
    let mut plotted = Vec::new();
    for name in ["speed_test.json", "speed_test2.json"] {
        let path = session.join("experiments").join(name);
        if !path.is_file() {
            report.push_str(&format!("`{name}` missing.\n"));
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        match speed::parse_speed_runs(&text) {
            Err(err) => report.push_str(&format!(
                "`{name}` malformed: {err}. No speed was invented.\n"
            )),
            Ok(runs) => {
                if !runs.is_empty() {
                    plotted.push(name);
                }
                for (index, run) in runs.iter().enumerate() {
                    let commanded = run.request_m_s * run.duration_s;
                    let delta = odom_delta(run.odom0, run.odom1);
                    let wire = wire_preview(run.request_m_s, 0.0);
                    let shifted = shift_angles(&run.before, yaw_disagreement_deg());
                    let shifted_after = shift_angles(&run.after, yaw_disagreement_deg());
                    report.push_str(&format!(
                        "- `{name}` run {index} request {:.4} m/s for {:.4} s, commanded distance {:.4} m (derived). Wire preview `{wire}` is not sent. Unscaled text would be `{}`.\n",
                        run.request_m_s,
                        run.duration_s,
                        commanded,
                        format_command(run.request_m_s, 0.0)
                    ));
                    if let Some(delta) = delta {
                        report.push_str(&format!(
                            "  odom raw dx {:.4} dy {:.4} dyaw {:.4} chord {:.4} forward {:.4} left {:.4}. Forward uses the start yaw and is inferred, not measured tape.\n",
                            delta.dx, delta.dy, delta.dyaw, delta.chord, delta.forward_at_start_yaw, delta.left_at_start_yaw
                        ));
                    }
                    let mut bars = vec![Bar {
                        label: format!("{name}:{index}"),
                        value: commanded,
                        whisker: 0.0,
                        color: "#1a5276",
                        kind: "derived commanded distance",
                    }];
                    if let Some(delta) = delta {
                        bars.push(Bar {
                            label: format!("{name}:{index}"),
                            value: delta.chord,
                            whisker: 0.0,
                            color: "#117a65",
                            kind: "derived raw odom chord",
                        });
                    }
                    for projection in [Projection::Ray, Projection::Plane] {
                        let shifted_mid =
                            scan_forward(&shifted, &shifted_after, GATES[1], projection);
                        for gate in GATES {
                            let estimate = scan_forward(&run.before, &run.after, gate, projection);
                            let (median, mad, pairs) = estimate
                                .as_ref()
                                .map(|item| {
                                    (
                                        format!("{:.6}", item.median_m),
                                        format!("{:.6}", item.mad_m),
                                        item.pairs.to_string(),
                                    )
                                })
                                .unwrap_or_else(|| (String::new(), String::new(), "0".to_string()));
                            if gate.name == GATES[1].name
                                && let Some(item) = &estimate
                            {
                                let (color, kind) = match projection {
                                    Projection::Ray => {
                                        ("#b9770e", "derived ray-projection, assumed gate")
                                    }
                                    Projection::Plane => {
                                        ("#922b21", "derived frontal-plane, assumed gate")
                                    }
                                };
                                bars.push(Bar {
                                    label: format!("{name}:{index}"),
                                    value: item.median_m,
                                    whisker: item.mad_m,
                                    color,
                                    kind,
                                });
                            }
                            let shifted_text = if gate.name == GATES[1].name {
                                shifted_mid
                                    .as_ref()
                                    .map(|item| format!("{:.6}", item.median_m))
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            let (dx, dy, dyaw, chord, forward, left, scaled) = delta
                                .map(|item| {
                                    (
                                        format!("{:.6}", item.dx),
                                        format!("{:.6}", item.dy),
                                        format!("{:.6}", item.dyaw),
                                        format!("{:.6}", item.chord),
                                        format!("{:.6}", item.forward_at_start_yaw),
                                        format!("{:.6}", item.left_at_start_yaw),
                                        format!(
                                            "{:.6}",
                                            item.chord * command::DIST_SCALE_NOT_ADOPTED
                                        ),
                                    )
                                })
                                .unwrap_or_default();
                            csv.push_str(&format!(
                            "{name},{index},{:.6},{:.6},{:.6},{dx},{dy},{dyaw},{chord},{forward},{left},{scaled},{wire},{},{},{median},{mad},{pairs},{shifted_text}\n",
                            run.request_m_s, run.duration_s, commanded, gate.name, projection.as_str()
                        ));
                            if let Some(item) = estimate {
                                report.push_str(&format!(
                                "  scan gate `{}` projection `{}` median {:.4} m mad {:.4} m pairs {}. Gate is assumed correspondence, not a calibration.\n",
                                item.gate, projection.as_str(), item.median_m, item.mad_m, item.pairs
                            ));
                            } else {
                                report.push_str(&format!(
                                "  scan gate `{}` projection `{}` has fewer than 8 pairs. No speed was invented.\n",
                                gate.name,
                                projection.as_str()
                            ));
                            }
                        }
                        if let Some(item) = shifted_mid {
                            report.push_str(&format!(
                            "  unselected configured-front reconstruction, projection `{}` mid gate median {:.4} m. Same ranges. Not an adopted speed estimate.\n",
                            projection.as_str(),
                            item.median_m
                        ));
                        }
                    }
                    groups.push(bars);
                }
            }
        }
    }
    std::fs::write(out.join("speed.csv"), csv)?;
    if !groups.is_empty() {
        let dataset = if plotted.is_empty() {
            "no plotted speed file".to_string()
        } else {
            plotted.join(" and ")
        };
        let svg = plot::bar_groups(
            &format!(
                "Commanded distance, raw odom chord, and two mid-gate scan models. Dataset: {dataset}. Unit: metres."
            ),
            "Blue commanded is request times recorded duration. Green chord is raw odometry. Amber is ray projection. Dark red is a frontal plane. Whisker is the median absolute deviation of one model. Scales in wheels.py are not applied.",
            "metres",
            &groups,
        );
        std::fs::write(out.join("speed.svg"), svg)?;
    }
    let mut sensitivity = String::from("range_m,lateral_m,kind\n");
    let mut points = Vec::new();
    for range in [1.0_f64, 2.0, 4.0] {
        let lateral = profile::lateral_from_yaw_disagreement(range);
        sensitivity.push_str(&format!("{range:.1},{lateral:.6},derived\n"));
        points.push((range, lateral));
    }
    std::fs::write(out.join("yaw-sensitivity.csv"), sensitivity)?;
    let svg = plot::xy_plot(
        "Lateral separation of the two lidar fronts. Dataset: frozen 178.7 deg and 183.5 deg. Unit: metres. Kind: derived.",
        "Formula range * sin(configured front - measured front). This is not a calibration and not a selected yaw.",
        "range metres",
        "lateral metres",
        &[XySeries {
            name: "yaw disagreement".to_string(),
            kind: "derived",
            color: "#6c3483",
            points,
        }],
    );
    std::fs::write(out.join("yaw-sensitivity.svg"), svg)?;
    report.push_str("Wrote speed.csv, speed.svg, yaw-sensitivity.csv, and yaw-sensitivity.svg.\n");
    Ok(())
}

fn push_static(
    report: &mut String,
    out: &Path,
    dir: &Path,
    bars: &mut Vec<Bar>,
) -> Result<(), Error> {
    report.push_str("\n## Static recording\n\n");
    if !dir.is_dir() {
        report.push_str("static_recording_pose9 missing.\n");
        return Ok(());
    }
    let raw_path = dir.join("lidar_raw.bin");
    let chunk_path = dir.join("lidar_chunks.bin");
    let raw = if raw_path.is_file() {
        Some(std::fs::read(&raw_path)?)
    } else {
        None
    };
    let chunk_bytes = if chunk_path.is_file() {
        Some(std::fs::read(&chunk_path)?)
    } else {
        None
    };
    if let Some(raw) = &raw {
        let parsed = lidar::parse_lidar(raw);
        let zeros = parsed
            .samples
            .iter()
            .filter(|sample| sample.range_m == 0.0)
            .count();
        report.push_str(&format!(
            "lidar_raw.bin bytes {} samples {} zeros {} packets {} rejects {} tail {} revolutions {}.\n",
            raw.len(),
            parsed.samples.len(),
            zeros,
            parsed.packets_ok,
            parsed.rejects.len(),
            parsed.unparsed_tail,
            lidar::split_revolutions(&parsed.samples).len()
        ));
        if !parsed.samples.is_empty() {
            bars.push(Bar {
                label: "static-lidar_raw.bin".to_string(),
                value: zeros as f64 / parsed.samples.len() as f64,
                whisker: 0.0,
                color: "#1f4e79",
                kind: "measured",
            });
        }
    } else {
        report.push_str("lidar_raw.bin missing.\n");
    }
    if let Some(bytes) = &chunk_bytes {
        let parsed = parse_chunks(bytes);
        let times: Vec<f64> = parsed.chunks.iter().map(|chunk| chunk.t_s).collect();
        let order = stamp_order(&times);
        let mut concat = Vec::new();
        let mut spans = Vec::new();
        for chunk in &parsed.chunks {
            concat.extend_from_slice(&chunk.bytes);
            spans.push((concat.len(), chunk.t_s));
        }
        let same = raw.as_ref().is_some_and(|raw| raw == &concat);
        report.push_str(&format!(
            "lidar_chunks.bin chunks {} stamp order {order:?} payload bytes {} equal to lidar_raw.bin: {same}.",
            parsed.chunks.len(),
            concat.len()
        ));
        if let Some(err) = &parsed.error {
            report.push_str(&format!(" Chunk tail error: {err}."));
        }
        report.push('\n');
        if order == Freshness::Fresh {
            let lidar = lidar::parse_lidar(&concat);
            let revolutions = lidar::split_revolutions(&lidar.samples);
            let mut starts = Vec::new();
            for revolution in &revolutions {
                if let Some(sample) = revolution.first()
                    && let Some(time) = time_at(&spans, sample.at)
                {
                    starts.push(time);
                }
            }
            let mut periods = Vec::new();
            for pair in starts.windows(2) {
                let dt = pair[1] - pair[0];
                if dt > 1e-6 {
                    periods.push(dt);
                }
            }
            if let Some(median) = median(&mut periods) {
                report.push_str(&format!(
                    "Median positive revolution period {median:.6} s, derived rate {:.4} Hz. This is not a stored constant.\n",
                    1.0 / median
                ));
            } else {
                report.push_str("Not enough positive revolution periods to derive a rate.\n");
            }
            if let Some(first) = times.first() {
                let mut early_zero = 0;
                let mut early_n = 0;
                let mut late_zero = 0;
                let mut late_n = 0;
                for sample in &lidar.samples {
                    let Some(time) = time_at(&spans, sample.at) else {
                        continue;
                    };
                    let zero = sample.range_m == 0.0;
                    if time <= first + EARLY_PROBE_S {
                        early_n += 1;
                        early_zero += usize::from(zero);
                    } else {
                        late_n += 1;
                        late_zero += usize::from(zero);
                    }
                }
                report.push_str(&format!(
                    "Early probe {EARLY_PROBE_S:.1} s is a split chosen to test a session note, not a sensor constant. Early zeros {early_zero}/{early_n}. Later zeros {late_zero}/{late_n}.\n"
                ));
            }
        } else {
            report.push_str(
                "Stamp order is reversed or unusable. Revolution rate is not computed.\n",
            );
        }
    } else {
        report.push_str("lidar_chunks.bin missing.\n");
    }
    let meta = dir.join("meta.json");
    if meta.is_file() {
        let text = std::fs::read_to_string(&meta)?;
        match meta_summary(&text) {
            Ok(summary) => report.push_str(&summary),
            Err(err) => report.push_str(&format!("meta.json malformed: {err}.\n")),
        }
    }
    let _ = out;
    Ok(())
}

fn meta_summary(text: &str) -> Result<String, String> {
    let value = json::parse(text)?;
    let rows = value
        .get("odometry")
        .and_then(Value::as_array)
        .ok_or_else(|| "odometry".to_string())?;
    let mut poses = Vec::new();
    for row in rows {
        let pair = row.as_array().ok_or_else(|| "odometry row".to_string())?;
        let pose = pair
            .get(1)
            .and_then(Value::as_array)
            .ok_or_else(|| "pose".to_string())?;
        if pose.len() != 3 {
            return Err("pose".to_string());
        }
        let mut numbers = [0.0; 3];
        for (index, item) in pose.iter().enumerate() {
            numbers[index] = item.as_f64().ok_or_else(|| "pose value".to_string())?;
        }
        poses.push(numbers);
    }
    let mut max_step = 0.0_f64;
    for pair in poses.windows(2) {
        let step = (pair[1][0] - pair[0][0]).abs()
            + (pair[1][1] - pair[0][1]).abs()
            + (pair[1][2] - pair[0][2]).abs();
        max_step = max_step.max(step);
    }
    let duration = value.get("duration_s").and_then(Value::as_f64);
    Ok(format!(
        "meta.json odometry rows {}. Max absolute pose step {:.6}. Duration field {}. Idle means the step is zero. The pose values are recorded odometry, not a map name.\n",
        poses.len(),
        max_step,
        duration_text(duration)
    ))
}

fn time_at(spans: &[(usize, f64)], offset: usize) -> Option<f64> {
    spans
        .iter()
        .find(|(end, _)| offset < *end)
        .map(|(_, time)| *time)
}

fn push_logs(report: &mut String, out: &Path, dir: &Path) -> Result<(), Error> {
    report.push_str("\n## Captured logs\n\n");
    report.push_str("Every transcribed command has sent=false. Address-like lines are omitted.\n");
    if !dir.is_dir() {
        report.push_str("logs directory missing.\n");
        return Ok(());
    }
    let mut csv = String::from("file,line,text,sent\n");
    for path in list_files(dir, ".log")? {
        let name = file_name(&path);
        let text = std::fs::read_to_string(&path)?;
        let mut stamps = Vec::new();
        let mut lost = 0;
        for (index, line) in text.lines().enumerate() {
            let clean = sanitize_line(line);
            if clean.to_ascii_lowercase().contains("lost") {
                lost += 1;
            }
            if let Some(value) = stamp_in_line(&clean) {
                stamps.push(value);
            }
            csv.push_str(&format!(
                "{name},{},{},false\n",
                index + 1,
                csv_field(&clean)
            ));
        }
        let mut deltas = stamps
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|dt| *dt > 0.0)
            .collect::<Vec<_>>();
        report.push_str(&format!(
            "- `{name}` lines transcribed, lines containing the captured word lost: {lost}. That word is captured text, not proof a person ceased to exist."
        ));
        if let Some(median_dt) = median(&mut deltas) {
            let min = deltas.iter().copied().fold(f64::MAX, f64::min);
            let max = deltas.iter().copied().fold(f64::MIN, f64::max);
            report.push_str(&format!(
                ", positive stamp deltas {} median {median_dt:.4} s min {min:.4} s max {max:.4} s. Derived from this file only, not a controller constant.",
                deltas.len()
            ));
        }
        report.push('\n');
    }
    std::fs::write(out.join("commands.csv"), csv)?;
    report.push_str("Wrote commands.csv. sent is false on every row.\n");
    Ok(())
}

fn duration_text(value: Option<f64>) -> String {
    match value {
        Some(seconds) if seconds.is_finite() => format!("{seconds:.1} s"),
        _ => "missing".to_string(),
    }
}

fn opt_metres(value: Option<f64>) -> String {
    match value {
        Some(metres) => format!("{metres:.4} m"),
        None => "no floor hit".to_string(),
    }
}

fn stamp_in_line(line: &str) -> Option<f64> {
    let rest = line.split("t=").nth(1)?;
    let token: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == '-')
        .collect();
    let value = token.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn push_prims(report: &mut String, path: &Path) {
    report.push_str("\n## Drive primitives\n\n");
    if !path.is_file() {
        report.push_str("prims6.json missing.\n");
        return;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        report.push_str("prims6.json unreadable.\n");
        return;
    };
    match prim_summary(&text) {
        Ok(summary) => report.push_str(&summary),
        Err(err) => report.push_str(&format!("prims6.json malformed: {err}.\n")),
    }
}

fn prim_summary(text: &str) -> Result<String, String> {
    let value = json::parse(text)?;
    let prims = value
        .get("prims")
        .and_then(Value::as_array)
        .ok_or_else(|| "prims".to_string())?;
    let mut drives = 0;
    let mut turns = 0;
    let mut drive_sum = 0.0;
    for prim in prims {
        let pair = prim.as_array().ok_or_else(|| "prim".to_string())?;
        let kind = pair.first().and_then(Value::as_str).unwrap_or("");
        let amount = pair.get(1).and_then(Value::as_f64).unwrap_or(0.0);
        if kind == "drive" {
            drives += 1;
            drive_sum += amount;
        } else if kind == "turn" {
            turns += 1;
        }
    }
    Ok(format!(
        "Captured primitives: {drives} drive and {turns} turn. Sum of stored drive numbers {drive_sum:.4}. The JSON has no unit field. Not sent. move.py compares a drive number with scaled odometry; that scale is not applied here.\n"
    ))
}

fn push_maps(report: &mut String, dir: &Path) {
    report.push_str("\n## Map arrays\n\n");
    report.push_str("Headers only. These arrays are not given to a controller.\n");
    if !dir.is_dir() {
        report.push_str("maps directory missing.\n");
        return;
    }
    let Ok(files) = list_files(dir, ".npy") else {
        report.push_str("maps directory unreadable.\n");
        return;
    };
    for path in files {
        let name = file_name(&path);
        let Ok(bytes) = std::fs::read(&path) else {
            report.push_str(&format!("`{name}` unreadable.\n"));
            continue;
        };
        match npy::inspect_npy(&bytes) {
            Ok(info) => report.push_str(&format!(
                "- `{name}` descr `{}` fortran {} shape {:?}. Not a controller input.\n",
                info.descr, info.fortran, info.shape
            )),
            Err(err) => report.push_str(&format!("- `{name}` header not read: {err}.\n")),
        }
    }
}

fn push_npz(report: &mut String, path: &Path) {
    report.push_str("\n## Synchronized camera archive\n\n");
    if !path.is_file() {
        report.push_str("camera.npz missing.\n");
        return;
    }
    let Ok(bytes) = std::fs::read(path) else {
        report.push_str("camera.npz unreadable.\n");
        return;
    };
    match zip_entries(&bytes) {
        Ok(entries) => {
            for entry in entries {
                report.push_str(&format!(
                    "- `{}` zip method {} blocked {}. Deflate payloads are not decoded, so no frame was invented.\n",
                    entry.name, entry.method, entry.blocked
                ));
            }
        }
        Err(err) => report.push_str(&format!("camera.npz not a readable zip header: {err}.\n")),
    }
}

fn list_files(dir: &Path, suffix: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(suffix) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file")
        .to_string()
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

#[cfg(test)]
pub(crate) fn duration_line_for_test(text: &str) -> Result<String, String> {
    meta_summary(text)
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::write_replay;
    use crate::lidar::encode_packet;
    use crate::npy::encode_npy_u16;

    #[test]
    fn a_synthetic_session_keeps_both_profiles_and_drops_no_secret() {
        let root = std::env::temp_dir().join(format!("physical-foundation-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("raw_lidar")).unwrap();
        std::fs::create_dir_all(root.join("camera")).unwrap();
        std::fs::create_dir_all(root.join("experiments")).unwrap();
        std::fs::create_dir_all(root.join("static_recording_pose9")).unwrap();
        std::fs::create_dir_all(root.join("logs")).unwrap();
        std::fs::create_dir_all(root.join("pi_config")).unwrap();
        let packet = encode_packet(0, &[(178.7, 4000, 1), (178.7, 0, 0)]).unwrap();
        std::fs::write(root.join("raw_lidar/course_lidar.bin"), &packet).unwrap();
        std::fs::write(
            root.join("camera/box_depth_mm.npy"),
            encode_npy_u16(2, 2, &[0, 400, 500, 1000]),
        )
        .unwrap();
        std::fs::write(root.join("experiments/speed_test.json"), speed_json()).unwrap();
        std::fs::write(root.join("experiments/speed_test2.json"), "{").unwrap();
        let mut chunks = chunk_bytes(5.0, &packet);
        chunks.extend(chunk_bytes(4.0, &packet));
        std::fs::write(
            root.join("static_recording_pose9/lidar_chunks.bin"),
            &chunks,
        )
        .unwrap();
        std::fs::write(
            root.join("static_recording_pose9/lidar_raw.bin"),
            packet.repeat(2),
        )
        .unwrap();
        std::fs::write(
            root.join("pi_config/robot.json"),
            r#"{"pi_ip":"203.0.113.50","lidar":{"yaw_deg":183.5,"z_m":0.22},"camera":{"z_m":0.27,"pitch_deg":0.0},"footprint":{"half_width_m":0.2},"motion":{"manual_linear_mps":0.15}}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("logs/loop_timing.log"),
            "wait S# t=0.00 yaw=1.0\nwait S# t=0.06 yaw=1.0\nlost S#\nssh 203.0.113.50\n",
        )
        .unwrap();
        let mut npz = vec![0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 8, 0];
        npz.resize(30, 0);
        npz.extend_from_slice(b"color.npy");
        std::fs::write(root.join("static_recording_pose9/camera.npz"), npz).unwrap();
        std::fs::write(
            root.join("static_recording_pose9/meta.json"),
            r#"{"duration_s":15.0,"odometry":[[0,[0,0,0]],[1,[0,0,0]]]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("figures")).unwrap();
        std::fs::write(root.join("figures/hidden.png"), b"FIGURES-NOT-READ").unwrap();
        let out = root.join("out");
        write_replay(&root, &out, None).unwrap();
        let report = std::fs::read_to_string(out.join("report.md")).unwrap();
        assert!(
            report.contains("File lidar yaw 183.5000 deg matches configured 183.5000: true. Matches measured 178.7000: false."),
            "{report}"
        );
        assert!(!report.contains("not recoverable"), "{report}");
        assert!(report.contains("unselected"), "{report}");
        assert!(!report.contains("Some("), "{report}");
        assert!(report.contains("Duration field 15.0 s"), "{report}");
        assert!(
            report.contains("PNG inventory scope is `camera/`"),
            "{report}"
        );
        assert!(!report.contains("FIGURES-NOT-READ"), "{report}");
        assert!(report.contains("reversed") || report.contains("Reversed"));
        assert!(
            report.contains("blocked") || report.contains("Blocked") || report.contains("method 8")
        );
        assert!(report.contains("SO-ARM100"));
        assert!(report.contains("not validated"));
        assert!(report.contains("unknown"));
        assert!(!report.contains("203.0.113.50"), "{report}");
        assert!(!contains_anywhere(&out, "203.0.113.50"));
        let speed = std::fs::read_to_string(out.join("speed.csv")).unwrap();
        assert!(
            speed.contains("0.200000") || speed.contains("0.2000"),
            "{speed}"
        );
        let svg = std::fs::read_to_string(out.join("scan-longest.svg")).unwrap();
        assert!(svg.contains("measured"));
        assert!(svg.contains("configured"));
        assert!(svg.contains("metres"));
        assert!(svg.contains("data-origin=\"robot\""), "{svg}");
        let frame = std::fs::read_to_string(out.join("frame-geometry.svg")).unwrap();
        assert!(!frame.contains("Some("), "{frame}");
        assert!(frame.contains("0.4980 m"), "{frame}");
        assert!(frame.contains("0.6417 m"), "{frame}");
        assert!(frame.contains("no floor hit"), "{frame}");
        let speed_svg = std::fs::read_to_string(out.join("speed.svg")).unwrap();
        assert!(speed_svg.contains("speed_test.json"), "{speed_svg}");
        assert!(!speed_svg.contains("speed_test2.json"), "{speed_svg}");
        let commands = std::fs::read_to_string(out.join("commands.csv")).unwrap();
        let mut command_rows = 0;
        for line in commands.lines().skip(1).filter(|line| !line.is_empty()) {
            command_rows += 1;
            assert!(line.ends_with(",false"), "{line}");
            assert!(!line.contains(",true"), "{line}");
        }
        assert!(command_rows >= 4, "{commands}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_duration_is_labelled_and_a_missing_duration_is_not_debug_text() {
        let present = super::duration_line_for_test(
            r#"{"duration_s":15.0,"odometry":[[0,[1,2,3]],[1,[1,2,3]]]}"#,
        )
        .unwrap();
        assert!(present.contains("Duration field 15.0 s"), "{present}");
        assert!(!present.contains("Some("), "{present}");
        let missing = super::duration_line_for_test(r#"{"odometry":[[0,[0,0,0]]]}"#).unwrap();
        assert!(missing.contains("Duration field missing"), "{missing}");
        assert!(!missing.contains("Some("), "{missing}");
    }

    fn contains_anywhere(dir: &std::path::Path, needle: &str) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && contains_anywhere(&path, needle) {
                return true;
            }
            if let Ok(text) = std::fs::read_to_string(&path)
                && text.contains(needle)
            {
                return true;
            }
        }
        false
    }

    fn speed_json() -> String {
        format!(
            r#"[{{"req":0.1,"secs":2,"odom0":[0,0,0],"odom1":[0.2,0,0],"A":[{}],"B":[{}]}}]"#,
            wall(2.0),
            wall(1.8)
        )
    }

    fn wall(distance: f64) -> String {
        let mut parts = Vec::new();
        for angle in -40..=40 {
            let range = distance / f64::from(angle).to_radians().cos();
            parts.push(format!("[{angle},{range:.6}]"));
        }
        parts.push("[0,0]".to_string());
        parts.push("[10,50]".to_string());
        parts.join(",")
    }

    fn chunk_bytes(time: f64, payload: &[u8]) -> Vec<u8> {
        let mut bytes = time.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(u32::try_from(payload.len()).unwrap()).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }
}
