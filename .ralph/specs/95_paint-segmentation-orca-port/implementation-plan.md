# Implementation Plan: 95_paint-segmentation-orca-port

## Execution Rules

- One sub-step at a time. Each sub-step ships with its own unit tests passing before the next begins.
- Sub-step 7's `boostvoronoi` API spike is a RISK GATE — pause before proceeding to sub-step 8 if any of the four API requirements fails.
- Test output teed to `target/test-output.log`.
- Per-sub-step traceability via `task-map.md`.

## Steps

### Step 0: Capture pre-packet baselines into closure-log.md

- Task IDs: `TASK-245`
- Objective: regression-guard SHAs. Both SHAs are written to `.ralph/specs/95_paint-segmentation-orca-port/closure-log.md` as `P94_BASELINE_SHA=<hex>` (wedge) and `P94_CUBE_BASELINE_SHA=<hex>` (cube) so AC-19's shell command and Step 15's determinism check can read them back.
- Precondition: P94 closed.
- Postcondition: 2 SHAs recorded in `closure-log.md`.
- Files allowed to edit:
  - `.ralph/specs/95_paint-segmentation-orca-port/closure-log.md` (CREATE or append).
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p95-wedge-baseline.gcode && sha256sum target/p95-wedge-baseline.gcode | awk '{print $1}'`; return FACT (single sha256, hex only)".
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p95-cube-baseline.gcode && sha256sum target/p95-cube-baseline.gcode | awk '{print $1}'`; return FACT (single sha256, hex only)".
  - Then write `P94_BASELINE_SHA=<wedge_hash>` and `P94_CUBE_BASELINE_SHA=<cube_hash>` as separate lines in `closure-log.md` (delegated edit).
- Context cost: `S`.
- Verification: `grep -q 'P94_BASELINE_SHA=[a-f0-9]\{64\}' .ralph/specs/95_paint-segmentation-orca-port/closure-log.md && grep -q 'P94_CUBE_BASELINE_SHA=[a-f0-9]\{64\}' .ralph/specs/95_paint-segmentation-orca-port/closure-log.md` exits 0.
- Exit condition: closure-log.md carries both baseline SHAs.

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
  - boostvoronoi crate docs (via `cargo doc` if needed) and the canonical source <https://codeberg.org/eadf/boostvoronoi_rs> (current v0.12.1; Rust port of Boost 1.76.0 `polygon::voronoi`) — delegate via the API spike dispatch.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/Cargo.toml`.
  - `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` (NEW).
- Expected dispatches:
  - "Spike: add `boostvoronoi = \"0.12\"` to `slicer-core/Cargo.toml`, build the `MMU_Graph` skeleton (no kernel logic yet) on a synthetic 4-line-segment square input, and verify the **only remaining open API question**: construct the diagram TWICE from byte-identical input and compare the emitted `Vertex` sequence (by `(get_id(), x(), y())`). Return FACT: (a) does the vertex sequence match exactly across both runs? (b) if not, what is the divergence shape (different indices? same indices, different coords? reordered?). API references already confirmed via docs.rs — see `design.md` §Read-Only Context (Vertex::get_color/Edge::is_primary/Edge::twin all present); no spike needed for those. If determinism fails, add a sort pass keyed on `(x, y, get_id())` immediately after construction in `voronoi_graph.rs` and re-run the determinism check." — purpose: AC-6 + the one remaining `[FWD]` open question.
- Context cost: `M`.
- Exit condition: AC-6 satisfied. The line-segment-site / `Vertex::get_color` / `Edge::is_primary` / `Edge::twin` API surface is pre-confirmed via docs.rs — those four are NO LONGER risk-gate failure modes. **The one remaining failure mode is non-deterministic vertex emission order**: if the construct-twice comparison diverges, ADD the `(x, y, get_id())` sort pass inside `voronoi_graph.rs` immediately after construction and re-run the determinism check before proceeding to Step 6. Sort-pass fallback is in-packet (no design change); only a *third* unforeseen API mismatch from the spike (none currently expected) would HALT and force the user-facing fallback choice (spade + custom wrapper, or cxx-bridge to OrcaSlicer's `boost::polygon::voronoi`).

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
- Files allowed to read:
  - `.ralph/specs/95_paint-segmentation-orca-port/closure-log.md` — to retrieve `P94_BASELINE_SHA=<hex>` for the AC-19 comparison.
- Expected dispatches:
  - "Run the AC-19 baseline-compare shell command (see `packet.spec.md` AC-19): `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p95-wedge-post.gcode && test \"$(sha256sum target/p95-wedge-post.gcode | awk '{print $1}')\" = \"$(grep -oE 'P94_BASELINE_SHA=[a-f0-9]+' .ralph/specs/95_paint-segmentation-orca-port/closure-log.md | head -1 | cut -d= -f2)\"`; return FACT exit code".
  - Painted determinism check (run cube_4color twice, diff) per AC-N3's verification command.
  - Run `paint_segmentation_skip_when_no_paint_or_no_opted_in_semantic` test for AC-N2.
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

### Step 17: **REOPEN — Prereq**: fix off-by-one 3MF extruder mapping

- Task IDs: `TASK-245`
- Objective: 3MF object-level `extruder=N` (1-indexed) must produce `T<N-1>` in gcode (0-indexed). Currently `crates/slicer-runtime/src/layer_executor.rs:675` reads `ConfigValue::Int(n)` directly as the 0-indexed tool index, so `extruder=1` ships as `T1` instead of `T0`. This pre-existed packet 95 but was masked by v1's paint pipeline; once v2's paint dispatch lands, this becomes the dominant wall-tool source for the BASE chain and must be correct.
- Precondition: P95 reopened. Step 17 lands BEFORE Step 18 because Step 19's AC-22 gate fails on the bug.
- Postcondition: Conversion happens at exactly ONE seam (either at the loader when stamping `extensions["extruder"]` into ResolvedConfig, OR at the executor read site). Unit test asserts: 3MF `<metadata key="extruder" value="1"/>` → gcode `T0`; `value="3"` → gcode `T2`.
- Files allowed to edit (≤ 3): trace via dispatch; expected sites are `crates/slicer-model-io/src/loader.rs` (loader stamp site) OR `crates/slicer-runtime/src/layer_executor.rs:669-680` (executor read site).
- Expected dispatches:
  - "Grep `extruder` in `crates/slicer-runtime/src/layer_executor.rs` + `crates/slicer-model-io/src/loader.rs` + `crates/slicer-runtime/src/prepass.rs`; return LOCATIONS of every read/write of the extruder config key + a one-line sketch of which seam is the natural 1→0 conversion site."
  - "Land the fix at the chosen seam; add a unit test in `crates/slicer-runtime/tests/unit/` (or appropriate existing file); run targeted cargo test; FACT pass/fail."
- Context cost: `S`.
- Verification: `cargo test -p slicer-runtime --test unit extruder_1_indexed_to_0_indexed 2>&1 | tee target/test-output.log | grep -q 'test result: ok'`.
- Exit condition: 1-indexed → 0-indexed conversion in place; unit test green; no regression in existing `cargo check --workspace --all-targets`.

### Step 18: **REOPEN — AC-22 RED tests** land first (gcode-behavior gate)

- Task IDs: `TASK-245`
- Objective: AC-22. New gcode-output behavior tests that the diagnose session identified as missing. MUST land RED (i.e., test exists and fails on current main) before Step 19's kernel fix begins, so the kernel fix has a concrete falsifying signal.
- Precondition: Step 17 complete.
- Postcondition: New executor test file `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` exists with (a) test asserting unique `T<N>` set in emitted gcode for cube_4color = `{0,1,2,3}`; (b) test asserting per-layer `;TYPE:Outer wall` block count within ±1 of an unpainted cube baseline; (c) test asserting fuzzy-skin coordinate jitter present only on painted faces of cube_fuzzyPainted. Tests RED on first run.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (NEW).
  - `crates/slicer-runtime/tests/executor/main.rs` (register the new test module).
- Expected dispatches:
  - "Sketch the test fixtures: cube_4color.3mf already in resources/; identify an unpainted 25mm cube fixture (e.g. `resources/20mm_cube.obj` or generate a synthetic one)."
  - "Write the test bodies; run; FACT pass/fail (must be RED — failures expected and intentional)."
- Context cost: `M`.
- Exit condition: tests compile, run, RED with the documented failure mode matching the diagnose audit findings.

### Step 19: **REOPEN — D9 dispatch wiring (Option B′)** — drive AC-22 GREEN

- Task IDs: `TASK-245`
- Objective: AC-22 GREEN. Wire variant_chain through the planning, dispatch, and tool-selection layers so the per-variant SlicedRegions produced by the kernel actually drive per-tool perimeter geometry and gcode tool dispatch.
- Precondition: Steps 17 + 18 complete; AC-22 tests RED.
- Postcondition: AC-22 GREEN. All prior ACs (AC-1..21 + AC-N1..3) remain GREEN. cube_4color gcode emits `{T0,T1,T2,T3}`; per-layer perimeter count matches unpainted baseline ±1.
- Files allowed to edit (≤ 3 per commit; multi-commit acceptable):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (kernel: unique region_id per painted SlicedRegion; cell-tiling assertion; BASE-chain narrowing to D14 carrier only).
  - `crates/slicer-runtime/src/layer_executor.rs:683-712` (set feature_flags.tool_index from region.variant_chain Material entry before perimeter module call; same for infill region_id assignment).
  - `crates/slicer-wasm-host/src/host.rs:5071-5108` (verify PerimeterRegionOrigin tuple splits per-variant cleanly with the new unique region_id; extend tuple if needed).
- Expected dispatches:
  - "Audit `crates/slicer-core/src/algos/region_mapping.rs:493-647` cross-product expansion: does RegionPlan.stage_modules get populated per painted-variant RegionKey? Or does the variant inherit BASE's full unfiltered module list? Return FACT + ≤ 10 line snippet of the stage_modules assignment site."
  - "Implement unique region_id strategy in paint_segmentation/mod.rs: `region_id = base_region_id * STRIDE + variant_chain_hash(variant_chain)` where STRIDE is large enough that base region_ids never collide (recommend 1_000_000). Add `debug_assert!(union_ex(painted_chain.polygons).area >= base.area * (1 - eps) && <= base.area * (1 + eps))` for cell-tiling. FACT pass/fail of kernel unit tests."
  - "In layer_executor.rs:683, derive `paint_tool` from `region.variant_chain` Material/ToolIndex when `feature_flags.tool_index` is None. Trace whether `region` here is the source perimeter region or the painted SlicedRegion — need to resolve which path produces the variant_chain info accessible at this point. FACT + ≤ 10 line snippet."
  - "Run AC-22 + AC-17 + AC-18 + cube_4color test bucket; FACT per-test counts."
- Context cost: `L` (acceptable here because this step is the integration point — splitting further would multiply rework risk on shared files).
- Exit condition: AC-22 GREEN. AC-17/AC-18 still GREEN. AC-19 unpainted wedge still SHA-identical to P94_BASELINE_SHA. `cargo check --workspace --all-targets` clean.

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
| Step 17 | S | (prereq — extruder 1-indexed fix) |
| Step 18 | M | (AC-22 RED tests) |
| Step 19 | L | (D9 dispatch wiring; integration point) |

Aggregate: M (Step 19 is L by design — single integration point spanning kernel + executor + host; splitting would multiply rework risk on shared files).

## Packet Completion Gate

- All 17 sub-steps complete; all 22 ACs + 3 negative cases verified.
- Closure log records: baselines + post-packet SHAs, AC-17 / AC-18 per-test pass counts, boostvoronoi API spike outcome.
- `docs/07_implementation_status.md` updated for `TASK-245` (delegate).

## Acceptance Ceremony

- Re-dispatch every AC; confirm PASS.
- Confirm `cargo test --workspace` green at close (final gate; dispatched).
- Confirm `cargo xtask build-guests --check` clean.
- Peak context usage under 70%.
