---
status: implemented
packet: 62_paint-annotator-performance
task_ids:
  - TASK-130c
  - TASK-181
---

# 62_paint-annotator-performance

## Goal

Reduce `com.host.slice-postprocess-paint-annotator` wall-clock from ~188 s to single-digit seconds on benchy-class prints by (1) unioning per-facet paint regions at harvest, (2) caching redundant `paint_regions.get()` lookups, (3) adding `BoundingBox2` AABB pre-filter on `SemanticRegion`, (4) parallelizing contour point annotation with `rayon::par_iter()`, and (5) adding an early-break heuristic in `point_in_paint_region`.

## Problem Statement

`com.host.slice-postprocess-paint-annotator` consumes ~188 s wall-clock (2 255 853 ms summed across 12 threads) on a benchy-class print — roughly 120× the prepass pipeline. The root cause is structural:

1. `harvest_paint_segmentation_ir` at `dispatch.rs:2003-2085` copies guest-emitted `PaintRegionEntry` entries verbatim, producing one `SemanticRegion` per painted facet. For a benchy_4color object, this is hundreds of single-triangle `SemanticRegion` entries per `(layer, semantic)`.

2. `point_in_paint_region` at `paint_region.rs:24-55` iterates every region linearly with no spatial pre-filter. On a miss, `is_point_numerically_ambiguous` at `slice_postprocess.rs:510-540` does a second full O(regions × polygons × edges) scan. The comment at `slice_postprocess.rs:527` explicitly blames "un-unioned per-facet projected triangles."

3. `paint_regions.get(layer_index, semantic)` is called 3+ times per contour point (inside `point_in_paint_region`, inside `is_point_numerically_ambiguous`, and in the fallback branch) — redundant `BTreeMap` lookups for the same key.

4. The contour-point annotation loop at `slice_postprocess.rs:357-411` is fully serial despite `rayon` being an existing dependency and `PaintRegionIR` being `Arc`-wrapped and thread-safe.

5. `point_in_paint_region` iterates all regions even after finding a definitive winner — no early break on descending `paint_order`.

OrcaSlicer's MMU pipeline (`pseudocode_multimaterial_segmentation.md` Phase 1) unions per-facet projections into `ExPolygons` before any query — a documented porting gap (DEV-025, `docs/DEVIATION_LOG.md:33`). This packet closes that gap and adds the caching, spatial pre-filter, and parallelization that OrcaSlicer achieves via `EdgeGrid` and `tbb::parallel_for`.

## Architecture Constraints

- This packet edits `crates/slicer-ir/src/slice_ir.rs` — adding a field to `SemanticRegion`. Existing serialized IR does not contain `aabb`; the `#[serde(skip_deserializing, default)]` annotation ensures backward compatibility. New IR written after this change will skip `aabb` in serialization, so on-disk format is unchanged.
- `BoundingBox2` uses `Point2 { x: i64, y: i64 }` in 100 nm units — same coordinate system as the rest of the pipeline. No scale conversion needed for AABB comparisons.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `slicer_core::union` helper at `polygon_ops.rs:93` uses `flat_map` + `union_64` — no `PolyTree`. Output polygons have zero holes regardless of input hole topology. This is safe for the current guest path (guest emits `ExPolygonView` with `holes: vec![]` — triangles only), but must be documented at the call site in `harvest_paint_segmentation_ir` as a known limitation.

- `paint_order` values change from densely-incrementing per-facet indices to `min(paint_order)` per group. Test assertions on specific `paint_order` values in `paint_segmentation_executor_tdd.rs`, `macro_paint_region_roundtrip_tdd.rs`, `scenario_traces_tdd.rs`, and `paint_region_annotator_tdd.rs` must be updated. The precedence contract (higher `paint_order` wins) is preserved.

- Guest WASM is **not** affected by this packet. The change surface is entirely host-side: IR types, harvest logic, and query helpers. No guest source, WIT, or SDK is edited. No WASM rebuild is required.

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR` output shape unchanged (still `HashMap<u32, LayerPaintMap>` with `HashMap<PaintSemantic, Vec<SemanticRegion>>`). `SemanticRegion` gains one optional field (`aabb`), serde-skipped — no schema version bump.
- WIT boundary: unchanged. Guest still emits `paint-region-entry` records; host still receives them into `paint_region_entries`. Only the conversion in `harvest_paint_segmentation_ir` changes.
- Determinism: group sorting by `(paint_order, object_id, value_key)` within each semantic Vec ensures byte-deterministic output across runs. The per-layer `HashMap` iteration order is already non-deterministic at the `per_layer` level but deterministic within each `LayerPaintMap.semantic_regions` Vec.
- Scheduler: no change to stage order, DAG edges, or claim semantics.

## Locked Assumptions and Invariants

- `slicer_core::union` discards holes — all guest-produced paint region entries carry `holes: vec![]` (triangles only). If a future guest version emits entries with holes, the harvest path must switch to a hole-preserving union variant.
- `paint_order` remains a `u64` running along the WIT entry insertion order. After grouping, `min(paint_order)` per group preserves precedence ordering between groups of different values.
- The AABB pre-filter is an optional optimization — setting `aabb = None` at construction time (or skipping the computation) must not change correctness, only performance.
- `rayon` is already a `slicer-host` dependency — no new Cargo.toml entry.
- Group key `(layer_index, object_id, semantic, value)` is correct because: (a) regions with the same value are query-equivalent and safe to merge; (b) `object_id` preserves per-object boundaries; (c) `paint_order` conflict logic only triggers between regions of different values, never within a same-value group.

## Risks and Tradeoffs

- **Test assertion churn**: 16 `paint_order` and 1 `polygons.len()` assertion across 4 test files must be updated. Each is a behavioral check, not noise — updating them validates the new contract.
- **Union performance at harvest time**: computing `union()` for hundreds of tiny triangles adds harvest-time cost. However, the harvest runs once per pipeline, while the annotation query runs per contour point. The upfront cost is amortized within the first few hundred queries.
- **Hole loss**: `slicer_core::union` flattens holes. Mitigated by the fact that guest output has no holes. If this assumption changes, the downstream effects are: (a) hole-containing paint regions would lose their holes silently; (b) annotation results would treat previously-excluded hole interiors as included. The call-site documentation serves as the canary.
- **par_iter() thread safety**: `PaintRegionIR` is `Arc`-wrapped and read-only. `warnings` and `degraded` require thread-local collection + merge. No shared mutable state beyond these two accumulators.
- **Early-break correctness**: requires regions to be sorted descending by `paint_order`. If the sort is omitted or incorrect, the early-break could skip a region with higher `paint_order`. The sort validation lives in the harvest path and is tested by the same assertions that check `paint_order` values.
