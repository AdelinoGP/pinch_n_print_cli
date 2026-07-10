---
status: implemented
packet: 56c_threemf-negative-and-support-subtype-routing
task_ids:
  - TASK-192b
  - TASK-192c
  - TASK-193
---

# 56c_threemf-negative-and-support-subtype-routing

## Goal

Wire the remaining two downstream consumers for non-`NormalPart` subtypes already routed into `ObjectMesh.modifier_volumes` by Packet 56b:

1. **`negative_part`** — Introduce a new host stage `apply_negative_part_subtract` (file: `crates/slicer-host/src/negative_part_subtract.rs`) with signature `pub fn apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` (singular `SliceIR` — one layer at a time). Insert its invocation **per-layer inside `crates/slicer-host/src/layer_executor.rs::run_paint_annotation`**, after `arena.take_slice()` returns the layer's `SliceIR` and BEFORE the paint annotation loop begins. `pipeline.rs` and `prepass.rs` are NOT modified — `Vec<SliceIR>` is produced per-layer in `layer_executor.rs::execute_layer_slice` (after prepass returns), so the subtract must land at the per-layer seam. For each `ModifierVolume` whose `config_delta.fields[&ConfigKey::from("subtype")] == ConfigValue::String("negative_part")`, resolve `slice_ir.z` against the modifier's Z extent; if inside the extent, project the modifier mesh at `slice_ir.z` via `slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])` and replace each `slice_ir.regions[ri].polygons` with `slicer_core::polygon_ops::difference(&slice_ir.regions[ri].polygons, &projection)`. Layers outside the extent are skipped. This is the Activation Q3 = Option 1 lock from the original packet, with the insertion point corrected to reflect the real pipeline topology (the original "phase-0 built-in inside `prepass.rs`" intent was architecturally infeasible — `SliceIR` does not exist at prepass time).

2. **`support_enforcer` / `support_blocker`** — Augment `crates/slicer-host/src/paint_segmentation.rs` to emit synthetic `PaintRegionIR` entries for each `support_enforcer` / `support_blocker` modifier volume. `paint_segmentation.rs` reads modifier volumes directly from `mesh_ir.objects[].modifier_volumes` (no new parameter on `execute_paint_segmentation`). Project each volume per layer via `slice_mesh_ex`; map to `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. Synthetic entries are union-merged with any existing `Vec<SemanticRegion>` at the same `(layer, semantic)` via `slicer_core::polygon_ops::union`. These synthetic entries flow through Packet 51's `paint_overrides` overlay; no new region-mapping code beyond Packet 56b's `modifier_part` overlap stamp.

This packet adds one new synthetic-fixture E2E suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds IR structs in-memory (IndexedTriangleSet meshes, ModifierVolume sidecars, and MeshIR directly) — no 3MF archive parsing. The synthetic fixtures exercise: a `negative_part` reducing layer polygon area; a `support_enforcer` emitting `PaintRegionIR` at every overlapping layer; a `support_blocker` emitting `PaintRegionIR` at every overlapping layer.

No new deviations are registered by this packet. DEV-047, DEV-048, and DEV-049 are already closed by Packets 56 / 56b. The behavior added here is contract-conformant: it consumes existing `ObjectMesh.modifier_volumes` plumbing and Packet 51's paint-semantic overlay; no fallback paths.

## Problem Statement

Packet 56b (`56b_threemf-modifier-part-ir-routing`) routes ALL four non-`NormalPart` subtypes (`modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`) into `ObjectMesh.modifier_volumes` and wires the `modifier_part` consumer (region-mapping fuzzy overlap stamp). After Packet 56b lands, fixtures with `negative_part` or `support_*` parts have populated `modifier_volumes` entries — but no downstream consumer reads them. A `negative_part` cube does not subtract from the parent's slice polygons. A `support_enforcer` volume does not emit `PaintRegionIR` entries.

This packet (56c) closes that gap. It introduces:

1. A new host stage `apply_negative_part_subtract` in `crates/slicer-host/src/negative_part_subtract.rs` with signature `pub fn apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` (singular `SliceIR` — one layer at a time). The stage is invoked per-layer inside `crates/slicer-host/src/layer_executor.rs::run_paint_annotation`, after `arena.take_slice()` and BEFORE the paint annotation loop begins (Activation Q3 = Option 1 locked at original-packet-author time; insertion point corrected from the infeasible original "phase-0 built-in inside `prepass.rs`" — `Vec<SliceIR>` does not exist at prepass time, it is produced per-layer in `layer_executor.rs::execute_layer_slice` after prepass returns). For each `negative_part` modifier volume, it resolves `slice_ir.z` against the modifier's Z extent; if inside, it projects the modifier mesh at `slice_ir.z` via `slice_mesh_ex(&mv.mesh, &[slice_ir.z])` and calls `slicer_core::polygon_ops::difference` against each `slice_ir.regions[ri].polygons`. Modifiers outside the Z extent are skipped.

2. Synthetic `PaintRegionIR` emission for `support_enforcer` and `support_blocker` volumes in `crates/slicer-host/src/paint_segmentation.rs::execute_paint_segmentation`. Modifier volumes are read directly from `mesh_ir.objects[].modifier_volumes` (already populated by Packet 56b). Piggybacks the existing per-object `for object in &mesh_ir.objects` loop at `paint_segmentation.rs:383` — no new parameter added to the signature beyond the `union_paint_regions_at_harvest: bool` already introduced by Packet 64. Each volume is projected per layer; the projections are emitted as `SemanticRegion` entries inserted into `LayerPaintMap.semantic_regions` under `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`, union-merged with any existing entries via `slicer_core::polygon_ops::union`. These flow through Packet 51's `paint_overrides` overlay path with no new region-mapping code.

3. A new synthetic-fixture E2E test suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds IR structs in-memory (IndexedTriangleSet meshes via `box_mesh()`, ModifierVolume sidecars via `modifier_volume_with_subtype()`, and MeshIR via `mesh_ir_with_modifier()`) — no 3MF archive parsing. The synthetic fixtures cover the three subtypes' consumer behavior plus pipeline-ordering correctness (negative subtract runs per-layer inside `layer_executor.rs::run_paint_annotation` at line 635, after `arena.take_slice()` and before the paint annotation loop, mutating `slice_ir.regions[].polygons` so downstream per-layer consumers see the post-subtract polygons) plus four degenerate-case negative tests (negative above parent, empty negative, empty support_enforcer, empty support_blocker).

No new IR types are introduced. `SliceIR`, `PaintRegionIR`, `PaintSemantic::SupportEnforcer`, `PaintSemantic::SupportBlocker` already exist (Packets 50b / 51). This packet is consumer-side wiring on already-populated IR.

No new deviations are registered. DEV-047, DEV-048, and DEV-049 were closed by Packets 56 and 56b. The behavior here is contract-conformant; the synthetic fixtures exercise positive paths only (plus two degenerate-case negative tests for completeness).

This packet is the third and terminal packet in the three-way split. It runs `cargo test --workspace` exactly once at acceptance ceremony — the only packet in the split that does so. This workspace gate confirms that the full original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice (sidecar parser → IR routing → all four consumer wirings) is operational without regressions.

WIT scope is **clean** — confirmed by Packets 56 / 56b. This packet introduces no IR types and is not re-checked.

This packet does not modify Packet 56's or Packet 56b's directories. Cross-Packet Mutation Rule satisfied.

## Architecture Constraints

- Scaled integer units: `slicer_core::polygon_ops` operates on `ExPolygon` in scaled integer units. All modifier projections produced by `slice_mesh_ex` are already in scaled integer units and require no conversion before `polygon_ops::difference` / `::union` calls.
- Per-layer projection: use `slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])` (verified at `crates/slicer-core/src/triangle_mesh_slicer.rs:46`; signature `(&IndexedTriangleSet, &[f32]) -> Vec<Vec<ExPolygon>>` — one `Vec<ExPolygon>` per requested Z). The per-layer call site uses a single-element Z slice because `apply_negative_part_subtract` is invoked once per layer from `layer_executor.rs::run_paint_annotation`. This is the same function Packet 56b uses for modifier-part fuzzy-skin projections in `layer_executor.rs::run_paint_annotation` (call pattern at `layer_executor.rs:559-562`).
- Layer-executor ordering: inside `run_paint_annotation`, the sequence becomes `arena.take_slice()` → `apply_negative_part_subtract(&mut slice_ir, &modifier_volumes)` → paint annotation loop → downstream per-layer consumers. The order is critical because paint annotation and all subsequent per-layer consumers must see post-subtract polygons (otherwise per-layer paint annotation operating on a region subtracted by a negative volume would emit phantom regions). Step 2 FACT dispatch confirms the exact insertion line within `layer_executor.rs::run_paint_annotation`.
- Determinism: per-layer call order is locked. `apply_negative_part_subtract` is purely functional given `&mut SliceIR + modifier_volumes`; no global state.
- WIT boundary: clean (re-confirmed at Packet 56b Step 0). This packet introduces no IR types and does not re-check.
- `support_*` synthetic `PaintRegionIR` polygons MUST be union-merged into any existing `Vec<SemanticRegion>` at the same `(layer, semantic)` key (in case paint-supports triangle painting AND a `support_enforcer` volume coexist on the same model). Union helper verified: `slicer_core::polygon_ops::union(&[ExPolygon], &[ExPolygon]) -> Vec<ExPolygon>` at `polygon_ops.rs:93`.

## Data and Contract Notes

- `SliceIR` is **per-layer** (`crates/slicer-ir/src/slice_ir.rs:1102`): `pub struct SliceIR { schema_version: SemVer, global_layer_index: u32, z: f32, regions: Vec<SlicedRegion> }`. One `SliceIR` instance represents one layer. The host pipeline produces a `Vec<SliceIR>` (one per layer); `apply_negative_part_subtract` takes `&mut [SliceIR]` and iterates layer by layer.
- `SlicedRegion` layer polygon storage (`slice_ir.rs:1068-1074`): `pub struct SlicedRegion { object_id, region_id, polygons: Vec<ExPolygon>, infill_areas: Vec<ExPolygon>, … }`. Per-layer 2D polygons live at `slice_irs[li].regions[ri].polygons`.
- `PaintRegionIR` shape (`slice_ir.rs:945-950`): `pub struct PaintRegionIR { schema_version, per_layer: HashMap<u32, LayerPaintMap> }`. Access is `paint_region_ir.per_layer.get(&global_layer_index)`, NOT array indexing.
- `LayerPaintMap` shape (`slice_ir.rs:936-941`): `pub struct LayerPaintMap { global_layer_index, semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>> }`.
- `SemanticRegion` shape (`slice_ir.rs:923-932`): `pub struct SemanticRegion { object_id, polygons: Vec<ExPolygon>, value: PaintValue, paint_order: u64, aabb: Option<Aabb> }`. A `HashMap<PaintSemantic, Vec<SemanticRegion>>` lookup returns `Option<&Vec<SemanticRegion>>`; per-layer area is the sum of `region.polygons` area across all returned `SemanticRegion`s.
- `PaintSemantic::SupportEnforcer` (`slice_ir.rs:180`) and `PaintSemantic::SupportBlocker` (`slice_ir.rs:182`) exist as enum variants (Packet 50b precedent, confirmed).
- `slicer_core::polygon_ops::difference` — Clipper2-backed. Verified signature (`polygon_ops.rs:103`): `pub fn difference(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>`.
- `slicer_core::polygon_ops::union` — verified at `polygon_ops.rs:93`: `pub fn union(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>`.
- `slicer_core::slice_mesh_ex` — verified at `triangle_mesh_slicer.rs:46`: `pub fn slice_mesh_ex(mesh: &IndexedTriangleSet, zs: &[f32]) -> Vec<Vec<ExPolygon>>`. In `paint_segmentation.rs:383-427`, it is called once per volume with `&layer_zs` (all layer Zs batched), not per-layer.
- `ModifierVolume.mesh: IndexedTriangleSet` (`slice_ir.rs:252-265`) is in world space (Packet 56b's invariant). Per-layer projection slices at layer Z directly.
- `ModifierVolume.config_delta: ConfigDelta` (`slice_ir.rs:231-235`) has `fields: HashMap<ConfigKey, ConfigValue>` with `ConfigKey = String` and `ConfigValue::String(String)`. The access `mv.config_delta.fields.get("subtype")` (used at `paint_segmentation.rs:384`) returns `Option<&ConfigValue>`.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|---|
| Negative-part subtract changes per-layer polygons. Downstream per-layer consumers (paint annotation and beyond) must see the post-subtract polygons. | Stage inserted as a per-layer call inside `layer_executor.rs::run_paint_annotation` at line 635, AFTER `arena.take_slice()` at line 628 and BEFORE the paint annotation loop. AC-5 asserts downstream consumers see post-subtract polygons. |
| Per-layer insertion inside `layer_executor.rs::run_paint_annotation` sits alongside Packet 56b's modifier-part fuzzy-skin call. Risk of unintended interaction. | Insertion at line 635 is additive — placed before the paint annotation loop and before Packet 56b's fuzzy-skin call so both see post-subtract polygons. |
| Support enforcer / blocker piggyback re-uses Packet 50b / 51's paint-supports semantic. If those test surfaces drift, this packet's tests may regress. | Step 4 regression sweep explicitly re-runs Packet 50b / 51 regression tests. |
| Support enforcer/blocker loop runs in `execute_paint_segmentation` which is now a host-native fallback for `PrePass::PaintSegmentation` (Packet 64). If a WASM module registers for `PrePass::PaintSegmentation`, the host-native piggyback is bypassed and support volumes emit nothing. | Packet 64 deleted the WASM module; the host-native path is the sole path. If a future WASM module registers for this stage, the support-volume emission must be re-verified. |
| `apply_negative_part_subtract` mutates the `&mut SliceIR` in place — accidental double-application if `run_paint_annotation` runs the stage twice per layer. | Stage is purely functional; `run_paint_annotation` dequeues each `SliceIR` from the arena exactly once per layer, so the call fires exactly once per layer by construction. Call is at exactly one site (line 635). |
| Performance: per-layer × per-volume × per-triangle projection is O(L × V × T). The per-layer call site makes one `slice_mesh_ex` call per volume per layer. For synthetic 5×5×5 mm cube volumes (12 triangles), trivial. | Out of scope. TODO logged in `negative_part_subtract.rs`. |
| The synthetic-fixture builder uses IR struct builders instead of 3MF archive parsing. This tests consumer logic directly but does not exercise the full 3MF parse → IR → consumers pipeline. | 3MF-to-IR parsing is tested by Packet 56's `threemf_sidecar_classification_tdd.rs` and Packet 56b's `benchy_4color_modifier_part_e2e_tdd.rs`. This packet's IR-level tests complement those with focused consumer-logic coverage. |
| `PaintRegionIR.per_layer` is `HashMap<u32, LayerPaintMap>` — code MUST use `.get(&n)` / `.get_mut(&n)` / `.entry(n).or_insert_with(...)`. | AC prose uses explicit `HashMap` access; tests use the real API. In `paint_segmentation.rs:383-427`, entries are created via `.entry(layer.index).or_insert_with(...)`. |

## Locked Assumptions and Invariants

1. WIT scope is clean (confirmed by Packets 56 / 56b; not re-checked here).
2. `ObjectMesh.modifier_volumes` is populated for ALL non-`NormalPart` subtypes by Packet 56b. This packet consumes that plumbing.
3. Inside `layer_executor.rs::run_paint_annotation` (line 613), the per-layer ordering is: `arena.take_slice()` (line 628) → `apply_negative_part_subtract(&mut slice_ir, &modifier_volumes)` → paint annotation loop → downstream per-layer consumers. Order is critical: paint annotation and all subsequent per-layer consumers must see post-subtract polygons. Paint segmentation runs earlier inside prepass and operates on triangle paint attributes (not `SliceIR` polygons), so it is unaffected by this stage.
4. `negative_part` subtract is a per-layer call invoked from `layer_executor.rs::run_paint_annotation`, NOT a `prepass.rs` phase-0 built-in (the original design was infeasible because `Vec<SliceIR>` does not exist at prepass time), NOT a `pipeline.rs` insertion, and NOT a region-mapping inline operation.
5. `support_enforcer` / `support_blocker` emit synthetic `PaintRegionIR` entries; they do NOT introduce new region-mapping code. `paint_segmentation.rs` reads modifier volumes directly from `mesh_ir.objects[].modifier_volumes` at line 383. No new parameter was added to `execute_paint_segmentation` beyond `union_paint_regions_at_harvest: bool` (introduced by Packet 64). The second modifier-volume loop runs unconditionally (not gated by `union_paint_regions_at_harvest`).
6. `apply_negative_part_subtract` operates on `&mut SliceIR` (singular — one layer at a time) and mutates `slice_ir.regions[ri].polygons` in place. `layer_executor.rs::run_paint_annotation` invokes it per-layer for each `SliceIR` dequeued from the arena.
7. `PaintRegionIR.per_layer` is `HashMap<u32, LayerPaintMap>`; access via `.get(&n)` / `.get_mut(&n)`. `LayerPaintMap.semantic_regions` is `HashMap<PaintSemantic, Vec<SemanticRegion>>`; per-layer area for a given semantic is the sum across all returned `SemanticRegion`s' `polygons`.
8. Existing tests for Packets 50 / 50b / 51 / 56 / 56b stay GREEN.
9. No new IR types introduced.
10. No new deviations registered.
11. `cargo test --workspace` runs exactly once at Step 7 acceptance ceremony.
12. The terminal packet of the three-way split closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice in full. No follow-up packet is needed in this scope; consumers like `extruder` per-modifier override remain future work.
