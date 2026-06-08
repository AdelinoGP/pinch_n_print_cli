# Implementation Plan: 95_paint-segmentation-orca-port

## Execution Rules

- One sub-step at a time. Each sub-step ships with its own unit tests passing before the next begins.
- Sub-step 7's `boostvoronoi` API spike is a RISK GATE — pause before proceeding to sub-step 8 if any of the four API requirements fails.
- Test output teed to `target/test-output.log`.
- Per-sub-step traceability via `task-map.md`.

## Steps

### Step 0: Capture pre-packet baselines

- Task IDs: `TASK-245`
- Objective: regression-guard SHAs.
- Precondition: P94 closed.
- Postcondition: 2 SHAs recorded (wedge + cube).
- Expected dispatches:
  - "Run `cargo run ... regression_wedge.stl ... && sha256sum`; return FACT".
  - "Run `cargo run ... cube_4color.3mf ... && sha256sum`; return FACT".
- Context cost: `S`.
- Exit condition: SHAs recorded.

### Step 1: Polygon helpers (sub-step 0)

- Task IDs: `TASK-245`
- Objective: AC-1.
- Precondition: Step 0 complete.
- Postcondition: 9 helper functions + unit tests green.
- Files allowed to edit (≤ 3 per commit; multiple commits):
  - `crates/slicer-core/src/polygon_ops.rs`.
  - `crates/slicer-core/tests/polygon_ops_ex_tdd.rs` (NEW or extend existing).
- Expected dispatches:
  - "Run `cargo test -p slicer-core polygon_ops 2>&1 | tee target/test-output.log`; return FACT pass/fail with per-test breakdown".
- Context cost: `M`.
- Authoritative docs: roadmap §"P3 sub-step 0" + helper spec.
- Exit condition: AC-1 satisfied.

### Step 2: `triangle_z_intersection` (sub-step 1) + `EdgeGrid` (sub-step 2) + `PaintedLineVisitor`/`PaintedLine` (sub-step 3)

- Task IDs: `TASK-245`
- Objective: AC-2, AC-3, AC-4 (intermediate).
- Files allowed to edit (≤ 3 per sub-step; multi-commit acceptable):
  - `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/edge_grid.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/painted_line.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (NEW).
  - Unit tests per file.
- Expected dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` for `triangle_z_intersection` shape; return SUMMARY".
  - "Run `cargo test -p slicer-core paint_segmentation::triangle_intersect 2>&1 | tee target/test-output.log`; FACT".
  - "Run `cargo test -p slicer-core paint_segmentation::edge_grid 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-2, AC-3 satisfied.

### Step 3: Phase 1 preprocess (sub-step 4) + Phase 3 driver (sub-step 5)

- Task IDs: `TASK-245`
- Objective: AC-4.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/preprocess.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/phase3.rs` (NEW).
- Expected dispatches:
  - "Summarize Phase 3 from `docs/specs/orca-paint-segmentation-parity.md` — SUMMARY".
  - "Run `cargo test -p slicer-core paint_segmentation::phase3_painted_lines 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-4 satisfied.

### Step 4: Phase 4a/4b `colorize_contours` (sub-step 6)

- Task IDs: `TASK-245`
- Objective: AC-5.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/colorize.rs` (NEW).
- Expected dispatches:
  - "Summarize Phase 4a/4b from spec; SUMMARY".
  - "Run `cargo test -p slicer-core paint_segmentation::colorize 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-5 satisfied.

### Step 5: **RISK GATE** — `boostvoronoi` API spike + `MMU_Graph` (sub-step 7)

- Task IDs: `TASK-245`
- Objective: AC-6 + confirm risk-table mitigation.
- Precondition: Step 4 green.
- Postcondition: `boostvoronoi` dep added; spike confirms all 4 API features; `MMU_Graph` builds on synthetic input.
- Files allowed to read:
  - boostvoronoi crate docs (via `cargo doc` if needed) — delegate via the API spike dispatch.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/Cargo.toml`.
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` (NEW).
- Expected dispatches:
  - "Spike: add boostvoronoi to `slicer-core/Cargo.toml`, write a tiny synthetic test (4 line segments forming a square) that constructs a Voronoi, dumps each vertex.coord, vertex.color, edge.is_primary, edge.twin status. Return SUMMARY ≤ 200 words on whether all four features are usable as the spec assumes. If any fails, name the failure" — purpose: AC-6 + risk gate.
- Context cost: `M`.
- Exit condition: AC-6 satisfied. If risk gate FAILS, escalate before continuing — packet design changes.

### Step 6: Voronoi pruning (sub-step 8) + `extract_colored_segments` (sub-step 9)

- Task IDs: `TASK-245`
- Objective: AC-7, AC-8.
- Files allowed to edit (≤ 3 per sub-step):
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs` (NEW).
- Expected dispatches:
  - "Summarize Phase 4d/4e (`remove_multiple_edges_in_vertices`, `remove_nodes_with_one_arc`) from spec; SUMMARY".
  - "Summarize Phase 4f (`extract_colored_segments`) from spec, focus on H562 repair sentinel; SUMMARY".
  - Per-module test runs.
- Context cost: `M`.
- Exit condition: AC-7, AC-8 satisfied.

### Step 7: `slice_mesh_slabs` (sub-step 10) + Phase 6 propagation (sub-step 11)

- Task IDs: `TASK-245`
- Objective: AC-9, AC-10.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/triangle_mesh_slicer.rs` (extend).
  - `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs` (NEW).
- Expected dispatches:
  - "Summarize Phase 6 from spec; SUMMARY".
  - Per-module test runs.
- Context cost: `M`.
- Exit condition: AC-9, AC-10 satisfied.

### Step 8: Phase 7 variant-chain composition (sub-step 12)

- Task IDs: `TASK-245`
- Objective: AC-11.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/compose_variants.rs` (NEW).
- Expected dispatches:
  - "Summarize Phase 7 from spec, focus on D5 geometric composition (`intersection_ex`, `difference_ex`); SUMMARY".
  - Test run.
- Context cost: `M`.
- Exit condition: AC-11 satisfied.

### Step 9: New driver `execute_paint_segmentation_v2` (sub-step 13)

- Task IDs: `TASK-245`
- Objective: AC-12.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (driver + public entry).
- Expected dispatches:
  - "Summarize `apply_mm_segmentation` from `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:924-1081`; SUMMARY".
  - Test run.
- Context cost: `M`.
- Exit condition: AC-12 satisfied.

### Step 10: Modifier-volume sub-pipeline preserved (sub-step 14)

- Task IDs: `TASK-245`
- Objective: AC-13.
- Files allowed to read:
  - The OLD `paint_segmentation.rs:374-417` (modifier-volume code being salvaged).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs` (NEW).
- Expected dispatches:
  - Test run.
- Context cost: `M`.
- Exit condition: AC-13 satisfied.

### Step 11: Wire into prepass driver at new position (sub-step 15)

- Task IDs: `TASK-245`
- Objective: AC-14.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/prepass.rs` (insert new position; delete old position).
  - `crates/slicer-runtime/src/builtins/paint_segmentation_producer.rs` (update stage name if needed).
- Expected dispatches:
  - "Open `crates/slicer-runtime/src/prepass.rs` lines 555-595 (around shell_classification + support_geometry); return SNIPPETS (≤ 40 lines)".
  - Full workspace check.
- Context cost: `M`.
- Exit condition: AC-14 satisfied.

### Step 12: Delete `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `paint_region.rs`, `Blackboard::paint_regions` + `commit_paint_regions` + `point_in_paint_region` (sub-step 16)

- Task IDs: `TASK-245`
- Objective: AC-15, AC-N1.
- Precondition: Step 11 green (new driver writes via `replace_slice_ir`; old surface confirmed unused).
- Files allowed to read:
  - `crates/slicer-core/src/paint_region.rs` (briefly; confirm consumers).
- Files allowed to edit (≤ 3 per commit; multi-commit):
  - `crates/slicer-ir/src/slice_ir.rs` (type deletions).
  - `crates/slicer-runtime/src/blackboard.rs` (accessor + commit deletion).
  - `crates/slicer-core/src/paint_region.rs` (DELETE entire file).
  - `crates/slicer-core/src/lib.rs` (drop `pub mod paint_region;`).
  - `crates/slicer-runtime/src/slice_postprocess.rs:24` (drop rtree field).
- Expected dispatches:
  - "Run `rg -nE 'PaintRegionIR|LayerPaintMap|SemanticRegion|point_in_paint_region|commit_paint_regions|paint_regions\(\)' crates/`; return LOCATIONS or empty" — purpose: post-delete sweep.
- Context cost: `M`.
- Exit condition: AC-15, AC-N1 satisfied.

### Step 13: Stub or remove `run_paint_annotation` / `execute_slice_postprocess_paint_annotation` (sub-step 17)

- Task IDs: `TASK-245`
- Objective: AC-16.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/layer_executor.rs:494-528` (no-op or remove).
  - `crates/slicer-runtime/src/slice_postprocess.rs:302` (no-op or remove).
- Expected dispatches:
  - Workspace check.
- Context cost: `S`.
- Exit condition: AC-16 satisfied.

### Step 14: Cube_4color + cube_fuzzy_painted RED → GREEN gate

- Task IDs: `TASK-245`
- Objective: AC-17, AC-18.
- Precondition: Steps 11-13 green.
- Files allowed to edit:
  - Only individual cube tests whose assertions need minor alignment with the kernel output (justify each in closure log).
- Expected dispatches:
  - "Run `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; FACT with per-test breakdown".
  - "Run `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: 12/12 + 12/12 PASS.

### Step 15: AC-19 unpainted byte-identical + AC-N3 painted determinism

- Task IDs: `TASK-245`
- Objective: AC-19, AC-N2, AC-N3.
- Expected dispatches:
  - Wedge SHA capture + compare to Step 0 baseline.
  - Painted determinism check (run cube_4color twice, diff).
  - Run `paint_segmentation_skip_when_no_paint` test.
- Context cost: `S`.
- Exit condition: AC-19, AC-N2, AC-N3 satisfied.

### Step 16: Guest WASM + workspace final gate

- Task IDs: `TASK-245`
- Objective: AC-20, AC-21.
- Expected dispatches:
  - "Run `cargo xtask build-guests && cargo xtask build-guests --check`; FACT".
  - "Run `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; FACT per-bucket".
- Context cost: `S` (dispatch-only; the workspace test is long-running but the implementer doesn't absorb the output).
- Exit condition: AC-20, AC-21 satisfied; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Sub-steps covered |
| --- | --- | --- |
| Step 0 | S | (baseline) |
| Step 1 | M | 0 (polygon helpers) |
| Step 2 | M | 1, 2, 3 |
| Step 3 | M | 4, 5 |
| Step 4 | M | 6 |
| Step 5 | M | 7 (RISK GATE) |
| Step 6 | M | 8, 9 |
| Step 7 | M | 10, 11 |
| Step 8 | M | 12 |
| Step 9 | M | 13 |
| Step 10 | M | 14 |
| Step 11 | M | 15 |
| Step 12 | M | 16 |
| Step 13 | S | 17 |
| Step 14 | M | (acceptance — cube RED→GREEN) |
| Step 15 | S | (regression checks) |
| Step 16 | S | (workspace gate) |

Aggregate: M (no L step). Total implementer effort is large; per-step context is bounded.

## Packet Completion Gate

- All 17 sub-steps complete; all 21 ACs + 3 negative cases verified.
- Closure log records: baselines + post-packet SHAs, AC-17 / AC-18 per-test pass counts, boostvoronoi API spike outcome.
- `docs/07_implementation_status.md` updated for `TASK-245` (delegate).

## Acceptance Ceremony

- Re-dispatch every AC; confirm PASS.
- Confirm `cargo test --workspace` green at close (final gate; dispatched).
- Confirm `cargo xtask build-guests --check` clean.
- Peak context usage under 70%.
