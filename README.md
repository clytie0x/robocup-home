# robocup-home
RoboCup@Home sim-first software (start 3.1 Carry My Luggage) — Rust modules, laptop sim only

The repository has a Rust workspace skeleton. There is no task behavior, no simulated world, and no hardware procedure.

`clytie0x/robocup-home` is private. Do not change its visibility.
The toolchain pin is stable 1.99.1 in [rust-toolchain.toml](rust-toolchain.toml). The edition is 2024. `rust-version` matches that pin. CI is [.github/workflows/ci.yml](.github/workflows/ci.yml).

Crates live under `crates/`. `domain` is the one shared contract between task logic and adapters. The name `task_api` is that role. `task_3_1` depends on `domain` and the capability crates. `sim_runtime` depends on `domain`. No crate depends on `sim_runtime`.

- Locked conventions: [AGENTS.md](AGENTS.md)
- Task 3.1 Carry My Luggage acceptance specification, from the professor PDF dated 2024-07-09: [docs/tasks/3.1-carry-my-luggage.md](docs/tasks/3.1-carry-my-luggage.md)
- Verification log: [WORKLOG.md](WORKLOG.md)
