//! Posted captures and the unresolved physical spec for section 3.1.
//!
//! The crate does not drive hardware, does not award a score, and does not
//! treat the simulated gripper as an installed arm.

mod arm;
mod base;
mod chunk;
mod command;
mod json;
mod kind;
mod lidar;
mod navpost;
mod npy;
mod observe;
mod plot;
mod profile;
mod replay;
mod scene;
mod sha256;
mod speed;
mod zipnote;

pub use arm::{ArmIdentity, UrdfRead, grasp_status, identity, read_urdf};
pub use base::{BaseViews, demo_views};
pub use chunk::{Chunk, ChunkParse, parse_chunks};
pub use command::{format_command, wire_preview};
pub use kind::Kind;
pub use lidar::{LidarParse, RawSample, encode_packet, parse_lidar, split_revolutions};
pub use navpost::{
    ChunkReport, ClockLabel, PostClock, PostOutcome, SampleUse, ScanPost, UnusableKind,
    frame_sample, load_chunks, offset_ms, post_chunks, post_samples, write_chunk_post,
};
pub use npy::{NpyInfo, inspect_npy, load_depth_u16};
pub use observe::{
    Actuation, ControllerNote, DepthClass, DepthView, Freshness, PostedScan, classify_depth_mm,
    contains_ipv4, controller_note, delivery_freshness, pixel_bearing_deg, sanitize_line,
    stamp_order,
};
pub use profile::{
    CameraProfile, FloorHit, LidarProfile, RANGE_DIVISOR, camera_profiles, floor_hit,
    footprint_width_m, front_face_width_m, lateral_from_yaw_disagreement, lidar_misses_below,
    lidar_profiles, opening_blocks_chassis, wrap_full_deg, wrap_signed_deg, yaw_disagreement_deg,
};
pub use replay::{Error, write_replay};
pub use scene::{
    Aabb, Cast, Mount, PacketScene, Scene, SceneError, SceneViews, Vec3, cast_bearing,
    demonstration, inverse_bearing, packet_revolution, project_oblique, render_views,
    sensor_samples,
};
pub use sha256::{sha256, sha256_hex};
pub use speed::{
    ForwardEstimate, ForwardGate, GATES, Projection, SpeedRun, odom_delta, parse_speed_runs,
    scan_forward,
};
pub use zipnote::{ZipEntry, zip_entries};
