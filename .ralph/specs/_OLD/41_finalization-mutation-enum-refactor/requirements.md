# Requirements: finalization-mutation-enum-refactor

## Packet Metadata

- Grouped task IDs:
  - `TASK-172` (NEW)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Supersedes: none
- Depends on: `40_finalization-mutation-builder` (`implemented` required; closed 2026-05-07)
- Unblocks: future PostPass mutation modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`)

## Problem Statement

Packet `40_finalization-mutation-builder` shipped a four-method WIT surface for finalization-stage mutation: `push_entity_with_priority`, `modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`. Three of those four were authored with **closure-typed** SDK signatures that cannot round-trip across the WIT boundary. The macro-generated `slicer-macros::run_finalization` drain-back loop forwards `priority_pushes` (the `push_entity_with_priority` records) but discards `merge_ops` (the closure-bearing recordings of the other three). The host-side `apply_to` then runs against an empty `merge_ops` Vec.

Result: a WASM finalization module that calls `output.modify_entity(layer, id, |e| e.path.speed_factor = 0.5)` records a closure in WASM-side memory; the macro drain-back skips it; the host applies nothing. **No error, no warning, no diagnostic** — the call vanishes. This is a contract lie: the WIT surface advertises `modify-entity` as a callable method, but invoking it from a guest module is silently a no-op.

`DEV-041` registered this gap as an "open" deviation with the rationale that no WASM module in packet 40 exercises the three closure-typed methods. That rationale has expired:

1. The macro-generated guest-side glue (Step 3b-fix in packet 40) now exposes the methods to every WASM finalization module that gets built. Future module authors will reach for them. The first one to use `modify_entity` will ship a silent regression — speed-modulation that doesn't take effect, layer-time enforcement that does nothing, flush-volume calculation that emits but never applies.
2. The four future PostPass modules listed in packet 40's design.md `## Open Questions` (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) are explicitly the consumers of these methods. Each was deferred under the assumption that "the API exists, just go build the module." That assumption is wrong today.

The fix is structural, not patchwork. Closures cannot cross WIT; the SDK API must be reshaped to take the same serializable values the WIT carries. This packet:

1. Defines three new types in `slicer-sdk`:
   - `EntityMutation` — enum with concrete `Set*` variants for the PrintEntity fields a near-future module will mutate. Exact variant list locked at Step 0 after auditing future-module design intent.
   - `SortKey` — enum with concrete sort-key variants. Exact list locked at Step 0.
   - `SyntheticLayerData` — record with the minimal `(z, paths)` payload sufficient to author a synthetic layer; host fills sibling fields.
2. Replaces the SDK closure-typed signatures with enum-typed ones that match the existing WIT shapes (introduced in packet 40 Step 3b).
3. Refactors the SDK's internal `MergeOp` enum to carry only serializable variants; removes `Box<dyn FnOnce>` / `Box<dyn FnMut>` storage.
4. Updates `apply_to` to translate each new `MergeOp` variant directly to a concrete in-place mutation.
5. Migrates the 8 existing `crates/slicer-sdk/tests/finalization_builder_tdd.rs` tests from closure-form to enum-form.
6. Extends the `slicer-macros` drain-back loop to forward `merge_ops` via WIT.
7. Adds a tiny new test guest at `test-guests/finalization-mutation-roundtrip-guest/` and a host-side end-to-end test that proves a guest's `modify_entity` call actually mutates the host IR — the substantive validation absent today.
8. Closes `DEV-041` in `docs/DEVIATION_LOG.md` (the live registry; the row currently sits at line 47). The legacy `docs/14_deviation_audit_history.md` is an archive and is not edited.

The result: WASM modules and native test fixtures share one API shape. The drain-back loop is straight-line forwarding, no impedance mismatch. The four future PostPass modules can be authored against a contract that actually delivers what it promises.

## In Scope

- `crates/slicer-sdk/src/traits.rs` (or wherever `FinalizationOutputBuilder` and `MergeOp` are defined post-Packet-40 — Step 0 confirms exact lines):
  - Define `pub enum EntityMutation { … }` with concrete `Set*` variants.
  - Define `pub enum SortKey { … }` with concrete variants.
  - Define `pub struct SyntheticLayerData { z: f32, paths: Vec<ExtrusionPath3D> }`.
  - Replace the three closure-typed methods on `FinalizationOutputBuilder` with the enum-typed forms.
  - Refactor `MergeOp` to plain variants (`ModifyEntity { layer, entity_id, mutation: EntityMutation }`, `SortLayer { layer, key: SortKey }`, `InsertSynthLayer { idx, data: SyntheticLayerData }`).
  - Update `apply_to` to consume the new `MergeOp` shape.
  - Re-export the new types from `crates/slicer-sdk/src/lib.rs` if necessary.
- `crates/slicer-sdk/tests/finalization_builder_tdd.rs`:
  - Migrate the existing 8 tests from closure-form to enum-form.
  - Add new tests for the explicit `Set*` variants: AC-1 verifying path-level `speed_factor` and AC-2 verifying per-point `flow_factor` (the volumetric lever; `ExtrusionPath3D` carries no path-level `extrusion_width_factor` field today, so a `SetExtrusionWidthFactor` variant was considered and rejected for this packet).
  - Add a `closure_api_is_fully_removed` regression test that grep-asserts the closure-bound generic signatures are absent (NEG-4 / closure-removal contract).
- `wit/world-finalization.wit`:
  - Reconcile names with the new SDK types if drift is found. Packet 40 already added `entity-mutation`, `sort-key`, `synthetic-layer-data` shapes; verify they align with this packet's `EntityMutation`/`SortKey`/`SyntheticLayerData` and rename if needed for clarity.
- `crates/slicer-host/src/wit_host.rs`:
  - Simplify the `HostFinalizationOutputBuilder` impl methods for `modify_entity` / `sort_layer_by` / `insert_synthetic_layer_after` — they now forward directly (no closure construction needed).
- `crates/slicer-macros/src/lib.rs`:
  - Inline WIT in `build_finalization_world_glue` — confirm signatures match the canonical `wit/world-finalization.wit`. Update if drift.
  - `run_finalization` drain-back loop (around lines 1198–1214) — extend to iterate `sdk_output.merge_ops()` after the existing `priority_pushes()` loop. For each `MergeOp` variant, call the corresponding WIT method on the bound `output` resource. Remove any catch-all that drops `merge_ops`.
- `test-guests/finalization-mutation-roundtrip-guest/` (NEW crate):
  - `Cargo.toml` modeled on existing `test-guests/sdk-finalization-guest/Cargo.toml`.
  - `src/lib.rs` implementing `FinalizationModule` with a `run_finalization` body that calls `output.modify_entity(layer, 1, EntityMutation::SetSpeedFactor(0.5))` and (in a separate exported entry point or via a config flag) `modify_entity(layer, 99, …)` for the unknown-id NEG case.
  - Manifest entry so the host can discover and load the guest from a test-only path.
- `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` (NEW):
  - Host-side end-to-end test that loads the new guest, runs the pipeline against a minimal fixture, and asserts the in-memory IR shows the mutation.
  - At least 3 tests: `modify_entity_round_trips_through_wit`, `modify_entity_unknown_id_round_trips_error`, `drain_back_forwards_merge_ops` (the last is a code-shape assertion via grep or a no-op-merge-op canary that proves the macro change took effect).
- `docs/07_implementation_status.md`:
  - Insert one new row for `TASK-172`.
- `docs/DEVIATION_LOG.md`:
  - Update the existing `DEV-041` row at line 47 — change Status column from `Open` to `Closed YYYY-MM-DD` (date at packet acceptance) with a one-paragraph closure note. The legacy `docs/14_deviation_audit_history.md` is an archive only and is NOT touched.

## Out of Scope

- Implementing any of the four future PostPass modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`). They each consume this packet's APIs but are each their own future packet.
- Adding new `ExtrusionRole` variants.
- Adding a `Custom(Vec<u8>)` / `Patch(serde_json::Value)` escape-hatch variant to `EntityMutation`. Defer until a real motivating consumer surfaces.
- Mirroring full `LayerCollectionIR` field set into `SyntheticLayerData`. Minimal `(z, paths)` only; expand when a real consumer needs more.
- Changes to the host's per-layer merge sequence in `crates/slicer-host/src/dispatch.rs`. Packet 40 Step 4 is closed; the dispatch.rs apply_to call site stays put.
- Changes to skirt-brim, wipe-tower, top-surface-ironing module sources. None consume `merge_ops`; all three were migrated in Packet 40 follow-up (skirt-brim and wipe-tower) or were already builder-using (top-surface-ironing).
- Adding a runtime guard in slicer-macros for a non-empty `merge_ops` Vec ahead of this packet. The user explicitly chose to skip the bridge guard.
- Producer-emitted entity reordering, role-priority changes.
- Persistent / config-driven mutation registries.
- Refactoring `push_entity_with_priority` (the one closure-free method from packet 40 — it stays as is).

## Authoritative Docs

- `docs/01_system_architecture.md` lines 328–363 — `PostPass::LayerFinalization` mutability contract. Direct read.
- `docs/03_wit_and_manifest.md` — WIT surface conventions for the `world-finalization` shape. Delegate SUMMARY if > 300 lines.
- `docs/05_module_sdk.md` — `FinalizationOutputBuilder` API description. Delegate SUMMARY for any section that documents closure bounds (those need updating in a future docs packet, OOS here).
- `docs/02_ir_schemas.md` — `PrintEntity`, `ExtrusionPath3D`, `LayerCollectionIR`, `TravelMove` shapes. Direct read; narrow.
- `docs/04_host_scheduler.md` lines 309–317, 680–717 — composable multi-writer patterns; PostPass scheduler shape. Direct read.
- `.ralph/specs/40_finalization-mutation-builder/design.md` — predecessor's "Open Questions" + future-module list. Direct read; narrow.
- `docs/DEVIATION_LOG.md` — `DEV-041` entry (line 47). Direct read; narrow. The legacy `docs/14_deviation_audit_history.md` is the archive and does NOT carry the live DEV-041 row.

## OrcaSlicer Reference Obligations

None required. If parity is challenged for the speed/flow factor variants, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` for context only. All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases:
  1. `modify_entity(EntityMutation::SetSpeedFactor)` mutates the named entity (AC-1).
  2. `modify_entity(EntityMutation::SetFlowFactor)` mutates every per-point `flow_factor` on the named entity's path (AC-2 — the volumetric lever; the previously-considered `SetExtrusionWidthFactor` was rejected because `ExtrusionPath3D` carries no such field today and an IR shape change is out of scope for this packet).
  3. `sort_layer_by(SortKey::ByPriorityAndEntityId)` sorts the layer with travel-anchor preservation (AC-3).
  4. `insert_synthetic_layer_after(SyntheticLayerData)` inserts a layer with default sibling fields (AC-4).
  5. **WASM round-trip**: a guest module's `modify_entity` call mutates the host-side IR (AC-5 — substantive `DEV-041` closure validation).
  6. The 8 existing `finalization_builder_tdd` tests continue to PASS against the new enum API (AC-6).
  7. `slicer-macros` drain-back forwards `merge_ops` (AC-7 — code-shape contract).
  8. Existing benchy regression continues to PASS (AC-8 — packet 40 print-quality fix preserved).
- Negative cases:
  1. SDK `modify_entity` with unknown id returns Err naming `entity_id` and offending value (NEG-1).
  2. SDK `insert_synthetic_layer_after` with out-of-bounds idx returns Err naming `synthetic` and offending value (NEG-2).
  3. WASM round-trip of `modify_entity` with unknown id surfaces error (NEG-3).
  4. Closure-bound generic signatures genuinely removed from SDK (NEG-4).
- Measurable outcomes:
  - `cargo test -p slicer-sdk --test finalization_builder_tdd` PASS (≥ 9 tests).
  - `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd` PASS (≥ 3 tests).
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd` PASS (regression).
  - `cargo build --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
  - `cargo test --workspace` PASS at acceptance ceremony.
  - `DEV-041` row in `docs/DEVIATION_LOG.md` annotated as `Closed YYYY-MM-DD` at acceptance date. (`docs/14_deviation_audit_history.md` is NOT modified — it is an archive only.)
- Cross-packet impact:
  - Closes `DEV-041`.
  - Provides the round-trip-validated mutation API surface that the four future PostPass modules will consume.
  - Does not alter packet 40's print-quality fix; benchy regression confirms.

## Verification Commands

- `cargo test -p slicer-sdk --test finalization_builder_tdd -- --nocapture`
- `cargo test -p slicer-host --test finalization_mutation_roundtrip_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- `cargo test -p skirt-brim -- --nocapture`
- `cargo test -p wipe-tower -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only)

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L).

## Context Discipline Notes

- Large files in the read-only path (delegate; do NOT load full):
  - `crates/slicer-host/src/wit_host.rs` — narrow ranges only (the `HostFinalizationOutputBuilder` impl block).
  - `crates/slicer-macros/src/lib.rs` — narrow ranges (inline WIT around line 948–974, drain-back around line 1198–1214).
  - `crates/slicer-sdk/src/traits.rs` — narrow if > 600 lines.
- Likely temptation reads (avoid):
  - `crates/slicer-host/src/dispatch.rs` — packet 40 changes are closed; do not re-open.
  - `modules/core-modules/{skirt-brim,wipe-tower,top-surface-ironing}/src/lib.rs` — none of these consume `merge_ops`.
  - `OrcaSlicerDocumented/` — never load directly.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail with failing-assertion ≤ 20 lines on FAIL.
  - SDK API discovery → FACT or LOCATIONS (file:line).
  - Future-module audit → SUMMARY ≤ 200 words covering each module's plausible mutation needs.
