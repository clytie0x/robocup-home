# WORKLOG

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
