---
status: implemented
packet: 56c_threemf-negative-and-support-subtype-routing
task_ids:
  - TASK-192b
  - TASK-192c
  - TASK-193
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - 56_threemf-sidecar-parser (must be status: implemented)
  - 56b_threemf-modifier-part-ir-routing (must be status: implemented)
  - 64_paint-native-migration (must be status: implemented; provides host-native PrePass::PaintSegmentation + union_paint_regions_at_harvest)
unblocks: []
---

# Packet Contract: 56c_threemf-negative-and-support-subtype-routing

> This packet is the third of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet. Packet 56 owns the sidecar parser (TASK-190). Packet 56b owns `resolve_object` branching + schema bump + `modifier_part` region overlap (TASK-191, TASK-192a). This packet (56c) owns the remaining two downstream consumers — `negative_part` and `support_enforcer`/`support_blocker` — plus the synthetic-fixture E2E coverage that ties them together (TASK-192b, TASK-192c, TASK-193). Aggregate cost is **M**.

## Goal

Wire the remaining two downstream consumers for non-`NormalPart` subtypes already routed into `ObjectMesh.modifier_volumes` by Packet 56b:

1. **`negative_part`** — Introduce a new host stage `apply_negative_part_subtract` (file: `crates/slicer-host/src/negative_part_subtract.rs`) with signature `pub fn apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` (singular `SliceIR` — one layer at a time). Insert its invocation **per-layer inside `crates/slicer-host/src/layer_executor.rs::run_paint_annotation`**, after `arena.take_slice()` returns the layer's `SliceIR` and BEFORE the paint annotation loop begins. `pipeline.rs` and `prepass.rs` are NOT modified — `Vec<SliceIR>` is produced per-layer in `layer_executor.rs::execute_layer_slice` (after prepass returns), so the subtract must land at the per-layer seam. For each `ModifierVolume` whose `config_delta.fields[&ConfigKey::from("subtype")] == ConfigValue::String("negative_part")`, resolve `slice_ir.z` against the modifier's Z extent; if inside the extent, project the modifier mesh at `slice_ir.z` via `slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])` and replace each `slice_ir.regions[ri].polygons` with `slicer_core::polygon_ops::difference(&slice_ir.regions[ri].polygons, &projection)`. Layers outside the extent are skipped. This is the Activation Q3 = Option 1 lock from the original packet, with the insertion point corrected to reflect the real pipeline topology (the original "phase-0 built-in inside `prepass.rs`" intent was architecturally infeasible — `SliceIR` does not exist at prepass time).

2. **`support_enforcer` / `support_blocker`** — Augment `crates/slicer-host/src/paint_segmentation.rs` to emit synthetic `PaintRegionIR` entries for each `support_enforcer` / `support_blocker` modifier volume. `paint_segmentation.rs` reads modifier volumes directly from `mesh_ir.objects[].modifier_volumes` (no new parameter on `execute_paint_segmentation`). Project each volume per layer via `slice_mesh_ex`; map to `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. Synthetic entries are union-merged with any existing `Vec<SemanticRegion>` at the same `(layer, semantic)` via `slicer_core::polygon_ops::union`. These synthetic entries flow through Packet 51's `paint_overrides` overlay; no new region-mapping code beyond Packet 56b's `modifier_part` overlap stamp.

This packet adds one new synthetic-fixture E2E suite (`threemf_subtypes_synthetic_e2e_tdd.rs`) that builds IR structs in-memory (IndexedTriangleSet meshes, ModifierVolume sidecars, and MeshIR directly) — no 3MF archive parsing. The synthetic fixtures exercise: a `negative_part` reducing layer polygon area; a `support_enforcer` emitting `PaintRegionIR` at every overlapping layer; a `support_blocker` emitting `PaintRegionIR` at every overlapping layer.

No new deviations are registered by this packet. DEV-047, DEV-048, and DEV-049 are already closed by Packets 56 / 56b. The behavior added here is contract-conformant: it consumes existing `ObjectMesh.modifier_volumes` plumbing and Packet 51's paint-semantic overlay; no fallback paths.

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW file. Defines `apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` (singular `SliceIR`). For each `negative_part` volume, resolves `slice_ir.z` against the modifier's Z extent, projects the modifier mesh at that Z via `slice_mesh_ex`, and calls `slicer_core::polygon_ops::difference` per region. Mutates `slice_ir.regions[ri].polygons` in place. Modifiers outside the extent are skipped.
  - `crates/slicer-host/src/layer_executor.rs` — insert `apply_negative_part_subtract(...)` call inside `run_paint_annotation`, after `arena.take_slice()` and BEFORE the paint annotation loop begins. Modifier volumes are pulled from the current object's `ObjectMesh.modifier_volumes` already in scope at the layer-executor seam.
  - `crates/slicer-host/src/paint_segmentation.rs` — augment `execute_paint_segmentation` to read `mesh_ir.objects[].modifier_volumes` internally and emit synthetic `PaintRegionIR` per layer for each `support_enforcer` / `support_blocker` volume. No new parameter on the function signature.
  - `crates/slicer-host/src/lib.rs` (or the module-root file confirmed at Step 2 via FACT dispatch) — declare `pub mod negative_part_subtract`.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW. Builds IR structs in-memory (IndexedTriangleSet meshes, ModifierVolume sidecars wired into MeshIR directly). Asserts: post-subtract polygon area, `PaintRegionIR` entries at correct layers, polygon match within ±0.005 mm² of the projected modifier volume.
  - `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows naming this packet.

- Out of scope:
  - Sidecar parser, `resolve_object` branching, schema bump, `modifier_part` region overlap, fuzzy-skin manifest gate — all closed by Packets 56 / 56b.
  - Any change to `crates/slicer-ir/`. `SliceIR`, `PaintRegionIR`, `PaintSemantic` already exist; this packet consumes them unchanged.
  - Any change to `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs`. WIT scope confirmed clean by Packets 56 / 56b.
  - Any change to `crates/slicer-macros/`, `crates/slicer-sdk/`.
  - Any change to `crates/slicer-host/src/region_mapping.rs` or `model_loader.rs`. Owned by Packets 56b / 56 respectively; touching them here violates Cross-Packet Mutation Rule for in-flight active packets.
  - Any change to `crates/slicer-host/src/pipeline.rs` or `crates/slicer-host/src/prepass.rs`. `Vec<SliceIR>` does not exist at prepass time; the per-layer insertion lands inside `layer_executor.rs::run_paint_annotation`, which is already orchestrated by the existing layer-execution path.
  - Any change to `modules/core-modules/fuzzy-skin/` (manifest already gated by Packet 56b).
  - Any new fuzzy-skin behavior (this packet does not stamp `fuzzy_skin.apply-to-all` for `negative_part` or `support_*` volumes — those subtypes have their own consumers).
  - Consuming `<part>/<metadata key="extruder">` per-modifier override (not consumed by this packet either; future work).
  - Sidecar `<assemble>` / `<plate>` sections.

## Prerequisites and Blockers

- Depends on:
  - **Packet 56 (`56_threemf-sidecar-parser`) status: implemented.** Provides `parse_3mf_sidecar` and `PartSubtype` enum.
  - **Packet 56b (`56b_threemf-modifier-part-ir-routing`) status: implemented.** Provides `resolve_object` branching, `MeshIR.schema_version == 1.1.0`, populated `ObjectMesh.modifier_volumes` for ALL non-`NormalPart` subtypes (including `negative_part` and `support_*` which were routed but had no consumer until this packet).
  - `slicer_core::polygon_ops::difference` (Clipper2-backed). Public export.
  - Packet 50b's paint-supports semantic and Packet 51's `paint_overrides` overlay remain intact (regression).
- Unblocks: nothing further. This is the terminal packet in the three-way split.
- Activation blockers (must be resolved before flipping `status: draft` → `active`):
  - **Q1 (Packets 56 and 56b status).** Confirm both are `status: implemented`. Verify by grep on each packet's `packet.spec.md`.

## Acceptance Criteria

- **Given** a `MeshIR` with two `ObjectMesh` entries where the parent is a 20 × 20 × 20 mm cube and a second object carries a `ModifierVolume` classified `subtype="negative_part"` with a 5 × 5 × 5 mm cube mesh positioned inside the parent's extent, **when** `apply_negative_part_subtract` executes per-layer, **then** for every layer Z in the negative volume's extent the post-subtract sum of layer polygon areas is strictly less than the pre-subtract sum AND for every layer Z outside the negative volume's extent the polygon areas are unchanged (verify by comparing two slice runs differing only in the presence of the negative modifier volume). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area -- --exact --nocapture`
- **Given** the same IR fixture, **when** the polygon area reduction is measured at the centroid Z of the negative volume, **then** the reduction matches `25.0 mm²` (the negative cube's cross-section) within ±0.005 mm² (Clipper2 rounding tolerance). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_area_reduction_matches_cube_cross_section -- --exact --nocapture`
- **Given** a `MeshIR` where an `ObjectMesh` carries a `ModifierVolume` classified `support_enforcer`, **when** `execute_paint_segmentation` processes the model, **then** for every global layer index `n` intersecting the enforcer volume `paint_region_ir.per_layer.get(&n).and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer))` is `Some(regions)` with at least one `SemanticRegion` AND the aggregate polygon area across all returned `SemanticRegion`s' `polygons` matches the modifier's per-layer projection area within ±0.005 mm² total area difference. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region -- --exact --nocapture`
- **Given** a `MeshIR` where an `ObjectMesh` carries a `ModifierVolume` classified `support_blocker`, **when** `execute_paint_segmentation` processes the model, **then** the same invariant holds with `PaintSemantic::SupportBlocker` in place of `PaintSemantic::SupportEnforcer` (aggregate area across all returned `SemanticRegion`s within ±0.005 mm² of the per-layer projection). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_blocker_emits_paint_region -- --exact --nocapture`
- **Given** the new `apply_negative_part_subtract` stage runs per-layer in `layer_executor.rs::run_paint_annotation` BEFORE paint annotation reads the layer's polygons, **when** a synthetic fixture combining `negative_part` AND any geometric reference to `slice_ir.regions[].polygons` is processed, **then** downstream consumers see the post-subtract polygons (i.e., the mutated `slice_ir.regions[ri].polygons` reflect the difference). (Test function name is legacy from the original packet design; renaming is out of scope for this docs-only correction.) | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_subtract_runs_before_paint_segmentation -- --exact --nocapture`
- **Given** the `support_enforcer` synthetic fixture runs through Packet 51's `paint_overrides` overlay, **when** `ResolvedConfig` is resolved for the `PaintSemantic::SupportEnforcer` semantic at any intersecting layer, **then** the resolved config's `support_overhang_angle` field differs from the default `ResolvedConfig` value for the same layer. (The AC originally referenced `support_threshold_angle` as an example; the actual `ResolvedConfig` field is `support_overhang_angle` — see DEV-P56c-6.) | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_flows_through_paint_overrides -- --exact --nocapture`
- **Given** clippy is the lint gate, **when** Step 5 runs, **then** `cargo clippy -p slicer-host --tests -- -D warnings` is green AND `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy -p slicer-host --tests -- -D warnings && cargo clippy --workspace -- -D warnings`
- **Given** the existing regression-defense surfaces must stay GREEN, **when** Step 5 runs, **then** `cargo test -p slicer-host --test threemf_transform_tdd` AND `cargo test -p slicer-host --test gcode_emit_tdd` AND `cargo test -p slicer-host --test benchy_painted_e2e_tdd` AND `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` AND `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` AND `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` all report all-pass. | `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd`
- **Given** TASK-192b, TASK-192c, and TASK-193 are registered by this packet, **when** Step 6 runs, **then** `docs/07_implementation_status.md` contains rows `[x] TASK-192b`, `[x] TASK-192c`, AND `[x] TASK-193` each naming this packet (`56c_threemf-negative-and-support-subtype-routing`). | `rg -q '\[x\] TASK-192b.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-192c.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-193.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md`
- **Given** no new deviations are introduced by this packet, **when** Step 6 runs, **then** `docs/DEVIATION_LOG.md` contains no DEV row whose status column reads `Closed — Packet 56c` (zero rows; the negative ensures no accidental DEV registration). | `! rg -q '^\| DEV-.*Closed.*Packet 56c' docs/DEVIATION_LOG.md`

## Negative Test Cases

- Negative volume entirely above the parent's Z-extent → no subtract occurs at any layer; parent polygons unchanged. Verified by:
  - **Given** an IR fixture where the `negative_part` cube's Z-min is greater than the parent's Z-max, **when** `apply_negative_part_subtract` executes, **then** the parent's per-layer polygons are bit-identical to a baseline run without the negative modifier volume. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_above_parent_no_subtract -- --exact --nocapture`
- `negative_part` volume with zero triangles (degenerate) → `slice_mesh_ex` returns empty per-layer projection; `polygon_ops::difference` is short-circuited and parent polygons unchanged; no warning. Verified by:
  - **Given** an IR fixture where the `negative_part` mesh has zero triangles, **when** `apply_negative_part_subtract` executes per-layer, **then** every `slice_ir.regions[ri].polygons` across all layers is bit-identical to a baseline run without the negative modifier volume AND no warning is emitted. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_negative_part_no_subtract -- --exact --nocapture`
- `support_enforcer` volume with zero triangles (degenerate) → no `PaintRegionIR` entries emitted; no warning. Verified by:
  - **Given** an IR fixture where the `support_enforcer` mesh has zero triangles, **when** `execute_paint_segmentation` runs, **then** for every global layer index `n` the lookup `paint_region_ir.per_layer.get(&n).and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportEnforcer))` is `None` (or `Some(regions)` with `regions.is_empty()`) AND no warning is emitted (degenerate is not an error). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_support_enforcer_emits_nothing -- --exact --nocapture`
- `support_blocker` volume with zero triangles (degenerate) → no `PaintRegionIR` entries emitted; no warning. Verified by:
  - **Given** an IR fixture where the `support_blocker` mesh has zero triangles, **when** `execute_paint_segmentation` runs, **then** for every global layer index `n` the lookup `paint_region_ir.per_layer.get(&n).and_then(|m| m.semantic_regions.get(&PaintSemantic::SupportBlocker))` is `None` (or `Some(regions)` with `regions.is_empty()`) AND no warning is emitted. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd empty_support_blocker_emits_nothing -- --exact --nocapture`

## Deviations

- **DEV-P56c-1 (test pattern)** — Specified: ZipWriter 3MF archives | Implemented: IR struct builders (`box_mesh`, `modifier_volume_with_subtype`, `mesh_ir_with_modifier`) | Reason: IR-level tests are faster and avoid re-testing 3MF parsing (covered by Packets 56/56b).
- **DEV-P56c-2 (object iteration scope)** — Specified: subtract only current object's modifier_volumes | Implemented: iterate ALL objects' modifier_volumes | Reason: Negative volumes are subtractive globally per 3MF spec.
- **DEV-P56c-3 (projection batching)** — Specified: per-layer `slice_mesh_ex(&mv.mesh, &[slice_ir.z])` | Implemented: single batched `slice_mesh_ex(&mv.mesh, &layer_zs)` | Reason: More efficient; same result.
- **DEV-P56c-4 (union merge scope)** — Specified: union-merge into any existing `Vec<SemanticRegion>` | Implemented: union-merge into FIRST existing SemanticRegion only | Reason: `group_and_union_paint_regions` (Packet 64) already unions per-semantic; multi-entry same-(layer,semantic) is theoretical.
- **DEV-P56c-5 (warning assertion gap)** — Specified: tests assert "no `log::warn!` is emitted" for degenerate meshes | Implemented: degenerate meshes are silently skipped (correct behavior) but tests do not assert warning absence | Reason: No log-capture infrastructure; behavior correct by construction (`continue` with no side effects).
- **DEV-P56c-6 (field name)** — Specified: AC-6 referenced `support_threshold_angle` | Implemented: test asserts `support_overhang_angle` | Reason: `support_overhang_angle` is the actual field name in `ResolvedConfig`; the AC's original name was a Packet 51 example key suggestion. The test comment at `threemf_subtypes_synthetic_e2e_tdd.rs:506-507` acknowledges this.

## Verification

- `cargo check --workspace` — compile health.
- `cargo clippy -p slicer-host --tests -- -D warnings` — lint gate (per-crate).
- `cargo clippy --workspace -- -D warnings` — lint gate (workspace).
- `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` — synthetic-fixture E2E.
- `cargo test -p slicer-host --test threemf_transform_tdd` — transform regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — G-code emission regression.
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — no-sidecar E2E regression.
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — paint-semantic regression.
- `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — Packet 56b's modifier-part E2E (must stay green).
- `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — Packet 56's parser suite (must stay green).
- `cargo test --workspace` — acceptance ceremony only (Step 7); dispatched via worker as `FACT pass/fail`. This packet closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice; a workspace-wide gate is justified at terminal closure of the three-way-split.

## Doc Impact Statement

- `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193 rows after TASK-192a (line 144). Verify: `rg -c 'TASK-192[bc]|TASK-193' docs/07_implementation_status.md` → 3.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `PaintRegionIR`, `PaintSemantic` (lines covering paint IR; delegate to the `PaintRegionIR` block search). Informational; no IR edit.
- `docs/04_host_scheduler.md` — prepass / region-mapping ordering. Delegate SUMMARY at Step 2 for the exact insertion point name.
- `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility. Informational.
- `docs/08_coordinate_system.md` — scaled integer units. Read directly.
- `docs/07_implementation_status.md` — append TASK-192b, TASK-192c, TASK-193.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` (or sibling) — negative-part per-layer subtract entry. Delegate ONE Explore agent dispatch at Step 2 with the LOCATIONS contract.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) — support enforcer/blocker geometry paths. Delegate ONE Explore agent dispatch at Step 3 with the LOCATIONS contract.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

Aggregate cost is **M** (3M + 5S step distribution). Downstream agents:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor the out-of-bounds list — `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, OrcaSlicer source, `crates/slicer-sdk/`, `crates/slicer-ir/`, `crates/slicer-host/src/region_mapping.rs`, `crates/slicer-host/src/model_loader.rs` — they must not be loaded directly (the last two were closed by Packets 56 / 56b and are immutable here);
- delegate every `cargo` run via a sub-agent FACT contract;
- stop reading at 60% context and hand off at 85%.

This packet closes the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice. Step 7 runs `cargo test --workspace` exactly once at acceptance ceremony via worker FACT dispatch — the only packet in the three-way split that does so.
