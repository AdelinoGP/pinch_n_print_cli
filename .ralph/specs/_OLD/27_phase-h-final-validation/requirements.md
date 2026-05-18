# Requirements: 27_phase-h-final-validation

## Problem Statement

After Packets 24, 25, and 26 land, a final validation gate is needed to: (1) rebuild any checked-in WASM artifacts whose live tests depend on changed bindings or manifests, (2) run the focused test matrix covering all six affected test files, (3) confirm workspace build and clippy pass, and (4) update `docs/07_implementation_status.md` to close TASK-120 and verify TASK-120b's live-evidence citation (added by packet 26) still references the real live support-module tests and the Benchy-with-tree-support acceptance check.

Note: the test file originally named `live_support_generation_tdd.rs` was deleted in commit `b6fb366` (2026-04-30) when the prepass support stage was renamed to `SupportGeometry`. Its live-dispatch tests now live in `crates/slicer-host/tests/live_layer_support_tdd.rs`.

## Grouped Task IDs

- TASK-120 (Produce fully sliced Benchy `.gcode` with tree supports enabled as Phase H acceptance)

## In-Scope

- WASM artifact rebuild via `modules/core-modules/build-core-modules.sh` (run via Git Bash on Windows or any POSIX shell on Linux/macOS)
- Focused test matrix run: `core_module_ir_access_contract_tdd`, `pipeline_tdd`, `wit_drift_detection_tdd`, `live_layer_support_tdd`, `live_seam_path_tdd`, `benchy_end_to_end_tdd`
- Workspace build verification
- Workspace clippy verification
- `docs/07_implementation_status.md` TASK-120 closure plus TASK-120b live-evidence verification

## Out-of-Scope

- Full workspace test suite (known slicer-cli-only failures are pre-existing and unrelated to this work)
- New feature development
- Broader doc changes beyond TASK-120 closure and TASK-120b verification

## Acceptance Summary

After this packet lands:
1. All checked-in WASM artifacts are rebuilt after binding/manifest changes.
2. All six focused test files pass.
3. `cargo build --workspace` exits 0.
4. `cargo clippy --workspace -- -D warnings` exits 0 with no warnings.
5. `docs/07_implementation_status.md` TASK-120b's live-evidence citation (added by packet 26) is still intact and names the live support-module tests.
6. Workstream 3 explicitly names the Benchy-with-tree-support acceptance tests (`benchy_with_support_enabled`, `benchy_support_marker_present`, `benchy_support_deterministic`).
7. TASK-120 is marked complete (`[x]`) and Phase H is no longer blocked on the live Benchy run.

## Verification

```
./modules/core-modules/build-core-modules.sh
cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture
cargo test -p slicer-host --test pipeline_tdd -- --nocapture
cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture
cargo test -p slicer-host --test live_layer_support_tdd -- --nocapture
cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture
cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture
cargo build --workspace
cargo clippy --workspace -- -D warnings
```
