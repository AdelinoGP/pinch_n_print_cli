---
status: draft
packet: 62_paint-annotator-performance
task_ids:
  - TASK-130c   # [x] completed — provides ExPolygon-bearing paint-region transport
  - TASK-181    # [x] completed — provides paint-semantic-aware RegionMap
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Builds on completed TASK-130c and TASK-181 infrastructure. No open TASK-### directly covers post-completion optimization of this pipeline stage.
---

# Packet Contract: 62_paint-annotator-performance

## Goal

Reduce `com.host.slice-postprocess-paint-annotator` wall-clock from ~188 s to single-digit seconds on benchy-class prints by (1) unioning per-facet paint regions at harvest, (2) caching redundant `paint_regions.get()` lookups, (3) adding `BoundingBox2` AABB pre-filter on `SemanticRegion`, (4) parallelizing contour point annotation with `rayon::par_iter()`, and (5) adding an early-break heuristic in `point_in_paint_region`.

## Scope Boundaries

This packet optimizes the paint annotation query path end-to-end: from `harvest_paint_segmentation_ir` in `dispatch.rs` (where per-facet fragmentation enters the IR) through `point_in_paint_region` and `semantic_region_contains_point` in `paint_region.rs` (the O(regions × polygons × edges) query hot path) and `execute_slice_postprocess_paint_annotation` in `slice_postprocess.rs` (the contour-point loop). It adds one new IR type (`BoundingBox2`) and one new `SemanticRegion` field (`aabb`), both serialization-skipped. The host executor path in `paint_segmentation.rs` and the `slicer_core::union` hole-loss limitation are noted but out of scope. A follow-up packet (`63_paint-annotator-r-tree`) adds an R-tree spatial index atop the `BoundingBox2` infrastructure.

## Prerequisites and Blockers

- Depends on: TASK-130c (paint-region transport with ExPolygon support), TASK-181 (paint-semantic-aware RegionMap) — both `[x]` completed
- Unblocks: `63_paint-annotator-r-tree` (reuses `BoundingBox2` type and `SemanticRegion.aabb` field)
- Activation blockers: None

## Acceptance Criteria

- **AC-1. Given** a `PaintRegionIR` built from a benchy_4color paint-segmentation run, **when** `harvest_paint_segmentation_ir` completes, **then** for each `(layer_index, semantic)` key, the number of `SemanticRegion` entries equals the number of *distinct* `(object_id, value)` combinations (not the number of painted facets), and each region's `polygons` Vec contains unioned ExPolygons rather than individual per-facet triangles. | `cargo test -p slicer-host --test paint_segmentation_executor_tdd`

- **AC-2. Given** the annotation loop at `execute_slice_postprocess_paint_annotation`, **when** processing contour points for a `(layer_index, semantic)` pair, **then** `paint_regions.get(layer_index, semantic)` is called exactly once per pair (not once per contour point), and the resulting `&[SemanticRegion]` slice is passed through to `point_in_paint_region` and `is_point_numerically_ambiguous`. | `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`

- **AC-3. Given** a `SemanticRegion` with `aabb: Some(BoundingBox2)` whose bounding box does **not** contain the query point, **when** `semantic_region_contains_point` is called, **then** the function returns `false` without iterating any polygon or calling `ex_polygon_contains_point`. | `cargo test -p slicer-core paint_region`

- **AC-4. Given** a `SemanticRegion` with `aabb: Some(BoundingBox2)` whose bounding box **does** contain the query point, **when** `semantic_region_contains_point` is called, **then** the function proceeds to polygon containment as before (AABB is a pre-filter, not a substitute). | `cargo test -p slicer-core paint_region`

- **AC-5. Given** a `SemanticRegion` constructed via `harvest_paint_segmentation_ir` from deserialized IR (no serialized `aabb` field), **when** `SemanticRegion.aabb` is read, **then** it is `None` and the query path falls through to full polygon containment without panicking — the AABB is optional/reconstruction-only. | `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`

- **AC-6. Given** the contour point loop in `execute_slice_postprocess_paint_annotation`, **when** run on a multi-core machine, **then** `region.polygons` is iterated with `rayon::par_iter()` and wall-clock time scales down with thread count relative to the pre-change serial baseline. | `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`

- **AC-7. Given** `point_in_paint_region` iterating `SemanticRegion` entries sorted by descending `paint_order`, **when** a winner is found at `paint_order = N`, **then** iteration stops immediately — no region with `paint_order <= N` is checked. | `cargo test -p slicer-core paint_region`

- **AC-8. Given** the `harvest_paint_segmentation_ir` change, **when** the resulting `PaintRegionIR` is queried via `point_in_paint_region` for a point that falls inside two regions of the same semantic but different `paint_order`, **then** the region with the higher `paint_order` (lower original insertion index after grouping takes `min`) wins — the precedence contract is preserved. | `cargo test -p slicer-host --test scenario_traces_tdd`

- **AC-9. Given** the union-at-harvest change, **when** a contour point lands exactly on the shared edge between two adjacent same-value painted facets that were merged by union, **then** the point registers as `Inside` (or `Boundary` per `BoundaryInclusion`) — the `code:504` (`numerical-edge-ambiguity`) warning count on a benchy_4color run drops to near-zero compared to the pre-change baseline. | `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.stl --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html 2>&1 | Select-String -Pattern '"code":504'`

- **AC-10. Given** a full pipeline run with `--report`, **when** the slicer-report HTML is inspected, **then** the `com.host.slice-postprocess-paint-annotator` wall-clock row shows time reduced by at least 90% from the ~188 s baseline on a benchy_4color model. | `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.stl --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html`

## Negative Test Cases

- **AC-N1. Given** two `SemanticRegion` entries with the same semantic, same `paint_order`, both `PaintSemantic::Custom` but different values, **when** `point_in_paint_region` is called for a point inside both, **then** the function returns `Err(PaintRegionQueryError::DeterministicConflict)` — grouping only same-value entries cannot synthesise or hide conflicts. | `cargo test -p slicer-host --test scenario_traces_tdd`

- **AC-N2. Given** a `SemanticRegion` with empty `polygons` Vec, **when** `semantic_region_contains_point` is called, **then** the result is `false` (no AABB pre-filter should cause a panic on empty polygon lists). | `cargo test -p slicer-core paint_region`

- **AC-N3. Given** the union-at-harvest change, **when** the same benchy_4color model is sliced twice in succession, **then** the resulting `PaintRegionIR` is byte-deterministic across runs — group sorting by `(paint_order, object_id, value_key)` within each semantic Vec produces identical output. | `cargo test -p slicer-host --test paint_segmentation_executor_tdd`

## Verification

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test paint_segmentation_executor_tdd`
- `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
- `cargo test -p slicer-core paint_region`

## Authoritative Docs

- `docs/02_ir_schemas.md` §"PaintRegionIR" (lines 469-488) — `SemanticRegion`, `LayerPaintMap`, `PaintRegionIR` field definitions. Delegate for exact field names (doc is large, only this section needed).
- `docs/01_system_architecture.md` — dispatch lifecycle. Delegate SUMMARY since > 300 lines; implementer only needs the `harvest_paint_segmentation_ir` call chain.
- `docs/04_host_scheduler.md` §"PrePass Stage Order" (lines 80-160) — PaintSegmentation stage position and PaintRegionIR flow through blackboard. Range-read only.
- `docs/08_coordinate_system.md` — unit system (1 unit = 100 nm). Range-read only; the `BoundingBox2` comparisons use `Point2` in 100 nm units directly.

## Doc Impact Statement

1. `docs/02_ir_schemas.md` §"PaintRegionIR" — add `BoundingBox2` struct definition (2D AABB with `min: Point2, max: Point2`) and `aabb: Option<BoundingBox2>` field to `SemanticRegion` struct, with `#[serde(skip_deserializing, default)]` note. | `rg -q 'BoundingBox2' docs/02_ir_schemas.md`
2. `docs/02_ir_schemas.md` §"PaintRegionIR" — document that `aabb` is reconstruction-only (computed at `harvest_paint_segmentation_ir` time, `None` when deserialized, used as optional pre-filter in `semantic_region_contains_point`). | `rg -q 'reconstruction-only' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — Phase 1 `union_ex` of region slices confirms OrcaSlicer unions paint polygons before querying; parity anchor for union-at-harvest
- `OrcaSlicerDocumented/generated_documentation/03_algorithmic_complexities.md` §"AABB Tree" — AABBTreeIndirect O(log N) queries; parity anchor for AABB pre-filter direction (full tree deferred to packet 63)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
