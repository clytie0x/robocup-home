//! NumPy `.npy` version 1 and 2 headers. Pixel arrays are loaded only for `<u2`.

#[derive(Clone, Debug, PartialEq)]
pub struct NpyInfo {
    pub descr: String,
    pub fortran: bool,
    pub shape: Vec<usize>,
    pub data_offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepthFrame {
    pub info: NpyInfo,
    pub mm: Vec<u16>,
}

pub fn inspect_npy(bytes: &[u8]) -> Result<NpyInfo, String> {
    if bytes.len() < 10 || &bytes[0..6] != b"\x93NUMPY" {
        return Err("not an npy file".to_string());
    }
    let major = bytes[6];
    let header_len: usize = match major {
        1 => u16::from_le_bytes([bytes[8], bytes[9]]) as usize,
        2 | 3 => {
            if bytes.len() < 12 {
                return Err("truncated npy header".to_string());
            }
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize
        }
        _ => return Err(format!("unsupported npy version {major}")),
    };
    let header_at: usize = if major == 1 { 10 } else { 12 };
    let header_end = header_at
        .checked_add(header_len)
        .ok_or_else(|| "npy header length".to_string())?;
    if header_end > bytes.len() {
        return Err("truncated npy header".to_string());
    }
    let header = std::str::from_utf8(&bytes[header_at..header_end])
        .map_err(|_| "npy header is not utf-8".to_string())?;
    let descr =
        between(header, "'descr': '", "'").ok_or_else(|| "npy descr missing".to_string())?;
    let fortran = if header.contains("'fortran_order': False") {
        false
    } else if header.contains("'fortran_order': True") {
        true
    } else {
        return Err("npy fortran flag missing".to_string());
    };
    let shape_text =
        between(header, "'shape': (", ")").ok_or_else(|| "npy shape missing".to_string())?;
    let mut shape = Vec::new();
    for part in shape_text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        shape.push(
            part.parse::<usize>()
                .map_err(|_| format!("bad shape {part}"))?,
        );
    }
    if shape.is_empty() {
        return Err("empty npy shape".to_string());
    }
    Ok(NpyInfo {
        descr,
        fortran,
        shape,
        data_offset: header_end,
    })
}

pub fn load_depth_u16(bytes: &[u8]) -> Result<DepthFrame, String> {
    let info = inspect_npy(bytes)?;
    if info.descr != "<u2" {
        return Err(format!("depth descr {} is not <u2", info.descr));
    }
    if info.fortran {
        return Err("fortran order is not loaded".to_string());
    }
    if info.shape.len() != 2 {
        return Err("depth shape is not rank 2".to_string());
    }
    let count = info.shape[0]
        .checked_mul(info.shape[1])
        .ok_or_else(|| "shape overflow".to_string())?;
    let bytes_needed = count
        .checked_mul(2)
        .ok_or_else(|| "shape overflow".to_string())?;
    let end = info
        .data_offset
        .checked_add(bytes_needed)
        .ok_or_else(|| "shape overflow".to_string())?;
    if end > bytes.len() {
        return Err("truncated depth payload".to_string());
    }
    let mut mm = Vec::with_capacity(count);
    let payload = &bytes[info.data_offset..end];
    let (pairs, rest) = payload.as_chunks::<2>();
    if !rest.is_empty() {
        return Err("truncated depth payload".to_string());
    }
    for pair in pairs {
        mm.push(u16::from_le_bytes(*pair));
    }
    if mm.len() != count {
        return Err("truncated depth payload".to_string());
    }
    Ok(DepthFrame { info, mm })
}

#[cfg(test)]
pub fn encode_npy_u16(height: usize, width: usize, mm: &[u16]) -> Vec<u8> {
    let mut header =
        format!("{{'descr': '<u2', 'fortran_order': False, 'shape': ({height}, {width}), }}");
    let preamble = 10;
    let pad = (16 - ((preamble + header.len() + 1) % 16)) % 16;
    header.push_str(&" ".repeat(pad));
    header.push('\n');
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x93NUMPY");
    bytes.push(1);
    bytes.push(0);
    bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
    bytes.extend_from_slice(header.as_bytes());
    for value in mm {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn between(text: &str, start: &str, end: &str) -> Option<String> {
    let from = text.find(start)? + start.len();
    let rest = &text[from..];
    let to = rest.find(end)?;
    Some(rest[..to].to_string())
}

#[cfg(test)]
mod tests {
    use super::{encode_npy_u16, inspect_npy, load_depth_u16};

    #[test]
    fn depth_round_trip_keeps_zeros_and_fortran_is_refused() {
        let bytes = encode_npy_u16(2, 2, &[0, 400, 500, 1000]);
        let frame = load_depth_u16(&bytes).unwrap();
        assert_eq!(frame.mm, vec![0, 400, 500, 1000]);
        let info = inspect_npy(&bytes).unwrap();
        let mut header = bytes[..info.data_offset].to_vec();
        let text = String::from_utf8(header[10..].to_vec())
            .unwrap()
            .replace("False", "True ");
        header.truncate(10);
        header.extend(text.bytes());
        let info = inspect_npy(&header).unwrap();
        assert!(info.fortran);
        assert!(load_depth_u16(&header).is_err());
        assert!(load_depth_u16(b"not npy").is_err());
    }
}
