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

/// True when the open disk of `radius_mm` around `center` meets the segment.
///
/// The radius is a caller assumption. A negative radius meets nothing. Equality
/// with the radius is outside the disk, so a tangent segment is not a hit.
/// The test is the whole segment: clear endpoints can still hide a hit in the middle.
pub fn segment_within(from: Point, to: Point, center: Point, radius_mm: i32) -> bool {
    if radius_mm <= 0 {
        return false;
    }
    let ax = i128::from(from.x().get());
    let ay = i128::from(from.y().get());
    let bx = i128::from(to.x().get());
    let by = i128::from(to.y().get());
    let cx = i128::from(center.x().get());
    let cy = i128::from(center.y().get());
    let abx = bx - ax;
    let aby = by - ay;
    let acx = cx - ax;
    let acy = cy - ay;
    let ab2 = abx * abx + aby * aby;
    let limit = i128::from(radius_mm);
    let limit2 = limit * limit;
    if ab2 == 0 {
        return acx * acx + acy * acy < limit2;
    }
    let dot = acx * abx + acy * aby;
    if dot <= 0 {
        return acx * acx + acy * acy < limit2;
    }
    if dot >= ab2 {
        let bcx = cx - bx;
        let bcy = cy - by;
        return bcx * bcx + bcy * bcy < limit2;
    }
    let cross = acx * aby - acy * abx;
    wide_less(
        cross.unsigned_abs(),
        u128::try_from(limit2).expect("radius square"),
        u128::try_from(ab2).expect("length square"),
    )
}

/// `left * left < right_a * right_b`, with products that may exceed `u128`.
fn wide_less(left: u128, right_a: u128, right_b: u128) -> bool {
    let (lh, ll) = mul_wide(left, left);
    let (rh, rl) = mul_wide(right_a, right_b);
    (lh, ll) < (rh, rl)
}

fn mul_wide(a: u128, b: u128) -> (u128, u128) {
    let a_lo = a & u128::from(u64::MAX);
    let a_hi = a >> 64;
    let b_lo = b & u128::from(u64::MAX);
    let b_hi = b >> 64;
    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;
    let mid = (ll >> 64) + (lh & u128::from(u64::MAX)) + (hl & u128::from(u64::MAX));
    let lo = (ll & u128::from(u64::MAX)) | ((mid & u128::from(u64::MAX)) << 64);
    let hi = hh + (lh >> 64) + (hl >> 64) + (mid >> 64);
    (hi, lo)
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
    use super::{GeomError, Mm, Point, segment_within, within_reach};

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

    fn at(x: i32, y: i32) -> Point {
        Point::new(Mm::new(x), Mm::new(y))
    }

    /// Independent of [`segment_within`]. Squared distance from `center` to the
    /// closest point of the segment, compared with `radius` squared.
    fn reference_hits(from: Point, to: Point, center: Point, radius_mm: i32) -> bool {
        if radius_mm <= 0 {
            return false;
        }
        let ax = i64::from(from.x().get());
        let ay = i64::from(from.y().get());
        let bx = i64::from(to.x().get());
        let by = i64::from(to.y().get());
        let cx = i64::from(center.x().get());
        let cy = i64::from(center.y().get());
        let abx = bx - ax;
        let aby = by - ay;
        let acx = cx - ax;
        let acy = cy - ay;
        let ab2 = abx * abx + aby * aby;
        let (px, py) = if ab2 == 0 {
            (ax, ay)
        } else {
            let dot = acx * abx + acy * aby;
            if dot <= 0 {
                (ax, ay)
            } else if dot >= ab2 {
                (bx, by)
            } else {
                (ax + abx * dot / ab2, ay + aby * dot / ab2)
            }
        };
        let dx = cx - px;
        let dy = cy - py;
        dx * dx + dy * dy < i64::from(radius_mm) * i64::from(radius_mm)
    }

    #[test]
    fn a_segment_cannot_tunnel_through_a_disk_between_clear_ends() {
        let from = at(-1000, 0);
        let to = at(1000, 0);
        let center = at(0, 0);
        assert!(within_reach(from, center, 1000));
        assert!(!within_reach(from, center, 999));
        assert!(!within_reach(to, center, 999));
        assert!(segment_within(from, to, center, 1));
        assert!(segment_within(from, to, center, 100));
        assert!(!segment_within(from, to, at(0, 100), 100));
        assert!(segment_within(from, to, at(0, 100), 101));
        assert!(!segment_within(from, to, at(0, 100), 0));
        assert!(!segment_within(from, to, at(0, 100), -5));
        assert!(segment_within(at(0, 0), at(0, 0), at(0, 0), 1));
        assert!(!segment_within(at(0, 0), at(0, 0), at(0, 0), 0));
        let samples = [
            (from, to, center, 50),
            (from, to, at(1000, 0), 1),
            (from, to, at(-1000, 1), 1),
            (from, to, at(2500, 0), 1499),
            (from, to, at(2500, 0), 1500),
            (at(0, 0), at(0, 0), at(3, 4), 5),
            (at(0, 0), at(0, 0), at(3, 4), 6),
            (at(-20, 5), at(40, -7), at(10, -1), 4),
            (at(i32::MAX - 5, 0), at(i32::MAX, 0), at(i32::MAX - 2, 0), 2),
            (
                at(i32::MIN, 0),
                at(i32::MIN + 10, 0),
                at(i32::MIN + 4, 3),
                4,
            ),
        ];
        for (start, end, disk, radius) in samples {
            assert_eq!(
                segment_within(start, end, disk, radius),
                reference_hits(start, end, disk, radius),
                "{start:?} {end:?} {disk:?} {radius}"
            );
        }
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
