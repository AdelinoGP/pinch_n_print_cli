# Implementation Plan: wipe-tower-finalization-live-path

## Execution Rules

- One atomic step at a time.
- Port the module surface first, then prove the host path is retired from the legacy helper.

## Steps

### Step 1: Add failing module tests for `run_finalization()` wipe-tower pushes

- Task IDs:
  - `TASK-143`
- Objective:
  Freeze the exact finalization-builder expectations for wipe-tower pushes, purge-volume scaling, and layer targeting.
- Precondition:
  The module still only has live geometry on `process()`.
- Postcondition:
  `finalization_live_tdd.rs` exists with failing `run_finalization()` assertions.
- Files expected to change:
  - `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`
- Verification:
  - `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_pushes_wipe_tower_entities_for_tool_change_layers -- --exact --nocapture`
  - `cargo test -p wipe-tower --test finalization_live_tdd purge_volume_controls_finalization_push_count -- --exact --nocapture`
  - `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_targets_only_layers_with_tool_changes -- --exact --nocapture`
  - `cargo test -p wipe-tower --test finalization_live_tdd disabled_or_no_tool_changes_emit_no_finalization_pushes -- --exact --nocapture`
- Exit condition:
  Focused module tests exist and fail only because `run_finalization()` is still a no-op.

### Step 2: Port wipe-tower purge logic onto `run_finalization()`

- Task IDs:
  - `TASK-143`
- Objective:
  Implement `run_finalization()` using `LayerCollectionView` and `FinalizationOutputBuilder`, preserving the existing purge logic.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  Module-level wipe-tower finalization tests are green.
- Files expected to change:
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs`
- Authoritative docs:
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.hpp`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`
- Verification:
  - `cargo test -p wipe-tower --test finalization_live_tdd -- --nocapture`
- Exit condition:
  All module-level finalization tests pass.

### Step 3: Add a live host merge regression and retire the legacy path

- Task IDs:
  - `TASK-143`
- Objective:
  Prove the real host finalization path now consumes `run_finalization()` output and no longer depends on the legacy helper.
- Precondition:
  Module-level finalization tests are green AND `wipe-tower.wasm` has been rebuilt via `./modules/core-modules/build-core-modules.sh`.
- Postcondition:
  A host integration test proves `LayerCollectionIR` receives merged wipe-tower entities from finalization output.
- Files expected to change:
  - `crates/slicer-host/tests/finalization_live_tdd.rs` — This file already exists from packet 16. Step 3 adds one new test function: `live_finalization_dispatch_merges_wipe_tower_entity_pushes`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`
- Verification:
  - `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_wipe_tower_entity_pushes -- --exact --nocapture`
- Exit condition:
  The host finalization merge regression passes.
- Notes:
  The host test uses the real `wipe-tower.wasm` artifact (not `sdk-finalization-guest`). The test must pass `wipe_tower_enabled = true` in the module config and supply a layer with at least one `ToolChange` entry to trigger `run_finalization()` output.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `./modules/core-modules/build-core-modules.sh` run to rebuild `wipe-tower.wasm` with the new `run_finalization()` implementation.
- Confirm the live WASM path emits wipe-tower entities through `run_finalization()` and not the legacy `process()` path.
- `docs/07_implementation_status.md` updated for `TASK-143`.
- `docs/DEVIATION_LOG.md` updated to reflect DEV-013 progress.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm the host no longer requires the legacy `process()` path for WipeTower finalization.
- Record any remaining packet-local risk before status changes.