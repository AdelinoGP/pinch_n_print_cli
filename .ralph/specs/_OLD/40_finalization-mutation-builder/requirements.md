# Requirements: finalization-mutation-builder

## Packet Metadata

- Grouped task IDs:
  - `TASK-171` (NEW)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Supersedes: none
- Depends on: `39_stable-entity-ids` (`implemented` required)
- Unblocks: future PostPass mutation modules

## Problem Statement

Two motivating problems converge:

**Problem 1 — Print order**: Packet `38-rev1_top-surface-ironing` shipped successfully but the host's `crates/slicer-host/src/dispatch.rs:2877` literal-prepend (`splice(0..0, fin_entities)`) places ironing G-code BEFORE top-fill within each top layer. This is physically wrong (ironing must follow the surface it irons). The Benchy AC asserted only label presence (`;TYPE:Ironing` and `;TYPE:Top surface`), not order, so AC-6 passed empirically — but the print quality is incorrect. The fix is to land ironing entities AFTER `TopSolidInfill` entities at G-code emit time. Packet `38-rev1`'s design.md anticipated this; the contingency was deferred to a follow-up packet (this one).

**Problem 2 — Architecture vs. implementation gap**: `docs/01_system_architecture.md:328-363` declares `PostPass::LayerFinalization` semantics as *"Vec<LayerCollectionIR> (all layers, mutable — may append entities or insert synthetic layers)"*. The current `FinalizationOutputBuilder` is push-only over a read-only view of `[LayerCollectionView]` — strictly weaker than the doc's contract. Several future modules listed in the same doc (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) need richer mutation primitives:
- Reorder existing entities (e.g., `SequentialPrintOrder` groups by `object_id`).
- Mutate existing entities in place (e.g., `MinLayerTimeEnforcer` slows specific extrusions; `FlushVolumeCalculator` adjusts wipe-tower flow).
- Insert synthetic layers (e.g., a cooling-pause synthetic layer between two real layers).
- Insert at a chosen position (e.g., `PrimeTower` strokes per layer at a deterministic pre-print slot).

Packet 39 (prerequisite, `stable-entity-ids`) provides the foundation: every entity has a stable `entity_id: u64`; `TravelMove` anchors by ID, not index. With that foundation, this packet can offer mutation/reorder/insert primitives that don't require hand-rewriting positional anchors.

This packet:
1. Adds `ExtrusionRole::default_priority() -> u32` const fn with a documented gap-spaced table (room for future role insertion).
2. Extends `FinalizationOutputBuilder` with `push_entity_with_priority`, `modify_entity`, `sort_layer_by`, and `insert_synthetic_layer_after`. Keeps `push_entity_to_layer` as a thin backcompat alias mapping to `push_entity_with_priority(..., 0)` so skirt-brim's prepend semantics are preserved.
3. Replaces the host's per-layer merge at `dispatch.rs:2877` with: extend → assign IDs → stable-sort by priority → apply mutations → apply sort closures → apply synthetic-layer inserts.
4. Migrates `top-surface-ironing/src/lib.rs` (one-line change) to use `push_entity_with_priority(..., ExtrusionRole::Ironing.default_priority())`.
5. Strengthens the Benchy AC to assert `;TYPE:Top surface` line precedes `;TYPE:Ironing` line within a top layer.

The result: the print-quality bug from Packet 38-rev1 is fixed, and the four future PostPass modules can build on a contract-compliant builder without re-touching the host merge code.

## In Scope

- `crates/slicer-ir/src/slice_ir.rs` (or wherever `ExtrusionRole` is defined per Packet 39's confirmed shape) — add `pub const fn default_priority(&self) -> u32` returning per-variant priorities. Gap-spaced (≥ 100 between adjacent values) to leave room for future roles.
- `crates/slicer-sdk/src/builders.rs` (or wherever `FinalizationOutputBuilder` lives — Step 0 confirms exact path) — add four new methods:
  - `push_entity_with_priority(layer: u32, path: ExtrusionPath3D, region: RegionKey, priority: u32)`
  - `modify_entity(layer: u32, entity_id: u64, closure: impl FnOnce(&mut Entity))` — records (layer, entity_id, boxed closure or operation marker)
  - `sort_layer_by(layer: u32, key_fn: impl Fn(&Entity) -> impl Ord)` — records (layer, boxed key_fn)
  - `insert_synthetic_layer_after(idx: u32, new_layer: LayerCollectionIR)` — records (idx, layer)
  - Preserve `push_entity_to_layer(layer, path, region)` as a thin alias for `push_entity_with_priority(layer, path, region, 0)`.
- `crates/slicer-host/src/dispatch.rs:2877` (or current location post-Packet-39) — replace `splice(0..0, fin_entities)` with the merge sequence:
  1. Extend `layer.ordered_entities` with `fin_entities`.
  2. For each newly-pushed entity, stamp an `entity_id` from the layer's `LayerEntityIdGen`.
  3. Stable-sort `ordered_entities` by `(effective_priority, original_insertion_index)` where `effective_priority = explicit_priority_if_provided.unwrap_or(role.default_priority())`.
  4. Apply each `modify_entity` operation by `entity_id` lookup; surface diagnostic on dangling ID.
  5. Apply each `sort_layer_by` closure to `ordered_entities`.
  6. After per-layer merges complete, apply `insert_synthetic_layer_after` operations to the outer `Vec<LayerCollectionIR>` (idx-validated; surface diagnostic on out-of-bounds).
- `modules/core-modules/top-surface-ironing/src/lib.rs` — single-line migration: change the `output.push_entity_to_layer(...)` call to `output.push_entity_with_priority(layer, path, region, ExtrusionRole::Ironing.default_priority())`. No other module changes.
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — add a new test `benchy_top_surface_precedes_ironing` that asserts line-position of `;TYPE:Top surface` < `;TYPE:Ironing` within at least one layer of the Benchy output. Existing assertions in `benchy_gcode_contains_ironing_evidence` are unchanged.
- `crates/slicer-ir/tests/extrusion_role_priority_tdd.rs` (new file) — verify `default_priority` ordering and gap invariants.
- `crates/slicer-sdk/tests/finalization_builder_tdd.rs` (new file) — verify each new builder method's contract.
- Insert `TASK-171` row into `docs/07_implementation_status.md`.

## Out of Scope

- Any change to `entity_id` issuance or `TravelMove` anchor semantics — owned by Packet 39 (prerequisite).
- Any new `ExtrusionRole` variant.
- Implementing `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower` — each is a separate future packet; this packet provides the API they will consume.
- Sorting non-finalization entities (perimeter, infill from `layer_executor.rs` are NOT re-sorted; only post-finalization layer state is sorted by the new merge sequence). Producer order is authoritative for non-finalization entities.
- Crossing the WIT boundary with the new builder methods if they're host-side-only (Step 0 confirms whether `FinalizationOutputBuilder` lives host-side or also has a guest-side mirror; design.md handles each case).
- Per-region or per-object overrides for `default_priority`.
- Manifest-driven priority configuration.
- Extending `LayerModule` (per-layer parallel) modules with the new builder.
- Migrating `skirt-brim` or any module other than `top-surface-ironing`. Skirt-brim's call site stays on the legacy `push_entity_to_layer` alias.

## Authoritative Docs

- `docs/01_system_architecture.md` lines 328–363 — `PostPass::LayerFinalization` mutability contract. Direct read.
- `docs/05_module_sdk.md` — `FinalizationOutputBuilder` API and `FinalizationModule` trait. Delegate SUMMARY ≤ 200 words for relevant section.
- `docs/04_host_scheduler.md` lines 309–317, 680–717 — composable multi-writer patterns; PostPass scheduler shape. Direct read.
- `docs/02_ir_schemas.md` — `ExtrusionRole` variants, `LayerCollectionIR`, entity struct shape post-Packet-39. Direct read.
- `docs/09_progress_events.md` — FACT confirm no events required by the new APIs.

## OrcaSlicer Reference Obligations

None required. If OrcaSlicer parity is challenged for the role-priority defaults, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/PrintExtents.cpp` (or the equivalent layer-emit ordering site). All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases:
  1. `push_entity_with_priority` lands the entity at the correct sorted position (AC-1).
  2. `modify_entity` applies the closure to the right entity by ID (AC-2).
  3. `sort_layer_by` applies an arbitrary stable comparator and travel anchors survive (AC-3).
  4. `insert_synthetic_layer_after` inserts a new layer at the correct position with independent ID namespace (AC-4).
  5. `default_priority` ordering and gap invariants hold (AC-5).
  6. **Benchy ordering**: `;TYPE:Top surface` line precedes `;TYPE:Ironing` line within each top layer (AC-6 — the substantive print-quality fix).
  7. Existing `benchy_gcode_contains_ironing_evidence` continues to PASS (AC-7).
  8. Skirt-brim's legacy call site preserves prepend semantics (AC-8).
- Negative cases:
  1. `modify_entity` with unknown ID returns Err naming the ID (NEG-1).
  2. `insert_synthetic_layer_after` with out-of-bounds idx returns Err naming the idx (NEG-2).
  3. Tied priorities preserve producer insertion order (NEG-3).
- Measurable outcomes:
  - `cargo test -p slicer-ir --test extrusion_role_priority_tdd` PASS.
  - `cargo test -p slicer-sdk --test finalization_builder_tdd` PASS (8 tests).
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd` PASS (existing + 1 new test).
  - `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd` PASS (regression — already 8/8 pre-this-packet).
  - `cargo build --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
  - `cargo test --workspace` PASS at acceptance ceremony.
- Cross-packet impact:
  - Closes the deferred print-order concern from Packet 38-rev1's final report.
  - Provides the API surface that future PostPass mutation modules will consume.

## Verification Commands

- `cargo test -p slicer-ir --test extrusion_role_priority_tdd -- --nocapture`
- `cargo test -p slicer-sdk --test finalization_builder_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
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
  - `crates/slicer-host/src/dispatch.rs` — only the post-Packet-39 finalization-merge site (≤ 30 lines around the previous `splice(0..0, ...)` location). FACT-narrowed reads only.
  - `crates/slicer-sdk/src/builders.rs` (or located path) — full read only if < 300 lines; otherwise delegate SUMMARY of the `FinalizationOutputBuilder` impl.
- Likely temptation reads (avoid):
  - All other core modules beyond `top-surface-ironing` (the migration target) and `skirt-brim` (the regression check).
  - `crates/slicer-host/src/wit_host.rs` and `gcode_emit.rs` outside narrow ranges.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail with failing-assertion ≤ 20 lines on FAIL.
  - SDK API discovery → FACT or LOCATIONS (file:line).
  - Reference template summary → SUMMARY ≤ 200 words.
