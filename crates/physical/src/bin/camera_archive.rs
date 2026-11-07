//! Read one measured camera archive and optionally compare an independent decode.
//!
//! The program prints counts and hashes. It does not print pixels.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use physical::{read_camera_npz, sha256_hex};

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
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("camera-archive: {err}");
            return ExitCode::from(1);
        }
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
    if !compare {
        return ExitCode::from(0);
    }
    let mut framed = Vec::new();
    if let Err(err) = io::stdin().read_to_end(&mut framed) {
        eprintln!("camera-archive: {err}");
        return ExitCode::from(1);
    }
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
    if framed.len() < 4 || &framed[..4] != b"NPZ1" {
        return Err("frame header".to_string());
    }
    let mut pos = 4;
    let rgb_in = section(&mut pos, framed, "rgb")?;
    let depth_in = section(&mut pos, framed, "depth")?;
    let stamp_in = section(&mut pos, framed, "stamps")?;
    if pos != framed.len() {
        return Err(format!("frame trailing offset={pos}"));
    }
    same("rgb", &rgb_in, rgb)?;
    same("depth", &depth_in, depth)?;
    same("stamps", &stamp_in, stamps)?;
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
    let len = u64::from_le_bytes(len_bytes) as usize;
    *pos += 8;
    let end = pos
        .checked_add(len)
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

fn usage() -> ExitCode {
    eprintln!("usage: camera-archive FILE [--compare]");
    ExitCode::from(2)
}
