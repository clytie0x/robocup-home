# WORKLOG

## Visibility instruction (2026-11-06)

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

Privacy, read only:

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
