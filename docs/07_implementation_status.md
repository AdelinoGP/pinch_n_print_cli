# Implementation Status

Last updated: 2026-04-10

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
- [ ] TASK-107 Wire `LayerSlice` into the pipeline (`slice_mesh_ex`)
- [ ] TASK-108 Wire `SlicePostProcess` paint annotator into the pipeline — annotator function exists (`execute_slice_postprocess_paint_annotation`) and is tested, but is not invoked by `pipeline.rs` / `layer_executor.rs`
- [ ] TASK-109 Implement `wit_bindgen::generate!` WASM export logic in `#[slicer_module]` macro — macro currently emits marker methods only; no `wit_bindgen` export bindings generated
- [x] TASK-110 Add `.toml` manifests for all MVP core modules — all 16 core modules under `modules/core-modules/` have matching-stem manifests
- [ ] TASK-111 Apply `#[slicer_module]` macro to all MVP core modules — core modules implement SDK traits directly; macro is not applied (blocked on TASK-109)
- [x] TASK-112 Implement `ConfigSchema` CLI output in `slicer-host`'s `main.rs` — emits pretty-printed JSON via `build_config_schema_json(&load_report.modules)`
- [x] TASK-113 Wire real WASM runners and DAG validation into `slicer-host/src/main.rs` (replacing `Noop` mocks) and update `slicer run` — `Noop*Runner` removed; `WasmRuntimeDispatcher` wired for all four stage runners

## Phase H — End-to-End Integration & Review
- [ ] TASK-120 Produce a fully sliced `.gcode` of the Benchy STL with tree supports enabled as an E2E integration test

## Known Deviations from Architecture Docs
- **TASK-109 / TASK-111 drift** — `#[slicer_module]` does not yet emit `wit_bindgen::generate!` export bindings. Core modules consequently implement SDK traits directly rather than via the macro. Current host dispatch reaches modules through the compiled component + `WasmRuntimeDispatcher`, so functional wiring is intact, but the documented authoring path (macro → WIT exports) is not yet realized.
- **TASK-108 drift** — the paint-annotation helper is implemented and tested but not invoked by the pipeline orchestration path; slice post-process paint annotation currently does not run end-to-end.
- **TASK-107 open** — the `LayerSlice` (`slice_mesh_ex`) wiring is not yet implemented; the mesh → slice path has no host-side slicing stage feeding the layer loop. (TASK-104 Python text-postprocess bridge, TASK-105 built-in MeshAnalysis, and TASK-106 built-in RegionMapping have landed.)
- **Support ABI (deviation #7)** — resolved. `SliceRegionView::needs_support()` in `crates/slicer-sdk/src/views.rs` surfaces the `SurfaceClassificationIR.needs_support` flag (docs/02 §IR 2 line 231). Both `traditional-support` and `tree-support` apply the documented precedence (blocker → no; enforcer → yes; default → `needs_support()`), matching docs/02 §412 and docs/06 §702–704.
- **ConfigView typed access (deviation #8)** — partial. `ConfigView` in `crates/slicer-ir/src/slice_ir.rs` implements the documented WIT accessors (`get_bool`, `get_int`, `get_float`, `get_string`, `get_float_list`, `get_string_list`, `keys`) with documented subnormal normalization on floats. Remaining drift: the `fields` map is `pub` rather than encapsulated (docs specify a read-only resource), and the host does not pre-filter `ConfigView` by a module's declared config reads — `main.rs::build_plan_from_loaded_modules` constructs every `CompiledModule` with an empty `HashMap::new()`, so declared-reads filtering is not yet exercised end-to-end.
- **SDK host-service wrappers (deviation #9)** — resolved. `crates/slicer-sdk/src/host.rs` is no longer placeholder: logging routes through an installable thread-local sink (fallback to stderr), mesh queries use a thread-local `MeshSource` with an explicit `HostUnavailable` error for `object_bounds` (replacing the silent zero-box), geometry helpers delegate to `slicer_core::polygon_ops` (same backend the host uses), `simplify_polygon` actually drops collinear vertices, and `now_us()` is monotonic relative to a process-start `Instant` (not wall-clock).
- **Paint annotation degraded/fallback semantics (deviation #10)** — resolved for semantics; pipeline wiring still open. `execute_slice_postprocess_paint_annotation` in `crates/slicer-host/src/slice_postprocess.rs` emits structured `SlicePostProcessPaintAnnotationWarning` records with stable codes, `degraded: bool`, deterministic `fallback_value`, and a `paint_annotation_warning_to_progress_event` adapter; missing-paint conditions produce typed fatal errors. The end-to-end gap is TASK-108 above — the annotator is not yet called from `pipeline.rs` / `layer_executor.rs`.
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
