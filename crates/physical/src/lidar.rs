//! LDS-03 style packets. Bad packets are counted and are not turned into ranges.

use crate::profile::{RANGE_DIVISOR, wrap_full_deg};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawSample {
    pub at: usize,
    pub raw_deg: f64,
    pub range_m: f64,
    pub intensity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectKind {
    BadChecksum,
    CountOutOfRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketReject {
    pub at: usize,
    pub kind: RejectKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LidarParse {
    pub samples: Vec<RawSample>,
    pub rejects: Vec<PacketReject>,
    pub packets_ok: usize,
    pub unparsed_tail: usize,
    pub resync_bytes: usize,
}

pub fn parse_lidar(bytes: &[u8]) -> LidarParse {
    let mut samples = Vec::new();
    let mut rejects = Vec::new();
    let mut packets_ok = 0;
    let mut resync_bytes = 0;
    let mut index = 0;
    while index + 10 < bytes.len() {
        if bytes[index] != 0xaa || bytes[index + 1] != 0x55 {
            index += 1;
            resync_bytes += 1;
            continue;
        }
        let count = usize::from(bytes[index + 3]);
        if !(1..=40).contains(&count) {
            rejects.push(PacketReject {
                at: index,
                kind: RejectKind::CountOutOfRange,
            });
            index += 1;
            continue;
        }
        let need = 10 + 3 * count;
        if index + need > bytes.len() {
            break;
        }
        let packet = &bytes[index..index + need];
        if !checksum_ok(packet) {
            rejects.push(PacketReject {
                at: index,
                kind: RejectKind::BadChecksum,
            });
            index += 1;
            continue;
        }
        let first = f64::from(word(packet, 4) >> 1) / 64.0;
        let last = f64::from(word(packet, 6) >> 1) / 64.0;
        let span = if count > 1 {
            wrap_full_deg(last - first)
        } else {
            0.0
        };
        let step = if count > 1 {
            span / (count - 1) as f64
        } else {
            0.0
        };
        for sample in 0..count {
            let at = 10 + 3 * sample;
            let raw = word(packet, at + 1);
            samples.push(RawSample {
                at: index + at,
                raw_deg: wrap_full_deg(first + step * sample as f64),
                range_m: f64::from(raw) / RANGE_DIVISOR,
                intensity: packet[at],
            });
        }
        packets_ok += 1;
        index += need;
    }
    LidarParse {
        samples,
        rejects,
        packets_ok,
        unparsed_tail: bytes.len() - index,
        resync_bytes,
    }
}

pub fn split_revolutions(samples: &[RawSample]) -> Vec<&[RawSample]> {
    let mut revolutions = Vec::new();
    let mut start = 0;
    for index in 1..samples.len() {
        if samples[index].raw_deg < samples[index - 1].raw_deg - 180.0 {
            revolutions.push(&samples[start..index]);
            start = index;
        }
    }
    if start < samples.len() {
        revolutions.push(&samples[start..]);
    }
    revolutions
}

pub fn encode_packet(ct: u8, samples: &[(f64, u16, u8)]) -> Result<Vec<u8>, &'static str> {
    let count = samples.len();
    if !(1..=40).contains(&count) {
        return Err("count");
    }
    let mut packet = vec![0_u8; 10 + 3 * count];
    packet[0] = 0xaa;
    packet[1] = 0x55;
    packet[2] = ct;
    packet[3] = u8::try_from(count).map_err(|_| "count")?;
    let first = angle_word(samples[0].0)?;
    let last = angle_word(samples[count - 1].0)?;
    packet[4] = first as u8;
    packet[5] = (first >> 8) as u8;
    packet[6] = last as u8;
    packet[7] = (last >> 8) as u8;
    for (index, (_, distance, intensity)) in samples.iter().enumerate() {
        let at = 10 + 3 * index;
        packet[at] = *intensity;
        packet[at + 1] = *distance as u8;
        packet[at + 2] = (*distance >> 8) as u8;
    }
    let check = expected_checksum(&packet);
    packet[8] = check as u8;
    packet[9] = (check >> 8) as u8;
    Ok(packet)
}

fn angle_word(degrees: f64) -> Result<u16, &'static str> {
    if !degrees.is_finite() || !(0.0..360.0).contains(&degrees) {
        return Err("angle");
    }
    let steps = (degrees * 64.0).round();
    if !(0.0..65_536.0).contains(&steps) {
        return Err("angle");
    }
    let shifted = (steps as u32) << 1;
    u16::try_from(shifted).map_err(|_| "angle")
}

fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from(bytes[at]) | (u16::from(bytes[at + 1]) << 8)
}

fn expected_checksum(packet: &[u8]) -> u16 {
    let count = packet[3];
    let mut check = 0x55aa_u16
        ^ (u16::from(packet[2]) | (u16::from(count) << 8))
        ^ word(packet, 4)
        ^ word(packet, 6);
    for sample in 0..usize::from(count) {
        let at = 10 + 3 * sample;
        check ^= u16::from(packet[at]) ^ word(packet, at + 1);
    }
    check
}

fn checksum_ok(packet: &[u8]) -> bool {
    expected_checksum(packet) == word(packet, 8)
}

#[cfg(test)]
mod tests {
    use super::{RejectKind, encode_packet, parse_lidar};

    #[test]
    fn a_hand_built_packet_keeps_its_range_and_a_bad_checksum_does_not() {
        let packet = [
            0xaa, 0x55, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x5b, 0x00, 0xa0, 0x0f,
        ];
        let parsed = parse_lidar(&packet);
        assert_eq!(parsed.packets_ok, 1);
        assert_eq!(parsed.samples.len(), 1);
        assert_eq!(parsed.samples[0].raw_deg, 0.0);
        assert!((parsed.samples[0].range_m - 1.0).abs() < 1e-12);
        assert!(parsed.rejects.is_empty());
        assert_eq!(encode_packet(0, &[(0.0, 4000, 0)]).unwrap(), packet);

        let mut broken = packet;
        broken[12] ^= 0xff;
        let bad = parse_lidar(&broken);
        assert!(bad.samples.is_empty());
        assert_eq!(bad.rejects.len(), 1);
        assert_eq!(bad.rejects[0].kind, RejectKind::BadChecksum);

        let mut both = broken.to_vec();
        both.extend_from_slice(&packet);
        let recovered = parse_lidar(&both);
        assert_eq!(recovered.samples.len(), 1);
        assert!((recovered.samples[0].range_m - 1.0).abs() < 1e-12);
    }

    #[test]
    fn zeros_are_kept_and_the_span_uses_a_positive_modulo() {
        let packet = [
            0xaa, 0x55, 0x00, 0x02, 0x00, 0x00, 0x00, 0x05, 0xaa, 0x52, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let parsed = parse_lidar(&packet);
        assert_eq!(parsed.samples.len(), 2);
        assert_eq!(parsed.samples[0].range_m, 0.0);
        assert_eq!(parsed.samples[1].range_m, 0.0);
        assert!((parsed.samples[1].raw_deg - 10.0).abs() < 1e-9);

        let wrapped = encode_packet(0x99, &[(350.0, 0, 1), (10.0, 8, 2)]).unwrap();
        let parsed = parse_lidar(&wrapped);
        assert_eq!(parsed.packets_ok, 1);
        assert!((parsed.samples[0].raw_deg - 350.0).abs() < 1e-6);
        assert!((parsed.samples[1].raw_deg - 10.0).abs() < 1e-6);
        assert_eq!(parsed.samples[1].intensity, 2);
    }

    #[test]
    fn a_short_tail_is_unparsed_and_a_wild_count_is_a_reject() {
        let mut bytes = encode_packet(0, &[(0.0, 4000, 0)]).unwrap();
        bytes.extend_from_slice(&[0xaa, 0x55, 0x00, 0x01]);
        let parsed = parse_lidar(&bytes);
        assert_eq!(parsed.samples.len(), 1);
        assert_eq!(parsed.unparsed_tail, 4);
        let wild = parse_lidar(&[0xaa, 0x55, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0]);
        assert!(wild.samples.is_empty());
        assert_eq!(wild.rejects[0].kind, RejectKind::CountOutOfRange);
    }
}
