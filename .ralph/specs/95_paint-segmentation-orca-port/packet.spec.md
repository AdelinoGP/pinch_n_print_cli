---
status: draft
packet: 95
task_ids: [TASK-245]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 95 — Paint-Segmentation Port (OrcaSlicer Phases 1, 2, 3, 4, 6, 7)

## Goal

Replace the broken paint-segmentation kernel (`crates/slicer-core/src/algos/paint_segmentation.rs:298-362` — projects facets' XY shadows onto every layer, drops Z, no slice-plane intersection, no Voronoi, no top/bottom propagation, no width limiting) and the obsolete `execute_slice_postprocess_paint_annotation` driver at `crates/slicer-runtime/src/slice_postprocess.rs:302` with the OrcaSlicer-parity Phases 1, 2, 3, 4, 6, 7 from `docs/specs/orca-paint-segmentation-parity.md` — running POST-`host:slice` (D1; between `host:shell_classification` and `host:support_geometry`), reading `SliceIR` directly (not `MeshIR`'s top-of-prepass slot), computing per-semantic Voronoi-based contour colorization with EdgeGrid spatial cell indexing and `triangle_z_intersection` slice-plane math (Phases 2-3), `boostvoronoi`-backed `MMU_Graph` construction with `remove_multiple_edges_in_vertices` / `remove_nodes_with_one_arc` pruning + `extract_colored_segments` leftmost-arc walk with `Option<usize>` repair sentinel (Phase 4; H561-H567 hazards), `slice_mesh_slabs` top/bottom propagation (Phase 6), per-semantic outputs composed into variant-chain ExPolygon maps via `intersection_ex` / `difference_ex` (Phase 7; D5 geometric composition) — and inlining the resulting per-variant polygons into the existing `SliceIR.regions[*]` via `Blackboard::replace_slice_ir` (D8); declaring `[[region_split]]` on the `material` and `fuzzy_skin` core paint semantics in the manifest of a NEW core module `paint-segmentation-default` (or in the host's effective manifest if the kernel stays a host built-in — to be decided in design); deleting `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, the `Blackboard::paint_regions()` + `commit_paint_regions` + `PaintRegionRTreeEntry/Index` + `point_in_paint_region` (`crates/slicer-core/src/paint_region.rs:22-93`); turning the per-layer `run_paint_annotation` at `crates/slicer-runtime/src/layer_executor.rs:494-528` into a no-op or deleting it; preserving the modifier-volume sub-pipeline routing to `segment_annotations[SupportEnforcer/Blocker]` per D14; making all 12 RED cube_4color tests + all 12 RED cube_fuzzy_painted tests GREEN. This is the largest, riskiest packet in the roadmap; it ships 17 sub-steps as a single coherent slice because the IR contract switch (delete PaintRegionIR + inline into SliceIR + change driver position) cannot be partially landed.

## Scope Boundaries

This packet replaces the entire paint-segmentation kernel with the OrcaSlicer-parity pipeline per the authoritative handoff spec at `docs/specs/orca-paint-segmentation-parity.md` (1021 lines, normative for algorithm details). It does NOT implement Phase 5 (width limiting + interlocking) — that is packet P4 (96). It does NOT delete the WASM mesh-segmentation surface (97 files) — that is packet P5a (97). It does NOT change the loader's per-channel decoder — that is packet P5b (98). Doc updates are P5c (99). Full in/out-of-scope lists in `requirements.md`. The 17 sub-steps trace to the roadmap's P3 sub-step table; `task-map.md` provides the explicit crosswalk.

## Prerequisites and Blockers

- Depends on: P91 (IR scaffolding), P92 (manifest + dispatch), P93 (RegionMapping cross-product), P94 (mesh-segmentation wiring) all `implemented`. Without P94, sub-facet strokes leak into the paint-segmentation input and the kernel can't assume facet_values is authoritative.
- Unblocks: P4 (96, Phase 5 width-limiting) reads the kernel's per-variant polygons. P5a (97, WASM mesh-segmentation deletion) can land in parallel with this packet but typically after.
- Activation blockers: P91, P92, P93, P94 all `implemented`. Confirmation that `boostvoronoi` crate API supports line-segment sites, `vertex.color()` metadata, and infinite-edge clipping via `is_primary()` / `twin()` — verified in sub-step 7 (P3-S7) as the first non-helper work.

## Acceptance Criteria

### AC-1 — Sub-step 0: Polygon helpers `union_ex`, `intersection_ex`, `difference_ex`, `opening`, `closing_ex`, `remove_small_and_small_holes`, `expolygons_simplify`, `remove_duplicates`, `clip_line_with_bbox` exist in `slicer-core/src/polygon_ops.rs`

**Given** the kernel's geometry needs,
**When** `crates/slicer-core/src/polygon_ops.rs` is grepped,
**Then** all nine helper functions are public, documented, and have at least one unit test each.

| `for f in union_ex intersection_ex difference_ex opening closing_ex remove_small_and_small_holes expolygons_simplify remove_duplicates clip_line_with_bbox; do rg -q "pub fn $f" crates/slicer-core/src/polygon_ops.rs || { echo "MISSING: $f"; exit 1; }; done && cargo test -p slicer-core polygon_ops 2>&1 | tee target/test-output.log`

### AC-2 — Sub-step 1: `triangle_z_intersection(p0,p1,p2,z) -> Option<Line>` is implemented and unit-tested

**Given** Phase 3's slice-plane math need,
**When** `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` is inspected,
**Then** the function exists; returns `Some(Line)` for triangles crossing Z, `None` for triangles fully above/below or coplanar; unit tests cover the four canonical cases (above, below, crossing-1-edge, crossing-2-edges).

| `cargo test -p slicer-core paint_segmentation::triangle_intersect 2>&1 | tee target/test-output.log`

### AC-3 — Sub-step 2: `EdgeGrid` data structure exists with `visit_cells_intersecting_line`

**Given** Phase 2's spatial-indexing need,
**When** `crates/slicer-core/src/algos/paint_segmentation/edge_grid.rs` is inspected,
**Then** the `EdgeGrid` type exists, supports cell construction from a 2D bounding box + cell size, and provides `visit_cells_intersecting_line` that calls a visitor closure for each cell intersected by a 2D line segment.

| `cargo test -p slicer-core paint_segmentation::edge_grid 2>&1 | tee target/test-output.log`

### AC-4 — Sub-step 3+4+5: Phase 1 preprocess + `PaintedLineVisitor` + Phase 3 driver produce per-painted-line records

**Given** Phases 1, 2, 3 wired together,
**When** the pipeline runs on `cube_4color.3mf`'s painted faces,
**Then** the intermediate `PaintedLine` records for one layer contain (a) the line endpoints (post-`triangle_z_intersection`), (b) the painted facet's `PaintSemantic` + `PaintValue`, (c) the line's spatial-cell membership index in the EdgeGrid; the count matches the expected per-face contribution derived from the cube's face triangulation.

| `cargo test -p slicer-core paint_segmentation::phase3_painted_lines 2>&1 | tee target/test-output.log`

### AC-5 — Sub-step 6: `post_process_painted_lines` + `colorize_contours` produce `ColoredLine` records

**Given** Phase 4a/4b colorization,
**When** the per-layer `Vec<PaintedLine>` is processed,
**Then** the output `Vec<ColoredLine>` carries (a) the contour segment's PaintValue assignment, (b) the leftmost-arc winding direction tag.

| `cargo test -p slicer-core paint_segmentation::colorize 2>&1 | tee target/test-output.log`

### AC-6 — Sub-step 7: `boostvoronoi` dep added + `MMU_Graph` constructs from `Vec<ColoredLine>`

**Given** the Voronoi-graph construction,
**When** `crates/slicer-core/Cargo.toml` is inspected and the API spike runs,
**Then** `boostvoronoi = "<version>"` appears in the dependency list; an integration test confirms the crate supports line-segment site input + `vertex.color()` reads + infinite-edge clipping via `is_primary()` / `twin()` (the four API features the kernel relies on per spec §2); `MMU_Graph` builds from a tiny synthetic `ColoredLine` set without panicking.

| `rg -q '^boostvoronoi' crates/slicer-core/Cargo.toml && cargo test -p slicer-core paint_segmentation::voronoi_graph 2>&1 | tee target/test-output.log`

### AC-7 — Sub-step 8: `remove_multiple_edges_in_vertices` + `remove_nodes_with_one_arc` Phase 4d/4e pruning

**Given** the MMU_Graph pruning steps from the handoff spec,
**When** they run on a synthetic graph with known multi-edges and one-arc dead-ends,
**Then** the pruner produces a structurally clean graph (no parallel edges between same vertex pair; no node with degree 1).

| `cargo test -p slicer-core paint_segmentation::voronoi_prune 2>&1 | tee target/test-output.log`

### AC-8 — Sub-step 9: `extract_colored_segments` walks leftmost-arcs with `Option<usize>` repair sentinel (H562)

**Given** Phase 4f extraction,
**When** the extractor runs against the pruned graph,
**Then** the output `Vec<ColoredSegment>` correctly tracks coloration boundaries via the leftmost-arc winding; the repair sentinel uses `Option<usize>::None` (NOT `usize::MAX`) per H562; an `Option::None` outside the documented repair entry-points triggers a `debug_assert` for early detection.

| `cargo test -p slicer-core paint_segmentation::extract_segments 2>&1 | tee target/test-output.log`

### AC-9 — Sub-step 10: `slice_mesh_slabs(mesh, z_bottom, z_top, slab_count)` exists in `slicer-core/src/triangle_mesh_slicer.rs`

**Given** Phase 6's top/bottom propagation need,
**When** the function is inspected,
**Then** it produces per-slab footprint polygons (the projection of every triangle in the slab into the XY plane), supports a configurable slab count, and has a unit test asserting a known geometry's slab footprints.

| `cargo test -p slicer-core triangle_mesh_slicer::slice_mesh_slabs 2>&1 | tee target/test-output.log`

### AC-10 — Sub-step 11: Phase 6 top/bottom propagation produces per-layer per-semantic polygons

**Given** Phase 6 wired to `slice_mesh_slabs`,
**When** it runs on the cube fixtures,
**Then** the propagated per-layer per-semantic polygons match the expected face projection: cube_4color's +X face contributes material=ToolIndex(1) on the full vertical extent; cube_fuzzyPainted's painted faces contribute fuzzy_skin=Flag(true) similarly.

| `cargo test -p slicer-core paint_segmentation::top_bottom 2>&1 | tee target/test-output.log`

### AC-11 — Sub-step 12: Phase 7 variant-chain composition produces ExPolygon-per-variant-chain map via geometric composition (D5)

**Given** per-semantic outputs from Phase 4 + Phase 6,
**When** Phase 7 composes,
**Then** the resulting map keyed by `Vec<(String, PaintValue)>` carries the disjoint ExPolygon set for each variant chain — base chain (unpainted area) = total contour minus union of all painted variants; each painted chain = intersection_ex of its constituent semantic outputs minus union_ex of overlapping higher-priority chains.

| `cargo test -p slicer-core paint_segmentation::compose_variants 2>&1 | tee target/test-output.log`

### AC-12 — Sub-step 13: `execute_paint_segmentation_v2` driver produces `Arc<Vec<SliceIR>>` with per-variant SlicedRegion entries

**Given** the new driver,
**When** it runs against the post-slice Blackboard for `cube_4color.3mf`,
**Then** the output `SliceIR` carries one `SlicedRegion` per (base region, variant chain) cross-product element, with `polygons` populated per Phase 7's composition.

| `cargo test -p slicer-core paint_segmentation::driver_v2 2>&1 | tee target/test-output.log`

### AC-13 — Sub-step 14: Modifier-volume sub-pipeline preserved; routes to `segment_annotations[SupportEnforcer/Blocker]` (D14)

**Given** the modifier-volume sub-pipeline from the OLD `paint_segmentation.rs:374-417`,
**When** a mesh with modifier-volume SupportEnforcer / SupportBlocker is processed,
**Then** the modifier-volume polygons are sliced per-layer (preserving the old approach), routed to `SlicedRegion.segment_annotations[PaintSemantic::SupportEnforcer]` (or `SupportBlocker`) on the BASE variant region, NOT region-split. (D14 explicitly routes modifier-volume support to segment_annotations.)

| `cargo test -p slicer-core paint_segmentation::modifier_volumes 2>&1 | tee target/test-output.log`

### AC-14 — Sub-step 15: `host:paint_segmentation` runs between `host:shell_classification` and `host:support_geometry` (D1)

**Given** the new driver position,
**When** the prepass driver is inspected,
**Then** the order is `host:mesh_segmentation` → `host:mesh_analysis` → user-early → `host:region_mapping` → `host:slice` → `host:shell_classification` → **`host:paint_segmentation`** → `host:support_geometry` → user-late; the `PaintSegmentation` stage commits via `Blackboard::replace_slice_ir`.

| `rg -B1 -A15 'PrePass::PaintSegmentation' crates/slicer-runtime/src/prepass.rs | rg -q 'replace_slice_ir|shell_classification|support_geometry'`

### AC-15 — Sub-step 16: `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `paint_region.rs` (rtree + `point_in_paint_region`) all DELETED; `Blackboard::paint_regions()` + `commit_paint_regions` GONE

**Given** D8 (inline polygons into SliceIR; delete PaintRegionIR),
**When** the workspace is grepped,
**Then** no type definitions for `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion` survive under `crates/`; `crates/slicer-core/src/paint_region.rs` does not exist; `Blackboard::paint_regions` and `commit_paint_regions` are gone.

| `! rg -q 'pub struct PaintRegionIR|pub struct LayerPaintMap|pub struct SemanticRegion' crates/ && test ! -f crates/slicer-core/src/paint_region.rs && ! rg -q 'fn commit_paint_regions|fn paint_regions' crates/slicer-runtime/src/blackboard.rs`

### AC-16 — Sub-step 17: `run_paint_annotation` body at `layer_executor.rs:494-528` is removed or stubbed (no-op)

**Given** that paint annotation is now intrinsic to per-variant SlicedRegions,
**When** `crates/slicer-runtime/src/layer_executor.rs` is inspected,
**Then** the original `run_paint_annotation` function body either (a) does nothing and returns Ok (transitional no-op, with TODO to delete in a follow-up packet) OR (b) is fully removed; the `execute_slice_postprocess_paint_annotation` shim at `slice_postprocess.rs:302` is either no-op or removed.

| `rg -B1 -A30 'fn run_paint_annotation' crates/slicer-runtime/src/layer_executor.rs | rg -qE 'Ok\(\(\)\)|^\s*\}|deleted'`

### AC-17 — All 12 cube_4color RED tests turn GREEN

**Given** the cherry-pick `5c272ef`'s 12-test RED suite at `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs`,
**When** the test bucket runs,
**Then** all 12 tests pass.

| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 12 passed; 0 failed'`

### AC-18 — All 12 cube_fuzzy_painted RED tests turn GREEN

**Given** the cherry-pick's 12-test RED suite at `crates/slicer-runtime/tests/executor/cube_fuzzy_painted_tdd.rs`,
**When** the test bucket runs,
**Then** all 12 tests pass.

| `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 12 passed; 0 failed'`

### AC-19 — Behavior preservation on unpainted regression_wedge.stl

**Given** unpainted geometry,
**When** `pnp_cli slice` runs,
**Then** g-code is byte-identical to the post-P94 baseline (paint-segmentation short-circuits on no-paint-data input).

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p95-wedge.gcode && sha256sum /tmp/p95-wedge.gcode`

### AC-20 — `cargo test --workspace` passes (final gate; PRTK packet)

**Given** the IR + driver + kernel reshape,
**When** the workspace suite runs (delegated per `CLAUDE.md` §Test Discipline),
**Then** every bucket reports `test result: ok`.

| `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`

### AC-21 — Guest WASM `--check` clean

| `cargo xtask build-guests && cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — No code path under `crates/` mentions `PaintRegionIR`, `point_in_paint_region`, or `commit_paint_regions`

| `! rg -q 'PaintRegionIR|point_in_paint_region|commit_paint_regions' crates/`

### AC-N2 — Paint-segmentation short-circuits on no-paint-data input (no kernel work performed)

**Given** an unpainted mesh,
**When** the driver runs,
**Then** the driver detects `aggregated_region_split.is_empty() OR all_objects_have_empty_paint_data` and emits a "PaintSegmentation skipped" structured event; `replace_slice_ir` is NOT called.

| `cargo test -p slicer-runtime --test executor paint_segmentation_skip_when_no_paint_or_no_opted_in_semantic 2>&1 | tee target/test-output.log`

### AC-N3 — Single g-code SHA across two runs of the same painted slice (determinism)

**Given** a painted mesh,
**When** `pnp_cli slice` runs twice,
**Then** the SHAs match (paint-segmentation output is deterministic).

| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p95-cube-1.gcode && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p95-cube-2.gcode && diff -q /tmp/p95-cube-1.gcode /tmp/p95-cube-2.gcode`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-core paint_segmentation 2>&1 | tee target/test-output.log` (per-sub-step kernel tests)
4. `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log` (AC-17)
5. `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log` (AC-18)
6. `cargo xtask build-guests && cargo xtask build-guests --check`
7. `cargo test --workspace 2>&1 | tee target/test-output.log` (AC-20 — workspace final gate)

Full per-AC matrix lives in `requirements.md`. Per-sub-step crosswalk in `task-map.md`.

## Authoritative Docs

- `docs/specs/orca-paint-segmentation-parity.md` — **NORMATIVE** 1021-line spec; range-read each Phase section (1, 2, 3, 4, 6, 7) per sub-step. Hazard list H561-H567 at §8 is required reading.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P3 — Paint-segmentation port" — sub-step table.
- `docs/02_ir_schemas.md` — SliceIR, SlicedRegion, PaintRegionIR (the latter being deleted; reference only).
- `docs/04_host_scheduler.md` — prepass stage ordering.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm constants (every OrcaSlicer geometric constant divides by 100).

## Doc Impact Statement

A list of specific doc sections that this packet modifies / removes:

- New module `crates/slicer-core/src/algos/paint_segmentation/` — doc-commented at every public symbol — `rg -q 'execute_paint_segmentation_v2' crates/slicer-core/src/algos/paint_segmentation/`.
- `crates/slicer-ir/src/slice_ir.rs` — PaintRegionIR / LayerPaintMap / SemanticRegion type declarations REMOVED — `! rg -q 'PaintRegionIR' crates/slicer-ir/src/`.

`docs/01`, `docs/02`, `docs/04`, `docs/07` updates are deferred to P5c (99). `docs/specs/orca-paint-segmentation-parity.md` flips `Status:` from `awaiting Slice Rework` to `implemented` in P5c.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` or `SUMMARY` (≤ 200 words). Code snippets ≤ 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` (~2,539 LOC) — the source-of-truth for Phases 1–7. Per-sub-step: SUMMARY against the specific phase section by line range.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — `PaintedRegion` / `FuzzySkinPaintedRegion` final structure (already confirmed in P1a; reference only).
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:924-1081` — `apply_mm_segmentation` driver shape; SUMMARY confirming our `execute_paint_segmentation_v2` signature aligns.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
