---
status: implemented
packet: 30_support-planner-prepass-wit-plumbing
task_ids:
  - TASK-162
---

# 30_support-planner-prepass-wit-plumbing

## Goal

Close the two correctness gaps that packet 28 carved out as v1 limitations of `support-planner`: (a) the planner is layer-height-agnostic because `LayerPlanIR.layers` is not surfaced through the prepass WIT, and (b) every emitted entry uses the canonical `region_id = "0"` bucket because per-region segmentation is not surfaced either. This packet extends `run-support-geometry` to receive `layer-plan-view` and `region-segmentation-view` inputs derived from the committed `LayerPlanIR` and `RegionMapIR`, updates `support-planner` to walk the real layer plan and emit one entry per `(layer, object, region)` triple that exists in `RegionMapIR`, restores `LayerPlanIR` to the planner's manifest reads, adds `RegionMapIR` to those reads, and adds `RegionMap` to the host-side prerequisite chain for `PrePass::SupportGeometry`. Branch geometry remains object-wide (the same segment set is emitted under each region the object owns on a layer); geometry-aware multi-region branch placement is deferred to packet `31b`.

## Problem Statement

Packet `28_tree-support-multi-layer-propagation` introduced `PrePass::SupportGeometry`, the `SupportPlanIR` blackboard contract, and the `support-planner` core module. To stay inside one bounded slice, packet 28 deliberately accepted two correctness gaps as v1 limitations and documented them in `modules/core-modules/support-planner/src/lib.rs`:

1. **Layer-height-agnostic.** The planner walks `(bmax[2] - bmin[2]) / DEFAULT_LAYER_HEIGHT_MM` instead of consulting the committed `LayerPlanIR.layers`. This works for the test fixtures that use a uniform 0.2 mm layer height but silently produces wrong layer counts and Z-values for any object configured with variable layer heights. `LayerPlanIR` was dropped from the planner's manifest `[ir-access].reads` because the prepass WIT does not surface it to the guest.
2. **Single-region per object.** Every emitted `SupportPlanEntry` uses the canonical `region_id = "0"` bucket because `MeshObjectView` does not carry per-region segmentation. Single-region fixtures match correctly because tree-support invokes `support_plan_segments_for(object_id, 0)`; multi-region objects collapse all branches into the first region.

Both gaps are correctness bugs (not just incomplete OrcaSlicer parity) and both block the algorithmic packets that follow: the avoidance/collision cache needs the real layer plan, and any geometry-aware multi-region branch placement needs real region IDs on each entry.

This packet closes both gaps by extending `run-support-geometry` to receive a `layer-plan-view` and a `region-segmentation-view` projected from the committed `LayerPlanIR.layers` and `RegionMapIR.entries`. The planner walks the real layer plan and emits one `SupportPlanEntry` per `(layer, object, region)` triple that exists in `RegionMapIR`. Branch geometry remains object-wide for now — every region under one object on a layer receives the same branch segments — because true geometry-aware multi-region placement requires per-layer slice polygons that are deferred to packet `31b`.

This packet does **not** supersede packet 28; it extends the v1 contract additively and removes the documented v1 carve-outs.

## Architecture Constraints

- **Additive WIT change rebuild cascade.** Every prepass `wit-guest/` and resulting `.wasm` must be rebuilt because the WIT package version is `slicer:world-prepass@1.0.0` and changing any export shape inside that package invalidates all bindings in the package. Existing prepass modules' export signatures (`run-mesh-segmentation`, `run-mesh-analysis`, `run-layer-planning`, `run-paint-segmentation`, `run-seam-planning`) are not changed by this packet, but the package as a whole must compile cleanly under the extended `run-support-geometry` shape. We do not bump the WIT package version (still `1.0.0`); per `docs/03_wit_and_manifest.md` §additive change rule, adding parameters to one export is treated as an additive minor revision so long as it ships before any external consumer locks the v1.0.0 shape. The package's authoritative version line (`package slicer:world-prepass@1.0.0;`) is unchanged; the new records and parameters carry no version annotation.
- **`ensure_stage_prerequisites` single source of truth.** Adding `RegionMap` to the prerequisite slice is the only place the new dependency is declared. The planner module does not duplicate the check — it trusts that the slot is committed when its export is invoked.
- **Blackboard slots remain write-once.** No new slot is introduced; this packet only adds a read of the existing `RegionMap` slot via the host-side projector.
- **Manifest reads must include every IR the planner reads at runtime.** `LayerPlanIR` and `RegionMapIR` move into `[ir-access].reads` because the host now projects their data through the WIT into the guest. The host-side audit (`runtime_reads ⊆ manifest_reads`) must continue to pass.
- **Coordinate system invariant.** `LayerPlanIR.layers[*].z` is mm-valued (per `docs/02_ir_schemas.md`). The WIT projector passes Z values through unchanged. The planner emits `Point3WithWidth.z` in mm to match the existing contract.
- **Determinism.** `RegionMapIR.entries` is a `HashMap` — projection into the WIT view must sort by `(global_layer_index ASC, object_id ASC, region_id ASC)` so two identical runs produce byte-identical `LayerPlanView` and `RegionSegmentationView` inputs and therefore byte-identical `SupportPlanIR.entries`. Packet 28's `support_planner_is_deterministic_across_runs` test must continue to pass.

## Data and Contract Notes

- **WIT records:**
  - `record layer-plan-view-entry { global-layer-index: layer-idx, z: f32, effective-layer-height: f32 }`
  - `record layer-plan-view { layers: list<layer-plan-view-entry> }`
  - `record region-segmentation-view-entry { object-id: object-id, layer-index: layer-idx, region-ids: list<region-id> }`
  - `record region-segmentation-view { entries: list<region-segmentation-view-entry> }`
- **Export signature:** `export run-support-geometry: func(objects: list<mesh-object-view>, layer-plan: layer-plan-view, region-segmentation: region-segmentation-view, output: support-geometry-output, config: config-view) -> result<_, module-error>;`
- **Determinism contract:** the host projector iterates `RegionMapIR.entries` after sorting keys by `(layer_index, object_id, region_id)` (all ascending). Two consecutive runs must produce byte-identical `LayerPlanView` and `RegionSegmentationView` inputs. The packet 28 determinism test (`support_planner_is_deterministic_across_runs`) must continue to pass.
- **Branch geometry vs region keying:** the planner emits the same `branch_segments` `Vec<ExtrusionPath3D>` under each region the object owns on a layer. Tree-support's `support_plan_segments_for(object, region)` lookup matches by `region_id` so any region the object owns will find branches.
- **Host-side prerequisite chain:** `required_slots("PrePass::SupportGeometry")` becomes `&[SurfaceClassification, LayerPlan, RegionMap]`. Order matters for the negative-AC error message (the host reports the first missing slot). Existing AC-4 from packet 28 (`prepass_support_generation_fails_without_layer_plan`) must still pass — it uses a fixture missing `LayerPlanIR`, and `LayerPlan` precedes `RegionMap` in the slice.

## Locked Assumptions and Invariants

1. `LayerPlanIR.layers` is committed with non-empty `layers` whenever `LayerPlanning` runs successfully. The host audit asserts this; the planner can therefore treat an empty `LayerPlanView.layers` as a host-internal error and return `ModuleError::fatal`.
2. `RegionMapIR.entries` keys are unique `(layer_index, object_id, region_id)` triples (per `docs/02_ir_schemas.md`).
3. `region-id` is a string at the WIT layer (`type region-id = string;` in `wit/deps/ir-types.wit`) and a `u64` in the host IR; the existing `parse_canonical_region_id` helper in `crates/slicer-host/src/dispatch.rs` round-trips both. The host projector emits canonical decimal strings for region IDs in `RegionSegmentationView` and the planner passes them straight through to the output.
4. Packet 28's claim layout (`support-planner` claim on `PrePass::SupportGeometry`) is unchanged. No other packet holds that claim.
5. The packet 26 and packet 28 acceptance tests (Benchy + tree-support fallback) still exercise the grid-MST fallback path when no `support-planner` is loaded — this packet does not change the fallback path.
6. No IR schema versions bump. Adding fields to a WIT view that only `support-planner` consumes does not affect any persisted IR.

## Risks and Tradeoffs

- **Risk: rebuild cascade slows local builds.** Every prepass module's `wit-guest/` rebuilds when the WIT package's binding output changes. Mitigation: order Step 6 (rebuild cascade) at the end of the implementation plan so the host code path is verified first; one rebuild covers all modules.
- **Risk: host projector ordering bug breaks determinism.** `HashMap` iteration is non-deterministic. Mitigation: explicit sort by `(layer, object, region)` in the projector with a determinism test asserting byte-identical projections across two runs.
- **Risk: a prepass module other than `support-planner` accidentally accepts the new args via the macro.** Mitigation: the macro routes args based on `stage.id`; the `PrepassModule::run_support_geometry` default body returns `unimplemented`, so any module declaring a different stage id never sees the new args. Verify with a build of every existing prepass core module.
- **Tradeoff: branch geometry is still object-wide.** This is correct for single-region objects (the common case) and produces over-emission (duplicate segments per region) for multi-region objects. Geometry-aware multi-region placement is deferred to packet `31b` because it requires per-layer slice polygons that this packet does not surface.
- **Tradeoff: fatal `ModuleError` on empty `LayerPlanView`.** A bug elsewhere in the host that produces an empty layer plan now turns into a hard module failure rather than a silent no-op. This is intentional — silent no-ops are the v1 behavior we are correcting.
