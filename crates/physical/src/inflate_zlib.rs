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
    let fixed_lit = fixed_lit()?;
    let fixed_dist = construct(&[5u8; 32])?;
    let mut out = Vec::new();
    let mut blocks = 0usize;
    loop {
        blocks += 1;
        if blocks > input.len().saturating_add(1) {
            return Err(StillError::Inflate);
        }
        let last = bits.bit()?;
        match bits.bits(2)? {
            0 => stored_block(&mut bits, &mut out, cap)?,
            1 => coded_block(&mut bits, &fixed_lit, &fixed_dist, &mut out, cap)?,
            2 => dynamic_block(&mut bits, &mut out, cap)?,
            _ => return Err(StillError::Inflate),
        }
        if last == 1 {
            break;
        }
    }
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
