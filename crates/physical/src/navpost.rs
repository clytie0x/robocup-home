//! One captured scan posted into the existing navigation view.
//!
//! The caller names the front. This module does not choose 178.7 over 183.5,
//! does not add the configured mount offset, and does not send a command.

use std::fs;
use std::path::Path;

use domain::{Mm, Point, within_reach};
use navigation::{
    Cue, Follower, Intent, Posted, SampleHistory, SampleKind, SeenBody, SensorView, Taken, accept,
    plan_follow,
};

use crate::chunk::{Chunk, parse_chunks};
use crate::kind::Kind;
use crate::lidar::{RawSample, parse_lidar, split_revolutions};
use crate::observe::Freshness;
use crate::observe::stamp_order;
use crate::plot::{XySeries, robot_plan};
use crate::profile::{LidarProfile, base_xy, lidar_profiles};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleUse {
    Body { x_mm: i32, y_mm: i32 },
    ZeroReturn,
    NegativeRange,
    Boundary,
    NonFinite,
    Extreme,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockLabel {
    CaptureMonotonic,
    InjectedReplay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PostClock {
    pub captured_ms: Option<u64>,
    pub delivered_ms: u64,
    pub decision_ms: u64,
    pub label: ClockLabel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnusableKind {
    Missing,
    Stale,
    Duplicate,
    Reversed,
    Future,
    Malformed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostOutcome {
    Acted { intent: Intent },
    Unusable { kind: UnusableKind },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanPost {
    pub profile_id: &'static str,
    pub front_kind: Kind,
    pub uses: Vec<SampleUse>,
    pub view_points: Vec<(i32, i32)>,
    pub outcome: PostOutcome,
    pub sent: bool,
    pub command_text: String,
    pub people: usize,
    pub bags: usize,
    pub clock: ClockLabel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChunkReport {
    pub clock: ClockLabel,
    pub view_points: usize,
    pub raw_samples: usize,
    pub revolution_samples: usize,
    pub zero_returns: usize,
    pub people: usize,
    pub alternate_points_differ: usize,
    pub fresh: PostOutcome,
    pub stale: PostOutcome,
    pub sent: bool,
    pub profile_id: &'static str,
    pub front_kind: Kind,
    pub captured_ms: Option<u64>,
    pub points_m: Vec<(f64, f64)>,
}

/// Milliseconds from `origin_s` to `stamp_s` on one monotonic clock.
///
/// A negative, non-finite, or overflowing span is an error. It is not zero.
pub fn offset_ms(origin_s: f64, stamp_s: f64) -> Option<u64> {
    if !origin_s.is_finite() || !stamp_s.is_finite() {
        return None;
    }
    let delta_s = stamp_s - origin_s;
    if delta_s < 0.0 {
        return None;
    }
    let ms = (delta_s * 1000.0).round();
    if !ms.is_finite() || ms > u64::MAX as f64 {
        return None;
    }
    Some(ms as u64)
}

pub fn frame_sample(profile: &LidarProfile, raw_deg: f64, range_m: f64) -> SampleUse {
    if !profile.front_raw_deg.is_finite() || !raw_deg.is_finite() || !range_m.is_finite() {
        return SampleUse::NonFinite;
    }
    if range_m < 0.0 {
        return SampleUse::NegativeRange;
    }
    if range_m == 0.0 {
        return SampleUse::ZeroReturn;
    }
    let (x_m, y_m) = base_xy(raw_deg, range_m, profile.front_raw_deg);
    if !x_m.is_finite() || !y_m.is_finite() {
        return SampleUse::NonFinite;
    }
    let Ok(x_mm) = Mm::try_from_meters(x_m) else {
        return SampleUse::Extreme;
    };
    let Ok(y_mm) = Mm::try_from_meters(y_m) else {
        return SampleUse::Extreme;
    };
    if x_mm.get() == 0 && y_mm.get() == 0 {
        return SampleUse::Boundary;
    }
    SampleUse::Body {
        x_mm: x_mm.get(),
        y_mm: y_mm.get(),
    }
}

pub fn post_samples(
    profile: &LidarProfile,
    samples: &[RawSample],
    history: SampleHistory,
    clock: PostClock,
) -> (SampleHistory, ScanPost) {
    let uses: Vec<SampleUse> = samples
        .iter()
        .map(|sample| frame_sample(profile, sample.raw_deg, sample.range_m))
        .collect();
    let framed: Vec<(i32, i32)> = uses
        .iter()
        .filter_map(|use_| match use_ {
            SampleUse::Body { x_mm, y_mm } => Some((*x_mm, *y_mm)),
            _ => None,
        })
        .collect();
    let origin = Point::new(Mm::new(0), Mm::new(0));
    let in_view: Vec<(i32, i32)> = framed
        .into_iter()
        .filter(|(x_mm, y_mm)| {
            within_reach(
                origin,
                Point::new(Mm::new(*x_mm), Mm::new(*y_mm)),
                navigation::assume::SENSOR_MM,
            )
        })
        .collect();
    let mut post = ScanPost {
        profile_id: profile.id,
        front_kind: profile.kind,
        uses,
        view_points: Vec::new(),
        outcome: PostOutcome::Unusable {
            kind: UnusableKind::Missing,
        },
        sent: false,
        command_text: String::new(),
        people: 0,
        bags: 0,
        clock: clock.label,
    };
    let Some(captured_ms) = clock.captured_ms else {
        let posted = Posted::missing(clock.delivered_ms);
        let (history, taken, stamp) = accept(history, &posted, clock.decision_ms);
        post.outcome = outcome_from(taken, stamp.kind);
        return (history, post);
    };
    let bodies: Vec<SeenBody> = in_view
        .iter()
        .map(|(x_mm, y_mm)| SeenBody {
            center: Point::new(Mm::new(*x_mm), Mm::new(*y_mm)),
            radius_mm: 0,
        })
        .collect();
    let view = SensorView {
        robot: origin,
        people: Vec::new(),
        bodies,
        bags: Vec::new(),
        gesture: false,
        captured_ms,
    };
    let posted = Posted::ready(view, clock.delivered_ms);
    let (history, taken, stamp) = accept(history, &posted, clock.decision_ms);
    let acted_points = match &taken {
        Taken::Act(view) => view
            .bodies
            .iter()
            .map(|body| (body.center.x().get(), body.center.y().get()))
            .collect(),
        Taken::NeedFresh | Taken::Rejected => Vec::new(),
    };
    post.outcome = outcome_from(taken, stamp.kind);
    post.view_points = acted_points;
    (history, post)
}

fn outcome_from(taken: Taken, kind: SampleKind) -> PostOutcome {
    match taken {
        Taken::Act(view) => {
            let follower = Follower::follow(domain::Id::new(1).expect("follower id"));
            let intent = plan_follow(&follower, &view.sight(), Cue::None).1;
            PostOutcome::Acted { intent }
        }
        Taken::NeedFresh => PostOutcome::Unusable {
            kind: match kind {
                SampleKind::Stale => UnusableKind::Stale,
                SampleKind::Duplicate => UnusableKind::Duplicate,
                SampleKind::Missing => UnusableKind::Missing,
                SampleKind::Reversed => UnusableKind::Reversed,
                SampleKind::Future => UnusableKind::Future,
                SampleKind::Ready => UnusableKind::Malformed,
            },
        },
        Taken::Rejected => PostOutcome::Unusable {
            kind: match kind {
                SampleKind::Future => UnusableKind::Future,
                SampleKind::Reversed => UnusableKind::Reversed,
                _ => UnusableKind::Malformed,
            },
        },
    }
}

pub fn post_chunks(chunks: &[Chunk], profile: &LidarProfile) -> Result<ChunkReport, String> {
    if chunks.is_empty() {
        return Ok(empty_report(profile, UnusableKind::Missing));
    }
    let times: Vec<f64> = chunks.iter().map(|chunk| chunk.t_s).collect();
    let order = stamp_order(&times);
    let (samples, spans) = concatenate(chunks);
    let raw_samples = samples.len();
    let zero_returns = samples
        .iter()
        .filter(|sample| sample.range_m == 0.0)
        .count();
    if order != Freshness::Fresh {
        let kind = match order {
            Freshness::Reversed => UnusableKind::Reversed,
            Freshness::Malformed => UnusableKind::Malformed,
            Freshness::Missing => UnusableKind::Missing,
            Freshness::Stale { .. } => UnusableKind::Stale,
            Freshness::Fresh => UnusableKind::Malformed,
        };
        return Ok(ChunkReport {
            raw_samples,
            zero_returns,
            fresh: PostOutcome::Unusable { kind },
            ..empty_report(profile, kind)
        });
    }
    let Some(revolution) = longest(split_revolutions(&samples)) else {
        return Ok(ChunkReport {
            raw_samples,
            zero_returns,
            ..empty_report(profile, UnusableKind::Missing)
        });
    };
    let Some(first) = revolution.first() else {
        return Ok(empty_report(profile, UnusableKind::Missing));
    };
    let Some(stamp_s) = time_at(&spans, first.at) else {
        return Ok(ChunkReport {
            raw_samples,
            ..empty_report(profile, UnusableKind::Malformed)
        });
    };
    let origin_s = chunks[0].t_s;
    let Some(captured_ms) = offset_ms(origin_s, stamp_s) else {
        return Ok(ChunkReport {
            raw_samples,
            zero_returns,
            fresh: PostOutcome::Unusable {
                kind: UnusableKind::Malformed,
            },
            ..empty_report(profile, UnusableKind::Malformed)
        });
    };
    let fresh_clock = PostClock {
        captured_ms: Some(captured_ms),
        delivered_ms: captured_ms,
        decision_ms: captured_ms,
        label: ClockLabel::CaptureMonotonic,
    };
    let stale_decision = captured_ms.saturating_add(navigation::MAX_SAMPLE_AGE_MS + 1);
    let stale_clock = PostClock {
        captured_ms: Some(captured_ms),
        delivered_ms: captured_ms,
        decision_ms: stale_decision,
        label: ClockLabel::CaptureMonotonic,
    };
    let revolution_samples = revolution.len();
    let zero_returns = revolution
        .iter()
        .filter(|sample| sample.range_m == 0.0)
        .count();
    let (_history, fresh) = post_samples(profile, revolution, SampleHistory::new(), fresh_clock);
    let (_history, stale) = post_samples(profile, revolution, SampleHistory::new(), stale_clock);
    let other = other_profile(profile);
    let alternate_points_differ = revolution
        .iter()
        .filter(|sample| {
            frame_sample(profile, sample.raw_deg, sample.range_m)
                != frame_sample(&other, sample.raw_deg, sample.range_m)
        })
        .count();
    let points_m = fresh
        .view_points
        .iter()
        .map(|(x_mm, y_mm)| (f64::from(*x_mm) / 1000.0, f64::from(*y_mm) / 1000.0))
        .collect();
    Ok(ChunkReport {
        clock: ClockLabel::CaptureMonotonic,
        view_points: fresh.view_points.len(),
        raw_samples,
        revolution_samples,
        zero_returns,
        people: fresh.people,
        alternate_points_differ,
        fresh: fresh.outcome,
        stale: stale.outcome,
        sent: fresh.sent,
        profile_id: profile.id,
        front_kind: profile.kind,
        captured_ms: Some(captured_ms),
        points_m,
    })
}

pub fn write_chunk_post(
    chunks: &[Chunk],
    profile: &LidarProfile,
    out: &Path,
) -> Result<ChunkReport, String> {
    let report = post_chunks(chunks, profile)?;
    fs::create_dir_all(out).map_err(|err| err.to_string())?;
    fs::write(out.join("post.csv"), post_csv(&report)).map_err(|err| err.to_string())?;
    let series = [XySeries {
        name: format!("{} ({})", report.profile_id, report.front_kind.as_str()),
        kind: "posted-view",
        color: "#c0392b",
        points: report.points_m.clone(),
    }];
    let svg = robot_plan(
        &format!(
            "Navigation view of one captured revolution. Profile: {}. Not a mission.",
            report.profile_id
        ),
        "Points are the bodies inside the existing navigation sensor range. Radius is 0. Zeros are not drawn. The clock is capture monotonic time, not wall time. No command was sent.",
        &series,
    );
    fs::write(out.join("post.svg"), svg).map_err(|err| err.to_string())?;
    let mut points = String::from("x_m,y_m\n");
    for (x_m, y_m) in &report.points_m {
        points.push_str(&format!("{x_m:.3},{y_m:.3}\n"));
    }
    fs::write(out.join("points.csv"), points).map_err(|err| err.to_string())?;
    Ok(report)
}

fn post_csv(report: &ChunkReport) -> String {
    format!(
        "field,value\nprofile,{}\nfront_kind,{}\nclock,{}\ncaptured_ms,{}\nfile_samples,{}\nrevolution_samples,{}\nzero_returns,{}\nview_points,{}\npeople,{}\nalternate_points_differ,{}\nfresh,{}\nstale,{}\nsent,{}\ncommand,\nclear_field,false\n",
        report.profile_id,
        report.front_kind.as_str(),
        clock_name(report.clock),
        report
            .captured_ms
            .map(|ms| ms.to_string())
            .unwrap_or_else(|| "missing".to_string()),
        report.raw_samples,
        report.revolution_samples,
        report.zero_returns,
        report.view_points,
        report.people,
        report.alternate_points_differ,
        outcome_name(&report.fresh),
        outcome_name(&report.stale),
        report.sent,
    )
}

fn clock_name(label: ClockLabel) -> &'static str {
    match label {
        ClockLabel::CaptureMonotonic => "capture-monotonic",
        ClockLabel::InjectedReplay => "injected-replay",
    }
}

fn outcome_name(outcome: &PostOutcome) -> String {
    match outcome {
        PostOutcome::Acted { intent } => format!("acted:{intent:?}"),
        PostOutcome::Unusable { kind } => format!("unusable:{kind:?}"),
    }
}

fn empty_report(profile: &LidarProfile, kind: UnusableKind) -> ChunkReport {
    ChunkReport {
        clock: ClockLabel::CaptureMonotonic,
        view_points: 0,
        raw_samples: 0,
        revolution_samples: 0,
        zero_returns: 0,
        people: 0,
        alternate_points_differ: 0,
        fresh: PostOutcome::Unusable { kind },
        stale: PostOutcome::Unusable { kind },
        sent: false,
        profile_id: profile.id,
        front_kind: profile.kind,
        captured_ms: None,
        points_m: Vec::new(),
    }
}

fn other_profile(profile: &LidarProfile) -> LidarProfile {
    let profiles = lidar_profiles();
    if profile.id == profiles[0].id {
        profiles[1]
    } else {
        profiles[0]
    }
}

fn concatenate(chunks: &[Chunk]) -> (Vec<RawSample>, Vec<(usize, f64)>) {
    let mut bytes = Vec::new();
    let mut spans = Vec::new();
    for chunk in chunks {
        if !chunk.t_s.is_finite() {
            continue;
        }
        bytes.extend_from_slice(&chunk.bytes);
        spans.push((bytes.len(), chunk.t_s));
    }
    let finite = chunks.iter().all(|chunk| chunk.t_s.is_finite());
    if !finite {
        let parsed = parse_lidar(
            &chunks
                .iter()
                .flat_map(|chunk| chunk.bytes.iter().copied())
                .collect::<Vec<_>>(),
        );
        return (parsed.samples, Vec::new());
    }
    (parse_lidar(&bytes).samples, spans)
}

fn longest(revolutions: Vec<&[RawSample]>) -> Option<&[RawSample]> {
    let mut best: Option<&[RawSample]> = None;
    for revolution in revolutions {
        if best
            .map(|kept| revolution.len() > kept.len())
            .unwrap_or(true)
        {
            best = Some(revolution);
        }
    }
    best
}

fn time_at(spans: &[(usize, f64)], at: usize) -> Option<f64> {
    spans
        .iter()
        .find(|(end, _)| at < *end)
        .map(|(_, time)| *time)
}

pub fn load_chunks(bytes: &[u8]) -> Result<Vec<Chunk>, String> {
    let parsed = parse_chunks(bytes);
    if let Some(err) = parsed.error {
        return Err(err);
    }
    Ok(parsed.chunks)
}

#[cfg(test)]
mod tests {
    use super::{
        ClockLabel, PostClock, PostOutcome, SampleUse, UnusableKind, frame_sample, offset_ms,
        post_chunks, post_samples, write_chunk_post,
    };
    use crate::chunk::Chunk;
    use crate::kind::Kind;
    use crate::lidar::{RawSample, encode_packet};
    use crate::profile::{LIDAR_X_M, lidar_profiles};
    use domain::{Id, Mm, Point};
    use navigation::{
        Cue, Follower, Intent, SampleHistory, SeenBody, SeenPerson, Sight, plan_follow,
    };

    fn measured() -> crate::profile::LidarProfile {
        let profile = lidar_profiles()[0];
        assert_eq!(profile.id, "measured-front");
        assert_eq!(profile.front_raw_deg, 178.7);
        assert_eq!(profile.kind, Kind::Measured);
        profile
    }

    fn configured() -> crate::profile::LidarProfile {
        let profile = lidar_profiles()[1];
        assert_eq!(profile.id, "configured-front");
        assert_eq!(profile.front_raw_deg, 183.5);
        assert_eq!(profile.kind, Kind::Configured);
        profile
    }

    fn sample(raw_deg: f64, range_m: f64) -> RawSample {
        RawSample {
            at: 0,
            raw_deg,
            range_m,
            intensity: 0,
        }
    }

    fn injected(captured_ms: Option<u64>, delivered_ms: u64, decision_ms: u64) -> PostClock {
        PostClock {
            captured_ms,
            delivered_ms,
            decision_ms,
            label: ClockLabel::InjectedReplay,
        }
    }

    #[test]
    fn a_straight_metre_uses_the_named_front_and_not_the_other() {
        let measured_hit = frame_sample(&measured(), 178.7, 1.0);
        let configured_hit = frame_sample(&configured(), 178.7, 1.0);
        assert_eq!(
            measured_hit,
            SampleUse::Body {
                x_mm: 1000,
                y_mm: 0
            }
        );
        assert_eq!(
            configured_hit,
            SampleUse::Body {
                x_mm: 996,
                y_mm: 84
            }
        );
        assert_ne!(measured_hit, configured_hit);
        let mount_mm = (LIDAR_X_M * 1000.0).round() as i32;
        assert_ne!(
            measured_hit,
            SampleUse::Body {
                x_mm: 1000 + mount_mm,
                y_mm: 0
            }
        );
        assert_ne!(
            measured_hit,
            SampleUse::Body {
                x_mm: 1136,
                y_mm: 0
            }
        );
    }

    #[test]
    fn left_right_and_behind_keep_their_signs() {
        assert_eq!(
            frame_sample(&measured(), 88.7, 1.0),
            SampleUse::Body {
                x_mm: 0,
                y_mm: 1000
            }
        );
        assert_eq!(
            frame_sample(&measured(), 268.7, 1.0),
            SampleUse::Body {
                x_mm: 0,
                y_mm: -1000
            }
        );
        assert_eq!(
            frame_sample(&measured(), 358.7, 1.0),
            SampleUse::Body {
                x_mm: -1000,
                y_mm: 0
            }
        );
    }

    #[test]
    fn zero_negative_boundary_and_non_finite_samples_are_not_bodies() {
        assert_eq!(frame_sample(&measured(), 178.7, 0.0), SampleUse::ZeroReturn);
        assert_eq!(
            frame_sample(&measured(), 178.7, -0.2),
            SampleUse::NegativeRange
        );
        assert_eq!(
            frame_sample(&measured(), 178.7, 0.0004),
            SampleUse::Boundary
        );
        assert_eq!(
            frame_sample(&measured(), f64::NAN, 1.0),
            SampleUse::NonFinite
        );
        assert_eq!(
            frame_sample(&measured(), 178.7, f64::INFINITY),
            SampleUse::NonFinite
        );
        assert_eq!(frame_sample(&measured(), 178.7, 1.0e20), SampleUse::Extreme);
    }

    #[test]
    fn the_navigation_range_keeps_2500_mm_and_drops_2501_mm() {
        let samples = [sample(178.7, 2.5), sample(178.7, 2.501)];
        let (_history, post) = post_samples(
            &measured(),
            &samples,
            SampleHistory::new(),
            injected(Some(10), 10, 10),
        );
        assert_eq!(
            post.outcome,
            PostOutcome::Acted {
                intent: Intent::Lost
            }
        );
        assert_eq!(post.view_points, vec![(2500, 0)]);
        assert!(
            post.uses
                .iter()
                .any(|use_| matches!(use_, SampleUse::Body { x_mm: 2501, .. }))
        );
        assert_eq!(post.people, 0);
        assert_eq!(post.bags, 0);
        assert!(!post.sent);
        assert!(post.command_text.is_empty());
        assert_eq!(post.clock, ClockLabel::InjectedReplay);
    }

    #[test]
    fn a_stale_or_missing_clock_keeps_the_sample_and_does_not_plan() {
        let samples = [sample(178.7, 1.0)];
        let (_history, stale) = post_samples(
            &measured(),
            &samples,
            SampleHistory::new(),
            injected(Some(0), 0, 1_001),
        );
        assert_eq!(
            stale.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Stale
            }
        );
        assert!(stale.view_points.is_empty());
        assert!(matches!(
            stale.uses.first(),
            Some(SampleUse::Body {
                x_mm: 1000,
                y_mm: 0
            })
        ));
        let (_history, missing) = post_samples(
            &measured(),
            &samples,
            SampleHistory::new(),
            injected(None, 0, 0),
        );
        assert_eq!(
            missing.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Missing
            }
        );
        assert!(missing.view_points.is_empty());
        assert_eq!(missing.uses.len(), 1);
        assert!(offset_ms(0.0, f64::NAN).is_none());
        assert!(offset_ms(2.0, 1.0).is_none());
        assert_eq!(offset_ms(10.0, 11.5).unwrap(), 1_500);
    }

    #[test]
    fn reversed_duplicate_and_future_clocks_are_unusable() {
        let samples = [sample(178.7, 1.0)];
        let (history, first) = post_samples(
            &measured(),
            &samples,
            SampleHistory::new(),
            injected(Some(100), 100, 100),
        );
        assert!(matches!(first.outcome, PostOutcome::Acted { .. }));
        let (after, reversed) =
            post_samples(&measured(), &samples, history, injected(Some(40), 200, 200));
        assert_eq!(
            reversed.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Reversed
            }
        );
        assert_eq!(after.last_capture_ms(), Some(100));
        assert!(reversed.view_points.is_empty());
        let (_history, duplicate) =
            post_samples(&measured(), &samples, after, injected(Some(100), 300, 300));
        assert_eq!(
            duplicate.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Duplicate
            }
        );
        let (_history, future) = post_samples(
            &configured(),
            &samples,
            SampleHistory::new(),
            injected(Some(500), 100, 100),
        );
        assert_eq!(
            future.outcome,
            PostOutcome::Unusable {
                kind: UnusableKind::Future
            }
        );
        assert!(future.view_points.is_empty());
    }

    #[test]
    fn a_posted_body_changes_the_existing_follow_plan() {
        let (_history, post) = post_samples(
            &measured(),
            &[sample(178.7, 0.1)],
            SampleHistory::new(),
            injected(Some(1), 1, 1),
        );
        let PostOutcome::Acted { .. } = post.outcome else {
            panic!("{:?}", post.outcome);
        };
        let bodies: Vec<SeenBody> = post
            .view_points
            .iter()
            .map(|(x, y)| SeenBody {
                center: Point::new(Mm::new(*x), Mm::new(*y)),
                radius_mm: 0,
            })
            .collect();
        assert_eq!(bodies.len(), 1);
        let person = SeenPerson {
            id: Id::new(2).expect("id"),
            at: Point::new(Mm::new(3_000), Mm::new(0)),
        };
        let robot = Point::new(Mm::new(0), Mm::new(0));
        let with = Sight {
            robot,
            people: std::slice::from_ref(&person),
            bodies: &bodies,
        };
        let without = Sight {
            robot,
            people: std::slice::from_ref(&person),
            bodies: &[],
        };
        let follower = Follower::follow(person.id);
        let blocked = plan_follow(&follower, &with, Cue::None).1;
        let open = plan_follow(&follower, &without, Cue::None).1;
        assert_ne!(blocked, open);
        assert!(matches!(open, Intent::Move(_)));
    }

    fn packet(angle: f64, range_word: u16) -> Vec<u8> {
        encode_packet(0, &[(angle, range_word, 0)]).expect("packet")
    }

    #[test]
    fn a_chunk_clock_posts_one_scan_and_a_bad_clock_does_not() {
        let bytes = packet(178.7, 4_000);
        let report = post_chunks(
            &[Chunk {
                t_s: 5.0,
                bytes: bytes.clone(),
            }],
            &measured(),
        )
        .expect("chunks");
        assert_eq!(report.clock, ClockLabel::CaptureMonotonic);
        assert_eq!(report.view_points, 1);
        assert_eq!(report.raw_samples, 1);
        assert_eq!(report.people, 0);
        assert!(report.alternate_points_differ >= 1);
        assert_eq!(
            report.fresh,
            PostOutcome::Acted {
                intent: Intent::Lost
            }
        );
        assert_eq!(
            report.stale,
            PostOutcome::Unusable {
                kind: UnusableKind::Stale
            }
        );
        assert!(!report.sent);
        let bad = post_chunks(
            &[Chunk {
                t_s: f64::NAN,
                bytes: bytes.clone(),
            }],
            &measured(),
        )
        .expect("nan clock");
        assert_eq!(
            bad.fresh,
            PostOutcome::Unusable {
                kind: UnusableKind::Malformed
            }
        );
        assert_eq!(bad.view_points, 0);
        assert!(bad.raw_samples >= 1);
        let reversed = post_chunks(
            &[
                Chunk {
                    t_s: 5.0,
                    bytes: bytes.clone(),
                },
                Chunk { t_s: 4.0, bytes },
            ],
            &measured(),
        )
        .expect("order");
        assert_eq!(
            reversed.fresh,
            PostOutcome::Unusable {
                kind: UnusableKind::Reversed
            }
        );
        assert_eq!(reversed.view_points, 0);
    }

    #[test]
    fn the_csv_and_svg_repeat_the_posted_view() {
        let dir = std::env::temp_dir().join(format!("scan-nav-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let report = write_chunk_post(
            &[Chunk {
                t_s: 5.0,
                bytes: packet(178.7, 4_000),
            }],
            &measured(),
            &dir,
        )
        .expect("write");
        let csv = std::fs::read_to_string(dir.join("post.csv")).expect("csv");
        let points = std::fs::read_to_string(dir.join("points.csv")).expect("points");
        let svg = std::fs::read_to_string(dir.join("post.svg")).expect("svg");
        assert!(csv.contains(&format!("view_points,{}", report.view_points)));
        assert!(csv.contains("profile,measured-front"));
        assert!(csv.contains("clock,capture-monotonic"));
        assert!(csv.contains("sent,false"));
        assert!(csv.contains("clear_field,false"));
        assert!(csv.contains("fresh,acted:Lost"));
        assert_eq!(points.lines().count(), report.view_points + 1);
        assert!(points.contains("1.000,0.000"));
        assert!(svg.contains("data-origin=\"robot\""));
        assert!(!svg.contains("203.0.113"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
