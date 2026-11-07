//! Read one measured camera archive and optionally compare an independent decode.
//!
//! The program prints counts and hashes. It does not print pixels.

use std::collections::BTreeSet;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use physical::{
    ARCHIVE_FILE_LIMIT, BoundError, COMPARE_FRAME_LIMIT, read_bounded, read_camera_npz,
    read_file_bounded, sha256_hex, split_compare_frame,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        return usage();
    };
    let compare = match args.next().as_deref() {
        None => false,
        Some("--compare") => true,
        _ => return usage(),
    };
    if args.next().is_some() {
        return usage();
    }
    let bytes = match read_file_bounded(Path::new(&path), ARCHIVE_FILE_LIMIT) {
        Ok(bytes) => bytes,
        Err(err) => return bound_exit(err),
    };
    let framed = if compare {
        match read_bounded(&mut io::stdin(), COMPARE_FRAME_LIMIT) {
            Ok(framed) => Some(framed),
            Err(err) => return bound_exit(err),
        }
    } else {
        None
    };
    let archive = match read_camera_npz(&bytes) {
        Ok(archive) => archive,
        Err(err) => {
            eprintln!("camera-archive: {err:?}");
            return ExitCode::from(1);
        }
    };
    let mut depth = Vec::with_capacity(archive.depth_mm.len() * 2);
    for word in &archive.depth_mm {
        depth.extend_from_slice(&word.to_le_bytes());
    }
    let mut stamps = Vec::with_capacity(archive.stamps.len() * 8);
    for stamp in &archive.stamps {
        stamps.extend_from_slice(&stamp.to_le_bytes());
    }
    let rgb_distinct = distinct(&archive.rgb, 240 * 320 * 3);
    let depth_distinct = distinct(&depth, 240 * 320 * 2);
    println!(
        "frames=39 width=320 height=240 rgb_bytes={} depth_bytes={} stamp_bytes={} rgb_sha256={} depth_sha256={} stamp_sha256={} rgb_distinct={} depth_distinct={} stamp_first={} stamp_last={}",
        archive.rgb.len(),
        depth.len(),
        stamps.len(),
        sha256_hex(&archive.rgb),
        sha256_hex(&depth),
        sha256_hex(&stamps),
        rgb_distinct,
        depth_distinct,
        archive.stamps.first().copied().unwrap_or(f64::NAN),
        archive.stamps.last().copied().unwrap_or(f64::NAN),
    );
    for (index, frame) in archive.rgb.chunks(240 * 320 * 3).enumerate() {
        println!("rgb_frame={index} sha256={}", sha256_hex(frame));
    }
    for (index, frame) in depth.chunks(240 * 320 * 2).enumerate() {
        println!("depth_frame={index} sha256={}", sha256_hex(frame));
    }
    let Some(framed) = framed else {
        return ExitCode::from(0);
    };
    match compare_framed(&framed, &archive.rgb, &depth, &stamps) {
        Ok(()) => {
            println!("compared=yes match=yes");
            ExitCode::from(0)
        }
        Err(err) => {
            eprintln!("camera-archive: {err}");
            ExitCode::from(1)
        }
    }
}

fn distinct(bytes: &[u8], frame: usize) -> usize {
    let mut seen = BTreeSet::new();
    for chunk in bytes.chunks(frame) {
        seen.insert(sha256_hex(chunk));
    }
    seen.len()
}

fn compare_framed(framed: &[u8], rgb: &[u8], depth: &[u8], stamps: &[u8]) -> Result<(), String> {
    let parts = split_compare_frame(framed)?;
    same("rgb", parts.rgb, rgb)?;
    same("depth", parts.depth, depth)?;
    same("stamps", parts.stamps, stamps)?;
    Ok(())
}

fn bound_exit(err: BoundError) -> ExitCode {
    match err {
        BoundError::Bound { limit } => {
            eprintln!("camera-archive: input bound limit={limit}");
            ExitCode::from(1)
        }
        BoundError::Io(kind) => {
            let _ = writeln!(io::stderr(), "camera-archive: {kind:?}");
            ExitCode::from(1)
        }
    }
}

fn same(name: &str, got: &[u8], expected: &[u8]) -> Result<(), String> {
    if got.len() != expected.len() {
        return Err(format!("{name} length {} {}", got.len(), expected.len()));
    }
    if let Some(offset) = got.iter().zip(expected).position(|(a, b)| a != b) {
        return Err(format!("{name} mismatch offset={offset}"));
    }
    Ok(())
}

fn usage() -> ExitCode {
    eprintln!("usage: camera-archive FILE [--compare]");
    ExitCode::from(2)
}
