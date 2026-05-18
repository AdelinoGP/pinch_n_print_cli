# Task Map: 03_wit-canonical-source-and-validation

Use this file because the packet spans three task IDs and reopens no prior packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-144` | Step 1 (audit) | `docs/03_wit_and_manifest.md` | `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/wit_host.rs` | None | Inventory of all 7 inline WIT copies. Read-only discovery. |
| `TASK-144` | Step 2 (path resolution) | `crates/slicer-macros/Cargo.toml` | `crates/slicer-macros/src/lib.rs` | None | Verify `include_str!` path from proc-macro to `wit/`. |
| `TASK-144` | Step 3 (macro consolidation) | `docs/03_wit_and_manifest.md` | `crates/slicer-macros/src/lib.rs` | None | Replace 4 `build_*_world_glue` inline WIT strings with `include_str!`. |
| `TASK-144` | Step 4 (host consolidation) | `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/wit_host.rs`, `dag.rs`, `execution_plan.rs` | None | Replace host inline WIT; fix package names. |
| `TASK-145` | Step 5 (missing members) | `docs/03_wit_and_manifest.md` | `wit/deps/ir-types.wit`, `wit/world-postpass.wit` | None | Add `needs-support` to ir-types; add `push-z-hop` to postpass world. |
| `TASK-145` | Step 7 (drift detection) | `docs/03_wit_and_manifest.md` | `crates/slicer-host/tests/wit_drift_detection_tdd.rs` | None | Regression test preventing future WIT drift. |
| `TASK-146` | Step 6 (allowlist validation) | `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/manifest.rs`, `crates/slicer-host/src/dag.rs` or `module_load.rs` | None | Reject non-allowlisted `wit_world` at module-load with fatal diagnostic. |
| `TASK-145` | Step 8 (workspace gate) | — | Workspace-wide | None | `cargo build --workspace && clippy` — final consolidation gate. |
