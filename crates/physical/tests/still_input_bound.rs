//! The still CLI stops path reads at the supported-subset ceilings.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const PNG_LIMIT: usize = 3_407_917;
const DEPTH_LIMIT: usize = 219_145;
const CHECK_RGB: usize = 230_400;
const CHECK_DEPTH: usize = 153_600;

fn sparse(dir: &std::path::Path, name: &str, len: usize) -> PathBuf {
    let path = dir.join(name);
    let file = File::create(&path).expect("fixture");
    file.set_len(len as u64).expect("length");
    path
}

fn run(_dir: &std::path::Path, args: &[&std::path::Path]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_still-pair"));
    for arg in args {
        command.arg(arg);
    }
    command.output().expect("still-pair")
}

fn assert_bound(output: &std::process::Output, limit: usize) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(
        stderr.contains(&format!("input bound limit={limit}")),
        "stderr {stderr}"
    );
}

#[test]
fn a_png_one_byte_past_the_supported_ceiling_is_rejected() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("still-input-bound-png");
    std::fs::create_dir_all(&dir).expect("dir");
    let png = sparse(&dir, "over.png", PNG_LIMIT + 1);
    let depth = sparse(&dir, "tiny.npy", 16);
    let output = run(&dir, &[png.as_path(), depth.as_path()]);
    assert_bound(&output, PNG_LIMIT);
}

#[test]
fn a_depth_one_byte_past_the_supported_ceiling_is_rejected() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("still-input-bound-depth");
    std::fs::create_dir_all(&dir).expect("dir");
    let png = write_png(&dir);
    let depth = sparse(&dir, "over.npy", DEPTH_LIMIT + 1);
    let output = run(&dir, &[png.as_path(), depth.as_path()]);
    assert_bound(&output, DEPTH_LIMIT);
}

#[test]
fn a_check_rgb_one_byte_past_230400_is_rejected() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("still-input-bound-check");
    std::fs::create_dir_all(&dir).expect("dir");
    let png = write_png(&dir);
    let depth = write_depth(&dir);
    let rgb = sparse(&dir, "over.rgb", CHECK_RGB + 1);
    let raw = sparse(&dir, "exact.depth", CHECK_DEPTH);
    let output = Command::new(env!("CARGO_BIN_EXE_still-pair"))
        .arg(&png)
        .arg(&depth)
        .arg("--check-rgb")
        .arg(&rgb)
        .arg("--check-depth")
        .arg(&raw)
        .output()
        .expect("still-pair");
    assert_bound(&output, CHECK_RGB);
}

#[test]
fn a_check_depth_one_byte_past_153600_is_rejected() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("still-input-bound-check-depth");
    std::fs::create_dir_all(&dir).expect("dir");
    let png = write_png(&dir);
    let depth = write_depth(&dir);
    let rgb = sparse(&dir, "exact.rgb", CHECK_RGB);
    let raw = sparse(&dir, "over.depth", CHECK_DEPTH + 1);
    let output = Command::new(env!("CARGO_BIN_EXE_still-pair"))
        .arg(&png)
        .arg(&depth)
        .arg("--check-rgb")
        .arg(&rgb)
        .arg("--check-depth")
        .arg(&raw)
        .output()
        .expect("still-pair");
    assert_bound(&output, CHECK_DEPTH);
}

fn write_png(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("pair.png");
    let mut file = File::create(&path).expect("png");
    file.write_all(&minimal_png()).expect("png bytes");
    path
}

fn write_depth(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("pair.npy");
    let mut file = File::create(&path).expect("npy");
    file.write_all(&minimal_depth()).expect("npy bytes");
    path
}

fn minimal_png() -> Vec<u8> {
    let scan = vec![0u8; 240 * 961];
    let zlib = stored_zlib(&scan);
    let mut ihdr = [0u8; 13];
    ihdr[0..4].copy_from_slice(&320u32.to_be_bytes());
    ihdr[4..8].copy_from_slice(&240u32.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = 2;
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend(chunk(b"IHDR", &ihdr));
    png.extend(chunk(b"IDAT", &zlib));
    png.extend(chunk(b"IEND", &[]));
    png
}

fn minimal_depth() -> Vec<u8> {
    let mut header = "{'descr': '<u2', 'fortran_order': False, 'shape': (240, 320), }".to_string();
    let pad = (16 - ((10 + header.len() + 1) % 16)) % 16;
    header.push_str(&" ".repeat(pad));
    header.push('\n');
    let mut npy = Vec::new();
    npy.extend_from_slice(b"\x93NUMPY");
    npy.push(1);
    npy.push(0);
    npy.extend_from_slice(&(header.len() as u16).to_le_bytes());
    npy.extend(header.into_bytes());
    npy.extend(std::iter::repeat_n(0u8, 240 * 320 * 2));
    npy
}

fn stored_zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut index = 0;
    while index < data.len() {
        let n = (data.len() - index).min(65535);
        let last = index + n == data.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&data[index..index + n]);
        index += n;
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a += u32::from(byte);
        if a >= 65521 {
            a -= 65521;
        }
        b += a;
        if b >= 65521 {
            b -= 65521;
        }
    }
    (b << 16) | a
}

fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut signed = Vec::with_capacity(4 + data.len());
    signed.extend_from_slice(tag);
    signed.extend_from_slice(data);
    let crc = crc32(&signed);
    let mut out = Vec::new();
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend(signed);
    out.extend_from_slice(&crc.to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
