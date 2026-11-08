//! Read a file or a stream without taking more than a caller-supplied limit.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum BoundError {
    Bound { limit: usize },
    Io(io::ErrorKind),
}

pub fn read_file_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, BoundError> {
    let mut file = File::open(path).map_err(|err| BoundError::Io(err.kind()))?;
    read_bounded(&mut file, limit)
}

pub fn read_bounded(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>, BoundError> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if buf.len() > limit {
            return Err(BoundError::Bound { limit });
        }
        if buf.len() == limit {
            let mut extra = [0u8; 1];
            loop {
                match reader.read(&mut extra) {
                    Ok(0) => return Ok(buf),
                    Ok(_) => return Err(BoundError::Bound { limit }),
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => return Err(BoundError::Io(err.kind())),
                }
            }
        }
        let room = limit - buf.len();
        let want = room.min(chunk.len());
        match reader.read(&mut chunk[..want]) {
            Ok(0) => return Ok(buf),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(BoundError::Io(err.kind())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use super::{BoundError, read_bounded};
    use crate::camera_npz::{ARCHIVE_FILE_LIMIT, COMPARE_FRAME_LIMIT, split_compare_frame};
    use crate::still::STILL_FRAME_LIMIT;

    struct Step {
        data: Vec<u8>,
        pos: usize,
        step: usize,
        fail_at: Option<usize>,
    }

    impl Step {
        fn new(data: Vec<u8>, step: usize, fail_at: Option<usize>) -> Self {
            Self {
                data,
                pos: 0,
                step,
                fail_at,
            }
        }
    }

    impl Read for Step {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if buf.is_empty() {
                return Ok(0);
            }
            if self.fail_at == Some(self.pos) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "stopped"));
            }
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let n = self.step.min(buf.len()).min(self.data.len() - self.pos);
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }

    #[test]
    fn the_archive_and_frame_ceilings_match_the_documented_sizes() {
        let compressed = 5_561_734 + 1_002_834 + 342;
        let local = (30 + 20) * 3 + 9 + 12 + 15;
        let central = 46 * 3 + 9 + 12 + 15;
        let file_limit = compressed + local + central + 22;
        assert_eq!(file_limit, 6_565_292);
        assert_eq!(ARCHIVE_FILE_LIMIT, file_limit);
        let frame = 4 + (2 + 3 + 8 + 8_985_600) + (2 + 5 + 8 + 5_990_400) + (2 + 6 + 8 + 312);
        assert_eq!(frame, 14_976_360);
        assert_eq!(COMPARE_FRAME_LIMIT, frame);
        let still = 4 + (2 + 3 + 8 + 230_400) + (2 + 5 + 8 + 153_600);
        assert_eq!(still, 384_032);
        assert_eq!(STILL_FRAME_LIMIT, still);
    }

    #[test]
    fn one_byte_past_the_limit_is_rejected_without_reading_the_rest() {
        let mut src = Step::new(vec![7; 20], 1, None);
        let result = read_bounded(&mut src, 8);
        assert_eq!(result, Err(BoundError::Bound { limit: 8 }));
        assert_eq!(src.pos, 9);
    }

    #[test]
    fn a_short_stream_is_returned_and_an_exact_stream_ends_cleanly() {
        let mut short = Step::new(vec![1, 2, 3, 4, 5, 6, 7], 1, None);
        let got = read_bounded(&mut short, 8).expect("short");
        assert_eq!(got, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(short.pos, 7);
        let mut exact = Step::new(vec![4; 8], 3, None);
        let got = read_bounded(&mut exact, 8).expect("exact");
        assert_eq!(got, vec![4; 8]);
        assert_eq!(exact.pos, 8);
    }

    fn rgb_header(len: u64) -> Vec<u8> {
        let mut out = b"NPZ1".to_vec();
        out.extend_from_slice(&3u16.to_le_bytes());
        out.extend_from_slice(b"rgb");
        out.extend_from_slice(&len.to_le_bytes());
        out
    }

    #[test]
    fn a_declared_rgb_length_above_the_payload_is_rejected_before_the_bytes() {
        let framed = rgb_header(8_985_601);
        let err = split_compare_frame(&framed).expect_err("length");
        assert_eq!(err, "rgb length");
        assert!(framed.len() < 8_985_601);
    }

    #[test]
    fn a_declared_rgb_payload_that_stops_early_is_truncated() {
        let mut framed = rgb_header(8_985_600);
        framed.extend_from_slice(&[1, 2, 3]);
        let err = split_compare_frame(&framed).expect_err("truncated");
        assert!(err.contains("truncated"), "{err}");
        assert!(!err.contains("rgb length"), "{err}");
    }

    #[test]
    fn an_io_error_is_not_treated_as_the_end() {
        let mut src = Step::new(vec![9; 10], 1, Some(3));
        let result = read_bounded(&mut src, 8);
        assert_eq!(result, Err(BoundError::Io(io::ErrorKind::PermissionDenied)));
        assert_eq!(src.pos, 3);
    }
}
