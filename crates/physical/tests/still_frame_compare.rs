//! The still binary compares a framed stdin decode without a pixel file.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use physical::STILL_FRAME_LIMIT;

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

fn stored_zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut index = 0;
    loop {
        let n = (data.len() - index).min(65535);
        let last = index + n == data.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&data[index..index + n]);
        index += n;
        if last {
            break;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
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

fn synthetic_pair() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut scan = vec![0u8; 240 * 961];
    let mut rgb = Vec::with_capacity(230400);
    for row in 0..240 {
        for col in 0..320 {
            let at = row * 961 + 1 + col * 3;
            let red = (row as u8).wrapping_mul(3).wrapping_add(col as u8);
            let green = col as u8;
            let blue = row as u8;
            scan[at] = red;
            scan[at + 1] = green;
            scan[at + 2] = blue;
            rgb.extend_from_slice(&[red, green, blue]);
        }
    }
    let mut ihdr = [0u8; 13];
    ihdr[0..4].copy_from_slice(&320u32.to_be_bytes());
    ihdr[4..8].copy_from_slice(&240u32.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = 2;
    let zlib = stored_zlib(&scan);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend(chunk(b"IHDR", &ihdr));
    png.extend(chunk(b"IDAT", &zlib));
    png.extend(chunk(b"IEND", &[]));

    let mut words = Vec::with_capacity(240 * 320);
    let mut depth_le = Vec::with_capacity(240 * 320 * 2);
    for index in 0..240 * 320 {
        let word = (index % 3000) as u16 + 500;
        words.push(word);
        depth_le.extend_from_slice(&word.to_le_bytes());
    }
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
    for word in words {
        npy.extend_from_slice(&word.to_le_bytes());
    }
    (png, npy, rgb, depth_le)
}

fn frame(rgb: &[u8], depth: &[u8], trailing: &[u8]) -> Vec<u8> {
    let mut out = b"STL1".to_vec();
    section(&mut out, "rgb", rgb);
    section(&mut out, "depth", depth);
    out.extend_from_slice(trailing);
    out
}

fn section(out: &mut Vec<u8>, name: &str, payload: &[u8]) {
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(payload);
}

fn paths(label: &str) -> (PathBuf, PathBuf) {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("still-frame-compare-{label}"));
    std::fs::create_dir_all(&dir).expect("fixture directory");
    let (png, npy, _, _) = synthetic_pair();
    let png_path = dir.join("synthetic_color.png");
    let npy_path = dir.join("synthetic_depth_mm.npy");
    std::fs::write(&png_path, png).expect("synthetic png");
    std::fs::write(&npy_path, npy).expect("synthetic npy");
    (png_path, npy_path)
}

fn run(label: &str, framed: &[u8]) -> std::process::Output {
    let (png_path, npy_path) = paths(label);
    let mut child = Command::new(env!("CARGO_BIN_EXE_still-pair"))
        .arg(&png_path)
        .arg(&npy_path)
        .arg("--compare")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("still-pair");
    let mut stdin = child.stdin.take().expect("stdin");
    match stdin.write_all(framed) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(err) => panic!("frame: {err}"),
    }
    drop(stdin);
    child.wait_with_output().expect("still-pair output")
}

#[test]
fn a_framed_stdin_matches_every_synthetic_byte() {
    let (_, _, rgb, depth) = synthetic_pair();
    let output = run("match", &frame(&rgb, &depth, &[]));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout {stdout} stderr {stderr}"
    );
    assert!(stdout.contains("compared=yes match=yes"), "stdout {stdout}");
    assert!(stdout.contains("rgb_bytes=230400"), "stdout {stdout}");
    assert!(stdout.contains("depth_bytes=153600"), "stdout {stdout}");
}

#[test]
fn a_framed_rgb_byte_mismatch_reports_only_the_offset() {
    let (_, _, mut rgb, depth) = synthetic_pair();
    rgb[17] ^= 0x01;
    let output = run("mismatch", &frame(&rgb, &depth, &[]));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(stderr.contains("rgb mismatch offset=17"), "stderr {stderr}");
}

#[test]
fn a_trailing_frame_byte_is_rejected() {
    let (_, _, rgb, depth) = synthetic_pair();
    let output = run("trailing", &frame(&rgb, &depth, &[0]));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(stderr.contains("input bound"), "stderr {stderr}");
    assert!(
        stderr.contains(&format!("limit={STILL_FRAME_LIMIT}")),
        "stderr {stderr}"
    );
}

#[test]
fn a_frame_one_byte_past_the_still_ceiling_is_rejected() {
    let output = run("ceiling", &vec![0u8; STILL_FRAME_LIMIT + 1]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(stderr.contains("input bound"), "stderr {stderr}");
    assert!(
        stderr.contains(&format!("limit={STILL_FRAME_LIMIT}")),
        "stderr {stderr}"
    );
}
