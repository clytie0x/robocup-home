# WORKLOG

## R4 manifest, expectations locked before the matrix (2026-11-07)

R3 is merged at `61d694fd9523eae64796daef2d69dc7e67d4460e`. The sixteen rows are `S01` through `S16` in `crates/harness/src/manifest.rs`. `Basis::Prior` repeats an outcome already measured. `Basis::Hypothesis` is not edited after the run. `Expect::Record` does not claim task success.

| id | covers | expect | basis | seed |
| --- | --- | --- | --- | --- |
| S01 | normal, delay 0, crowd 4 | truth pass | prior | 1 |
| S02 | wrong-place cue | truth fail | prior | 1 |
| S03 | delay 200 | truth pass | prior | 1 |
| S04 | delay 1000 | truth pass | prior | 1 |
| S05 | missing delivery | truth pass | prior | 1 |
| S06 | reversed stamp | truth pass | hypothesis | 1 |
| S07 | 8 mm position error | truth pass | prior | 1 |
| S08 | causal loss, stay away | truth fail | prior | 1 |
| S09 | blocked approach, sensor omit | truth fail | prior | 1 |
| S10 | out-of-reach grasp | truth fail | prior | 1 |
| S11 | blocked handoff | truth fail | prior | 1 |
| S12 | actual drop | truth fail | prior | 1 |
| S13 | crowd 3 | truth pass | hypothesis | 2 |
| S14 | missing obstacle | truth pass | hypothesis | 1 |
| S15 | unmassaged geometry | record only | hypothesis | 1 |
| S16 | missing person | truth fail | prior | 1 |

Held-out seeds are 8 and 11. Development seeds are 1 and 2. The judge reads car, custody, robot-to-operator distance, and the clock. It does not read the script name or the trip flag. A wrong car, an early cue, and a false custody string must fail that judge. Replay of seed 1 must match. Reversing the obstacle list before spacing is not a controller permutation, because spacing sorts by x. The operator route does not clip a person radius. Obstacle spacing is a massage. `S15` skips that massage. A coordinate of 21400, if it appears, is reported and is not removed by moving the world.

Observed after that lock. Expectations were not edited. All 16 rows ran. All 12 prior claims agreed. The three hypotheses also agreed and stay hypotheses. `S15` was record-only: the world passed, clock 19800, maximum absolute coordinate 8000, `saw21400` false.

| id | truth | main | phase | fail | custody | clock | crowd | max abs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| S01 | pass | pass | complete | - | operator | 14800 | 4 | 8000 |
| S02 | fail | fail | complete | not at the car | operator | 1600 | 4 | 1000 |
| S03 | pass | pass | complete | - | operator | 15200 | 4 | 8000 |
| S04 | pass | pass | complete | - | operator | 21000 | 4 | 8000 |
| S05 | pass | pass | complete | - | operator | 15200 | 4 | 8000 |
| S06 | pass | pass | complete | - | operator | 14800 | 4 | 8000 |
| S07 | pass | pass | complete | - | operator | 14800 | 4 | 8005 |
| S08 | fail | fail | lost | lost | gripper | 7600 | 4 | 5600 |
| S09 | fail | fail | picking | stuck | free | 2000 | 4 | 1000 |
| S10 | fail | fail | picking | reach | free | 600 | 4 | 1000 |
| S11 | fail | fail | releasing | blocked-handoff | gripper | 14600 | 4 | 8000 |
| S12 | fail | fail | dropped | dropped | free | 1200 | 4 | 1000 |
| S13 | pass | pass | complete | - | operator | 13400 | 3 | 8000 |
| S14 | pass | pass | complete | - | operator | 14800 | 4 | 8000 |
| S15 | pass | pass | complete | - | operator | 19800 | 4 | 8000 |
| S16 | fail | fail | lost | unseen | gripper | 2400 | 4 | 8000 |

Nine of the sixteen were truth-pass. Seven were truth-fail. That split is not a field score. Held-out seed 8 was truth-pass, crowd 3, clock 14600. Held-out seed 11 was truth-pass, crowd 4, clock 14200. Neither is required to pass. Seed 1 replay matched. Reversing the obstacle list kept the same truth outcome and the same robot trace. The wrong-car, false-custody, and early-cue checks all failed the judge. No row showed coordinate 21400. The operator route still does not test a person radius against obstacles.

The obstacle reversal before `space_obstacles` is not the permutation test. That sort puts the disks back into x order, so the posted bag ids stay `[3, 4]`. The controller permutation reverses people, bodies, and bags on the posted view after geometry and the profile shift. Seed 1 shows bag ids `[3, 4]` against `[4, 3]`, people `2,12,13` against `13,12,2`, and bodies `(2600, 0, 350), (5600, 300, 200), (5600, 900, 200)` against `(5600, 900, 200), (5600, 300, 200), (2600, 0, 350)`. Positions, gesture, and the pointing ray stay the same. The selector still names bag 4. The truth judge stays pass. The full trace stays the frozen 75 lines. Equal-score steps now break ties by the candidate point, not by list index, so a later tie does not depend on order. `S06`, `S13`, and `S14` remain hypotheses and do not gate the test. Their agree flag is still printed. `S15` stays record-only.

This run's library count is 191: domain 8, harness 69, hri 2, manipulation 8, navigation 19, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests stay 7. Debug and release match. Section 3.1 stays 45 `pass` and 11 `not-run`. The 3.2 checker stays 42 `not-run`, 6 open, 2 hardware. `pdfinfo` and `pdftotext` are absent. The plain checker prints `pdf=False`. With `PROFESSOR_PDF` set, it prints `pdf=True` for hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600` and size 1129757. Page text was not read.

## R3 causal motion design (2026-11-07)

R2 is merged at `f7bd274c004d3e8020f1ead05bf0a2c233dfdbfc`. R3 adds causal cases on the existing episode. It does not rewrite the route engine, the mailbox, or the zero-delay trace.

The loss proof is a walk of at most 200 mm per sim step, with the trip clock advanced on each step. The operator is not moved by a single far `place_operator`. `Script::Loss` walks out and back, so the existing recovery can still finish. A stay-away walk is the cause. The same seed without that walk remains the followable control. The normal route is unchanged.

An approach disk is a world body. `try_slide` keeps using every body. Hiding that disk from the planner does not hide it from the slide. The legacy `collision_fault` stays a labeled diagnostic omission. It is not this milestone's physics, and it is not a drop.

An out-of-reach pick leaves the bag free. A disk on the handoff segment leaves the bag in the gripper. `drop_held` is the custody change. `grasp_fault` and `handoff_fault` stay injected diagnostics. No collision creates a drop.

The report's geometry, custody, clock, and trace are the outcome. The script name is not.

Red. Before the walk, `one_clock_covers_the_episode` exited 101: `operator jump -456,2370 at 1600`.

Green. Seed 1, measured. A stay-away walk leaves the sensor in steps of at most 200 mm, phase `lost`, custody `gripper`, clock 7600, robot `(-56, 370)`, operator `(400, 5600)`, `main_passed` false. The same seed without the walk completes, custody `operator`. `Script::Loss` walks out beyond the sensor and back, and `main_passed` stays true. A far bag with no disk completes at clock 15400, robot `(7997, -488)`. The same disk hidden from the planner stops the base at `(0, 0)`, phase `picking`, custody `free`, fail `stuck`, clock 2000. The same disk left visible is avoided: `main_passed` true, clock 15600, robot `(7999, -461)`. Moving the bag out of reach before the grasp leaves custody `free`, phase `picking`, fail `reach`, clock 600. `Script::Drop` records `Dropped` and custody `free`. `Script::BlockedHandoff` leaves custody `gripper` and phase `releasing`. The zero profile still matches the frozen 75-line trace.

This run's library count is 186: domain 8, harness 64, hri 2, manipulation 8, navigation 19, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests stay 7. Debug and release match. Section 3.1 stays 45 `pass` and 11 `not-run`. The 3.2 checker stays 42 `not-run`, 6 open, 2 hardware. `pdfinfo` and `pdftotext` are absent. The plain checker prints `pdf=False`. With `PROFESSOR_PDF` set, it prints `pdf=True` for hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600` and size 1129757. Page text was not read.

## R2 review fix, live base as the motion origin (2026-11-07)

Reviewer `bc-0faa` on `e0b7e9c` closed the three earlier blockers and found one new one. A delayed sample was still accepted, then the step was planned from the captured base. At seed 1, delay 200 ms, clock 1600, the captured base was `(0, 200)`, the live base was `(-102, 171)`, and the planned target was `(-89, 378)`. The live distance squared was 43018, above `200² = 40000`, so the episode stopped with `NotACommand`. The same people and empty body list planned from the live base give `(-169, 359)`, distance squared 39833. Delay 1000 ms stopped at clock 6400: captured base `(153, 358)`, live base `(244, 289)`, target `(294, 498)`, distance squared 46181. From the live base the step is `(367, 446)`, distance squared 39778. One missed delivery stopped at the same 1600 ms step.

The motion origin is now the current base. People, bodies, bags, the pointing ray, and the cue stay on the posted sample. The controller does not read a fresh exteroceptive list or the car for that step. A step longer than 200 mm asks for a fresh sample, advances the clock, and does not keep the sample.

Red. `cargo test -p harness --lib a_delayed_sample_steps_from_the_current_base --offline` exited 101: `left Err(NotACommand) right Ok(Moved { to: (-169, 359) })`. The episode profiles stopped with fail `command`.

Green. That step moves to `(-169, 359)` and the 1000 ms geometry moves to `(367, 446)`. Seed 1 after the change, measured, not required in advance: delay 200 ms completes, phase `complete`, clock 15200, custody `operator`, robot `(7997, -488)`, operator `(8000, -1500)`, 77 trace lines. Delay 1000 ms completes at clock 21000, robot `(7999, -628)`, 106 lines. One missed delivery completes at clock 15200, robot `(7997, -488)`, 77 lines. Shift `+8, -8` mm completes at clock 14800, robot `(8005, -495)`, operator still `(8000, -1500)`, 75 lines. The zero profile still matches the frozen 75-line trace. There was no remaining task-level failure on these four runs.

This run's library count is 185: domain 8, harness 63, hri 2, manipulation 8, navigation 19, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests stay 7. Debug and release match. Section 3.1 stays 45 `pass` and 11 `not-run`. The 3.2 checker stays 42 `not-run`, 6 open, 2 hardware. `pdfinfo` and `pdftotext` are absent. The plain checker prints `pdf=False`. With `PROFESSOR_PDF` set, it prints `pdf=True` for hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600` and size 1129757.

## R2 review fixes, decision age and episode delivery (2026-11-07)

Reviewer `bc-0faa` on `fc784b9` found three blockers. The library count 179 and CI were green and did not catch them.

Age is `decision_ms - captured_ms`. Delivery latency is logged and is not the stale bound. The same posted sample stays actionable through a decision age of 1000 ms and is stale at 1001 ms. A second presentation of a capture already in history is a duplicate: `NeedFresh`, history unchanged. A newer capture recovers. `step_drive` and `accompany` write history only after the command returns `Ok`. The episode keeps one mailbox. The shutter is the only control-path `sense`. It stores the pointing ray with that capture. `shift_view` and the delay happen before selection, cue, follow, and approach. Delay 0 and shift 0 stay on the frozen 75-line trace. A missing or not-yet-due sample advances the sim clock one step and does not pause the 300000 ms budget.

Red. With the delivery-latency age restored, `cargo test -p navigation --lib the_same_posted_sample_goes_stale_at_the_decision_clock --offline` exited 101: decision 1000 still reported age `Some(0)`. With history written before command validation, `cargo test -p harness --lib a_rejected_command_does_not_keep_the_sample_history --offline` exited 101: `left Some(400) right None`.

Green. The same posted capture, delivery still 0, is `NeedFresh` at decisions 200 through 1400 and at 100000. The base stays on the first step. A newer capture moves it. A step that is too long for the current base, a wrong person, a follow during `Picking`, and accompany of the wrong person after `Complete` leave history, pose, and clock unchanged. The zero profile matches `r1_main_trace.txt`.

Episode outcomes from `a_delayed_episode_uses_the_mailbox_and_can_miss_once`, seed 1, measured after the mailbox was wired. These are the world results, not a required pass. Delay 200 ms: `main_passed` false, phase `following`, clock 1600, custody `gripper`, fail `command`. Delay 1000 ms: `main_passed` false, phase `following`, clock 6400, custody `gripper`, fail `command`. One missed delivery: `main_passed` false, phase `following`, clock 1600, custody `gripper`, fail `command`. The fail is `NotACommand`: a delayed view still inside the 1000 ms decision age planned a step the current base could not take. Shift 8 mm and -8 mm per axis: `main_passed` true, phase `complete`, clock 14800, custody `operator`. The sensed operator differed from truth by that shift. Truth was not shifted.

This run's library count is 184: domain 8, harness 62, hri 2, manipulation 8, navigation 19, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests stay 7. Debug and release match. Section 3.1 stays 45 `pass` and 11 `not-run`. The 3.2 checker stays 42 `not-run`, 6 open, 2 hardware. `pdfinfo` and `pdftotext` are absent, so the plain checker prints `pdf=False` and page text was not read. With `PROFESSOR_PDF` pointed at the attached file, the checker prints `pdf=True`: hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`, size 1129757.

## R1 red-log clarification (2026-11-07)

Independent review re-ran the two R1 halo tests on `3c6cab12`. The harness test failed at the bag-membership assertion. The writer log that shows `unwrap_err()` on `Ok(Moved { to: (0, 200) })` is from a build where that membership assertion was not yet in front of the approach call. Both failures are the same leak. They are not the same assertion line.

## R2 sensor delivery (2026-11-07)

R1 is merged at `8228d76c8d770216059941b5ce3c92ef5effb866`. This branch implements R2 only.

### Design

The controller no longer calls `sense` inside `step_drive`. It receives a `Posted` value. `accept` checks that value against `SampleHistory` and the sim clock at the decision. `Missing` stays missing. A capture older than the last accepted capture, or a capture or delivery later than that clock, is rejected and does not change pose, custody, or history. Age above 1000 ms is stale: the base stays and the fresh world is not read. Ages 0, 200, and 1000 ms are the approved delay profiles and may still move the base from the captured pose. The episode budget uses the trip stamp, not `captured_ms`.

`shift_view` adds at most 8 mm to sensed people, bodies, and bags. The zero shift is identity. The shift does not take the world, so truth and `world.pick` stay on the real pose. 8 mm is an assumption named `synthetic-unmeasured-8mm`. It is not a measurement.

Rejected shapes. Replaying the last pose whenever a sample is missing would chase that pose without a bound. Reading `observe` inside `follow` on a gap would put fresh truth back on the controller path. A seeded random offset would look like a calibration.

The R1 main trace for seed 1 was frozen before these edits: 75 samples, clock 14800, 67 follow moves, longest step 200. The zero-delay path matches that trace. No pose field was dropped. Capture and delivery stamps are a separate log.

Red. `cargo test -p navigation --lib a_reversed_sample_keeps_history_and_a_missing_sample_is_not_truth --offline` exited 101 because `Posted` and `accept` were not defined. `cargo test -p harness --lib a_held_sample_is_followed_after_the_operator_moves --offline` exited 101 with the base at `(200, 0)` after the operator moved to `(1500, 0)`. The held sample still had the operator at `(0, 1000)`.

Green. That follow step stays on x = 0 and moves in y. A 200 ms sample and a 1000 ms sample do the same, with ages 200 and 1000. Approach toward a held bag at `(0, 3000)` still moves in y after the world bag is placed at `(4000, 0)`. A missing sample and a 1001 ms sample return `NeedFresh` and leave the base at the origin. A reversed capture leaves pose, custody, history, and then advances the clock to 500. A missing sample at 300000 ms stays out of `TimeUp`; the next stamp at 300001 ms is `TimeUp` with the base still at the origin. `shift_view` by 8 mm does not change `observe`, and `world.pick` still uses the true pose. `the_zero_profile_matches_the_r1_main_trace` passes. The R1 halo and fixed-car tests pass.

This run's library count is 179: domain 8, harness 58, hri 2, manipulation 8, navigation 18, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests stay 7. Debug and release match. Section 3.1 stays 45 `pass` and 11 `not-run`. `pdfinfo` and `pdftotext` are absent. The attached PDF hash and size still match.

## R1 review fix, unseen-operator bag halo (2026-11-07)

Reviewer `bc-0faa` reproduced a blocker on `3c6cab12`. Robot `(0, 0)`, operator `(0, 20000)`, visible free bag `(0, 19000)`, other bag `(50000, 50000)`, gesture offered. People were empty and the cue was `None`, but the view still contained bag 3 at `(0, 19000)`. `point_from_sample` could select it, and `step_drive` Approach moved the base to `(0, 200)` at clock 100 in `Picking`. Bag `(0, 18000)` was inside that halo and `(0, 17999)` was outside. Moving the unseen operator to `(20000, 0)` dropped the bag. The 2 m disk was tracking an unseen true person.

The printed 2 m rule is the setup disk. It applies only while the operator is inside the robot sensor. `executor_guard` is the world-truth check for a command the trip already named. It is not the selector and it does not move the base. `world.pick` still returns `OutOfReach` for that far bag.

Red. `cargo test -p navigation --lib an_unseen_operator_does_not_pull_a_nearby_bag_into_view --offline` exited 101: `assertion failed: view.bags.is_empty()`. `cargo test -p harness --lib an_unseen_operator_halo_is_not_selected_or_chased --offline` exited 101: `unwrap_err()` on `Ok(Moved { to: Point { x: Mm(0), y: Mm(200) } })`.

Green. Those two tests pass. The halo bag is absent, `point_from_sample` errs, Approach returns `GripError::Missing`, and the base stays at `(0, 0)` with clock 0. `world.pick` on that same bag returns `ArmError::OutOfReach`. `a_fixed_gesture_keeps_the_plan_when_the_car_moves` compares `step_drive` for `Cue::None` and `Cue::TakeBag` after `set_car`. `hidden_far_missing_and_role_ids_leave_the_world_and_the_trip` and `a_legal_bag_behind_the_operator_is_reached_by_moving_the_base` still pass.

This run's library count is 168: domain 8, harness 50, hri 2, manipulation 8, navigation 15, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests stay 7. Debug and release match. Section 3.1 stays 45 `pass` and 11 `not-run`. `pdfinfo` and `pdftotext` are absent. The attached PDF hash and size still match.

R1 through R5 stay authorized in order. R1 is active until this review gate passes. Later milestones do not need a new owner approval. They still stop for review. No contact drop is added.

## R1 sensor boundary, approved plan recorded (2026-11-07)

Teerapat approved the revised five-milestone plan at 2026-11-07 00:36 Asia/Bangkok, including the replay picture, and said to follow the plan. The record is [docs/tasks/r1-r5-realism-plan.md](docs/tasks/r1-r5-realism-plan.md). This pull request implements R1 only. It does not merge and it does not start R2. There is no sanctioned Issue tool in this run, so no Issue was opened.

### Design

The controller type is `navigation::SensorView`. It has robot pose, in-range people, in-range bodies, the bags the view may name, one gesture bit, and `captured_ms`. It has no car field. `sense` does not take a car. The scenario still decides when the human offers the gesture, and that decision may use the car. `cue_from` reads only the view.

World `try_slide`, `pick`, and `hand_to` keep using truth for collisions, reach, and custody. `collision_fault` stays a legacy omission on the obstacle slice the harness passes into `sense` for planning. The sensor function itself does not drop a class. `decide` for a named pick still classifies invisible, out of reach, and missing on the world bag, so those existing refusals stay distinct.

A visible bag enters the view when it is inside `SENSOR_MM` of the robot. The printed 2 m disk is added only when that operator is already inside the robot sensor. That keeps the legal far-bag approach and does not follow an unseen person. A bag outside both, and an obstacle disk outside the class sensor, stay in the world and stay out of the view. `executor_guard` classifies a command the trip already named against world truth. Follow, approach, and selection do not call it.

Rejected shapes. A car field zeroed inside the same snapshot still types the leak. Filtering `observe` would hide truth from the scorer. A new crate adds a package the workspace does not need.

### Throughput

Blocking step: record this plan, then a failing behavioral test, then the sensor wiring. One writer. `episode.rs` and `drive.rs` are shared, so the work is one pull request. R2 is a later review, not a parallel branch.

### Red and green

Red, before `sense` existed. `cargo test -p harness --lib an_unsensed_extreme_bag_does_not_move_the_base --offline` exited 101. The approach still returned `Ok(Moved { to: Point { x: Mm(200), y: Mm(0) } })` for a bag at x = 2968220. `cargo test -p navigation --lib an_offered_gesture_outside_range_is_lost_and_not_a_cue --offline` exited 101 with `cannot find function sense` and `cannot find function cue_from`.

Green. Those tests, plus `a_bag_outside_the_robot_sensor_and_the_two_meter_disk_is_absent`, `a_fixed_gesture_keeps_the_plan_when_the_car_moves`, `an_unseen_disk_stays_in_the_world_and_still_blocks`, and `an_in_range_gesture_at_the_car_passes_and_a_wrong_place_cue_does_not`, passed. The numeric cap for a huge approach coordinate stays in `approach_toward_a_far_bag_stays_on_axis_and_inside_the_step`. The old drive test was chasing that coordinate from world truth. It now expects `GripError::Missing` and an unmoved base.

The section 3.1 spec file gained one sentence that points at this plan. Its row results were not edited. `artifacts/check_m10_spec.py` pins that file, so the pin moved from `b88ea111e77f639c3fe589c10874c222ba087b3c07eb3cf462154ba14411bc06` to `6325c732a0c7819e43ba9d65d801eb147316364b5afe701d43f7b93dd7a49633`.

This run's library count is 166, not the earlier 161. The added tests are navigation 2 and harness 3. Doctests stay 7. Debug and release match. `check_m1_spec.py` reports 56 rows. `check_m10_spec.py` reports 42 `not-run`, 6 open, 2 hardware. The section 3.1 table was not rewritten, so its 45 `pass` and 11 `not-run` labels are unchanged. `pdfinfo` and `pdftotext` are absent here. `PROFESSOR_PDF` still matched the hash and the byte size, and the checker printed `pdf=True` on that basis. It did not run a page-count tool in this run.

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 49, hri 2, manipulation 8, navigation 14, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, task_3_2 1, harness 0, hri 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same library counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

python3 artifacts/check_workspace.py
ok members=9 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
pdfinfo and pdftotext absent; hash and size matched

python3 artifacts/check_m10_spec.py --self-test
self-test ok

python3 artifacts/check_m10_spec.py
ok rows=42 not-run=42 open=6 hardware=2
```

## M10 scaffold for section 3.2 (2026-11-06)

Tasks 1 and 2 of the approved plan only. Same pull request as the research. No command body, score, deadline, navigation, door hardware, or section 3.1 change. This does not merge. No later milestone was started.

### Independent review

Reviewer `bc-2385` read head `62b84c2bd8cf115b483cdae9c65539b974658112`. No verified blocker. The scaffold was cleared to proceed. The reviewer checked the section 3.2 excerpt at 85 whitespace-normalized lines and found `artifacts/sources/SHA256SUMS` valid. The section 3.1 `pdfinfo` fix was needed: the base raised `CalledProcessError` on a 9-byte file. The reviewer ran that self-test with and without poppler. Missing and unreadable paths still fail. This slice keeps that fix as it was.

The reviewer's in-memory mutations of the inventory still passed the checker at that head: the outstanding sheet cell 150 changed to 999, the absence prose 250 changed to 500, and the sentence "Both are in this PDF." removed. Those three now fail `artifacts/check_m10_spec.py`. The anchors are the printed sheet cell and the excerpt token 150, the general excerpt sentence "it gets a penalty of 250 points.", the prose row, and the open absence row. CI runs `--self-test` and the plain checker after the section 3.1 commands.

### Default-operator sentence

Printed page 9, physical page 14, section 2.2.3 item 1: "The default operator may repeat the command up to three times." The same item says ASR with the default operator has no score reduction. `GPSR-GEN-REPEAT` records that sentence. Its result is `not-run`. The acceptance text describes a later repeat count. The door scaffold does not speak a command and does not credit the row. `GPSR-PEN-ASR` still records the sheet value 3×–50 and the section 2.2.3 bands of 20 percent, 30 percent, and up to 30 percent. Printed page 3 gives the contrary test rule higher priority. The row does not compute one combined penalty.

The inventory is 42 rows, all `not-run`, 6 open, 2 hardware. Section 3.1 stays 45 `pass` and 11 `not-run`. There is no field score and no awarded total.

### Red and green

Task 1 red. A compiling `apply` ignored the event, returned `Ok` with effect `None`, set `AtPoint`, and stored stamp 1. `cargo test -p task_3_2 --offline` exited 101. Three library tests failed. The door test was `left: None`, `right: Some(GoToPoint)`. Arrival before the door was `left: Ok(Outcome { effect: None })`, `right: Err(OutOfOrder)`. The doctest failed the same `GoToPoint` assertion. That was the behavior red. A missing crate was not the claim.

Task 1 green. `DoorOpen` from `Outside` emits `GoToPoint` and moves to `GoingToPoint`. The effect carries no coordinate. `Arrived` before the door returns `OutOfOrder` and leaves the phase and the stamp unchanged. `cargo test -p task_3_2 --locked --offline` then passed 3 library tests and 1 doctest.

Task 2 wiring is the harness dependency, the workspace edge, and `mod gpsr`. It is what lets the test compile. The behavior red, before `Tick` stored a stamp, was `cargo test -p harness --lib door_open_uses_the_sim_clock --offline` exiting 101: `called Result::unwrap() on an Err value: OutOfOrder` at `Event::Tick`. `sim_runtime::Clock::advance(Step::from_ms(25))` had already made `clock.now_ms()` 25. The green stores that clock value. The passing assertion is `visit.now_ms() == Some(clock.now_ms())` and `clock.now_ms() == 25`. The phase stays `GoingToPoint`. There is no second clock and no literal 25 stored in the machine.

### This tree

Library tests are 161: domain 8, harness 46, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3. Doctests are 7: the previous 6 plus one in `task_3_2`. Debug and release match. `SimWorld::from_seed` is unchanged. The GPSR rows stay `not-run`. A door-state enum does not credit a section 3.2 condition. Known limits that remain: the command grammar, the open score rows, the unenforced 5 minute limit, and the physical door. This is not a completed General Purpose Service Robot run, and it does not say that no defect remains.

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 46, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 30, task_3_1 32, task_3_2 3
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, task_3_2 1, harness 0, hri 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=9 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
pdfinfo and pdftotext present

python3 artifacts/check_m10_spec.py --self-test
self-test ok

python3 artifacts/check_m10_spec.py
ok rows=42 not-run=42 open=6 hardware=2
```

## M10 section 3.2 research (2026-11-06)

Design and specification only. Branch `cursor/m10-next-task-research-88f6` from merge `d6d9940820e92c7b8b8e5c290a4a45b703b90a11`. One writer. No crate, workflow, Cargo manifest, or hardware change. The scaffold in the plan was not started. This does not merge. The section 3.1 acceptance table stays 45 `pass` and 11 `not-run`. There is no awarded total.

No sanctioned issue tool is available in this run. The pull-request tool opens a pull request. The GitHub CLI is read-only. No issue was opened by another route.

### Choice

The next task is section 3.2 General Purpose Service Robot. The contents and the Stage I contents on printed page 13 place it immediately after section 3.1. Printed pages 18, 19, and 20 are physical pages 23, 24, and 25. The main goal is to execute 3 commands. The printed cycle that can be tested without inventing a command is: start outside, move to the Instruction Point when the arena door is open, then return after the first and second command. The maximum time is 5 minutes.

That cycle can reuse `sim_runtime::Clock` from `harness`. The command list is not in the PDF. The footnote prints `https://github.com/johaq/CommandGenerator`. That live repository was not read and is not the baseline. GPSR-OPEN-CMD stays open.

3.3 Receptionist starts on printed page 21, physical page 26. 3.4 Serve Breakfast starts on printed page 24, physical page 29. 3.5 Storing Groceries starts on printed page 27, physical page 32. Each states a 5 minute maximum and its own score sheet. They wait. 3.4 also starts at a door, then requires a bowl, a spoon, a cereal box, and a milk carton. That is a new object catalog. 3.2's first transition does not need one.

### Source

The professor PDF is the rule. Cover 2024 Rev-1, revision 2024-07-09. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`, 1129757 bytes, 52 pages. `pdfinfo` 24.02.0 reports CreationDate Tue Jul 9 14:56:18 2024 UTC. Public 2024.2 and public 2025.1 are not the baseline. The raw PDF was not committed.

`poppler-utils` 24.02.0 was installed in this environment so the pages could be read. It is not a project dependency. `pdftotext -layout` produced [artifacts/sources/professor-3.2.txt](artifacts/sources/professor-3.2.txt). On physical page 25 the score token `3×–400` has the same right edge, xMax 517.0, as the other score tokens. Plain text joins it to the word "penalty" with no space. The label continues on the next line. The inventory keeps `3×–400` as the score-column token.

The positive factors are `3×400` and `3×100`. The products 1200 and 300 are arithmetic. Their sum is the printed total 1500. The sheet prints 150 on the outstanding line. One tenth of 1500 is 150. Section 2.3.3 does not print that multiplication, and the Technical Committee decides whether to award any of it.

### Boundary

Three shapes were compared in [the design](docs/tasks/m10-3.2-design.md). The recommended one is a typed `task_3_2` crate that depends on `domain` only, plus a private harness test that advances `sim_runtime::Clock`. A copied clock inside the task crate would miss the reuse proof. A task trait in `domain` would either edit `task_3_1::Trip` or sit unused. `SimWorld::from_seed` stays the section 3.1 layout. The first machine does not call it.

The inventory is [docs/tasks/3.2-general-purpose-service-robot.md](docs/tasks/3.2-general-purpose-service-robot.md): 41 rows, all `not-run`, 6 open, 2 hardware. The plan is [docs/tasks/m10-3.2-plan.md](docs/tasks/m10-3.2-plan.md). Task 1 is the next implementation step: `DoorOpen` emits `GoToPoint`. Task 2 is the clock import. Both stop before a command body, a score, or a 5 minute clamp. The clamp waits until GPSR-ASM-CLOCK names the arming event.

Open rows: GPSR-OPEN-OUT, GPSR-OPEN-CMD, GPSR-OPEN-RETURN, GPSR-OPEN-HELP, GPSR-OPEN-ABSENT, GPSR-OPEN-START. Assumptions that may later carry a label: GPSR-ASM-RETRY, GPSR-ASM-PAUSE, GPSR-ASM-CLOCK, GPSR-ASM-PLACE.

### Checker

`artifacts/check_m10_spec.py` checks the 41 rows, the open and hardware ids, the printed phrases, and the section 3.1 file hash `b88ea111e77f639c3fe589c10874c222ba087b3c07eb3cf462154ba14411bc06`. CI does not run it. A `pass` result fails the local self-test.

With `pdfinfo` present, `check_pdf` used to raise `CalledProcessError` on the non-PDF self-test fixture. A failed `pdfinfo` or `pdftotext` now adds `professor PDF pdfinfo failed` or `professor PDF pdftotext failed` and still reports the hash and the size. The exact file still prints `pdf=True`, including the 52-page check and the section 3.1 text check. Unset `PROFESSOR_PDF` prints `pdf=False`. A missing requested path prints `professor PDF missing` and exits 1.

### This tree

Section 3.1 is unchanged: 45 `pass`, 11 `not-run`. The 11 are `CML-BAG-DEF`, the seven open scoring rows (`CML-GEN-MAIN`, `CML-SHEET-START`, `CML-OPEN-PARTIAL`, `CML-OPEN-ABSENT`, `CML-OPEN-CAP`, `CML-OPEN-DROP`, `CML-OPEN-FOLLOW`), and the three hardware rows (`CML-HW-OUT`, `CML-HW-HIT`, `CML-HW-SPL`). Library tests remain 157. Doctests remain 6. Debug and release match. This is not full acceptance, and it does not say that no defect remains.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 45, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 30, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, harness 0, hri 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False
unset_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
pdfinfo and pdftotext present
exact_exit:0

python3 artifacts/check_m10_spec.py --self-test
self-test ok
m10_self_test_exit:0

python3 artifacts/check_m10_spec.py
ok rows=41 not-run=41 open=6 hardware=2
m10_exit:0
```

## M9.3 interrogate verdict (2026-11-06)

Documentation only. No production code changed after `2e8619159ccafc17f1dd53e52cf53c67f34b9484`. This does not merge and does not start M10. The acceptance table stays 45 `pass` and 11 `not-run`. There is no awarded total. No verified finding remains from this audit. That is not full acceptance, and it does not say that no defect remains.

### Intent

A reusable laptop simulation faithful to professor task 3.1, not a scripted success report. M9 may remove only justified duplication. Paths, negative evidence, the local-only navigator, the shared episode clock, honest acceptance limits, and rerunnable verification stay. Green CI and a row id in a test body are not task acceptance. The 45 `pass` rows and 11 `not-run` rows stay model, catalog, or labeled-assumption evidence. The professor PDF is the rule. Cover 2024 Rev-1, revision 2024-07-09. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2024.2 and public 2025.1 are not the baseline. The raw PDF was not committed.

### Findings

Audited head `298104e381a4fc101394446bd4e3e0fa581a3cb2`. Fix head `2e8619159ccafc17f1dd53e52cf53c67f34b9484`. Base `b446ae9b59cd9543c1c321dcbd461f2dc1586de9`.

One finding was fixed. On `298104e`, `PROFESSOR_PDF` set to a missing path printed `ok rows=56 pdf=True` and exited 0. An independent rerun reproduced that. On `2e861915`, the same command prints `professor PDF missing` and exits 1. Unset and empty print `ok rows=56 pdf=False` and exit 0. The exact file, and a byte copy, print `pdf=True` and exit 0. A 9-byte file, a directory, and a mode `000` file exit 1 with no `pdf=True`. `--self-test` prints `self-test ok`. `crates/harness` and `crates/sim_runtime` are unchanged in that commit. The fix note is the section below.

One finding was dropped. A SmallGround disk at `(2000, i32::MAX - 50)` with radius 80 makes `center + grown + 40` equal `2147483967`, and `saturating_add` clamps the pass segment to `i32::MAX`. The independent rerun of real `route_to` reported `sat arithmetic=2147483967 pass_y=2147483647 dy=50 grown=330 covers=true n_pass=14` and `ordinary pass_y=1370 dy=370 grown=330 covers=true inside=true`, with `1 passed` and `45 filtered out`. The measurement is real. It does not break the M9 intent. The M8 walker already used `saturating_add(grown).saturating_add(40)`. This branch only names that 40. `episode.rs` is identical on `2e861915`. The seeded margin is the labeled assumption. A disk within 50 mm of `i32::MAX` is not a Carry My Luggage route and is not an acceptance-row claim. The probe kept going on saturation and reported the seeded paths unchanged. This is not a denial of the clamp.

### Counts

Clock and claims, `grok-4.7-xhigh-fast`: 1 proposed, 0 dropped. That one is the PDF flag fixed on `2e861915`. The inverted `CML-TASK` sentence and the 632 mm assist gap were not filed. The first is the accepted id-presence limit of the checker. The second matches the M8 base.

Fault and route, `grok-4.7-xhigh`: 1 proposed, 1 dropped, for the saturated pass segment above.

Two proposals. One fixed. One dropped. None remain verified.

### Limits

`~/.cursor/rules/pstack-models.mdc` is absent. `claude-opus-5-5-xhigh` and `claude-sonnet-5-5-xhigh` were rejected because they are not included in weekly usage. The seats that ran were `grok-4.7-xhigh` for fault, live hit, and route, and `grok-4.7-xhigh-fast` for clock, custody, source, and verification. Both were separate worktrees on exact head `298104e`.

The clock seat ran the workspace tests, 157 library tests and 6 doctests, debug and release, plus the checker with and without the PDF. Its causal probe reported the shared clock, custody negatives, the local navigator, and the live hit cases holding. The coordinator did not re-execute those passes except the PDF cases.

The fault seat's full include-suite was not re-executed by the coordinator. It is not adopted as proof. The coordinator reran the saturated and ordinary `route_to` cases only. The fault report says the shared fault predicate, the drive list, the live remove, move, reclass, and inject cases, and the car at `(2501, 1000)` passed, and that 29 script traces match the base. Those statements stay in that report. They were not filed as findings.

The auditor did not have `pdfinfo` or `pdftotext`, so that pass did not check page count or page text. Hash and size still ran. CI does not set `PROFESSOR_PDF`. CI run `42238785579` succeeded at `298104e` with 157 library tests, 6 doctests, and `pdf=False`: https://github.com/clytie0x/robocup-home/actions/runs/42238785579. CI run `42243746431` succeeded at `2e861915`. Its checkout merge `284cba6b40c811c829b8b4ba42543dec8d824908` has the same tree. The log has `self-test ok`, `ok rows=56 pdf=False`, and the same test counts: https://github.com/clytie0x/robocup-home/actions/runs/42243746431.

The fault worktree still had uncommitted `include!` lines. They were not committed.

### This tree

`command -v pdfinfo` and `command -v pdftotext` are absent here too, so this pass did not run the page or text checks. No package was installed to add them. The attached file is 1129757 bytes and its SHA-256 is `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. With that path set, the checker prints `ok rows=56 pdf=True` from the hash and the size. Unset, it prints `ok rows=56 pdf=False`.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 45, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 30, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, harness 0, hri 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False
unset_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
hash_size_only_exit:0
```

## M9.3 missing-PDF flag (2026-11-06)

Scope. One verified finding from the M9.3 audit, rerun on `298104e381a4fc101394446bd4e3e0fa581a3cb2`. The rest of that audit is still in progress. This does not close M9.3, does not merge, and does not start M10. The acceptance table stays 45 `pass` and 11 `not-run`. No episode, route, or custody code changed.

Finding. `env PROFESSOR_PDF=/tmp/m93-audit/verifier/no-such.pdf python3 artifacts/check_m1_spec.py` printed `ok rows=56 pdf=True` and exited 0. The path did not exist, and the fallback upload `at-home-robocup-my-2025-1_763a.pdf` did not exist either. The same command on the M8 checker at `b446ae9b59cd9543c1c321dcbd461f2dc1586de9` did the same. The checker file was unchanged. The new README said `pdf=True` only for the exact file. Unset `PROFESSOR_PDF` printed `pdf=False` and exited 0. The exact file printed `pdf=True` and exited 0. A 9-byte file exited 1 with a hash mismatch and `professor PDF size 9`.

Cause. `main` printed `pdf={professor_pdf_path() is not None}`. A set path is not `None` even when the file is absent, so the flag was true before any hash. `collect` hashed a path only when it existed, and it reported `professor PDF missing` only when its `pdf` argument was passed. The CLI does not pass that argument, so a missing requested file produced no problem and a true flag.

Red. `python3 artifacts/check_m1_spec.py --self-test` exited 1. The missing-path case returned `code=0 out='ok rows=56 pdf=True\n'`. A directory raised `IsADirectoryError`. An unreadable file raised `PermissionError`.

Green. A requested path is admitted only after it is opened. A missing requested path prints `professor PDF missing` and exits 1. A directory prints `professor PDF is not a file` and exits 1. An unreadable file prints `professor PDF unreadable` and exits 1. `pdf=True` is printed only when the opened bytes match the pinned hash. Unset `PROFESSOR_PDF` still prints `pdf=False` and exits 0 when the fallback file is absent. A set path does not use that fallback. An empty value stays unset. A fallback that is not a file stays unset. A fallback file that is present is hashed, and a wrong file does not verify. The self-test covers those cases, including the exact attached PDF when a file with hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600` is visible. CI still does not require the PDF.

```text
python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False
unset_exit:0

env PROFESSOR_PDF=/tmp/m93-audit/verifier/no-such.pdf python3 artifacts/check_m1_spec.py
professor PDF missing
missing_exit:1

PROFESSOR_PDF=<attached copy, hash da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
exact_exit:0

PROFESSOR_PDF=<9-byte file> python3 artifacts/check_m1_spec.py
professor PDF hash 55e6967dae43099b5d6822df808b06e30454a2be93475a3d82ba612d8dd3bf29
professor PDF size 9
wrong_exit:1

rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 45, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 30, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, harness 0, hri 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0
```

## M9 simplify and verify (2026-11-06)

Scope. Issue 18, parts M9.1 and M9.2 only. Branch `cursor/m9-simplify-verify-2b9e` from merge `b446ae9b59cd9543c1c321dcbd461f2dc1586de9`. One writer. Laptop simulation. No hardware, deploy, registration, contact, spending, new dependency, or deletion. M9.3 was not run and is not claimed. M10 was not started. This does not say that no defect remains, and it is not full acceptance.

### M8 review history

The first full-run note below recorded 46 `sim` rows as `pass`. That count treated the four avoidance lines as pass when `avoided` was `present && !hit`. A disk with x greater than 2500 counted as present. The operator lane cleared the disks, so the robot did not face the four classes. Seeds 0, 1, 2, 3, 7, and `u64::MAX` left the hard-to-see disk outside the 800 mm sensor, and a car at `(2501, 1000)` still received all four credits. That claim was too wide.

The obstacle-credit review replaced that test with an abreast sight, the class sensor, and the close band, and it required the class to remain for a collision. `CML-BAG-DEF` was downgraded to `not-run`: the fixture stores `BagKind` and does not measure stiffness or weight. That note still described a hit on the layout copy after the live disk was removed.

The live-hit review moved `note_step` and `note_encounter` onto `observe().obstacles`. A removed class no longer sets a hit. An explicit collision fault keeps the class, omits it from the guard and the follower, and the hit is the live disk.

Independent verification of merged head `63931d905f14a411dcad221cee3e80bb5563c10a` is CI run `42236672054`, conclusion success: https://github.com/clytie0x/robocup-home/actions/runs/42236672054. That run is 154 library tests and 6 doctests, debug and release. The acceptance table on that head is 45 `pass` and 11 `not-run`. The 11 are `CML-BAG-DEF`, the seven open scoring rows (`CML-GEN-MAIN`, `CML-SHEET-START`, `CML-OPEN-PARTIAL`, `CML-OPEN-ABSENT`, `CML-OPEN-CAP`, `CML-OPEN-DROP`, `CML-OPEN-FOLLOW`), and the three hardware rows (`CML-HW-OUT`, `CML-HW-HIT`, `CML-HW-SPL`). That is model, catalog, and labeled-assumption evidence. It is not a field score and not a claim that no defect remains.

### M9.1

No new rulebook sentence. The professor PDF remains the rule. Cover 2024 Rev-1, revision 2024-07-09. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2024.2 and public 2025.1 are not the baseline. The raw PDF was not committed. Prior art for the follower is unchanged and is not a dependency.

The slide guard and the follower sight each omitted one collision-fault class. `SimWorld::obstacles_for_planner` is now that omission, and `try_slide` uses the same predicate. The default is still `None`. A set class stays until another call changes it. `observe` still contains the class. `note_step` still reads the live disks. The drive path calls the world method. Seed 1, then two replacement disks: with no fault the planner list matches `observe` and a slide through the small disk is blocked; with `SmallGround` the planner list matches the old filter, the small disk stays in `observe`, the slide crosses it, and the crowd disk still blocks. Clearing the fault blocks the small disk again.

The return walk and the marked route walk used two copies of the same step loop. `extend` now calls `extend_mark`. A direct comparison of both, from three starts to nine goals including an empty path, matches. A marked goal still carries its class tag.

`ENCOUNTER_WINDOW_MM` is 340, `CLOSE_BAND_MM` is 280, and `ROUTE_PASS_MM` is 40. The encounter check and the report use those names. The report gained the line `assumption route-pass-mm 40`. The robot path is unchanged. 341 mm is outside the abreast window. One millimeter past the close band is outside it. These lengths are assumptions, not official measurements. The printed avoidance lines stay 50. That catalog value is not this distance proof.

`main_passed` still requires the operator on the designated car, pointed-bag custody, local proximity, a locally seen cue, steps of at most 200 mm, and one clock at or under 300000 ms. It does not read the trip flag. An ordinary collision clears the avoidance class. It does not by itself fail the main carry. A major collision stays the separate hardware finding `CML-HW-HIT`.

### M9.2

The acceptance table is still 45 `pass` and 11 `not-run`. The checker was not loosened and was not changed to require the PDF. Without `PROFESSOR_PDF` it prints `pdf=False`. With the attached file it prints `pdf=True`. A `pass` row still only has to name a test whose body contains the row id.

The episode set is `cargo test -p harness --lib --locked --offline`. Those 45 harness tests ran inside the workspace command below, including every `Script` variant. Seed 1 is the main carry. The new summary label uses `Script::Overflow`, seed 0, start `u64::MAX - 40`.

Limits that remain. `execute_pick` can still pair world time 400000 ms with trip stamp 10. That helper is outside the episode. The episode uses one clock. Direct `try_slide` can still take a long clear segment. The episode marks a jump and does not pass. The navigator still receives no car and, with no fault, no class filter. The sensor ranges 800 and 2500, the disk radii, the 250 mm robot disk, the arena half-plane, and the three-step stay are assumptions. CI does not have the professor PDF.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked --offline
domain 8, harness 45, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 30, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked --offline
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, harness 0, hri 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release --offline
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release --offline
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False
spec_exit:0

PROFESSOR_PDF=<attached copy, hash da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_pdf_exit:0
```

The `--offline` flag is local. CI does not pass it. The spec self-test, the checker without the PDF, the checker with the attached PDF, and the workspace check were repeated after this note and the README edit. Each exited 0 with the same lines as above. M9.3 remains mandatory before milestone closure.

## M8 live hit geometry (2026-11-06)

Scope. Narrow review of `58d8745e2c5e5fbad59fd4b81b9760e90d23f008`. The abreast encounter credit held. The residual bug was `note_step`: it tested the disks copied when the route was built, not the bodies in the world at the step. No merge. M9 was not started. M9.3 was not run.

Red. `cargo test -p harness --lib obstacle_credit_requires_a_real_encounter --offline` failed after asserting a strip has no hits. Seed 1 with every live obstacle removed still reported `hit [false, true, true, true]`: hard, crowd, and barrier. Those were crossings of the frozen copy. `Script::Collide` deleted the class and expected that phantom hit.

Green. The hit test reads `observe().obstacles` at the step. The encounter test reads that same live set. The copied layout field is gone. A strip now has `hit [false, false, false, false]`. Dropping one class clears its credit and does not set its hit flag. Moving the hard disk 20000 mm off the path, or reclassing it as small, leaves the hard hit and the hard sight clear. An injected small disk on the live path is the disk the small hit meets. The collision negative keeps the class in the world, turns on `set_collision_fault` so the guard and the follower skip it, and routes the operator through that live center. The hit is that crossing, not contact with a deleted object. A car at `(2501, 1000)` has all four avoidance credits false.

45 rows stay `pass`. 11 stay `not-run`. The checker was not loosened.

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 43, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 29, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

## M8 obstacle credit (2026-11-06)

Scope. Independent review of `b6158a266c0f049b1ae990cd801d460ebec5011b` on pull request 17. The follow to the car, custody, the shared clock, and the negative trip-flag cases held. The merge blocker was the avoidance mark. No merge. M9 was not started. M9.3 was not run. This does not say that no defect remains.

`avoided` was `present && !hit`. `present` was true for any disk with x greater than 2500. The operator lane cleared the tallest disk, so the robot did not face the four classes. On seeds 0, 1, 2, 3, 7, and `u64::MAX` the hard-to-see disk stayed outside the 800 mm sensor. A car at `(2501, 1000)` still received all four credits. `set_obstacles` of an empty list after setup still passed `seeded_main_reaches_the_car`. The crowd credit had the same shape. `CML-ASM-ORDER` compared the placement vector, not the order the robot faced.

Red. Credit then required an abreast local sight. `cargo test -p harness --lib seeded_main_reaches_the_car --offline` failed: `main_passed false`, `fail budget`, `encounter []`, `seen [false, false, false, false]`, robot `(4084, 737)`, operator `(4850, 540)`, clock `300000`.

Green. The seeded class order is kept. The episode spaces those disks along the outside path so a local follower can pass each one. The pitch, the 340 mm abreast window, the class sensors, and the 280 mm band outside the grown disk are assumptions, not printed measurements. A class is credited only when the robot is abreast, the disk is in that class sensor, the robot is in the close band, and no motion segment hits the grown disk. The navigator still receives only `look`. Removing every obstacle after the route is built clears all four credits. Removing one class clears that class. Driving through a class whose live disk was removed records a hit on the layout disk and clears that class. A car at `(2501, 1000)` does not receive the four credits.

Other rows from the same review. `CML-BAG-DEF` is `not-run`: the fixture stores the bag kind and does not measure stiffness or weight. `CML-SET-PLACE` fails when the armed pose is outside and when the robot moves after `Begin`. `CML-SET-BAGS` accepts a bag at exactly 2000 mm and rejects 2001 mm. `CML-SCORE-SEE` is one mock confirmation for either a screen or speech, not a measured pair of channels. `CML-GEN-LIST` is a person taking the free bag while the gripper stays empty. `CML-GEN-BONUS` is a non-navigator slide during the return; the bonus lines stay 0. `CML-ASM-ORDER` is the order the robot faced, labeled as an assumption, and it is not a claim that the PDF requires one order.

Clock. A follow stamp at 300001 stores `TimeUp`. `Accompany` at 300001 after completion returns `TimeUp`, leaves the phase `Complete`, and does not move the world. The runner stopping because the next stamp would pass 300000, while the phase is still `Following`, is `budget`, not `TimeUp`. Direct `execute_pick` can still pair world time 400000 ms with trip stamp 10. That path is the old helper, not the episode.

`check_m1_spec.py` only checks that a `pass` row names a test function whose body contains the row id. It does not judge the meaning of the assertions. 45 rows are `pass`. 11 stay `not-run`: the seven open scoring rows, `CML-BAG-DEF`, and the three hardware rows. The sheet still does not award a run total.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 43, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 29, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

## M8 full run to the hidden car (2026-11-06)

Scope. Issue 16. One writer. Branch `cursor/m8-full-run-ffe7` from `4bf872ca88bfa2378cab3ca7d5acf4286561b3f7`. The episode follows the operator to the designated car and can re-enter with the same crowd. The car coordinate is scenario data. The navigator does not receive it. No hardware, deploy, registration, contact, or spending. M9 was not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. This does not say that no defect remains.

### M7 review

Reviewed head `0b4bed962f2a1a182e32b88edd1be3362ff40daf`. CI run `42222234917` succeeded: https://github.com/clytie0x/robocup-home/actions/runs/42222234917. That run is 143 library tests and 6 doctests, debug and release. The physical handoff guard refuses a world disk on the full transfer segment, including the hard-to-see disk at `(0, 1060)` with radius 60 between `(0, 200)` and `(0, 1400)`. No verified production blocker remained on that head. The component cue is not rulebook arrival: `TakeBag`, both poses leaving the first sight, and a clear observed segment can set `main_complete` after one follow step while the car stays at `(8000, -1500)`. That flag is not a score.

### Sources

The PDF is the rule. The other links are prior art for a local follower. They are not dependencies and they are not a substitute rulebook.

- Professor PDF section 3.1, printed pages 15–17, physical pages 20–22. Cover 2024 Rev-1, revision 2024-07-09. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. No public URL is assigned to that file. Public 2024.2 and public 2025.1 are not the baseline. The raw PDF was not committed.
- The robot reaches the car only by following the operator. The place is unknown before the test. Printed p.16.
- Local sensing with unknown obstacles: https://doi.org/10.1109/tac.1986.1104175
- Bug algorithms use local knowledge rather than a preloaded map: https://www.cs.columbia.edu/~allen/F19/NOTES/Chap2-Bug.pdf
- A standoff behind a moving person, as a pattern only: https://docs.nav2.org/jazzy/tutorials/general_tutorials/navigation2_dynamic_point_following/navigation2_dynamic_point_following/

### Decisions

`harness::run` is the episode. The scenario reads the hidden car and walks the operator on a lane that clears world disks. `robot_follow` plans from `look` and a cue. It has no car argument. `revealed_car` stays empty. Seeds, lane offsets, and car points vary so a single coordinate cannot be the success condition. The operator stops on the designated point. A stop 101 mm short does not pass. There is no minimum travel quota.

`main_passed` checks the operator point, pointed-bag custody, robot proximity, a locally observed cue, steps of at most 200 mm, and one clock at or under 300000 ms. It does not read `main_complete`. An early cue still sets that flag and fails the check. A direct long `try_slide`, a homeward-only trip, a wrong person, an unseen operator, a wrong bag, a drop, a cancel, a sealed robot, a blocked handoff, and a late stamp fail it. Loss and refind can still reach the car. Whether the 300 point follow line remains after that loss stays undecided.

The optional return follows a crowd person through the labeled entrance and stays three steps with the same people. The main trip can finish without it. Help on the return pays 0 on the bonus lines. A queue of different ids does not count. The sheet records printed lines and returns `Undecided` for the seven open conflicts. It does not award a run total.

Report, confirm, approach, pick, ready, follow, handoff, and return share one stamp. `Begin` is the origin. Stamp 300000 can move. Stamp 300001 stores `TimeUp` and does not move. After `Complete`, a later stamp does not rewrite the phase. A blocked step consumes the stamp. Overflow at `u64::MAX - 40` does not wrap. The old pick helpers can still pair a world time of 400000 ms with trip stamp 10. The episode does not.

The component `plan_follow` cue is unchanged and is documented as a fixture. The full runner cannot use it as arrival. Direct `try_slide` can still take a long clear segment. The episode marks that as a jump and does not pass. Collision checks in the simulator use world disks. The planner receives the local sight only. Radii, ranges, the 200 mm step, `ARRIVE_MM`, the entrance, and the stay are assumptions. The shorter hard-to-see range is not a contrast measurement. The operator is not a collision disk.

### Tests

Red. The first episode build failed: `Drive::Accompany` was not covered (`E0004`), and the report closures borrowed `self` while `commit` borrowed it mutably (`E0502`). After that compiled, a probe of `Script::Unseen` returned `fail=Some("setup")` because the operator was moved onto the car before the 2 m bag count. `main_passed` was false for that setup reason, not because the cue was unseen. The bag check now runs first. The same probe showed `Script::EarlyCue` with `trip_flag=true`, `main_passed=false`, and the operator at `(0, 1400)` while the car stayed at `(8000, -1500)`.

Green on this tree. Debug and release match. Unit mapping: [docs/tasks/m8-unit-coverage.md](docs/tasks/m8-unit-coverage.md). Acceptance: 46 `sim` rows `pass`, 7 open rows `not-run`, 3 hardware rows `not-run`. A model pass is not full acceptance and not a hardware result.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 41, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 29, task_3_1 32
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

The doc command was run again after a private rustdoc link was removed. That second run exited 0. The test counts are from the suite above. M9.3 remains mandatory before milestone closure.

## M7 physical handoff guard (2026-11-06)

Scope. Re-review of `bf211cbd4f899548b8d4247f742c6e24fdfd35db` kept the overflow and cancel fixes. One blocker remained: a hard-to-see disk outside the 800 mm sight still let the bag cross the world segment. Same pull request 15. No merge. M8 was not started. M9.3 was not run. The 56 acceptance rows stay `not-run`. The seven open scoring rows stay open.

Red. `hand_to` returned `HandoffFailed` for a disk between clear ends while the fault flag was set. The review case is robot `(0, 200)`, operator `(0, 1400)`, hard-to-see disk radius 60 at `(0, 1060)`. Endpoints sit outside the grown radius 310. The segment meets the disk. `cargo test -p sim_runtime --lib a_handoff_rejects_a_disk --offline` failed: `left: Err(HandoffFailed)` `right: Err(Blocked)`.

Green. `hand_to` checks that segment against every world disk after custody and before an injected fault. The radius argument is the same 250 mm disk a slide uses. The navigator is not given the hidden disk. A refusal leaves custody, pose, the trace, the fault flag, the clock, and the trip unchanged. `execute_handoff` calls that guard, so a direct call cannot skip it. A stamp after 300000 ms still stores `TimeUp` and does not transfer the bag. A tangent at exactly the grown radius is allowed. One millimeter closer is refused. Disk order does not matter. A disk at `(0, 500000)` on a long segment is refused. An empty gripper is still `Empty` when a disk is present.

The component cue is unchanged. Both leaving the first-sight poses, plus a clear observed segment, plus `TakeBag`, can set `main_complete` after handoff. One follow step to `(0, 200)` and an operator at `(0, 1001)` can do that while the car stays at `(8000, -1500)`. That flag is not rulebook arrival and not a score. M8 is mandatory: the scenario must reach the unknown car by following the operator, and the destination observation must not pass the car coordinate to the navigator. No distance quota is invented to stand in for that. M9.3 remains mandatory. This does not say that no defect remains.

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 35, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 28, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

## M7 review fixes (2026-11-06)

Scope. Independent review of `818a52e0fa764a0012072ba5f1baf19157038833` reproduced three blockers. Same pull request 15. No merge. M8 was not started. M9.3 was not run. The 56 acceptance rows stay `not-run`. The seven open scoring rows stay open. This note does not say that no defect remains.

### Heading overflow

`headed_step` squared an `i64` heading. Debug panicked. Release wrapped and could step off axis, report blocked on a clear line, or propose more than 200 mm. `plan_approach` is reachable from the harness with any bag coordinate, so the sensor range did not hide it.

Red. `cargo test -p navigation --lib approach_toward_a_far_bag --offline` panicked: `attempt to multiply with overflow` at the heading square.

Green. The step is scaled with `i128` and a ceiling length, so the Euclidean length is at most the requested cap. A candidate longer than 200 mm is dropped. The harness refuses such a proposal without moving the base or the clock. Bags at x = 2968220, 2968221, 3000000, and 5000000 step on +x by at most 200 mm in debug and release. The extreme follow pose stays capped. Two hundred random goals stay within 1 mm of the independent line. `try_slide` can still take a long clear segment when called directly. That is unchanged.

### Cancel admission

Cancel returned before the shared-clock and deadline checks. A following cancel at 300001 was `Ok(Cancelled)` with the world clock left at 0. A split of world 400000 ms and trip 10 also cancelled.

Every drive, including cancel, now admits the stamp first. Split, regression, and the same stamp change nothing and do not set the flag. A stamp after 300000 ms stores `TimeUp` and does not cancel. An in-budget cancel consumes that stamp, sets the follower cancelled, leaves the phase, and does not move the base. A later follow while cancelled stays cancelled and still consumes its stamp.

### Arrival

A take-bag cue inside 1200 mm arrived at the start, and again with a disk between the robot and the operator. Handoff then set `main_complete` while the car stayed hidden. That treated the cue as proof of arrival.

The cue remains an HRI signal, not a car coordinate. Arrival also requires three observed facts: the base has left the pose where following started, the followed person has left the pose where they were first seen, and the segment to that person is clear of observed disks. `ARRIVE_MM` 1200 stays an assumption. A cue at the start does not arrive and handoff does not complete. A cue across a disk after a real step does not arrive. After the operator leads the base off those start poses and the segment is clear, the cue can release and a later handoff can finish the trip state. The navigator still does not read the car. Lost, refind, the wrong person, a far person, and a missing person stay as they were.

### Checks

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 34, hri 2, manipulation 8, navigation 12, perception 20, sim_runtime 27, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

Still in force from the review, and not closed here. `segment_within` matched an independent reference on the structured set and on random samples. A tangent at exact radius is outside the open disk. Northern seeded routes can follow without a swept hit. A hard-to-see disk whose center is outside 800 mm is not in the sight, so the follower can stick instead of walking around it. The pick path can still pair world time 400000 ms with trip stamp 10. The operator is not a collision disk. `task_3_1` still does not depend on `sim_runtime`. None of this is a claim that no defect remains.

## M7 observed follow (2026-11-06)

Scope. Issue 14. One writer. Branch `cursor/m7-follow-nav-9321` from `8422451965925b43976a245df6b77f98714c6498`. The robot follows the observed operator through local disks. The base moves. A held bag moves with it. The car coordinate is not an input. No scorer, optional return, queue, hardware, deploy, registration, contact, or spending. M8 was not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. Every acceptance row stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 and public 2024.2 are not the baseline. The raw PDF was not committed.

### M6 review

Reviewed head `1009b66909483cbabe8b566065d3e074ea73ce47` had no verified production blocker. That check was 112 library tests and 5 doctests, debug and release, with a reach probe of 20000 points, 20000 trips of 40 steps, and world custody plus timeout and fault atomicity. CI run `42211202059` succeeded: https://github.com/clytie0x/robocup-home/actions/runs/42211202059. The wording commit `82740a2d0b4274c4c49a6194d09cf33158ea2f32` is 113 library tests and 5 doctests. CI run `42213551511` succeeded on that head: https://github.com/clytie0x/robocup-home/actions/runs/42213551511. Those commits are in the merge `8422451965925b43976a245df6b77f98714c6498`. This review does not say that no defect remains.

### Sources

The PDF is the rule. The other links are prior art for a local follower. They are not dependencies, and they are not a substitute rulebook.

- Professor PDF section 3.1, printed pages 15–17, physical pages 20–22. Cover 2024 Rev-1, revision 2024-07-09. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. No public URL is assigned to that file.
- Local sensing with unknown obstacles: https://doi.org/10.1109/tac.1986.1104175
- Bug algorithms use local knowledge rather than a preloaded map: https://www.cs.columbia.edu/~allen/F19/NOTES/Chap2-Bug.pdf
- A standoff behind a moving person, as a pattern only: https://docs.nav2.org/jazzy/tutorials/general_tutorials/navigation2_dynamic_point_following/navigation2_dynamic_point_following/

### Decisions

`navigation` plans one step from people and disks in sensor range. The straight step is at most 200 mm. If that step hits a disk, the next candidate is a tangent, and the side stays until the straight step is clear. Sixteen headings cover the case where the tangent is blocked. The plan is not a route and not a teleport. `look` drops a hard-to-see disk beyond 800 mm and keeps other disks out to 2500 mm. Both ranges are assumptions.

`sim_runtime` stores one disk for the small object, the hard-to-see object, and the barrier area. The crowd is 3 or 4 person disks, from `seed % 2`, spaced 600 mm apart. `try_slide` rejects a segment that meets a grown disk and otherwise moves the base. A held bag moves with the base. A free bag stays. Operator custody moves only when the fixture places the operator. That placement is not given to the planner. `revealed_car` stays empty.

`step_drive` refuses a split clock and a backward stamp without writing either side. The same stamp does not move. Stamp 300000 can move. Stamp 300001 stores `TimeUp` and does not move. A blocked step advances both clocks and leaves the pose. Eight blocked steps report stuck. Cancel, loss, a wrong person, and a finished trip do not arrive. Re-finding the same id returns to `Following` and does not arrive on that step. Arrival is `Cue::TakeBag` while that person is inside 1200 mm. The trip then emits `Release`. The handoff is a later call. `REACH_MM` stays 2500. A bag at `(0, 3000)` with the operator at `(0, 1000)` is approached by the base and then picked.

The pick path can still pair a world time of 400000 ms with a trip stamp of 10. The navigation path cannot.

### Tests

Red, before the behavior. `segment_within` returned false for every ordinary radius. `cargo test -p domain --lib segment_cannot_tunnel --offline` failed: `assertion failed: segment_within(from, to, center, 1)` for the segment from `(-1000, 0)` to `(1000, 0)` through a disk at the origin. `plan_follow` returned `Arrived` for every sight. `cargo test -p navigation --lib a_follow_plan_does_not_arrive_without_the_cue --offline` failed: `left: Arrived` `right: Arrived`.

Green on this tree. Debug and release match. They do not close an acceptance row. Unit mapping: [docs/tasks/m7-unit-coverage.md](docs/tasks/m7-unit-coverage.md).

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 8, harness 31, hri 2, manipulation 8, navigation 9, perception 20, sim_runtime 27, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, navigation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

Limits. The radii, the 200 mm step, the sensor ranges, and the 1200 mm cue radius are not printed. The hard-to-see disk is nearer in range, not dimmer. The crowd is disks, not a social model. Arrival is the cue plus proximity, so this does not score the 300 point follow line and does not decide `CML-OPEN-FOLLOW`. The pick path still does not share the navigation clock. No claim is made that every defect has been found.

## M6 review wording (2026-11-06)

Scope. Pull request 13, reviewed head `1009b66909483cbabe8b566065d3e074ea73ce47`. One factual sentence. No reach change, no clock rewrite, no algorithm change. M7 was not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. Every acceptance row stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 is not the baseline. The raw PDF was not committed.

Independent read-only review of that head reported no verified production blocker. Its full checks were 112 library tests and 5 doctests, debug and release, spec `pdf=True`, and CI run `42211202059` success: https://github.com/clytie0x/robocup-home/actions/runs/42211202059. It also reported a reach reference of 20000 points, 20000 trips of 40 steps, world custody, and timeout and fault atomicity, and those probes held. This follow-up did not repeat that probe. The review does not say that no defect remains.

The sentence in `manipulation` and in the M6 decision said a bag at the printed 2 m from the operator is inside the 2500 mm disk. That is true for the seeded horizontal placements. It is not true for the whole printed disk. The operator is at `(0, 1000)` and a bag at `(0, 3000)` is 2000 mm from the operator and 3000 mm from the robot at the origin. `decide` and `execute_pick` already return out of reach and leave the world unchanged. `REACH_MM` stays 2500. It is not a rulebook length. Reaching the rest of that disk means moving the base in M7.

The same review noted that the world clock and the trip stamp can differ. A world time of 400000 ms and a pick at trip stamp 10 is valid here, because the deadline reads the trip stamp. The clocks stay separate. They are not a wall clock. Joining them is M7 and M8 work.

`a_legal_two_meter_placement_behind_the_operator_is_outside_the_seeded_reach` passed on the existing decision before the wording edit. No earlier test used operator `(0, 1000)` and bag `(0, 3000)`.

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 7, harness 21, hri 2, manipulation 8, navigation 0, perception 20, sim_runtime 25, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
same counts, 0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
same doctest counts, 0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

## M6 simulated gripper (2026-11-06)

Scope. Issue 12. One writer. Branch `cursor/m6-sim-arm-37d5` from `c70a7b3b5fefab82f592671b7dbd145377ce2ea0`. A manipulation decision and a simulated single gripper for pick, carry, drop, and handoff, wired through the existing trip. No motion planner, camera, navigation, optional return, scorer, hardware, deploy, registration, contact, or spending. M7 and M8 were not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. Every acceptance row stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 is not the baseline. The raw PDF was not committed.

### M5 independent re-review

Reviewed `ae255f82c4f58650ad999c4b36397ee42f482bb6`, which is an ancestor of the merge `c70a7b3`. CI run `42207373451` completed with conclusion success on that head: https://github.com/clytie0x/robocup-home/actions/runs/42207373451. That run is the debug and release suite for the merged M5 tree. This session counted 86 `#[test]` functions and 4 doctests in that tree. It did not repeat the full release suite locally.

`keep_deadline` stores `TimeUp` when `Trip::apply` returns `Ok` with no effect. The three deadline tests passed here. With that latch removed, `a_late_confirmation_does_not_name_and_keeps_time_up` and `a_late_report_keeps_time_up_and_an_earlier_stamp_cannot_repost` returned `Err(NotAReport)` instead of `Err(TimeUp)`. The latch was restored. The floor-corridor tests were not re-mutated in this session. No M5 production change was made. M9.3 was not run. This review does not say that no defect remains.

### Plan

`manipulation` depends only on `domain` and decides a pick, drop, or release. `sim_runtime` still depends only on `domain` and owns the one gripper, bag custody, and location. `task_3_1` is unchanged. `harness` gains a dependency on `manipulation` and is still the only crate that may see both the trip and the simulator. `Cargo.lock` adds that path edge only.

### Decisions

The PDF says the robot picks up the pointed bag, tells the operator it is ready, and the operator takes the bag back at the car. It states no arm length, grasp pose, drop point, or gripper count. Those numbers below are assumptions.

`REACH_MM` is 2500. It covers the seeded bags, which sit on the horizontal through the operator at `(0, 1000)` with `|x|` from 400 through 2000. It does not cover every bag inside the printed 2 m of that operator. A bag at `(0, 3000)` is 2000 mm from the operator and 3000 mm from the robot at `(0, 0)`, so it is outside this disk. Picking that placement means moving the base, which is M7 navigation. The base is not moved here. A held bag and a dropped bag are recorded at the robot base (`SITE` = `robot-base`). There is no gripper frame and no contact model. After handoff the bag remains in the world at the operator pose with operator custody. It is not deleted and it is not free.

The world clock and the trip stamp are separate counts. Neither is a wall clock. The five-minute check uses the trip stamp. A world time of 400000 ms does not make a pick at trip stamp 10 late. Coordinating the clocks is M7 and M8 work. This milestone does not rewrite either clock.

Custody is `Free`, `Gripper`, or `Operator`. At most one bag is `Gripper`, and that id is `carried`. A refusal does not change custody, pose, the trace, or the trip, except a late stamp, which stores `TimeUp` and does not apply the payload. An injected grasp or handoff fault is a flag on the world. It is checked after the geometric and role refusals, and it does not move the bag.

`decide` is not execution and not confirmation. `execute_pick` admits `Grasped` on a trip copy first. A deadline is stored and the world is left alone. A refusal discards that copy. A legal grasp then changes the world and only then stores the trip. `PickEnd::Matched` requires the adapter and the trip to hold that same bag. The other bag is `Mismatched` and is not success. `confirm_ready` is a later `MockReady` call. It does not follow and it does not move the base. `execute_handoff` does not apply `Arrived`. Tests that need `Releasing` apply that event themselves and say it is not navigation.

### Sources

Used for the shape. Not taken as dependencies. The PDDL actions are not this model's coordinates.

- Professor PDF section 3.1, printed pages 15–17. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. No public URL is assigned to that file.
- Gripper STRIPS: `free` and `carry` are exclusive, pick deletes `at`, drop restores `at` and `free`: https://www.ida.liu.se/~TDDD48/labs/2023/domains/strips/gripper.pddl
- The same pick and drop effects: https://www.cs.toronto.edu/~sheila/2542/w09/A1/introtopddl2.pdf
- Pick replaces `AtPose` with `AtGrasp` and clears `HandEmpty`: https://learning-and-intelligent-systems.github.io/kitchen-worlds/domain-operators/
- `i128` products, as in the M5 selector: https://doc.rust-lang.org/std/primitive.i128.html

### Tests

Red, before the behavior. The reach stub returned true. `reach_is_an_inclusive_disk_and_does_not_wrap` failed because a 1 mm offset at radius 0 was inside. The decide stub returned `Grasped` of id 1. `cargo test -p manipulation --lib --offline` was 8 failed and 0 passed. Representative assertions: `(2501, 0)` was `Ok(Grasped { bag: Id(1) })` instead of `Err(OutOfReach)`; a hidden bag was that same `Ok` instead of `Err(Invisible)`. The gripper stub set `carried` and still returned success. `cargo test -p sim_runtime --lib --offline` on the new names was 8 failed. Representative assertions: reach 0 was `Ok(())` instead of `Err(OutOfReach)`; the held list was `[]` instead of `[Id(3)]`; an empty drop was `Ok(Point { x: 0, y: 0 })` instead of `Err(Empty)`; a handoff to a bag id was `Ok(())` instead of `Err(Role)`. The ready stub stored id 1. Its test was 1 failed: `Some(Id(1))` instead of `Some(Id(2))`. The harness stubs returned a matched id 1, a follow of id 2, a drop at the origin, and a release of id 1. The new harness tests were 8 failed. Representative assertions: seed pickup was `Matched { bag: Id(1) }` instead of `Id(3)`; a hidden bag was that `Ok` instead of `Err(Arm(Invisible))`; an empty drop was `Ok(Point)` instead of `Err(Trip(Impossible))`; ready before a grasp was `Ok(Follow)` instead of `Err(Trip(OutOfOrder))`; handoff before arrival was `Ok(Released { bag: Id(1) })` instead of `Err(Trip(OutOfOrder))`.

Green on this tree. Debug and release match. They do not close an acceptance row. Unit mapping: [docs/tasks/m6-unit-coverage.md](docs/tasks/m6-unit-coverage.md).

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 7, harness 20, hri 2, manipulation 8, navigation 0, perception 20, sim_runtime 25, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, manipulation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
domain 7, harness 20, hri 2, manipulation 8, navigation 0, perception 20, sim_runtime 25, task_3_1 30
0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
domain 1, manipulation 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

The attached copy is the same baseline hash. `pdf=True` checks that hash, the 1129757 byte size, 52 pages, and the 2024-07-09 creation date.

After green, four scratches were restored. An unconditional `Grasped` failed the reach, hidden, and injected-failure tests: `(2501, 0)` was `Ok(Grasped { bag: Id(3) })` instead of `Err(OutOfReach)`. A pick that set `carried` and left custody free failed `each_fixture_bag_is_held_once_and_leaves_the_other_bag_in_place` with `[]` instead of `[Id(3)]`. A drop that did not clear `carried` failed `drop_clears_the_gripper_at_the_robot_and_a_second_drop_does_not` with `Some(Id(3))` instead of `None`. Returning the trip copy from `keep_deadline` without storing `TimeUp` made `deadlines_do_not_execute_a_physical_action` return `Err(NotACommand)` instead of `Err(TimeUp)`.

### Limits

The reach disk and the robot-base site are assumptions, not printed measurements and not a motion plan. The gripper does not simulate contact, weight, or handles. Follow and arrival are not implemented. A handoff test that applies `Arrived` is a fixture, not an M7 pass. A passing test does not mean an acceptance row passed. The open scoring rows are untouched. No statement here says the software has no remaining defects. M7, M8, and M9.3 were not started.

## M5 review fixes (2026-11-06)

Scope. Pull request 11, reviewed head `1e28b3024cdfe63df98f5e46a4f4a85a5c5a6b85`. Two harness defects. No selector defect was verified. No merge, and M6 was not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. Every acceptance row stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 is not the baseline. The raw PDF was not committed.

The reviewed head's CI run `42204708843` completed with conclusion success: https://github.com/clytie0x/robocup-home/actions/runs/42204708843. That run is not this follow-up.

Code commits before this note: `a17a499` and `abcf916`.

### Findings

`Trip::apply` stores `TimeUp` and returns `Ok` with no effect when the stamp is past 300000 ms. The payload is not applied. `confirm_with_mock` treated that `Ok` as a named bag. Seed 1, operator id 2, bag id 3, `Begin` at 0, `stage_report` at 1, `confirm_with_mock` at 300001 returned `Ok(Id(3))` with phase `TimeUp`, `named` false, and an empty gripper. A stamp at 300000 still names and moves to `Picking`.

`stage_report` dropped that same `Ok` because the effect was missing, and it returned `Impossible` without storing `TimeUp`. `Begin` at 10 and `stage_report` at 300011 left the trip at `AwaitPoint` with `now_ms` 10. A later `stage_report` at 60 then posted the bag. Direct `Trip::apply` of the late point stores `TimeUp`.

The floor corridor was already the implementation. Every committed direction had an integer length, so `cross² <= 150² * (dx² + dy²)` still passed those tests. Direction `(2, 1)` has floor length 2. Cross 299 is inside that tube and cross 301 is outside. The exact width accepts 301. Direction `(1, 1)` rejects the point `(0, 180)`, about 127 mm from the ray. Direction `(100, 100)` accepts that same point. The floor length is not scale-invariant. It is not a physical width.

### Contract

`Ok` from `stage_report` is `Effect::ReportBag` and the phase is `Naming`. `Ok` from `confirm_with_mock` means the trip named that bag: phase `Picking`, `named` true, and the machine effect is `Pick` for that id. `FeedError::TimeUp` means the deadline was stored, `now_ms` is the late stamp, and the payload was not applied. `FeedError::Trip` leaves the trip equal to the value before the call. A stamp at 300000 ms is inside the budget. One millisecond later is `TimeUp`. An earlier stamp after that is `Regressed` and does not restore the trip. A repeated call on `TimeUp` is `Stale` and leaves the stored deadline in place.

### Tests

Red, on `1e28b30`, before the wrapper change. `a_late_confirmation_does_not_name_and_keeps_time_up` got `Ok(Id(3))` instead of `Err(TimeUp)`. `a_late_report_keeps_time_up_and_an_earlier_stamp_cannot_repost` got `Err(Trip(Impossible))` instead of `Err(TimeUp)`. `a_start_near_u64_max_names_inside_the_budget_and_latches_just_outside_it` got `Ok(Id(3))` instead of `Err(TimeUp)`. The exact-width scratch, `cross * cross` compared with `150² * (dx² + dy²)`, failed the two new perception tests: direction `(1, 1)` returned `Ok(Id(5))` instead of `Err(NoHit)`, and cross 301 returned `Ok(Id(4))` instead of `Err(NoHit)`. The scratch was removed. The floor test was not replaced.

Green on the tree that contains `a17a499` and `abcf916`. Debug and release match. They do not close an acceptance row.

```text
cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 6, harness 12, hri 1, manipulation 0, navigation 0, perception 20, sim_runtime 17, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
domain 6, harness 12, hri 1, manipulation 0, navigation 0, perception 20, sim_runtime 17, task_3_1 30
0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
domain 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

### Limits

The deadline rule is the existing trip rule. The wrappers no longer disagree with it. The corridor is still the floor length, so a short direction is stricter than a longer writing of the same ray. A passing test does not mean an acceptance row passed. No statement here says the software has no remaining defects. M6, M7, M8, and M9.3 were not started.

## M5 pointed-bag selection (2026-11-06)

Scope. Issue 10. One writer. Branch `cursor/m5-pointed-bag-1f1e` from `44b502ddac19cce65952c2503bbb508b875b9704`. Perception selects one visible bag from a pointing sample. The harness posts that bag on the M4 trip and confirms the report through a mock. No grasp, ready signal, follow, handoff, navigation, scorer, hardware, deploy, registration, contact, or spending. M6, M7, and M8 were not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. Every acceptance row in the task specification stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 is not the baseline. The raw PDF was not committed.

Parent CI. Pull-request run `42202405233` completed with conclusion success on `5e39e7b09786a7ab8c8a05ce7fd07891deb5b385`: https://github.com/clytie0x/robocup-home/actions/runs/42202405233. The merge push run `42202531854` completed with conclusion success on `44b502ddac19cce65952c2503bbb508b875b9704`: https://github.com/clytie0x/robocup-home/actions/runs/42202531854. The review note under M4 still stands. This section does not replace it.

Code commits before this note: `a8c5b30`, `3848589`, `a3daca9`, `8e33f2e`, and `6e96d4e`.

### Plan

Keep the selector in `perception`, depending only on `domain`. The fixture in `sim_runtime` builds an operator-to-bag offset and does not put the bag id in that sample. `task_3_1` is unchanged and still does not depend on `sim_runtime`. `sim_runtime` does not depend on `perception` or `task_3_1`. The checker rejects dev-dependencies, so the wiring lives in a new member, `harness`, whose edges are `domain`, `hri`, `perception`, `sim_runtime`, and `task_3_1`. Nothing depends on `harness`. That is an added edge, not a dropped check.

### Decisions

The PDF says the operator stands in front of the robot and points at one bag, that at least two bags are within 2 m and visible, and that perceiving the correct bag is a screen image or a spoken identification. It states no angle, sensor, tie rule, hand point, or bag width. Those numbers below are assumptions.

The ray starts at the operator pose. This model has no finger, elbow, or head joint, so it does not use a head-hand or elbow-hand ray. The direction is an integer millimeter vector. The nominal fixture vector is the offset from the operator to the bag the fixture was asked to measure. No extra random noise is added. The millimeter grid is the quantization.

A candidate counts only when it is visible, the dot product with the direction is positive, and the perpendicular distance `|dx * py - dy * px| / hypot(dx, dy)` is at most 150 mm. The test uses `floor(hypot)`, so the tube is never wider than 150 mm. A bag at exactly 150 mm on an axis-aligned ray is a hit. One millimeter outside is not. A non-positive dot is behind or beside the operator and is not a hit. A zero direction is rejected before any bag is considered.

Two hits are unique only when the perpendicular gap is greater than 2 mm, compared with `ceil(hypot)`. A gap of 2 mm or less is `Ambiguous`. One millimeter of movement on each of two candidates can close a 2 mm gap, so the safe output is no bag. The same output is used when two bags sit on the ray or on the same point with different ids. The choice does not fall through to the nearer bag, the earlier candidate, or the smaller id. Duplicate ids are rejected before the geometry runs. A hidden bag is skipped and does not create a tie.

`direction_mm` subtracts in `i64` and returns overflow when a component does not fit in `i32`. It does not return one axis. `select` uses `i128` for the dot and cross products and `u128::isqrt` for the length. The tested extremes, including `i32::MIN` and `i32::MAX`, return a result and do not panic. Coordinates are already integer millimeters, so a NaN cannot enter this API.

`stage_report` copies the trip and posts `Event::Pointed` with the selected id. The effect is `ReportBag`. That effect is not confirmation. `confirm_with_mock` accepts only `ReportBag`, shows that id to `MockReport`, and posts `Event::Named` with the id the mock returns. Other effects leave the trip unchanged. After a successful name the phase is `Picking`, the gripper is empty, and ready, arrival, and completion stay false. The machine emits `Effect::Pick` on that transition. The harness does not apply it. No grasp, ready, follow, or handoff event is sent.

The seeded layout still places both bags between 400 mm and 2000 mm of the operator, on opposite sides. An exact aim at one of them has a single forward hit because the other bag is behind the ray. That is a property of this fixture, not a rule that two forward bags are unique. The unit tests cover two forward bags.

### Sources

Used for the shape. Not taken as dependencies. The 9.6 cm and 2 cm figures are not this model's thresholds.

- Professor PDF section 3.1, printed pages 15–17. Hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. No public URL is assigned to that file.
- Ray parameter `t >= 0`. If the projection falls behind the origin, the closest point is the origin: https://www.geometrictools.com/Documentation/DistancePointLine.pdf
- Perpendicular distance from the length of the cross product: https://cs.oberlin.edu/~bob/cs357.08/VectorGeometry/VectorGeometry.pdf
- A pointing gesture that does not single out one object is region-pointing, not object-pointing: https://www.techfak.uni-bielefeld.de/ags/wbski/veroeffentlichungen/download/KranstedtEtAl_GW05.pdf
- Threshold, then an ambiguity test, otherwise no target: https://fja.sciencesconf.org/57073/pointing_hri.pdf
- Household pointing rays and a reported average position accuracy of 9.6 cm: http://webdocs.cs.ualberta.ca/~vis/HRI/papers/ICRA2013SEPO.pdf
- `u128::isqrt`: https://doc.rust-lang.org/std/primitive.u128.html#method.isqrt
- `i128` for products of `i32` coordinates: https://doc.rust-lang.org/std/primitive.i128.html

### Tests

Red, before the behavior. The perception stub returned `Ok(Id(1))` for every sample. `cargo test -p perception --all-targets --locked` was 18 failed and 0 passed. The doctest failed with `Id(1)` instead of `Id(4)`. Representative assertions: the second candidate was `Id(1)` instead of `Id(9)`; two bags on one ray were `Ok(Id(1))` instead of `Err(Ambiguous)`; a duplicate id was `Ok(Id(1))` instead of `Err(DuplicateId)`. The aim stub returned `(0, 0)`. `cargo test -p sim_runtime --lib --locked` was 14 passed and 3 failed, including `(0, 0)` instead of `(1531, 0)` for seed 1. The mock stub stored `Id(3)`. Its test was 1 failed: `Some(Id(3))` instead of `Some(Id(4))`. The harness stub returned the first bag and staged `Effect::Pick`. `cargo test -p harness --lib --offline` was 8 failed and 0 passed. Representative assertions: an aim at `Id(4)` returned `Id(3)`; the reversed slice returned `Id(4)` instead of `Id(3)`; a hidden aim was `Ok(Id(3))` instead of `Err(NoHit)`; a zero direction was `Ok(Id(3))` instead of `Err(NoDirection)`; a duplicate was `Ok(Id(3))` instead of `Err(DuplicateId)`; the staged effect was `Pick` instead of `ReportBag`.

Green counts below are the workspace commands on this tree. Debug and release match. They do not close an acceptance row. Unit mapping: [docs/tasks/m5-unit-coverage.md](docs/tasks/m5-unit-coverage.md).

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 6, harness 8, hri 1, manipulation 0, navigation 0, perception 18, sim_runtime 17, task_3_1 30
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
domain 6, harness 8, hri 1, manipulation 0, navigation 0, perception 18, sim_runtime 17, task_3_1 30
0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
domain 1, perception 1, sim_runtime 1, task_3_1 1, other crates 0
0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=8 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

The attached copy is 1129757 bytes, 52 pages, creation 2024-07-09. `pdfinfo` reported that page count and `CreationDate: Tue Jul  9 14:56:18 2024 UTC`. A Thai scan of the tree was `thai_line_count=0`. `Cargo.lock` adds the `harness` path package only.

### Adversarial notes

A zero direction is rejected even when the candidate list is empty. A missing operator is rejected before a missing direction. The reversed ray of a bag that was a hit returns `NoHit`, not that bag. A bag at the origin, a bag beside the operator, and a bag behind the operator are `NoHit`. The inclusive corridor point `(1000, 150)` with direction `(1000, 0)` is selected. `(1000, 151)` is `NoHit`. Perpendicular gaps of 2 mm and 3 mm are `Ambiguous` and a unique hit respectively. Two ids at the same point are `Ambiguous`, as are a near bag and a far bag on the same ray, in either slice order. An invisible bag on the ray is not selected, and it does not take a visible bag 10 mm off the ray or tie a visible bag on the ray. Seed 1 east is `(1531, 0)` and west is `(-453, 0)` from the operator at `(0, 1000)`. Neither sample is the car offset. Aiming at the car id, the robot id, or the operator id is `UnknownBag`, and the world, events, and hidden truth compare equal afterward. `direction_mm` from `i32::MIN` to `i32::MAX` is overflow, and both input points keep their coordinates. Seeds 0 through 31 and `u64::MAX` each aim both bags inside the 400 mm to 2000 mm bounds and replay the same sample. The harness reports each of those bags, leaves `named` false after `ReportBag`, and after the mock is `Picking` with an empty gripper. Aiming at the other bag reports that bag, not the referee id. Pick, ready, follow, and release effects do not confirm, and the trip compares equal. A confirmation stamp earlier than the report is `Regressed` and the trip stays unnamed. The same seed replays the same report.

### Limits

The corridor and the tie gap are assumptions, not printed measurements. `floor(hypot)` can make a non-square ray a fraction of a millimeter stricter than 150 mm. The fixture does not add Kinect-scale noise, and it does not model a hand. A passing test does not mean an acceptance row passed. The open scoring rows are untouched. No statement here says the software has no remaining defects. M6, M7, M8, and M9.3 were not started.

## M4 review test gap (2026-11-06)

Independent review of `833737192b59da784d5e2a09d93419cdd00f64db`. No verified production merge blocker. Its full pinned checks, debug and release, were 48 library tests and 3 doctests, all passed. It checked the professor PDF hash `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. A scratch probe reported 161902 steps, 352 completions, 5322 timeouts, 0 panics, and 0 contract failures. That review was bounded, not exhaustive. M9.3 was not run and is still later. Acceptance rows stay `not-run`. The seven open scoring rows stay open. Public 2025.1 is not the baseline. The raw PDF was not committed.

The gap is `every_rejection_leaves_the_previous_trip`. It asserts equality only when `apply` returns `Err`, so a mutant that accepts an invalid event still passes. Two scratch mutants survived the previous 28 library tests and 1 doctest. `Received` of the pointed bag by the operator is already `Reject::OutOfOrder` from the unmatched arm of `Following`. `Arrived` after `Dropped` is already `Reject::OutOfOrder` from the unmatched arm of `Dropped`. No new rejection code was added, and the production transitions were not rewritten.

Red, with `Event::Received { bag, by } => self.receive(...)` inserted into `on_following` only. Debug: `receipt_while_following_before_arrival_is_rejected` panicked at `debug_assert!(next.consistent())` because completion still had `arrived == false`; 29 passed and 1 failed. Release: that test got `Ok(Outcome { effect: None })` instead of `Err(OutOfOrder)`; 29 passed and 1 failed. The doctest passed. The insertion was then removed.

Red, with `Arrived` in `on_dropped` setting `arrived`, `main_complete`, and `Phase::Complete`. Debug and release both failed `arrival_after_a_drop_stays_dropped` with `Ok(Outcome { effect: None })` instead of `Err(OutOfOrder)`; 29 passed and 1 failed. The doctest passed. That arm was then removed. `git diff` of the trip file against `8337371` is only the two tests.

Clean tree, debug and release: domain 6, `sim_runtime` 14, `task_3_1` 30, other crates 0. Doctests 3. Zero failures. `fmt`, build, `clippy -D warnings`, rustdoc `-D warnings`, the workspace check, and the spec checker (`ok rows=56 pdf=True` on the attached PDF) exited 0.

## M4 task state machine (2026-11-06)

Scope. Issue 8. One writer. Branch `cursor/m4-task-state-machine-0c9f` from `1aead3bb8b49937e5fb89d2b0ce43711f9b4abaf`. Pure `task_3_1` orchestration only. No perception algorithm, grasp model, follower, end-to-end run, scorer, hardware, deploy, registration, contact, or spending. M5 was not started. M9.3 was not run and is not claimed. The seven open scoring rows stay open. Every acceptance row in the task specification stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 is not the baseline. The raw PDF was not committed.

### M3 independent review

Reviewed pull request 7 at head `c7cfe59ede792ddd9c6af06201642f7af75418d4`. `git diff c7cfe59ede792ddd9c6af06201642f7af75418d4 1aead3bb8b49937e5fb89d2b0ce43711f9b4abaf` was empty. The pull request had no review comments and no issue comments. CI run `42195311283` completed with conclusion success on that head: https://github.com/clytie0x/robocup-home/actions/runs/42195311283. No merge blocker was verified.

Re-ran the merged tree in debug and release before the M4 edits. Both profiles: domain 6 passed, `sim_runtime` 14 passed, other crates 0, doctests 2 passed (domain 1, `sim_runtime` 1). Zero failures.

An independent splitmix64 (2015) plus xoshiro256** 1.0 `next` matched the crate's first draw for seeds 0 through 306, including seed 0 `0x99ec5f36cb75f2b4`. Seeds 0 through 19999 each kept at least two bags, all four obstacle classes, `revealed_car() == None`, the car point out of the observation, and the car id off the visible ids. A 6 ms step from `u64::MAX - 5` returned overflow and left the observation unchanged. `uniform` on span `2^63 + 7`, 64 draws from seed 1, matched a mirrored rejection loop that rejected 69 draws. The committed M3 tests still do not enter that rejection branch. The probe found no mismatch, so no M3 code change was made. The M3 log's spec check was `pdf=False`. This review hashed the attached professor PDF and did not commit it: the same baseline hash, 1129757 bytes, 52 pages, creation 2024-07-09. This review does not say that no defect remains.

### Plan

Add one explicit trip in `task_3_1`. `apply(now_ms, event)` returns one command effect or a rejection. The trip is copied and replaced only after `Ok`, so a rejection leaves it unchanged. Effects ask a later module to report, pick, signal, follow, or release. Confirmations are separate events. `sim_runtime` stays out of the crate. The capability crates stay empty of algorithms. `Cargo.lock` did not change.

### Decisions

Phase order for the main trip is `Idle`, `AwaitPoint`, `Naming`, `Picking`, `Signaling`, `Following`, `Releasing`, `Complete`. Failure phases are `PickFailed`, `WrongBag`, `Dropped`, `Lost`, `HandoffFailed`, and `TimeUp`. `main_complete` is set only in the transition to `Complete`, which requires a confirmed arrival and a confirmed receipt of the pointed bag by the operator, and it clears the carried bag.

`Begin` arms a 300000 ms budget. The millisecond unit and `Begin` are assumption `CML-ASM-CLOCK`, stored as `CLOCK_START = "begin"`. They are not a printed start signal. `ALT_START` stays `"unresolved"`. Elapsed time is `now_ms.checked_sub(start_ms)`. A stamp with elapsed equal to 300000 is inside the budget. A larger stamp moves an unfinished trip to `TimeUp` and does not apply the payload. `Complete` stays complete, including after a later stamp. `Reenter` and `JoinQueue` are `Reject::OutOfScope` before the clock is read. There is no optional-goal timer and no score.

A loss of the operator can return to `Following` when that same id is found. That resume emits no follow command and decides no points. A second drop is not given a price: dropping with an empty gripper is `Reject::Impossible`.

### Sources

Used for the shape. Not taken as dependencies.

- Enumerations: https://doc.rust-lang.org/reference/items/enumerations.html
- `Result` for a rejected event: https://doc.rust-lang.org/std/result/enum.Result.html
- Typestate was read and not used. External events have to be rejected at runtime: https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html
- `u64::checked_sub`: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_sub
- The local clock and point already leave their value unchanged when the operation fails.

### Tests

Red, against a stub that marked every `apply` complete. `cargo test -p task_3_1 --all-targets --locked` was 1 passed (`new_trip_is_idle`) and 27 failed. `cargo test -p task_3_1 --doc --locked` was 1 failed, assertion `!trip.main_complete()`. The first transition body left `ready` and `arrived` unset, and the suite was then 14 failed, including `trip.ready()` and the complete-state check. Those flags are set only on `ReadyTold` and `Arrived`. The next run was 28 passed and the doctest passed.

Green counts below are the workspace commands on this tree. Debug and release match. They do not close an acceptance row. Unit mapping: [docs/tasks/m4-unit-coverage.md](docs/tasks/m4-unit-coverage.md).

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
clippy_exit:0

cargo test --workspace --all-targets --locked
domain 6 passed, sim_runtime 14 passed, task_3_1 28 passed, other crates 0 passed
0 failed
test_exit:0

cargo test --workspace --doc --locked
domain 1 passed, sim_runtime 1 passed, task_3_1 1 passed, other crates 0 passed
0 failed
doctest_exit:0

cargo test --workspace --all-targets --locked --release
domain 6 passed, sim_runtime 14 passed, task_3_1 28 passed, other crates 0 passed
0 failed
release_test_exit:0

cargo test --workspace --doc --locked --release
domain 1 passed, sim_runtime 1 passed, task_3_1 1 passed, other crates 0 passed
0 failed
release_doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=7 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

PROFESSOR_PDF=<attached copy> python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
spec_exit:0
```

`Cargo.lock` had no diff.

### Adversarial notes

A stamp earlier than the last accepted stamp is `Regressed` and the trip compares equal. The same stamp is accepted. `Begin` at `u64::MAX - 10` still treats a stamp at `u64::MAX` as inside the budget, which is the case `wrapping_add(300000)` would misread. A stamp at `u64::MAX` after `Begin` at 0 ends in `TimeUp` and does not wrap into the budget. Ticks at 100000, 200000, and 300000 ms after the start stay open; the next millisecond is `TimeUp`. Receipt at exactly `start + 300000` completes. Receipt or arrival one millisecond later does not, and an earlier stamp after that is `Regressed`. A tick at 400000 ms after completion stays `Complete`, with an empty gripper. A 30000 ms tick returns without a two-second wall wait. The wall clock is not the simulation time.

Swapped and repeated ids do not advance the phase. Naming or grasping the operator id is `Role`. Finding or paying the bag id as a person is `Role`. A different bag is `WrongId` on the name and `WrongBag` on a grasp, with no ready command. `GraspFailed` for the pointed bag ends in `PickFailed` and a later grasp is `Stale`. A drop clears the carried id and does not complete, including after arrival. A second drop is `Impossible`. Loss blocks arrival until the same operator id returns, with no new follow effect. `HandoffFailed` keeps the bag; only a later receipt by the operator completes and clears it. Optional events at a late stamp leave the trip equal, including after completion. The matrix of events from idle, naming, picking, following, releasing, lost, and pick-failed prefixes keeps the previous value whenever `apply` returns `Err`. One script that includes a regression, a loss, a failed handoff, and an optional event replays equal.

### Limits

No perception, manipulation, navigation, or HRI algorithm is in this milestone. The car coordinate is not an input. Optional re-entry and the queue are not implemented. The machine does not decide partial credit, the drop cap, a second drop's price, follow points after a loss, the absent-team numbers, or the alternative start. A passing unit test does not mean an acceptance row passed. No statement here says the software has no remaining defects.

## M3 clock and seeded world (2026-11-06)

Scope. Issue 6. One writer. Branch `cursor/m3-sim-clock-world-e955` from `58cfa9de29c033a0061dc7e654d84b20fb69e6f6`. Code commits before this note: `acf444a` and `fedf5ab`. No task state machine, perception, grasp, path planning, scorer, hardware, deploy, registration, contact, or spending. M4 was not started. M9.3 was not run. This self-review does not replace that interrogation. The seven open scoring rows stay open. Every acceptance row in the task specification stays `not-run`. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. Public 2025.1 is not the baseline.

### Plan

Add a reusable simulation clock and a seeded world behind `World`, `Arm`, and `Base`. Keep those traits in `domain`. Put the clock, the generator, and the adapters in `sim_runtime`. Prove the fixture, replay, clock bounds, and the hidden car with tests that failed before the behavior existed.

### Decisions

`World`, `Arm`, and `Base` stay in `domain`, so `sim_runtime` still depends only on `domain` and `task_3_1` still does not depend on the simulator. `artifacts/check_workspace.py` was not changed. Manipulation and navigation stay empty of grasp and planning. No registry dependency was added. `Cargo.lock` did not change.

The stored model uses integer millimeters and an integer millisecond count. A float enters only through `Mm::try_from_meters`, which rejects non-finite values and values outside `i32`. `Id` is a non-zero `u32`. `Step` is a `u64` count. Negative counts are rejected before a step exists. `Clock::advance` uses `u64::checked_add` and writes the new time only after that succeeds. A failed world advance does not append an event.

The generator is owned. It is xoshiro256** 1.0 (`next` only; jump functions were not copied), seeded by splitmix64 (2015). Version string: `xoshiro256**-1.0+splitmix64-2015`. Layout version: `m3-layout-1`. The seed is consumed while the layout is built. Replay means constructing again with the same seed and the same `advance` calls. A zero step is a successful call: the clock does not move, and the trace records the call. The generator is not cryptographic.

Layout assumptions, labeled in `sim_runtime::assume`, are not official measurements. The printed 2 m bag limit is the inclusive upper bound of 2000 mm. The 400 mm lower bound, the opposite-x placement, the operator at `(0, 1000)` mm, the robot at `(0, 0)` mm, the inside half-plane `x <= 2500` mm, the car at `(8000, -1500)` mm, the obstacle pitch, and the 400 mm clearance are assumptions. The millisecond unit is an assumption. The clock does not start or enforce the five-minute test. Each obstacle is one placeholder. The crowd placeholder is not three or four people. Bag kind `LightweightStiffVerticalHandles` records section 2.1.5. It is not a stiffness or weight measurement. `visible` records the setup fact, not a detector. The obstacle order is one Durstenfeld shuffle of the four classes. The PDF still calls that order arbitrary. The car point is fixed in the hidden truth, so a different seed cannot reveal it. Seeds 1 and 2 change bag offsets and obstacle order.

`SimWorld` does not implement `Debug`. The car is readable from `truth()`, which is not part of the `World` trait.

### Sources

Used for this design. Not taken as dependencies.

- xoshiro256** 1.0 `next`, public domain, David Blackman and Sebastiano Vigna: https://prng.di.unimi.it/xoshiro256starstar.c
- splitmix64, 2015, Sebastiano Vigna: https://prng.di.unimi.it/splitmix64.c
- `u64::checked_add`: https://doc.rust-lang.org/std/primitive.u64.html#method.checked_add
- `std::time::Instant` is an operating-system clock and is not used as simulation time: https://doc.rust-lang.org/std/time/struct.Instant.html
- embedded-hal 1.0.0 keeps traits separate from device implementations: https://docs.rs/embedded-hal/1.0.0/embedded_hal/
- Durstenfeld shuffle, high index down to 1: https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle
- The Rust Rand Book marks `SmallRng` not portable. The `rand` crate was not added: https://rust-random.github.io/book/guide-rngs.html
- Explicit virtual clocks were the pattern, not a dependency. Examples: https://docs.rs/vortex-sim/latest/vortex_sim/struct.SimClock.html and https://docs.rs/refault/latest/refault/
- The `xoshiro` crate at 0.0.5 was not added. The `next` function above was rewritten here.

### Tests

Red, before the behavior. Stubs compiled and returned the wrong value. `cargo test --workspace --all-targets --locked` stopped at `domain`: 6 failed, 0 passed. `cargo test --workspace --doc --locked` then failed the `Step::try_from_i64` doctest, 1 failed. `cargo test -p sim_runtime --all-targets --locked` failed 14, passed 0. `cargo test -p sim_runtime --doc --locked` failed the `Clock` doctest, 1 failed. Representative assertions: `try_from_i64(-1)` was `Ok(Step(0))` instead of `Err(Negative)`; `Mm::try_from_meters(NaN)` was `Ok(Mm(0))`; clock overflow returned `Ok(())`; seed 0 drew `0` instead of `0x99ec5f36cb75f2b4`; `revealed_car` was `Some((8000, -1500))`; the arm reported `Some(Id(1))`.

Green, after `acf444a` and `fedf5ab`, on the tree that also contains this note's documentation edits. Counts are 20 library tests and 2 doctests. Zero failures. The same commands were the check set. `pdf=False` means the professor PDF was absent. This run did not hash that file.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
clippy_exit:0

cargo test --workspace --all-targets --locked
domain: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
hri, manipulation, navigation, perception, task_3_1: ok. 0 passed; 0 failed
sim_runtime: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test_exit:0

cargo test --workspace --doc --locked
domain: ok. 1 passed; 0 failed
sim_runtime: ok. 1 passed; 0 failed
other crates: ok. 0 passed; 0 failed
doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
doc_exit:0

python3 artifacts/check_workspace.py
ok members=7 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False
spec_exit:0
```

`Cargo.lock` had no diff. A Thai scan of the tree was `thai_line_count=0`.

M3 rows. These passes are the commands above. They do not close an acceptance row in `docs/tasks/3.1-carry-my-luggage.md`.

| requirement | source | test | result | gate |
| --- | --- | --- | --- | --- |
| Robot, operator, at least two defined bags, and all four obstacle classes | Professor PDF section 3.1 and section 2.1.5. Prerequisite only. | `fixtures_keep_two_bags_and_four_classes`, seeds 0 through 31 and `u64::MAX`; `seed_one_matches_layout_v1` | pass, suite above | sim |
| Car hidden from the task port | Professor PDF car sentence. Prerequisite only. | `the_task_port_hides_the_car`, seed 1 | pass, suite above | sim |
| Clock zero, step, maximum, and overflow | Issue 6. The millisecond unit is `CML-ASM-CLOCK`, not a printed start event. | `clock::tests`, including a 30000 ms step that returns in under 2 wall-clock seconds | pass, suite above | sim |
| Same-seed replay | Issue 6. Algorithm versions above. | `the_same_seed_replays_the_same_trace`, seed 11, including one rejected overflow | pass, suite above | sim |
| Different seed changes the visible trace | Issue 6. | `a_different_seed_changes_the_visible_trace`, seeds 1 and 2 | pass, suite above | sim |

### Adversarial notes

`i64::MIN` and `-1` never become a step. `NaN`, both infinities, and `1e20` meters are rejected. An offset that overflows either axis returns an error and the original point still has both of its coordinates. A clock at `u64::MAX` rejects one more millisecond and stays there; a zero step at that value stays there. After a rejected world step, the observation, the event slice, and the hidden truth compare equal to the pre-call copies, and a later 5 ms step still applies. The task-facing function returns `None` while `truth().car()` is `(8000, -1500)` and that point is not among the observed points. Seed 0 and `u64::MAX` still build two bags and four distinct classes. The arm stays empty after a 40 ms step, and the base pose stays at the robot origin.

Clippy on the library target, without the test cfg, reported the fixture helpers as dead code. They moved into the test module. That was a warning, not a behavior change.

### Limits

The model is an abstract fixture. It is not a claim of physical accuracy, and it is not a hardware result. It does not grasp, follow, plan, perceive, or score. It does not discover the car. It does not apply the five-minute limit. It does not decide any open scoring row. A passing M3 test does not mean an acceptance row passed. No statement here says the software has no remaining defects.

## Visibility and M2 CI read (2026-11-06)

Read after the M2 merge. This is the current visibility. No visibility write was made in this read. The refused 403 attempt in the next section stays as history.

```text
gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,default_branch:.default_branch,html_url:.html_url}'
{"default_branch":"main","html_url":"https://github.com/clytie0x/robocup-home","private":false,"visibility":"public"}

gh repo view clytie0x/robocup-home --json nameWithOwner,isPrivate,visibility,url,defaultBranchRef
{"defaultBranchRef":{"name":"main"},"isPrivate":false,"nameWithOwner":"clytie0x/robocup-home","url":"https://github.com/clytie0x/robocup-home","visibility":"PUBLIC"}
```

M2 is merged at `58cfa9de29c033a0061dc7e654d84b20fb69e6f6`. Pull-request CI run `42186659932` attempt 2 completed with conclusion success on head `52e73bde7b0a236e723406ee60b643a515cbb68c`. The merge push also completed with conclusion success: run `42193390300` attempt 1 on that merge commit, https://github.com/clytie0x/robocup-home/actions/runs/42193390300. The billing failure on run `42185539820` remains the earlier record in the M2 section. It is not the current CI result.

```text
gh api repos/clytie0x/robocup-home/actions/runs/42186659932 --jq '{id,run_attempt,event,head_sha,head_branch,conclusion,status,html_url}'
{"conclusion":"success","event":"pull_request","head_branch":"cursor/rust-workspace-ci-91af","head_sha":"52e73bde7b0a236e723406ee60b643a515cbb68c","html_url":"https://github.com/clytie0x/robocup-home/actions/runs/42186659932","id":42186659932,"run_attempt":2,"status":"completed"}

gh api repos/clytie0x/robocup-home/actions/runs/42193390300 --jq '{id,run_attempt,event,head_sha,head_branch,conclusion,status,html_url}'
{"conclusion":"success","event":"push","head_branch":"main","head_sha":"58cfa9de29c033a0061dc7e654d84b20fb69e6f6","html_url":"https://github.com/clytie0x/robocup-home/actions/runs/42193390300","id":42193390300,"run_attempt":1,"status":"completed"}
```

## Visibility instruction (2026-11-06)

Historical refused attempt. It is not the current visibility. The current read is the section above.

On 2026-11-06 at about 15:14 Asia/Bangkok, the owner instructed that `clytie0x/robocup-home` be made public and that CI be rerun. That replaces the earlier private-only rule for this repository. It is not permission to publish any other repository, and it is not permission to change this visibility again without a new instruction.

Before the change, the tracked tree and history were scanned for private keys, GitHub tokens, cloud access-key ids, PDF files, and `.env` files. None were found. The laptop path and the professor excerpts remain intentional source notes. The sentence about an arena SSID/password in the professor excerpt is rule text, not a stored password.

The repository was still private. Both documented writes were refused. CI was not rerun, because the visibility did not become public.

```text
gh repo edit clytie0x/robocup-home --visibility public --accept-visibility-change-consequences
HTTP 403: Resource not accessible by integration

gh api -X PATCH repos/clytie0x/robocup-home -f visibility=public
{"message":"Resource not accessible by integration","documentation_url":"https://docs.github.com/rest/repos/repos#update-a-repository","status":"403"}

gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,html_url:.html_url,permissions:.permissions}'
{"html_url":"https://github.com/clytie0x/robocup-home","permissions":{"admin":false,"maintain":false,"pull":false,"push":false,"triage":false},"private":true,"visibility":"private"}
```

The active `gh` account is `cursor`. Its token is an integration token, and the repository permission read shows `admin: false`.

## M2 workspace and CI (2026-11-06)

Scope. Issue 4, opened before this branch. One writer. Laptop skeleton only. No task behavior, world, clock, state machine, scoring choice, hardware, deploy, registration, contact, or spending. M3 was not started. M9.3 was not run. The specification file is unchanged from `77e801b5ae5645af07cf8bb3f19725711dd491b8`. The seven open rows stay open, and every acceptance result stays `not-run`. Public 2025.1 is not the baseline. The professor file remains the baseline: `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`, cover 2024 Rev-1, revision 2024-07-09.

Branch `cursor/rust-workspace-ci-91af` from that merge. Commits before this note: `3b9c9addeeb22b17d7ade12e90d2de7215bec072` and `83c50e8b96d043cd164ed6239009a18f81b467a1`.

### Toolchain

Retrieved 2026-11-06. The machine default was rustc 1.83.0 (90b35a623 2024-11-26). That compiler was not used for the checks below. `rust-toolchain.toml` selected 1.99.1.

- Official channel manifest `https://static.rust-lang.org/dist/channel-rust-stable.toml` had `date = "2026-10-15"` and `[pkg.rustc] version = "1.99.1 (58cf8f9c3 2026-10-13)"`. `rustup toolchain install 1.99.1` reported the same update date and version.
- Announcement: `https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/`, 15 October 2026. 1.99.1 is a stable point release. It fixes an x86_64 FFI `bool` miscompilation present in 1.99.0, so the pin is 1.99.1.
- `https://releases.rs/docs/1.100.0/` said 1.100.0 was unreleased and would become stable on 12 November 2026. It is not the pin.
- Edition 2024 is the current stable edition. `https://doc.rust-lang.org/stable/edition-guide/rust-2024/` lists release version 1.85.0. The announcement is `https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/`, 20 February 2025. `cargo +1.99.1 new --lib` wrote `edition = "2024"`. 1.99.1 is newer than 1.85.0, so the edition is compatible. No nightly edition is selected.
- A virtual workspace has no package edition, so the resolver is set explicitly. `https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html` says edition 2024 implies resolver 3 for a package, and a virtual workspace must set `resolver`. The workspace uses `resolver = "3"`, which the Cargo book says requires Rust 1.84 or newer.
- `rust-version` is 1.99.1, the same value as the pin. `https://doc.rust-lang.org/cargo/reference/rust-version.html` describes that field. An older compiler was not verified, so none is claimed. `https://doc.rust-lang.org/stable/cargo/guide/continuous-integration.html` warns that a floating toolchain can change warnings. The sample matrix of stable, beta, and nightly is not used.
- `https://rust-lang.github.io/rustup/overrides.html` documents `channel`, `components`, and `profile`. The file pins channel `1.99.1`, components `rustfmt` and `clippy`, and profile `minimal`.

### Crates

One crate, `domain`, is the shared contract. The milestone name `task_api` is that role. There is no second domain crate. Members are `domain`, `perception`, `navigation`, `manipulation`, `hri`, `task_3_1`, and `sim_runtime`. `task_3_1` depends on `domain` and the four capability crates. `sim_runtime` depends on `domain`. No member depends on `sim_runtime`. The graph has no cycle. Libraries are empty apart from a one-line crate doc. No registry dependency was added. `Cargo.lock` is version 4 and lists those seven path packages, with no `source` and no `checksum` lines.

The M1 checker used to fail when `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, or `.github` existed. That guard is removed. The Thai scan now includes `.rs`, `.toml`, `.yml`, and `.yaml`. The checker still requires `not-run` results, the seven open rows, and the professor excerpt match. Passing it is source integrity, not task compliance. `artifacts/check_workspace.py` checks membership and edges only.

### Local checks

Run on commit `83c50e8b96d043cd164ed6239009a18f81b467a1` before this note. `cargo test --all-targets` does not include doctests on cargo 1.99.1, so doctests were a separate command. Zero tests means the crates have no behavior tests. It is not a task pass.

```text
rustc --version
rustc 1.99.1 (58cf8f9c3 2026-10-13)

cargo --version
cargo 1.99.1 (5f94df478 2026-08-27)

cargo fmt --all -- --check
fmt_exit:0

cargo build --workspace --all-targets --locked
   Compiling domain v0.1.0 (/agent/repos/robocup-home/crates/domain)
   Compiling navigation v0.1.0 (/agent/repos/robocup-home/crates/navigation)
   Compiling manipulation v0.1.0 (/agent/repos/robocup-home/crates/manipulation)
   Compiling perception v0.1.0 (/agent/repos/robocup-home/crates/perception)
   Compiling hri v0.1.0 (/agent/repos/robocup-home/crates/hri)
   Compiling task_3_1 v0.1.0 (/agent/repos/robocup-home/crates/task_3_1)
   Compiling sim_runtime v0.1.0 (/agent/repos/robocup-home/crates/sim_runtime)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
build_exit:0

cargo clippy --workspace --all-targets --locked -- -D warnings
    Checking domain v0.1.0 (/agent/repos/robocup-home/crates/domain)
    Checking perception v0.1.0 (/agent/repos/robocup-home/crates/perception)
    Checking navigation v0.1.0 (/agent/repos/robocup-home/crates/navigation)
    Checking manipulation v0.1.0 (/agent/repos/robocup-home/crates/manipulation)
    Checking hri v0.1.0 (/agent/repos/robocup-home/crates/hri)
    Checking sim_runtime v0.1.0 (/agent/repos/robocup-home/crates/sim_runtime)
    Checking task_3_1 v0.1.0 (/agent/repos/robocup-home/crates/task_3_1)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
clippy_exit:0

cargo test --workspace --all-targets --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running unittests src/lib.rs (target/debug/deps/domain-448ce7fff338a308)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/hri-ff596ed141144384)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/manipulation-f541569fa2d8584c)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/navigation-ded3b9b6ad1a8998)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/perception-0bc0c145d42af328)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/sim_runtime-0cf4ef297df5fdfb)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/task_3_1-fb66ac1bff22a062)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test_exit:0

cargo test --workspace --doc --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.00s
   Doc-tests domain
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests hri
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests manipulation
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests navigation
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests perception
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests sim_runtime
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests task_3_1
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
doctest_exit:0

RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.19s
   Generated /agent/repos/robocup-home/target/doc/domain/index.html and 6 other files
doc_exit:0

python3 artifacts/check_workspace.py
ok members=7 pin=1.99.1 edition=2024
workspace_exit:0

python3 artifacts/check_m1_spec.py --self-test
self-test ok
self_test_exit:0

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=False
spec_exit:0
```

`pdf=False` means the professor PDF was absent. This run did not hash that file. It is not a full PDF check.

### Adversarial fixtures

Uncommitted edits, then restored with `cp` from backups. No repository file was deleted. After restore, `cmp` matched the backups for `crates/domain/src/lib.rs`, `crates/task_3_1/Cargo.toml`, `Cargo.lock`, and `docs/tasks/3.1-carry-my-luggage.md`. `cargo fmt --all -- --check` then exited 0, `artifacts/check_workspace.py` printed `ok members=7 pin=1.99.1 edition=2024`, and `artifacts/check_m1_spec.py` printed `ok rows=56 pdf=False`.

```text
cargo fmt --all -- --check
Diff in /agent/repos/robocup-home/crates/domain/src/lib.rs:1:
 //! Shared contract between task logic and adapters. This crate is the task_api role.

-fn badly_formatted( ){let value=1;}
+fn badly_formatted() {
+    let value = 1;
+}
FMT_BAD:1

cargo clippy --workspace --all-targets --locked -- -D warnings
error: unused variable: `value`
 --> crates/domain/src/lib.rs:4:9
  |
4 |     let value = 1;
  |         ^^^^^ help: if this is intentional, prefix it with an underscore: `_value`
  |
  = note: `-D unused-variables` implied by `-D warnings`
error: could not compile `domain` (lib) due to 1 previous error
CLIPPY_BAD:101

cargo build --workspace --all-targets --locked
error[E0308]: mismatched types
 --> crates/domain/src/lib.rs:4:5
  |
3 | pub fn type_error() -> i32 {
  |                        --- expected `i32` because of return type
4 |     "no"
  |     ^^^^ expected `i32`, found `&str`
error: could not compile `domain` (lib) due to 1 previous error
BUILD_BAD:101

python3 artifacts/check_workspace.py
task_3_1 deps domain hri manipulation navigation perception sim_runtime
BOUNDARY_BAD:1

python3 artifacts/check_m1_spec.py
CML-TASK result pass
SPEC_BAD:1
```

### CI and review

Workflow `.github/workflows/ci.yml` runs on pull requests and on pushes to `main`. Permissions are `contents: read`. Checkout uses `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` with `persist-credentials: false`. That commit is tag `v7.0.1` of `actions/checkout`, message `prep v7.0.1 release (#2531)`, committer date 2026-07-17. It is the actions repository, not a fork. `GET /repos/actions/checkout/attestations/sha/3d3c42e5aac5ba805825da76410c181273ba90b1` returned HTTP 404, so this pin is the release commit and not an artifact attestation. GitHub's secure-use page, retrieved 2026-11-06 from `https://docs.github.com/en/actions/reference/security/secure-use`, says a workflow dependency lockfile is the strongest measure and that a full commit SHA is the measure when a lockfile is not adopted. `gh extension list` showed no extension, and no lockfile tool was added. The runner label is `ubuntu-24.04` because `https://github.blog/changelog/2026-09-17-ubuntu-26-generally-available-and-latest-migration/` says `ubuntu-latest` moves from 24.04 to 26.04 between 19 October 2026 and 19 November 2026.

The compiler install step reads `rust-toolchain.toml`. A later step refuses channel `stable`, `beta`, or `nightly`, and requires `rustc --version` to start with `rustc` plus that channel. Packages set `publish = false`. No crates.io dependency is declared.

GitHub Actions was read for commit `82d8e4c78cfc91b9b5833a1a92f43b09c2772357` on pull request 5. The job failed before any step ran. The runner did not execute fmt, build, clippy, or test. This is not a green check. No billing setting was changed. A green local command is not a green GitHub check. This note does not say the software is free of undiscovered defects.

```text
gh pr checks 5
check	fail	2s	https://github.com/clytie0x/robocup-home/actions/runs/42185539820/job/127784865147
checks_exit:1

gh api repos/clytie0x/robocup-home/actions/runs/42185539820/jobs
steps: []
conclusion: failure

annotation on .github:
The job was not started because recent account payments have failed or your spending limit needs to be increased. Please check the 'Billing & plans' section in your settings
```

That billing failure is run `42185539820`. It is not the current CI result. The completed runs are in the visibility and M2 CI read above.

Privacy read during M2, before the repository was public. It is not the current visibility.

```text
gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,default_branch:.default_branch}'
{"default_branch":"main","private":true,"visibility":"private"}

gh repo view --json nameWithOwner,isPrivate,visibility,url
{"isPrivate":true,"nameWithOwner":"clytie0x/robocup-home","url":"https://github.com/clytie0x/robocup-home","visibility":"PRIVATE"}
```

## M1 correction (2026-11-06)

The owner chose the professor PDF already on the laptop. The task stays 3.1. This year's public task is not the baseline. Public 2025.1 is not the normative baseline. The earlier M1 section below used that public release and is superseded for the task text.

Normative file. `C:\Users\teera\Downloads\at-home-robocup-my-2025-1.pdf`. The coordinator reports `C:\Users\teera\Downloads\at-home-robocup-my-2025.pdf` as a same-hash duplicate. The attached copy hashes to `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`, 1129757 bytes, 52 pages. Cover text is Version 2024 Rev-1, Last Revision Date 2024-07-09, Final version for RoboCup 2024 (Revision 2024-07-09 1). The filename says 2025. The printed year is 2024. No public URL is assigned to this file.

What the PDF says. Section 3.1, printed pages 15, 16, and 17. Pick up the pointed bag, signal ready, follow the operator to an unknown car location, and hand the bag back at the car. Maximum time is 5 minutes. Nine positive lines sum to 1015, and the sheet prints 1015. Optional re-entry is 100. The queue is 300. Each of the four avoids is 50. The sheet prints outstanding performance as 101. Ten percent of 1015 is 101.5, which is not printed. Section 2.3.1 prints 250 for not attending. The sheet prints -500. That conflict stays open.

This PDF has no safety-stop, restart, or door-start chapter. Those clauses were not copied from public 2025.1. Public 2024.2 is a different hash and is not the baseline. Older excerpts remain and are marked NONBASELINE.

Checks run on the corrected tree.

```text
python3 artifacts/check_m1_spec.py --self-test
self-test ok

python3 artifacts/check_m1_spec.py
ok rows=56 pdf=True
```

The self-test fails a total rewritten to 1016, a `result` cell set to `pass`, the number 1115 inserted into the specification, an open row whose "cannot pass" sentence is removed, and a fixed obstacle order. The attached PDF hash, size, page count, and section 3.1 text were checked because the file was present. fmt, build, clippy, test, and simulation were not run. No CI result exists to call green. No behavioral task test has passed.

## M1 status (2026-11-06, superseded by the correction above)

Scope. Issue 2. One writer. Documentation only. No application, simulation, hardware, dependency, toolchain pin, CI, registration, contact, or spending. M2 was not started. M9.3 interrogation was not run and is not claimed.

Baseline. The latest final RoboCup@Home release that still contains the test is tag 2025.1, commit `26403178bcdf39d2c5d9ed2e27849a9d8af1401e`, PDF SHA-256 `d23a76efaf436e6286076b34628a7451b3e8fd41e38b79e54b4db739acf9fe11`, 2378737 bytes. Printed identity is "Final version for RoboCup 2025", section 5.1 Help Me Carry, label `test:carry-my-luggage`. The requested number 3.1 is section 3.1 Team Registration in that edition. Section 3.1 Carry My Luggage is the 2024.2 final, and that text carries the bag to the car.

Current final. Tag 2026.2, commit `b844d5a6d6ebf4307c3250a01d1eca4ba56f8452`, PDF SHA-256 `d35e93c7f291a59a823691905b47dcd7e8de14ab3a4c546aca55ba20b30ce128`, matches the GitHub asset digest. Chapter 5 does not list the test. Release 2026.1 says Help me Carry was removed. The 2026.1 PDF body was not extracted.

Draft. The master PDF retrieved on 2026-11-06 has SHA-256 `45aa98b1773f202abfd7cd919b8aba5581cbf2e0182216fe479ee2eed59ace38` and the footer "Draft version for RoboCup 2027". It is not the baseline.

Decision. Score the laptop specification from 2025.1 section 5.1. Do not import 2024.2 numbers, the unreleased 2025-07-29 edit, the 2026.2 task list, the 2027 draft, or the Brazil Open 2025 draft (SHA-256 `9e4b1f735bffc6b29b64ade8af805fdc24bbeed4dab76c0a1650bfa622cca338`). Context.dev was not used. Its MCP server required authentication, and the official PDFs were fetched directly.

Arithmetic. Nine positive sheet lines sum to 1115, and printed page 41 prints 1115. Integer 10 percent of 1115 is 111, which is the printed outstanding line. The exact tenth is 111.5 and is not printed. The sheet prints not attending as -500. Section 3.9.1 prints 250. Both stay unresolved.

Coverage. 102 requirement rows. Open conflicts stay `block` `open` and `result` `not-run`. Arena construction, robot dimensions, other tests, and the Brazil Open score sheet are named gaps, not silent omissions. A `sim` row does not certify hardware.

Throughput. The official PDFs were hashed before any score was written. One writer owned the spec, the log, and `AGENTS.md`, because every row shares one baseline. No parallel writer was started.

Checks run after the specification was written.

```text
python3 artifacts/check_m1_spec.py --self-test
self-test ok

python3 artifacts/check_m1_spec.py
ok rows=102

tampered total 1116
printed score table does not match the 2025.1 excerpt

result cell pass
CML-ID-SECTION result pass
```

`curl -I -L` returned HTTP 200 for the 2025.1, 2026.2, and 2024.2 PDFs, the 2025.1 task source, both release tags, the master draft URL, and the Brazil Open PDF. There is no Cargo.toml, rust-toolchain.toml, Cargo.lock, or `.github` directory. fmt, build, clippy, test, and simulation were not run. No CI result exists to call green.

The decision trail for this milestone is this section. A second decision-log file was not added.

## Current status (documentation bootstrap, superseded in part by M1)

2026-11-06, 13:50 Asia/Bangkok. Documentation only. The working tree is English.
Commit `452dc4bb6ea8354b276bb1db659bb9fa88317243` still contains the earlier Thai prose. It was not amended, rebased, or force-pushed.

This workspace had no Grok Bot transcript to open. The policy change below is the follow-up that says those owner words were checked in that transcript. Nothing here adds a rule beyond that follow-up.

Owner confirmation already stored in that historical commit, in English paraphrase: confirmed, try the Cloud Agent on `clytie0x/robocup-home` to write the rules documentation only, and do not write runnable application code yet.

Current merge rule: Teerapat permits Grok Bot and RoboCup Lab to merge this repository without a fresh confirmation for each merge. The earlier "stop at the pull request and do not merge" line is superseded. It remains only as history, in the commit above and in the historical section below.

Current hardware rule: deploying and flashing are not categorically forbidden. No hardware is connected. The default is a laptop and simulation. Do not assume a physical device is available. Jetson, arm, and Pi stay untouched until a task specifically authorizes and connects that device. This milestone adds no hardware steps and does not touch hardware.

Still in force: Rust first; no stack, language, or style change without an explicit Teerapat instruction, a rationale, an Issue, and an `AGENTS.md` update; short names; no redundant comments; TDD and real command evidence; research the rulebook and prior art before implementation; private repository; no registration, competition submission, contact with organizers or another team, or spending. No code, simulation, CI, toolchain pin, or unverified rule is added here.

`AGENTS.md` now requires English for all repository prose. Translating the current files did not by itself lock that rule.

Pull request: https://github.com/clytie0x/robocup-home/pull/1
Branch: `cursor/docs-conventions-bootstrap-546a`

### Merge readiness

Pull request 1 is authorized for merge. This cloud agent does not merge it and does not touch hardware, code, or CI.

Historical evidence from this cloud-agent environment only: after `f8ee395008c95e985b83ce33547aa9b8680755a7` was pushed, this agent's pull-request tool was called with action `merge` and refused. Its actions were `create_pr`, `update_pr`, `post_comment`, `resolve_comment`, `get_ci_status`, and `set_pr_status`. Its check read reported no CI, which is not a green result. The pull request stayed open and `main` stayed `338dd0b70568dbf6a9d7e065f97e3e035f03f15b`. That refusal does not mean no sanctioned merge tool exists.

Current status: the coordinator has found first-party `cursor-github.merge_pull_request`. This pull request is ready for the coordinator to check GitHub mergeability and checks, then attempt that sanctioned merge. Do not bypass branch protection or a pending or failing required check.

Checks read before the English commit, on the previous head `452dc4b`:

```text
gh pr checks 1
no checks reported on the 'cursor/docs-conventions-bootstrap-546a' branch

gh api repos/clytie0x/robocup-home/commits/452dc4bb6ea8354b276bb1db659bb9fa88317243/status
{"state":"pending","statuses":[],"total":0}

gh api repos/clytie0x/robocup-home/commits/452dc4bb6ea8354b276bb1db659bb9fa88317243/check-runs
{"runs":[],"total":0}

gh api repos/clytie0x/robocup-home/rulesets
[]

gh api repos/clytie0x/robocup-home/rules/branches/main
[]

gh api repos/clytie0x/robocup-home/branches/main/protection
HTTP 403 Resource not accessible by integration
```

No checks reported is not a green CI result. There is no CI workflow in the repository. Classic branch protection could not be read because the API returned 403. Ruleset and branch-rules reads returned empty lists.

The post-push read of the new commit is added only if it differs or once it has been run. See the section added with the English commit's own verification below.

## Historical record (2026-11-06 documentation bootstrap)

Superseded where it says not to merge. The facts of what was done stay.

Scope that day: one writer, no parallel writers. Files touched: `README.md`, `AGENTS.md`, `docs/tasks/3.1-carry-my-luggage.md`, `WORKLOG.md`. No runnable application, simulation, dependency setup, CI, or hardware procedure. Visibility was not changed.

Decisions:

- Conventions live in `AGENTS.md`, so `docs/conventions.md` was not created.
- The Rust toolchain version, edition, and CI baseline were not locked.
- Task 3.1 is an acceptance skeleton. Edition, conditions, scores, penalties, and time limits stayed unverified.
- No Issue was opened. The sanctioned write tool was pull requests only. No Issue-create tool was found, and `gh` was not used to create one. Existing Issue and pull-request lists were empty.
- One commit held the four files: `452dc4bb6ea8354b276bb1db659bb9fa88317243`.

### Privacy reads that day

Read only. No visibility change.

Before the edits:

```text
gh repo view --json name,nameWithOwner,visibility,isPrivate,url,description,defaultBranchRef
{"defaultBranchRef":{"name":"main"},"description":"RoboCup@Home sim-first software (start 3.1 Carry My Luggage) — Rust modules, laptop sim only","isPrivate":true,"name":"robocup-home","nameWithOwner":"clytie0x/robocup-home","url":"https://github.com/clytie0x/robocup-home","visibility":"PRIVATE"}

gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,default_branch:.default_branch,html_url:.html_url}'
{"default_branch":"main","html_url":"https://github.com/clytie0x/robocup-home","private":true,"visibility":"private"}
```

After the Thai documents, before that commit:

```text
gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,default_branch:.default_branch,html_url:.html_url,permissions:.permissions}'
{"default_branch":"main","html_url":"https://github.com/clytie0x/robocup-home","permissions":{"admin":false,"maintain":false,"pull":false,"push":false,"triage":false},"private":true,"visibility":"private"}

gh repo view --json nameWithOwner,isPrivate,visibility,url,description
{"description":"RoboCup@Home sim-first software (start 3.1 Carry My Luggage) — Rust modules, laptop sim only","isPrivate":true,"nameWithOwner":"clytie0x/robocup-home","url":"https://github.com/clytie0x/robocup-home","visibility":"PRIVATE"}
```

The `permissions` object was a read of the `gh` token. It was not edited.

### Checks that day

`git show` of `README.md` at the parent compared with the first two lines of `README.md` through `cmp` printed `README_HEAD_PRESERVED`. The original two lines were not deleted.

```text
find . -type f -not -path './.git/*' | sort
./AGENTS.md
./docs/tasks/3.1-carry-my-luggage.md
./README.md
./WORKLOG.md
```

The same `cmp` printed `README_HEAD_PRESERVED`.

```text
gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,html_url:.html_url}'
{"html_url":"https://github.com/clytie0x/robocup-home","private":true,"visibility":"private"}

gh repo view --json nameWithOwner,isPrivate,visibility
{"isPrivate":true,"nameWithOwner":"clytie0x/robocup-home","visibility":"PRIVATE"}
```

A search for a toolchain version pin printed `no toolchain pin`.

```text
if [ -f Cargo.toml ] || [ -f rust-toolchain.toml ] || [ -d .github ]; then echo UNEXPECTED_CODE_OR_CI; else echo "no Cargo.toml, no rust-toolchain.toml, no .github; fmt/build/clippy/test/sim not applicable and not run"; fi
```

Result: `no Cargo.toml, no rust-toolchain.toml, no .github; fmt/build/clippy/test/sim not applicable and not run`

cargo fmt, build, clippy, test, and simulation were not run. There is no compile result to cite.

`gh issue list --state all --limit 20` and `gh pr list --state all --limit 20` printed no items at that time. Pull request 1 was opened afterward.

Branch: `cursor/docs-conventions-bootstrap-546a`, from `main` at `338dd0b70568dbf6a9d7e065f97e3e035f03f15b`.

### Limits recorded that day

- The RoboCup@Home rulebook and prior art were not researched, so no compliance claim was made.
- The traceability table had one row, result `unverified`, gate `unspecified`.
- No Issue, because no sanctioned Issue tool was available.
- No toolchain version was chosen.

### Next step written that day (superseded)

That note said Teerapat should review the pull request and that the documentation milestone must not merge it, then research the rulebook before any code. The do-not-merge sentence is superseded by the current merge rule above. Research before code is still required.

## English update checks

Run on the working tree before the English commit. `HEAD` was still `452dc4bb6ea8354b276bb1db659bb9fa88317243`.

```text
python3 Thai scan of every file outside .git, range U+0E00–U+0E7F
thai_line_count=0

markdown links
OK README.md -> AGENTS.md
OK README.md -> docs/tasks/3.1-carry-my-luggage.md
OK README.md -> WORKLOG.md
OK AGENTS.md -> WORKLOG.md
OK docs/tasks/3.1-carry-my-luggage.md -> ../../README.md
OK docs/tasks/3.1-carry-my-luggage.md -> ../../AGENTS.md
missing_links=0

tracked and untracked files outside .git
AGENTS.md
README.md
WORKLOG.md
docs/tasks/3.1-carry-my-luggage.md

cmp of the initial README (338dd0b) with the first two lines of README.md
README_ORIGINAL_TWO_LINES_PRESERVED

code and CI files
no Cargo.toml, no rust-toolchain.toml, no Cargo.lock, no .github; fmt/build/clippy/test/sim not applicable and not run

toolchain pin search
no toolchain pin

git diff --name-only HEAD
AGENTS.md
README.md
WORKLOG.md
docs/tasks/3.1-carry-my-luggage.md

gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,html_url:.html_url,default_branch:.default_branch}'
{"default_branch":"main","html_url":"https://github.com/clytie0x/robocup-home","private":true,"visibility":"private"}

gh repo view --json nameWithOwner,isPrivate,visibility,url
{"isPrivate":true,"nameWithOwner":"clytie0x/robocup-home","url":"https://github.com/clytie0x/robocup-home","visibility":"PRIVATE"}
```

fmt, build, clippy, test, and simulation were not run. Missing CI is not reported as green.

Post-push read of `f8ee395008c95e985b83ce33547aa9b8680755a7`, after that commit was already on the branch:

```text
gh pr checks 1
no checks reported on the 'cursor/docs-conventions-bootstrap-546a' branch
checks_exit:1

gh api repos/clytie0x/robocup-home/commits/f8ee395008c95e985b83ce33547aa9b8680755a7/status
{"state":"pending","statuses":[],"total":0}

gh api repos/clytie0x/robocup-home/commits/f8ee395008c95e985b83ce33547aa9b8680755a7/check-runs
{"runs":[],"total":0}

gh pr view 1 --json state,mergedAt,baseRefName,headRefOid,url
{"base":"main","head":"f8ee395008c95e985b83ce33547aa9b8680755a7","mergedAt":null,"state":"OPEN","url":"https://github.com/clytie0x/robocup-home/pull/1"}

gh api repos/clytie0x/robocup-home --jq '{private:.private,visibility:.visibility,default_branch:.default_branch}'
{"default_branch":"main","private":true,"visibility":"private"}

git ls-remote origin refs/heads/main
338dd0b70568dbf6a9d7e065f97e3e035f03f15b	refs/heads/main
```

The combined status `pending` has an empty status list. It is not a passing check suite. There is still no CI workflow to call green.
