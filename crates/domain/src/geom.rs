/// Geometry input that cannot be stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeomError {
    Overflow,
    NonFinite,
}

impl std::fmt::Display for GeomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GeomError::Overflow => "coordinate overflow",
            GeomError::NonFinite => "non-finite coordinate",
        })
    }
}

impl std::error::Error for GeomError {}

/// Signed millimeters. Every `i32` is in range. Float inputs use [`Mm::try_from_meters`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mm(i32);

impl Mm {
    pub const fn new(mm: i32) -> Self {
        Self(mm)
    }

    pub const fn get(self) -> i32 {
        self.0
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, GeomError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(GeomError::Overflow)
    }

    pub fn try_from_meters(meters: f64) -> Result<Self, GeomError> {
        if !meters.is_finite() {
            return Err(GeomError::NonFinite);
        }
        let mm = meters * 1000.0;
        if !mm.is_finite() {
            return Err(GeomError::Overflow);
        }
        let rounded = mm.round();
        if rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
            return Err(GeomError::Overflow);
        }
        Ok(Self(rounded as i32))
    }
}

/// Planar point in millimeters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    x: Mm,
    y: Mm,
}

impl Point {
    pub const fn new(x: Mm, y: Mm) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> Mm {
        self.x
    }

    pub const fn y(self) -> Mm {
        self.y
    }

    /// Returns the shifted point. On overflow the input is left unchanged.
    pub fn checked_offset(self, dx: Mm, dy: Mm) -> Result<Self, GeomError> {
        Ok(Self::new(self.x.checked_add(dx)?, self.y.checked_add(dy)?))
    }
}

/// Inclusive planar disk. A negative radius contains no point.
///
/// The square is computed in `i128`, so the ends of `i32` do not wrap into the disk.
pub fn within_reach(from: Point, to: Point, radius_mm: i32) -> bool {
    if radius_mm < 0 {
        return false;
    }
    let dx = i128::from(to.x().get()) - i128::from(from.x().get());
    let dy = i128::from(to.y().get()) - i128::from(from.y().get());
    let radius = i128::from(radius_mm);
    dx * dx + dy * dy <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::{GeomError, Mm, Point, within_reach};

    #[test]
    fn meters_reject_non_finite_and_out_of_range_values() {
        assert_eq!(Mm::try_from_meters(f64::NAN), Err(GeomError::NonFinite));
        assert_eq!(
            Mm::try_from_meters(f64::INFINITY),
            Err(GeomError::NonFinite)
        );
        assert_eq!(
            Mm::try_from_meters(f64::NEG_INFINITY),
            Err(GeomError::NonFinite)
        );
        assert_eq!(Mm::try_from_meters(1.0e20), Err(GeomError::Overflow));
        assert_eq!(Mm::try_from_meters(-1.0e20), Err(GeomError::Overflow));
    }

    #[test]
    fn meters_round_to_the_nearest_millimeter() {
        assert_eq!(Mm::try_from_meters(1.25).unwrap().get(), 1250);
        assert_eq!(Mm::try_from_meters(-0.0014).unwrap().get(), -1);
        assert_eq!(Mm::try_from_meters(0.0).unwrap().get(), 0);
    }

    #[test]
    fn offset_overflow_keeps_both_axes() {
        let point = Point::new(Mm::new(i32::MAX), Mm::new(4));
        assert_eq!(
            point.checked_offset(Mm::new(1), Mm::new(-3)),
            Err(GeomError::Overflow)
        );
        assert_eq!(point.x().get(), i32::MAX);
        assert_eq!(point.y().get(), 4);

        let point = Point::new(Mm::new(3), Mm::new(i32::MIN));
        assert_eq!(
            point.checked_offset(Mm::new(8), Mm::new(-1)),
            Err(GeomError::Overflow)
        );
        assert_eq!(point.x().get(), 3);
        assert_eq!(point.y().get(), i32::MIN);
    }

    #[test]
    fn reach_is_an_inclusive_disk_and_does_not_wrap() {
        let origin = Point::new(Mm::new(0), Mm::new(0));
        assert!(within_reach(origin, origin, 0));
        assert!(!within_reach(origin, Point::new(Mm::new(1), Mm::new(0)), 0));
        assert!(within_reach(
            origin,
            Point::new(Mm::new(2500), Mm::new(0)),
            2500
        ));
        assert!(!within_reach(
            origin,
            Point::new(Mm::new(2501), Mm::new(0)),
            2500
        ));
        assert!(within_reach(
            origin,
            Point::new(Mm::new(-2500), Mm::new(0)),
            2500
        ));
        assert!(within_reach(
            origin,
            Point::new(Mm::new(1500), Mm::new(2000)),
            2500
        ));
        assert!(!within_reach(
            origin,
            Point::new(Mm::new(1500), Mm::new(2001)),
            2500
        ));
        assert!(!within_reach(origin, origin, -1));
        assert!(!within_reach(
            Point::new(Mm::new(i32::MIN), Mm::new(0)),
            Point::new(Mm::new(i32::MAX), Mm::new(0)),
            2500
        ));
        assert!(!within_reach(
            Point::new(Mm::new(0), Mm::new(i32::MIN)),
            Point::new(Mm::new(0), Mm::new(i32::MAX)),
            i32::MAX
        ));
    }

    #[test]
    fn offset_writes_neither_axis_until_both_fit() {
        let point = Point::new(Mm::new(10), Mm::new(-2));
        let shifted = point.checked_offset(Mm::new(-4), Mm::new(5)).unwrap();
        assert_eq!(shifted.x().get(), 6);
        assert_eq!(shifted.y().get(), 3);
        assert_eq!(point.x().get(), 10);
        assert_eq!(point.y().get(), -2);
    }
}
