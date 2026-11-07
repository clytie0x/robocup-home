//! Posted views. A controller function cannot see scene truth.

use crate::kind::Kind;
use crate::lidar::RawSample;
use crate::profile::wrap_signed_deg;

#[derive(Clone, Debug, PartialEq)]
pub enum Freshness {
    Fresh,
    Stale { age_s: f64 },
    Missing,
    Malformed,
    Reversed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PostedScan {
    pub freshness: Freshness,
    pub samples: Vec<RawSample>,
    pub source_name: String,
}

impl PostedScan {
    pub fn fresh_ranges(&self) -> &[RawSample] {
        if self.freshness == Freshness::Fresh {
            &self.samples
        } else {
            &[]
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthClass {
    NoReturn,
    BelowGate,
    Valid,
}

pub fn classify_depth_mm(mm: u16, gate_mm: u16) -> DepthClass {
    if mm == 0 {
        DepthClass::NoReturn
    } else if mm < gate_mm {
        DepthClass::BelowGate
    } else {
        DepthClass::Valid
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepthView {
    pub freshness: Freshness,
    pub width: usize,
    pub height: usize,
    pub mm: Vec<u16>,
    pub gate_mm: u16,
    pub gate_kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonPost {
    NotInPostedView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actuation {
    NotSent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerNote {
    pub fresh_ranges: usize,
    pub retained_ranges: usize,
    pub fresh_valid_depth: usize,
    pub retained_depth: usize,
    pub person: PersonPost,
    pub actuation: Actuation,
}

pub fn controller_note(scan: &PostedScan, depth: Option<&DepthView>) -> ControllerNote {
    let fresh_valid_depth = depth
        .filter(|view| view.freshness == Freshness::Fresh)
        .map(|view| {
            view.mm
                .iter()
                .filter(|mm| classify_depth_mm(**mm, view.gate_mm) == DepthClass::Valid)
                .count()
        })
        .unwrap_or(0);
    ControllerNote {
        fresh_ranges: scan.fresh_ranges().len(),
        retained_ranges: scan.samples.len(),
        fresh_valid_depth,
        retained_depth: depth.map(|view| view.mm.len()).unwrap_or(0),
        person: PersonPost::NotInPostedView,
        actuation: Actuation::NotSent,
    }
}

pub fn stamp_order(times: &[f64]) -> Freshness {
    if times.is_empty() {
        return Freshness::Missing;
    }
    if times.iter().any(|time| !time.is_finite()) {
        return Freshness::Malformed;
    }
    if times.windows(2).any(|pair| pair[1] < pair[0]) {
        return Freshness::Reversed;
    }
    Freshness::Fresh
}

pub fn delivery_freshness(capture_s: f64, delivery_s: f64, max_age_s: f64) -> Freshness {
    if !capture_s.is_finite()
        || !delivery_s.is_finite()
        || !max_age_s.is_finite()
        || max_age_s < 0.0
    {
        return Freshness::Malformed;
    }
    if delivery_s < capture_s {
        return Freshness::Reversed;
    }
    let age_s = delivery_s - capture_s;
    if age_s > max_age_s {
        Freshness::Stale { age_s }
    } else {
        Freshness::Fresh
    }
}

/// Positive result is robot left (+y). `mirrored` follows the session image convention.
pub fn pixel_bearing_deg(col: f64, width: f64, hfov_deg: f64, mirrored: bool) -> Option<f64> {
    if !(width > 1.0 && col.is_finite() && hfov_deg.is_finite() && hfov_deg > 0.0) {
        return None;
    }
    let center = (width - 1.0) / 2.0;
    let half = (hfov_deg.to_radians() / 2.0).tan();
    let image_right = ((col - center) / center * half).atan().to_degrees();
    let bearing = if mirrored { image_right } else { -image_right };
    Some(wrap_signed_deg(bearing))
}

pub fn contains_ipv4(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() && ipv4_at(bytes, index) {
            return true;
        }
        index += 1;
    }
    false
}

pub fn sanitize_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if line.contains('@')
        || lower.contains("ts.net")
        || lower.contains("users\\")
        || lower.contains("/home/")
        || lower.contains("ssh ")
        || contains_ipv4(line)
    {
        "[omitted: address-like token]".to_string()
    } else {
        line.to_string()
    }
}

fn ipv4_at(bytes: &[u8], start: usize) -> bool {
    if start > 0 && bytes[start - 1].is_ascii_digit() {
        return false;
    }
    let mut index = start;
    for group in 0..4 {
        if index >= bytes.len() || !bytes[index].is_ascii_digit() {
            return false;
        }
        let mut value = 0_u32;
        let mut digits = 0;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            digits += 1;
            if digits > 3 {
                return false;
            }
            value = value * 10 + u32::from(bytes[index] - b'0');
            index += 1;
        }
        if value > 255 {
            return false;
        }
        if group < 3 {
            if index >= bytes.len() || bytes[index] != b'.' {
                return false;
            }
            index += 1;
        }
    }
    !(index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.'))
}

#[cfg(test)]
mod tests {
    use super::{
        Actuation, DepthClass, DepthView, Freshness, PostedScan, classify_depth_mm, contains_ipv4,
        controller_note, delivery_freshness, pixel_bearing_deg, sanitize_line, stamp_order,
    };
    use crate::kind::Kind;
    use crate::lidar::RawSample;
    use crate::profile::HFOV_DEG;

    fn sample(range_m: f64) -> RawSample {
        RawSample {
            at: 0,
            raw_deg: 0.0,
            range_m,
            intensity: 0,
        }
    }

    #[test]
    fn stale_and_reversed_views_keep_samples_out_of_the_fresh_window() {
        let scan = PostedScan {
            freshness: Freshness::Stale { age_s: 1.0 },
            samples: vec![sample(1.0), sample(0.0)],
            source_name: "synthetic".to_string(),
        };
        assert!(scan.fresh_ranges().is_empty());
        assert_eq!(scan.samples.len(), 2);
        let note = controller_note(&scan, None);
        assert_eq!(note.fresh_ranges, 0);
        assert_eq!(note.retained_ranges, 2);
        assert_eq!(note.actuation, Actuation::NotSent);
        assert_eq!(stamp_order(&[2.0, 1.0]), Freshness::Reversed);
        assert_eq!(
            delivery_freshness(1.0, 1.5, 0.2),
            Freshness::Stale { age_s: 0.5 }
        );
        assert_eq!(delivery_freshness(1.0, 0.5, 0.2), Freshness::Reversed);
        assert_eq!(delivery_freshness(1.0, 1.1, 0.2), Freshness::Fresh);
    }

    #[test]
    fn depth_classes_and_the_mirror_sign() {
        assert_eq!(classify_depth_mm(0, 500), DepthClass::NoReturn);
        assert_eq!(classify_depth_mm(499, 500), DepthClass::BelowGate);
        assert_eq!(classify_depth_mm(500, 500), DepthClass::Valid);
        let right = pixel_bearing_deg(319.0, 320.0, HFOV_DEG, true).unwrap();
        let left = pixel_bearing_deg(0.0, 320.0, HFOV_DEG, true).unwrap();
        assert!(right > 0.0);
        assert!(left < 0.0);
        assert!(
            pixel_bearing_deg(159.5, 320.0, HFOV_DEG, true)
                .unwrap()
                .abs()
                < 1e-6
        );
        let depth = DepthView {
            freshness: Freshness::Fresh,
            width: 2,
            height: 2,
            mm: vec![0, 400, 500, 1000],
            gate_mm: 500,
            gate_kind: Kind::Configured,
        };
        let fresh = PostedScan {
            freshness: Freshness::Fresh,
            samples: vec![],
            source_name: "s".to_string(),
        };
        assert_eq!(controller_note(&fresh, Some(&depth)).fresh_valid_depth, 2);
    }

    #[test]
    fn address_like_lines_are_omitted() {
        assert!(contains_ipv4("host 192.168.1.9 up"));
        assert!(contains_ipv4("100.64.1.2"));
        assert!(!contains_ipv4("t=0.01 yaw=2.070"));
        assert_eq!(sanitize_line("t=0.01 yaw=2.070"), "t=0.01 yaw=2.070");
        assert_eq!(
            sanitize_line("ssh 10.1.2.3"),
            "[omitted: address-like token]"
        );
    }
}
