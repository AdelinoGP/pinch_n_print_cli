---
status: superseded
packet: 208-wall-flag-path-clip
task_ids:
  - TASK-324
---

# 208-wall-flag-path-clip

## Goal

Replace `build_wall_flags`' nearest-original-vertex reprojection with canonical path-geometry clipping: deliver paint areas as `ExPolygon` sets on `SlicedRegion`, port `Algorithm::split_line` into `slicer-core`, and assign per-vertex `WallFeatureFlags` from clipped-run membership in both perimeter modules.

## Problem Statement

`build_wall_flags` and its helper `nearest_original_vertex` (`crates/slicer-core/src/perimeter_utils.rs`) attribute paint to inner-wall vertices by mapping each inset-ring vertex to the nearest *original* contour vertex and inheriting that vertex's `PaintSemantic::Material` / `PaintSemantic::FuzzySkin` annotation. This is a Pinch 'n Print invention with no canonical precedent, recorded as DEV-126. Canonical never attributes paint per vertex at all: `group_region_by_fuzzify` (`Feature/FuzzySkin/FuzzySkin.cpp`) groups `LayerRegion` surfaces into `ExPolygons` per config, and both `apply_fuzzy_skin` overloads route the finished wall path through `Algorithm::split_line(path, r.expolygons, closed)` and act only on the runs the clip reports as `clipped`. Attribution accuracy in PnP is therefore bounded by vertex spacing rather than by the paint-region boundary — a vertex near a semantic boundary can inherit the wrong side.

Grounding against the tree changed the shape of the fix twice relative to the plan row:

1. **The plan's "needs Clipper-based line-split infrastructure that does not exist in-tree" is half right.** `split_line` genuinely has zero occurrences under `crates/` and `modules/`, but `clip_polylines` (`crates/slicer-core/src/polygon_ops.rs`) already performs open-path clipping against an `ExPolygon` set through `Clipper64` with `add_open_subject`. What it cannot do is what `split_line` exists to do: it returns only the inside runs, drops the outside runs, leaves output ordering unspecified, and carries no provenance back to the source vertex. Canonical recovers provenance by stashing the source index in the Clipper `Z` coordinate (`ClipperZUtils::ZPath`); the workspace binding `clipper2-rust` exposes `Point64 { x, y }` with **no** `Z` channel, so that carrier cannot be ported. The port is therefore a native segment/edge intersection walk, not a new Clipper call.
2. **PnP delivers no paint areas to any module in production.** The WIT surface for it already exists and is dead: `crates/slicer-schema/wit/deps/ir-types.wit` declares `record semantic-region { object-id, polygons: list<ex-polygon>, value: paint-value }` and `paint-region-layer-view.get-regions`, and `HostPaintRegionLayerView::get_regions` (`crates/slicer-wasm-host/src/host.rs`) reads `PaintRegionLayerData.regions_by_semantic` — which both production construction sites (`crates/slicer-wasm-host/src/host.rs` and `crates/slicer-wasm-host/src/dispatch.rs`) initialise to `HashMap::new()` and never write. The Rust-side `PaintRegionLayerView` (`crates/slicer-sdk/src/traits.rs`) has no `get_regions` at all; its doc comment records that the v1 `PaintRegionIR`/`SemanticRegion` types were deleted in packet 95 (D8) and that per-layer paint now travels on `SliceIR.regions[*].segment_annotations` (D14). So the canonical clip has no input today, and supplying one is part of this packet rather than a prerequisite it can assume.

The area data itself is not missing — it is discarded. `build_modifier_segment_annotations` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) derives the per-vertex annotations by testing each contour-edge midpoint against `modifier_volumes::ModifierVolumeLayer::polygons` with `any_expolygon_contains_point`; and `segments_to_expolygons_by_color` in the same file already returns `BTreeMap<Option<PaintValue>, Vec<ExPolygon>>`. The per-vertex map is a lossy projection of `ExPolygon` data the host already holds.
