# Requirements: 95_paint-segmentation-orca-port

## Packet Metadata

- Grouped task IDs:
  - `TASK-245` — Paint-segmentation port (Phases 1, 2, 3, 4, 6, 7 from OrcaSlicer; replaces broken existing kernel; deletes PaintRegionIR + inlines into SliceIR via replace_slice_ir).
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P3 — Paint-segmentation port"
- Packet status: `draft`
- Aggregate context cost: `M` (deliberately broken into ~17 sub-steps each S/M to keep aggregate at M)

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

## In Scope

The 17 sub-steps from the roadmap's P3 sub-step table. Implementation-plan.md owns the per-step detail; this list summarizes:

- Sub-step 0: Polygon helpers (9 new functions in `polygon_ops.rs`).
- Sub-step 1: `triangle_z_intersection` pure math.
- Sub-step 2: `EdgeGrid` + `visit_cells_intersecting_line`.
- Sub-step 3: `PaintedLineVisitor` + `PaintedLine` private type.
- Sub-step 4: Phase 1 slice preprocess driver.
- Sub-step 5: Phase 3 driver (object × extruder × facet → painted_lines).
- Sub-step 6: `post_process_painted_lines` + `colorize_contours` + `ColoredLine`.
- Sub-step 7: `boostvoronoi` dep + API spike + `MMU_Graph` construction.
- Sub-step 8: `remove_multiple_edges_in_vertices` + `remove_nodes_with_one_arc`.
- Sub-step 9: `extract_colored_segments` with leftmost-arc + `Option<usize>` sentinel.
- Sub-step 10: `slice_mesh_slabs` helper in `triangle_mesh_slicer.rs`.
- Sub-step 11: Phase 6 top/bottom propagation.
- Sub-step 12: Phase 7 variant-chain composition via geometric composition.
- Sub-step 13: New driver `execute_paint_segmentation_v2`.
- Sub-step 14: Modifier-volume sub-pipeline preserved → `segment_annotations[SupportEnforcer/Blocker]`.
- Sub-step 15: Wire into prepass driver at new position (between `shell_classification` and `support_geometry`); replace_slice_ir commit.
- Sub-step 16: Delete `execute_paint_segmentation` (old), `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `paint_region.rs`, `Blackboard::paint_regions` / `commit_paint_regions` / `PaintRegionRTreeIndex` / `point_in_paint_region`.
- Sub-step 17: Stub or remove `run_paint_annotation` at `layer_executor.rs:494-528`; same for `execute_slice_postprocess_paint_annotation` at `slice_postprocess.rs:302`.

## Out of Scope

- Phase 5 (width limiting + interlocking) — P4 (96).
- WASM `modules/core-modules/mesh-segmentation/` deletion — P5a (97).
- Loader changes (per-channel symmetry) — P5b (98).
- Doc updates — P5c (99).
- Performance optimization beyond Rayon + 64-mutex bucket (e.g., single-pass Voronoi over multi-color sites, option Q from grilling) — deferred.
- `PaintValue::Vector(Vec<f32>)` IR addition — deferred.
- `host:raw_slice` promotion — deferred.
- 3MF community paint-channel ingestion — deferred.

## Authoritative Docs

- `docs/specs/orca-paint-segmentation-parity.md` — **PRIMARY NORMATIVE** spec. Per-sub-step range-reads.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P3" — sub-step table + driver positioning.
- `docs/02_ir_schemas.md` — `SliceIR`, `SlicedRegion`.
- `docs/04_host_scheduler.md` — prepass driver shape.
- `docs/08_coordinate_system.md` — coordinate constants (1 unit = 100 nm).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` or `SUMMARY` (≤ 200 words).

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` — every Phase. SUMMARY by line range per sub-step.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:924-1081` — `apply_mm_segmentation` driver shape.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp:243-289` — PaintedRegion/FuzzySkinPaintedRegion structure (already confirmed in P1a).

## Acceptance Summary

- Positive: `AC-1` through `AC-21`. Refinements:
  - Sub-step 7 (boostvoronoi API spike) is the EARLIEST risk gate. If the crate's API does not support all four required features (line-segment sites, vertex.color metadata, infinite-edge clipping via is_primary/twin, deterministic vertex ordering), implementer escalates BEFORE Phase 4 work and considers fallbacks per the roadmap risk table (spade + custom wrapper, or cxx-bridge to OrcaSlicer's boost::polygon::voronoi).
  - The "12 GREEN cube_4color tests" assertion (AC-17) tolerates 0 failures. If even 1 fails, root-cause before packet close — partial pass is not acceptance.
- Negative: `AC-N1` (PaintRegionIR fully gone), `AC-N2` (short-circuit on no paint), `AC-N3` (determinism on painted slice).
- Cross-packet impact: unblocks P4 (Phase 5).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-core paint_segmentation 2>&1 \| tee target/test-output.log` | All kernel sub-step tests | FACT pass/fail with breakdown |
| `cargo test -p slicer-core polygon_ops 2>&1 \| tee target/test-output.log` | AC-1 — helpers | FACT pass/fail |
| `cargo test -p slicer-core triangle_mesh_slicer 2>&1 \| tee target/test-output.log` | AC-9 — slabs | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 \| tee target/test-output.log` | AC-17 — 12 GREEN | FACT (must show `test result: ok. 12 passed; 0 failed`) |
| `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 \| tee target/test-output.log` | AC-18 — 12 GREEN | FACT (12/0) |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p95-wedge.gcode && sha256sum /tmp/p95-wedge.gcode` | AC-19 — unpainted byte-identical | FACT (sha256) compare to post-P94 baseline |
| `cargo test --workspace 2>&1 \| tee target/test-output.log` | AC-20 — workspace final gate (dispatched) | FACT per-bucket counts |
| `cargo xtask build-guests && cargo xtask build-guests --check` | AC-21 — guest clean | FACT pass/fail |
| `! rg -q 'PaintRegionIR\|point_in_paint_region\|commit_paint_regions' crates/` | AC-N1 — IR deletion sweep | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor paint_segmentation_skip_when_no_paint 2>&1 \| tee target/test-output.log` | AC-N2 — short-circuit | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf ... ; ... ; diff -q ...` | AC-N3 — determinism | FACT exit 0 |

`cargo test --workspace` is justified at packet close — this packet's IR + kernel + driver reshape touches enough crates that workspace gate is the only reliable confirmation.

## Step Completion Expectations

- **Sub-step 7 risk gate**: if `boostvoronoi` API doesn't match, escalate before continuing. Document the alternative chosen in the closure log and update the roadmap deviation log.
- **Sub-steps 0-9 build the kernel ground-up**; sub-step 13 wires them into the driver; sub-step 15 inserts at the new prepass position; sub-step 16 deletes the old surface. Order is non-negotiable — deleting the old surface before the new driver is in place leaves the workspace uncompilable.
- **Sub-step 17 is the safest deletion** (per-layer annotation function); leave it as a no-op stub during the transitional window to keep callers working until they're confirmed unused, then delete.
- **AC-19 byte-identical unpainted** is the regression-guard contract. Any g-code diff is investigated; the short-circuit must be working.
- **Closure log records**: pre/post wedge SHA, pre/post cube_4color SHA (cube SHA will change vs P94 — that's the expected behavior change), the AC-17 / AC-18 per-test pass-count table, the boostvoronoi-API-spike outcome (Step 7 risk gate).

## Context Discipline Notes

- `docs/specs/orca-paint-segmentation-parity.md` is 1021 lines. Range-read by Phase section per sub-step. Do NOT load in full.
- The new `crates/slicer-core/src/algos/paint_segmentation/` module is multiple new files; each step adds 1-2 files. Per-step plan keeps each step to ≤ 3 files.
- The OrcaSlicer-side files are HUGE (`MultiMaterialSegmentation.cpp` is ~2,500 LOC). Always delegate; never load. SUMMARYs by phase.
- `crates/slicer-runtime/src/prepass.rs` is large; range-read at the existing `shell_classification` + `support_geometry` lines (around 561-588) for the new insertion.
- `crates/slicer-core/src/paint_region.rs` (slated for deletion) is small (~93 lines); read briefly to confirm no surprising consumers.
- The cube `.3mf` fixtures are binary; never `Read`. Tests consume via loader.
