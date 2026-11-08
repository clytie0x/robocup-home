//! Offline reader for the measured 39-frame `camera.npz`.
//!
//! The walker accepts that one archive shape. It is not a general ZIP reader.
//! `zip_entries` still stops on method 8 and on a 32-bit size of `u32::MAX`.
//! The stamps are the host monotonic samples stored in the file. They are not
//! a device clock, and the arrays are not a registered or fresh observation.

use crate::inflate_zlib::inflate_raw_capped;
use crate::npy::{NpyInfo, inspect_npy};
use crate::still::StillError;

const COLOR_UNCOMPRESSED: u64 = 8_985_728;
const DEPTH_UNCOMPRESSED: u64 = 5_990_528;
const STAMP_UNCOMPRESSED: u64 = 440;
const COLOR_PAYLOAD: usize = 8_985_600;
const DEPTH_PAYLOAD: usize = 5_990_400;
const STAMP_PAYLOAD: usize = 312;
const COLOR_COMPRESSED: usize = 5_561_734;
const DEPTH_COMPRESSED: usize = 1_002_834;
const STAMP_COMPRESSED: usize = 342;

const fn zip_local(name_len: usize) -> usize {
    30 + name_len + 20
}

const fn zip_central(name_len: usize) -> usize {
    46 + name_len
}

const fn frame_section(name_len: usize, payload: usize) -> usize {
    2 + name_len + 8 + payload
}

/// Bytes the CLI may read from the measured archive shape.
///
/// Three method-8 members, local zip64 extras, central headers with empty
/// extras, and one 22-byte end record. Compressed sizes are the documented
/// lengths. This is a read ceiling, not a hash check.
pub const ARCHIVE_FILE_LIMIT: usize = COLOR_COMPRESSED
    + DEPTH_COMPRESSED
    + STAMP_COMPRESSED
    + zip_local(9)
    + zip_local(12)
    + zip_local(15)
    + zip_central(9)
    + zip_central(12)
    + zip_central(15)
    + 22;

/// Bytes the CLI may read from one `NPZ1` comparison frame.
///
/// The frame is the magic, then `rgb`, `depth`, and `stamps` at the exact
/// payload lengths. A longer stream is not a valid comparison.
pub const COMPARE_FRAME_LIMIT: usize = 4
    + frame_section(3, COLOR_PAYLOAD)
    + frame_section(5, DEPTH_PAYLOAD)
    + frame_section(6, STAMP_PAYLOAD);
const NAMES: [&str; 3] = ["color.npy", "depth_mm.npy", "t_monotonic.npy"];
const SIZES: [u64; 3] = [COLOR_UNCOMPRESSED, DEPTH_UNCOMPRESSED, STAMP_UNCOMPRESSED];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveError {
    Truncated,
    Size,
    Offset,
    Flag,
    Encryption,
    Descriptor,
    Method,
    Name,
    Order,
    Duplicate,
    Gap,
    Overlap,
    Trailing,
    Disk,
    Zip64End,
    Crc,
    Conflict,
    Header,
    Inflate,
    Npy,
    Version,
    Dtype,
    Shape,
    Fortran,
    Stamp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraArchive {
    pub rgb: Vec<u8>,
    pub depth_mm: Vec<u16>,
    pub stamps: Vec<f64>,
}

pub(crate) fn zip_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

struct Local {
    start: usize,
    flags: u16,
    method: u16,
    crc: u32,
    name: String,
    compressed: u64,
    uncompressed: u64,
    data_at: usize,
    data_end: usize,
}

pub fn read_camera_npz(bytes: &[u8]) -> Result<CameraArchive, ArchiveError> {
    let locals = read_locals(bytes)?;
    let (central_at, central_end) = read_centrals(bytes, &locals)?;
    read_end(bytes, central_at, central_end, locals.len())?;
    let mut inflated = Vec::with_capacity(locals.len());
    for local in &locals {
        let cap = usize::try_from(local.uncompressed).map_err(|_| ArchiveError::Size)?;
        let raw =
            inflate_raw_capped(&bytes[local.data_at..local.data_end], cap).map_err(map_inflate)?;
        if raw.len() != cap {
            return Err(ArchiveError::Size);
        }
        if zip_crc32(&raw) != local.crc {
            return Err(ArchiveError::Crc);
        }
        inflated.push(raw);
    }
    Ok(CameraArchive {
        rgb: parse_color(&inflated[0])?,
        depth_mm: parse_depth(&inflated[1])?,
        stamps: parse_stamps(&inflated[2])?,
    })
}

fn read_locals(bytes: &[u8]) -> Result<Vec<Local>, ArchiveError> {
    let mut locals: Vec<Local> = Vec::new();
    let mut pos = 0;
    while locals.len() < 3 {
        if pos + 30 > bytes.len() {
            return Err(ArchiveError::Truncated);
        }
        if bytes[pos..pos + 4] != [0x50, 0x4b, 0x03, 0x04] {
            if locals.is_empty() {
                return Err(ArchiveError::Header);
            }
            return Err(ArchiveError::Gap);
        }
        let version = u16_at(bytes, pos + 4);
        let flags = u16_at(bytes, pos + 6);
        let method = u16_at(bytes, pos + 8);
        let crc = u32_at(bytes, pos + 14);
        let comp32 = u32_at(bytes, pos + 18);
        let uncomp32 = u32_at(bytes, pos + 22);
        let name_len = usize::from(u16_at(bytes, pos + 26));
        let extra_len = usize::from(u16_at(bytes, pos + 28));
        if version != 45 {
            return Err(ArchiveError::Header);
        }
        if flags & 0x0001 != 0 {
            return Err(ArchiveError::Encryption);
        }
        if flags & 0x0008 != 0 {
            return Err(ArchiveError::Descriptor);
        }
        if flags != 0 {
            return Err(ArchiveError::Flag);
        }
        if method != 8 {
            return Err(ArchiveError::Method);
        }
        if comp32 != u32::MAX || uncomp32 != u32::MAX {
            return Err(ArchiveError::Header);
        }
        let name_at = pos + 30;
        let name_end = name_at.checked_add(name_len).ok_or(ArchiveError::Offset)?;
        let extra_end = name_end
            .checked_add(extra_len)
            .ok_or(ArchiveError::Offset)?;
        if extra_end > bytes.len() {
            return Err(ArchiveError::Truncated);
        }
        let name = std::str::from_utf8(&bytes[name_at..name_end])
            .map_err(|_| ArchiveError::Header)?
            .to_string();
        let (uncompressed, compressed) = zip64_sizes(&bytes[name_end..extra_end])?;
        if locals.iter().any(|local| local.name == name) {
            return Err(ArchiveError::Duplicate);
        }
        if name != NAMES[locals.len()] {
            if NAMES.contains(&name.as_str()) {
                return Err(ArchiveError::Order);
            }
            return Err(ArchiveError::Name);
        }
        if uncompressed != SIZES[locals.len()] {
            return Err(ArchiveError::Size);
        }
        let comp_len = usize::try_from(compressed).map_err(|_| ArchiveError::Offset)?;
        let data_end = extra_end
            .checked_add(comp_len)
            .ok_or(ArchiveError::Offset)?;
        if data_end > bytes.len() {
            return Err(ArchiveError::Truncated);
        }
        locals.push(Local {
            start: pos,
            flags,
            method,
            crc,
            name,
            compressed,
            uncompressed,
            data_at: extra_end,
            data_end,
        });
        pos = data_end;
    }
    if pos + 4 <= bytes.len() && bytes[pos..pos + 4] == [0x50, 0x4b, 0x03, 0x04] {
        return Err(ArchiveError::Name);
    }
    Ok(locals)
}

fn zip64_sizes(extra: &[u8]) -> Result<(u64, u64), ArchiveError> {
    let mut pos = 0;
    let mut found = None;
    while pos + 4 <= extra.len() {
        let id = u16_at(extra, pos);
        let size = usize::from(u16_at(extra, pos + 2));
        let data_at = pos + 4;
        let data_end = data_at.checked_add(size).ok_or(ArchiveError::Offset)?;
        if data_end > extra.len() || id != 1 || found.is_some() || size != 16 {
            return Err(ArchiveError::Header);
        }
        found = Some((u64_at(extra, data_at), u64_at(extra, data_at + 8)));
        pos = data_end;
    }
    if pos != extra.len() {
        return Err(ArchiveError::Header);
    }
    found.ok_or(ArchiveError::Header)
}

fn read_centrals(bytes: &[u8], locals: &[Local]) -> Result<(usize, usize), ArchiveError> {
    let mut pos = locals.last().ok_or(ArchiveError::Header)?.data_end;
    let central_at = pos;
    for local in locals {
        if pos + 46 > bytes.len() {
            return Err(ArchiveError::Truncated);
        }
        if bytes[pos..pos + 4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err(ArchiveError::Header);
        }
        let version = u16_at(bytes, pos + 6);
        let flags = u16_at(bytes, pos + 8);
        let method = u16_at(bytes, pos + 10);
        let crc = u32_at(bytes, pos + 16);
        let comp32 = u32_at(bytes, pos + 20);
        let uncomp32 = u32_at(bytes, pos + 24);
        let name_len = usize::from(u16_at(bytes, pos + 28));
        let extra_len = usize::from(u16_at(bytes, pos + 30));
        let comment_len = usize::from(u16_at(bytes, pos + 32));
        let disk = u16_at(bytes, pos + 34);
        let offset = u32_at(bytes, pos + 42) as usize;
        if version != 45 || disk != 0 || comment_len != 0 || extra_len != 0 {
            return Err(ArchiveError::Header);
        }
        if flags != local.flags || method != local.method || crc != local.crc {
            return Err(ArchiveError::Conflict);
        }
        if u64::from(comp32) != local.compressed || u64::from(uncomp32) != local.uncompressed {
            return Err(ArchiveError::Conflict);
        }
        let name_at = pos + 46;
        let name_end = name_at.checked_add(name_len).ok_or(ArchiveError::Offset)?;
        let end = name_end
            .checked_add(extra_len)
            .and_then(|value| value.checked_add(comment_len))
            .ok_or(ArchiveError::Offset)?;
        if end > bytes.len() {
            return Err(ArchiveError::Truncated);
        }
        let name =
            std::str::from_utf8(&bytes[name_at..name_end]).map_err(|_| ArchiveError::Header)?;
        if name != local.name {
            return Err(ArchiveError::Conflict);
        }
        if offset != local.start {
            let inside = locals
                .iter()
                .any(|item| offset >= item.data_at && offset < item.data_end);
            if inside {
                return Err(ArchiveError::Overlap);
            }
            return Err(ArchiveError::Conflict);
        }
        pos = end;
    }
    Ok((central_at, pos))
}

fn read_end(
    bytes: &[u8],
    central_at: usize,
    central_end: usize,
    count: usize,
) -> Result<(), ArchiveError> {
    if central_end + 4 > bytes.len() {
        return Err(ArchiveError::Truncated);
    }
    let sig = &bytes[central_end..central_end + 4];
    if sig == [0x50, 0x4b, 0x06, 0x06] || sig == [0x50, 0x4b, 0x06, 0x07] {
        return Err(ArchiveError::Zip64End);
    }
    if sig != [0x50, 0x4b, 0x05, 0x06] {
        return Err(ArchiveError::Header);
    }
    if central_end + 22 > bytes.len() {
        return Err(ArchiveError::Truncated);
    }
    let disk = u16_at(bytes, central_end + 4);
    let cd_disk = u16_at(bytes, central_end + 6);
    let entries_here = usize::from(u16_at(bytes, central_end + 8));
    let entries = usize::from(u16_at(bytes, central_end + 10));
    let cd_size = u32_at(bytes, central_end + 12) as usize;
    let cd_offset = u32_at(bytes, central_end + 16) as usize;
    let comment = usize::from(u16_at(bytes, central_end + 20));
    if disk != 0 || cd_disk != 0 {
        return Err(ArchiveError::Disk);
    }
    if comment != 0 || central_end + 22 + comment != bytes.len() {
        return Err(ArchiveError::Trailing);
    }
    if entries_here != count
        || entries != count
        || cd_size != central_end - central_at
        || cd_offset != central_at
    {
        return Err(ArchiveError::Header);
    }
    Ok(())
}

fn parse_color(bytes: &[u8]) -> Result<Vec<u8>, ArchiveError> {
    let (info, payload) = npy_bytes(bytes)?;
    expect_member(&info, "|u1", &[39, 240, 320, 3], COLOR_PAYLOAD, bytes.len())?;
    Ok(payload.to_vec())
}

fn parse_depth(bytes: &[u8]) -> Result<Vec<u16>, ArchiveError> {
    let (info, payload) = npy_bytes(bytes)?;
    expect_member(&info, "<u2", &[39, 240, 320], DEPTH_PAYLOAD, bytes.len())?;
    let (pairs, rest) = payload.as_chunks::<2>();
    if !rest.is_empty() {
        return Err(ArchiveError::Npy);
    }
    Ok(pairs.iter().copied().map(u16::from_le_bytes).collect())
}

fn parse_stamps(bytes: &[u8]) -> Result<Vec<f64>, ArchiveError> {
    let (info, payload) = npy_bytes(bytes)?;
    expect_member(&info, "<f8", &[39], STAMP_PAYLOAD, bytes.len())?;
    let (pairs, rest) = payload.as_chunks::<8>();
    if !rest.is_empty() {
        return Err(ArchiveError::Npy);
    }
    let mut stamps = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let value = f64::from_le_bytes(*pair);
        if !value.is_finite() {
            return Err(ArchiveError::Stamp);
        }
        if stamps.last().is_some_and(|previous| value <= *previous) {
            return Err(ArchiveError::Stamp);
        }
        stamps.push(value);
    }
    if stamps.len() != 39 {
        return Err(ArchiveError::Stamp);
    }
    Ok(stamps)
}

fn npy_bytes(bytes: &[u8]) -> Result<(NpyInfo, &[u8]), ArchiveError> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err(ArchiveError::Npy);
    }
    if bytes[6] != 1 || bytes[7] != 0 {
        return Err(ArchiveError::Version);
    }
    let info = inspect_npy(bytes).map_err(|_| ArchiveError::Npy)?;
    let offset = info.data_offset;
    if offset > bytes.len() {
        return Err(ArchiveError::Truncated);
    }
    Ok((info, &bytes[offset..]))
}

fn expect_member(
    info: &NpyInfo,
    descr: &str,
    shape: &[usize],
    payload_len: usize,
    bytes_len: usize,
) -> Result<(), ArchiveError> {
    if info.fortran {
        return Err(ArchiveError::Fortran);
    }
    if info.descr != descr {
        return Err(ArchiveError::Dtype);
    }
    if info.shape != shape {
        return Err(ArchiveError::Shape);
    }
    let end = info
        .data_offset
        .checked_add(payload_len)
        .ok_or(ArchiveError::Offset)?;
    if end > bytes_len {
        return Err(ArchiveError::Truncated);
    }
    if end != bytes_len {
        return Err(ArchiveError::Trailing);
    }
    Ok(())
}

fn map_inflate(err: StillError) -> ArchiveError {
    match err {
        StillError::Truncated => ArchiveError::Truncated,
        StillError::Size => ArchiveError::Size,
        StillError::Trailing => ArchiveError::Trailing,
        _ => ArchiveError::Inflate,
    }
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

#[derive(Debug)]
pub struct CompareParts<'a> {
    pub rgb: &'a [u8],
    pub depth: &'a [u8],
    pub stamps: &'a [u8],
}

pub fn split_compare_frame(framed: &[u8]) -> Result<CompareParts<'_>, String> {
    if framed.len() < 4 || &framed[..4] != b"NPZ1" {
        return Err("frame header".to_string());
    }
    let mut pos = 4;
    let rgb = take_frame_section(&mut pos, framed, "rgb")?;
    let depth = take_frame_section(&mut pos, framed, "depth")?;
    let stamps = take_frame_section(&mut pos, framed, "stamps")?;
    if pos != framed.len() {
        return Err(format!("frame trailing offset={pos}"));
    }
    Ok(CompareParts { rgb, depth, stamps })
}

fn take_frame_section<'a>(
    pos: &mut usize,
    framed: &'a [u8],
    expect: &str,
) -> Result<&'a [u8], String> {
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
    let expected = match expect {
        "rgb" => COLOR_PAYLOAD,
        "depth" => DEPTH_PAYLOAD,
        "stamps" => STAMP_PAYLOAD,
        _ => return Err(format!("frame name {expect}")),
    };
    if len != expected as u64 {
        return Err(format!("{expect} length"));
    }
    let end = pos
        .checked_add(expected)
        .ok_or_else(|| "frame length".to_string())?;
    if end > framed.len() {
        return Err(format!("{expect} truncated payload"));
    }
    let out = &framed[*pos..end];
    *pos = end;
    Ok(out)
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        bytes[at],
        bytes[at + 1],
        bytes[at + 2],
        bytes[at + 3],
        bytes[at + 4],
        bytes[at + 5],
        bytes[at + 6],
        bytes[at + 7],
    ])
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::{ArchiveError, read_camera_npz, zip_crc32};
    use crate::zipnote::zip_entries;

    const COLOR_RAW: usize = 8_985_728;
    const DEPTH_RAW: usize = 5_990_528;
    const STAMP_RAW: usize = 440;
    const COLOR_PAYLOAD: usize = 8_985_600;
    const DEPTH_WORDS: usize = 2_995_200;

    fn test_crc(data: &[u8]) -> u32 {
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

    fn stored_raw(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut rest = data;
        if rest.is_empty() {
            out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
            return out;
        }
        while !rest.is_empty() {
            let n = rest.len().min(65535);
            let last = n == rest.len();
            out.push(if last { 1 } else { 0 });
            let n16 = n as u16;
            out.extend_from_slice(&n16.to_le_bytes());
            out.extend_from_slice(&(!n16).to_le_bytes());
            out.extend_from_slice(&rest[..n]);
            rest = &rest[n..];
        }
        out
    }

    fn npy_member(descr: &str, shape: &str, payload: &[u8], total: usize) -> Vec<u8> {
        let header_len = total - payload.len() - 10;
        let mut text =
            format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': ({shape}), }}");
        assert!(
            text.len() < header_len,
            "header {len} does not fit {header_len}",
            len = text.len()
        );
        text.push_str(&" ".repeat(header_len - text.len() - 1));
        text.push('\n');
        assert_eq!(text.len(), header_len);
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"\x93NUMPY");
        out.push(1);
        out.push(0);
        out.extend_from_slice(&(header_len as u16).to_le_bytes());
        out.extend_from_slice(text.as_bytes());
        out.extend_from_slice(payload);
        assert_eq!(out.len(), total);
        out
    }

    fn color_payload() -> Vec<u8> {
        (0..COLOR_PAYLOAD).map(|i| (i % 251) as u8).collect()
    }

    fn depth_payload() -> Vec<u8> {
        let mut payload = Vec::with_capacity(DEPTH_WORDS * 2);
        for i in 0..DEPTH_WORDS {
            let word = (i % 3000) as u16 + 500;
            payload.extend_from_slice(&word.to_le_bytes());
        }
        payload
    }

    fn stamp_values(kind: StampKind) -> Vec<f64> {
        match kind {
            StampKind::Good => (0..39).map(|i| 1_000.0 + i as f64 * 0.25).collect(),
            StampKind::Nan => {
                let mut values = stamp_values(StampKind::Good);
                values[3] = f64::NAN;
                values
            }
            StampKind::Flat => vec![1.0; 39],
            StampKind::Backward => (0..39).map(|i| 50.0 - i as f64).collect(),
        }
    }

    enum StampKind {
        Good,
        Nan,
        Flat,
        Backward,
    }

    fn stamp_bytes(kind: StampKind) -> Vec<u8> {
        let values = stamp_values(kind);
        let mut payload = Vec::with_capacity(312);
        for value in values {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        npy_member("<f8", "39,", &payload, STAMP_RAW)
    }

    struct Full {
        color: Vec<u8>,
        depth: Vec<u8>,
        stamps: Vec<u8>,
    }

    fn full() -> &'static Full {
        static FULL: LazyLock<Full> = LazyLock::new(|| Full {
            color: npy_member("|u1", "39, 240, 320, 3", &color_payload(), COLOR_RAW),
            depth: npy_member("<u2", "39, 240, 320", &depth_payload(), DEPTH_RAW),
            stamps: stamp_bytes(StampKind::Good),
        });
        &FULL
    }

    #[derive(Clone)]
    struct Member {
        name: String,
        flags: u16,
        method: u16,
        version: u16,
        crc: u32,
        comp: u64,
        uncomp: u64,
        payload: Vec<u8>,
        central_extra: Vec<u8>,
        central_uncomp: Option<u32>,
        central_crc: Option<u32>,
        central_offset: Option<u32>,
    }

    fn member(name: &str, uncomp: u64) -> Member {
        Member {
            name: name.to_string(),
            flags: 0,
            method: 8,
            version: 45,
            crc: 0,
            comp: 1,
            uncomp,
            payload: vec![0],
            central_extra: Vec::new(),
            central_uncomp: None,
            central_crc: None,
            central_offset: None,
        }
    }

    fn stubs() -> Vec<Member> {
        vec![
            member("color.npy", COLOR_RAW as u64),
            member("depth_mm.npy", DEPTH_RAW as u64),
            member("t_monotonic.npy", STAMP_RAW as u64),
        ]
    }

    fn with_raw(slot: &mut Member, raw: &[u8]) {
        slot.uncomp = raw.len() as u64;
        slot.crc = test_crc(raw);
        slot.payload = stored_raw(raw);
        slot.comp = slot.payload.len() as u64;
    }

    struct Opts {
        gap_at: Option<usize>,
        fourth: bool,
        disk: u16,
        cd_disk: u16,
        eocd_sig: u32,
        before_eocd: Vec<u8>,
        tail: Vec<u8>,
    }

    fn opts() -> Opts {
        Opts {
            gap_at: None,
            fourth: false,
            disk: 0,
            cd_disk: 0,
            eocd_sig: 0x0605_4b50,
            before_eocd: Vec::new(),
            tail: Vec::new(),
        }
    }

    struct Located {
        bytes: Vec<u8>,
        data_at: Vec<usize>,
    }

    fn write_u16(out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn build(members: &[Member], opt: &Opts) -> Located {
        let mut owned = members.to_vec();
        if opt.fourth {
            owned.push(member("extra.npy", COLOR_RAW as u64));
        }
        let mut bytes = Vec::new();
        let mut starts = Vec::new();
        let mut data_at = Vec::new();
        for (index, member) in owned.iter().enumerate() {
            if opt.gap_at == Some(index) {
                bytes.push(0);
            }
            starts.push(bytes.len());
            write_u32(&mut bytes, 0x0403_4b50);
            write_u16(&mut bytes, member.version);
            write_u16(&mut bytes, member.flags);
            write_u16(&mut bytes, member.method);
            write_u16(&mut bytes, 0);
            write_u16(&mut bytes, 0);
            write_u32(&mut bytes, member.crc);
            write_u32(&mut bytes, u32::MAX);
            write_u32(&mut bytes, u32::MAX);
            write_u16(&mut bytes, member.name.len() as u16);
            write_u16(&mut bytes, 20);
            bytes.extend_from_slice(member.name.as_bytes());
            write_u16(&mut bytes, 1);
            write_u16(&mut bytes, 16);
            write_u64(&mut bytes, member.uncomp);
            write_u64(&mut bytes, member.comp);
            data_at.push(bytes.len());
            if member.comp != u64::MAX {
                let n = member.comp as usize;
                assert!(member.payload.len() >= n);
                bytes.extend_from_slice(&member.payload[..n]);
            }
        }
        let central_at = bytes.len();
        for (index, member) in owned.iter().enumerate() {
            write_u32(&mut bytes, 0x0201_4b50);
            write_u16(&mut bytes, 0x032d);
            write_u16(&mut bytes, 45);
            write_u16(&mut bytes, member.flags);
            write_u16(&mut bytes, member.method);
            write_u16(&mut bytes, 0);
            write_u16(&mut bytes, 0);
            write_u32(&mut bytes, member.central_crc.unwrap_or(member.crc));
            let comp = u32::try_from(member.comp).unwrap_or(u32::MAX);
            write_u32(&mut bytes, comp);
            write_u32(
                &mut bytes,
                member
                    .central_uncomp
                    .unwrap_or(u32::try_from(member.uncomp).unwrap_or(u32::MAX)),
            );
            write_u16(&mut bytes, member.name.len() as u16);
            write_u16(&mut bytes, member.central_extra.len() as u16);
            write_u16(&mut bytes, 0);
            write_u16(&mut bytes, 0);
            write_u16(&mut bytes, 0);
            write_u32(&mut bytes, 0);
            write_u32(
                &mut bytes,
                member.central_offset.unwrap_or(starts[index] as u32),
            );
            bytes.extend_from_slice(member.name.as_bytes());
            bytes.extend_from_slice(&member.central_extra);
        }
        let central_end = bytes.len();
        bytes.extend_from_slice(&opt.before_eocd);
        write_u32(&mut bytes, opt.eocd_sig);
        write_u16(&mut bytes, opt.disk);
        write_u16(&mut bytes, opt.cd_disk);
        let count = owned.len() as u16;
        write_u16(&mut bytes, count);
        write_u16(&mut bytes, count);
        write_u32(&mut bytes, (central_end - central_at) as u32);
        write_u32(&mut bytes, central_at as u32);
        write_u16(&mut bytes, 0);
        bytes.extend_from_slice(&opt.tail);
        Located { bytes, data_at }
    }

    fn archive(members: &[Member]) -> Vec<u8> {
        build(members, &opts()).bytes
    }

    fn content(color: &[u8], depth: &[u8], stamps: &[u8]) -> Vec<u8> {
        let mut members = stubs();
        with_raw(&mut members[0], color);
        with_raw(&mut members[1], depth);
        with_raw(&mut members[2], stamps);
        archive(&members)
    }

    fn assert_same_bytes(got: &[u8], expected: &[u8]) {
        assert_eq!(got.len(), expected.len());
        if let Some(index) = got.iter().zip(expected).position(|(a, b)| a != b) {
            assert_eq!(got[index], expected[index], "index {index}");
        }
    }

    #[test]
    fn zip_crc32_matches_the_iso_check_vector() {
        assert_eq!(zip_crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn zip_entries_stays_closed_for_method_8_and_u32_max() {
        let bytes = archive(&stubs());
        let entries = zip_entries(&bytes).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].blocked);
        assert_eq!(entries[0].method, 8);
        assert_eq!(entries[0].name, "color.npy");
    }

    #[test]
    fn a_synthetic_archive_returns_every_rgb_byte_depth_word_and_stamp() {
        let got = read_camera_npz(&content(&full().color, &full().depth, &full().stamps)).unwrap();
        assert_eq!(got.rgb.len(), COLOR_PAYLOAD);
        assert_eq!(got.depth_mm.len(), DEPTH_WORDS);
        assert_eq!(got.stamps.len(), 39);
        assert_eq!(got.depth_mm[0], 500);
        assert_eq!(got.stamps[0], 1_000.0);
        assert_eq!(got.stamps[38], 1_000.0 + 38.0 * 0.25);
        assert!(got.stamps.windows(2).all(|pair| pair[1] > pair[0]));
        assert_same_bytes(&got.rgb, &color_payload());
        let mut previous = &got.rgb[..240 * 320 * 3];
        for frame in got.rgb.chunks(240 * 320 * 3).skip(1) {
            assert_ne!(frame, previous);
            previous = frame;
        }
        let mut previous = &got.depth_mm[..240 * 320];
        for frame in got.depth_mm.chunks(240 * 320).skip(1) {
            assert_ne!(frame, previous);
            previous = frame;
        }
    }

    #[test]
    fn the_encryption_flag_is_rejected() {
        let mut members = stubs();
        members[0].flags = 1;
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Encryption)
        );
    }

    #[test]
    fn the_data_descriptor_flag_is_rejected() {
        let mut members = stubs();
        members[0].flags = 0x0008;
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Descriptor)
        );
    }

    #[test]
    fn an_unsupported_flag_bit_is_rejected() {
        let mut members = stubs();
        members[0].flags = 0x0800;
        assert_eq!(read_camera_npz(&archive(&members)), Err(ArchiveError::Flag));
    }

    #[test]
    fn a_method_other_than_deflate_is_rejected() {
        let mut members = stubs();
        members[0].method = 0;
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Method)
        );
    }

    #[test]
    fn a_declared_size_other_than_the_member_is_rejected() {
        let mut members = stubs();
        members[0].uncomp = 440;
        assert_eq!(read_camera_npz(&archive(&members)), Err(ArchiveError::Size));
    }

    #[test]
    fn a_u64_max_declared_size_is_rejected() {
        let mut members = stubs();
        members[0].uncomp = u64::MAX;
        assert_eq!(read_camera_npz(&archive(&members)), Err(ArchiveError::Size));
    }

    #[test]
    fn an_offset_that_overflows_is_rejected() {
        let mut members = stubs();
        members[0].comp = u64::MAX;
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Offset)
        );
    }

    #[test]
    fn a_duplicate_name_is_rejected() {
        let mut members = stubs();
        members[1].name = "color.npy".to_string();
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Duplicate)
        );
    }

    #[test]
    fn a_reordered_member_is_rejected() {
        let mut members = stubs();
        members[0].name = "depth_mm.npy".to_string();
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Order)
        );
    }

    #[test]
    fn an_unknown_name_is_rejected() {
        let mut members = stubs();
        members[0].name = "notes.txt".to_string();
        assert_eq!(read_camera_npz(&archive(&members)), Err(ArchiveError::Name));
    }

    #[test]
    fn a_fourth_member_is_rejected() {
        let mut opt = opts();
        opt.fourth = true;
        assert_eq!(
            read_camera_npz(&build(&stubs(), &opt).bytes),
            Err(ArchiveError::Name)
        );
    }

    #[test]
    fn a_gap_between_members_is_rejected() {
        let mut opt = opts();
        opt.gap_at = Some(1);
        assert_eq!(
            read_camera_npz(&build(&stubs(), &opt).bytes),
            Err(ArchiveError::Gap)
        );
    }

    #[test]
    fn a_trailing_archive_byte_is_rejected() {
        let mut opt = opts();
        opt.tail = vec![0];
        assert_eq!(
            read_camera_npz(&build(&stubs(), &opt).bytes),
            Err(ArchiveError::Trailing)
        );
    }

    #[test]
    fn a_truncated_archive_is_rejected() {
        let mut bytes = archive(&stubs());
        bytes.truncate(12);
        assert_eq!(read_camera_npz(&bytes), Err(ArchiveError::Truncated));
    }

    #[test]
    fn a_crc_mismatch_is_rejected() {
        let mut members = stubs();
        with_raw(&mut members[0], &full().color);
        with_raw(&mut members[1], &full().depth);
        with_raw(&mut members[2], &full().stamps);
        members[0].crc = members[0].crc.wrapping_add(1);
        members[0].central_crc = Some(members[0].crc);
        assert_eq!(
            read_camera_npz(&archive(&members)).map(|_| ()),
            Err(ArchiveError::Crc)
        );
    }

    #[test]
    fn a_central_crc_that_disagrees_is_rejected() {
        let mut members = stubs();
        members[0].central_crc = Some(1);
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Conflict)
        );
    }

    #[test]
    fn a_central_size_that_disagrees_is_rejected() {
        let mut members = stubs();
        members[1].central_uncomp = Some(7);
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Conflict)
        );
    }

    #[test]
    fn a_central_zip64_extra_is_rejected() {
        let mut members = stubs();
        members[0].central_extra =
            vec![1, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Header)
        );
    }

    #[test]
    fn a_multi_disk_end_record_is_rejected() {
        let mut opt = opts();
        opt.disk = 1;
        assert_eq!(
            read_camera_npz(&build(&stubs(), &opt).bytes),
            Err(ArchiveError::Disk)
        );
    }

    #[test]
    fn a_zip64_end_record_is_rejected() {
        let mut opt = opts();
        opt.eocd_sig = 0x0606_4b50;
        assert_eq!(
            read_camera_npz(&build(&stubs(), &opt).bytes),
            Err(ArchiveError::Zip64End)
        );
    }

    #[test]
    fn a_zip64_locator_is_rejected() {
        let mut opt = opts();
        opt.before_eocd = vec![
            0x50, 0x4b, 0x06, 0x07, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(
            read_camera_npz(&build(&stubs(), &opt).bytes),
            Err(ArchiveError::Zip64End)
        );
    }

    #[test]
    fn an_overlapping_central_offset_is_rejected() {
        let probe = build(&stubs(), &opts());
        let mut members = stubs();
        members[1].central_offset = Some(probe.data_at[0] as u32);
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Overlap)
        );
    }

    #[test]
    fn a_version_below_zip64_is_rejected() {
        let mut members = stubs();
        members[0].version = 20;
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Header)
        );
    }

    #[test]
    fn a_numpy_major_version_other_than_1_is_rejected() {
        let mut color = full().color.clone();
        color[6] = 2;
        let bytes = content(&color, &full().depth, &full().stamps);
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Version)
        );
    }

    #[test]
    fn fortran_order_is_rejected() {
        let mut color = full().color.clone();
        let header = std::str::from_utf8(&color[10..128]).unwrap().to_string();
        let swapped = header.replacen("False", "True ", 1);
        color[10..128].copy_from_slice(swapped.as_bytes());
        let bytes = content(&color, &full().depth, &full().stamps);
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Fortran)
        );
    }

    #[test]
    fn a_transposed_depth_shape_is_rejected() {
        let mut depth = full().depth.clone();
        let header = std::str::from_utf8(&depth[10..128]).unwrap().to_string();
        let swapped = header.replacen("(39, 240, 320)", "(39, 320, 240)", 1);
        depth[10..128].copy_from_slice(swapped.as_bytes());
        let bytes = content(&full().color, &depth, &full().stamps);
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Shape)
        );
    }

    #[test]
    fn a_big_endian_depth_descr_is_rejected() {
        let mut depth = full().depth.clone();
        let header = std::str::from_utf8(&depth[10..128]).unwrap().to_string();
        let swapped = header.replacen("<u2", ">u2", 1);
        depth[10..128].copy_from_slice(swapped.as_bytes());
        let bytes = content(&full().color, &depth, &full().stamps);
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Dtype)
        );
    }

    #[test]
    fn a_little_endian_colour_descr_is_rejected() {
        let mut color = full().color.clone();
        let header = std::str::from_utf8(&color[10..128]).unwrap().to_string();
        let swapped = header.replacen("|u1", "<u1", 1);
        color[10..128].copy_from_slice(swapped.as_bytes());
        let bytes = content(&color, &full().depth, &full().stamps);
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Dtype)
        );
    }

    #[test]
    fn bytes_after_the_numpy_payload_are_rejected() {
        let mut color = full().color.clone();
        let header_len = u16::from_le_bytes([color[8], color[9]]) as usize;
        assert!(header_len > 16);
        color[8..10].copy_from_slice(&((header_len - 16) as u16).to_le_bytes());
        let bytes = content(&color, &full().depth, &full().stamps);
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Trailing)
        );
    }

    #[test]
    fn a_non_finite_stamp_is_rejected() {
        let bytes = content(&full().color, &full().depth, &stamp_bytes(StampKind::Nan));
        assert_eq!(
            read_camera_npz(&bytes).map(|_| ()),
            Err(ArchiveError::Stamp)
        );
    }

    #[test]
    fn a_non_increasing_stamp_is_rejected() {
        let flat = content(&full().color, &full().depth, &stamp_bytes(StampKind::Flat));
        assert_eq!(read_camera_npz(&flat).map(|_| ()), Err(ArchiveError::Stamp));
        let backward = content(
            &full().color,
            &full().depth,
            &stamp_bytes(StampKind::Backward),
        );
        assert_eq!(
            read_camera_npz(&backward).map(|_| ()),
            Err(ArchiveError::Stamp)
        );
    }

    #[test]
    fn a_corrupt_deflate_payload_is_rejected() {
        let mut members = stubs();
        members[0].payload = vec![1, 1, 0, 0, 0, 0x7f];
        members[0].comp = 6;
        assert_eq!(
            read_camera_npz(&archive(&members)),
            Err(ArchiveError::Inflate)
        );
    }
}
