# Implementation Plan: 63_paint-annotator-r-tree

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the budget contract for this step.

## Steps

### Step 1: Add `rstar` dependency and `PaintRegionRTreeIndex` newtype

- Task IDs: none
- Objective: Add `rstar = "0.12"` to `slicer-core/Cargo.toml`, define `PaintRegionRTreeIndex` newtype in `slicer-core/src/paint_region.rs` wrapping `HashMap<u32, HashMap<PaintSemantic, RTree<(BoundingBox2, usize)>>>`. Change `point_in_paint_region` signature to accept `rtree_index: Option<&PaintRegionRTreeIndex>`. Update all call sites to pass `None` (linear scan fallback). Verify compilation (including WASM target) and that existing tests are unaffected when `None` is passed.
- Precondition: Packet 62 complete. `cargo check --workspace` clean.
- Postcondition: `slicer-core` has `rstar` dependency and `PaintRegionRTreeIndex` type. `point_in_paint_region` signature changed. All call sites updated to pass `None`. All existing tests pass unchanged (linear fallback used). WASM build check passes.
- Files allowed to read:
  - `crates/slicer-core/Cargo.toml` — read current `[dependencies]` section
  - `crates/slicer-core/src/paint_region.rs` — read current `point_in_paint_region` signature and callers within the file
  - `crates/slicer-core/src/lib.rs` — confirm module structure (where to place the newtype)
- Files allowed to edit:
  - `crates/slicer-core/Cargo.toml` — add `rstar = "0.12"`
  - `crates/slicer-core/src/paint_region.rs` — define `PaintRegionRTreeIndex` newtype; change `point_in_paint_region` signature; add `use rstar::RTree`
  - `crates/slicer-host/src/slice_postprocess.rs` — update `point_in_paint_region` call sites to pass `None`
  - (all test files that call `point_in_paint_region` directly — update signatures to pass `None`)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs` — R-tree construction in later steps
  - `crates/slicer-ir/` — no changes (index is external to IR)
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT pass/fail"
  - "Run `cargo check -p slicer-core --target wasm32-unknown-unknown`; return FACT pass/fail — verify WASM compatibility of rstar"
  - "Run `cargo test -p slicer-core paint_region`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)"
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT or SNIPPETS"
- Context cost: `S`
- Authoritative docs: none
- OrcaSlicer refs: none
- Verification:
  - `cargo check --workspace`
  - `cargo check -p slicer-core --target wasm32-unknown-unknown`
  - `cargo test -p slicer-core paint_region`
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
- Exit condition: `cargo check --workspace` passes. WASM check passes. All tests pass with `rtree_index: None` at all call sites.

### Step 2: Build `PaintRegionRTreeIndex` at harvest time

- Task IDs: none
- Objective: In `harvest_paint_segmentation_ir` (dispatch.rs), after building `per_layer`, iterate and construct one `RTree<(BoundingBox2, usize)>` per `(layer_index, semantic)` key. Insert `(region.aabb.unwrap_or_default(), region_index)` for each `SemanticRegion`. Wrap the completed tree map in `PaintRegionRTreeIndex` and return alongside `PaintRegionIR`. Thread the index through the blackboard to the annotation request.
- Precondition: Step 1 complete (newtype defined, signature changed, all call sites pass `None`).
- Postcondition: `harvest_paint_segmentation_ir` returns both `PaintRegionIR` and `PaintRegionRTreeIndex`. The blackboard and annotation request are updated to carry the index. Query path still uses linear scan (index passed but not yet consumed by R-tree logic).
- Files allowed to read:
  - `crates/slicer-host/src/dispatch.rs` — read `harvest_paint_segmentation_ir` body + call site (updated by packet 62)
  - `crates/slicer-host/src/dispatch.rs` — read `run_prepass_module_impl` around line 2174-2181 (where harvest result is returned)
  - `crates/slicer-host/src/blackboard.rs` — read `commit_paint_regions` and the paint_regions storage field (lines ~260-280) — purpose: where to store the companion index
- Files allowed to edit:
  - `crates/slicer-host/src/dispatch.rs` — build `PaintRegionRTreeIndex` in `harvest_paint_segmentation_ir`; update return type
  - `crates/slicer-host/src/blackboard.rs` — add `paint_region_rtree` field (or equivalent) alongside `paint_regions`
  - `crates/slicer-host/src/slice_postprocess.rs` — add `paint_region_rtree: Option<Arc<PaintRegionRTreeIndex>>` to `SlicePostProcessPaintAnnotationRequest`; populate from blackboard
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/paint_region.rs` — query logic unchanged in this step
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd`; return FACT (pass) or SNIPPETS"
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT or SNIPPETS"
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` — delegate FACT: does `PaintRegionIR` implement `Default`? Needed to confirm the `..Default::default()` pattern in harvest still works.
- OrcaSlicer refs: `03_algorithmic_complexities.md` — delegate FACT: does OrcaSlicer's AABBTreeIndirect use bulk-load or incremental-insert?
- Verification:
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd`
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo check --workspace`
- Exit condition: all dispatched tests pass. `PaintRegionRTreeIndex` is built and threaded through the pipeline but not yet consumed by query logic.

### Step 3: Replace linear scan with R-tree lookup in query path

- Task IDs: none
- Objective: In `point_in_paint_region` (paint_region.rs), when `rtree_index` is `Some`, look up the `RTree` for `(layer_index, semantic)`, call `locate_in_envelope(&query_aabb)`, collect candidate indices, retrieve candidates from `semantic_regions` Vec, run the existing precedence loop on candidates only. When `rtree_index` is `None`, fall back to the existing linear-scan-with-AABB-pre-filter path. Update `is_point_numerically_ambiguous` in `slice_postprocess.rs` to also accept and use the index.
- Precondition: Step 2 complete (index built, threaded through request). `SlicePostProcessPaintAnnotationRequest` has `paint_region_rtree` field. `point_in_paint_region` signature accepts `Option<&PaintRegionRTreeIndex>` (from Step 1).
- Postcondition: `point_in_paint_region` uses O(log N) candidate selection when index is available. Results identical to linear scan. Deserialized IR path (no index) falls back correctly. `is_point_numerically_ambiguous` also uses the index.
- Files allowed to read:
  - `crates/slicer-core/src/paint_region.rs` — read in full (post-packet-62 version, ~130-160 lines)
  - `crates/slicer-host/src/slice_postprocess.rs` — read `is_point_numerically_ambiguous` (lines ~510-540) and the annotation loop (lines ~357-430) — purpose: where to pass the index parameter
- Files allowed to edit:
  - `crates/slicer-core/src/paint_region.rs` — implement R-tree lookup in `point_in_paint_region` when `rtree_index` is `Some`
  - `crates/slicer-host/src/slice_postprocess.rs` — update `is_point_numerically_ambiguous` signature to accept index; pass `request.paint_region_rtree.as_deref()` at all call sites
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs` — harvest logic unchanged
  - `crates/slicer-host/src/blackboard.rs` — storage unchanged
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core paint_region`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)"
  - "Run `cargo test -p slicer-host --test scenario_traces_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test paint_annotation_integration_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`; return FACT or SNIPPETS"
- Context cost: `M`
- Authoritative docs: none
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-core paint_region`
  - `cargo test -p slicer-host --test scenario_traces_tdd`
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`
- Exit condition: all dispatched tests pass. R-tree query path and linear fallback produce identical results.

### Step 4: Update docs and final verification

- Task IDs: none
- Objective: Update `docs/02_ir_schemas.md` with `rtree_index` field note on `PaintRegionIR`. Run end-to-end benchmark to measure additional speedup.
- Precondition: Steps 1-3 complete. All tests pass.
- Postcondition: Docs updated. End-to-end benchmark confirms additional wall-clock reduction from O(log N) lookup vs packet-62 baseline.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — read `PaintRegionIR` struct listing
- Files allowed to edit:
  - `docs/02_ir_schemas.md` — add `rtree_index` field note
- Files explicitly out-of-bounds for this step:
  - All source files — doc + benchmark only
- Expected sub-agent dispatches:
  - "Run `rg -q 'rtree_index\|R-tree\|spatial index' docs/02_ir_schemas.md`; return FACT (found or not found)"
  - "Run `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.stl --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html 2>&1`; return FACT (annotator row wall-clock time in seconds)"
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail"
- Context cost: `S`
- Authoritative docs: `docs/02_ir_schemas.md` — doc edit
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'rtree_index\|R-tree' docs/02_ir_schemas.md`
  - `cargo clippy --workspace -- -D warnings`
  - End-to-end `slicer-host --report` annotator time comparison
- Exit condition: docs updated, clippy clean, end-to-end benchmark shows additional speedup.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Dependency + type addition, no consumers |
| Step 2 | M | R-tree construction at harvest |
| Step 3 | M | R-tree query replacement in paint_region.rs |
| Step 4 | S | Doc update + final benchmark |
| **Aggregate** | **M** | |

## Packet Completion Gate

- All 4 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS):
  - `cargo test -p slicer-core paint_region` → PASS
  - `cargo test -p slicer-host --test scenario_traces_tdd` → PASS
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` → PASS
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd` → PASS
  - `cargo test -p slicer-host --test paint_annotation_integration_tdd` → PASS
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd` → PASS
  - `cargo check --workspace` → PASS
  - `cargo check -p slicer-core --target wasm32-unknown-unknown` → PASS
  - `cargo clippy --workspace -- -D warnings` → PASS
  - End-to-end `slicer-host --report` → annotator time reduced from packet-62 baseline
- `docs/02_ir_schemas.md` updated with `rtree_index` documentation.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-6, AC-N1 through AC-N2).
- Confirm packet-level verification commands are green.
- Confirm the implementer's peak context usage stayed under 70%.

## Benchmark Results

| Metric | Packet 62 baseline | Packet 63 (R-tree) | Delta |
|--------|-------------------|---------------------|-------|
| Pipeline total | 274,752 ms | 205,213 ms | −69,539 ms (−25.3%) |
| PrePass::PaintSegmentation | 92,057 ms | 73,197 ms | −18,860 ms (−20.5%) |
| Layer::SlicePostProcess (all threads) | 1,370,992 ms | 1,116,339 ms | −254,653 ms (−18.6%) |
| Peak host mem | 5.88 GB | 6.39 GB | +0.51 GB (+8.7%) |

Packet 62 baseline source: `.ralph/specs/62_paint-annotator-performance/implementation-plan.md` Acceptance Ceremony.
Packet 63 model: `benchy_4color.3mf`, 292 layers, 12 threads.
