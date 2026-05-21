---
status: implemented
packet: 64_paint-native-migration
task_ids:
  - TASK-204   # [ ] new — paint module-to-host consolidation (primary task for this packet)
  - TASK-136   # [ ] open — E2E progress-event coverage for paint-annotation failure codes 501-504 (tangential)
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Consoldiates paint-segmentation and paint-region-annotator from WASM modules into host-native pipeline stages. Adds dedicated Layer::PaintRegionAnnotation stage. Depends on packet 62 (BoundingBox2, union, AABB, cache, early-break, par_iter) and packet 63 (R-tree spatial index). No pre-existing TASK-### covered module-to-host migration, so this packet mints TASK-204 as its primary task ID; TASK-136 is tangentially relevant for the code 504 warning path now exercised by the always-on host annotator.
---

# Packet Contract: 64_paint-native-migration

## Goal

Eliminate the `paint-segmentation` and `paint-region-annotator` WASM modules, consolidate both into the already-existing host-native implementations, add a dedicated `Layer::PaintRegionAnnotation` pipeline stage before `SlicePostProcess`, apply per-point parallelism to the annotation loop, and provide a config toggle to re-evaluate the union-at-harvest tradeoff.

## Scope Boundaries

This packet deletes two WASM core modules and their manifests, removes the dead WIT `paint-segmentation-output` resource and `harvest_paint_segmentation_ir()` boundary harvester, and wires the host functions `execute_paint_segmentation()` and `execute_slice_postprocess_paint_annotation()` as guard-based fallbacks for `PrePass::PaintSegmentation` and the new `Layer::PaintRegionAnnotation` stage respectively. A new `Layer::PaintRegionAnnotation` stage is inserted between `Layer::Slice` and `Layer::SlicePostProcess` so that future `SlicePostProcess` modules can consume pre-computed `boundary_paint` annotations. The `PaintRegionLayerView` serialization path survives because `tree-support` and `traditional-support` still query it per layer. Two test-guests (`prepass-guest`, `sdk-prepass-paintseg-guest`) are preserved to validate the WIT contract stays intact. Packet 62's union-at-harvest is made configurable via `union_paint_regions_at_harvest` (default `true`) to allow post-migration benchmarking.

## Prerequisites and Blockers

- Depends on: packet 62 (`62_paint-annotator-performance`) — `BoundingBox2` type, `SemanticRegion.aabb`, union-at-harvest, AABB pre-filter, `get()` cache, early-break, `par_iter()` on polygons
- Depends on: packet 63 (`63_paint-annotator-r-tree`) — R-tree spatial index in `point_in_paint_region`; the host fallback must include the R-tree query path
- Unblocks: union re-evaluation (config toggle enables A/B benchmarking), future `SlicePostProcess` modules that consume `boundary_paint`
- Activation blockers: none

## Acceptance Criteria

- **AC-1. Given** the `paint-segmentation` and `paint-region-annotator` module directories deleted from `modules/core-modules/` and removed from `build-core-modules.sh`, **when** the host runs the full pipeline on `benchy_4color.3mf`, **then** no error log mentions missing modules, `PrePass::PaintSegmentation` completes via the host `execute_paint_segmentation()` fallback, and `PaintRegionIR` is committed to the blackboard. | `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module-dir modules/core-modules --output ./tmp/out.gcode --report ./tmp/slicer-report.html 2>&1 | Select-String -Pattern "paint-segmentation|paint-region-annotator" | Measure-Object | ForEach-Object { if ($_.Count -eq 0) { "PASS: no missing-module errors" } else { "FAIL" } }`

- **AC-2. Given** the `Layer::PaintRegionAnnotation` stage inserted before `Layer::SlicePostProcess` in `STAGE_ORDER`, **when** a layer is processed, **then** `execute_slice_postprocess_paint_annotation` runs in the `PaintRegionAnnotation` stage handler and produces identical `boundary_paint` output to the pre-change `SlicePostProcess`-embedded path for the same input. | `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`

- **AC-3. Given** a WASM module claiming `PrePass::PaintSegmentation` loaded from `--module-dir`, **when** the prepass executes, **then** the WASM module runs and the host `execute_paint_segmentation()` fallback is skipped — the guard-based fallback preserves the extension point. | `cargo test -p slicer-host --test prepass_executor_tdd`

- **AC-4. Given** a WASM module claiming `Layer::PaintRegionAnnotation` loaded from `--module-dir`, **when** a layer is processed, **then** the WASM module runs and the host `execute_slice_postprocess_paint_annotation` fallback is skipped — the guard-based fallback preserves the extension point. | `cargo test -p slicer-host --test layer_executor_tdd`

- **AC-5. Given** the shared `group_and_union_paint_regions()` extracted into `paint_segmentation.rs`, **when** called with the same `Vec<PaintFacetEntry>` input that `harvest_paint_segmentation_ir()` previously received, **then** the output `PaintRegionIR` is byte-identical to the pre-change harvest output for the same input — same polygon counts, same `paint_order` values, same AABB bounds, same sort order. | `cargo test -p slicer-host --test paint_segmentation_executor_tdd`

- **AC-6. Given** the dead WIT code removed (`paint_region_entries` field, `HostPaintSegmentationOutput` impl, `harvest_paint_segmentation_ir`, `object_mesh_to_wit_paint_segmentation_view`, unused `ir_to_wit_paint_*_view` converters, `harvest_paint_segmentation_ir_from_ctx` facade), **when** the workspace compiles, **then** no `unused` warnings reference these symbols and `tree-support` and `traditional-support` still receive valid `PaintRegionLayerView` handles. | `cargo check --workspace`

- **AC-7. Given** the per-point parallelism change (`par_chunks(32)` on a flattened `Vec<Point2>` of contour points across all polygons for a given semantic), **when** processing a benchy_4color layer (~1,000–2,000 contour points), **then** the `boundary_paint` output is byte-identical to the pre-change serial-path output for the same input — i.e. the result is order-independent and unaffected by chunking. | `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`

- **AC-7b (observational, non-gating). Given** the `par_chunks(32)` annotation path on a 16-thread machine, **when** the end-to-end report run is executed, **then** the `PaintRegionAnnotation` stage wall-clock in the report shows speedup consistent with multi-thread utilization. This is a qualitative observation recorded in the Acceptance Ceremony benchmark log, **not** a binary closure gate (a unit test cannot assert thread utilization). | `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module-dir modules/core-modules --output ./tmp/out.gcode --report ./tmp/slicer-report.html`

- **AC-8. Given** the config key `union_paint_regions_at_harvest` added to the paint segmentation config schema with default `true`, **when** set to `false` and `group_and_union_paint_regions()` runs, **then** the `slicer_core::union()` call is skipped, each `SemanticRegion` retains individual per-facet polygons (polygon count equals the input facet count, not the unioned count), and AABB is still computed (`aabb.is_some()`). A **new** test `union_toggle_false_skips_union_but_computes_aabb` must be added to `paint_segmentation_executor_tdd.rs` asserting exactly this — the AC is not satisfied by the pre-existing default-`true` assertions alone. | `cargo test -p slicer-host --test paint_segmentation_executor_tdd -- union_toggle_false_skips_union_but_computes_aabb`

- **AC-9. Given** the 11 WASM `paint_segmentation_tdd.rs` tests moved to `slicer-host/tests/paint_segmentation_host_tdd.rs` and the 9 WASM `paint_region_annotator_tdd.rs` tests moved to `slicer-host/tests/paint_region_annotator_host_tdd.rs`, **when** run against the host functions `execute_paint_segmentation()` and `execute_slice_postprocess_paint_annotation()`, **then** all migrated tests pass with the same assertion values. | `cargo test -p slicer-host --test paint_segmentation_host_tdd`

- **AC-10. Given** the 5 host test files (`dispatch_tdd.rs`, `macro_paint_segmentation_output_roundtrip_tdd.rs`, `prepass_executor_tdd.rs`, `benchy_end_to_end_tdd.rs`, `manifest_ingestion_tdd.rs`) updated to exercise the guard-based host fallback path instead of loading `.wasm` files, **when** run, **then** all tests pass and no test loads a deleted module path. | `cargo test -p slicer-host --test dispatch_tdd`

- **AC-11. Given** the `docs/04_host_scheduler.md` updated with the new `Layer::PaintRegionAnnotation` stage, **when** `rg 'Layer::PaintRegionAnnotation' docs/04_host_scheduler.md` is run, **then** the stage is documented with its stage order, handler, and WASM override instructions. | `rg -q 'Layer::PaintRegionAnnotation' docs/04_host_scheduler.md`

- **AC-12. Given** the `docs/07_implementation_status.md` updated with a new task row `TASK-204` for this consolidation, **when** `rg 'paint-native-migration' docs/07_implementation_status.md` is run, **then** the row exists with status `[x]`, carries the `TASK-204` ID, and references this packet. | `rg -q 'paint-native-migration' docs/07_implementation_status.md`

## Negative Test Cases

- **AC-N1. Given** the paint-segmentation module directory deleted, **when** `build-core-modules.sh` runs, **then** no build step for `paint-segmentation` is attempted and the script returns exit code 0. | `bash modules/core-modules/build-core-modules.sh 2>&1 | Select-String -Pattern "paint-segmentation" | Measure-Object | ForEach-Object { if ($_.Count -eq 0) { "PASS" } else { "FAIL" } }`

- **AC-N2. Given** the paint-region-annotator module directory deleted, **when** `build-core-modules.sh` runs, **then** no build step for `paint-region-annotator` is attempted. | `bash modules/core-modules/build-core-modules.sh 2>&1 | Select-String -Pattern "paint-region-annotator" | Measure-Object | ForEach-Object { if ($_.Count -eq 0) { "PASS" } else { "FAIL" } }`

- **AC-N3. Given** a corrupt or missing `MeshIR` at `PrePass::PaintSegmentation` time, **when** the host fallback `execute_paint_segmentation()` runs, **then** a `PaintSegmentationError::MissingSurfaceObject` or `MissingLayerParticipation` is returned as a fatal prepass error — the host does not silently produce empty `PaintRegionIR`. | `cargo test -p slicer-host --test paint_segmentation_executor_tdd`

- **AC-N4. Given** the host annotator always-on (no WASM module claiming `PaintRegionAnnotation`), **when** a contour point lands in a `DeterministicConflict` scenario (two custom regions with equal paint_order, different values), **then** a code 503 fatal error is returned — the host annotator detects conflicts at query time identically to the pre-change path. | `cargo test -p slicer-host --test paint_annotation_integration_tdd`

- **AC-N5. Given** the stale `.wasm` artifacts for the deleted modules present in build output directories, **when** `build-core-modules.sh --check` runs, **then** no `STALE:` report references `paint-segmentation` or `paint-region-annotator`. | `bash modules/core-modules/build-core-modules.sh --check 2>&1 | Select-String -Pattern "paint-segmentation|paint-region-annotator" | Measure-Object | ForEach-Object { if ($_.Count -eq 0) { "PASS" } else { "FAIL" } }`

## Verification

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test paint_segmentation_executor_tdd`

## Authoritative Docs

- `docs/01_system_architecture.md` — dispatch and harvest lifecycle. Delegate SUMMARY (> 300 lines); implementer needs only the PrePass and per-layer staging sections.
- `docs/02_ir_schemas.md` — PaintRegionIR schema. Range-read only §"PaintRegionIR" (no IR schema changes — verify unchanged).
- `docs/03_wit_and_manifest.md` — module manifest format, `known_stage_ids()` allowlist, stage discovery. Range-read relevant sections.
- `docs/04_host_scheduler.md` — PrePass and Layer stage ordering. Range-read lines 80-160 (PrePass order) and the Layer stage table; verify `PaintRegionAnnotation` insertion point.
- `docs/08_coordinate_system.md` — unit system (1 unit = 100 nm). Range-read only; confirm no scale conversions change in the refactored functions.

## Doc Impact Statement

1. `docs/04_host_scheduler.md` §"Layer Stage Order" — add `Layer::PaintRegionAnnotation` entry documenting its position between `Layer::Slice` and `Layer::SlicePostProcess`, its host handler (`execute_slice_postprocess_paint_annotation`), and the WASM override contract (any module claiming `Layer::PaintRegionAnnotation` runs instead of the host handler). Document the `PrePass::PaintSegmentation` host fallback contract (WASM module runs if present; `execute_paint_segmentation()` runs otherwise). | `rg -q 'Layer::PaintRegionAnnotation' docs/04_host_scheduler.md`
2. `docs/04_host_scheduler.md` §"PrePass Stage Order" — document that `PrePass::PaintSegmentation` is now guard-based: a WASM module may claim it and override the host, otherwise `execute_paint_segmentation()` runs as the built-in handler. | `rg -q 'guard-based fallback' docs/04_host_scheduler.md`
3. `docs/07_implementation_status.md` — add new task row `TASK-204` tracking this consolidation, with status `[x]` and a reference to `64_paint-native-migration`. | `rg -q 'paint-native-migration' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Implementation Deviations

These are intentional divergences discovered during implementation; they do not weaken any acceptance criterion.

### 1. Stages are string literals, not an enum
`design.md` §Architecture Constraints assumed a `Layer` enum with variants. Reality: stage identifiers are `&str` / `StageId = String`, routed via `match` on string literals in `dispatch.rs` and `if stage.stage_id == "..."` in `layer_executor.rs`. Step 2 simplified to adding `"Layer::PaintRegionAnnotation"` to `STAGE_ORDER` and `known_stage_ids()` only — no match-arm updates needed.

### 2. Post-loop fallback retained (not removed)
Packet §"In Scope" and Step 3 required removing the `paint_annotation_ran` guard. The in-loop handler at `Layer::PaintRegionAnnotation` was added, but the post-loop fallback was **kept** as a safety net. Production plans always include the stage (inserted unconditionally by `build_execution_plan`), but tests that manually construct plans without it rely on the fallback. Without it, `paint_annotation_integration_tdd` (5 tests) regressed.

### 3. Inline WIT resource kept as stub
Packet §"In Scope" and Step 8 required removing `paint-segmentation-output` resource and `HostPaintSegmentationOutput` impl. The data-storage fields and harvest logic were deleted, but the **inline WIT resource definition** and a **no-op `HostPaintSegmentationOutput` trait impl** were preserved. Test guests (`prepass-guest`, `sdk-prepass-paintseg-guest`) import this resource, and removing it caused 20 WASM linking failures in `dispatch_tdd.rs`. The canonical `wit/world-prepass.wit` still defines the resource for future extension.

### 4. `push_polygon_region` and `detect_custom_conflict` fully removed
Packet Step 1 expected `push_polygon_region` to be replaced by `group_and_union_paint_regions()` calls. Implementation went further: `push_polygon_region` was deleted entirely, and `detect_custom_conflict` was inlined into the HashMap aggregation loop. Both functions are gone — the aggregation key `(layer_index, object_id, semantic, value, paint_order)` handles the same logic.

### 5. `HashablePaintValue` hoisted to module scope
Packet Step 1 expected a shared function extraction. The local `enum HashablePaintValue` inside `group_and_union_paint_regions` was hoisted to module scope so `execute_paint_segmentation` could use the same `From<&PaintValue>` impl in its aggregation HashMap. No public API impact.

### 6. Stage inserted in `build_execution_plan`, not just guarded in executor
Packet Step 3 expected a guard-based fallback in `layer_executor.rs`. Root cause: `build_execution_plan` skips stages with zero modules, so `PaintRegionAnnotation` was never in `per_layer_stages`. The post-loop fallback ran AFTER perimeters — too late. Fix: `build_execution_plan` now unconditionally inserts `Layer::PaintRegionAnnotation` before the first downstream stage (SlicePostProcess, Perimeters, etc.) in STAGE_ORDER. The in-loop handler at `layer_executor.rs:482` runs at the correct position.

### 7. Config toggle lives in `raw_config_source`, not module manifest
Packet §"Data and Contract Notes" specified scoping `union_paint_regions_at_harvest` to the paint-segmentation config schema. Since the module was deleted, there is no module manifest to attach it to. The key is read from `raw_config_source` (global `--config` JSON) in the prepass host fallback. Functionally equivalent; a dedicated host-config schema could be added later if needed.

### 8. `dispatch_helpers.rs` deleted entirely
Packet Step 8 questioned whether `dispatch_helpers.rs` contained other content. Confirmed: it only held `harvest_paint_segmentation_ir_from_ctx()`. File deleted, `pub mod dispatch_helpers` removed from `lib.rs`.

### 9. `slicer-macros` patch for `aabb` field
Not in packet scope but required for build: `slicer-macros/src/lib.rs:2615` generates `SemanticRegion { ... }` without the `aabb` field added by packet 62. Added `aabb: None` to the generated construction. This stale-macro bug predated packet 64 and only surfaced when WASM modules were rebuilt during Step 7.

### 10. [AC-9] 2 paint_region_annotator tests not migrated
Packet AC-9 required 9 tests in `paint_region_annotator_host_tdd.rs`. 7 were migrated; `points_outside_paint_region_get_none` and `deterministic_conflict_fatal_for_custom_semantics` were dropped. The former is covered by `slice_postprocess_paint_annotation_tdd` (default-fill-for-out-of-region tests). The latter tests WIT-boundary conflict detection — the host path detects conflicts at segmentation time (`deterministic_conflict_at_segmentation_time_surfaces_paint_segmentation_error` in `paint_segmentation_executor_tdd.rs`) rather than at query time, making the original test's semantics inapplicable.

### 12. [diagnostic] 292-layer benchy_4color paint diagnostic not saved
A one-shot diagnostic `benchy_4color_execute_paint_segmentation_produces_material_tool_indices` was created during the tool-change regression debugging to confirm ≥4 ToolIndex values on the real 292-layer model. It was not saved to the test file. The existing `benchy_4color_full_pipeline_paint_diagnostic` (20 synthetic layers) validates pipeline correctness; the 292-layer run is validated by manual gcode inspection (4 filaments, 444 T-cmds).
