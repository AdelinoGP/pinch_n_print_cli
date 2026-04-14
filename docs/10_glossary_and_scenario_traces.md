# ModularSlicer — Canonical Glossary & Scenario Traces

This document is normative for term definitions and end-to-end behavior traces used in architecture reviews and implementation validation.

---

## Canonical Glossary

| Term | Definition | Invariants |
|---|---|---|
| Blackboard | Host-owned shared state populated during PrePass and treated as read-only during per-layer execution. | No module may mutate Blackboard directly in Tier 2. |
| Global layer | One authoritative Z plane in `LayerPlanIR.global_layers`. | `global_layer_index` is monotonic and unique. |
| Object-local layer | Layer index relative to one object (`ObjectLayerRef.local_layer_index`). | Mapping to global index is deterministic through `object_participation`. |
| Active region | One `(object_id, region_id)` at one global layer with fully resolved config. | Contains final resolved config; no runtime fallback chain. |
| Sync layer | Global layer where objects with heterogeneous layer heights align on common Z. | Derived from planning, not recomputed at runtime. |
| Catch-up layer | Region-layer where an object spans from prior local Z to current sync Z. | `is_catchup_layer=true`, `catchup_z_bottom < layer.z + effective_layer_height`. |
| Claim | Exclusive capability slot (for example `infill-generator`). | Exactly one holder per `(layer, object, region, claim)` at execution. |
| Region override | Configuration or module selection override applied at region scope. | Must not produce non-deterministic claim-holder ties. |
| Degraded success | Slice completes after one or more non-fatal module failures. | Must emit `module_error` events and set `degraded=true` in final result metadata. |
| Fatal failure | Module/contract/integrity error requiring immediate abort. | Slice command terminates; no silent continuation. |
| Paint semantic | Typed meaning for paint (`Material`, `FuzzySkin`, `SupportEnforcer`, `SupportBlocker`, `Custom`). | Overlap resolution uses deterministic precedence rules. |
| PaintRegionIR | Per-layer semantic paint polygons computed in PrePass. | Treated read-only in Tier 2 and queried by semantic. |
| SlicedRegion | One region entry in `SliceIR` after slicing and polygon processing. | `boundary_paint` cardinality must align with contour points. |
| WallLoop | One perimeter loop entity in `PerimeterIR`. | Segment features are driven by `feature_flags` parallel to path segments. |
| feature_flags | Segment-level wall metadata propagated from boundary paint. | Length and indexing must remain deterministic through wall transforms. |
| paint_order | Deterministic tie-break key for paint overlap resolution. | Equal-precedence conflicting values are fatal. |
| boundary_paint | Per-contour-point semantic paint annotations in `SlicedRegion`. | Must exist (possibly defaulted) before `Layer::Perimeters` consumption. |

---

## Scenario Trace 1 — Mixed Layer Heights + Catch-Up

### Inputs
- Object A layer height: `0.20 mm`
- Object B layer height: `0.30 mm`
- Shared claim: `infill-generator`
- Region overrides: none

### Planned global layers
- `Z = [0.20, 0.30, 0.40, 0.60, 0.80, 0.90, ...]`
- Sync at `0.60 mm` and `1.20 mm`

### Execution trace (first sync window)
1. `PrePass::LayerPlanning` emits sync at `0.60`.
2. At global layer `0.40`, Object A has normal local layer; Object B is inactive.
3. At global layer `0.60`, Object A has normal local layer; Object B emits catch-up layer with `catchup_z_bottom=0.30`, `effective_layer_height=0.30`.
4. `PrePass::PaintSegmentation` projects paint polygons using authoritative global Z list.
5. `PrePass::RegionMapping` resolves one infill claim holder per active region.

### Expected outcomes
- No claim transitions across layers for same object.
- Catch-up metadata is present only where required.
- No per-layer recomputation of layer planning or claim resolution.

---

## Scenario Trace 2 — Paint-Heavy Multi-Material + Overlaps

### Inputs
- Two tools (`T0`, `T1`) with `Material` paint.
- `FuzzySkin=true` on subset of outer perimeter segments.
- Overlapping `SupportEnforcer=true` and `SupportBlocker=true` in one zone.
- Custom semantic: `Custom(com.example.texture/roughness@1)`.

### Execution trace
1. `PrePass::MeshSegmentation` normalizes sub-facet strokes to deterministic triangle assignments.
2. `PrePass::PaintSegmentation` emits `PaintRegionIR` per semantic per layer with `paint_order`.
3. `Layer::SlicePostProcess` annotates `SlicedRegion.boundary_paint` after polygon edits.
4. `Layer::Perimeters` maps boundary paint to `WallLoop.feature_flags` and material boundaries.
5. `Layer::PerimetersPostProcess` applies perpendicular XY fuzzy perturbation only where `feature_flags.fuzzy_skin=true`.
6. `Layer::Support` applies support precedence: blocker over enforcer.

### Expected outcomes
- At overlap points, support is blocked (`SupportBlocker` wins).
- Material boundary segments include `WallBoundaryType::MaterialBoundary` where adjacent tool differs.
- Custom paint overlap uses highest `paint_order`; equal-order conflicting values are fatal.

---

## Scenario Trace 3 — Mid-Layer Module Failure

### Inputs
- `com.community.fuzzy-skin` in `Layer::PerimetersPostProcess`.
- Layer `42` contains malformed module output (`feature_flags` cardinality mismatch).

### Execution trace (non-fatal path)
1. Module returns `module-error { fatal=false, code=..., message=... }`.
2. Host emits `module_error` event with `status=non_fatal_error` for layer `42`.
3. Host keeps pre-stage `PerimeterIR` for this module invocation and continues downstream stages.
4. Slice completes with `degraded=true` in `slice_complete` summary.

### Execution trace (fatal path)
1. Module returns `fatal=true` or host contract validation fails.
2. Host emits `module_error` event with `status=fatal_error`.
3. Slice command aborts immediately; no further layer processing.

### Expected outcomes
- Non-fatal failures are never silent.
- Fatal failures never continue execution.
- Frontend can distinguish successful vs degraded vs aborted from emitted events.

---

## Compliance Checklist

A documentation or implementation update is compliant with this spec only if all are true:
- Uses glossary terms exactly as defined above.
- Preserves deterministic claim-holder and overlap behavior.
- Preserves explicit degraded/fatal error semantics and event visibility.
- Keeps mixed-height catch-up behavior aligned with `LayerPlanIR` as source of truth.

## Scenario Validation Artifacts

Each scenario should be mapped to a runnable validation artifact:
- Scenario 1 → catch-up planning fixture + assertion on sync/catch-up metadata.
- Scenario 2 → paint overlap fixture + assertion on precedence and fuzzy/material propagation.
- Scenario 3 → failure-injection fixture + assertion on degraded/fatal event behavior.

Evidence files should be stored under:
- `./docs/evidence/<release-id>/scenario-1-*`
- `./docs/evidence/<release-id>/scenario-2-*`
- `./docs/evidence/<release-id>/scenario-3-*`
