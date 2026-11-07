//! A capped zlib inflater for one still PNG.
//!
//! The bit reader and canonical Huffman decode follow puff (Mark Adler,
//! public domain) and RFC 1951. The output stops at `cap`.

use crate::still::StillError;

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const CODE_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

struct Huff {
    count: [u16; 16],
    symbol: [u16; 288],
}

struct Bits<'a> {
    input: &'a [u8],
    pos: usize,
    buf: u32,
    nbits: u8,
}

pub(crate) fn inflate_capped(input: &[u8], cap: usize) -> Result<Vec<u8>, StillError> {
    if input.len() < 2 {
        return Err(StillError::Truncated);
    }
    let cmf = input[0];
    let flg = input[1];
    if cmf & 0x0f != 8 || cmf >> 4 > 7 || flg & 0x20 != 0 {
        return Err(StillError::Inflate);
    }
    if (u16::from(cmf) * 256 + u16::from(flg)) % 31 != 0 {
        return Err(StillError::Inflate);
    }
    let mut bits = Bits {
        input,
        pos: 2,
        buf: 0,
        nbits: 0,
    };
    let out = inflate_blocks(&mut bits, cap)?;
    bits.align()?;
    if bits.pos.checked_add(4).ok_or(StillError::Size)? > input.len() {
        return Err(StillError::Truncated);
    }
    let adler = u32::from_be_bytes([
        input[bits.pos],
        input[bits.pos + 1],
        input[bits.pos + 2],
        input[bits.pos + 3],
    ]);
    bits.pos += 4;
    if adler32(&out) != adler {
        return Err(StillError::Adler);
    }
    if bits.pos != input.len() {
        return Err(StillError::Trailing);
    }
    Ok(out)
}

fn stored_block(bits: &mut Bits<'_>, out: &mut Vec<u8>, cap: usize) -> Result<(), StillError> {
    bits.align()?;
    if bits.pos.checked_add(4).ok_or(StillError::Size)? > bits.input.len() {
        return Err(StillError::Truncated);
    }
    let len = u16::from_le_bytes([bits.input[bits.pos], bits.input[bits.pos + 1]]);
    let nlen = u16::from_le_bytes([bits.input[bits.pos + 2], bits.input[bits.pos + 3]]);
    if nlen != !len {
        return Err(StillError::Inflate);
    }
    bits.pos += 4;
    let len = usize::from(len);
    let end = bits.pos.checked_add(len).ok_or(StillError::Size)?;
    if end > bits.input.len() {
        return Err(StillError::Truncated);
    }
    if out.len().checked_add(len).ok_or(StillError::Size)? > cap {
        return Err(StillError::Inflate);
    }
    out.extend_from_slice(&bits.input[bits.pos..end]);
    bits.pos = end;
    Ok(())
}

fn dynamic_block(bits: &mut Bits<'_>, out: &mut Vec<u8>, cap: usize) -> Result<(), StillError> {
    let nlit = bits.bits(5)? as usize + 257;
    let ndist = bits.bits(5)? as usize + 1;
    let ncode = bits.bits(4)? as usize + 4;
    if nlit > 286 || ndist > 32 || ncode > 19 {
        return Err(StillError::Inflate);
    }
    let mut code_len = [0u8; 19];
    for slot in CODE_ORDER.iter().take(ncode) {
        code_len[*slot] = bits.bits(3)? as u8;
    }
    let code_huff = construct(&code_len)?;
    let mut lens = vec![0u8; nlit + ndist];
    let mut index = 0;
    while index < lens.len() {
        match decode(bits, &code_huff)? {
            symbol @ 0..=15 => {
                lens[index] = symbol as u8;
                index += 1;
            }
            16 => {
                if index == 0 {
                    return Err(StillError::Inflate);
                }
                let repeat = bits.bits(2)? as usize + 3;
                let value = lens[index - 1];
                index = fill_value(&mut lens, index, repeat, value)?;
            }
            17 => {
                let repeat = bits.bits(3)? as usize + 3;
                index = fill_value(&mut lens, index, repeat, 0)?;
            }
            18 => {
                let repeat = bits.bits(7)? as usize + 11;
                index = fill_value(&mut lens, index, repeat, 0)?;
            }
            _ => return Err(StillError::Inflate),
        }
    }
    if lens.get(256).copied().unwrap_or(0) == 0 {
        return Err(StillError::Inflate);
    }
    let lit = construct(&lens[..nlit])?;
    let dist = construct(&lens[nlit..])?;
    coded_block(bits, &lit, &dist, out, cap)
}

fn fill_value(
    lens: &mut [u8],
    mut index: usize,
    repeat: usize,
    value: u8,
) -> Result<usize, StillError> {
    for _ in 0..repeat {
        let Some(slot) = lens.get_mut(index) else {
            return Err(StillError::Inflate);
        };
        *slot = value;
        index += 1;
    }
    Ok(index)
}

fn coded_block(
    bits: &mut Bits<'_>,
    lit: &Huff,
    dist: &Huff,
    out: &mut Vec<u8>,
    cap: usize,
) -> Result<(), StillError> {
    loop {
        match decode(bits, lit)? {
            symbol @ 0..=255 => {
                if out.len() >= cap {
                    return Err(StillError::Inflate);
                }
                out.push(symbol as u8);
            }
            256 => return Ok(()),
            symbol @ 257..=285 => {
                let which = (symbol - 257) as usize;
                let length = usize::from(LEN_BASE[which]) + bits.bits(LEN_EXTRA[which])? as usize;
                let dist_symbol = decode(bits, dist)? as usize;
                let Some(base) = DIST_BASE.get(dist_symbol) else {
                    return Err(StillError::Inflate);
                };
                let extra = DIST_EXTRA[dist_symbol];
                let distance = usize::from(*base) + bits.bits(extra)? as usize;
                if distance == 0 || distance > out.len() || distance > 32768 {
                    return Err(StillError::Inflate);
                }
                let end = out.len().checked_add(length).ok_or(StillError::Size)?;
                if end > cap {
                    return Err(StillError::Inflate);
                }
                for _ in 0..length {
                    let byte = out[out.len() - distance];
                    out.push(byte);
                }
            }
            _ => return Err(StillError::Inflate),
        }
    }
}

fn fixed_lit() -> Result<Huff, StillError> {
    let mut lens = [0u8; 288];
    lens[..144].fill(8);
    lens[144..256].fill(9);
    lens[256..280].fill(7);
    lens[280..].fill(8);
    construct(&lens)
}

fn construct(lengths: &[u8]) -> Result<Huff, StillError> {
    if lengths.len() > 288 {
        return Err(StillError::Inflate);
    }
    let mut count = [0u16; 16];
    for &len in lengths {
        if usize::from(len) >= count.len() {
            return Err(StillError::Inflate);
        }
        count[usize::from(len)] = count[usize::from(len)]
            .checked_add(1)
            .ok_or(StillError::Inflate)?;
    }
    let mut left = 1i32;
    for &count_len in count.iter().skip(1) {
        left <<= 1;
        left -= i32::from(count_len);
        if left < 0 {
            return Err(StillError::Inflate);
        }
    }
    let mut offset = [0usize; 16];
    for len in 1..15 {
        offset[len + 1] = offset[len] + usize::from(count[len]);
    }
    let mut symbol = [0u16; 288];
    for (sym, &len) in lengths.iter().enumerate() {
        if len != 0 {
            let at = offset[usize::from(len)];
            let Some(slot) = symbol.get_mut(at) else {
                return Err(StillError::Inflate);
            };
            *slot = sym as u16;
            offset[usize::from(len)] = at + 1;
        }
    }
    Ok(Huff { count, symbol })
}

fn decode(bits: &mut Bits<'_>, huff: &Huff) -> Result<u16, StillError> {
    let mut code = 0u32;
    let mut first = 0u32;
    let mut index = 0usize;
    for len in 1..=15 {
        code |= u32::from(bits.bit()?);
        let count = u32::from(huff.count[len]);
        if count > 0 && code >= first && code - first < count {
            let at = index + (code - first) as usize;
            return huff.symbol.get(at).copied().ok_or(StillError::Inflate);
        }
        index += count as usize;
        first = first.wrapping_add(count) << 1;
        code <<= 1;
    }
    Err(StillError::Inflate)
}

impl Bits<'_> {
    fn bit(&mut self) -> Result<u8, StillError> {
        if self.nbits == 0 {
            self.pull()?;
        }
        let bit = (self.buf & 1) as u8;
        self.buf >>= 1;
        self.nbits -= 1;
        Ok(bit)
    }

    fn bits(&mut self, n: u8) -> Result<u32, StillError> {
        let mut value = 0u32;
        for shift in 0..n {
            value |= u32::from(self.bit()?) << shift;
        }
        Ok(value)
    }

    fn pull(&mut self) -> Result<(), StillError> {
        let Some(byte) = self.input.get(self.pos) else {
            return Err(StillError::Truncated);
        };
        self.pos += 1;
        self.buf |= u32::from(*byte) << self.nbits;
        self.nbits += 8;
        Ok(())
    }

    fn align(&mut self) -> Result<(), StillError> {
        let extra = self.nbits % 8;
        if extra != 0 {
            self.buf >>= extra;
            self.nbits -= extra;
        }
        while self.nbits >= 8 {
            if self.pos == 0 {
                return Err(StillError::Inflate);
            }
            self.nbits -= 8;
            self.pos -= 1;
            self.buf >>= 8;
        }
        self.buf = 0;
        self.nbits = 0;
        Ok(())
    }
}

pub(crate) fn inflate_raw_capped(input: &[u8], cap: usize) -> Result<Vec<u8>, StillError> {
    const MAX_OUTPUT: usize = 8_985_728;
    if cap > MAX_OUTPUT {
        return Err(StillError::Size);
    }
    if input.is_empty() {
        return Err(StillError::Truncated);
    }
    let mut bits = Bits {
        input,
        pos: 0,
        buf: 0,
        nbits: 0,
    };
    let out = inflate_blocks(&mut bits, cap)?;
    bits.align()?;
    if bits.pos != input.len() {
        return Err(StillError::Trailing);
    }
    Ok(out)
}

fn inflate_blocks(bits: &mut Bits<'_>, cap: usize) -> Result<Vec<u8>, StillError> {
    let fixed_lit = fixed_lit()?;
    let fixed_dist = construct(&[5u8; 32])?;
    let mut out = Vec::with_capacity(cap);
    let mut blocks = 0usize;
    loop {
        blocks += 1;
        if blocks > bits.input.len().saturating_add(1) {
            return Err(StillError::Inflate);
        }
        let last = bits.bit()?;
        match bits.bits(2)? {
            0 => stored_block(bits, &mut out, cap)?,
            1 => coded_block(bits, &fixed_lit, &fixed_dist, &mut out, cap)?,
            2 => dynamic_block(bits, &mut out, cap)?,
            _ => return Err(StillError::Inflate),
        }
        if last == 1 {
            break;
        }
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::inflate_capped;

    #[test]
    fn a_fixed_huffman_hello_is_not_the_stored_test_encoder() {
        let zlib: &[u8] = &[
            0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00, 0x06, 0x2c, 0x02, 0x15,
        ];
        assert_eq!(inflate_capped(zlib, 64).as_deref(), Ok(b"hello".as_slice()));
    }

    #[test]
    fn output_past_the_cap_is_rejected() {
        let zlib: &[u8] = &[
            0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00, 0x06, 0x2c, 0x02, 0x15,
        ];
        assert!(inflate_capped(zlib, 4).is_err());
    }
}

#[cfg(test)]
mod raw_inflate {
    use super::{inflate_capped, inflate_raw_capped};
    use crate::still::StillError;

    const PATTERN_LEN: usize = 70_000;
    const STORED: &[u8] = include_bytes!("raw_stored.deflate");
    const FIXED: &[u8] = include_bytes!("raw_fixed.deflate");
    const DYNAMIC: &[u8] = include_bytes!("raw_dynamic.deflate");

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| ((i * 17 + 3) & 255) as u8).collect()
    }

    fn assert_same(got: &[u8], expected: &[u8]) {
        assert_eq!(got.len(), expected.len());
        if let Some(index) = got.iter().zip(expected).position(|(a, b)| a != b) {
            assert_eq!(got[index], expected[index], "index {index}");
        }
    }

    fn stored_final(data: &[u8]) -> Vec<u8> {
        assert!(data.len() <= 65535);
        let mut out = vec![1];
        let len = data.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    fn stored_prefix(data: &[u8]) -> Vec<u8> {
        assert!(data.len() <= 65535);
        let mut out = vec![0];
        let len = data.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    struct Pack {
        out: Vec<u8>,
        acc: u8,
        n: u8,
    }

    impl Pack {
        fn push(&mut self, value: u32, n: u8) {
            for shift in 0..n {
                self.acc |= (((value >> shift) & 1) as u8) << self.n;
                self.n += 1;
                if self.n == 8 {
                    self.out.push(self.acc);
                    self.acc = 0;
                    self.n = 0;
                }
            }
        }

        fn push_code(&mut self, value: u32, n: u8) {
            for shift in (0..n).rev() {
                self.push((value >> shift) & 1, 1);
            }
        }

        fn finish(mut self) -> Vec<u8> {
            if self.n > 0 {
                self.out.push(self.acc);
            }
            self.out
        }
    }

    fn fixed_backref(distance: usize, length: usize) -> Vec<u8> {
        assert_eq!(length, 3);
        let mut pack = Pack {
            out: Vec::new(),
            acc: 0,
            n: 0,
        };
        pack.push(0b011, 3);
        pack.push_code(1, 7);
        if distance == 32768 {
            pack.push_code(29, 5);
            pack.push(8191, 13);
        } else if distance == 200 {
            pack.push_code(15, 5);
            pack.push(7, 6);
        } else {
            panic!("unsupported distance");
        }
        pack.push_code(0, 7);
        pack.finish()
    }

    #[test]
    fn stored_output_over_64k_matches_every_byte() {
        assert!(STORED.len() > 70_000);
        assert_eq!(STORED[0] & 7, 0);
        let got = inflate_raw_capped(STORED, PATTERN_LEN).unwrap();
        assert_same(&got, &pattern(PATTERN_LEN));
    }

    #[test]
    fn fixed_output_over_64k_matches_every_byte() {
        assert!(FIXED.len() < 20_000);
        assert_eq!(FIXED[0] & 7, 3);
        let got = inflate_raw_capped(FIXED, PATTERN_LEN).unwrap();
        assert_same(&got, &pattern(PATTERN_LEN));
    }

    #[test]
    fn dynamic_output_over_64k_matches_every_byte() {
        assert!(DYNAMIC.len() < 20_000);
        assert_eq!(DYNAMIC[0] & 7, 5);
        let got = inflate_raw_capped(DYNAMIC, PATTERN_LEN).unwrap();
        assert_same(&got, &pattern(PATTERN_LEN));
    }

    #[test]
    fn a_backref_at_the_32k_window_is_copied() {
        let mut stream = stored_prefix(&[0xa5; 32768]);
        stream.extend_from_slice(&fixed_backref(32768, 3));
        let got = inflate_raw_capped(&stream, 32768 + 3).unwrap();
        assert_same(&got, &vec![0xa5; 32768 + 3]);
    }

    #[test]
    fn a_distance_past_the_output_is_rejected() {
        let mut stream = stored_prefix(&[0x11; 64]);
        stream.extend_from_slice(&fixed_backref(200, 3));
        assert_eq!(inflate_raw_capped(&stream, 128), Err(StillError::Inflate));
    }

    #[test]
    fn a_distance_of_32768_without_that_history_is_rejected() {
        let mut stream = stored_prefix(&[0xa5; 32767]);
        stream.extend_from_slice(&fixed_backref(32768, 3));
        assert_eq!(
            inflate_raw_capped(&stream, 32767 + 3),
            Err(StillError::Inflate)
        );
    }

    #[test]
    fn raw_output_past_the_cap_is_rejected() {
        let stream = stored_final(&[7; 50]);
        assert_eq!(inflate_raw_capped(&stream, 49), Err(StillError::Inflate));
    }

    #[test]
    fn a_raw_cap_above_the_archive_maximum_is_rejected() {
        let stream = stored_final(&[7; 4]);
        assert_eq!(
            inflate_raw_capped(&stream, 8_985_729),
            Err(StillError::Size)
        );
    }

    #[test]
    fn a_truncated_raw_stream_is_rejected() {
        let mut stream = stored_final(&[1, 2, 3, 4]);
        stream.pop();
        assert_eq!(inflate_raw_capped(&stream, 4), Err(StillError::Truncated));
    }

    #[test]
    fn a_trailing_raw_byte_is_rejected() {
        let mut stream = stored_final(&[1, 2, 3, 4]);
        stream.push(0);
        assert_eq!(inflate_raw_capped(&stream, 4), Err(StillError::Trailing));
    }

    #[test]
    fn a_stored_length_complement_mismatch_is_rejected() {
        let mut stream = stored_final(&[9, 8, 7]);
        stream[3] ^= 0xff;
        assert_eq!(inflate_raw_capped(&stream, 3), Err(StillError::Inflate));
    }

    #[test]
    fn an_empty_raw_stream_is_rejected() {
        assert_eq!(inflate_raw_capped(&[], 1), Err(StillError::Truncated));
    }

    #[test]
    fn the_zlib_wrapper_rejects_a_raw_stream_that_raw_inflate_accepts() {
        assert_eq!(
            inflate_capped(STORED, PATTERN_LEN).unwrap_err(),
            StillError::Inflate
        );
        let got = inflate_raw_capped(STORED, PATTERN_LEN).unwrap();
        assert_same(&got, &pattern(PATTERN_LEN));
    }
}
