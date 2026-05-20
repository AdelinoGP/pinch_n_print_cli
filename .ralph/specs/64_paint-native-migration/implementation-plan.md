# Implementation Plan: 64_paint-native-migration

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the budget contract for this step.

## Steps

### Step 1: Extract shared `group_and_union_paint_regions()` and bring `execute_paint_segmentation()` to feature parity

- Task IDs:
  - TASK-136
- Objective: Extract the grouping+union+AABB+sort logic from `harvest_paint_segmentation_ir()` into a shared free function `group_and_union_paint_regions(entries: Vec<PaintFacetEntry>) -> PaintRegionIR` in `paint_segmentation.rs`. Replace `execute_paint_segmentation()`'s internal `push_polygon_region()` calls with a call to the shared function, bringing the host implementation to feature parity with the WASM+harvest path (unioned polygons, computed AABB, descending paint_order sort). The `harvest_paint_segmentation_ir()` also calls the shared function — existing output is unchanged, proving byte-identical parity.
- Precondition: `cargo check --workspace` clean on HEAD. Packet 62 and packet 63 changes are already landed.
- Postcondition: `execute_paint_segmentation()` and `harvest_paint_segmentation_ir()` both produce identical `PaintRegionIR` for the same `PaintFacetEntry` input. `cargo test -p slicer-host --test paint_segmentation_executor_tdd` passes unchanged. The shared function signature is `fn group_and_union_paint_regions(entries: Vec<PaintFacetEntry>) -> PaintRegionIR` where `PaintFacetEntry` is a new struct `{ layer_index: u32, object_id: String, semantic: PaintSemantic, value: PaintValue, paint_order: u64, polygons: Vec<ExPolygon> }`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/dispatch.rs` — lines 2003-2172 (`harvest_paint_segmentation_ir` full body, ~170 lines)
  - `crates/slicer-host/src/paint_segmentation.rs` — lines 51-207 (`execute_paint_segmentation` body), lines 209-230 (`push_polygon_region`)
  - `crates/slicer-core/src/polygon_ops.rs` — lines 93-95 (`union` signature), line 62 (hole-loss comment)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/paint_segmentation.rs` — add `PaintFacetEntry` struct, add `group_and_union_paint_regions()`, update `execute_paint_segmentation()` to call it
  - `crates/slicer-host/src/dispatch.rs` — update `harvest_paint_segmentation_ir()` to call the shared function (no logic change, just redirect)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/slice_postprocess.rs` — query path unchanged
  - `crates/slicer-host/src/wit_host.rs` — WIT types unchanged
  - All test files except `paint_segmentation_executor_tdd.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)" — validate shared function produces identical output
  - "Run `cargo check --workspace`; return FACT pass/fail" — compile gate
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` §"PaintRegionIR" — range-read lines 469-488; confirm `PaintRegionIR` struct fields for the shared function's return type
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd`
  - `cargo check --workspace`
- Exit condition: `cargo test -p slicer-host --test paint_segmentation_executor_tdd` passes unchanged. Both `execute_paint_segmentation()` and `harvest_paint_segmentation_ir()` use the shared function.

### Step 2: Add `Layer::PaintRegionAnnotation` stage variant and stage ordering

- Task IDs:
  - TASK-136
- Objective: Add `Layer::PaintRegionAnnotation` variant to the `Layer` stage enum, insert it before `Layer::SlicePostProcess` in `STAGE_ORDER`, and add `"Layer::PaintRegionAnnotation"` to `known_stage_ids()`. No handler is wired yet — pure infrastructure.
- Precondition: Step 1 complete. `cargo check --workspace` clean.
- Postcondition: `Layer::PaintRegionAnnotation` variant exists in the `Layer` enum. All match arms on `Layer` are updated to include the new variant (delegate `cargo check` to discover all affected arms). `STAGE_ORDER` lists `PaintRegionAnnotation` before `SlicePostProcess`. `known_stage_ids()` includes `"Layer::PaintRegionAnnotation"`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — search for `Layer::SlicePostProcess` to locate the enum definition; read the full enum
  - `crates/slicer-host/src/execution_plan.rs` — read `STAGE_ORDER` constant (lines 27-48)
  - `crates/slicer-host/src/manifest.rs` — search for `known_stage_ids`; read the function body
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs` — add `Layer::PaintRegionAnnotation` variant
  - `crates/slicer-host/src/execution_plan.rs` — insert `Layer::PaintRegionAnnotation` before `Layer::SlicePostProcess`
  - `crates/slicer-host/src/manifest.rs` — add `"Layer::PaintRegionAnnotation"` to `known_stage_ids()`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/layer_executor.rs` — handler wiring is Step 3
  - `crates/slicer-host/src/dispatch.rs` — dispatch wiring is Steps 3-4
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT pass/fail plus any match-arm errors with file:line" — discover all match arms that need updating
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — range-read Layer stage ordering section; confirm insertion point
  - `docs/03_wit_and_manifest.md` — range-read `known_stage_ids()` section; confirm allowlist contract
- OrcaSlicer refs: none
- Verification:
  - `cargo check --workspace`
- Exit condition: `cargo check --workspace` passes. No `non-exhaustive pattern` warnings on `Layer` match arms.

### Step 3: Move host paint annotator to `Layer::PaintRegionAnnotation` handler

- Task IDs:
  - TASK-136
- Objective: Add the `Layer::PaintRegionAnnotation` handler in `layer_executor.rs` that calls `execute_slice_postprocess_paint_annotation()`. Remove the post-loop `paint_annotation_ran` guard and the host annotator call from the `SlicePostProcess` section. The annotator should run during the new stage, not after `SlicePostProcess` modules. Add the guard-based fallback: if a WASM module claimed `Layer::PaintRegionAnnotation` and ran, skip the host handler.
- Precondition: Step 2 complete. `Layer::PaintRegionAnnotation` variant exists and compiles.
- Postcondition: `Layer::PaintRegionAnnotation` handler runs `execute_slice_postprocess_paint_annotation()` when no WASM module claims the stage. The `paint_annotation_ran` flag is removed. `SlicePostProcess` no longer runs the host annotator. All existing `slice_postprocess_paint_annotation_tdd` tests pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/layer_executor.rs` — lines 250-500 (stage dispatch loop + paint annotation fallback guard)
  - `crates/slicer-host/src/slice_postprocess.rs` — lines 290-636 (`execute_slice_postprocess_paint_annotation` function + request/response types)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_executor.rs` — add `Layer::PaintRegionAnnotation` handler, remove `paint_annotation_ran` guard and post-loop call
  - `crates/slicer-host/src/dispatch.rs` — if the per-layer dispatch references `paint_annotation_ran`, remove those references; ensure `PaintRegionAnnotation` stage dispatch builds `PaintRegionLayerData` for WASM modules that need it
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/slice_postprocess.rs` — function body unchanged in this step
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT (pass) or SNIPPETS"
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — range-read Layer stage handler section; confirm handler wiring pattern
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo check --workspace`
- Exit condition: `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd` passes. `paint_annotation_ran` is removed. The `PaintRegionAnnotation` handler code path is exercised.

### Step 4: Wire `execute_paint_segmentation()` as `PrePass::PaintSegmentation` host fallback

- Task IDs:
  - TASK-136
- Objective: Add a guard-based host fallback for `PrePass::PaintSegmentation` in `dispatch.rs` / `prepass.rs`. If a WASM module claims the stage and runs (via existing `dispatch_prepass_call()`), the host fallback is skipped. If no module ran, call `execute_paint_segmentation(blackboard.mesh(), blackboard.surface_classification(), blackboard.layer_plan())` and commit the result via `blackboard.commit_paint_regions()`. Map `PaintSegmentationError` variants to the prepass error type.
- Precondition: Step 1 complete (shared function available). Step 3 complete (guard pattern established for `Layer::PaintRegionAnnotation`).
- Postcondition: `PrePass::PaintSegmentation` has a guard-based host fallback. The `prepass_executor_tdd` tests pass with both the WASM-module-present path (AC-3) and the host-fallback path (no module).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/dispatch.rs` — lines 950-990 (PrePass dispatch block), lines 2260-2268 (harvest commit block)
  - `crates/slicer-host/src/prepass.rs` — lines 539-603 (stage handler table, `PrePass::PaintSegmentation` entry, `commit_paint_regions` call)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/dispatch.rs` — add guard-based `PrePass::PaintSegmentation` fallback after `dispatch_prepass_call()`; remove `harvest_paint_segmentation_ir()` call point (keep function body for now — deleted in Step 8)
  - `crates/slicer-host/src/prepass.rs` — add host handler function or inline the fallback at the `PrePass::PaintSegmentation` entry in the stage handler table
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/paint_segmentation.rs` — shared function already extracted in Step 1
  - `crates/slicer-host/src/wit_host.rs` — WIT bindings still used by the WASM path; cleanup is Step 8
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test prepass_executor_tdd`; return FACT (pass) or SNIPPETS"
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` — delegate SUMMARY of dispatch lifecycle; implementer needs the PrePass guard pattern
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test prepass_executor_tdd`
  - `cargo check --workspace`
- Exit condition: `cargo test -p slicer-host --test prepass_executor_tdd` passes. Both WASM-present and host-fallback paths are exercised.

### Step 5: Migrate WASM module tests to host test files

- Task IDs:
  - TASK-136
- Objective: Move `paint_segmentation_tdd.rs` (11 tests) and `paint_region_annotator_tdd.rs` (9 tests) from the deleted module directories to `crates/slicer-host/tests/`. Port the paint-segmentation tests from WASM `PrepassModule::run_paint_segmentation()` calls to host `execute_paint_segmentation()` calls. The paint-region-annotator tests already call host types directly — minimal import changes needed.
- Precondition: Steps 1-4 complete. Host functions are wired and tested.
- Postcondition: Two new test files in `slicer-host/tests/`: `paint_segmentation_host_tdd.rs` (11 tests) and `paint_region_annotator_host_tdd.rs` (9 tests). All 20 tests pass. The original module test files are deleted (with the directories in Step 7).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/paint-segmentation/tests/paint_segmentation_tdd.rs` — full file (~200 lines, 11 test functions)
  - `modules/core-modules/paint-region-annotator/tests/paint_region_annotator_tdd.rs` — full file (~200 lines, 9 test functions)
  - `crates/slicer-host/tests/paint_segmentation_executor_tdd.rs` — read imports and test structure style for consistency
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/paint_segmentation_host_tdd.rs` — new file
  - `crates/slicer-host/tests/paint_region_annotator_host_tdd.rs` — new file
- Files explicitly out-of-bounds for this step:
  - Module source files — tests only; module source deletion is Step 7
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test paint_segmentation_host_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test paint_region_annotator_host_tdd`; return FACT or SNIPPETS"
- Context cost: `M`
- Authoritative docs: none — test migration only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test paint_segmentation_host_tdd`
  - `cargo test -p slicer-host --test paint_region_annotator_host_tdd`
- Exit condition: both new test files pass with 11 and 9 tests respectively.

### Step 6: Rewrite host test files that load `.wasm` to exercise guard-based fallbacks

- Task IDs:
  - TASK-136
- Objective: Rewrite 5 host test files that load paint-segmentation or paint-region-annotator `.wasm` files to instead exercise the guard-based host fallback path. Each rewrite must preserve the original test's assertion strength while removing `.wasm` path dependencies.
- Precondition: Steps 1-4 complete (guard-based fallbacks wired). Step 5 complete (migrated tests pass — confirms host functions are correct).
- Postcondition: All 5 test files pass without loading `.wasm`. No test references deleted module paths or IDs.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/tests/dispatch_tdd.rs` — search for "paint-segmentation.wasm" (8 references); read each test function that loads `.wasm`
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` — search for "paint-segmentation.wasm"; read the `.wasm`-loading test functions
  - `crates/slicer-host/tests/prepass_executor_tdd.rs` — search for "com.example.paint-segmentation"; read the test functions using this ID
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — search for "paint-segmentation" and "paint-region-annotator"; read the module-name references
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — search for "com.core.paint-segmentation" and "com.core.paint-region-annotator"; read the manifest test data
- Files allowed to edit (≤ 3 per parallel worker; total 5):
  - `crates/slicer-host/tests/dispatch_tdd.rs`
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs`
  - `crates/slicer-host/tests/prepass_executor_tdd.rs`
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
  - `crates/slicer-host/tests/manifest_ingestion_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - Module directories — not yet deleted (Step 7)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test dispatch_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test manifest_ingestion_tdd`; return FACT or SNIPPETS"
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `M` (largest single step — 5 test files with diverse patterns)
- Authoritative docs: none — test rewrite only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test dispatch_tdd`
  - `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd`
  - `cargo test -p slicer-host --test prepass_executor_tdd`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd`
  - `cargo test -p slicer-host --test manifest_ingestion_tdd`
  - `cargo check --workspace`
- Exit condition: all 5 test files pass. No test loads a deleted module path or references a deleted module ID.

### Step 7: Delete WASM module directories, update build script, clean stale artifacts

- Task IDs:
  - TASK-136
- Objective: Delete `modules/core-modules/paint-segmentation/` and `modules/core-modules/paint-region-annotator/` entirely. Remove both from `build-core-modules.sh`. Delete stale `.wasm` artifacts. Run `--check` to verify no stale references.
- Precondition: Steps 1-6 complete. All tests pass without loading `.wasm` from these modules. No code references the deleted modules.
- Postcondition: Module directories deleted. Build script excludes both modules. `build-core-modules.sh --check` reports no `STALE:` for paint-segmentation or paint-region-annotator.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/build-core-modules.sh` — search for "paint-segmentation" and "paint-region-annotator"; read the build lines
- Files allowed to edit (≤ 3):
  - `modules/core-modules/build-core-modules.sh` — remove build lines for both modules
- Files deleted:
  - `modules/core-modules/paint-segmentation/` (entire directory)
  - `modules/core-modules/paint-region-annotator/` (entire directory)
- Files explicitly out-of-bounds for this step:
  - Remaining core modules — unchanged
  - `test-guests/` — preserved
- Expected sub-agent dispatches:
  - "Run `bash modules/core-modules/build-core-modules.sh`; return FACT pass/fail" — confirm no build errors from deleted modules
  - "Run `bash modules/core-modules/build-core-modules.sh --check`; return FACT (STALE: or CLEAN:)" — confirm no stale references
  - "Run `cargo check --workspace`; return FACT pass/fail" — compile gate
- Context cost: `S`
- Authoritative docs: none — file deletion only
- OrcaSlicer refs: none
- Verification:
  - `bash modules/core-modules/build-core-modules.sh`
  - `bash modules/core-modules/build-core-modules.sh --check`
  - `cargo check --workspace`
- Exit condition: build script runs without referencing deleted modules. `--check` reports clean. `cargo check --workspace` passes.

### Step 8: Remove dead WIT code

- Task IDs:
  - TASK-136
- Objective: Remove all WIT code that existed solely for the paint-segmentation WASM guest: `paint_region_entries` field on `HostExecutionContext`, `push_paint_segmentation_output()`, `paint_region_entries()` getter, `HostPaintSegmentationOutput` trait impl, `PaintSegmentationOutputData` struct, `object_mesh_to_wit_paint_segmentation_view()`, `ir_to_wit_paint_stroke_view()`, `ir_to_wit_paint_layer_view()`, WIT records `paint-region-entry` and `paint-segmentation-output`, `harvest_paint_segmentation_ir()` function body, and `harvest_paint_segmentation_ir_from_ctx()` in `dispatch_helpers.rs`. **Keep** everything needed by `tree-support` and `traditional-support`: `PaintRegionLayerData`, `paint_region_ir_to_layer_data()`, `paint_semantic_key()`, `paint_semantic_to_string()`, `ir_to_wit_paint_value_view()`, `push_paint_region_layer_view()`, `HostPaintRegionLayerView` impl, `build_paint_layer_data()`.
- Precondition: Step 7 complete (modules deleted — dead code is now truly unreferenced).
- Postcondition: No dead WIT symbols remain. `cargo check --workspace` passes with no `unused` warnings for the removed symbols. Support modules (`tree-support`, `traditional-support`) still compile and their tests pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/wit_host.rs` — read lines 469-471 (PaintSegmentationOutputData), 1456-1461 (paint_region_entries), 1742 (getter), 1985 (push fn), 2628-2760 (converter fns), 4383-4425 (HostPaintSegmentationOutput impl). Keep: lines 216-228 (PaintRegionLayerData), 2644-2693 (paint_region_ir_to_layer_data)
  - `crates/slicer-host/src/dispatch.rs` — read lines 2003-2172 (`harvest_paint_segmentation_ir` body for deletion), lines 955-988 (push_paint_segmentation_output call for deletion)
  - `crates/slicer-host/src/dispatch_helpers.rs` — read full file; check if `harvest_paint_segmentation_ir_from_ctx` is the only content
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/wit_host.rs` — remove dead code, keep support-module path
  - `crates/slicer-host/src/dispatch.rs` — remove harvest function body and dead WIT calls
  - `crates/slicer-host/src/dispatch_helpers.rs` — remove function (delete file if empty)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/tree-support/`, `modules/core-modules/traditional-support/` — delegate fact-checks only
- Expected sub-agent dispatches:
  - "Find all callers of `harvest_paint_segmentation_ir`; return LOCATIONS" — confirm no orphan call sites
  - "Run `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd`; return FACT or SNIPPETS" — confirm support module path intact
  - "Run `cargo check --workspace`; return FACT pass/fail plus any `unused` warnings for removed symbols" — dead code verification
- Context cost: `M`
- Authoritative docs: none — dead code removal only
- OrcaSlicer refs: none
- Verification:
  - `cargo check --workspace`
  - `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd`
- Exit condition: `cargo check --workspace` clean. `region_mapping_paint_semantic_tdd` passes. No orphan callers of deleted functions.

### Step 9: Apply per-point parallelism with `par_chunks(32)`

- Task IDs:
  - TASK-136
- Objective: Replace the current per-polygon `par_iter()` in `execute_slice_postprocess_paint_annotation` with a flattened-per-semantic `par_chunks(32)` on contour points. For each semantic, collect all contour points from all polygons in all regions into a single `Vec<Point2>` (with index tracking), apply `par_chunks(32).map(...)` for containment checks, and merge thread-local `warnings` and `degraded` flags.
- Precondition: Steps 1-8 complete. `execute_slice_postprocess_paint_annotation` is the always-on handler for `Layer::PaintRegionAnnotation`. All existing annotation tests pass.
- Postcondition: Contour points are processed in parallel chunks of 32. Results are identical to the serial path (verified by `slice_postprocess_paint_annotation_tdd`). On a 16-thread benchy_4color run, all threads show >50% utilization during the `PaintRegionAnnotation` stage.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/slice_postprocess.rs` — read lines 357-430 (contour point loop body with current `par_iter()`)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/slice_postprocess.rs` — flatten points, apply `par_chunks(32)`, merge thread-local accumulators
- Files explicitly out-of-bounds for this step:
  - All other files — this step touches only the annotation loop body
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd -- --nocapture`; return FACT or SNIPPETS"
  - "Run `cargo test -p slicer-host --test paint_annotation_integration_tdd`; return FACT or SNIPPETS"
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail"
- Context cost: `S`
- Authoritative docs: none — performance optimization only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`
  - `cargo test -p slicer-host --test paint_annotation_integration_tdd`
  - `cargo clippy -p slicer-host`
- Exit condition: all dispatched tests pass. `par_chunks(32)` replaces `par_iter()` with correct thread-local merge. `cargo clippy` clean.

### Step 10: Add `union_paint_regions_at_harvest` config toggle

- Task IDs:
  - TASK-136
- Objective: Add a `bool` config key `union_paint_regions_at_harvest` (default `true`) to `group_and_union_paint_regions()`. When `false`, the function skips `slicer_core::union()` but still computes AABB from individual polygon contour points and still sorts regions. The config key is documented as a temporary benchmarking toggle.
- Precondition: Step 1 complete (shared function exists). Steps 2-9 complete.
- Postcondition: `union_paint_regions_at_harvest: true` produces unioned polygons (current behavior). `false` produces un-unioned polygons with AABB. Both paths produce semantically correct `PaintRegionIR`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-host/src/paint_segmentation.rs` — read `group_and_union_paint_regions()` and `execute_paint_segmentation()` for config plumbing
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/paint_segmentation.rs` — add config parameter, branch on it in the shared function
- Files explicitly out-of-bounds for this step:
  - All other files — config change only
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd`; return FACT or SNIPPETS" — both config paths tested
  - "Run `cargo check --workspace`; return FACT pass/fail"
- Context cost: `S`
- Authoritative docs: none — config toggle only
- OrcaSlicer refs: none
- Verification:
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd`
  - `cargo check --workspace`
- Exit condition: `cargo test -p slicer-host --test paint_segmentation_executor_tdd` passes. The `false` path is tested.

### Step 11: Update documentation

- Task IDs:
  - TASK-136
- Objective: Update `docs/04_host_scheduler.md` with the new `Layer::PaintRegionAnnotation` stage and guard-based fallback contracts for both stages. Update `docs/07_implementation_status.md` with a new task row for this consolidation.
- Precondition: Steps 1-10 complete. All tests pass.
- Postcondition: Both doc files contain the specified content (verifiable by `rg` commands in the Doc Impact Statement).
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/04_host_scheduler.md` — read Layer stage ordering section and PrePass stage ordering section
  - `docs/07_implementation_status.md` — read the last few task rows for insertion point reference
- Files allowed to edit (≤ 3):
  - `docs/04_host_scheduler.md` — add `Layer::PaintRegionAnnotation` documentation and guard-based fallback contracts
  - `docs/07_implementation_status.md` — add task row
- Files explicitly out-of-bounds for this step:
  - All source files — doc-only change
- Expected sub-agent dispatches:
  - "Run `rg 'Layer::PaintRegionAnnotation' docs/04_host_scheduler.md`; return FACT (found or not found)"
  - "Run `rg 'guard-based fallback' docs/04_host_scheduler.md`; return FACT (found or not found)"
  - "Run `rg 'paint-native-migration' docs/07_implementation_status.md`; return FACT (found or not found)"
- Context cost: `S`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — edit the canonical scheduler doc
  - `docs/07_implementation_status.md` — edit the backlog ledger
- OrcaSlicer refs: none
- Verification:
  - `rg -q 'Layer::PaintRegionAnnotation' docs/04_host_scheduler.md`
  - `rg -q 'guard-based fallback' docs/04_host_scheduler.md`
  - `rg -q 'paint-native-migration' docs/07_implementation_status.md`
- Exit condition: all three `rg` commands return found.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Single shared function extraction + parity update |
| Step 2 | M | New stage variant — requires discovering all match arms via `cargo check` |
| Step 3 | M | Handler wiring + guard removal — touches the dispatch loop |
| Step 4 | M | PrePass fallback wiring — similar guard pattern to Step 3 |
| Step 5 | M | Test migration — 20 tests across 2 new files, porting WASM calls to host calls |
| Step 6 | M | Largest step — 5 test files rewritten for guard-based fallback paths |
| Step 7 | S | File/directory deletion + build script update |
| Step 8 | M | Dead code removal across `wit_host.rs` and `dispatch.rs` — must verify no orphan callers |
| Step 9 | S | `par_chunks(32)` in one function body |
| Step 10 | S | Config toggle in shared function |
| Step 11 | S | Doc updates only |
| **Aggregate** | **M** | |

## Packet Completion Gate

- All 11 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS):
  - `cargo test -p slicer-host --test paint_segmentation_executor_tdd` → PASS (AC-5, AC-8)
  - `cargo test -p slicer-host --test paint_segmentation_host_tdd` → PASS (AC-9)
  - `cargo test -p slicer-host --test paint_region_annotator_host_tdd` → PASS (AC-9)
  - `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd` → PASS (AC-2, AC-7)
  - `cargo test -p slicer-host --test dispatch_tdd` → PASS (AC-10)
  - `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd` → PASS (AC-10)
  - `cargo test -p slicer-host --test prepass_executor_tdd` → PASS (AC-3)
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd` → PASS (AC-10)
  - `cargo test -p slicer-host --test manifest_ingestion_tdd` → PASS (AC-10)
  - `cargo test -p slicer-host --test paint_annotation_integration_tdd` → PASS (AC-N4)
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` → PASS (AC-1)
  - `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd` → PASS (AC-6)
  - `cargo test -p slicer-host --test paint_region_transport_widening_tdd` → PASS (AC-5)
  - `cargo check --workspace` → PASS
  - `cargo clippy --workspace -- -D warnings` → PASS
  - `bash modules/core-modules/build-core-modules.sh` → PASS (AC-N1, AC-N2)
  - `bash modules/core-modules/build-core-modules.sh --check` → CLEAN (AC-N5)
  - End-to-end `slicer-host --report` → no missing-module errors (AC-1)
- `docs/04_host_scheduler.md` updated with `Layer::PaintRegionAnnotation` stage and guard-based fallback contracts (AC-11).
- `docs/07_implementation_status.md` updated with task row (AC-12).
- Stale `.wasm` artifacts cleaned (AC-N5).
- No open activation-blocking questions remain.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-12, AC-N1 through AC-N5).
- Confirm packet-level verification commands are green.
- Record the progressive benchmark measurements:
  - Baseline: Pre-change paint-region-annotator guest CPU-ms (1.37M), prepass time (~92s with union from packet 62), total pipeline time (~275s)
  - After migration: host-only paint annotation CPU-ms, prepass time, total pipeline time
  - After per-point parallelism: paint annotation CPU-ms wall-clock vs thread count
  - After union toggle (false): prepass time vs per-layer time tradeoff
- Record code:504 warning count at each phase (should stay at zero from packet 62).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
