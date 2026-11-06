# WORKLOG

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
