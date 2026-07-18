# Task Map: 95_paint-segmentation-orca-port

This packet spans 17 sub-steps, each tracing back to a row in `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P3 — Paint-segmentation port (Phases 1, 2, 3, 4, 6, 7)" sub-step table. The single docs/07 task ID is `TASK-245`. The table below crosswalks sub-step → implementation-plan step → OrcaSlicer parity reference → expected code surface.

| Sub-step | Implementation step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost |
| --- | --- | --- | --- | --- | --- |
| 0 — Polygon helpers (`union_ex`, `intersection_ex`, `difference_ex`, `opening`, `closing_ex`, `remove_small_and_small_holes`, `expolygons_simplify`, `remove_duplicates`, `clip_line_with_bbox`) | Step 1 | `docs/specs/orca-paint-segmentation-parity.md` §5 constants | `crates/slicer-core/src/polygon_ops.rs`; `crates/slicer-core/tests/polygon_ops_ex_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2252` (`remove_small_and_small_holes`) | M |
| 1 — `triangle_z_intersection(p0,p1,p2,z) -> Option<Line>` | Step 2 | spec §3 Phase 3 | `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` | MultiMaterialSegmentation.cpp Phase 3 pseudocode | M |
| 2 — `EdgeGrid` + `visit_cells_intersecting_line` | Step 2 | spec §3 Phase 2, §4 | `crates/slicer-core/src/algos/paint_segmentation/edge_grid.rs` | MultiMaterialSegmentation.cpp Phase 2 + EdgeGrid utility | M |
| 3 — `PaintedLineVisitor` + `PaintedLine` private type | Step 2 | spec §4 | `crates/slicer-core/src/algos/paint_segmentation/painted_line.rs` | MultiMaterialSegmentation.cpp Phase 3 line emission | M |
| 4 — Phase 1 slice preprocessing | Step 3 | spec §3 Phase 1 | `crates/slicer-core/src/algos/paint_segmentation/preprocess.rs` | MultiMaterialSegmentation.cpp Phase 1 | M |
| 5 — Phase 3 driver (object × extruder × facet → painted_lines) | Step 3 | spec §3 Phase 3 | `crates/slicer-core/src/algos/paint_segmentation/phase3.rs` | MultiMaterialSegmentation.cpp Phase 3 driver | M |
| 6 — `post_process_painted_lines` + `colorize_contours` + `ColoredLine` | Step 4 | spec §3 Phase 4a/4b | `crates/slicer-core/src/algos/paint_segmentation/colorize.rs` | MultiMaterialSegmentation.cpp Phase 4a/4b | M |
| 7 — **RISK GATE** boostvoronoi dep + API spike + `MMU_Graph` | Step 5 | spec §3 Phase 4c, §2 (boostvoronoi assumptions) | `crates/slicer-core/Cargo.toml`; `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` | MultiMaterialSegmentation.cpp Phase 4c | M |
| 8 — `remove_multiple_edges_in_vertices` + `remove_nodes_with_one_arc` | Step 6 | spec §3 Phase 4d/4e | `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` | MultiMaterialSegmentation.cpp Phase 4d/4e | M |
| 9 — `extract_colored_segments` (leftmost-arc walk + `Option<usize>` sentinel per H562) | Step 6 | spec §3 Phase 4f, §8 H562 | `crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs` | MultiMaterialSegmentation.cpp Phase 4f extraction loop | M |
| 10 — `slice_mesh_slabs` helper in slicer-core | Step 7 | spec §3 Phase 6 | `crates/slicer-core/src/triangle_mesh_slicer.rs` (extend) | MultiMaterialSegmentation.cpp Phase 6 slab driver | M |
| 11 — Phase 6 top/bottom propagation | Step 7 | spec §3 Phase 6 | `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs` | MultiMaterialSegmentation.cpp Phase 6 | M |
| 12 — Phase 7 variant-chain composition (per-semantic outputs → variant-chain ExPolygon map via `intersection_ex` / `difference_ex`) | Step 8 | spec §3 Phase 7, D5 | `crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs` | MultiMaterialSegmentation.cpp Phase 7 merge | M |
| 13 — New driver `execute_paint_segmentation_v2(mesh, slice, layer_plan, region_map, config) -> Arc<Vec<SliceIR>>` | Step 9 | spec §7 + roadmap D8 | `crates/slicer-core/src/algos/paint_segmentation/mod.rs` | PrintObjectSlice.cpp:924-1081 `apply_mm_segmentation` | M |
| 14 — Modifier-volume sub-pipeline preserved → `segment_annotations[SupportEnforcer/Blocker]` | Step 10 | roadmap D14, spec §7 modifier-volume | `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs` | OrcaSlicer modifier-volume + LayerRegion pattern (no direct mm-seg ref) | M |
| 15 — Wire into prepass driver: new built-in `host:paint_segmentation` between `host:shell_classification` and `host:support_geometry`; commit via `Blackboard::replace_slice_ir`; old position removed | Step 11 | roadmap D1 + spec §7 driver | `crates/slicer-runtime/src/prepass.rs`; `crates/slicer-runtime/src/builtins/paint_segmentation_producer.rs` | none (host-driver concern) | M |
| 16 — Delete `execute_paint_segmentation` (old) + `PaintRegionIR` + `LayerPaintMap` + `SemanticRegion` + `Blackboard::paint_regions()` + `commit_paint_regions` + `PaintRegionRTreeEntry/Index` + `point_in_paint_region` + `crates/slicer-core/src/paint_region.rs` | Step 12 | roadmap D8 | various (see Step 12 file list) | none (deletion) | M |
| 17 — Replace `run_paint_annotation` at `layer_executor.rs:494-528` (no-op or remove) + `execute_slice_postprocess_paint_annotation` at `slice_postprocess.rs:302` | Step 13 | roadmap D8 + spec §7 | `crates/slicer-runtime/src/layer_executor.rs`; `crates/slicer-runtime/src/slice_postprocess.rs` | none | S |

Aggregate sub-step costs: M (the integrating Step 9 + the RISK GATE at Step 5 dominate; all bounded).

## Why this packet does NOT split

Each sub-step is individually small, but they share an IR contract (D8: inline polygons into SliceIR; delete PaintRegionIR) that cannot be partially landed:

- Sub-steps 0-14 build the new kernel + driver IN ISOLATION (the old kernel still exists; the new driver doesn't yet replace it).
- Sub-step 15 SWITCHES the driver position (new module wires in at new prepass position).
- Sub-step 16 DELETES the old PaintRegionIR + accessors.
- Sub-step 17 stubs the per-layer annotation.

Landing 0-14 + 15 without 16 leaves the workspace with TWO paint pipelines (the new kernel committing to SliceIR + the old PaintRegionIR still sitting on the Blackboard with stale data). Landing 16 without 15 deletes the old surface before the new one is wired — uncompilable. The 17 sub-steps are a coherent slice that must land together. This is the largest packet in the roadmap by design.

## Why no per-sub-step packet split is feasible

The roadmap considered slicing P3 further (e.g., sub-steps 0-9 in one packet, 10-17 in another) but rejected: the cube_4color RED tests would still RED until ALL sub-steps land. There is no useful intermediate "GREEN" state to ship to.
