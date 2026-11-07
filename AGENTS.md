# Working conventions — robocup-home

Long-term conventions for Teerapat Vatpitak and the team, for people and agents.
This file is the only conventions document, so there is no `docs/conventions.md`.
The repository has a Rust workspace. `sim_runtime` has a deterministic clock, a seeded world model, a pointing sample for one fixture bag, one simulated gripper, and a simulated base. `perception` selects a visible bag from that sample. `manipulation` decides a pick, drop, or handoff for that single gripper. `physical` reads posted captures and keeps disagreeing sensor profiles side by side. It does not replace that gripper and it does not validate an arm mount. `navigation` proposes one bounded step from a local view of the operator and obstacle disks. `task_3_1` is a pure task state machine and records the printed sheet without awarding a run total. `harness` posts the selection, confirms the report, confirms a gripper action only after the adapter and the trip agree, and runs one seeded episode on one clock from the report through the optional return. The car coordinate stays in the scenario. There is no awarded total and no hardware procedure. The model is not a claim of physical accuracy.

On 2026-11-06 at about 15:14 Asia/Bangkok, the owner instructed that `clytie0x/robocup-home` be made public and that CI be rerun. That instruction replaces the earlier private-only rule for this repository. It is not permission to publish any other repository, and it is not permission to change this visibility again without a new instruction. A later API read found this repository public. The refused attempt and that read are in [WORKLOG.md](WORKLOG.md).

## Current rules

Where an older sentence conflicts with this section, this section wins.
The older sentence stays in git history and is labelled superseded in [WORKLOG.md](WORKLOG.md).

### Merge

On 2026-11-06 at 13:50 Asia/Bangkok, Teerapat allowed Grok Bot and RoboCup Lab to merge this repository without a fresh confirmation for each merge.

The path is still Issue, then branch, then pull request, then `main`.
Use sanctioned Issue and pull-request tools. If no sanctioned tool can open an Issue, do not open one by another route. Record that limit and continue with the tools that exist.
A merge still has to go through the sanctioned merge tool. Do not bypass branch protection, a pending or failing required check, or a merge tool that is missing or that refuses the merge.

Superseded: the first documentation commit said to stop at the pull request and not merge. That was the instruction for that milestone. It is not the current rule.

### Hardware

The default is a laptop and simulation. Do not assume a physical device is available.
Deploying and flashing are not categorically forbidden. No hardware is connected, so Jetson, arm, and Pi stay untouched until a task specifically authorizes and connects that device.
This milestone adds no deploy, flash, or other hardware steps.
A simulation result is not a hardware result.

### Still in force

- Write all repository prose in English: `README.md`, `AGENTS.md`, task docs, `WORKLOG.md`, any necessary code comments, and future commit messages and pull-request titles and bodies.
- Rust is the primary language.
- Do not change stack, language, or style unless Teerapat gives an explicit instruction. That change needs the instruction, a rationale, an Issue, and an update to this file.
- Modules stay separate: perception, navigation, manipulation, HRI, and task.
- Workspace members are `domain`, `perception`, `navigation`, `manipulation`, `hri`, `task_3_1`, `task_3_2`, `sim_runtime`, `physical`, and `harness`. `domain` is the one shared contract. The name `task_api` is that role. `task_3_1` depends on `domain` and the capability crates. `task_3_2` depends on `domain`. `sim_runtime` depends on `domain`. `harness` is the integration crate and may depend on `sim_runtime`. `task_3_1` does not depend on `sim_runtime`. `task_3_2` does not depend on `sim_runtime`. No other crate depends on `sim_runtime`. `physical` depends on no workspace crate. No other crate depends on `physical`. The 2026-11-07 foundation instruction added `physical`. No sanctioned Issue tool was available, so that limit is recorded in [WORKLOG.md](WORKLOG.md).
- Adapters keep the simulator out of task logic. Logic must not call a simulator directly.
- OpenCV and large pretrained models may be direct dependencies.
- Rewrite any other dependency that is old or unmaintained as Rust code this repository owns.
- The Rust toolchain is pinned to stable 1.99.1 in `rust-toolchain.toml`, with rustfmt and clippy. The edition is 2024. `rust-version` is 1.99.1, the same pin. A channel change needs a new instruction, a rationale, an Issue, and an update to this file. Sources are in [WORKLOG.md](WORKLOG.md).
- Use short, clear names.
- Keep diffs small. One commit changes one behavior.
- Do not add comments that repeat the code. Put a needed rationale in short documentation.
- Keep text that law or safety requires.
- Do not delete a file, branch, or the repository without the owner's approval.
- Do not register for an event, submit competition results, contact organizers or another team, or spend money.

### Quality locks

- Do not treat a milestone as complete while a sourced row is missing, or while a conflict is written as a pass.
- A known software defect stays on a software row. Do not close it by moving it to hardware.
- Do not claim that no undiscovered defect remains.
- Interrogate the milestone set before M9 closure. The Carry My Luggage specification does not perform that interrogation and does not claim it.

## Tests, once code exists

TDD order is red, then green, then refactor, on three layers: unit, scenario simulation, and adapter contract.
Evidence has to come from a real fmt, build, clippy, test, or simulation command.
A documentation-only change has no compile result. Do not invent one.
Missing CI or test infrastructure is not a pass.
Review the evidence before calling the work complete, and do not claim more than the command that just ran.

Each task keeps a traceability table with the columns requirement, source, test, and result.
Add a gate column: `sim`, `hardware`, or `unspecified`.
`unspecified` is allowed only while the requirement is `unverified` and its layer is still unknown.
Allowed results are `unverified`, `not-run`, `pass`, and `fail`.
`pass` and `fail` must name the command and the output.
Do not write that every condition passed unless every sourced row exists and each of those rows has real evidence.

## Loop, once code is authorized

plan, then implement, then compile, then test each sourced condition, then tighten, then verify.

- Research the authoritative rulebook and prior art before implementation. An unsourced requirement stays unresolved.
- Give each condition a source before writing its test and its code.
- Include cases that should fail, and boundary cases.
- Record a reproducible simulation seed in the test column.
- A simulation pass does not close a hardware row.
- A hardware row stays `unverified` or `not-run` until that task's device is authorized and connected.
