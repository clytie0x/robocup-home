# robocup-home
RoboCup@Home sim-first software (start 3.1 Carry My Luggage) — Rust modules, laptop sim only

The repository has a Rust workspace. `sim_runtime` has a deterministic clock, a seeded world model, a pointing sample for one fixture bag, one simulated gripper, and a simulated base. `perception` selects a visible bag from that sample. `manipulation` decides a pick, drop, or handoff for that single gripper. `navigation` proposes one bounded step from a local view of the operator and obstacle disks. `task_3_1` is a pure task state machine and records the printed sheet without awarding a run total. `harness` posts the selection, confirms the report, confirms a gripper action only after the adapter and the trip agree, and runs one seeded episode on one clock from the report through the optional return. The car coordinate stays in the scenario. There is no hardware procedure. The model is not a claim of physical accuracy.

`clytie0x/robocup-home` is public. The owner's instruction and the API read are in [WORKLOG.md](WORKLOG.md). That applies to this repository only. Do not change this visibility again without a new instruction.
The toolchain pin is stable 1.99.1 in [rust-toolchain.toml](rust-toolchain.toml). The edition is 2024. `rust-version` matches that pin. CI is [.github/workflows/ci.yml](.github/workflows/ci.yml).

M8 is merged at `b446ae9b59cd9543c1c321dcbd461f2dc1586de9`. The acceptance table has 45 `pass` rows and 11 `not-run` rows. A `pass` is a labeled model, catalog, or assumption check in [the task specification](docs/tasks/3.1-carry-my-luggage.md). It is not a field score and not a hardware result. The 11 `not-run` rows are `CML-BAG-DEF` (the fixture stores the bag kind and does not measure stiffness or weight), the seven open scoring rows (`CML-GEN-MAIN`, `CML-SHEET-START`, `CML-OPEN-PARTIAL`, `CML-OPEN-ABSENT`, `CML-OPEN-CAP`, `CML-OPEN-DROP`, `CML-OPEN-FOLLOW`), and the three hardware rows (`CML-HW-OUT`, `CML-HW-HIT`, `CML-HW-SPL`). There is no awarded total. M9.3 ran on `298104e381a4fc101394446bd4e3e0fa581a3cb2`, and the missing-PDF fix is `2e8619159ccafc17f1dd53e52cf53c67f34b9484`. One finding was fixed. One saturated-integer route probe was dropped as preexisting and outside this task. No verified finding remains. That is not full acceptance, and it does not say that no defect remains. The record is in [WORKLOG.md](WORKLOG.md).

The professor PDF is the rule. Cover 2024 Rev-1, revision 2024-07-09. SHA-256 `da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600`. The raw PDF is not in git. Public 2024.2 and public 2025.1 are not the baseline. `artifacts/check_m1_spec.py` checks that a `pass` or `fail` row names a test whose body contains the row id. It does not judge the meaning of the assertions. CI does not have the PDF. With `PROFESSOR_PDF` unset and the optional fallback file absent, the checker prints `pdf=False` and exits 0. A set path that is missing, not a file, unreadable, or a different file is an error and does not print `pdf=True`. `pdf=True` is printed only after the opened file matches the pinned hash.

From the repository root, with the pinned toolchain:

```text
cargo fmt --all -- --check
cargo build --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo test --workspace --all-targets --locked --release
cargo test --workspace --doc --locked --release
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
python3 artifacts/check_workspace.py
python3 artifacts/check_m1_spec.py --self-test
python3 artifacts/check_m1_spec.py
PROFESSOR_PDF=/path/to/the/professor.pdf python3 artifacts/check_m1_spec.py
cargo test -p harness --lib --locked
```

The last command is the seeded episode set. The counts from the latest run are in [WORKLOG.md](WORKLOG.md). Abreast 340 mm, the 280 mm close band, the 40 mm route margin, the 800 mm and 2500 mm sensors, the disk radii, the 250 mm robot disk, the arena half-plane, and the three-step stay are assumptions, not official measurements.

Crates live under `crates/`. `domain` is the one shared contract between task logic and adapters. The name `task_api` is that role. `task_3_1` depends on `domain` and the capability crates. `task_3_2` depends on `domain`. `sim_runtime` depends on `domain`. `harness` may depend on `sim_runtime`. `task_3_1` does not. `task_3_2` does not. No other crate depends on `sim_runtime`.

M9 is merged at `d6d9940820e92c7b8b8e5c290a4a45b703b90a11`. M10 adds `task_3_2` for the section 3.2 door-open move. `DoorOpen` emits `GoToPoint` and carries no coordinate. The section 3.2 inventory rows stay `not-run`. That enum is not a completed General Purpose Service Robot run.

- Locked conventions: [AGENTS.md](AGENTS.md)
- Task 3.1 Carry My Luggage acceptance specification, from the professor PDF dated 2024-07-09: [docs/tasks/3.1-carry-my-luggage.md](docs/tasks/3.1-carry-my-luggage.md)
- Section 3.2 design inventory, from the same professor PDF: [docs/tasks/3.2-general-purpose-service-robot.md](docs/tasks/3.2-general-purpose-service-robot.md)
- Verification log: [WORKLOG.md](WORKLOG.md)
