# Requirements: 27_phase-h-final-validation

## Problem Statement

After Packets 24, 25, and 26 land, a final validation gate is needed to: (1) rebuild any checked-in WASM artifacts whose live tests depend on changed bindings or manifests, (2) run the focused test matrix covering all six affected test files, (3) confirm workspace build and clippy pass, and (4) update `docs/07_implementation_status.md` with accurate TASK-120/TASK-120b closure evidence that reflects the real live support-module coverage and true Benchy-with-tree-support acceptance check.

## Grouped Task IDs

- TASK-120 (Produce fully sliced Benchy `.gcode` with tree supports enabled as Phase H acceptance)

## In-Scope

- WASM artifact rebuild via `modules/core-modules/build-core-modules.sh`
- Focused test matrix run: `core_module_ir_access_contract_tdd`, `pipeline_tdd`, `wit_drift_detection_tdd`, `live_support_generation_tdd`, `live_seam_path_tdd`, `benchy_end_to_end_tdd`
- Workspace build verification
- Workspace clippy verification
- `docs/07_implementation_status.md` TASK-120/TASK-120b status updates

## Out-of-Scope

- Full workspace test suite (known slicer-cli-only failures are pre-existing and unrelated to this work)
- New feature development
- Broader doc changes beyond TASK-120/TASK-120b status notes

## Acceptance Summary

After this packet lands:
1. All checked-in WASM artifacts are rebuilt after binding/manifest changes.
2. All six focused test files pass.
3. `cargo build --workspace` exits 0.
4. `cargo clippy --workspace -- -D warnings` exits 0 with no warnings.
5. `docs/07_implementation_status.md` TASK-120b cites real live support-module evidence.
6. Workstream 3 explicitly tracks the new true Benchy-with-tree-support acceptance check.

## Verification

```
./modules/core-modules/build-core-modules.sh
cargo test -p slicer-host --test core_module_ir_access_contract_tdd -- --nocapture
cargo test -p slicer-host --test pipeline_tdd -- --nocapture
cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture
cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture
cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture
cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture
cargo build --workspace
cargo clippy --workspace -- -D warnings
```
