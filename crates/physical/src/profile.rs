//! Frozen sensor and body numbers. Disagreeing profiles stay side by side.

use crate::kind::Kind;

#[derive(Clone, Copy, Debug)]
pub struct LidarProfile {
    pub id: &'static str,
    pub front_raw_deg: f64,
    pub kind: Kind,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct CameraProfile {
    pub id: &'static str,
    pub x_m: f64,
    pub z_m: f64,
    pub pitch_down_deg: f64,
    pub kind: Kind,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct FloorHit {
    pub profile_id: &'static str,
    pub lower_ray_down_deg: f64,
    pub ground_range_m: Option<f64>,
    pub optical_ground_range_m: Option<f64>,
}

pub const LIDAR_X_M: f64 = 0.175;
pub const LIDAR_Z_M: f64 = 0.22;
pub const LIDAR_XYZ_SOURCE: &str =
    "pi robot.json lidar x_m y_m z_m; sensors.json repeats the same xyz. Not a second measurement.";
pub const DEPTH_GATE_M: f64 = 0.5;
pub const DEPTH_GATE_SOURCE: &str =
    "sensors.json depth_valid_from_m. Recorded as an OpenNI property. Not remeasured in this run.";
pub const HFOV_DEG: f64 = 58.59;
pub const VFOV_DEG: f64 = 45.64;
pub const FOV_SOURCE: &str = "sensors.json and the pi-drive README, both citing OpenNI properties. Not remeasured in this run.";
pub const MIRROR_SOURCE: &str = "pi-drive README: the image is mirrored, so image right is robot left. Not remeasured in this run.";
pub const RANGE_DIVISOR: f64 = 4000.0;
pub const RANGE_SOURCE: &str = "lidar.py and the session README: metres are the 16-bit word divided by 4000. Not tape-calibrated in this run.";
pub const SCENARIO_GAP_M: f64 = 0.28;
pub const SCENARIO_GAP_SOURCE: &str =
    "Approved section 3.1 failure case. Not a width measured in the 2026-11-07 captures.";
pub const CONFIG_LINEAR_M_S: f64 = 0.15;
pub const CONFIG_ANGULAR_DEG_S: f64 = 15.0;
pub const MOTION_SOURCE: &str = "pi robot.json motion.manual_linear_mps and manual_angular_degps. Configured caps, not a measured speed.";

/// Plan view from footprint.json. x forward, y left, metres.
pub const FOOTPRINT_XY_M: [(f64, f64); 8] = [
    (0.2, -0.16),
    (0.2, 0.16),
    (0.16, 0.2),
    (-0.16, 0.2),
    (-0.2, 0.16),
    (-0.2, -0.16),
    (-0.16, -0.2),
    (0.16, -0.2),
];

pub fn lidar_profiles() -> [LidarProfile; 2] {
    [
        LidarProfile {
            id: "measured-front",
            front_raw_deg: 178.7,
            kind: Kind::Measured,
            source: "lidar.py FRONT_LASER_DEG and sensors.json. The capture script used this front. Not chosen over the config value.",
        },
        LidarProfile {
            id: "configured-front",
            front_raw_deg: 183.5,
            kind: Kind::Configured,
            source: "pi robot.json lidar.yaw_deg. The file also has camera.mount_confirmed true at a different pose. That flag does not settle this yaw.",
        },
    ]
}

pub fn camera_profiles() -> [CameraProfile; 2] {
    [
        CameraProfile {
            id: "measured-mount",
            x_m: 0.14,
            z_m: 0.282,
            pitch_down_deg: 6.7,
            kind: Kind::Measured,
            source: "sensors.json, from a floor-plane fit dated 2026-11-07. This run does not refit that plane.",
        },
        CameraProfile {
            id: "configured-mount",
            x_m: 0.14,
            z_m: 0.27,
            pitch_down_deg: 0.0,
            kind: Kind::Configured,
            source: "pi robot.json camera z_m and pitch_deg. mount_confirmed in that file is a flag, not a measurement.",
        },
    ]
}

pub fn yaw_disagreement_deg() -> f64 {
    let profiles = lidar_profiles();
    profiles[1].front_raw_deg - profiles[0].front_raw_deg
}

pub fn wrap_signed_deg(mut degrees: f64) -> f64 {
    while degrees > 180.0 {
        degrees -= 360.0;
    }
    while degrees <= -180.0 {
        degrees += 360.0;
    }
    degrees
}

pub fn wrap_full_deg(mut degrees: f64) -> f64 {
    degrees %= 360.0;
    if degrees < 0.0 {
        degrees += 360.0;
    }
    degrees
}

/// Base-frame degrees, counter-clockwise from +x, for one raw clockwise laser angle.
pub fn base_deg(raw_clockwise_deg: f64, front_raw_deg: f64) -> f64 {
    wrap_signed_deg(front_raw_deg - raw_clockwise_deg)
}

pub fn base_xy(raw_clockwise_deg: f64, range_m: f64, front_raw_deg: f64) -> (f64, f64) {
    let radians = base_deg(raw_clockwise_deg, front_raw_deg).to_radians();
    (range_m * radians.cos(), range_m * radians.sin())
}

pub fn lateral_from_yaw_disagreement(range_m: f64) -> f64 {
    range_m * yaw_disagreement_deg().to_radians().sin()
}

pub fn lidar_misses_below(top_z_m: f64, plane_z_m: f64) -> bool {
    top_z_m < plane_z_m
}

pub fn footprint_width_m() -> f64 {
    let mut low = f64::MAX;
    let mut high = f64::MIN;
    for (_, y) in FOOTPRINT_XY_M {
        low = low.min(y);
        high = high.max(y);
    }
    high - low
}

pub fn front_face_width_m() -> f64 {
    let mut low = f64::MAX;
    let mut high = f64::MIN;
    for (x, y) in FOOTPRINT_XY_M {
        if (x - 0.2).abs() < 1e-9 {
            low = low.min(y);
            high = high.max(y);
        }
    }
    high - low
}

pub fn opening_blocks_chassis(opening_m: f64, chassis_width_m: f64) -> bool {
    opening_m < chassis_width_m
}

fn ground_range(z_m: f64, down_deg: f64) -> Option<f64> {
    if !(down_deg > 0.0 && z_m > 0.0) {
        return None;
    }
    let range = z_m / down_deg.to_radians().tan();
    range.is_finite().then_some(range)
}

pub fn floor_hit(profile: CameraProfile) -> FloorHit {
    let lower = profile.pitch_down_deg + VFOV_DEG / 2.0;
    FloorHit {
        profile_id: profile.id,
        lower_ray_down_deg: lower,
        ground_range_m: ground_range(profile.z_m, lower),
        optical_ground_range_m: ground_range(profile.z_m, profile.pitch_down_deg),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LIDAR_Z_M, SCENARIO_GAP_M, base_deg, base_xy, camera_profiles, floor_hit,
        footprint_width_m, front_face_width_m, lateral_from_yaw_disagreement, lidar_misses_below,
        lidar_profiles, opening_blocks_chassis, wrap_signed_deg, yaw_disagreement_deg,
    };

    #[test]
    fn adding_the_disagreement_rebuilds_the_unselected_front() {
        for raw in [0.0, 10.0, 178.7, 183.5, 350.0, 359.9] {
            let measured = base_deg(raw, 178.7);
            let configured = base_deg(raw, 183.5);
            let shifted = wrap_signed_deg(measured + yaw_disagreement_deg());
            assert!(
                (shifted - configured).abs() < 1e-9,
                "raw {raw} measured {measured} shifted {shifted} configured {configured}"
            );
        }
    }

    #[test]
    fn the_two_fronts_stay_apart() {
        let profiles = lidar_profiles();
        assert_eq!(profiles.len(), 2);
        assert!(yaw_disagreement_deg() > 4.0);
        assert!(yaw_disagreement_deg() < 5.0);
        assert_eq!(base_deg(178.7, profiles[0].front_raw_deg), 0.0);
        let (x_measured, y_measured) = base_xy(178.7, 1.0, profiles[0].front_raw_deg);
        let (_x_config, y_config) = base_xy(178.7, 1.0, profiles[1].front_raw_deg);
        assert!((x_measured - 1.0).abs() < 1e-9);
        assert!(y_measured.abs() < 1e-9);
        assert!(y_config > 0.08 && y_config < 0.09);
        let lateral = lateral_from_yaw_disagreement(1.0);
        assert!(lateral > 0.08 && lateral < 0.09);
    }

    #[test]
    fn signed_wrap_matches_the_open_upper_bound() {
        assert_eq!(wrap_signed_deg(180.0), 180.0);
        assert_eq!(wrap_signed_deg(-180.0), 180.0);
        assert!((wrap_signed_deg(180.1) + 179.9).abs() < 1e-9);
    }

    #[test]
    fn a_low_top_misses_the_scan_plane_and_a_narrow_gap_blocks_the_chassis() {
        assert!(lidar_misses_below(0.10, LIDAR_Z_M));
        assert!(!lidar_misses_below(LIDAR_Z_M, LIDAR_Z_M));
        let width = footprint_width_m();
        let front = front_face_width_m();
        assert!((width - 0.40).abs() < 1e-9);
        assert!((front - 0.32).abs() < 1e-9);
        assert!(opening_blocks_chassis(SCENARIO_GAP_M, width));
        assert!(opening_blocks_chassis(SCENARIO_GAP_M, front));
        assert!(!opening_blocks_chassis(width, width));
    }

    #[test]
    fn camera_profiles_do_not_collapse_and_the_lower_ray_hits_the_floor() {
        let profiles = camera_profiles();
        assert!((profiles[0].z_m - profiles[1].z_m).abs() > 0.01);
        assert!(profiles[0].pitch_down_deg > profiles[1].pitch_down_deg);
        let measured = floor_hit(profiles[0]);
        let configured = floor_hit(profiles[1]);
        let measured_ground = measured.ground_range_m.expect("measured lower ray");
        let configured_ground = configured.ground_range_m.expect("configured lower ray");
        assert!(measured.optical_ground_range_m.is_some());
        assert!(configured.optical_ground_range_m.is_none());
        assert!(measured_ground < configured_ground);
        assert!(measured_ground > 0.45 && measured_ground < 0.55);
    }
}
