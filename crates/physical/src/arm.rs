//! SO-ARM100 identity. Upstream geometry is not an installed calibration.

use crate::kind::Kind;
use crate::sha256::sha256_hex;

pub const URDF_SHA256: &str = "f7050417ebbb79358f113ddd4bca2b457b2c6d5048193331db5a04a0066439ed";
pub const UPSTREAM_COMMIT: &str = "a758567c3978dfeefe282ede0500085a48fe8f78";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraspStatus {
    NotValidated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmIdentity {
    pub name: &'static str,
    pub vendor: &'static str,
    pub commit: &'static str,
    pub urdf_sha256: &'static str,
    pub license: &'static str,
    pub geometry: &'static str,
    pub mount_to_base: Kind,
    pub joint_limits: Kind,
    pub grasp: GraspStatus,
    pub simulated_gripper_is_this_arm: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpstreamJoint {
    pub name: String,
    pub lower: f64,
    pub upper: f64,
    pub effort: f64,
    pub velocity: f64,
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UrdfRead {
    pub hash_matches_recorded: bool,
    pub sha256: String,
    pub joints: Vec<UpstreamJoint>,
    pub mentions_approximate_inertia: bool,
    pub link_mass_count: usize,
    pub mass_status: &'static str,
}

pub fn identity() -> ArmIdentity {
    ArmIdentity {
        name: "SO-ARM100",
        vendor: "TheRobotStudio",
        commit: UPSTREAM_COMMIT,
        urdf_sha256: URDF_SHA256,
        license: "Apache-2.0 LICENSE at that commit. No NOTICE file was in the repository root listing. Meshes are not committed here.",
        geometry: "Private reference URDF and 13 STL files. Hashes checked against MODEL_SOURCE.json. Not an installed-robot measurement.",
        mount_to_base: Kind::Unknown,
        joint_limits: Kind::UpstreamNominal,
        grasp: GraspStatus::NotValidated,
        simulated_gripper_is_this_arm: false,
    }
}

pub fn grasp_status() -> GraspStatus {
    GraspStatus::NotValidated
}

pub fn read_urdf(bytes: &[u8]) -> UrdfRead {
    let text = String::from_utf8_lossy(bytes);
    let sha256 = sha256_hex(bytes);
    UrdfRead {
        hash_matches_recorded: sha256 == URDF_SHA256,
        sha256,
        joints: revolute_joints(&text),
        mentions_approximate_inertia: text.contains("Approximate inertial values"),
        link_mass_count: text.matches("<mass ").count(),
        mass_status: "upstream text, not an installed-robot measurement",
    }
}

fn revolute_joints(text: &str) -> Vec<UpstreamJoint> {
    let mut joints = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<joint ") {
        let after = &rest[start..];
        let end = after.find("</joint>").unwrap_or(after.len());
        let block = &after[..end];
        rest = &after[end..];
        if !block.contains("type=\"revolute\"") && !block.contains("type='revolute'") {
            continue;
        }
        let Some(name) = attr(block, "name") else {
            continue;
        };
        let Some(limit_at) = block.find("<limit ") else {
            continue;
        };
        let limit_end = block[limit_at..]
            .find("/>")
            .map(|at| limit_at + at)
            .unwrap_or(block.len());
        let limit = &block[limit_at..limit_end];
        let (Some(lower), Some(upper), Some(effort), Some(velocity)) = (
            attr(limit, "lower").and_then(|text| text.parse().ok()),
            attr(limit, "upper").and_then(|text| text.parse().ok()),
            attr(limit, "effort").and_then(|text| text.parse().ok()),
            attr(limit, "velocity").and_then(|text| text.parse().ok()),
        ) else {
            continue;
        };
        joints.push(UpstreamJoint {
            name,
            lower,
            upper,
            effort,
            velocity,
            kind: Kind::UpstreamNominal,
        });
    }
    joints
}

fn attr(block: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    let start = block.find(&pattern)? + pattern.len();
    let end = block[start..].find('"')? + start;
    Some(block[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::{GraspStatus, grasp_status, identity, read_urdf};
    use crate::kind::Kind;

    #[test]
    fn the_arm_is_identified_and_grasp_stays_unvalidated() {
        let arm = identity();
        assert_eq!(arm.name, "SO-ARM100");
        assert_eq!(arm.mount_to_base, Kind::Unknown);
        assert_eq!(arm.joint_limits, Kind::UpstreamNominal);
        assert!(!arm.simulated_gripper_is_this_arm);
        assert_eq!(grasp_status(), GraspStatus::NotValidated);
        let text = r#"<robot name="so_arm100"><joint name="gripper" type="revolute"><limit lower="-0.2" upper="2.0" effort="35" velocity="1"/></joint><mass value="1.0"/><!-- Approximate inertial values --></robot>"#;
        let read = read_urdf(text.as_bytes());
        assert!(!read.hash_matches_recorded);
        assert_eq!(read.joints.len(), 1);
        assert_eq!(read.joints[0].name, "gripper");
        assert_eq!(read.joints[0].kind, Kind::UpstreamNominal);
        assert!(read.mentions_approximate_inertia);
        assert_eq!(grasp_status(), GraspStatus::NotValidated);
    }
}
