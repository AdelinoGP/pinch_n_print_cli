---
status: implemented
packet: finalization-mutation-builder
task_ids:
  - TASK-171
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: finalization-mutation-builder

## Goal

Promote `FinalizationOutputBuilder` from a push-only emitter into a true mutation builder that honors the `docs/01_system_architecture.md:328-363` contract for `PostPass::LayerFinalization` (*"Vec<LayerCollectionIR> (all layers, mutable — may append entities or insert synthetic layers)"*). Add four primitives — `push_entity_with_priority(layer, path, region, priority)`, `modify_entity(layer, entity_id, closure)`, `sort_layer_by(layer, key_fn)`, `insert_synthetic_layer_after(idx, new_layer)` — backed by a per-role priority table (`ExtrusionRole::default_priority()`). Replace the host's `crates/slicer-host/src/dispatch.rs:2877` `splice(0..0, fin_entities)` with: extend + ID-assign + stable-sort by priority + apply recorded mutations + apply recorded sort closures + apply recorded synthetic-layer inserts. Migrate `top-surface-ironing` to `push_entity_with_priority(..., ExtrusionRole::Ironing.default_priority())` so ironing G-code emits AFTER top-fill within the same top layer (correct print order). Skirt-brim's existing `push_entity_to_layer` call site is preserved via a thin backcompat wrapper that maps to `push_entity_with_priority(..., 0)` — preserving its current "frame around object" prepend semantics. Strengthen the Benchy AC to assert `;TYPE:Top surface` precedes `;TYPE:Ironing` line-wise within a top layer.

## Scope Boundaries

- In scope:
  - Add `pub const fn default_priority(&self) -> u32` to `ExtrusionRole` with a documented gap-spaced table (e.g., `Skirt = 0`, `OuterWall = 1000`, `InnerWall = 1500`, `ThinWall = 1700`, `SparseInfill = 3000`, `BridgeInfill = 3500`, `BottomSolidInfill = 4000`, `TopSolidInfill = 4500`, `SupportMaterial = 5000`, `SupportInterface = 5500`, `Ironing = 6000`, `WipeTower = 8000`, `PrimeTower = 8500`, `Custom(_) = 9000`). Final values set by Step 1 with rationale comments inline. Gap spacing leaves room for future roles and module-level overrides.
  - Add `push_entity_with_priority(layer, path, region, priority: u32)` to `FinalizationOutputBuilder`.
  - Add `modify_entity(layer, entity_id: u64, closure: impl FnOnce(&mut Entity))` to `FinalizationOutputBuilder` — records mutation; applied at host merge.
  - Add `sort_layer_by(layer, key_fn: impl Fn(&Entity) -> K)` for arbitrary stable-sort over a layer's `ordered_entities` post-merge.
  - Add `insert_synthetic_layer_after(idx, new_layer: LayerCollectionIR)` for outer-Vec mutation.
  - Preserve `push_entity_to_layer(layer, path, region)` as a thin backcompat alias for `push_entity_with_priority(layer, path, region, 0)` (so skirt-brim stays prepended).
  - Replace `crates/slicer-host/src/dispatch.rs:2877` `splice(0..0, fin_entities)` with the new merge sequence: extend → assign IDs (using the per-layer `LayerEntityIdGen` from Packet 39) → stable-sort `ordered_entities` by priority → apply `modify_entity` mutations → apply `sort_layer_by` closures → apply `insert_synthetic_layer_after` operations.
  - Migrate `modules/core-modules/top-surface-ironing/src/lib.rs` from `push_entity_to_layer(...)` to `push_entity_with_priority(layer, path, region, ExtrusionRole::Ironing.default_priority())`. Verify ironing emits AFTER top-fill in Benchy.
  - Strengthen `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` to assert `;TYPE:Top surface` line position precedes `;TYPE:Ironing` line position within the same top layer of the emitted G-code.
  - Verify skirt-brim's existing call site continues to render at the start of each layer (fixture or benchy regression).
  - Insert `TASK-171` row into `docs/07_implementation_status.md`.
- Out of scope:
  - Changes to `entity_id` issuance or `TravelMove` anchor — owned by Packet 39 (this packet's prerequisite).
  - Adding new `ExtrusionRole` variants — defer to a separate packet if needed.
  - Implementing the four future PostPass modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) — they consume this packet's APIs but each is its own packet.
  - Sorting non-finalization entities (perimeter, infill produced by `layer_executor.rs` are NOT re-sorted; only the post-finalization layer state is sorted). Producer order is authoritative for non-finalization entities.
  - Wiring `FinalizationOutputBuilder` mutations across WIT boundary — the builder lives host-side per session memory; modules call its methods via the existing FFI shape unchanged. Step 0 confirms.
  - Extending `LayerModule` (per-layer parallel) modules with the new builder — those modules use a different output channel; out of scope.
  - Per-region or per-object overrides of `default_priority` — modules pick their own priority via `push_entity_with_priority` arg; no per-region table.
  - Persistent role-priority configuration in module manifests — config-driven priority is out of scope; if needed later, a follow-up packet adds it.

## Prerequisites and Blockers

- Depends on:
  - Packet `39_stable-entity-ids` — must be `implemented`. Without `entity_id`, `modify_entity` cannot reference entities and `LayerEntityIdGen` cannot stamp IDs at merge time.
  - Packet `38-rev1_top-surface-ironing` — `implemented`.
- Unblocks:
  - Future modules: `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower` (each will be its own packet).
- Activation blockers:
  - Packet 39 acceptance ceremony PASSED (`TASK-170` in `docs/07_implementation_status.md`).
  - Step 0 must enumerate every existing `FinalizationModule` impl in `modules/core-modules/`. Working hypothesis from session memory: `skirt-brim`, `top-surface-ironing`, possibly `wipe-tower`, possibly `prime-tower`. The migration-cost decision (one-line opt-in vs. opt-out) depends on what Step 0 finds.
  - Step 0 must confirm the host's per-layer entity merge (post-Packet-39) still happens at `dispatch.rs` near the previous splice site; line numbers may have shifted.

## Acceptance Criteria

- **Given** a `FinalizationOutputBuilder` test harness and a fixture layer initialized with three entities of roles `OuterWall`, `SparseInfill`, `TopSolidInfill` (in producer-emit order), **when** a finalization module pushes one entity with `push_entity_with_priority(layer, path, region, ExtrusionRole::Ironing.default_priority())` and the host applies its merge sequence, **then** the post-merge `layer.ordered_entities` order by role is `[OuterWall, SparseInfill, TopSolidInfill, Ironing]` AND zero entities are reordered relative to producer-emit order at the same priority. | `cargo test -p slicer-sdk --test finalization_builder_tdd push_with_priority_lands_at_sorted_position -- --exact --nocapture`
- **Given** a fixture layer with three entities (IDs `1`, `2`, `3`; `entity_id == 2` has `flow_factor == 1.0`), **when** a module calls `output.modify_entity(layer, 2, |e| { e.path.flow_factor = 0.5; })` and the host applies its merge sequence, **then** post-merge `layer.ordered_entities` entry with `entity_id == 2` has `flow_factor == 0.5` AND the other two entries are byte-unchanged. | `cargo test -p slicer-sdk --test finalization_builder_tdd modify_entity_by_id_applies_closure -- --exact --nocapture`
- **Given** a fixture layer with five entities, **when** a module calls `output.sort_layer_by(layer, |e| (e.path.role.default_priority(), e.entity_id))` and the host applies the merge sequence, **then** post-merge `ordered_entities` is sorted ascending by `(role.default_priority, entity_id)` AND every `TravelMove.entity_id` still resolves to an entity present in the layer (regression check that the Packet-39 anchor invariant survives reorder). | `cargo test -p slicer-sdk --test finalization_builder_tdd sort_layer_by_applies_comparator -- --exact --nocapture`
- **Given** a `Vec<LayerCollectionIR>` with 3 layers (indices 0, 1, 2), **when** a module calls `output.insert_synthetic_layer_after(0, synthetic_layer)`, **then** post-merge `Vec<LayerCollectionIR>` has 4 layers in order `[layers[0], synthetic_layer, layers[1], layers[2]]` AND each layer's `entity_id` namespace is independent (the synthetic layer's IDs do not collide with neighboring layers). | `cargo test -p slicer-sdk --test finalization_builder_tdd insert_synthetic_layer_inserts_at_position -- --exact --nocapture`
- **Given** the const fn `ExtrusionRole::default_priority`, **when** evaluated for the variants `Skirt`, `OuterWall`, `InnerWall`, `SparseInfill`, `TopSolidInfill`, `Ironing`, `WipeTower`, **then** the returned `u32` values strictly satisfy `Skirt < OuterWall < InnerWall < SparseInfill < TopSolidInfill < Ironing < WipeTower` AND the gap between any two adjacent values is `>= 100` (room for future role insertion). | `cargo test -p slicer-ir --test extrusion_role_priority_tdd default_priority_orders_correctly -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end with `ironing: true`, **when** the slicer produces G-code, **then** within each layer of the emitted G-code that contains both a `;TYPE:Top surface` block and a `;TYPE:Ironing` block, the line number of the `;TYPE:Top surface` line is strictly less than the line number of the `;TYPE:Ironing` line. **This is the substantive print-quality fix from Packet 38-rev1's deferred concern.** | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_top_surface_precedes_ironing -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end, **when** the slicer produces G-code, **then** every existing assertion in `benchy_end_to_end_tdd::benchy_gcode_contains_ironing_evidence` continues to PASS — both `;TYPE:Ironing` and `;TYPE:Top surface` are present in the output. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --exact --nocapture`
- **Given** `skirt-brim` running unchanged with its existing `output.push_entity_to_layer(...)` call (the legacy alias), **when** the host applies the merge sequence on a layer that also has perimeter entities, **then** the skirt entity appears at index `0` of `ordered_entities` (pre-perimeter) — preserving skirt-brim's existing "frame around object" behavior bit-for-bit. | `cargo test -p slicer-sdk --test finalization_builder_tdd legacy_push_preserves_prepend -- --exact --nocapture`

## Negative Test Cases

- **Given** a `FinalizationOutputBuilder` and a fixture layer with entities of IDs `{1, 2}`, **when** a module calls `output.modify_entity(layer, 99, |e| { ... })` and the host applies the merge, **then** the host returns or surfaces an `Err`/`Result::Err` whose diagnostic message contains the literal substring `entity_id` AND the literal substring `99` AND no entity in the layer is mutated. | `cargo test -p slicer-sdk --test finalization_builder_tdd modify_entity_unknown_id_errors -- --exact --nocapture`
- **Given** a `Vec<LayerCollectionIR>` with 3 layers, **when** a module calls `output.insert_synthetic_layer_after(99, synthetic_layer)` (`99` exceeds layer count), **then** the host returns/surfaces an `Err` whose diagnostic message contains the literal substring `synthetic` AND the literal substring `99` AND the original `Vec<LayerCollectionIR>` length is unchanged. | `cargo test -p slicer-sdk --test finalization_builder_tdd insert_synthetic_layer_out_of_bounds_errors -- --exact --nocapture`
- **Given** a fixture layer that already has two entities of role `Ironing` produced by separate finalization modules with the same priority `6000`, **when** the host applies the merge, **then** the relative order of those two entities is the producer-call order (stable-sort property; tie preservation). | `cargo test -p slicer-sdk --test finalization_builder_tdd ties_preserve_insertion_order -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test -p slicer-ir --test extrusion_role_priority_tdd -- --nocapture`
- `cargo test -p slicer-sdk --test finalization_builder_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture`
- `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test manifest_ingestion_tdd -- --nocapture`
- `cargo test -p slicer-host --test claim_transition_matrix_tdd -- --nocapture`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace` (closure gate only — run once at acceptance ceremony, never during implementation iterations)

## Authoritative Docs

- `docs/01_system_architecture.md` lines 328–363 — `PostPass::LayerFinalization` mutability contract (this packet brings the implementation up to the doc's stated contract). Read directly.
- `docs/05_module_sdk.md` — `FinalizationOutputBuilder` API and `FinalizationModule` trait. Delegate SUMMARY ≤ 200 words for the relevant section.
- `docs/04_host_scheduler.md` lines 309–317, 680–717 — composable multi-writer patterns and PostPass scheduler shape. Read directly; narrow.
- `docs/02_ir_schemas.md` — `ExtrusionRole` variants, `LayerCollectionIR`, `Entity` (or `PrintEntity`) shape post-Packet-39. Read directly; one section.
- `docs/09_progress_events.md` — confirm no progress events are required by this packet's API additions (FACT confirm at Step 0).

## OrcaSlicer Reference Obligations

None directly required. OrcaSlicer's per-layer entity ordering is producer-driven (same model as this slicer pre-Packet-40); not a parity reference for the priority API. If the role-priority defaults need parity rationale, delegate one OrcaSlicer SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/PrintExtents.cpp` or the equivalent layer-emit ordering site. All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the spec-packet-generator's context_discipline preamble. Downstream agents implementing or reviewing this packet must:

- Treat `design.md`'s code change surface as authoritative; touch nothing outside it.
- Honor `design.md`'s out-of-bounds list (no IR shape changes; no `entity_id` issuance changes; no module changes other than the one-line `top-surface-ironing` migration).
- Delegate every cargo run, every workspace search, and every authoritative-doc fact-check.
- Stop reading at 60% context; hand off at 85%.

This is a **builder-API enrichment + host-merge replacement** packet. The biggest implementation risks are (a) ordering correctness post-merge — verified by AC-1, AC-3, AC-5, AC-6 — and (b) preserving the regression contract for skirt-brim — verified by AC-8. AC-6 is the substantive print-quality fix this packet exists to ship.
