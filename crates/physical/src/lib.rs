//! Posted captures and the unresolved physical spec for section 3.1.
//!
//! The crate does not drive hardware, does not award a score, and does not
//! treat the simulated gripper as an installed arm.

mod arm;
mod base;
mod bounded_read;
mod camera;
mod camera_npz;
mod chunk;
mod command;
mod frame;
mod inflate_zlib;
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
mod still;
mod zipnote;

pub use arm::{ArmIdentity, UrdfRead, grasp_status, identity, read_urdf};
pub use base::{BaseViews, demo_views};
pub use bounded_read::{BoundError, read_bounded, read_file_bounded};
pub use camera::{
    CameraViews, CastQuery, DepthRay, IMAGE_HEIGHT, IMAGE_WIDTH, apply_dropout, cast_axis,
    cast_pixel, demo_camera, pixel_for_tangent, post_depth,
};
pub use camera_npz::{
    ARCHIVE_FILE_LIMIT, ArchiveError, COMPARE_FRAME_LIMIT, CameraArchive, CompareParts,
    read_camera_npz, split_compare_frame,
};
pub use chunk::{Chunk, ChunkParse, parse_chunks};
pub use command::{format_command, wire_preview};
pub use frame::{
    CameraFrame, FrameViews, cast_frame, class_ppm, decode_frame, demo_frames, depth_view,
    encode_frame, fault_csv, rgb_ppm,
};
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
pub use still::{
    STILL_DEPTH_WORDS, STILL_FRAME_LIMIT, STILL_HEIGHT, STILL_RGB_BYTES, STILL_WIDTH, StillError,
    StillPair, depth_bins, read_still_depth, read_still_pair, read_still_png,
};
pub use zipnote::{ZipEntry, zip_entries};
