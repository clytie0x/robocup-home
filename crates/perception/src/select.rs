use domain::{Id, Point};

use crate::assume;

/// One bag the selector is allowed to see. `visible` is the setup fact from the fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub id: Id,
    pub at: Point,
    pub visible: bool,
}

/// Why selection did not name a bag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectError {
    MissingOperator,
    MissingPointing,
    NoDirection,
    NoCandidate,
    DuplicateId,
    Ambiguous,
    NoHit,
}

impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SelectError::MissingOperator => "missing operator",
            SelectError::MissingPointing => "missing pointing",
            SelectError::NoDirection => "zero direction",
            SelectError::NoCandidate => "no candidate",
            SelectError::DuplicateId => "duplicate id",
            SelectError::Ambiguous => "ambiguous",
            SelectError::NoHit => "no hit",
        })
    }
}

impl std::error::Error for SelectError {}

/// Chooses the visible bag the ray points at, or a typed rejection.
///
/// `direction` is `(dx, dy)` in millimeters. The ray starts at `origin` and
/// continues only where the dot product with a candidate offset is positive.
/// The perpendicular distance is `|dx * py - dy * px| / hypot(dx, dy)`.
/// The comparison uses `floor(hypot)` so the corridor is never wider than
/// [`crate::assume::CORRIDOR_MM`]. That floor length is not scale-invariant:
/// a short integer direction rejects points that the same ray, written with a
/// longer direction, accepts. This is the integer test, not a physical width.
/// A unique choice also has to beat the next hit by more than
/// [`crate::assume::TIE_GAP_MM`] using `ceil(hypot)`, which refuses a tie that
/// quantization could flip. Neither comparison uses the candidate order or the
/// id value.
pub fn select(
    origin: Option<Point>,
    direction: Option<(i32, i32)>,
    candidates: &[Candidate],
) -> Result<Id, SelectError> {
    let Some(origin) = origin else {
        return Err(SelectError::MissingOperator);
    };
    let Some((dx, dy)) = direction else {
        return Err(SelectError::MissingPointing);
    };
    if dx == 0 && dy == 0 {
        return Err(SelectError::NoDirection);
    }
    if candidates.is_empty() {
        return Err(SelectError::NoCandidate);
    }
    if duplicate_ids(candidates) {
        return Err(SelectError::DuplicateId);
    }

    let (floor_len, ceil_len) = ray_length(dx, dy);
    let reach = mm(assume::CORRIDOR_MM) * floor_len;
    let mut best: Option<(Id, u128)> = None;
    let mut second: Option<u128> = None;
    for candidate in candidates {
        if !candidate.visible {
            continue;
        }
        let px = i128::from(candidate.at.x().get()) - i128::from(origin.x().get());
        let py = i128::from(candidate.at.y().get()) - i128::from(origin.y().get());
        let dot = i128::from(dx) * px + i128::from(dy) * py;
        if dot <= 0 {
            continue;
        }
        let cross = (i128::from(dx) * py - i128::from(dy) * px).unsigned_abs();
        if cross > reach {
            continue;
        }
        match best {
            None => best = Some((candidate.id, cross)),
            Some((_, dist)) if cross < dist => {
                second = Some(dist);
                best = Some((candidate.id, cross));
            }
            Some((_, _)) => {
                if second.is_none_or(|prev| cross < prev) {
                    second = Some(cross);
                }
            }
        }
    }

    let Some((id, dist)) = best else {
        return Err(SelectError::NoHit);
    };
    let Some(other) = second else {
        return Ok(id);
    };
    let gap = other.saturating_sub(dist);
    if gap > mm(assume::TIE_GAP_MM) * ceil_len {
        Ok(id)
    } else {
        Err(SelectError::Ambiguous)
    }
}

fn mm(value: i32) -> u128 {
    u128::from(u32::try_from(value).unwrap_or(0))
}

fn ray_length(dx: i32, dy: i32) -> (u128, u128) {
    let dx = u128::from(dx.unsigned_abs());
    let dy = u128::from(dy.unsigned_abs());
    let square = dx * dx + dy * dy;
    let floor = square.isqrt();
    let ceil = if floor * floor == square {
        floor
    } else {
        floor + 1
    };
    (floor, ceil)
}

fn duplicate_ids(candidates: &[Candidate]) -> bool {
    candidates.iter().enumerate().any(|(index, candidate)| {
        candidates[index + 1..]
            .iter()
            .any(|other| other.id == candidate.id)
    })
}

#[cfg(test)]
fn point(x: i32, y: i32) -> Point {
    Point::new(domain::Mm::new(x), domain::Mm::new(y))
}

#[cfg(test)]
fn cand(id: u32, x: i32, y: i32, visible: bool) -> Candidate {
    Candidate {
        id: Id::new(id).unwrap(),
        at: point(x, y),
        visible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin() -> Point {
        point(0, 0)
    }

    #[test]
    fn the_second_candidate_is_selected_when_the_ray_aims_at_it() {
        let east = cand(9, 1000, 0, true);
        let north = cand(2, 0, 1000, true);
        let chosen = select(Some(origin()), Some((1000, 0)), &[north, east]).unwrap();
        assert_eq!(chosen, east.id);
        assert_ne!(chosen, north.id);
        assert_ne!(chosen.get(), 1);
    }

    #[test]
    fn the_other_physical_bag_is_selected_when_the_ray_aims_there() {
        let expected_task = cand(3, 1200, 0, true);
        let pointed = cand(4, -800, 0, true);
        let chosen = select(Some(origin()), Some((-800, 0)), &[expected_task, pointed]).unwrap();
        assert_eq!(chosen, pointed.id);
        assert_ne!(chosen, expected_task.id);
    }

    #[test]
    fn reversed_candidate_order_returns_the_same_bag() {
        let east = cand(3, 1000, 0, true);
        let north = cand(8, 0, 1400, true);
        let forward = select(Some(origin()), Some((1000, 0)), &[east, north]).unwrap();
        let backward = select(Some(origin()), Some((1000, 0)), &[north, east]).unwrap();
        assert_eq!(forward, east.id);
        assert_eq!(backward, east.id);
    }

    #[test]
    fn a_diagonal_ray_selects_the_aligned_bag_and_not_the_first_id() {
        let off = cand(3, 0, 800, true);
        let on = cand(6, 300, 400, true);
        let chosen = select(Some(origin()), Some((3, 4)), &[off, on]).unwrap();
        assert_eq!(chosen, on.id);
    }

    #[test]
    fn the_corridor_boundary_is_inclusive_and_one_millimeter_outside_is_a_miss() {
        assert_eq!(assume::CORRIDOR_MM, 150);
        let on_edge = cand(3, 1000, assume::CORRIDOR_MM, true);
        let chosen = select(Some(origin()), Some((1000, 0)), &[on_edge]).unwrap();
        assert_eq!(chosen, on_edge.id);

        let outside = cand(3, 1000, assume::CORRIDOR_MM + 1, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[outside]),
            Err(SelectError::NoHit)
        );

        let inside = cand(7, 1000, 0, true);
        let beside = cand(8, 1000, assume::CORRIDOR_MM + 1, true);
        let chosen = select(Some(origin()), Some((1000, 0)), &[beside, inside]).unwrap();
        assert_eq!(chosen, inside.id);
    }

    #[test]
    fn a_two_millimeter_gap_is_ambiguous_and_a_three_millimeter_gap_is_unique() {
        assert_eq!(assume::TIE_GAP_MM, 2);
        let best = cand(3, 1000, 0, true);
        let close = cand(4, 1000, assume::TIE_GAP_MM, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[close, best]),
            Err(SelectError::Ambiguous)
        );
        let clear = cand(4, 1000, assume::TIE_GAP_MM + 1, true);
        let chosen = select(Some(origin()), Some((1000, 0)), &[clear, best]).unwrap();
        assert_eq!(chosen, best.id);
        assert_ne!(chosen, clear.id);
    }

    #[test]
    fn two_bags_on_the_same_ray_are_ambiguous() {
        let near = cand(3, 400, 0, true);
        let far = cand(4, 1600, 0, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[near, far]),
            Err(SelectError::Ambiguous)
        );
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[far, near]),
            Err(SelectError::Ambiguous)
        );
    }

    #[test]
    fn equal_positions_with_different_ids_are_ambiguous() {
        let left = cand(7, 1000, 0, true);
        let right = cand(8, 1000, 0, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[left, right]),
            Err(SelectError::Ambiguous)
        );
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[right, left]),
            Err(SelectError::Ambiguous)
        );
    }

    #[test]
    fn duplicate_ids_are_rejected_even_when_one_point_is_a_clear_hit() {
        let hit = cand(3, 1000, 0, true);
        let copy = cand(3, 0, 1000, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[hit, copy]),
            Err(SelectError::DuplicateId)
        );
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[copy, hit]),
            Err(SelectError::DuplicateId)
        );
    }

    #[test]
    fn a_zero_direction_is_rejected() {
        let bag = cand(3, 1000, 0, true);
        assert_eq!(
            select(Some(origin()), Some((0, 0)), &[bag]),
            Err(SelectError::NoDirection)
        );
        assert_eq!(
            select(Some(origin()), Some((0, 0)), &[]),
            Err(SelectError::NoDirection)
        );
    }

    #[test]
    fn a_missing_operator_or_pointing_is_rejected() {
        let bag = cand(3, 1000, 0, true);
        assert_eq!(
            select(None, Some((1000, 0)), &[bag]),
            Err(SelectError::MissingOperator)
        );
        assert_eq!(
            select(Some(origin()), None, &[bag]),
            Err(SelectError::MissingPointing)
        );
        assert_eq!(
            select(None, None, &[bag]),
            Err(SelectError::MissingOperator)
        );
    }

    #[test]
    fn an_empty_candidate_list_is_rejected() {
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[]),
            Err(SelectError::NoCandidate)
        );
    }

    #[test]
    fn a_reversed_ray_does_not_select_the_bag_it_used_to_aim_at() {
        let bag = cand(3, 1000, 0, true);
        assert_eq!(
            select(Some(origin()), Some((-1000, 0)), &[bag]),
            Err(SelectError::NoHit)
        );
    }

    #[test]
    fn a_bag_behind_or_beside_the_operator_is_not_a_hit() {
        let behind = cand(3, -500, 20, true);
        let beside = cand(4, 0, 500, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[behind, beside]),
            Err(SelectError::NoHit)
        );
        let at_origin = cand(5, 0, 0, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[at_origin]),
            Err(SelectError::NoHit)
        );
    }

    #[test]
    fn an_invisible_bag_on_the_ray_is_not_selected() {
        let hidden = cand(3, 1000, 0, false);
        let visible_off = cand(4, 0, 1000, true);
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[hidden, visible_off]),
            Err(SelectError::NoHit)
        );
        assert_eq!(
            select(Some(origin()), Some((1000, 0)), &[hidden]),
            Err(SelectError::NoHit)
        );
    }

    #[test]
    fn a_hidden_bag_on_the_ray_does_not_steal_or_tie_the_visible_bag() {
        let hidden = cand(3, 1000, 0, false);
        let visible = cand(4, 1000, 10, true);
        let chosen = select(Some(origin()), Some((1000, 0)), &[hidden, visible]).unwrap();
        assert_eq!(chosen, visible.id);
        let also_on_ray = cand(3, 400, 0, false);
        let visible_on_ray = cand(4, 1500, 0, true);
        let chosen = select(
            Some(origin()),
            Some((1000, 0)),
            &[also_on_ray, visible_on_ray],
        )
        .unwrap();
        assert_eq!(chosen, visible_on_ray.id);
    }

    #[test]
    fn large_coordinates_select_the_aligned_bag() {
        let on = cand(3, i32::MAX, 0, true);
        let off = cand(4, i32::MAX, 1000, true);
        let chosen = select(Some(origin()), Some((i32::MAX, 0)), &[off, on]).unwrap();
        assert_eq!(chosen, on.id);

        let origin = point(i32::MIN, i32::MIN / 2);
        let on = cand(8, i32::MIN + 800, i32::MIN / 2, true);
        let off = cand(9, i32::MIN, i32::MIN / 2 + 900, true);
        let chosen = select(Some(origin), Some((800, 0)), &[off, on]).unwrap();
        assert_eq!(chosen, on.id);
        assert_ne!(chosen, off.id);
    }

    #[test]
    fn a_non_integer_length_accepts_cross_299_and_rejects_cross_301() {
        let inside = cand(3, 1, 150, true);
        let chosen = select(Some(origin()), Some((2, 1)), &[inside]).unwrap();
        assert_eq!(chosen, inside.id);
        let outside = cand(4, 1, 151, true);
        assert_eq!(
            select(Some(origin()), Some((2, 1)), &[outside]),
            Err(SelectError::NoHit)
        );
        assert_ne!(
            select(Some(origin()), Some((2, 1)), &[outside]),
            Ok(outside.id)
        );
    }

    #[test]
    fn the_floor_corridor_rejects_a_short_direction_that_a_longer_one_accepts() {
        let bag = cand(5, 0, 180, true);
        assert_eq!(
            select(Some(origin()), Some((1, 1)), &[bag]),
            Err(SelectError::NoHit)
        );
        let chosen = select(Some(origin()), Some((100, 100)), &[bag]).unwrap();
        assert_eq!(chosen, bag.id);
    }

    #[test]
    fn the_same_sample_replays_the_same_result() {
        let bags = [cand(4, 500, 40, true), cand(3, 1000, 0, true)];
        let once = select(Some(origin()), Some((1000, 0)), &bags);
        let twice = select(Some(origin()), Some((1000, 0)), &bags);
        assert_eq!(once, twice);
        assert_eq!(once, Ok(bags[1].id));
        let ambiguous = select(Some(origin()), Some((1000, 0)), &[bags[1], bags[1]]);
        assert_eq!(ambiguous, Err(SelectError::DuplicateId));
        assert_eq!(
            ambiguous,
            select(Some(origin()), Some((1000, 0)), &[bags[1], bags[1]])
        );
    }
}
