//! Sixteen scenarios, locked before the matrix runs.
//!
//! `Expect` is the predeclared claim. `Basis::Prior` repeats an outcome already
//! measured on this code. `Basis::Hypothesis` is a claim that is not edited
//! after the run. `Expect::Record` makes no task-success claim.
//! The judge reads the world fields copied onto the report. It does not read
//! the script name, the trip flag, or the planner.

use domain::{Mm, Point, within_reach};
use navigation::assume::ARRIVE_MM;
use sim_runtime::assume::INSIDE_X_MAX_MM;
use task_3_1::LIMIT_MS;

use crate::episode::{EpisodeReport, Probe, Script, execute_probe};

fn near(a: (i32, i32), b: (i32, i32)) -> bool {
    within_reach(
        Point::new(Mm::new(a.0), Mm::new(a.1)),
        Point::new(Mm::new(b.0), Mm::new(b.1)),
        ARRIVE_MM,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    TruthPass,
    TruthFail,
    Record,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Basis {
    Prior,
    Hypothesis,
}

struct Slot {
    id: &'static str,
    covers: &'static str,
    probe: Probe,
    expect: Expect,
    basis: Basis,
}

fn main_seed(seed: u64) -> Probe {
    Probe::main(seed)
}

fn slots() -> [Slot; 16] {
    [
        Slot {
            id: "S01",
            covers: "normal, delay 0, crowd 4",
            probe: main_seed(1),
            expect: Expect::TruthPass,
            basis: Basis::Prior,
        },
        Slot {
            id: "S02",
            covers: "wrong-place cue",
            probe: Probe::from_script(Script::EarlyCue { seed: 1 }),
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
        Slot {
            id: "S03",
            covers: "delay 200",
            probe: Probe {
                delay_ms: 200,
                ..main_seed(1)
            },
            expect: Expect::TruthPass,
            basis: Basis::Prior,
        },
        Slot {
            id: "S04",
            covers: "delay 1000",
            probe: Probe {
                delay_ms: 1_000,
                ..main_seed(1)
            },
            expect: Expect::TruthPass,
            basis: Basis::Prior,
        },
        Slot {
            id: "S05",
            covers: "missing delivery",
            probe: Probe {
                miss_one: true,
                ..main_seed(1)
            },
            expect: Expect::TruthPass,
            basis: Basis::Prior,
        },
        Slot {
            id: "S06",
            covers: "reversed stamp",
            probe: Probe {
                reverse_once: true,
                ..main_seed(1)
            },
            expect: Expect::TruthPass,
            basis: Basis::Hypothesis,
        },
        Slot {
            id: "S07",
            covers: "bounded position error 8 mm",
            probe: Probe {
                dx: 8,
                dy: -8,
                ..main_seed(1)
            },
            expect: Expect::TruthPass,
            basis: Basis::Prior,
        },
        Slot {
            id: "S08",
            covers: "causal loss, stay away",
            probe: Probe {
                loss_stay: true,
                ..Probe::from_script(Script::Loss { seed: 1 })
            },
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
        Slot {
            id: "S09",
            covers: "blocked approach, sensor omit",
            probe: Probe {
                block_approach: true,
                hide_block: true,
                ..main_seed(1)
            },
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
        Slot {
            id: "S10",
            covers: "out-of-reach grasp",
            probe: Probe {
                reach_fail: true,
                ..main_seed(1)
            },
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
        Slot {
            id: "S11",
            covers: "blocked handoff",
            probe: Probe::from_script(Script::BlockedHandoff { seed: 1 }),
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
        Slot {
            id: "S12",
            covers: "actual drop",
            probe: Probe::from_script(Script::Drop { seed: 1 }),
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
        Slot {
            id: "S13",
            covers: "crowd 3",
            probe: main_seed(2),
            expect: Expect::TruthPass,
            basis: Basis::Hypothesis,
        },
        Slot {
            id: "S14",
            covers: "missing obstacle",
            probe: Probe::from_script(Script::Strip { seed: 1 }),
            expect: Expect::TruthPass,
            basis: Basis::Hypothesis,
        },
        Slot {
            id: "S15",
            covers: "unmassaged geometry",
            probe: Probe {
                raw_layout: true,
                ..main_seed(1)
            },
            expect: Expect::Record,
            basis: Basis::Hypothesis,
        },
        Slot {
            id: "S16",
            covers: "missing person",
            probe: Probe::from_script(Script::Unseen { seed: 1 }),
            expect: Expect::TruthFail,
            basis: Basis::Prior,
        },
    ]
}

const HELD_OUT_SEEDS: [u64; 2] = [8, 11];
const DEVELOPMENT_SEEDS: [u64; 2] = [1, 2];

struct Row {
    id: &'static str,
    covers: &'static str,
    expect: Expect,
    basis: Basis,
    truth_ok: bool,
    main_passed: bool,
    phase: &'static str,
    fail: Option<&'static str>,
    custody: &'static str,
    clock_ms: u64,
    crowd: usize,
    max_abs: i32,
    saw_21400: bool,
}

fn judge_at(report: &EpisodeReport, car: (i32, i32), custody: &str) -> bool {
    let at_car = report.operator == car && custody == "operator";
    let near = near(report.robot, report.operator);
    let outside = car.0 > INSIDE_X_MAX_MM;
    let in_time =
        report.clock_ms.saturating_sub(report.start_ms) <= LIMIT_MS && report.clocks_match;
    at_car && near && outside && in_time
}

fn judge(report: &EpisodeReport) -> bool {
    judge_at(report, report.car, report.custody)
}

fn extremes(report: &EpisodeReport) -> (i32, bool) {
    let mut max_abs = 0i32;
    let mut saw = false;
    for sample in &report.trace {
        for value in [
            sample.robot_x,
            sample.robot_y,
            sample.operator_x,
            sample.operator_y,
        ] {
            max_abs = max_abs.max(value.saturating_abs());
            if value == 21_400 || value == -21_400 {
                saw = true;
            }
        }
    }
    (max_abs, saw)
}

fn run_slot(slot: &Slot) -> Row {
    let report = execute_probe(slot.probe);
    let (max_abs, saw_21400) = extremes(&report);
    Row {
        id: slot.id,
        covers: slot.covers,
        expect: slot.expect,
        basis: slot.basis,
        truth_ok: judge(&report),
        main_passed: report.main_passed,
        phase: report.phase,
        fail: report.fail,
        custody: report.custody,
        clock_ms: report.clock_ms,
        crowd: report.crowd,
        max_abs,
        saw_21400,
    }
}

fn agrees(row: &Row) -> bool {
    match row.expect {
        Expect::TruthPass => row.truth_ok,
        Expect::TruthFail => !row.truth_ok,
        Expect::Record => true,
    }
}

fn line(row: &Row) -> String {
    format!(
        "{} {} expect={:?} basis={:?} agree={} truth={} main={} phase={} fail={} custody={} clock={} crowd={} max_abs={} saw21400={}",
        row.id,
        row.covers,
        row.expect,
        row.basis,
        agrees(row),
        row.truth_ok,
        row.main_passed,
        row.phase,
        row.fail.unwrap_or("-"),
        row.custody,
        row.clock_ms,
        row.crowd,
        row.max_abs,
        row.saw_21400,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_is_sixteen_and_prior_claims_stay_locked() {
        let slots = slots();
        assert_eq!(slots.len(), 16, "denominator");
        let ids: Vec<_> = slots.iter().map(|slot| slot.id).collect();
        assert_eq!(
            ids,
            [
                "S01", "S02", "S03", "S04", "S05", "S06", "S07", "S08", "S09", "S10", "S11", "S12",
                "S13", "S14", "S15", "S16"
            ]
        );
        let rows: Vec<_> = slots.iter().map(run_slot).collect();
        for row in &rows {
            println!("{}", line(row));
            if row.basis == Basis::Prior {
                assert!(agrees(row), "prior claim moved: {}", line(row));
            }
        }
        assert!(rows.iter().any(|row| row.covers.contains("crowd 4")));
        assert!(rows.iter().any(|row| row.covers.contains("crowd 3")));
        assert_eq!(rows.iter().find(|row| row.id == "S01").unwrap().crowd, 4);
        assert_eq!(rows.iter().find(|row| row.id == "S13").unwrap().crowd, 3);
    }

    #[test]
    fn replay_matches_and_obstacle_order_does_not_change_the_truth_outcome() {
        let first = execute_probe(Probe::main(1));
        let second = execute_probe(Probe::main(1));
        assert_eq!(first.trace.len(), second.trace.len());
        for (left, right) in first.trace.iter().zip(second.trace.iter()) {
            assert_eq!(
                (
                    left.ms,
                    left.robot_x,
                    left.robot_y,
                    left.operator_x,
                    left.operator_y
                ),
                (
                    right.ms,
                    right.robot_x,
                    right.robot_y,
                    right.operator_x,
                    right.operator_y
                )
            );
        }
        assert_eq!(judge(&first), judge(&second));
        let reversed = execute_probe(Probe {
            reverse_obstacles: true,
            ..Probe::main(1)
        });
        println!(
            "MORPH truth_normal={} truth_reversed={} trace_same={}",
            judge(&first),
            judge(&reversed),
            first.trace.len() == reversed.trace.len()
                && first
                    .trace
                    .iter()
                    .zip(reversed.trace.iter())
                    .all(|(left, right)| left.ms == right.ms
                        && left.robot_x == right.robot_x
                        && left.robot_y == right.robot_y)
        );
        assert_eq!(judge(&first), judge(&reversed));
    }

    #[test]
    fn held_out_seeds_are_not_the_development_seeds() {
        assert!(
            HELD_OUT_SEEDS
                .iter()
                .all(|seed| !DEVELOPMENT_SEEDS.contains(seed))
        );
        for seed in HELD_OUT_SEEDS {
            let report = execute_probe(Probe::main(seed));
            let (max_abs, saw) = extremes(&report);
            println!(
                "HELD seed={seed} truth={} main={} phase={} fail={} custody={} clock={} crowd={} max_abs={max_abs} saw21400={saw}",
                judge(&report),
                report.main_passed,
                report.phase,
                report.fail.unwrap_or("-"),
                report.custody,
                report.clock_ms,
                report.crowd,
            );
        }
    }

    #[test]
    fn the_judge_rejects_a_wrong_car_an_early_cue_and_a_false_custody() {
        let ok = execute_probe(Probe::main(1));
        assert!(judge(&ok));
        assert!(!judge_at(&ok, (0, 0), ok.custody));
        assert!(!judge_at(&ok, ok.car, "gripper"));
        let early = execute_probe(Probe::from_script(Script::EarlyCue { seed: 1 }));
        assert_ne!(early.operator, early.car);
        assert!(!judge(&early));
        assert!(early.trip_flag);
        assert!(!early.main_passed);
    }
}
