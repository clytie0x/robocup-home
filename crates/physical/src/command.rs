//! Reference wheel-command text. Nothing in this module is written to a port.

pub const LINEAR_SCALE_NOT_ADOPTED: f64 = 1.28;
pub const ANGULAR_SCALE_NOT_ADOPTED: f64 = 1.50;
pub const YAW_SCALE_NOT_ADOPTED: f64 = 1.16;
pub const DIST_SCALE_NOT_ADOPTED: f64 = 1.0 / 0.88;
pub const MAX_LINEAR_M_S: f64 = 0.15;
pub const MAX_ANGULAR_RAD_S: f64 = 0.26;

pub const SCALE_SOURCE: &str = "wheels.py LINEAR_SCALE, ANGULAR_SCALE, and YAW_SCALE, and move.py DIST_SCALE. They are reference-code knobs. This run does not adopt them as measured gains.";

pub fn format_command(linear_m_s: f64, angular_rad_s: f64) -> String {
    if !linear_m_s.is_finite() || !angular_rad_s.is_finite() {
        return "malformed".to_string();
    }
    let linear = clamp(linear_m_s, MAX_LINEAR_M_S);
    let angular = clamp(angular_rad_s, MAX_ANGULAR_RAD_S);
    if linear.abs() < 0.001 && angular.abs() < 0.001 {
        return "S#".to_string();
    }
    let letter = if linear >= 0.0 { 'F' } else { 'B' };
    let sign = if angular >= 0.0 { '+' } else { '-' };
    format!("{letter}{}|{sign}{}#", digits(linear), digits(angular))
}

/// The text `wheels.py` would send after scaling. The scale is not a measurement.
pub fn wire_preview(request_m_s: f64, angular_rad_s: f64) -> String {
    format_command(
        request_m_s * LINEAR_SCALE_NOT_ADOPTED,
        angular_rad_s * ANGULAR_SCALE_NOT_ADOPTED,
    )
}

fn clamp(value: f64, limit: f64) -> f64 {
    value.clamp(-limit, limit)
}

fn digits(value: f64) -> String {
    let magnitude = value.abs();
    let ones = (magnitude as i64) % 10;
    let tenths = (magnitude * 10.0) as i64 % 10;
    let hundredths = (magnitude * 100.0) as i64 % 10;
    format!("{ones}{tenths}{hundredths}")
}

#[cfg(test)]
mod tests {
    use super::{format_command, wire_preview};

    #[test]
    fn the_reference_encoding_is_not_rounded_and_is_not_sent() {
        assert_eq!(format_command(0.0, 0.0), "S#");
        assert_eq!(format_command(0.0, 0.0009), "S#");
        assert_eq!(format_command(0.1, -0.04), "F010|-004#");
        assert_eq!(format_command(0.149, 0.0), "F014|+000#");
        assert_eq!(format_command(0.199, 0.0), "F015|+000#");
        assert_eq!(format_command(0.0, 0.009), "F000|+000#");
        assert_eq!(wire_preview(0.117, 0.0), "F014|+000#");
        assert_eq!(format_command(1.0, 0.0), "F015|+000#");
    }
}
