# Implementation Status

Last updated: 2026-04-14

## Phase Dependencies (Normative Planning View)
- Phase B depends on Phase A.
- Phase C depends on Phases A and B.
- Phase D depends on Phases A and C.
- Phase E (MVP) depends on Phases A, B, C, and D.
- Phase F (Post-MVP) depends on Phase E.
- Phase G (Pipeline Wiring & WASM Integration) depends on Phase F.
- Phase H (End-to-End Integration & Review) depends on Phase G.
- Architecture Acceptance Gate requires evidence from completed Phases B-H.

## Phase A — Foundation
- [x] TASK-001 Workspace Cargo.toml with all crate members
- [x] TASK-002 crates/slicer-ir/ — all IR structs
- [x] TASK-003 wit/ directory — all WIT files
- [x] TASK-004 crates/slicer-macros/ — proc-macro crate skeleton
- [x] TASK-005 crates/slicer-test/ — mock host + fixture builders
- [x] TASK-006 crates/slicer-sdk/ — re-exports + host service wrappers

## Phase B — Core Algorithms
- [x] TASK-010 Clipper2-Rust + polygon operations
- [x] TASK-011 TriangleMeshSlicer (slice_mesh_ex)
- [x] TASK-012 Loop chaining (chain_lines_by_triangle_connectivity)
- [x] TASK-013 Geometry helpers
- [x] TASK-014 AABB tree for mesh queries
- [x] TASK-015 Point-in-polygon for paint region queries

## Phase C — Host Scheduler
- [x] TASK-020 Manifest ingestion
- [x] TASK-021 DAG construction
- [x] TASK-022 DAG validation (all 13 passes)
- [x] TASK-023 Topological sort
- [x] TASK-024 WASM instance pool
- [x] TASK-025 ExecutionPlan builder
- [x] TASK-026 Blackboard + LayerArena
- [x] TASK-027 PrePass executor
- [x] TASK-028 MeshSegmentation stage executor
- [x] TASK-029 PaintSegmentation stage executor
- [x] TASK-030 SlicePostProcess paint annotation executor (PaintRegionAnnotator)
- [x] TASK-031 Per-layer parallel executor
- [x] TASK-032 LayerFinalization executor
- [x] TASK-033 PostPass executor
- [x] TASK-034 GCodeEmit built-in serializer
- [x] TASK-035 Config schema query API
- [x] TASK-036 Progress event emitter

## Phase D — SDK Tooling
- [x] TASK-040 #[slicer_module] proc-macro
- [x] TASK-041 #[module_test] proc-macro
- [x] TASK-042 LayerModule trait + WIT bindings
- [x] TASK-043 PrepassModule trait + WIT bindings
- [x] TASK-044 PostpassModule trait + WIT bindings
- [x] TASK-045 SliceRegionViewBuilder
- [x] TASK-046 PerimeterRegionViewBuilder
- [x] TASK-047 ConfigViewBuilder
- [x] TASK-048 Output capture types
- [x] TASK-049 assert_paths helpers
- [x] TASK-050 `slicer new` command
- [x] TASK-051 `slicer build` command
- [x] TASK-052 `slicer test` command
- [x] TASK-053 `slicer validate` command
- [x] TASK-054 `slicer run` command
- [x] TASK-055 Create crates/slicer-helpers/ workspace member; add meshopt, truck-stepio, truck-meshing to root Cargo.toml
- [x] TASK-056 Write failing tests in repair_tdd.rs; implement repair.rs (degenerate removal, orientation normalization, open-edge closure); all tests pass
- [x] TASK-057 Write failing tests in decimate_tdd.rs; implement decimate.rs via meshopt; all tests pass
- [x] TASK-058 Create STEP test fixtures; write failing tests in import_step_tdd.rs; implement import/step.rs via truck; all tests pass

## Phase E — MVP Core Modules & CLI

- [x] TASK-070 layer-planner-default
- [x] TASK-071 classic-perimeters
- [x] TASK-072 rectilinear-infill
- [x] TASK-073 traditional-support
- [x] TASK-074 CLI argument parsing
- [x] TASK-075 Main entry point
- [x] TASK-076 File format loaders (STL/OBJ/3MF) — mesh repair component superseded by TASK-017 (slicer-helpers)
- [x] TASK-077 Integration test: end-to-end STL pipeline with model loading

## Phase F — Post-MVP & Advanced Features
- [x] TASK-081 arachne-perimeters
- [x] TASK-082 gyroid-infill
- [x] TASK-083 lightning-infill
- [x] TASK-084 seam-placer
- [x] TASK-085 tree-support
- [x] TASK-086 support-surface-ironing
- [x] TASK-087 mesh-segmentation
- [x] TASK-088 paint-segmentation
- [x] TASK-089 wipe-tower
- [x] TASK-090 skirt-brim
- [x] TASK-091 paint-region-annotator
- [x] TASK-092 fuzzy-skin
- [x] TASK-093 classic-perimeters (boundary_paint propagation)
- [x] TASK-094 arachne-perimeters (boundary_paint propagation)
- [x] TASK-095 traditional-support (enforcer/blocker)
- [x] TASK-096 tree-support (enforcer/blocker)
- [x] TASK-097 verify paint-region-annotator implementation (verified: 9 tests pass)

## Phase G — Pipeline Wiring & WASM Integration
- [x] TASK-100 Add `wasmtime` and `wit-bindgen` dependencies to `slicer-host`
- [x] TASK-101 Implement `WasmInstance` wrapper for compiled `wasmtime::component::Instance`
- [x] TASK-102 Implement concrete WASM trait runners — consolidated into a single `WasmRuntimeDispatcher` in `crates/slicer-host/src/dispatch.rs` that implements `PrepassStageRunner`, `LayerStageRunner`, `FinalizationStageRunner`, and `PostpassStageRunner`
- [x] TASK-103 Implement WASM module compilation and linking in `ExecutionPlan` builder — `main.rs::build_plan_from_loaded_modules` compiles each `.wasm` via `WasmEngine::compile_component` and attaches the resulting component to `CompiledModule`
- [x] TASK-104 Integrate Python bridge for text post-processing — `crates/slicer-host/src/python_bridge.rs` implements `PythonBinding`, `PythonBridge`, and `PythonPostpassRunner` on the real `execute_postpass` path. Backend is embedded PyO3 (`pyo3 = "0.28.3"`, feature `auto-initialize`); scripts are loaded via `importlib.util.spec_from_file_location` and the declared entry is called as `entry(text, config_dict)`. Failures surface as `PostpassError::FatalModule` wrapping `PythonBridgeError { phase, message }` with phases `MissingScript`, `ConfigEncoding`, `Init`, `ScriptError`, `OutputEncoding`. DEV-001 is closed.
- [x] TASK-105 Implement `PrePassMeshAnalysis` built-in stage — `crates/slicer-host/src/mesh_analysis.rs` classifies triangle facets (`TopSurface` / `BottomSurface` / `Overhang{angle_deg}` / `Normal`) from per-object normals, emits one baseline `SurfaceGroup` per object and an `OverhangRegion` aggregating overhang facets. Wired into the real prepass path via `execute_prepass_with_builtins` (used by `pipeline.rs`); `execute_prepass` itself is unchanged so existing runner-driven tests keep their module-commit contracts. Failures surface as `PrepassExecutionError::MeshAnalysis { source: MeshAnalysisError }`.
- [x] TASK-106 Implement `PrePassRegionMapping` built-in stage — `crates/slicer-host/src/region_mapping.rs` compiles `RegionMapIR` from the committed `LayerPlanIR` + scheduler-bound `ExecutionPlan`, one `RegionPlan` per `(layer, object, region)`. Invoked at the end of `execute_prepass_with_builtins` (after any user `PrePass::LayerPlanning` module); idempotent if a caller pre-committed the map. Enforces `DEFAULT_REGION_MAP_CAP`; structured `RegionMappingError { CapExceeded, DuplicateRegionKey }` plus wrapper `RegionMappingBuiltinError { MissingLayerPlan, Mapping, Blackboard }` surfaced through `PrepassExecutionError::RegionMapping`.
- [x] TASK-107 Wire `LayerSlice` into the pipeline (`slice_mesh_ex`) — `crates/slicer-host/src/layer_slice.rs::execute_layer_slice` runs inside `execute_single_layer` (see `layer_executor.rs`), slices each `GlobalLayer.active_regions` entry via `slice_mesh_ex` and commits the resulting `SliceIR` into the per-layer arena before any user `Layer::Slice` / `Layer::SlicePostProcess` module. The slicer-core `chain_lines` was rewritten to undirected point connectivity and `intersect_edge` now canonicalizes interpolation by vertex ID, so the real 3DBenchy mesh now produces non-empty hull contours at low-Z layers (previously every Benchy slice returned 0 polygons due to opposite-winding adjacent triangles fragmenting the directed chain walker). Regression tests: `layer_slice_builtin_produces_real_polygons_for_benchy_mesh`, `layer_slice_builtin_is_deterministic_for_benchy_mesh` (slicer-host), `test_shared_edge_with_opposite_windings_produces_closed_loop` (slicer-core).
- [x] TASK-108 Wire `SlicePostProcess` paint annotator into the pipeline — `execute_slice_postprocess_paint_annotation` runs on the production per-layer path via `layer_executor.rs::run_paint_annotation`, invoked at the end of the `Layer::SlicePostProcess` stage (or as a fallback when no such stage was scheduled). Fallback warnings flow through `paint_annotation_warning_to_progress_event` to the `LayerProgressSink` wired by `pipeline.rs::run_pipeline_with_events`; the slicer-host binary's `Run` arm constructs a `RuntimeProgressSink` backed by `JsonLinesEmitter` + `SliceEventCollector` so non-fatal fallbacks raise `degraded=true` and emit stable-code JSONL records. Fatal contract violations surface as `LayerExecutionError::PaintAnnotation { source: SlicePostProcessPaintAnnotationError }`. Covered by `slicer-host/tests/paint_annotation_integration_tdd.rs` (8 tests, incl. determinism, fatal-missing-semantic, runtime-sink fan-out, and main.rs wiring guard).
- [x] TASK-109 Implement real export/binding glue in `#[slicer_module]` macro — the macro now emits real `wit_bindgen::generate!`-backed typed component exports for every supported WIT world: `postpass-module` (gcode + text postprocess), `finalization-module` (layer finalization), `prepass-module` (mesh-analysis + layer-planning), and `layer-module` (all 8 stage exports + `on-print-start` / `on-print-end` lifecycle). A shared `emit_world_preamble` helper emits the inline-WIT `wit_bindgen::generate!` expansion plus a typed `ConfigView` adapter (every `ConfigValue` variant preserved) and a `ModuleError` cross-boundary mapper. A per-world `impl Guest for __Slicer<World>Component` routes the detected stage into the corresponding SDK trait method (`PostpassModule` / `FinalizationModule` / `PrepassModule` / `LayerModule`), and the `placeholder extern "C" fn ... -> i32 { 0 }` shims are suppressed for every supported world so they do not collide with or contaminate the real component exports. Four round-trip guests authored purely via `#[slicer_module]` (no hand-rolled `wit_bindgen::generate!` / `export!`) compile to real component-model `.wasm` artifacts and round-trip typed config + typed `Result<_, ModuleError>` through `WasmRuntimeDispatcher`. The two currently un-routed prepass stages (`MeshSegmentation`, `PaintSegmentation`) deliberately remain on the placeholder path because the host-side dispatcher does not yet invoke them. Covered by `crates/slicer-macros/tests/all_worlds_glue_tdd.rs` (10 source guards), `crates/slicer-macros/tests/postpass_text_glue_tdd.rs` (5 tests, carried forward), `crates/slicer-host/tests/macro_postpass_text_roundtrip_tdd.rs` (3 postpass round-trip tests), and `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` (9 end-to-end round-trip tests for finalization, prepass, and layer worlds).
- [x] TASK-110 Add `.toml` manifests for all MVP core modules — all 16 core modules under `modules/core-modules/` have matching-stem manifests
- [x] TASK-111 Apply `#[slicer_module]` macro to all MVP core modules — all 16 core modules under `modules/core-modules/` now carry a `#[slicer_module]` impl and depend directly on `slicer-schema` so the macro's emitted `::slicer_schema::SlicerModuleSchema` path resolves. 14 modules already implemented an SDK trait (Layer / Prepass / Postpass); the two legacy finalization modules (`skirt-brim`, `wipe-tower`) additionally gained an additive `impl FinalizationModule` adapter whose `on_print_start` delegates to the existing `from_config` constructor and whose `run_finalization` retains the trait default — preserving the legacy `process(&mut Vec<LayerCollectionIR>)` runtime path unchanged (no pipeline caller for the trait boundary exists yet). Covered by `slicer-host/tests/core_module_macro_adoption_tdd.rs` (macro adoption, slicer-schema dep, matching-stem manifest).
- [x] TASK-112 Implement `ConfigSchema` CLI output in `slicer-host`'s `main.rs` — emits pretty-printed JSON via `build_config_schema_json(&load_report.modules)`
- [x] TASK-113 Wire real WASM runners and DAG validation into `slicer-host/src/main.rs` (replacing `Noop` mocks) and update `slicer run` — `Noop*Runner` removed; `WasmRuntimeDispatcher` wired for all four stage runners

## Phase H — End-to-End Integration & Review
- [ ] TASK-120 Produce a fully sliced `.gcode` of the Benchy STL with tree supports enabled as an E2E integration test

## Known Deviations from Architecture Docs
- **TASK-109 closed (TASK-111 closed)** — `#[slicer_module]` emits typed `SlicerModuleSchema` reflection for every world AND real `wit_bindgen::generate!`-backed typed dispatch for every supported world (postpass, finalization, prepass, layer). For `world-finalization` specifically the typed path now does **real resource-level deep copy**: `Vec<LayerCollectionView>` inputs are forwarded from host `LayerCollectionViewData` (carrying `layer_index`, `z`, `entity_count`, `tool_changes`) through wit-bindgen resource accessors into SDK `LayerCollectionView` values inside the macro-emitted `impl Guest`; and `FinalizationOutputBuilder` pushes emitted by the guest are captured (via the resource's `drop` handler moving entries onto `HostExecutionContext::finalization_pushes`), drained by `FinalizationStageRunner`, and applied to the downstream `&mut Vec<LayerCollectionIR>` as ordered extrusion appends / synthetic layers. Remaining narrow gap: resource-level deep copy is not yet implemented for the prepass and layer worlds (trait methods there still receive empty-but-typed SDK views/builders), and the two un-routed prepass stages (`MeshSegmentation`, `PaintSegmentation`) stay on the placeholder path because the host dispatcher does not yet invoke them. All 16 core modules have adopted `#[slicer_module]` (TASK-111 closed).
- **TASK-108 closed** — paint-annotation helper now runs on the real per-layer pipeline path (`layer_executor.rs::run_paint_annotation` → `execute_slice_postprocess_paint_annotation`) and its warnings reach the documented progress-event transport via `run_pipeline_with_events` + `RuntimeProgressSink` (JSONL emitter + `SliceEventCollector`). Verified by `slicer-host/tests/paint_annotation_integration_tdd.rs`.
- **TASK-107 closed** — `execute_layer_slice` now runs on the production per-layer path and commits a real `SliceIR` before any user `Layer::Slice` / `Layer::SlicePostProcess` module. The real 3DBenchy STL passes through `slice_mesh_ex` and produces non-empty hull contours at representative Z values (≥20 contour points at z ∈ {0.2, 1.0, 5.0, 10.0}). The end-to-end `benchy_e2e_against_real_core_modules_is_diagnosable` run still produces empty G-code, but that is now downstream of slicing — the current blocker is that `Layer::Perimeters`, `Layer::Infill`, and `Layer::PathOptimization` core-modules are still 8-byte placeholder .wasm binaries (TASK-109/TASK-111 drift below), so no `PerimeterIR` / `InfillIR` entities are generated to feed `DefaultGCodeEmitter`.
- **Support ABI (deviation #7)** — resolved. `SliceRegionView::needs_support()` in `crates/slicer-sdk/src/views.rs` surfaces the `SurfaceClassificationIR.needs_support` flag (docs/02 §IR 2 line 231). Both `traditional-support` and `tree-support` apply the documented precedence (blocker → no; enforcer → yes; default → `needs_support()`), matching docs/02 §412 and docs/06 §702–704.
- **ConfigView typed access (deviation #8)** — resolved. `ConfigView.fields` is now private in `crates/slicer-ir/src/slice_ir.rs`; every external-crate construction goes through `ConfigView::new`, `ConfigView::from_map`, or `ConfigView::from_declared`, and every read goes through the typed accessors (`get`, `get_bool`, `get_int`, `get_float`, `get_string`, `keys`, `iter_entries`) — mirroring the read-only `resource config-view` in `wit/deps/config.wit`. The live host path (`main.rs` → `build_live_execution_plan` → `bind_module_config_view` → `ConfigView::from_declared`) pre-filters every per-module view to the module's declared `[config.schema]` keys, and the plan builder's `ExecutionPlanError::UndeclaredConfigKey` guardrail still fails closed if any caller bypasses the helper. Covered by `crates/slicer-ir/tests/config_view_encapsulation_tdd.rs` (8 external-crate contract tests) and `crates/slicer-host/tests/config_view_encapsulation_source_tdd.rs` (2 source-level regression guards on the `pub` field and the main.rs wiring).
- **SDK host-service wrappers (deviation #9)** — resolved. `crates/slicer-sdk/src/host.rs` is no longer placeholder: logging routes through an installable thread-local sink (fallback to stderr), mesh queries use a thread-local `MeshSource` with an explicit `HostUnavailable` error for `object_bounds` (replacing the silent zero-box), geometry helpers delegate to `slicer_core::polygon_ops` (same backend the host uses), `simplify_polygon` actually drops collinear vertices, and `now_us()` is monotonic relative to a process-start `Instant` (not wall-clock).
- **Paint annotation degraded/fallback semantics (deviation #10)** — resolved. `execute_slice_postprocess_paint_annotation` in `crates/slicer-host/src/slice_postprocess.rs` emits structured `SlicePostProcessPaintAnnotationWarning` records with stable codes, `degraded: bool`, deterministic `fallback_value`, and a `paint_annotation_warning_to_progress_event` adapter; missing-paint conditions produce typed fatal errors. Pipeline wiring is now closed via TASK-108 above — the annotator runs on the live per-layer path and its warnings reach the JSONL transport and `SliceEventCollector` through `RuntimeProgressSink`.
- If additional deviations are introduced, add them to `./docs/DEVIATION_LOG.md` and link their IDs here.

## Architecture Acceptance Gate
- Status: NOT YET EVALUATED
- Evidence links:
	- Determinism: (pending)
	- Recoverability: (pending)
	- Resource bounds: (pending)
	- Coupling control: (pending)
	- Compatibility: (pending)
	- Operability: (pending)
- Notes:
	- Use `./docs/11_operational_governance_and_acceptance_gate.md` rubric.
	- Metric thresholds are defined in `./docs/12_architecture_gate_metrics.md`.

## Blocked Tasks
- None currently.

## Governance Checklist Status
- Module/claim rollout checklist: [NOT STARTED | IN PROGRESS | COMPLETE]
- Compatibility policy checks: [NOT STARTED | IN PROGRESS | COMPLETE]
- Release checklist: [NOT STARTED | IN PROGRESS | COMPLETE]
