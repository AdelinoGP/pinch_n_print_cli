---
status: implemented
packet: finalization-mutation-builder
task_ids:
  - TASK-171
---

# 40_finalization-mutation-builder

## Goal

Promote `FinalizationOutputBuilder` from a push-only emitter into a true mutation builder that honors the `docs/01_system_architecture.md:328-363` contract for `PostPass::LayerFinalization` (*"Vec<LayerCollectionIR> (all layers, mutable — may append entities or insert synthetic layers)"*). Add four primitives — `push_entity_with_priority(layer, path, region, priority)`, `modify_entity(layer, entity_id, closure)`, `sort_layer_by(layer, key_fn)`, `insert_synthetic_layer_after(idx, new_layer)` — backed by a per-role priority table (`ExtrusionRole::default_priority()`). Replace the host's `crates/slicer-host/src/dispatch.rs:2877` `splice(0..0, fin_entities)` with: extend + ID-assign + stable-sort by priority + apply recorded mutations + apply recorded sort closures + apply recorded synthetic-layer inserts. Migrate `top-surface-ironing` to `push_entity_with_priority(..., ExtrusionRole::Ironing.default_priority())` so ironing G-code emits AFTER top-fill within the same top layer (correct print order). Skirt-brim's existing `push_entity_to_layer` call site is preserved via a thin backcompat wrapper that maps to `push_entity_with_priority(..., 0)` — preserving its current "frame around object" prepend semantics. Strengthen the Benchy AC to assert `;TYPE:Top surface` precedes `;TYPE:Ironing` line-wise within a top layer.

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

## Architecture Constraints

- **Trait signature stability for FinalizationModule**: the existing `run_finalization(&self, layers: &[LayerCollectionView], output: &mut FinalizationOutputBuilder, _config: &ConfigView)` shape stays unchanged. Modules opt into new methods by calling them; legacy modules continue to work via the `push_entity_to_layer` alias.
- **Builder operation recording, not direct mutation**: `modify_entity`, `sort_layer_by`, and `insert_synthetic_layer_after` RECORD operations on the builder. The host applies them deterministically AFTER the module returns. This preserves the existing dispatch-site invariant that modules do not directly mutate the IR; mutations are reified as data and applied by the host.
- **Stable identity**: relies on Packet 39's `entity_id`. `modify_entity(layer, entity_id, ...)` matches by ID; `sort_layer_by` and `insert_synthetic_layer_after` do not depend on identity but rely on the existing `entity_id` invariant for travel anchors to survive reorder.
- **Deterministic merge order**: when the host applies recorded operations, the order is:
  1. Extend `ordered_entities` with new pushes (new pushes carry `Option<u32>` explicit priority; `None` defaults to role priority).
  2. Stamp `entity_id` on every newly-pushed entity from the layer's `LayerEntityIdGen`.
  3. Stable-sort the entire `ordered_entities` by `(effective_priority(entry), original_index_within_layer_at_post-extend)`. The "original index" tiebreaker keeps producer-emit order stable for ties.
  4. Apply each `modify_entity` op by `entity_id` lookup; surface diagnostic on dangling ID.
  5. Apply each `sort_layer_by` closure (rare but documented; runs after `modify_entity`).
  6. After all per-layer merges, apply `insert_synthetic_layer_after` ops to the outer `Vec<LayerCollectionIR>`. Multiple inserts at the same `idx` apply in module-call order (stable insertion).
- **Backwards compatibility**: `push_entity_to_layer(layer, path, region)` is `#[inline]` to `push_entity_with_priority(layer, path, region, 0)`. Skirt-brim's behavior is preserved bit-for-bit (priority 0 places its entities first; ties preserve producer order).
- **Producer (non-finalization) entities are NOT re-sorted**: only the post-extend layer state is sorted. Wait — that's incorrect: if the producer-emitted entries should keep their producer order, they MUST have priorities matching their emit order. The role-priority table is designed so producer-emit order matches priority order:
  - Producers emit in order: walls (OuterWall < InnerWall) → fill (Sparse → Solid → Top → Bottom) → support → ironing-not-yet-emitted-by-producer.
  - Role priority table mirrors this: `OuterWall=1000 < InnerWall=1500 < ThinWall=1700 < SparseInfill=3000 < BridgeInfill=3500 < BottomSolidInfill=4000 < TopSolidInfill=4500 < SupportMaterial=5000 < SupportInterface=5500 < Ironing=6000 < WipeTower=8000 < PrimeTower=8500 < Skirt=0` (skirt is special; lowest, which preserves prepend).
  - Therefore stable-sorting the entire vec by priority preserves producer order for non-finalization entries, AND lands finalization entries at their correct slot. **This is the load-bearing invariant.** Step 1 explicitly verifies it via a unit test that constructs all roles in producer order, sorts by `default_priority`, and asserts the result equals the producer-order vec.
- **Tie-stable**: stable-sort is mandatory. Two entities with the same priority preserve their post-extend insertion order. Critical for two finalization modules pushing at the same priority and for producer pairs (e.g., two `OuterWall` entities in the same layer).
- **Travel anchor survival**: travels anchor by `entity_id` (Packet 39 invariant). Reorder via stable-sort preserves all `entity_id` values; lookup at emit time still resolves correctly. AC-3 explicitly verifies.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `ExtrusionRole` — new const fn `default_priority()`. Additive; no breaking change.
  - No new IR fields. Entity struct already carries `entity_id` and `path.role` post-Packet-39.
- WIT boundary considerations: `FinalizationOutputBuilder` is host-side / SDK-side only per session memory. Step 0 confirms by grep. If WIT mirror exists, packet escalates.
- Determinism / scheduler constraints:
  - PostPass is sequential (`docs/04_host_scheduler.md:680-717`). No parallelism inside the merge.
  - Stable-sort is mandatory (Rust's `sort_by` is unstable; use `sort_by_key` with a tuple `(priority, original_index)` OR use `sort_by` with a stable comparator from the `slicer-helpers` crate if one exists).
  - Multiple finalization modules pushing at the same priority preserve producer-call order via the stable-sort tiebreaker.

## Locked Assumptions and Invariants

- `ExtrusionRole` enum has the variants enumerated by Packet 38-rev1's Step 0 (13 + `Custom(String)`): `OuterWall, InnerWall, ThinWall, TopSolidInfill, BottomSolidInfill, SparseInfill, SupportMaterial, SupportInterface, WipeTower, PrimeTower, Ironing, BridgeInfill, Skirt`.
- Producers emit in role order: `OuterWall < InnerWall < ThinWall < SparseInfill < BridgeInfill < BottomSolidInfill < TopSolidInfill < SupportMaterial < SupportInterface < Ironing`. The default-priority table is designed so this order is preserved by the stable-sort.
- Skirt has the lowest priority (0); legacy `push_entity_to_layer` maps to priority 0 to preserve skirt-brim's prepend semantics.
- `Custom(String)` defaults to a fixed priority (e.g., 9000) representing "after most things"; this is a deliberate choice — modules using `Custom` are typically ad-hoc additions.
- Packet 39 is `implemented` and `LayerEntityIdGen` is available for the host merge to stamp IDs on finalization-pushed entities.
- `FinalizationOutputBuilder` lives host-side / SDK-side only (no WIT mirror).
- `gcode_emit.rs` does NOT re-sort `ordered_entities`; the order at emit time is the order produced by the merge sequence.
