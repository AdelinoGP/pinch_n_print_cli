---
status: implemented
packet: 95
task_ids: [TASK-245]
---

# 95_paint-segmentation-orca-port

## Goal

Replace the broken paint-segmentation kernel with the OrcaSlicer-parity Phases 1, 2, 3, 4, 6, 7 pipeline — repositioned to run between `host:shell_classification` and `host:support_geometry`, committed into `SliceIR.regions[*]` via `Blackboard::replace_slice_ir`, with `PaintRegionIR` / `LayerPaintMap` / `SemanticRegion` and their host-side rtree deleted and the modifier-volume sub-pipeline preserved — turning all 12 RED `cube_4color_paint_tdd` + 12 RED `cube_fuzzy_painted_tdd` tests GREEN.

### Solution Shape

- **Broken kernel removed**: `crates/slicer-core/src/algos/paint_segmentation.rs:298-362` (projects facets' XY shadows onto every layer, drops Z, no slice-plane intersection, no Voronoi, no top/bottom propagation) and `execute_slice_postprocess_paint_annotation` at `crates/slicer-runtime/src/slice_postprocess.rs:302` are deleted.
- **New module structure**: `crates/slicer-core/src/algos/paint_segmentation/` with one file per phase + helpers (see `design.md` §Code Change Surface).
- **Driver position D1**: runs POST-`host:slice`, between `host:shell_classification` and `host:support_geometry`; reads `SliceIR` directly (not `MeshIR`); writes via `Blackboard::replace_slice_ir` (D8).
- **Phase 2-3**: EdgeGrid spatial cell indexing + `triangle_z_intersection` slice-plane math.
- **Phase 4**: `boostvoronoi`-backed `MMU_Graph` construction + `remove_multiple_edges_in_vertices` / `remove_nodes_with_one_arc` pruning + `extract_colored_segments` leftmost-arc walk with `Option<usize>` repair sentinel (H561-H567 hazards encoded per spec §8).
- **Phase 6**: `slice_mesh_slabs` top/bottom propagation.
- **Phase 7**: per-semantic outputs composed into variant-chain ExPolygon maps via `intersection_ex` / `difference_ex` (D5 geometric composition).
- **Modifier-volume preserved** (D14): `SupportEnforcer` / `SupportBlocker` route to `SlicedRegion.segment_annotations` on the BASE variant — NOT region-split.
- **Deleted surface**: `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `crates/slicer-core/src/paint_region.rs` (rtree + `point_in_paint_region`), `Blackboard::paint_regions()` + `commit_paint_regions`, `PaintRegionRTreeEntry`/`Index`; `run_paint_annotation` at `layer_executor.rs:494-528` is no-op or removed.
- **17 sub-steps land as one packet**: the IR-contract switch (delete `PaintRegionIR` + inline into `SliceIR` + change driver position) cannot be partially landed — any intermediate state leaves the workspace uncompilable or with two parallel paint pipelines.

## Problem Statement

The current `crates/slicer-core/src/algos/paint_segmentation.rs:298-362` is fundamentally broken: it projects each painted facet's XY shadow onto every layer the object participates in. There is no slice-plane intersection (`triangle_z_intersection`), no EdgeGrid spatial cell indexing, no Voronoi-based contour colorization, no top/bottom propagation, no width limiting. Any non-vertical painted facet produces wrong tool/material assignments — wrong both in *which* layers see the paint and in *which* contour segments carry the assignment.

The cherry-picked test suites at `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` (12 RED tests) and `cube_fuzzy_painted_tdd.rs` (12 RED tests) expose this: each test asserts per-face, per-layer, per-variant geometry the current kernel cannot produce.

The authoritative algorithm spec is `docs/specs/orca-paint-segmentation-parity.md` (1021 lines), which describes a 7-phase OrcaSlicer pipeline:

- **Phase 1** — slice preprocessing (already happens in `host:slice` after the P1 port; this packet just reads `SliceIR`).
- **Phase 2** — EdgeGrid construction per layer + line visitor.
- **Phase 3** — for each (object × extruder × painted facet) triple, intersect the facet's triangles with the layer's Z plane via `triangle_z_intersection`, contribute the resulting lines to the EdgeGrid + `PaintedLine` records.
- **Phase 4** — colorize contours: post-process `PaintedLine`s (Phase 4a), construct `MMU_Graph` from `boostvoronoi` line-segment sites (Phase 4c), prune via `remove_multiple_edges_in_vertices` / `remove_nodes_with_one_arc` (Phase 4d/4e), extract colored segments with leftmost-arc walk and `Option<usize>` repair sentinel (Phase 4f).
- **Phase 5** — width limiting + interlocking (out of scope for THIS packet; packet 96 does it).
- **Phase 6** — top/bottom propagation: for each painted facet, identify the slab of layers it covers, project the painted area down through the slab via `slice_mesh_slabs`.
- **Phase 7** — merge per-semantic outputs into variant-chain ExPolygon maps via `intersection_ex` / `difference_ex` (D5 geometric composition).

This packet ports Phases 1, 2, 3, 4, 6, 7 — Phase 5 deferred to packet 96. The kernel's host-side position changes: it now runs POST-`host:slice` (D1), reading `SliceIR` directly (not the pre-slice mesh), and writes per-variant `SlicedRegion`s into the existing `SliceIR.regions[*]` Vec via `Blackboard::replace_slice_ir` (D8). `PaintRegionIR` and its tooling are deleted because the per-variant polygons are inlined into the SliceIR.

The OrcaSlicer parity hazards H561–H567 (typed-state Voronoi vertex wrappers, `Option<usize>` repair sentinel, conservative AND-logic prefilter, Rayon banding, per-extruder nozzle lookup fix, HashSet dedup, explicit index tracking for force-edge pointer arithmetic) are all encoded into the sub-step design per `docs/specs/orca-paint-segmentation-parity.md` §8.

Threading: Rayon `par_iter`/`par_iter_mut` everywhere TBB is used in OrcaSlicer (spec §6). The 64-mutex bucket pattern (`Vec<Mutex<()>>` of length 64 indexed by `layer_idx & 63`) is the agreed parallelism shape.

Behavior preservation guarantee: unpainted meshes pass byte-identical through this packet because the driver short-circuits on `aggregated_region_split.is_empty() OR no_object_has_paint_data`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Phase-isolation invariant: each phase's output type is documented and unit-tested in isolation. Phase 3 outputs `Vec<PaintedLine>`; Phase 4 outputs `Vec<ColoredSegment>`; Phase 6 outputs per-semantic ExPolygon maps; Phase 7 outputs the final variant-chain map. Mixing phase outputs across kernel functions is forbidden.
- H561 invariant: the boostvoronoi color slot is dual-use (winding-tag AND graph metadata). The Rust crate exposes it as `Vertex::get_color() -> ColorType` / `Vertex::set_color(ColorType) -> ColorType` / `Vertex::or_color(ColorType) -> ColorType` (matching `Edge::get_color`/`set_color`/`or_color`) — note `get_color`, NOT `color()` as in the OrcaSlicer C++ source. Use typed-state wrappers `VoronoiVertex` (boost-emitted) and `GraphVertex` (post-pruning) to prevent silent mix; never read `get_color()` on a `GraphVertex`-tagged value.
- Boostvoronoi-`twin` invariant: `Edge::twin()` returns `Result<EdgeIndex, BvError>` (not a bare pointer like the C++ source). Every twin-walk in the kernel MUST `?`-propagate the `BvError` — never `.unwrap()` or `.expect()` on `twin()` in non-test code, since a graph-construction error here corrupts every downstream Phase 4d/4e/4f walk and the H567 explicit-index-tracking invariant assumes the walk completed without error.
- H562 invariant: repair sentinel for `extract_colored_segments` is `Option<usize>::None`, never `usize::MAX`. `usize::MAX` is a valid graph node index in extreme cases; using it as a sentinel is the OrcaSlicer-port bug we explicitly avoid.
- H565 invariant: do NOT replicate the OrcaSlicer bug of hardcoding extruder 0's nozzle. Read each extruder's own nozzle from `config-view`.
- H566 invariant: degree-bounded dedup uses `HashSet<EdgeKey>` with `debug_assert!(degree <= 20)` (graph degrees are bounded by triangle adjacency in practice).
- H567 invariant: explicit index tracking in `extract_colored_segments`, NOT pointer arithmetic.
- Driver-position invariant: paint-segmentation runs AFTER `host:slice` (consumes `SliceIR`) and AFTER `host:shell_classification` (consumes surface classification). Writes via `replace_slice_ir` — the existing blackboard contract.
- Input contract: paint-segmentation consumes `PaintLayer.facet_values` AND `PaintLayer.strokes` directly. The loader's `split_triangle_strokes` (`crates/slicer-model-io/src/loader.rs:1900-1961`) is the canonical TriangleSelector normalization site; `PaintLayer.strokes` arrives in OrcaSlicer's flat-leaf form (per TASK-250 architectural verdict that retired the `PrePass::MeshSegmentation` host stage in P94r). No host-side intermediate normalization stage exists. Phase 3's `collect_facets()` per parity-doc lines 140-141 unifies both representations.
- D14 invariant: modifier-volume support (`SupportEnforcer` / `SupportBlocker`) routes to `segment_annotations`, NOT region-split. This is critical — modifier-volume support is a per-contour-point property, not a variant axis.
- D15 invariant: per-variant polygons may be empty (no geometric coverage) but the entry still exists in `RegionMapIR` (placed by P1c). This packet just populates the polygons.
- **Empty-polygon ownership (handed off from P93 refinement)**: `RegionPlan` entries arriving from P93's `RegionMapIR` may carry empty per-variant polygons unconditionally (P93 follows D15 by emit-without-gating). P95 has the polygons in hand via `replace_slice_ir` and owns the empty-polygon gate. **Decision deferred to sub-step 13 (the integrating driver)**: the integrator chooses whether to filter empty-polygon entries when emitting `SlicedRegion`s, OR to leave them as no-ops downstream. The packet-level acceptance does not pre-bind that choice.

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion` are DELETED (D8). `SlicedRegion.variant_chain` (added in P1a) now becomes meaningfully populated. `SlicedRegion.segment_annotations` (renamed in P1a) becomes the routing destination for non-region-split semantics.
- WIT boundary considerations: deleting `PaintRegionIR` is breaking for any WIT consumer that referenced it. The `wit/` files likely never referenced PaintRegionIR (the rtree was host-only), but Step 1 dispatch confirms.
- Determinism or scheduler constraints: `boostvoronoi` vertex output ordering — confirm in sub-step 7 that the crate provides deterministic ordering OR add a sort pass.

## Locked Assumptions and Invariants

- **Phases 1-4-6-7 (not 5) shipped this packet**: Phase 5 in packet 96. Documented in the closure log.
- **`Option<usize>` repair sentinel (H562)**: forbids `usize::MAX` as a sentinel anywhere in the new module.
- **Driver position D1**: paint-segmentation runs POST-`host:slice`, between `host:shell_classification` and `host:support_geometry`. The DAG validator (via `required_slots` table) enforces.
- **Modifier-volume routing D14**: SupportEnforcer / SupportBlocker volumes route to `segment_annotations`, NOT to a region-split. Spec §7 + roadmap D14.
- **`PaintRegionIR` and related types deleted**: no transitional shim remains.
- **Short-circuit telemetry pattern**: when the driver short-circuits on no-paint-data input, it emits `ProgressEventType::StageStart` then immediately `ProgressEventType::StageComplete` with `elapsed_ms == 0` and zero intervening `ProgressEventType::ModuleStart` events. No new `ProgressEventType::StageSkipped` variant is added — that schema change is deferred. Workspace today has no `StageSkipped` (per `docs/09_progress_events.md` + `crates/slicer-runtime/src/progress_events.rs::ProgressEventType`); reuse the existing channel.

## Risks and Tradeoffs

- **Risk: `boostvoronoi` API doesn't match spec assumptions** (sub-step 7). Mitigation: API spike at the START of Phase 4 work; fall back to `spade` + custom Voronoi wrapper or cxx-bridge to OrcaSlicer's boost::polygon::voronoi. Document in `docs/specs/orca-paint-segmentation-parity.md` open Q5.
- **Risk: Phase 6 `slice_mesh_slabs` more involved than expected** (sub-step 10). Mitigation: separately landable; verifiable with a single cube_4color RED test that targets the top face's two tool indices (the "projection coverage" test).
- **Risk: per-semantic Voronoi pass count balloons on contrived inputs**. Mitigation: spec §6 threading model + Rayon par_iter; document scaling.
- **Risk: modifier-volume sub-pipeline diverges from main paint pipeline**. Mitigation: unit-test the mix (modifier-volume SupportEnforcer + facet Material on same layer).
- **Risk: removing PaintRegionIR breaks a consumer not in the audit list**. Mitigation: `rg -nl 'PaintRegionIR|PaintRegionRTreeIndex|point_in_paint_region' crates/` post-delete; expect 0.
- **Tradeoff: large packet vs. fine-grained packets.** The IR-contract switch (delete PaintRegionIR + inline + driver position change) cannot be partially landed. Smaller packets would leave the workspace uncompilable mid-port.
