# Requirements: 64_paint-native-migration

## Packet Metadata

- Grouped task IDs:
  - `TASK-136` (open — E2E progress-event coverage for paint-annotation failure codes 501-504; tangentially relevant)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The ModularSlicer pipeline runs two WASM modules — `paint-segmentation` (PrePass) and `paint-region-annotator` (per-layer `SlicePostProcess`) — that duplicate host-native implementations already present in `paint_segmentation.rs` and `slice_postprocess.rs`. The guest `paint-region-annotator` consumes 1,370,992 CPU-ms across threads on a benchy_4color run, performing point-in-region containment checks that the host's `execute_slice_postprocess_paint_annotation` already computes natively. The guest `paint-segmentation` projects 3D facets to 2D via WIT serialization, while `execute_paint_segmentation()` contains a complete independent implementation never wired into the dispatch path.

Beyond duplication, the current architecture has a design defect: `Layer::SlicePostProcess` conflates general post-processing with paint-specific annotation. A WASM module claiming `SlicePostProcess` for a different purpose (e.g., polygon smoothing) has no interaction with paint annotation, but the stage name and fallback guard (`paint_annotation_ran`) make the relationship implicit and fragile.

The WASM boundary imposes serialization cost even after migration: `paint_region_ir_to_layer_data()` re-serializes `PaintRegionIR` (~60 KB per layer) for `tree-support` and `traditional-support` modules that query `PaintRegionLayerView`. Eliminating the two guest modules removes the dominant CPU cost (1.37M CPU-ms) while the support-module serialization path survives independently.

Packet 62 optimized the host annotation path (union, AABB, cache, early-break, `par_iter`) but these optimizations only apply when the host fallback runs — not when the WASM module is loaded. Packet 63 will add R-tree spatial indexing to the query path. Making the host path always-on ensures both packets' optimizations are always active.

This packet completes the consolidation: delete both WASM modules, wire the host implementations as guard-based fallbacks, add a dedicated `Layer::PaintRegionAnnotation` stage, apply per-point parallelism, and provide a config toggle to re-evaluate the union-at-harvest tradeoff.

## In Scope

- Delete `modules/core-modules/paint-segmentation/` (src, Cargo.toml, wit-guest, tests, manifest `.toml`)
- Delete `modules/core-modules/paint-region-annotator/` (src, Cargo.toml, wit-guest, tests, manifest `.toml`)
- Remove both modules from `modules/core-modules/build-core-modules.sh` build list
- Clean stale `.wasm` artifacts for both modules from build output directories
- Extract shared `group_and_union_paint_regions()` free function from `harvest_paint_segmentation_ir()` into `paint_segmentation.rs`
- Update `execute_paint_segmentation()` to call the shared function — bring it to feature parity with the current WASM+harvest path (union polygons, compute AABB, sort descending by paint_order)
- Add `Layer::PaintRegionAnnotation` variant to the `Layer` stage enum, insert before `Layer::SlicePostProcess` in `STAGE_ORDER`, add to `known_stage_ids()` in `manifest.rs`
- Wire `execute_paint_segmentation()` as a guard-based host fallback for `PrePass::PaintSegmentation` (runs only if no WASM module claims the stage)
- Wire `execute_slice_postprocess_paint_annotation()` as a guard-based host fallback for `Layer::PaintRegionAnnotation` (runs only if no WASM module claims the stage)
- Remove `paint_annotation_ran` flag and guard — the host annotator is now a dedicated stage handler, not a post-loop fallback
- Remove dead WIT code: `paint_region_entries` field on `HostExecutionContext`, `HostPaintSegmentationOutput` trait impl, `harvest_paint_segmentation_ir()`, `object_mesh_to_wit_paint_segmentation_view()`, `push_paint_segmentation_output()`, unused `ir_to_wit_paint_*_view` converters, WIT `paint-region-entry` record, `paint-segmentation-output` resource definition, `harvest_paint_segmentation_ir_from_ctx()` facade in `dispatch_helpers.rs`
- Move `paint_region_annotator_tdd.rs` (9 tests) from `modules/core-modules/paint-region-annotator/tests/` to `crates/slicer-host/tests/paint_region_annotator_host_tdd.rs`
- Move `paint_segmentation_tdd.rs` (11 tests) from `modules/core-modules/paint-segmentation/tests/` to `crates/slicer-host/tests/paint_segmentation_host_tdd.rs`, port from WASM module calls to `execute_paint_segmentation()` calls
- Rewrite 5 host test files that load `.wasm` files to exercise the guard-based host fallback path instead: `dispatch_tdd.rs`, `macro_paint_segmentation_output_roundtrip_tdd.rs`, `prepass_executor_tdd.rs`, `benchy_end_to_end_tdd.rs`, `manifest_ingestion_tdd.rs`
- Apply per-point parallelism: flatten contour points across all polygons for a given semantic, then `par_chunks(32)` in `slice_postprocess.rs`
- Add `union_paint_regions_at_harvest` config key (default `true`) to `group_and_union_paint_regions()`, documented in `paint_segmentation.rs` module config schema
- Update `docs/04_host_scheduler.md` — document new `Layer::PaintRegionAnnotation` stage, guard-based fallback contracts for both `PrePass::PaintSegmentation` and `Layer::PaintRegionAnnotation`, and WASM override instructions
- Update `docs/07_implementation_status.md` — add task row for this consolidation
- Preserve two test-guests (`test-guests/prepass-guest/`, `test-guests/sdk-prepass-paintseg-guest/`) unchanged — they validate the WIT contract stays intact

## Out of Scope

- Deleting or modifying the WIT world definition (`slicer:world-prepass@1.0.0`) — the WIT contract stays for future extension
- Modifying `tree-support` or `traditional-support` to use `PaintRegionIR` directly instead of `PaintRegionLayerView`
- Modifying `classic-perimeters` or `arachne-perimeters` (they accept but don't use `PaintRegionLayerView`)
- Adding a dedicated `cargo bench` target for paint annotation
- Modifying any `OrcaSlicerDocumented/` reference — no OrcaSlicer parity is involved (pure host architecture consolidation)
- Changing the `code:504` ambiguity warning policy, message text, or `EPSILON_UNITS = 1` constant
- Adding `Layer::PaintRegionAnnotation` to the `slicer-ir` stage enum if the stage enum lives outside `layer_executor.rs` — delegate location check to implementer
- Any changes to packet 62 or packet 63 code — this packet builds on them, does not modify them

## Authoritative Docs

- `docs/01_system_architecture.md` — dispatch and harvest lifecycle. Delegate SUMMARY (> 300 lines); implementer needs only the PrePass stage dispatch section and per-layer stage dispatch section.
- `docs/02_ir_schemas.md` — PaintRegionIR schema. Range-read only §"PaintRegionIR"; confirm no IR field changes are needed (the IR types stay).
- `docs/03_wit_and_manifest.md` — module manifest format, `known_stage_ids()` allowlist, `ingest_manifest()` validation, stage discovery via directory scanning. Range-read the manifest schema and ingestion sections.
- `docs/04_host_scheduler.md` — PrePass and Layer stage ordering. Range-read lines 80-160 (PrePass stage order table) and the Layer stage ordering section; verify insertion point for `Layer::PaintRegionAnnotation`.
- `docs/08_coordinate_system.md` — unit system (1 unit = 100 nm). Range-read only; the shared `group_and_union_paint_regions()` uses `Point2` in native 100 nm units — no scale conversion changes needed.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-12` from `packet.spec.md`
  - AC-1: Pipeline runs without missing-module errors after directory deletion
  - AC-2: `Layer::PaintRegionAnnotation` produces identical `boundary_paint` output
  - AC-3: WASM module can still override `PrePass::PaintSegmentation` (guard preserved)
  - AC-4: WASM module can still override `Layer::PaintRegionAnnotation` (guard preserved)
  - AC-5: Shared `group_and_union_paint_regions()` produces byte-identical output to pre-change harvest
  - AC-6: Dead WIT code removed, support modules still receive `PaintRegionLayerView`
  - AC-7: Per-point `par_chunks(32)` saturates threads, output matches serial path
  - AC-8: `union_paint_regions_at_harvest: false` skips union, still computes AABB
  - AC-9: Migrated WASM tests pass against host functions
  - AC-10: Host test files updated, no `.wasm` loading of deleted modules
  - AC-11: `docs/04_host_scheduler.md` documents new stage and WASM override instructions
  - AC-12: `docs/07_implementation_status.md` has new task row
- Negative cases: `AC-N1` through `AC-N5` from `packet.spec.md`
  - AC-N1: Build script does not attempt paint-segmentation build
  - AC-N2: Build script does not attempt paint-region-annotator build
  - AC-N3: Host `execute_paint_segmentation()` returns errors on corrupt input
  - AC-N4: Code 503 fatal error on `DeterministicConflict` preserved
  - AC-N5: `--check` does not report stale `.wasm` for deleted modules
- Cross-packet impact: unblocks union re-evaluation (config toggle), feeds `boundary_paint` to future `SlicePostProcess` modules, preserves WIT extension surface for both stages

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-host --test paint_segmentation_executor_tdd` | AC-5: shared function parity; AC-N3: error handling | FACT pass/fail |
| `cargo test -p slicer-host --test paint_segmentation_host_tdd` | AC-9: migrated WASM tests pass | FACT pass/fail |
| `cargo test -p slicer-host --test paint_region_annotator_host_tdd` | AC-9: migrated annotator tests pass | FACT pass/fail |
| `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd` | AC-2: identical output; AC-7: per-point parallelism | FACT pass/fail |
| `cargo test -p slicer-host --test dispatch_tdd` | AC-10: dispatch tests updated | FACT pass/fail |
| `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd` | AC-10: roundtrip tests updated | FACT pass/fail |
| `cargo test -p slicer-host --test prepass_executor_tdd` | AC-3: WASM override preserved | FACT pass/fail |
| `cargo test -p slicer-host --test benchy_end_to_end_tdd` | AC-10: e2e tests updated | FACT pass/fail |
| `cargo test -p slicer-host --test manifest_ingestion_tdd` | AC-10: manifest tests updated | FACT pass/fail |
| `cargo test -p slicer-host --test paint_annotation_integration_tdd` | AC-N4: code 503 fatal on conflict | FACT pass/fail |
| `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` | AC-1: pipeline health after deletion | FACT pass/fail |
| `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd` | AC-6: support module path intact | FACT pass/fail |
| `cargo test -p slicer-host --test paint_region_transport_widening_tdd` | AC-5: transport contract unchanged | FACT pass/fail |
| `cargo check --workspace` | AC-6: no unused symbol warnings; compile gate | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |
| `bash modules/core-modules/build-core-modules.sh` | AC-N1, AC-N2: no build attempt for deleted modules | FACT pass/fail |
| `bash modules/core-modules/build-core-modules.sh --check` | AC-N5: no stale `.wasm` report | FACT pass/fail |
| `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html` | AC-1: pipeline runs without module errors; benchmark opportunity | FACT: pass/fail + report metadata timestamp |

## Step Completion Expectations

- Cross-step invariant: after Step 1 (shared function extraction), `cargo test -p slicer-host --test paint_segmentation_executor_tdd` must pass unchanged — the shared function produces identical output to the pre-extraction path.
- Cross-step invariant: after Step 2 (new stage addition), `cargo check --workspace` must pass before any stage handler is wired — the new `Layer` variant must compile in all match arms.
- Cross-step invariant: after Step 8 (dead WIT code removal), `tree-support` and `traditional-support` must still build and their tests must pass — `PaintRegionLayerView` serialization survives.
- Cross-step invariant: after Step 10 (config toggle), `union_paint_regions_at_harvest: false` must produce regions with un-unioned polygons but with computed AABB — AABB is always computed, union is optional.
- Step ordering rationale:
  - Step 1 (shared function) must precede Step 4 (host fallback wiring) because the host fallback calls the shared function.
  - Step 2 (new stage variant) must precede Step 3 (annotator handler move) because the handler targets the new stage.
  - Step 5 (test migration) must precede Step 6 (host test rewrite) because the migrated tests exercise the host functions that the host tests reference.
  - Step 7 (module deletion) must precede Step 8 (dead code removal) because the dead code depends on the modules' WIT types.
  - Step 9 (per-point parallelism) must be the last code change — it builds on the fully wired stage handler.
  - Step 10 (config toggle) and Step 11 (docs) can run in parallel with Step 9.
- Cross-step shared scratch: `group_and_union_paint_regions()` extracted in Step 1 is consumed by Step 4 (host fallback wiring) and Step 10 (config toggle). The `Layer::PaintRegionAnnotation` variant added in Step 2 is consumed by Step 3 (handler move) and Step 4 (guard wiring).

## Context Discipline Notes

- Large files in the read-only path:
  - `crates/slicer-host/src/dispatch.rs` (~2200 lines) — range-read the PrePass dispatch block (lines 950-990), `harvest_paint_segmentation_ir` body (lines 2003-2172), per-layer paint dispatch block (lines 630-670), and dead-code sections only.
  - `crates/slicer-host/src/wit_host.rs` (~4500 lines) — range-read `paint_region_entries` field (line 1461), `push_paint_segmentation_output` (line 1985), `HostPaintSegmentationOutput` impl (lines 4383-4425), `paint_region_ir_to_layer_data` (lines 2644-2693), and `PaintRegionLayerData` (lines 216-228). Do not load the full file.
  - `crates/slicer-host/src/layer_executor.rs` (~800 lines) — range-read the stage dispatch loop (lines 250-500) and the paint annotation fallback guard (lines 469-484).
  - `docs/01_system_architecture.md` — delegate SUMMARY; implementer reads only the dispatch lifecycle sections referenced in `design.md`.
- Likely temptation reads:
  - `crates/slicer-host/src/paint_segmentation.rs` (beyond `execute_paint_segmentation` body lines 51-207) — the modifier volume post-processing and helper functions are read once, not re-read during dead-code removal.
  - `OrcaSlicerDocumented/` — no OrcaSlicer parity is involved. Do not load.
  - Full `Cargo.toml` of slicer-core or slicer-host — no new external dependencies are added. Delegate dependency checks if needed.
- Sub-agent return-format hints:
  - All `cargo test` dispatches return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines).
  - The module deletion verification (`build-core-modules.sh`) returns FACT (pass/fail with error line).
  - The end-to-end `slicer-host --report` run returns FACT (annotator module timing row + report timestamp).
  - Find-caller dispatches return LOCATIONS (file:line + one-line context, ≤ 20 entries).
