//! Offline reader for one 320 by 240 still pair.
//!
//! The PNG path accepts the measured subset only: colour type 2, bit depth 8,
//! no interlace, and a filter byte of 0 on every row. Depth must be a
//! little-endian `u16` array of shape 240 by 320 in C order.

use crate::inflate_zlib::inflate_capped;
use crate::npy::inspect_npy;

pub const STILL_WIDTH: usize = 320;
pub const STILL_HEIGHT: usize = 240;
pub const STILL_RGB_BYTES: usize = STILL_WIDTH * STILL_HEIGHT * 3;
pub const STILL_DEPTH_WORDS: usize = STILL_WIDTH * STILL_HEIGHT;
pub const STILL_FRAME_LIMIT: usize =
    4 + (2 + 3 + 8 + STILL_RGB_BYTES) + (2 + 5 + 8 + STILL_DEPTH_WORDS * 2);
/// Largest PNG this reader accepts.
///
/// One IHDR, one or more nonempty IDAT chunks whose data totals at most
/// 262144 bytes, and one empty IEND. Empty IDAT chunks are rejected so the
/// count is finite. This is a support ceiling, not a PNG specification maximum,
/// and the previous reader did not enforce it.
pub const STILL_PNG_FILE_LIMIT: usize = 8 + (12 + 13) + 12 + 262_144 * (12 + 1);
/// Largest depth file this reader accepts.
///
/// NumPy 1.0 only: a 10-byte preamble, a `u16` header of at most 65535 bytes,
/// and 153600 payload bytes. `inspect_npy` still parses major 2 and 3.
/// Those versions are outside `read_still_depth`. This is not their maximum.
pub const STILL_DEPTH_FILE_LIMIT: usize = 10 + 65_535 + STILL_DEPTH_WORDS * 2;
pub const STILL_CHECK_RGB_LIMIT: usize = STILL_RGB_BYTES;
pub const STILL_CHECK_DEPTH_LIMIT: usize = STILL_DEPTH_WORDS * 2;
const SCAN_BYTES: usize = STILL_HEIGHT * (1 + STILL_WIDTH * 3);
const MAX_IDAT: usize = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StillError {
    Truncated,
    Size,
    Crc,
    Adler,
    Header,
    ChunkOrder,
    Duplicate,
    Filter,
    Inflate,
    Trailing,
    Depth,
    Shape,
    Order,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StillPair {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
    pub depth_mm: Vec<u16>,
}

pub fn read_still_png(bytes: &[u8]) -> Result<Vec<u8>, StillError> {
    const SIG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.len() < SIG.len() || &bytes[..SIG.len()] != SIG {
        return Err(StillError::Header);
    }
    let mut pos = SIG.len();
    let mut seen_ihdr = false;
    let mut idat_open = false;
    let mut idat_closed = false;
    let mut seen_iend = false;
    let mut idat = Vec::new();
    while pos < bytes.len() {
        if seen_iend {
            return Err(StillError::Trailing);
        }
        if pos.checked_add(8).ok_or(StillError::Size)? > bytes.len() {
            return Err(StillError::Truncated);
        }
        let len = usize::try_from(be_u32(&bytes[pos..pos + 4])?).map_err(|_| StillError::Size)?;
        if len > MAX_IDAT {
            return Err(StillError::Size);
        }
        let data_at = pos + 8;
        let data_end = data_at.checked_add(len).ok_or(StillError::Size)?;
        let crc_end = data_end.checked_add(4).ok_or(StillError::Size)?;
        if crc_end > bytes.len() {
            return Err(StillError::Truncated);
        }
        let tag = &bytes[pos + 4..pos + 8];
        let data = &bytes[data_at..data_end];
        let stored = be_u32(&bytes[data_end..crc_end])?;
        if crc32(&[tag, data]) != stored {
            return Err(StillError::Crc);
        }
        if tag == b"IHDR" {
            if seen_ihdr {
                return Err(StillError::Duplicate);
            }
            if idat_open {
                return Err(StillError::ChunkOrder);
            }
            check_ihdr(data)?;
            seen_ihdr = true;
        } else if tag == b"IDAT" {
            if !seen_ihdr || seen_iend || idat_closed {
                return Err(StillError::ChunkOrder);
            }
            if data.is_empty() {
                return Err(StillError::Size);
            }
            let next = idat.len().checked_add(data.len()).ok_or(StillError::Size)?;
            if next > MAX_IDAT {
                return Err(StillError::Size);
            }
            idat.extend_from_slice(data);
            idat_open = true;
        } else if tag == b"IEND" {
            if !seen_ihdr || !idat_open || !data.is_empty() {
                return Err(StillError::ChunkOrder);
            }
            seen_iend = true;
        } else {
            return Err(StillError::ChunkOrder);
        }
        if tag != b"IDAT" && idat_open {
            idat_closed = true;
        }
        pos = crc_end;
    }
    if !seen_iend {
        return Err(StillError::Truncated);
    }
    let scan = inflate_capped(&idat, SCAN_BYTES)?;
    if scan.len() != SCAN_BYTES {
        return Err(StillError::Size);
    }
    let mut rgb = Vec::with_capacity(STILL_RGB_BYTES);
    for row in 0..STILL_HEIGHT {
        let start = row * (1 + STILL_WIDTH * 3);
        let Some(row_bytes) = scan.get(start..start + 1 + STILL_WIDTH * 3) else {
            return Err(StillError::Size);
        };
        if row_bytes[0] != 0 {
            return Err(StillError::Filter);
        }
        rgb.extend_from_slice(&row_bytes[1..]);
    }
    Ok(rgb)
}

pub fn read_still_depth(bytes: &[u8]) -> Result<Vec<u16>, StillError> {
    if bytes.len() < 8 || &bytes[..6] != b"\x93NUMPY" || bytes[6] != 1 || bytes[7] != 0 {
        return Err(StillError::Depth);
    }
    let header = inspect_npy(bytes).map_err(|_| StillError::Depth)?;
    if header.fortran {
        return Err(StillError::Order);
    }
    if header.descr != "<u2" {
        return Err(StillError::Depth);
    }
    if header.shape.as_slice() != [STILL_HEIGHT, STILL_WIDTH] {
        return Err(StillError::Shape);
    }
    let payload = STILL_DEPTH_WORDS.checked_mul(2).ok_or(StillError::Size)?;
    let end = header
        .data_offset
        .checked_add(payload)
        .ok_or(StillError::Size)?;
    if bytes.len() < end {
        return Err(StillError::Truncated);
    }
    if bytes.len() != end {
        return Err(StillError::Size);
    }
    let (pairs, rest) = bytes[header.data_offset..end].as_chunks::<2>();
    if !rest.is_empty() {
        return Err(StillError::Size);
    }
    let mut words = Vec::with_capacity(STILL_DEPTH_WORDS);
    for pair in pairs {
        words.push(u16::from_le_bytes(*pair));
    }
    if words.len() != STILL_DEPTH_WORDS {
        return Err(StillError::Size);
    }
    Ok(words)
}

pub fn read_still_pair(png: &[u8], depth: &[u8]) -> Result<StillPair, StillError> {
    Ok(StillPair {
        width: STILL_WIDTH,
        height: STILL_HEIGHT,
        rgb: read_still_png(png)?,
        depth_mm: read_still_depth(depth)?,
    })
}

fn check_ihdr(data: &[u8]) -> Result<(), StillError> {
    if data.len() != 13 {
        return Err(StillError::Header);
    }
    let width = be_u32(&data[0..4])?;
    let height = be_u32(&data[4..8])?;
    if width != STILL_WIDTH as u32
        || height != STILL_HEIGHT as u32
        || data[8] != 8
        || data[9] != 2
        || data[10] != 0
        || data[11] != 0
        || data[12] != 0
    {
        return Err(StillError::Header);
    }
    Ok(())
}

fn be_u32(bytes: &[u8]) -> Result<u32, StillError> {
    let Some(chunk) = bytes.get(..4) else {
        return Err(StillError::Truncated);
    };
    let Ok(chunk) = <[u8; 4]>::try_from(chunk) else {
        return Err(StillError::Truncated);
    };
    Ok(u32::from_be_bytes(chunk))
}

fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for part in parts {
        for &byte in *part {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
    }
    !crc
}

pub fn depth_bins(words: &[u16]) -> (usize, usize, usize) {
    let mut zero = 0;
    let mut low = 0;
    let mut valid = 0;
    for word in words {
        if *word == 0 {
            zero += 1;
        } else if *word < 500 {
            low += 1;
        } else {
            valid += 1;
        }
    }
    (zero, low, valid)
}

#[cfg(test)]
mod tests {
    use super::{
        STILL_CHECK_DEPTH_LIMIT, STILL_CHECK_RGB_LIMIT, STILL_DEPTH_FILE_LIMIT, STILL_DEPTH_WORDS,
        STILL_FRAME_LIMIT, STILL_HEIGHT, STILL_PNG_FILE_LIMIT, STILL_RGB_BYTES, STILL_WIDTH,
        StillError, depth_bins, read_still_depth, read_still_pair, read_still_png,
    };

    const DYNAMIC_ZLIB: &[u8] = include_bytes!("still_dynamic.zlib");

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

    fn ihdr(color: u8, interlace: u8) -> [u8; 13] {
        let mut data = [0u8; 13];
        data[0..4].copy_from_slice(&320u32.to_be_bytes());
        data[4..8].copy_from_slice(&240u32.to_be_bytes());
        data[8] = 8;
        data[9] = color;
        data[11] = 0;
        data[12] = interlace;
        data
    }

    fn png_chunks(parts: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        for (tag, data) in parts {
            out.extend(chunk(tag, data));
        }
        out
    }

    fn png(header: [u8; 13], zlib: &[u8]) -> Vec<u8> {
        png_chunks(&[(b"IHDR", &header), (b"IDAT", zlib), (b"IEND", &[])])
    }

    fn scan_pattern() -> Vec<u8> {
        let mut scan = vec![0u8; 240 * 961];
        for row in 0..240 {
            for col in 0..320 {
                let at = row * 961 + 1 + col * 3;
                scan[at] = (row as u8).wrapping_mul(3).wrapping_add(col as u8);
                scan[at + 1] = col as u8;
                scan[at + 2] = row as u8;
            }
        }
        scan
    }

    fn rgb_of(scan: &[u8]) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(STILL_RGB_BYTES);
        for row in 0..240 {
            let start = row * 961 + 1;
            rgb.extend_from_slice(&scan[start..start + 960]);
        }
        rgb
    }

    fn npy_u16(height: usize, width: usize, words: &[u16], descr: &str, fortran: bool) -> Vec<u8> {
        let flag = if fortran { "True" } else { "False" };
        let mut header = format!(
            "{{'descr': '{descr}', 'fortran_order': {flag}, 'shape': ({height}, {width}), }}"
        );
        let pad = (16 - ((10 + header.len() + 1) % 16)) % 16;
        header.push_str(&" ".repeat(pad));
        header.push('\n');
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend(header.into_bytes());
        for value in words {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn first3(bytes: &[u8]) -> Option<Vec<u8>> {
        bytes.get(0..3).map(|slice| slice.to_vec())
    }

    #[test]
    fn a_dynamic_png_keeps_channel_order_and_row_major_pixels() {
        let rgb = read_still_png(&png(ihdr(2, 0), DYNAMIC_ZLIB)).expect("png");
        assert_eq!(first3(&rgb), Some(vec![9, 8, 7]));
        assert_eq!(rgb.get(3..6), Some(&[4, 5, 6][..]));
        assert_eq!(rgb.get(319 * 3..319 * 3 + 3), Some(&[10, 11, 12][..]));
        assert_eq!(rgb.get(320 * 3..320 * 3 + 3), Some(&[1, 2, 3][..]));
        assert_eq!(rgb.len(), STILL_RGB_BYTES);
    }

    #[test]
    fn a_stored_png_round_trips_every_pixel() {
        let scan = scan_pattern();
        let rgb = read_still_png(&png(ihdr(2, 0), &stored_zlib(&scan))).expect("png");
        let expected = rgb_of(&scan);
        assert_eq!(rgb.len(), expected.len());
        assert_eq!(rgb.iter().zip(&expected).position(|(a, b)| a != b), None);
    }

    #[test]
    fn a_bad_chunk_crc_is_rejected() {
        let mut bytes = png(ihdr(2, 0), DYNAMIC_ZLIB);
        bytes[29] ^= 0xff;
        assert_eq!(read_still_png(&bytes).err(), Some(StillError::Crc));
    }

    #[test]
    fn a_bad_adler_is_rejected() {
        let mut zlib = DYNAMIC_ZLIB.to_vec();
        let last = zlib.len() - 1;
        zlib[last] ^= 0xff;
        assert_eq!(
            read_still_png(&png(ihdr(2, 0), &zlib)).err(),
            Some(StillError::Adler)
        );
    }

    #[test]
    fn a_truncated_zlib_is_rejected() {
        let mut zlib = DYNAMIC_ZLIB.to_vec();
        zlib.pop();
        assert_eq!(
            read_still_png(&png(ihdr(2, 0), &zlib)).err(),
            Some(StillError::Truncated)
        );
    }

    #[test]
    fn trailing_compressed_bytes_are_rejected() {
        let mut zlib = DYNAMIC_ZLIB.to_vec();
        zlib.push(0);
        assert_eq!(
            read_still_png(&png(ihdr(2, 0), &zlib)).err(),
            Some(StillError::Trailing)
        );
    }

    #[test]
    fn trailing_file_bytes_are_rejected() {
        let mut bytes = png(ihdr(2, 0), DYNAMIC_ZLIB);
        bytes.push(0);
        assert_eq!(read_still_png(&bytes).err(), Some(StillError::Trailing));
    }

    #[test]
    fn an_inflated_stream_past_the_scan_cap_is_rejected() {
        let zlib = stored_zlib(&vec![0u8; 230641]);
        assert_eq!(
            read_still_png(&png(ihdr(2, 0), &zlib)).err(),
            Some(StillError::Inflate)
        );
    }

    #[test]
    fn a_non_zero_filter_is_rejected() {
        let mut scan = scan_pattern();
        scan[0] = 1;
        assert_eq!(
            read_still_png(&png(ihdr(2, 0), &stored_zlib(&scan))).err(),
            Some(StillError::Filter)
        );
    }

    #[test]
    fn colour_type_and_interlace_outside_the_measured_subset_are_rejected() {
        assert_eq!(
            read_still_png(&png(ihdr(6, 0), DYNAMIC_ZLIB)).err(),
            Some(StillError::Header)
        );
        assert_eq!(
            read_still_png(&png(ihdr(2, 1), DYNAMIC_ZLIB)).err(),
            Some(StillError::Header)
        );
    }

    #[test]
    fn a_second_ihdr_and_an_idat_before_ihdr_are_rejected() {
        let header = ihdr(2, 0);
        let doubled = png_chunks(&[
            (b"IHDR", &header),
            (b"IHDR", &header),
            (b"IDAT", DYNAMIC_ZLIB),
            (b"IEND", &[]),
        ]);
        assert_eq!(read_still_png(&doubled).err(), Some(StillError::Duplicate));
        let early = png_chunks(&[(b"IDAT", DYNAMIC_ZLIB), (b"IHDR", &header), (b"IEND", &[])]);
        assert_eq!(read_still_png(&early).err(), Some(StillError::ChunkOrder));
    }

    #[test]
    fn a_chunk_length_past_the_file_is_rejected_without_a_huge_buffer() {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&100u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        assert_eq!(read_still_png(&bytes).err(), Some(StillError::Truncated));
        let mut huge = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        huge.extend_from_slice(&u32::MAX.to_be_bytes());
        huge.extend_from_slice(b"IHDR");
        assert_eq!(read_still_png(&huge).err(), Some(StillError::Size));
    }

    #[test]
    fn two_idat_chunks_are_one_stream() {
        let split = DYNAMIC_ZLIB.len() / 2;
        let rgb = read_still_png(&png_chunks(&[
            (b"IHDR", &ihdr(2, 0)),
            (b"IDAT", &DYNAMIC_ZLIB[..split]),
            (b"IDAT", &DYNAMIC_ZLIB[split..]),
            (b"IEND", &[]),
        ]))
        .expect("png");
        assert_eq!(first3(&rgb), Some(vec![9, 8, 7]));
    }

    #[test]
    fn the_file_ceilings_match_the_supported_subset() {
        assert_eq!(8 + (12 + 13) + 12 + 262_144 * (12 + 1), 3_407_917);
        assert_eq!(STILL_PNG_FILE_LIMIT, 3_407_917);
        assert_eq!(10 + 65_535 + STILL_DEPTH_WORDS * 2, 219_145);
        assert_eq!(STILL_DEPTH_FILE_LIMIT, 219_145);
        assert_eq!(STILL_CHECK_RGB_LIMIT, 230_400);
        assert_eq!(STILL_CHECK_DEPTH_LIMIT, 153_600);
        assert_eq!(STILL_FRAME_LIMIT, 384_032);
    }

    #[test]
    fn an_empty_idat_beside_real_data_is_rejected() {
        let bytes = png_chunks(&[
            (b"IHDR", &ihdr(2, 0)),
            (b"IDAT", &[]),
            (b"IDAT", DYNAMIC_ZLIB),
            (b"IEND", &[]),
        ]);
        assert_eq!(read_still_png(&bytes).err(), Some(StillError::Size));
    }

    #[test]
    fn a_numpy_2_still_depth_is_rejected() {
        let text = "{'descr': '<u2', 'fortran_order': False, 'shape': (240, 320), }\n";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.push(2);
        bytes.push(0);
        bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
        bytes.extend(text.as_bytes());
        bytes.extend(std::iter::repeat_n(0u8, STILL_DEPTH_WORDS * 2));
        assert_eq!(read_still_depth(&bytes).err(), Some(StillError::Depth));
    }

    #[test]
    fn depth_keeps_zero_and_486_and_rejects_a_swapped_shape() {
        let mut words = vec![0u16; STILL_DEPTH_WORDS];
        words[0] = 0;
        words[1] = 486;
        words[2] = 500;
        words[3] = 65535;
        let depth = npy_u16(STILL_HEIGHT, STILL_WIDTH, &words, "<u2", false);
        let got = read_still_depth(&depth).expect("depth");
        assert_eq!(got.len(), STILL_DEPTH_WORDS);
        assert_eq!(got[0], 0);
        assert_eq!(got[1], 486);
        assert_eq!(got[2], 500);
        assert_eq!(got[3], 65535);
        assert_eq!(depth_bins(&got), (STILL_DEPTH_WORDS - 3, 1, 2));
        let swapped = npy_u16(STILL_WIDTH, STILL_HEIGHT, &words, "<u2", false);
        assert_eq!(read_still_depth(&swapped).err(), Some(StillError::Shape));
        let fortran = npy_u16(STILL_HEIGHT, STILL_WIDTH, &words, "<u2", true);
        assert_eq!(read_still_depth(&fortran).err(), Some(StillError::Order));
        let big = npy_u16(STILL_HEIGHT, STILL_WIDTH, &words, ">u2", false);
        assert_eq!(read_still_depth(&big).err(), Some(StillError::Depth));
        let mut extra = depth.clone();
        extra.push(0);
        assert_eq!(read_still_depth(&extra).err(), Some(StillError::Size));
    }

    #[test]
    fn a_pair_keeps_the_png_bytes_and_the_depth_words() {
        let mut words = vec![928u16; STILL_DEPTH_WORDS];
        words[10] = 0;
        words[11] = 486;
        let pair = read_still_pair(
            &png(ihdr(2, 0), DYNAMIC_ZLIB),
            &npy_u16(STILL_HEIGHT, STILL_WIDTH, &words, "<u2", false),
        )
        .expect("pair");
        assert_eq!(pair.width, STILL_WIDTH);
        assert_eq!(pair.height, STILL_HEIGHT);
        assert_eq!(first3(&pair.rgb), Some(vec![9, 8, 7]));
        assert_eq!(pair.depth_mm[10], 0);
        assert_eq!(pair.depth_mm[11], 486);
        assert_eq!(pair.rgb.len(), STILL_RGB_BYTES);
        assert_eq!(pair.depth_mm.len(), STILL_DEPTH_WORDS);
    }
}
