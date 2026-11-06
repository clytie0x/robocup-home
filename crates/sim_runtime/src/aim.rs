use domain::Point;

/// Pointing measurement. It has an origin and a direction, and no bag id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AimSample {
    origin: Point,
    dx: i32,
    dy: i32,
}

impl AimSample {
    pub const fn origin(self) -> Point {
        self.origin
    }

    pub const fn dx(self) -> i32 {
        self.dx
    }

    pub const fn dy(self) -> i32 {
        self.dy
    }

    pub(crate) const fn new(origin: Point, dx: i32, dy: i32) -> Self {
        Self { origin, dx, dy }
    }
}

/// Why a pointing sample was not produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AimError {
    UnknownBag,
    Overflow,
}

impl std::fmt::Display for AimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AimError::UnknownBag => "unknown bag",
            AimError::Overflow => "direction overflow",
        })
    }
}

impl std::error::Error for AimError {}

/// Integer millimeter direction from `from` to `to`.
///
/// A difference that does not fit in `i32` is [`AimError::Overflow`]. Neither
/// point is changed. The result is not a bag id.
pub fn direction_mm(from: Point, to: Point) -> Result<(i32, i32), AimError> {
    let dx = i64::from(to.x().get()) - i64::from(from.x().get());
    let dy = i64::from(to.y().get()) - i64::from(from.y().get());
    let dx = i32::try_from(dx).map_err(|_| AimError::Overflow)?;
    let dy = i32::try_from(dy).map_err(|_| AimError::Overflow)?;
    Ok((dx, dy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Mm;

    fn point(x: i32, y: i32) -> Point {
        Point::new(Mm::new(x), Mm::new(y))
    }

    #[test]
    fn deltas_keep_sign_and_reject_an_axis_that_does_not_fit() {
        assert_eq!(
            direction_mm(point(0, 1000), point(1531, 1000)),
            Ok((1531, 0))
        );
        assert_eq!(
            direction_mm(point(0, 1000), point(-453, 1000)),
            Ok((-453, 0))
        );
        assert_eq!(direction_mm(point(4, 5), point(4, 5)), Ok((0, 0)));
        let low = point(i32::MIN, 4);
        let high = point(i32::MAX, 9);
        assert_eq!(direction_mm(low, high), Err(AimError::Overflow));
        assert_eq!(low.x().get(), i32::MIN);
        assert_eq!(low.y().get(), 4);
        assert_eq!(high.x().get(), i32::MAX);
        assert_eq!(high.y().get(), 9);
        assert_eq!(
            direction_mm(point(1, i32::MAX), point(2, i32::MIN)),
            Err(AimError::Overflow)
        );
        assert_eq!(
            direction_mm(point(i32::MAX, i32::MIN), point(i32::MIN, i32::MAX)),
            Err(AimError::Overflow)
        );
    }
}
