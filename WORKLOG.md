# WORKLOG

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
