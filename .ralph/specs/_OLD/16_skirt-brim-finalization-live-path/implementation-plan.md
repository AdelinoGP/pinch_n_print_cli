# Implementation Plan: skirt-brim-finalization-live-path

## Execution Rules

- One atomic step at a time.
- Port the module surface first, then prove the host no longer depends on the legacy path.

## Steps

### Step 1: Add failing module tests for `run_finalization()` skirt and brim pushes

- Task IDs:
  - `TASK-142`
- Objective:
  Freeze the exact finalization-builder expectations for skirt, brim, and height targeting.
- Precondition:
  The module still only has live geometry on `process()`.
- Postcondition:
  `finalization_live_tdd.rs` exists with failing `run_finalization()` push assertions.
- Files expected to change:
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`
- Verification:
  - `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_skirt_entities_to_target_layers -- --exact --nocapture`
  - `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_brim_entities_on_layer_zero_only -- --exact --nocapture`
  - `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_respects_skirt_height_layer_targeting -- --exact --nocapture`
  - `cargo test -p skirt-brim --test finalization_live_tdd disabled_or_empty_input_emits_no_finalization_pushes -- --exact --nocapture`
- Exit condition:
  The three positive tests (`run_finalization_pushes_skirt_entities_to_target_layers`,
  `run_finalization_pushes_brim_entities_on_layer_zero_only`,
  `run_finalization_respects_skirt_height_layer_targeting`) exist and fail because
  `run_finalization()` still does nothing. The negative test
  (`disabled_or_empty_input_emits_no_finalization_pushes`) exists and passes — the
  default no-op is already correct behavior for disabled or empty inputs.

### Step 2: Port the geometry helpers onto `run_finalization()`

- Task IDs:
  - `TASK-142`
- Objective:
  Implement `run_finalization()` using `LayerCollectionView` and `FinalizationOutputBuilder`, preserving the existing geometry behavior.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  The module-level skirt, brim, and height-targeting tests pass.
- Files expected to change:
  - `modules/core-modules/skirt-brim/src/lib.rs`
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs`
- Authoritative docs:
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Brim.hpp`
- Verification:
  - `cargo test -p skirt-brim --test finalization_live_tdd -- --nocapture`
- Exit condition:
  Module-level finalization tests are green.

### Step 3: Add a live host finalization merge regression

- Task IDs:
  - `TASK-142`
- Objective:
  Prove the real host finalization path now consumes `run_finalization()` output instead
  of relying on the legacy helper. Update `dispatch.rs` so finalization entity pushes are
  **prepended** before existing ordered entities (matching legacy `process()` ordering
  where skirt and brim appear before model entities). Record DEV-013 partial progress.
- Precondition:
  Module-level finalization tests are green.
- Postcondition:
  A host integration test proves `LayerCollectionIR` receives merged skirt/brim entities
  from finalization output and those entities appear before the original model entities.
  `docs/DEVIATION_LOG.md` notes that `SkirtBrim` is ported and `WipeTower` remains open.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs` (change entity-push merge from `ordered_entities.push()` to batch-prepend so finalization entities precede model entities)
  - `crates/slicer-host/tests/finalization_live_tdd.rs`
  - `docs/DEVIATION_LOG.md`
- Implementation note:
  Collect all `EntityToLayer` pushes for each target layer before inserting them, then
  prepend the whole group at once via `ordered_entities.splice(0..0, collected_pushes)`.
  Do **not** call `insert(0, entity)` in a loop — that reverses the emission order of
  entities within the same layer. The batch-splice preserves the order the guest emitted
  them while placing the entire finalization group before the original model entities.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`
- Verification:
  - `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_skirt_brim_entity_pushes -- --exact --nocapture`
- Exit condition:
  The host finalization merge regression passes.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-142`.
- `docs/DEVIATION_LOG.md` updated to reflect DEV-013 progress.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm the host no longer requires the legacy `process()` path for SkirtBrim finalization.
- Record any remaining packet-local risk before status changes.