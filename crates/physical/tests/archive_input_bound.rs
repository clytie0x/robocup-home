//! The archive CLI rejects an oversized file or frame before it parses one.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use physical::{ARCHIVE_FILE_LIMIT, COMPARE_FRAME_LIMIT};

fn sized_file(name: &str, len: usize) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let file = File::create(&path).expect("fixture");
    file.set_len(len as u64).expect("length");
    path
}

fn run_file(path: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_camera-archive"))
        .arg(path)
        .output()
        .expect("camera-archive")
}

fn run_stdin(framed: &[u8]) -> std::process::Output {
    let path = sized_file("archive-input-bound-empty.npz", 32);
    let mut child = Command::new(env!("CARGO_BIN_EXE_camera-archive"))
        .arg(&path)
        .arg("--compare")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("camera-archive");
    let mut stdin = child.stdin.take().expect("stdin");
    match stdin.write_all(framed) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(err) => panic!("frame write: {err}"),
    }
    drop(stdin);
    child.wait_with_output().expect("camera-archive output")
}

#[test]
fn a_file_one_byte_past_the_archive_ceiling_is_rejected() {
    let path = sized_file("archive-input-bound-oversize.npz", ARCHIVE_FILE_LIMIT + 1);
    let output = run_file(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(stderr.contains("input bound"), "stderr {stderr}");
    assert!(
        stderr.contains(&format!("limit={ARCHIVE_FILE_LIMIT}")),
        "stderr {stderr}"
    );
}

#[test]
fn a_file_at_the_ceiling_is_a_format_error_not_a_read_bound() {
    let path = sized_file("archive-input-bound-exact.npz", ARCHIVE_FILE_LIMIT);
    let output = run_file(&path);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(!stderr.contains("input bound"), "stderr {stderr}");
}

#[test]
fn a_comparison_stream_one_byte_past_the_frame_ceiling_is_rejected() {
    let framed = vec![0u8; COMPARE_FRAME_LIMIT + 1];
    let output = run_stdin(&framed);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr {stderr}");
    assert!(stderr.contains("input bound"), "stderr {stderr}");
    assert!(
        stderr.contains(&format!("limit={COMPARE_FRAME_LIMIT}")),
        "stderr {stderr}"
    );
}
