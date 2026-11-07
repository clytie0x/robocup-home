//! Commanded distance, recorded odometry, and a gated scan-forward estimate.

use crate::json::{self, Value};
use crate::profile::wrap_signed_deg;

#[derive(Clone, Copy, Debug)]
pub struct ForwardGate {
    pub name: &'static str,
    pub half_angle_deg: f64,
    pub max_abs_range_delta_m: f64,
}

pub const GATES: [ForwardGate; 3] = [
    ForwardGate {
        name: "narrow-20deg-0.40m",
        half_angle_deg: 20.0,
        max_abs_range_delta_m: 0.40,
    },
    ForwardGate {
        name: "mid-30deg-0.80m",
        half_angle_deg: 30.0,
        max_abs_range_delta_m: 0.80,
    },
    ForwardGate {
        name: "wide-45deg-1.50m",
        half_angle_deg: 45.0,
        max_abs_range_delta_m: 1.50,
    },
];

#[derive(Clone, Debug, PartialEq)]
pub struct ForwardEstimate {
    pub gate: &'static str,
    pub median_m: f64,
    pub mad_m: f64,
    pub pairs: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpeedRun {
    pub request_m_s: f64,
    pub duration_s: f64,
    pub odom0: [f64; 3],
    pub odom1: [f64; 3],
    pub before: Vec<(f64, f64)>,
    pub after: Vec<(f64, f64)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OdomDelta {
    pub dx: f64,
    pub dy: f64,
    pub dyaw: f64,
    pub chord: f64,
    pub forward_at_start_yaw: f64,
    pub left_at_start_yaw: f64,
}

pub fn parse_speed_runs(text: &str) -> Result<Vec<SpeedRun>, String> {
    let value = json::parse(text)?;
    let items = value
        .as_array()
        .ok_or_else(|| "speed file is not an array".to_string())?;
    let mut runs = Vec::new();
    for (index, item) in items.iter().enumerate() {
        runs.push(one_run(item).map_err(|err| format!("run {index}: {err}"))?);
    }
    Ok(runs)
}

fn one_run(item: &Value) -> Result<SpeedRun, String> {
    let request_m_s = number(item, "req")?;
    let duration_s = number(item, "secs")?;
    if duration_s < 0.0 {
        return Err("negative duration".to_string());
    }
    Ok(SpeedRun {
        request_m_s,
        duration_s,
        odom0: triple(item, "odom0")?,
        odom1: triple(item, "odom1")?,
        before: pairs(item, "A")?,
        after: pairs(item, "B")?,
    })
}

fn number(item: &Value, key: &str) -> Result<f64, String> {
    item.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing {key}"))
}

fn triple(item: &Value, key: &str) -> Result<[f64; 3], String> {
    let values = item
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing {key}"))?;
    if values.len() != 3 {
        return Err(format!("{key} is not a triple"));
    }
    let mut out = [0.0; 3];
    for (index, value) in values.iter().enumerate() {
        out[index] = value
            .as_f64()
            .ok_or_else(|| format!("{key} has a non-finite value"))?;
    }
    Ok(out)
}

fn pairs(item: &Value, key: &str) -> Result<Vec<(f64, f64)>, String> {
    let values = item
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing {key}"))?;
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let pair = value.as_array().ok_or_else(|| format!("{key} pair"))?;
        if pair.len() != 2 {
            return Err(format!("{key} pair"));
        }
        let angle = pair[0].as_f64().ok_or_else(|| format!("{key} angle"))?;
        let range = pair[1].as_f64().ok_or_else(|| format!("{key} range"))?;
        out.push((angle, range));
    }
    Ok(out)
}

pub fn odom_delta(start: [f64; 3], end: [f64; 3]) -> Option<OdomDelta> {
    if start.iter().any(|value| !value.is_finite()) || end.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let dyaw = wrap_rad(end[2] - start[2]);
    let yaw = start[2];
    Some(OdomDelta {
        dx,
        dy,
        dyaw,
        chord: dx.hypot(dy),
        forward_at_start_yaw: dx * yaw.cos() + dy * yaw.sin(),
        left_at_start_yaw: -dx * yaw.sin() + dy * yaw.cos(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Projection {
    Ray,
    Plane,
}

impl Projection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Projection::Ray => "ray-projection",
            Projection::Plane => "frontal-plane",
        }
    }
}

pub fn scan_forward(
    before: &[(f64, f64)],
    after: &[(f64, f64)],
    gate: ForwardGate,
    projection: Projection,
) -> Option<ForwardEstimate> {
    let left = bin(before);
    let right = bin(after);
    let mut deltas = Vec::new();
    let half = gate.half_angle_deg.round() as i32;
    for angle in -half..=half {
        let (Some(ra), Some(rb)) = (left.get(&angle).copied(), right.get(&angle).copied()) else {
            continue;
        };
        if (rb - ra).abs() > gate.max_abs_range_delta_m {
            continue;
        }
        let cosine = f64::from(angle).to_radians().cos();
        if cosine < 0.25 {
            continue;
        }
        let delta = match projection {
            Projection::Ray => (ra - rb) / cosine,
            Projection::Plane => (ra - rb) * cosine,
        };
        if delta.is_finite() {
            deltas.push(delta);
        }
    }
    if deltas.len() < 8 {
        return None;
    }
    let median_m = median(&mut deltas.clone())?;
    let mut abs = deltas
        .iter()
        .map(|value| (value - median_m).abs())
        .collect::<Vec<_>>();
    let mad_m = median(&mut abs)?;
    Some(ForwardEstimate {
        gate: gate.name,
        median_m,
        mad_m,
        pairs: deltas.len(),
    })
}

pub fn shift_angles(scan: &[(f64, f64)], delta_deg: f64) -> Vec<(f64, f64)> {
    scan.iter()
        .map(|(angle, range)| (wrap_signed_deg(angle + delta_deg), *range))
        .collect()
}

fn bin(scan: &[(f64, f64)]) -> std::collections::BTreeMap<i32, f64> {
    let mut grouped: std::collections::BTreeMap<i32, Vec<f64>> = std::collections::BTreeMap::new();
    for (angle, range) in scan {
        if !angle.is_finite() || !range.is_finite() || !(0.05..20.0).contains(range) {
            continue;
        }
        let rounded = angle.round();
        if !rounded.is_finite() {
            continue;
        }
        let index = rounded as i32;
        if !(-180..=180).contains(&index) {
            continue;
        }
        grouped.entry(index).or_default().push(*range);
    }
    grouped
        .into_iter()
        .filter_map(|(angle, mut ranges)| median(&mut ranges).map(|value| (angle, value)))
        .collect()
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}

fn wrap_rad(mut radians: f64) -> f64 {
    while radians > std::f64::consts::PI {
        radians -= 2.0 * std::f64::consts::PI;
    }
    while radians <= -std::f64::consts::PI {
        radians += 2.0 * std::f64::consts::PI;
    }
    radians
}

#[cfg(test)]
mod tests {
    use super::{GATES, Projection, odom_delta, parse_speed_runs, scan_forward};

    fn wall(distance: f64) -> Vec<(f64, f64)> {
        let mut scan = Vec::new();
        for angle in -40..=40 {
            let radians = f64::from(angle).to_radians();
            scan.push((f64::from(angle), distance / radians.cos()));
        }
        scan.push((0.0, 0.0));
        scan.push((10.0, 50.0));
        scan
    }

    #[test]
    fn a_known_forward_shift_survives_zeros_and_an_outlier() {
        let before = wall(2.0);
        let mut after = wall(1.8);
        after.retain(|sample| sample.1 != 50.0);
        after.push((10.0, 50.0));
        let plane = scan_forward(&before, &after, GATES[1], Projection::Plane).unwrap();
        assert!((plane.median_m - 0.2).abs() < 1e-9, "{}", plane.median_m);
        assert!(plane.mad_m < 1e-9);
        let ray = scan_forward(&before, &after, GATES[1], Projection::Ray).unwrap();
        assert!((ray.median_m - 0.2).abs() > 1e-4);
        let mut circle_before = Vec::new();
        let mut circle_after = Vec::new();
        for angle in -30..=30 {
            let cosine = f64::from(angle).to_radians().cos();
            circle_before.push((f64::from(angle), 2.0));
            circle_after.push((f64::from(angle), 2.0 - 0.2 * cosine));
        }
        let ray_circle =
            scan_forward(&circle_before, &circle_after, GATES[1], Projection::Ray).unwrap();
        assert!((ray_circle.median_m - 0.2).abs() < 1e-9);
        let text =
            r#"[{"req":0.1,"secs":2,"odom0":[0,0,0],"odom1":[0.2,0,0],"A":[[0,2]],"B":[[0,1.8]]}]"#;
        let runs = parse_speed_runs(text).unwrap();
        assert_eq!(runs.len(), 1);
        assert!(scan_forward(&runs[0].before, &runs[0].after, GATES[1], Projection::Ray).is_none());
        let delta = odom_delta([0.0, 0.0, 0.0], [0.2, 0.0, 0.0]).unwrap();
        assert!((delta.chord - 0.2).abs() < 1e-12);
        assert!((delta.forward_at_start_yaw - 0.2).abs() < 1e-12);
        assert!(odom_delta([f64::NAN, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
        assert!(parse_speed_runs(r#"[{"req":1}]"#).is_err());
    }
}
