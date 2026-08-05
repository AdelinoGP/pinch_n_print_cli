---
status: superseded
packet: 31a_support-geometry-prepass-and-layer-height
task_ids:
  - TASK-163
---

# 31a_support-geometry-prepass-and-layer-height

## Goal

Establish the architectural foundation for variable-height support planning in Pinch 'n Print. Unlike OrcaSlicer, which ties support layer height to model layer height (wasteful for high-resolution prints), Pinch 'n Print already has `LayerPlanIR` before any slicing begins — enabling support planning at a different (coarser) resolution than the model. This packet introduces:

1. **`SupportGeometryIR`** — a new Tier-1 IR type holding per-layer 2D polygon outlines at support layer resolution (not model resolution).
2. **`PrePass::SupportGeometry`** — a lightweight host-built-in prepass stage that computes coarse support outlines via plane-triangle intersection at support layer boundaries only.
3. **`support_layer_height_mm` config key** — new config on both `support-planner` and `tree-support` modules, defaulting to the model layer height (no behavior change by default).
4. **`support_top_z_distance_mm` config key** — special-case refinement near the model: when a support column's top is within this distance of the model, the top few support layers use model resolution (not support resolution) so `support_top_z_distance` is honored precisely. This requires interpolation between support resolution and model resolution at the top of each column.
5. **`tree-support` module updates** — traditional supports also use `support_layer_height_mm`; the `tree-support` emitter handles the height interpolation when emitting support paths at model resolution.

After this packet, support generation can plan at coarse resolution (fast, sparse outlines) while the emitter interpolates down to model resolution for actual path planning. The foundation enables OrcaSlicer-competitive support quality at significantly reduced compute (especially for high-layer-count prints).

## Problem Statement

OrcaSlicer ties support layer height to model layer height — so a 0.1mm miniature print gets 0.1mm support layers. This is wasteful: support structures don't need that resolution. OrcaSlicer computes `lslices` at model resolution for the whole model, then runs tree support — it can't do otherwise because slice data comes from the layer loop.

Pinch 'n Print has a structural advantage: `LayerPlanIR` is committed in Tier 1 (before any slicing) and describes the complete layer sequence. This means we can plan support geometry at a different (coarser) resolution than the model — even determining support layer boundaries before a single triangle is intersected.

This packet establishes the architectural foundation for this differentiator:
1. `SupportGeometryIR` — coarse per-layer polygon outlines at support layer resolution
2. `PrePass::SupportGeometry` — lightweight host-built-in prepass that computes them
3. `support_layer_height_mm` config — enables coarse support resolution
4. `support_top_z_distance_mm` config — refinement near model contact zones

After this packet, support planning runs at coarse resolution (fast, sparse outlines) while the support-planner interpolates down to model resolution near the top of each column. This is a genuine competitive advantage: OrcaSlicer-competitive support quality at significantly reduced compute.

## Architecture Constraints

- **PrePass remains sequential.** `PrePass::SupportGeometry` is a new sequential stage running in Tier 1. It must complete before any user prepass module that reads `SupportGeometryIR` (e.g., `support-planner`) runs.
- **`SupportGeometryIR` is Tier-1-only data.** It is consumed by `PrePass::SupportGeneration` and then dropped from the blackboard (it does not survive into Tier 2). The tree-support module does NOT read `SupportGeometryIR` in Tier 2 — it falls back to its existing grid-MST path which uses model-resolution slices computed in `Layer::Slice`. The "variable support height" feature means `support-planner` emits support at coarse resolution and the emitter interpolates; the tree-support module (which is a Tier 2 consumer) receives already-interpolated support entries and doesn't need direct access to `SupportGeometryIR`.
- **Support layer boundary determination uses `LayerPlanIR`.** The prepass computes support layer boundaries by walking `LayerPlanIR.layers` and emitting a support layer at every layer whose `effective_layer_height >= support_layer_height_mm` threshold (Q1 resolution). This correctly handles catch-up layers: a catch-up layer with `effective_layer_height = 0.3mm` counts toward the support layer sequence even if adjacent model layers are 0.1mm.
- **No new IR type for support entries.** `SupportPlanIR` is unchanged by this packet. The interpolation from coarse support geometry to model resolution happens inside `support-planner` at emit time — each output `SupportPlanEntry` still carries model-layer Z values and effective heights. The coarse `SupportGeometryIR` is only the input geometry; the output is unchanged.
- **Additive WIT change.** Adding parameters to `run-support-generation` is treated as an additive minor revision per `docs/03_wit_and_manifest.md §additive change rule`. All prepass `.wasm` artifacts must be rebuilt.
- **Determinism.** The projector sorts by `(global_support_layer_index ASC, object_id ASC, region_id ASC)`. Polygon union uses deterministic algorithms.

## Data and Contract Notes

- **`SupportGeometryIR`:**
  ```rust
  pub struct SupportGeometryIR {
      pub schema_version: SemVer { major: 1, minor: 0, patch: 0 },
      pub support_layer_height_mm: f32,  // 0.0 = use model layer height
      pub support_top_z_distance_mm: f32,
      pub entries: HashMap<SupportGeometryKey, Vec<ExPolygon>>,
  }

  pub struct SupportGeometryKey {
      pub global_support_layer_index: u32,  // index into the coarse support layer sequence
      pub object_id: ObjectId,
      pub region_id: RegionId,
  }

  // Intermediate layers near model contact (within support_top_z_distance):
  // global_support_layer_index = u32::MAX indicates "model layer" (not a support layer)
  ```
- **WIT records:**
  - `record support-geometry-view-entry { global-support-layer-index: layer-idx, object-id: object-id, region-id: region-id, outlines: list<ex-polygon> }`
  - `record support-geometry-view { entries: list<support-geometry-view-entry> }`
- **Export signature:** `export run-support-generation: func(objects: list<mesh-object-view>, layer-plan: layer-plan-view, region-segmentation: region-segmentation-view, support-geometry: support-geometry-view, output: support-generation-output, config: config-view) -> result<_, module-error>;`
- **Prerequisite slice for `PrePass::SupportGeneration`:** `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]`.
- **Support layer boundary formula (Q1):** Walk `LayerPlanIR.layers` accumulating height. When accumulated height >= `support_layer_height_mm`, emit a support layer boundary at the current layer's Z. A catch-up layer's full `effective_layer_height` counts toward the accumulation. This is computed by `support_layer_boundaries()` helper.
- **Interpolation formula:** For each support layer at Z_s with outline O_s, find model layers within `support_top_z_distance_mm` of the top of the support column. Emit additional `SupportPlanEntry` records at those model layer Z values with outlines linearly interpolated between O_s and the next support layer's outline O_{s+1}.

## Locked Assumptions and Invariants

1. `LayerPlanIR.layers` is committed before any prepass stage runs. The host can safely read it to compute support layer boundaries.
2. `MeshIR` triangles are available during `PrePass::SupportGeometry` for plane-triangle intersection.
3. `support_layer_height_mm = 0.0` is the sentinel meaning "use model layer height" — when this is the case, `SupportGeometryIR` has one entry per model layer (1:1 mapping).
4. `SupportGeometryIR` does not survive into Tier 2. It is consumed by `PrePass::SupportGeneration` and then dropped.
5. `support-planner` emits `SupportPlanEntry` records at model resolution (model Z values, model effective heights). The coarse `SupportGeometryIR` is only the input geometry; output Z values are model-layer Z values.
6. The tree-support module falls back to grid-MST when no `support-planner` is loaded. This fallback path uses `SliceIR` from Tier 2, unchanged by this packet.
7. Packet 30's `LayerPlanView` and `RegionSegmentationView` shapes are stable and are the only WIT inputs to `run-support-generation` (this packet adds `SupportGeometryView` as a fourth).
8. No IR schema versions bump for `SupportPlanIR` — the interpolation happens at emit time inside the planner and produces standard `SupportPlanEntry` records.

## Risks and Tradeoffs

- **Risk: support layer boundary computation ignores per-region layer height variation.** If different regions have different layer heights (due to modifiers), the support layer boundary computation uses the global layer sequence, not per-region sequences. This is acceptable for v2 — per-region support height variation is out of scope.
- **Risk: interpolation produces incorrect outline shapes near column tops.** The linear interpolation between coarse support outlines may not match the actual model geometry at high resolution. Mitigation: `support_top_z_distance_mm` refinement adds actual model-resolution outline layers near contact zones, reducing interpolation error at the critical interface.
- **Risk: WIT change forces rebuild of all prepass `.wasm` artifacts.** Mitigation: the rebuild is a one-time cost; no ongoing penalty.
- **Tradeoff: plane-triangle intersection in the host vs guest.** `PrePass::SupportGeometry` runs in the host (not a guest module), so it has direct access to `MeshIR` and can compute outlines without crossing the WIT boundary. This is the right choice — the computation is fast (coarse resolution only) and the host already has the data.
