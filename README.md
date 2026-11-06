# robocup-home
RoboCup@Home sim-first software (start 3.1 Carry My Luggage) — Rust modules, laptop sim only

The repository has a Rust workspace. `sim_runtime` has a deterministic clock, a seeded world model, a pointing sample for one fixture bag, one simulated gripper, and a simulated base. `perception` selects a visible bag from that sample. `manipulation` decides a pick, drop, or handoff for that single gripper. `navigation` proposes one bounded step from a local view of the operator and obstacle disks. `task_3_1` is a pure task state machine. `harness` posts the selection, confirms the report, confirms a gripper action only after the adapter and the trip agree, and on the navigation path keeps the world clock and the trip stamp on one count. There is no scorer and no hardware procedure. The model is not a claim of physical accuracy.

`clytie0x/robocup-home` is public. The owner's instruction and the API read are in [WORKLOG.md](WORKLOG.md). That applies to this repository only. Do not change this visibility again without a new instruction.
The toolchain pin is stable 1.99.1 in [rust-toolchain.toml](rust-toolchain.toml). The edition is 2024. `rust-version` matches that pin. CI is [.github/workflows/ci.yml](.github/workflows/ci.yml).

Crates live under `crates/`. `domain` is the one shared contract between task logic and adapters. The name `task_api` is that role. `task_3_1` depends on `domain` and the capability crates. `sim_runtime` depends on `domain`. `harness` may depend on `sim_runtime`. `task_3_1` does not. No other crate depends on `sim_runtime`.

- Locked conventions: [AGENTS.md](AGENTS.md)
- Task 3.1 Carry My Luggage acceptance specification, from the professor PDF dated 2024-07-09: [docs/tasks/3.1-carry-my-luggage.md](docs/tasks/3.1-carry-my-luggage.md)
- Verification log: [WORKLOG.md](WORKLOG.md)
