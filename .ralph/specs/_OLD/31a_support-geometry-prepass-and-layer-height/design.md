# Design: 31a_support-geometry-prepass-and-layer-height

## Controlling Code Paths

- **Primary code paths:**
  - `crates/slicer-ir/src/slice_ir.rs` — `SupportGeometryIR` struct + `SupportGeometryKey` type.
  - `crates/slicer-host/src/blackboard.rs` — `BlackboardPrepassSlot::SupportGeometry`, `commit_support_geometry`, `support_geometry()`.
  - `crates/slicer-host/src/prepass.rs` — `PrePass::SupportGeometry` built-in computation in `execute_prepass_with_builtins`; `required_slots` extension.
  - `crates/slicer-host/src/wit_host.rs` — `project_support_geometry_view` projector.
  - `wit/world-prepass.wit` — new WIT records and extended `run-support-generation` signature.
  - `crates/slicer-sdk/src/prepass_types.rs` + `prelude.rs` — `SupportGeometryView`, `SupportGeometryViewEntry`.
  - `crates/slicer-sdk/src/traits.rs` — `PrepassModule::run_support_generation` signature extension.
  - `crates/slicer-macros/src/lib.rs` — macro arg routing for new WIT param.
  - `modules/core-modules/support-planner/support-planner.toml` — config schema + reads.
  - `modules/core-modules/tree-support/tree-support.toml` — config schema (traditional supports).
  - `modules/core-modules/support-planner/src/lib.rs` — support interpolation from coarse to model resolution near column tops.
  - `modules/core-modules/tree-support/src/lib.rs` — fallback path + optional SupportGeometryIR consumption.
- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` (new).
  - Existing tests from packets 28 and 30 must remain green.
- **No OrcaSlicer reference for the core differentiator** — variable support layer height is a Pinch 'n Print innovation, not an OrcaSlicer feature.

## Architecture Constraints

- **PrePass remains sequential.** `PrePass::SupportGeometry` is a new sequential stage running in Tier 1. It must complete before any user prepass module that reads `SupportGeometryIR` (e.g., `support-planner`) runs.
- **`SupportGeometryIR` is Tier-1-only data.** It is consumed by `PrePass::SupportGeneration` and then dropped from the blackboard (it does not survive into Tier 2). The tree-support module does NOT read `SupportGeometryIR` in Tier 2 — it falls back to its existing grid-MST path which uses model-resolution slices computed in `Layer::Slice`. The "variable support height" feature means `support-planner` emits support at coarse resolution and the emitter interpolates; the tree-support module (which is a Tier 2 consumer) receives already-interpolated support entries and doesn't need direct access to `SupportGeometryIR`.
- **Support layer boundary determination uses `LayerPlanIR`.** The prepass computes support layer boundaries by walking `LayerPlanIR.layers` and emitting a support layer at every layer whose `effective_layer_height >= support_layer_height_mm` threshold (Q1 resolution). This correctly handles catch-up layers: a catch-up layer with `effective_layer_height = 0.3mm` counts toward the support layer sequence even if adjacent model layers are 0.1mm.
- **No new IR type for support entries.** `SupportPlanIR` is unchanged by this packet. The interpolation from coarse support geometry to model resolution happens inside `support-planner` at emit time — each output `SupportPlanEntry` still carries model-layer Z values and effective heights. The coarse `SupportGeometryIR` is only the input geometry; the output is unchanged.
- **Additive WIT change.** Adding parameters to `run-support-generation` is treated as an additive minor revision per `docs/03_wit_and_manifest.md §additive change rule`. All prepass `.wasm` artifacts must be rebuilt.
- **Determinism.** The projector sorts by `(global_support_layer_index ASC, object_id ASC, region_id ASC)`. Polygon union uses deterministic algorithms.

## Code Change Surface

### Selected approach

**Host-built-in `PrePass::SupportGeometry` computes coarse outlines at support layer resolution; `support-planner` consumes them and interpolates to model resolution at column tops.**

`LayerPlanIR` is already committed before any prepass stage runs. The host walks `LayerPlanIR.layers` to compute support layer boundaries. For each support layer boundary Z, the host runs plane-triangle intersection on `MeshIR` triangles to collect all polygons at that Z (one polygon per object region). These polygons are unioned per `(object_id, region_id)` to produce the coarse outline set.

Near model contact zones (within `support_top_z_distance_mm` of the top of any support column), additional intermediate outline layers are computed at model resolution so that `support_top_z_distance` is honored precisely. These intermediate layers are keyed by their actual model layer index (not a support layer index).

`SupportGeometryIR` is committed to the blackboard and read by `support-planner` through `SupportGeometryView`. The planner's propagation loop operates at support resolution (using `SupportGeometryView` outlines for collision). When emitting `SupportPlanEntry` output, the planner interpolates to model resolution near column tops: for each support layer entry, if the model's top layers are within `support_top_z_distance`, additional entries are emitted at the model layer Z values with interpolated outline data.

### Exact functions, traits, manifests, tests, or fixtures expected to change

**Created:**
- `crates/slicer-ir/src/slice_ir.rs` — `SupportGeometryIR` struct and `SupportGeometryKey` type (keyed by `(global_support_layer_index, object_id, region_id)`).
- `crates/slicer-host/src/blackboard.rs` — `BlackboardPrepassSlot::SupportGeometry` variant, `commit_support_geometry`, `support_geometry()` accessor.
- `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` — at least 6 tests (5 positive + 3 negatives from ACs).
- `crates/slicer-ir/src/slice_ir.rs` (extension) — helper for support layer boundary computation: `fn support_layer_boundaries(layers: &[LayerPlanEntry], support_height_mm: f32) -> Vec<SupportLayerBoundary>`.

**Modified — WIT and SDK:**
- `wit/world-prepass.wit` — add 2 records, extend `run-support-generation` signature.
- `crates/slicer-sdk/src/prepass_types.rs` — `SupportGeometryViewEntry`, `SupportGeometryView`.
- `crates/slicer-sdk/src/prelude.rs` — re-export new types.
- `crates/slicer-sdk/src/traits.rs` — extend `PrepassModule::run_support_generation` signature.
- `crates/slicer-macros/src/lib.rs` — thread `support_geometry_view` arg.

**Modified — host:**
- `crates/slicer-host/src/prepass.rs` — `PrePass::SupportGeometry` in `execute_prepass_with_builtins`; `required_slots` extension.
- `crates/slicer-host/src/wit_host.rs` — `project_support_geometry_view` + dispatcher wiring.

**Modified — modules:**
- `modules/core-modules/support-planner/support-planner.toml` — `[config.schema]` + `[ir-access].reads += "SupportGeometryIR"`.
- `modules/core-modules/tree-support/tree-support.toml` — `[config.schema]` additions (traditional supports).
- `modules/core-modules/support-planner/src/lib.rs` — support interpolation logic from coarse to model resolution near column tops.
- `modules/core-modules/tree-support/src/lib.rs` — fallback path when `SupportGeometryIR` unavailable.

**Modified — backlog:**
- `docs/07_implementation_status.md` — `TASK-163` row.

### Rejected alternatives

- **Read `SliceIR` in `PrePass::SupportGeneration` (original packet 31 design).** Rejected because `SliceIR` is Tier-2 data (produced by `Layer::Slice`). `PrePass::SupportGeneration` runs in Tier 1 before any slicing. Reading `SliceIR` in prepass would require restructuring the entire pipeline to slice before support planning — exactly what OrcaSlicer does, which eliminates the variable support layer height advantage entirely.
- **Compute support outlines lazily inside the support-planner.** Rejected — the planner would need to re-run plane-triangle intersection on every propagation step, and the results would not be reusable across module invocations. A dedicated prepass stage with a committed IR is cleaner and allows the host to cache results.
- **`SlicePreviewIR` instead of `SupportGeometryIR`.** Rejected — the name `SlicePreview` implied "a preview of the full slice" which is not accurate. `SupportGeometryIR` correctly describes the data's purpose: coarse support geometry at support layer resolution, not a preview of model slicing.
- **Make `SupportGeometryIR` survive into Tier 2.** Rejected — the tree-support module (Tier 2) falls back to grid-MST when no `support-planner` is loaded, which uses model-resolution slices from `Layer::Slice`. If we make `SupportGeometryIR` survive to Tier 2, every Tier 2 consumer needs to handle it. Keeping it Tier-1-only is simpler and correct: the `support-planner` produces `SupportPlanIR` entries at model resolution; the tree-support module receives those entries and emits them without needing to read `SupportGeometryIR` directly.

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

## Open Questions

All open questions are resolved.

- **Q1 (resolved):** Support layer boundary computation — **(a) Accumulator approach**: walk `LayerPlanIR.layers` accumulating `effective_layer_height`; when accumulated >= `support_layer_height_mm`, emit a support layer boundary at that layer's Z. Catch-up layers count their full `effective_layer_height`. Handles variable heights and catch-up correctly; more robust than fixed-ratio.
- **Q2 (resolved):** How `support_top_z_distance` refinement works — **(a) Intermediate model-resolution layers**: for each support column, add `SupportGeometryIR` entries at every model layer within `support_top_z_distance_mm` of the contact Z. These use `global_support_layer_index = u32::MAX` (sentinel for "model layer, not support layer"). `u32::MAX` is safe — at 0.05mm min layer height, you'd need 85,900 layers (8.6km of print) to collide with it. Actual computed polygons, not interpolation at emit time.
- **Q3 (resolved):** Sentinel for "use model layer height" — **(a) `0.0`**. Intuitive; `min > 0` in the config schema ensures 0.0 is never a valid layer height. Cleaner than `-1.0`.
- **Q4 (resolved):** `SupportGeometryIR` is Tier-1-only and does not survive into Tier 2. Tree-support module falls back to grid-MST path (unchanged) when no `support-planner` is loaded.
