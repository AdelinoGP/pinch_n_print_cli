---
status: implemented
packet: 247-visual-debug-silhouette-core
task_ids:
  - TASK-442
  - TASK-443
  - TASK-444
  - TASK-445
---

# 247-visual-debug-silhouette-core

## Goal

Add the `silhouette` visualization kind (schema 1.2.0) to `pnp_cli visual-debug`: schema gate and full fail-closed validation matrix, one composite X–Z/Y–Z image per (tap, view) rendered through the existing `Projector`/`Canvas` via exact interval projection, model-wide Z framing, the 1.2.0 manifest shape (`view`, `layers_rendered`, optional `layer_index`/`layer_z`), and the first two tap families — the `CapturedIr::Slice` taps and `PrePass::SupportGeometry` (`SupportPlanIR` roles with raft/coarse warnings).

## Problem Statement

`pnp_cli visual-debug` renders only top-down XY views. Tree-support defects — branch tapering, interface-band placement/count, raft/base structure — are vertical: invisible from above and currently diagnosable only from IR JSON or G-code text. The approved plan (`docs/specs/visual-debug-silhouette-side-views-plan.md`, reviewed 2026-08-27) defines a `silhouette` visualization kind (schema 1.2.0) that composites selected layers into one X–Z or Y–Z image per (tap, view) via mathematically exact interval projection. This packet is queue row #1 (plan §4.7 steps 1+2): the tracer over the simplest slab source plus the motivating support-plan use case, carrying every foundation later rows build on — the schema gate, the mixing ban, the composite render path, model-wide Z framing, and the 1.2.0 manifest shape.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Projector single-owner rule (archived spec, binding): the silhouette path feeds `Projector::project(x_or_y_mm, z_mm)` and never defines its own world→pixel transform. `Projector`'s built-in y-flip makes larger Z render toward the top — correct orientation for free.
- Z is mm floats end-to-end (`docs/08_coordinate_system.md`) and must never round-trip through `mm_to_units`; polygon X/Y is read via `Point2::to_mm` only.
- Version-locking (mandatory pattern from the template): 1.0.0/1.1.0 manifests must stay byte-identical. `legend_version_for` (`crates/pnp-cli/src/visual_debug.rs`) already pins 1.0.0 to the literal `"1.0.0"`; this packet extends `schema_supported` without touching either existing branch's output, and pins byte-compat with serialization tests (AC-8), not just parsing tests. `LEGEND_VERSION` is deliberately not bumped (fills, not glyphs).
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): new test literals of watched types (`SliceIR`, `SlicedRegion`, `SupportPlanEntry`, `SupportPlanIR`, `VisualDebugRequest`, …) need `..` FRU or an `// exhaustive: <reason>` waiver; `SupportPlanEntry` has no `Default` — existing suites use the exhaustive waiver, follow them.

## Data and Contract Notes

- IR/manifest contracts: the manifest mirrors the request's declared `schema_version` (existing comment in `run_visual_debug`); 1.2.0 entries may carry `view`/`layers_rendered` and omit `layer_index`/`layer_z`; 1.0/1.1 output is byte-frozen (AC-8). `world_bounds_mm` reuses `ViewportBoundsMm` — inside a silhouette bundle `min_y`/`max_y` carry Z millimeters; legality rests on the mixing ban + one-view-per-bundle + the per-entry `view` field (plan §10 item 1, confirmed).
- WIT boundary: none — no WIT, IR struct, or guest-facing type changes; `ImageEntry` is a Serialize-only CLI type.
- Determinism/scheduler constraints: captures arrive sorted (`STAGE_ORDER` position, then layer) from `execute_blackboard_taps`; rectangle emission is ascending layer → fixed class order → ascending interval start; warnings order W1, W2, occlusion; group order `STAGE_ORDER` position then tap; all sources are `Vec`s (no `HashMap` iteration reaches the silhouette path — `SupportPlanIR.entries` is a `Vec`; entries within a layer sort by `(object_id, region_id)` like `support_geometry_shapes`).

## Locked Assumptions and Invariants

- One silhouette plane per bundle; every silhouette entry in a bundle shares one byte-identical `world_bounds_mm` (extends the pinned fact-6 invariant unchanged for 1.0/1.1 consumers).
- Slabs are `[z − effective_layer_height, z]` per region for `CapturedIr::Slice` taps; `[previous global z (0.0 for the first), z]` for the support tap. `GlobalLayer.z` is the layer **top**, and on a catch-up region the height reaches the catch-up bottom (layer-planner pinned: `effective_layer_height == z − catchup_z_bottom` — an `ActiveRegion`-level invariant; `SlicedRegion` carries only `effective_layer_height`, no catch-up flags, so the renderer needs nothing beyond that one field).
- The silhouette never draws raft entries, coarse `SupportGeometryIR.entries`, sub-pixel inflation, or inferred geometry; every omission is a named warning or a named rejection.
- `SILHOUETTE_TAP_STAGE_IDS` = the four `CapturedIr::Slice` taps + `PrePass::SupportGeometry`; later packets extend it rather than bypassing validation.

## Risks and Tradeoffs

- `execute_blackboard_taps` clones the whole `SupportGeometryIR`+`SupportPlanIR` composite per selected layer (pre-existing behavior); an all-layers support silhouette multiplies that clone by layer count. Accepted for 247 (plans are far smaller than the postpass whole-print IR that motivated D10); if profiling ever shows it matters, a dedup shaped like D10 is the follow-up — do not fix it speculatively here.
- The occlusion warning fires only when overlap actually occurs; a reader of an overlap-free bundle relies on the docs/19 caveat alone. Accepted: an always-on warning would train agents to ignore warnings.
- f32 interval endpoints: exact-comparison unions may leave a sub-pixel seam between two regions that abut at nearly-but-not-exactly equal coordinates; slabs tile exactly by construction (same `layer_z` source), so only horizontal seams from IR-level near-touches can occur, and they render honestly (the gap exists in the IR).
- AC-8 freezes `"layer_z": null` for markerless gcode layers; if a future packet wants absent-when-unknown it must version-gate it.
