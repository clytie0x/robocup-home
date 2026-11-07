//! Print hashes of one saved still pair.
//!
//! The program reads a PNG and a depth `.npy`. It does not print pixels.

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use physical::{depth_bins, read_still_pair, sha256_hex};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(png) = args.next() else {
        return usage();
    };
    let Some(depth) = args.next() else {
        return usage();
    };
    let mut check_rgb = None;
    let mut check_depth = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check-rgb" => check_rgb = args.next(),
            "--check-depth" => check_depth = args.next(),
            _ => return usage(),
        }
    }
    let png_bytes = match fs::read(&png) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("still-pair: {err}");
            return ExitCode::from(1);
        }
    };
    let depth_bytes = match fs::read(&depth) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("still-pair: {err}");
            return ExitCode::from(1);
        }
    };
    let pair = match read_still_pair(&png_bytes, &depth_bytes) {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("still-pair: {err:?}");
            return ExitCode::from(1);
        }
    };
    let mut depth_le = Vec::with_capacity(pair.depth_mm.len() * 2);
    for word in &pair.depth_mm {
        depth_le.extend_from_slice(&word.to_le_bytes());
    }
    let (zero, low, valid) = depth_bins(&pair.depth_mm);
    println!(
        "width={} height={} rgb_sha256={} depth_sha256={} zero={} low={} valid={}",
        pair.width,
        pair.height,
        sha256_hex(&pair.rgb),
        sha256_hex(&depth_le),
        zero,
        low,
        valid
    );
    match (check_rgb, check_depth) {
        (None, None) => ExitCode::from(0),
        (Some(rgb_path), Some(depth_path)) => check(
            Path::new(&rgb_path),
            &pair.rgb,
            Path::new(&depth_path),
            &depth_le,
        ),
        _ => usage(),
    }
}

fn check(rgb_path: &Path, rgb: &[u8], depth_path: &Path, depth: &[u8]) -> ExitCode {
    let rgb_file = match fs::read(rgb_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("still-pair: {err}");
            return ExitCode::from(1);
        }
    };
    let depth_file = match fs::read(depth_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("still-pair: {err}");
            return ExitCode::from(1);
        }
    };
    let rgb_at = mismatch(&rgb_file, rgb);
    let depth_at = mismatch(&depth_file, depth);
    println!(
        "rgb_mismatch={} depth_mismatch={}",
        show(rgb_at),
        show(depth_at)
    );
    if rgb_at.is_none() && depth_at.is_none() {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn mismatch(left: &[u8], right: &[u8]) -> Option<usize> {
    if left.len() != right.len() {
        return Some(left.len().min(right.len()));
    }
    left.iter().zip(right).position(|(a, b)| a != b)
}

fn show(at: Option<usize>) -> String {
    match at {
        Some(index) => index.to_string(),
        None => "none".to_string(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: still-pair PNG DEPTH [--check-rgb RGBFILE --check-depth DEPTHFILE]");
    ExitCode::from(2)
}
