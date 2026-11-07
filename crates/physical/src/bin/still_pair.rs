//! Print hashes of one saved still pair.
//!
//! The program reads a PNG and a depth `.npy`. It does not print pixels.
//! `--compare` reads one framed independent decode on stdin.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use physical::{
    BoundError, STILL_FRAME_LIMIT, depth_bins, read_bounded, read_still_pair, sha256_hex,
};

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
    let mut compare = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check-rgb" => check_rgb = args.next(),
            "--check-depth" => check_depth = args.next(),
            "--compare" => compare = true,
            _ => return usage(),
        }
    }
    if compare && (check_rgb.is_some() || check_depth.is_some()) {
        return usage();
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
    if compare {
        let framed = match read_bounded(&mut io::stdin(), STILL_FRAME_LIMIT) {
            Ok(framed) => framed,
            Err(err) => return bound_exit(err),
        };
        return match compare_framed(&framed, &pair.rgb, &depth_le) {
            Ok(()) => {
                println!(
                    "compared=yes match=yes frame_bytes={} rgb_bytes={} depth_bytes={}",
                    framed.len(),
                    pair.rgb.len(),
                    depth_le.len()
                );
                ExitCode::from(0)
            }
            Err(err) => {
                eprintln!("still-pair: {err}");
                ExitCode::from(1)
            }
        };
    }
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

fn bound_exit(err: BoundError) -> ExitCode {
    match err {
        BoundError::Bound { limit } => {
            eprintln!("still-pair: input bound limit={limit}");
            ExitCode::from(1)
        }
        BoundError::Io(kind) => {
            let _ = writeln!(io::stderr(), "still-pair: {kind:?}");
            ExitCode::from(1)
        }
    }
}

fn compare_framed(framed: &[u8], rgb: &[u8], depth: &[u8]) -> Result<(), String> {
    if framed.len() < 4 || &framed[..4] != b"STL1" {
        return Err("frame header".to_string());
    }
    let mut pos = 4;
    let rgb_in = section(&mut pos, framed, "rgb")?;
    let depth_in = section(&mut pos, framed, "depth")?;
    if pos != framed.len() {
        return Err(format!("frame trailing offset={pos}"));
    }
    same("rgb", &rgb_in, rgb)?;
    same("depth", &depth_in, depth)?;
    Ok(())
}

fn section(pos: &mut usize, framed: &[u8], expect: &str) -> Result<Vec<u8>, String> {
    if *pos + 2 > framed.len() {
        return Err(format!("{expect} truncated name"));
    }
    let name_len = u16::from_le_bytes([framed[*pos], framed[*pos + 1]]) as usize;
    *pos += 2;
    let name_end = pos
        .checked_add(name_len)
        .ok_or_else(|| "frame name".to_string())?;
    if name_end + 8 > framed.len() {
        return Err(format!("{expect} truncated"));
    }
    let name =
        std::str::from_utf8(&framed[*pos..name_end]).map_err(|_| "frame name".to_string())?;
    if name != expect {
        return Err(format!("frame name {name}"));
    }
    *pos = name_end;
    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&framed[*pos..*pos + 8]);
    let len = u64::from_le_bytes(len_bytes);
    *pos += 8;
    let expected: u64 = match expect {
        "rgb" => 230_400,
        "depth" => 153_600,
        _ => return Err(format!("frame name {expect}")),
    };
    if len != expected {
        return Err(format!("{expect} length"));
    }
    let end = pos
        .checked_add(expected as usize)
        .ok_or_else(|| "frame length".to_string())?;
    if end > framed.len() {
        return Err(format!("{expect} truncated payload"));
    }
    let out = framed[*pos..end].to_vec();
    *pos = end;
    Ok(out)
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
    eprintln!(
        "usage: still-pair PNG DEPTH [--compare | --check-rgb RGBFILE --check-depth DEPTHFILE]"
    );
    ExitCode::from(2)
}
