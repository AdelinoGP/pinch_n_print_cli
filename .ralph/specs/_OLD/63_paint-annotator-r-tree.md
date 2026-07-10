---
status: implemented
packet: 63_paint-annotator-r-tree
task_ids: []
---

# 63_paint-annotator-r-tree

## Goal

Replace the linear O(N) `for region in paint_regions.get(...)` scan in `point_in_paint_region` with an O(log N) `rstar::RTree<BoundingBox2>` spatial index lookup per `(layer_index, semantic)` key, built once at `PaintRegionIR` construction and queried via `locate_in_envelope()`.

## Problem Statement

Packet 62 reduces the multiplicative factor in `point_in_paint_region` by unioning per-facet regions, adding AABB pre-filters, caching lookups, and parallelizing contour iteration. However, the region selection loop within `point_in_paint_region` remains a **linear O(N) scan** over all `SemanticRegion` entries for a given `(layer_index, semantic)` key:

```rust
for region in paint_regions.get(layer_index, semantic) {
    if !semantic_region_contains_point(region, point, boundary_inclusion) { continue; }
    // ... winner logic
}
```

After union-at-harvest, a `(layer, semantic)` bucket might contain 2-20 regions (materials, modifiers, fuzzy skin zones). The AABB pre-filter from packet 62 skips polygon containment for regions whose bounding box misses the point, but the outer loop still iterates every region to check its AABB. This is O(N) in region count, where N is the number of distinct `(object_id, value)` combinations per semantic per layer.

OrcaSlicer's `AABBTreeIndirect` (`03_algorithmic_complexities.md`, line 331) provides O(log N) nearest-neighbor and intersection queries via a static balanced tree. This packet adds equivalent O(log N) region lookup by building an `rstar::RTree<BoundingBox2>` index per `(layer_index, semantic)` key at `PaintRegionIR` construction time, then replacing the linear scan with `rtree.locate_in_envelope(&point_aabb)` to obtain only the candidate regions that could contain the point.

## Architecture Constraints

- The R-tree index is reconstruction-only (like `SemanticRegion.aabb` from packet 62). It is NOT serialized, NOT part of `PartialEq`, and rebuilt from scratch at each `harvest_paint_segmentation_ir` call. Deserialized `PaintRegionIR` has no index; queries fall back to the packet-62 linear-scan-with-AABB-pre-filter path.
- `rstar` must be compatible with WASM (the `slicer-core` crate is used in WASM guests). Default `rstar` features should work; verify no `std`-only features are enabled that break `no_std` builds.
- The R-tree index stores `(BoundingBox2, usize)` pairs where `usize` is the index into `Vec<SemanticRegion>`. This avoids storing `SemanticRegion` references (which would have lifetime issues with `RTree`).

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR` is **unchanged** — no new fields, no schema bump. The `PaintRegionRTreeIndex` is a companion type in `slicer-core`, passed alongside `PaintRegionIR`, not stored on it.
- WIT boundary: unchanged.
- Function signature change: `point_in_paint_region` gains `rtree_index: Option<&PaintRegionRTreeIndex>` parameter. This is a breaking change for any external callers of this public function. Update all call sites: `slice_postprocess.rs` (annotation loop), `is_point_numerically_ambiguous`, and all test fixtures that construct call arguments.
- Determinism: `rstar::RTree` bulk-load is deterministic for the same input order. Input order is deterministic (same group sort from packet 62).
- Memory: One `RTree<(BoundingBox2, usize)>` per `(layer_index, semantic)` key in the `PaintRegionRTreeIndex`. For benchy_4color with ~200 layers and ~4 semantics, ~800 small trees. Each tree contains ~2-20 entries after union-at-harvest. Memory overhead is negligible (~KB).
- Blackboard: `harvest_paint_segmentation_ir` currently returns only `PaintRegionIR`. After this change, it returns a tuple `(PaintRegionIR, PaintRegionRTreeIndex)` or the dispatcher wraps the index alongside the IR. The blackboard `commit_paint_regions` may need to store both, or the index can be threaded directly into the annotation request.

## Locked Assumptions and Invariants

- The query point AABB for `locate_in_envelope` is a zero-area envelope: `AABB::from_corners(point, point)`. This is the standard rstar pattern for point-in-envelope queries.
- `rstar::RTree` uses `RTree::locate_in_envelope` which returns an iterator of `&T`. We store `(BoundingBox2, usize)` and use the `usize` to index into `Vec<SemanticRegion>`.
- The R-tree is rebuilt from scratch at each `harvest_paint_segmentation_ir` call — there is no incremental update. This matches the pipeline model where `PaintRegionIR` is constructed once per run.
- The linear-scan fallback path (when `rtree_index` is `None`) is identical to the packet-62 postcondition. This ensures correctness when running against deserialized IR or when the index is not yet built.
- `rstar` version 0.12 is expected to be WASM-compatible. If it pulls in std-only dependencies, the implementer should try `default-features = false` or pin an earlier compatible version.
- The `PaintRegionRTreeIndex` is `Arc`-wrapped (matching the `Arc<PaintRegionIR>` pattern) for sharing across threads in the parallel query path from packet 62.

## Risks and Tradeoffs

- **New dependency risk**: `rstar` is a new crate dependency. It has a moderate compile time cost. It is widely used and maintained, but any future breaking changes require a coordinated update.
- **R-tree build time at harvest**: Building an R-tree per `(layer, semantic)` key adds O(N log N) construction time at harvest. For ~200 layers × ~4 semantics × ~10 regions = ~8000 entries total, the build time is negligible (< 1 ms). The query-time savings far outweigh this.
- **WASM compatibility**: `slicer-core` is used by WASM guests. If `rstar` pulls in `std::collections` or `alloc` features incompatible with WASM, the implementer must use `default-features = false` or add a `wasm` feature flag that conditionally compiles the R-tree only for host targets.
- **Memory overhead**: One `RTree` per `(layer, semantic)` key. For benchy_4color (~200 layers, ~4 semantics), this is ~800 trees. Each tree is small (~10-20 entries). Total memory is ~few KB. For large models with many layers and many materials, the tree count scales linearly but each tree remains small after union-at-harvest.
