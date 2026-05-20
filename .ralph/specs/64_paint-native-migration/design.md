# Design: 64_paint-native-migration

## Controlling Code Paths

- Primary code paths:
  - **PrePass**: `dispatch.rs` → `run_stage()` dispatches `PrePass::PaintSegmentation`. If a WASM module claims the stage, it runs via `dispatch_prepass_call()`. Otherwise, the guard-based host fallback calls `execute_paint_segmentation(Arc<MeshIR>, Arc<SurfaceClassificationIR>, Arc<LayerPlanIR>) → Result<Arc<PaintRegionIR>, PaintSegmentationError>`. The shared `group_and_union_paint_regions()` is called inside `execute_paint_segmentation()` to union polygons and compute AABB.
  - **Per-Layer**: `layer_executor.rs` dispatches `Layer::PaintRegionAnnotation` (new stage, before `SlicePostProcess`). If a WASM module claims the stage, it runs. Otherwise, the guard-based host fallback calls `execute_slice_postprocess_paint_annotation()`. The `paint_annotation_ran` flag is removed — the stage handler is always available.
  - **Per-Layer (support)**: `tree-support` and `traditional-support` continue to receive `PaintRegionLayerView` handles built by `build_paint_layer_data()` → `paint_region_ir_to_layer_data()` → `push_paint_region_layer_view()`. This path survives untouched.

- Neighboring tests or fixtures:
  - `paint_segmentation_executor_tdd.rs` — 5 host executor tests (already testing `execute_paint_segmentation()`); stay as-is
  - `paint_segmentation_host_tdd.rs` — 11 migrated WASM tests (new file, moved from `modules/core-modules/paint-segmentation/tests/`)
  - `paint_region_annotator_host_tdd.rs` — 9 migrated WASM tests (new file, moved from `modules/core-modules/paint-region-annotator/tests/`)
  - `slice_postprocess_paint_annotation_tdd.rs` — 12 annotation loop tests; stay, updated for per-point parallelism
  - `dispatch_tdd.rs` — 8 paint-segmentation WASM loading tests; rewritten to exercise guard-based host fallback
  - `macro_paint_segmentation_output_roundtrip_tdd.rs` — loads `.wasm`; rewritten to test host path
  - `prepass_executor_tdd.rs` — references `com.example.paint-segmentation` (test-only module ID); adapted for guard-based fallback
  - `benchy_end_to_end_tdd.rs` — references both modules; module-path references removed, fallback path exercised
  - `manifest_ingestion_tdd.rs` — references `com.core.paint-segmentation` and `com.core.paint-region-annotator`; module ID references removed from manifest test data
  - `paint_annotation_integration_tdd.rs` — warning/error paths; stay as-is
  - `region_mapping_paint_semantic_tdd.rs` — empty-polygon fixtures; stay as-is
  - `paint_region_transport_widening_tdd.rs` — hole fidelity after union changes; stay as-is
  - `benchy_4color_modifier_part_e2e_tdd.rs` — e2e health check; stay as-is

- OrcaSlicer comparison surface: none. This is a pure host architecture consolidation — no algorithm porting, no geometry changes. See `requirements.md` §OrcaSlicer Reference Obligations (absent — section skipped).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `Layer` stage enum lives in `crates/slicer-ir/src/slice_ir.rs` (or equivalent executor enum). Adding `Layer::PaintRegionAnnotation` requires updating all match arms across the workspace. The implementer must delegate a `cargo check` after adding the variant to discover all affected match arms.

- `execute_paint_segmentation()` currently uses `push_polygon_region()` which appends per-facet polygons without unioning and sets `aabb: None`. The shared `group_and_union_paint_regions()` replaces `push_polygon_region()` — it groups by `(layer_index, object_id, semantic, value)`, unions polygons via `slicer_core::union()`, computes AABB from unioned contour points, and sorts descending by `paint_order`. This must produce byte-identical output to the current `harvest_paint_segmentation_ir()` post-processing.

- `execute_paint_segmentation()` validates `MissingSurfaceObject` and `MissingLayerParticipation` (errors not present in the WASM guest). These validations are **kept** — they are correctness improvements that fail fast rather than producing silently wrong output. The WASM guest would fail later with a worse error. These errors must be mapped to the dispatch error type in the guard-based fallback.

- `execute_paint_segmentation()` detects `DeterministicConflict` at segmentation time via `detect_custom_conflict()` with polygon-overlap checks. The WASM guest does not — conflicts surface at query time in `point_in_paint_region`. The conflict detection is **kept** — it is a correctness improvement (fail-fast at segmentation rather than failing per-layer at query time). This is a behavioral change: overlapping custom regions with equal `paint_order` now fail during the prepass rather than during per-layer annotation. The `point_in_paint_region` conflict path is still preserved as a defense-in-depth check.

- `execute_slice_postprocess_paint_annotation()` has richer behavior than the WASM `paint-region-annotator` guest: edge-ambiguity detection (code 504 warnings), default value filling for out-of-region points, and FuzzySkin modifier support. The richer behavior is **kept** — it is strictly better (fewer `None` values, better diagnostics). The WASM guest's minimal behavior (leave `None`, no warnings) was a gap, not a designed contract.

- The `union_paint_regions_at_harvest` config key is a `bool` defaulting to `true`. When `false`, `group_and_union_paint_regions()` skips `slicer_core::union()` but still computes AABB. The key is scoped to the paint-segmentation config schema, not a global config. It is documented as a temporary benchmarking toggle; the user may remove it after data confirms the right default.

## Code Change Surface

- Selected approach: Extract shared grouping+union+AABB+sort function, wire guard-based host fallbacks for both stages, delete WASM modules and dead WIT code, add dedicated `Layer::PaintRegionAnnotation` stage, apply per-point parallelism, add config toggle for union.

- Exact functions, traits, manifests, tests, or fixtures expected to change:

  1. `crates/slicer-host/src/paint_segmentation.rs` — add `group_and_union_paint_regions()` shared free function (signature: `fn group_and_union_paint_regions(entries: Vec<PaintFacetEntry>) -> PaintRegionIR`). Replace `push_polygon_region()` calls in `execute_paint_segmentation()` with a call to the shared function. Add `union_paint_regions_at_harvest` config parameter.
  2. `crates/slicer-ir/src/slice_ir.rs` (or stage enum location) — add `Layer::PaintRegionAnnotation` variant.
  3. `crates/slicer-host/src/execution_plan.rs` — insert `Layer::PaintRegionAnnotation` before `Layer::SlicePostProcess` in `STAGE_ORDER`.
  4. `crates/slicer-host/src/manifest.rs` — add `"Layer::PaintRegionAnnotation"` to `known_stage_ids()`.
  5. `crates/slicer-host/src/layer_executor.rs` — add `Layer::PaintRegionAnnotation` stage handler that calls `execute_slice_postprocess_paint_annotation()` when no WASM module claims the stage. Remove the post-loop `paint_annotation_ran` guard and host annotator call from the `SlicePostProcess` section.
  6. `crates/slicer-host/src/dispatch.rs` — add guard-based host fallback for `PrePass::PaintSegmentation` (call `execute_paint_segmentation()` if no module handled the stage). Remove `harvest_paint_segmentation_ir()` body. Remove per-layer paint annotation dispatch arm (moved to `Layer::PaintRegionAnnotation`).
  7. `crates/slicer-host/src/prepass.rs` — add host handler for `PrePass::PaintSegmentation` that reads `blackboard.mesh()`, `blackboard.surface_classification()`, `blackboard.layer_plan()`, calls `execute_paint_segmentation()`, and commits via `blackboard.commit_paint_regions()`.
  8. `crates/slicer-host/src/slice_postprocess.rs` — flatten contour points across polygons per semantic, apply `par_chunks(32)`, merge thread-local results.
  9. `crates/slicer-host/src/wit_host.rs` — remove `paint_region_entries` field, `push_paint_segmentation_output()`, `paint_region_entries()` getter, `HostPaintSegmentationOutput` impl, `object_mesh_to_wit_paint_segmentation_view()`, `ir_to_wit_paint_stroke_view()`, `ir_to_wit_paint_layer_view()`, `PaintSegmentationOutputData` struct, WIT records `paint-region-entry` and `paint-segmentation-output`. **Keep** `PaintRegionLayerData`, `paint_region_ir_to_layer_data()`, `paint_semantic_key()`, `paint_semantic_to_string()`, `ir_to_wit_paint_value_view()`, `push_paint_region_layer_view()`, and the `HostPaintRegionLayerView` trait impl.
  10. `crates/slicer-host/src/dispatch_helpers.rs` — remove `harvest_paint_segmentation_ir_from_ctx()` facade (if it is the only content, delete the file).
  11. `modules/core-modules/build-core-modules.sh` — remove `paint-segmentation` and `paint-region-annotator` from the module build list.
  12. `modules/core-modules/paint-segmentation/` — delete entire directory.
  13. `modules/core-modules/paint-region-annotator/` — delete entire directory.
  14. `crates/slicer-host/tests/paint_segmentation_host_tdd.rs` — new file, migrated 11 tests from `modules/core-modules/paint-segmentation/tests/paint_segmentation_tdd.rs`, ported to call `execute_paint_segmentation()`.
  15. `crates/slicer-host/tests/paint_region_annotator_host_tdd.rs` — new file, migrated 9 tests from `modules/core-modules/paint-region-annotator/tests/paint_region_annotator_tdd.rs`.
  16. `crates/slicer-host/tests/dispatch_tdd.rs` — rewrite 8 `.wasm`-loading tests to exercise guard-based host fallback.
  17. `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` — rewrite to test host path.
  18. `crates/slicer-host/tests/prepass_executor_tdd.rs` — adapt `com.example.paint-segmentation` references for guard-based fallback.
  19. `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — remove module-path references.
  20. `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — remove `com.core.paint-segmentation` and `com.core.paint-region-annotator` references from manifest test data.
  21. `docs/04_host_scheduler.md` — add `Layer::PaintRegionAnnotation` stage documentation with guard-based fallback contract.
  22. `docs/07_implementation_status.md` — add task row for this consolidation.

- Rejected alternatives:
  - **Delete modules but keep paint annotation in `SlicePostProcess` without a dedicated stage**: Rejected — the `SlicePostProcess` stage conflates general post-processing with paint annotation. A dedicated stage makes the pipeline self-documenting and allows future `SlicePostProcess` modules to consume pre-computed `boundary_paint` without the `paint_annotation_ran` flag fragility.
  - **Stub `execute_paint_segmentation()` to match WASM guest's minimal behavior** (remove validation errors, remove conflict detection): Rejected — the validations are correctness improvements. Failing fast with `MissingSurfaceObject` is better than producing silently wrong `PaintRegionIR`. Detecting conflicts at segmentation time is better than failing per-layer at query time.
  - **Delete the WIT world `run-paint-segmentation` export**: Rejected — the WIT contract is an extension surface, not an implementation detail. Future module authors should be able to implement `PrePass::PaintSegmentation` in WASM. The host fallback preserves this.
  - **Make `execute_paint_segmentation()` call `harvest_paint_segmentation_ir()` by fabricating a `HostExecutionContext`**: Rejected — fragile. Extract a clean shared function instead.

## Files in Scope (read + edit)

- `crates/slicer-host/src/paint_segmentation.rs` — role: host paint segmentation implementation; expected change: add `group_and_union_paint_regions()`, update `execute_paint_segmentation()` to call it, add config toggle
- `crates/slicer-host/src/dispatch.rs` — role: stage dispatch; expected change: add PrePass guard fallback, remove harvest and per-layer paint annotation dispatch, remove dead WIT calls
- `crates/slicer-host/src/layer_executor.rs` — role: per-layer stage executor; expected change: add `Layer::PaintRegionAnnotation` handler, remove `paint_annotation_ran` guard
- `crates/slicer-host/src/slice_postprocess.rs` — role: host paint annotation; expected change: per-point `par_chunks(32)` with flatten
- `crates/slicer-host/src/wit_host.rs` — role: WIT type bindings; expected change: remove dead paint-segmentation WIT code, keep support-module path
- `crates/slicer-host/src/prepass.rs` — role: prepass stage handlers; expected change: add `PrePass::PaintSegmentation` host handler
- `crates/slicer-host/src/execution_plan.rs` — role: stage ordering; expected change: insert `Layer::PaintRegionAnnotation`
- `crates/slicer-host/src/manifest.rs` — role: module manifest ingestion; expected change: add `known_stage_ids()` entry
- `crates/slicer-ir/src/slice_ir.rs` — role: stage enum (verify location); expected change: add `Layer::PaintRegionAnnotation` variant
- `crates/slicer-host/src/dispatch_helpers.rs` — role: harvest facade; expected change: remove `harvest_paint_segmentation_ir_from_ctx()` (delete file if empty)
- `modules/core-modules/build-core-modules.sh` — role: WASM build script; expected change: remove two module build lines
- `crates/slicer-host/tests/paint_segmentation_host_tdd.rs` — role: new file; expected change: 11 migrated tests
- `crates/slicer-host/tests/paint_region_annotator_host_tdd.rs` — role: new file; expected change: 9 migrated tests
- `crates/slicer-host/tests/dispatch_tdd.rs` — role: dispatch tests; expected change: rewrite `.wasm`-loading tests
- `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` — role: roundtrip tests; expected change: rewrite to host path
- `crates/slicer-host/tests/prepass_executor_tdd.rs` — role: prepass executor tests; expected change: adapt for guard fallback
- `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — role: e2e tests; expected change: remove module-path references
- `crates/slicer-host/tests/manifest_ingestion_tdd.rs` — role: manifest ingestion tests; expected change: remove deleted module IDs
- `docs/04_host_scheduler.md` — role: scheduler docs; expected change: document new stage + guard contracts
- `docs/07_implementation_status.md` — role: backlog ledger; expected change: add task row

## Read-Only Context

- `crates/slicer-host/src/dispatch.rs` — read lines 950-990 (PrePass dispatch block), lines 2003-2172 (`harvest_paint_segmentation_ir` body), lines 630-670 (per-layer paint dispatch), lines 1510-1534 (`build_paint_layer_data`). Purpose: understand current dispatch flow before refactoring.
- `crates/slicer-host/src/layer_executor.rs` — read lines 250-500 (stage dispatch loop), lines 469-484 (paint annotation fallback guard). Purpose: understand current `SlicePostProcess` flow.
- `crates/slicer-host/src/paint_segmentation.rs` — read lines 51-207 (`execute_paint_segmentation` body), lines 209-230 (`push_polygon_region`), lines 232-264 (`detect_custom_conflict`), lines 266-285 (`project_facet`). Purpose: understand current host implementation before extracting shared function.
- `crates/slicer-core/src/polygon_ops.rs` — read lines 93-95 (`union` signature) and line 62 (hole-loss comment). Purpose: confirm `slicer_core::union` contract for the shared function.
- `docs/04_host_scheduler.md` — read lines 80-160 (PrePass stage order table), Layer stage ordering section. Purpose: verify insertion point for new stage.
- `modules/core-modules/paint-segmentation/src/lib.rs` — read full file (168 lines). Purpose: confirm feature parity requirements between WASM guest and shared function.
- `modules/core-modules/paint-region-annotator/src/lib.rs` — read full file (~200 lines). Purpose: confirm host annotator covers all guest behavior.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — no OrcaSlicer parity involved; never load
- `target/`, `Cargo.lock` — never load
- `test-guests/prepass-guest/src/`, `test-guests/sdk-prepass-paintseg-guest/src/` — preserved unchanged; delegate fact-checks if needed
- `crates/slicer-host/src/paint_segmentation.rs` (beyond lines 266) — helper functions like `transform_point`, `compare_semantic_regions`, geometry predicates are read once, not re-read
- `wit/` — WIT definitions are unchanged; never load
- `modules/core-modules/tree-support/`, `modules/core-modules/traditional-support/` — support modules survive; delegate fact-checks only

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)" — purpose: validate Step 1 shared function parity
- "Run `cargo check --workspace`; return FACT pass/fail" — purpose: validate Step 2 new stage variant compiles; validate after every subsequent step
- "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT or SNIPPETS" — purpose: validate Step 3 annotator handler move and Step 9 per-point parallelism
- "Run `cargo test -p slicer-host --test dispatch_tdd`; return FACT or SNIPPETS" — purpose: validate Step 6 dispatch test rewrite
- "Run `cargo test -p slicer-host --test paint_segmentation_host_tdd`; return FACT or SNIPPETS" — purpose: validate Step 5 migrated WASM tests
- "Run `cargo test -p slicer-host --test paint_region_annotator_host_tdd`; return FACT or SNIPPETS" — purpose: validate Step 5 migrated annotator tests
- "Run `bash modules/core-modules/build-core-modules.sh && bash modules/core-modules/build-core-modules.sh --check`; return FACT pass/fail" — purpose: validate Step 7 module deletion and AC-N5 stale check
- "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail" — purpose: lint gate after all steps
- "Run `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html`; return FACT (annotator stage wall time + report timestamp)" — purpose: validate AC-1 end-to-end
- "Find all callers of `harvest_paint_segmentation_ir` after deletion; return LOCATIONS" — purpose: confirm no orphan call sites in Step 8
- "Summarize `docs/01_system_architecture.md` §'dispatch lifecycle' for the PrePass and per-layer staging sections; return SUMMARY ≤ 200 words" — purpose: confirm dispatch contract before Step 4 wiring

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR` output shape unchanged. `SemanticRegion` retains its fields (`object_id`, `polygons`, `value`, `paint_order`, `aabb`). No schema version bump.
- WIT boundary considerations: `slicer:world-prepass@1.0.0` still defines `run-paint-segmentation`. The host no longer dispatches it, but the WIT contract stays for future extension. `PaintRegionLayerView` serialization (`paint_region_ir_to_layer_data()`) survives for support modules.
- Determinism: `group_and_union_paint_regions()` sorts groups by `(paint_order, object_id, value_key)` within each semantic Vec — identical to the current harvest sort order. Byte-deterministic output across runs is preserved (tested by AC-N3 in packet 62).
- Scheduler: new `Layer::PaintRegionAnnotation` stage inserted between `Layer::Slice` and `Layer::SlicePostProcess`. No DAG edge changes — it's a per-layer sequential stage. `PrePass::PaintSegmentation` order unchanged.
- Manifest: each deleted module's `.toml` manifest is deleted with the directory. Discovery via `discover_manifest_paths()` naturally skips them. No hardcoded module paths exist.
- Config: new `union_paint_regions_at_harvest` key added to paint segmentation config schema. Default `true`. No other config changes.

## Locked Assumptions and Invariants

- `slicer_core::union` discards holes. All guest-produced paint region entries carry `holes: vec![]` (triangles only). The shared function documents this at the call site. If a future module emits hole-bearing paint regions through the WASM override path, the host fallback's union path must switch to a hole-preserving variant.
- `paint_order` values from the shared function are `min(paint_order)` per group — identical to the current harvest behavior. Precedence (higher `paint_order` wins) is preserved.
- The AABB pre-filter in `semantic_region_contains_point` is an optional optimization — the shared function always computes it (even when `union_paint_regions_at_harvest: false`). Setting `aabb = None` at construction time would change query-path performance but not correctness.
- `rayon` is already a `slicer-host` dependency — no new `Cargo.toml` entry. `par_chunks(32)` uses the existing `use rayon::prelude::*` import.
- `PaintRegionIR` is `Arc`-wrapped and read-only — thread-safe for `par_chunks` parallel point queries.
- Group key `(layer_index, object_id, semantic, value)` is correct: same-value regions are query-equivalent and safe to merge; `object_id` preserves per-object boundaries; `paint_order` conflict logic only triggers between regions of different values.
- WIT `PaintRegionLayerView` serialization stays because `tree-support` and `traditional-support` query it per layer. Removing this path is a separate work item (these modules could be refactored to use `PaintRegionIR` directly).
- Test-guests `test-guests/prepass-guest/` and `test-guests/sdk-prepass-paintseg-guest/` stay unchanged — they validate the WIT contract, not the production module.

## Risks and Tradeoffs

- **Test churn**: 20 test files touched (2 migrated, 5 rewritten, 13 read-only verification). This is the largest single source of work in the packet. Each rewrite must preserve the original test's assertion strength.
- **WASM extension surface**: The guard-based fallback preserves the ability for future WASM modules to override both stages. If no module ever does, the guard is dead code — but it costs one `if wasm_ran { skip }` check per stage execution.
- **`execute_paint_segmentation()` validation errors**: `MissingSurfaceObject` and `MissingLayerParticipation` are new failure modes that didn't exist in the WASM guest path. If the upstream stages (`MeshAnalysis`, `LayerPlanning`) have bugs that the WASM guest silently tolerated, these errors could surface as pipeline failures after migration. Mitigated by the fact that the host path already validates these in tests (`paint_segmentation_executor_tdd.rs`).
- **Conflict detection at segmentation time vs query time**: `DetectCustomConflict` now fires during the prepass rather than during per-layer annotation. This is a behavioral change: the error surfaces earlier and is a prepass-level fatal, not a per-layer error. Downstream error-handling code that expected conflicts at query time may need updating. The `point_in_paint_region` conflict check is preserved as defense-in-depth.
- **Per-point parallelism determinism**: `par_chunks(32)` processes points non-deterministically. The `boundary_paint` output must be order-independent — each point's result depends only on its coordinates, not on other points' results. Verified by the existing `slice_postprocess_paint_annotation_tdd` tests.
- **Per-point parallelism overhead**: Rayon's per-task overhead for 32 containment checks (~32 × AABB check + 0-1 polygon containment) is higher than for 64. But 1,000-2,000 points / 32 = 32-64 tasks per layer, providing 2-4 tasks per thread on 16 cores — enough for good utilization. If profiling shows per-task overhead dominating, increase to `par_chunks(64)`.
- **Union toggle**: `union_paint_regions_at_harvest: false` produces un-unioned regions (many small polygons per SemanticRegion). This regresses query-path performance (more polygons to iterate, even with AABB pre-filter). The toggle is for benchmarking only — not recommended for production. Document as such.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` — Step 1 (shared function): S, Step 2 (new stage): M, Step 3 (annotator move): M, Step 4 (PrePass fallback): M, Step 5 (test migration): M, Step 6 (test rewrite): M, Step 7 (module deletion): S, Step 8 (dead WIT removal): M, Step 9 (per-point parallelism): S, Step 10 (config toggle): S, Step 11 (docs): S
- Largest single step: `M` (Step 6: rewriting 5 test files — each requires reading the current `.wasm`-loading test, understanding its assertions, and porting to the host fallback path)
- Highest-risk dispatch: the `slicer-host --release -- run --report` on benchy_4color — may produce > 100 lines of HTML. Implementer must filter for the annotator timing row and report timestamp only. Return format: FACT (wall-clock + timestamp + pass/fail).

## Open Questions

- [FWD] Where exactly does the `Layer` stage enum live? The implementer must locate it (likely `crates/slicer-ir/src/slice_ir.rs` or a stage-specific module) before adding the variant. Delegate a LOCATIONS search for `"Layer::SlicePostProcess"` to find the enum definition.
- [FWD] Does `dispatch_helpers.rs` contain content beyond `harvest_paint_segmentation_ir_from_ctx()`? If so, delete only the function; if it's the sole content, delete the file. Delegate a file-read before Step 8.
- [FWD] Are there any `.wasm` artifacts for the deleted modules in `modules/core-modules/*/` build output directories? The implementer should run `build-core-modules.sh` after deletion and verify no stale artifacts remain. Delegate a `find` or `Get-ChildItem` for `paint-segmentation*.wasm` and `paint-region-annotator*.wasm` after Step 7.
- None activation-blocking.
