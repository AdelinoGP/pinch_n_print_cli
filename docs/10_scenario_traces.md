# ModularSlicer — Canonical Scenario Traces

This document is normative for the end-to-end behavior traces used in
architecture reviews and implementation validation.

Term definitions live in the project glossary at `../CONTEXT.md` (concept-level
vocabulary) and in the authoritative contract docs (`docs/02_ir_schemas.md`,
`docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`) for IR/WIT/SDK names.

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

## Scenario Trace 4 — Planner-Consuming Tree Support

### Inputs

- One overhanging object printed with `support_enabled = true`.
- Module set installs both `support-planner` (PrePass) and `tree-support`
  (Layer::Support). `traditional-support` is not installed for this scenario.
- `tree-support.toml` declares `SupportPlanIR` as a manifest read.

### Execution trace

1. `PrePass::MeshAnalysis` populates `SurfaceClassificationIR` (host built-in).
2. `PrePass::LayerPlanning` commits `LayerPlanIR`.
3. `PrePass::SupportGeometry` runs the `support-planner`; the host built-in commits `SupportGeometryIR` first, then guests emit `SupportPlanIR` via `run-support-geometry`:
   - `detect_overhangs` extracts contact points from overhang/bridge facets and
     `SupportEnforcer` paint regions (drops contacts inside `SupportBlocker`).
   - Top-down propagation (per-layer Prim MST merge-then-move) produces
     `SupportPlanIR.entries` keyed by `(global_layer_index, object_id, region_id)`.
4. Per-layer rayon tier runs.
5. `Layer::Support` for the `tree-support` module looks up
   `SupportPlanIR.entries` matching the current `(layer, object, region)`:
   - When entries exist: emit their `branch_segments` directly with
     `ExtrusionRole::SupportMaterial`, skip the grid-MST filler.
   - When no entries exist: fall back to the per-layer grid-MST filler
     (byte-identical to packet 26 baseline).

### Expected outcomes

- The committed `SupportIR.support_paths` for the planner-driven layers match
  the planner's `branch_segments` byte-for-byte (≤ 1e-4 mm tolerance).
- Without a `support-planner` module installed, the same `tree-support`
  module emits identical paths to the pre-planner baseline.
- Re-running the planner on the same fixture yields byte-identical
  `SupportPlanIR` (deterministic node ordering and MST tie-break).

### Negative cases (normative)

- Empty overhangs + no enforcer paint → `SupportPlanIR.entries` is empty and
  the planner returns `Ok(())` (no `ModuleError`).
- `PrePass::SupportGeometry` scheduled before `LayerPlanIR` is committed →
  `PrepassExecutionError::MissingRequiredPrepass { slot: LayerPlan }` aborts
  the prepass before any module runs.
- Two modules declaring `holds = ["support-planner"]` on the same stage →
  alphabetical first-winner dedup keeps one and emits a `DiagnosticLevel::Info`
  diagnostic naming the dropped module.

---

## Compliance Checklist

A documentation or implementation update is compliant with this spec only if all are true:

- Uses glossary terms exactly as defined in `../CONTEXT.md` and the contract docs.
- Preserves deterministic claim-holder and overlap behavior.
- Preserves explicit degraded/fatal error semantics and event visibility.
- Keeps mixed-height catch-up behavior aligned with `LayerPlanIR` as source of truth.

## Scenario Validation Artifacts

Each scenario should be mapped to a runnable validation artifact:

- Scenario 1 → catch-up planning fixture + assertion on sync/catch-up metadata.
- Scenario 2 → paint overlap fixture + assertion on precedence and fuzzy/material propagation.
- Scenario 3 → failure-injection fixture + assertion on degraded/fatal event behavior.
- Scenario 4 → overhang fixture + `prepass_support_generation_tdd` (positive,
  empty-overhang, missing-`LayerPlanIR`, dedup, determinism) and
  `live_support_generation_tdd::planner_consuming_tier` (plan-driven emission,
  fallback, traditional-support no-op).

Evidence files should be stored under:

- `./docs/evidence/<release-id>/scenario-1-*`
- `./docs/evidence/<release-id>/scenario-2-*`
- `./docs/evidence/<release-id>/scenario-3-*`

<!-- VERIFY: `docs/evidence/` does not exist at the time of writing. The
     path is the prescribed location for the Architecture Acceptance Gate
     evidence (see `docs/11_operational_governance_and_acceptance_gate.md`);
     it will be created when evidence is first staged. -->

- `./docs/evidence/<release-id>/scenario-4-*`
