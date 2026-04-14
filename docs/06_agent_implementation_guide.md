# ModularSlicer — Agent Implementation Guide

This document defines how Claude Code agents should implement this project. It specifies the Planner Agent, the three SubAgent roles, the TDD workflow, and the implementation order.

---

## Agent Architecture

```
┌─────────────────────────────────────────────────────┐
│                  PLANNER AGENT                       │
│                                                      │
│  - Reads all ./docs/ files before issuing any tasks   │
│  - Decomposes work into atomic tasks                 │
│  - Assigns tasks to the correct SubAgent role        │
│  - Reviews SubAgent output before marking done      │
│  - Maintains implementation_status.md               │
│  - Never writes implementation code directly         │
└──────────────────┬──────────────────────────────────┘
                   │ issues tasks to
        ┌──────────┼──────────────────┐
        ▼          ▼                  ▼
  ┌───────────┐ ┌───────────┐ ┌─────────────┐
  │  CODING   │ │    QA     │ │    DOCS     │
  │  AGENT    │ │  AGENT    │ │   AGENT     │
  └───────────┘ └───────────┘ └─────────────┘
```

---

## Planner Agent

### System Prompt

```
You are the Planner Agent for the ModularSlicer project.

Your responsibilities:
1. Read ALL files in ./docs/ before creating any task.
2. Decompose the implementation into atomic tasks (one task = one PR-sized unit of work).
3. Assign each task to the correct SubAgent role (Coding, QA, or Docs).
4. Verify SubAgent output against the architecture docs before marking a task complete.
5. Maintain ./docs/implementation_status.md with current progress.
6. Never write implementation code yourself.
7. Never skip the TDD cycle: tests must exist and fail before implementation begins.
8. Before creating tasks for Coding or QA agents, check ./OrcaSlicerDocumented/ for any
  existing source files or tests related to the feature and reference them in the task.

Rules:
- A task is NOT complete until: (a) tests pass, (b) code compiles, (c) docs are updated.
- If a SubAgent output contradicts the architecture docs, reject it and re-issue with corrections.
- Implementation must follow the exact crate structure defined in ./docs/00_project_overview.md.
- IR types must exactly match ./docs/02_ir_schemas.md — no deviation without updating the doc first.
- WIT interfaces must match ./docs/03_wit_and_manifest.md exactly.

- Before issuing any Coding or QA tasks, inspect the folder `./OrcaSlicerDocumented/` for
  related source files and tests. If relevant artifacts are found, include references to
  them in the task description so SubAgents can reuse or adapt existing material.

When issuing a task, always include:
- Which doc file(s) are authoritative for this task
- The exact file(s) to create or modify
- The acceptance criteria (what tests must pass)
- Which SubAgent role to use
```

### Task Template

```markdown
## Task: [TASK-ID] [Short Title]

**Role:** Coding | QA | Docs
**Authoritative docs:** ./docs/XX_filename.md (section: "...")
**Files to create/modify:**
- `crates/slicer-ir/src/slice_ir.rs` (create)
- `crates/slicer-ir/src/lib.rs` (modify: add pub mod)

**Context:**
[Brief description of what this task accomplishes and why]

**Acceptance criteria:**
- [ ] `cargo test -p slicer-ir` passes
- [ ] All IR structs match ./docs/02_ir_schemas.md exactly (field names, types, comments)
- [ ] `schema_version` field present on all top-level IR structs
- [ ] Serde derives present (Serialize, Deserialize, Clone, Debug)
- [ ] No public fields without doc comments

**TDD requirement:**
Write tests in `crates/slicer-ir/tests/` BEFORE implementing the structs.
Tests should verify: struct construction, serde round-trip, schema_version presence.
```

---

## Coding SubAgent

### System Prompt

```
You are the Coding SubAgent for the ModularSlicer project.

Your responsibilities:
1. Implement exactly what the Planner task specifies — no more, no less.
2. Follow TDD strictly: write failing tests first, then implement.
3. Match all types, field names, and signatures to the architecture docs exactly.
4. Use the crate structure defined in ./docs/00_project_overview.md.
5. Every public item must have a doc comment.
6. Every module must have at least one unit test.

**Coordinate system rules (non-negotiable):**
- Geometry coordinates: i64 scaled integers (100nm units) for Point2, f32 mm for Point3
- 1 unit = 100 nm = 10⁻⁴ mm. Scaling factor: 10_000.
- OrcaSlicer uses 1_000_000. We use 10_000. They differ by 100×.
- Never write a raw integer coordinate literal. Always use `Point2::mm_to_units()` or `Point2::units_to_mm()`.
- When porting an OrcaSlicer constant, divide by 100 and add a comment:
  `// OrcaSlicer: 400_000 → ModularSlicer: 4_000 (÷100, see ./docs/08_coordinate_system.md)`
- If you see `SCALING_FACTOR = 1_000_000` anywhere in ModularSlicer/, that is a bug. Stop and flag it to the Planner before continuing.

**Paint system rules:**
- `PaintSemantic` is an enum, not a string tag. Never use raw strings where
  a `PaintSemantic` variant is expected.
- `PaintRegionIR.get(layer, semantic)` returns `&[]` (empty slice) when no
  paint of that semantic exists — never `.unwrap()` this call.
- `WallLoop.feature_flags` is parallel to `path.points`. If you add or remove
  points from a path, you must add or remove corresponding entries from
  `feature_flags` to maintain the invariant. The host validates this in debug
  builds.
- `boundary_paint` on `SlicedRegion` is populated by `PaintRegionAnnotator`
  in `Layer::SlicePostProcess`. Never read it in earlier stages — it will be empty.
- Support modules must check `SupportBlocker` before `SupportEnforcer`.
  A point that is both blocked and enforced resolves to blocked.

Rust style rules:
- Edition 2021
- All warnings must be clean (no #[allow(unused)] unless justified in a comment)
- Use `thiserror` for error types
- Use `serde` with `#[serde(rename_all = "snake_case")]` on all IR types
- Prefer `Arc<T>` for shared read-only data, `&T` for borrowed references
- No `unwrap()` in non-test code — use `?` or explicit error handling

When implementing a WIT-bound function:
- Match the exact function signature from ./docs/03_wit_and_manifest.md
- Never add parameters not in the WIT signature
- Map WIT errors to ModuleError using the SDK helpers

When implementing an IR struct:
- Match ./docs/02_ir_schemas.md field-for-field
- Add #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] to all structs
- Add schema_version: SemVer to all top-level IR structs

File placement:
- IR types → crates/slicer-ir/src/
- Host scheduler → crates/slicer-host/src/scheduler/
- Core algorithms → crates/slicer-core/src/
- SDK types → crates/slicer-sdk/src/
- Tests → in the same file (unit) or crates/*/tests/ (integration)
```

---

## QA SubAgent

### System Prompt

```
You are the QA SubAgent for the ModularSlicer project.

Your responsibilities:
1. Write comprehensive test suites for features implemented by the Coding SubAgent.
2. Use TDD: your tests must be written and failing before the Coding SubAgent implements.
3. Cover: happy path, edge cases, error conditions, boundary values.
4. Use slicer-test fixtures and helpers — do not build raw mock structs by hand.
5. Performance tests for hot-path functions (per-layer stage functions).

Test categories you must cover for every module stage function:

Unit tests (in src/lib.rs or tests/ of the crate):
  - Correct output for a known input (golden test)
  - Empty input returns empty output (not an error)
  - Config boundary values (min, max, just-above-min, just-below-max)
  - Non-fatal error returned for recoverable bad input
  - Fatal error returned for unrecoverable bad input

Integration tests (in tests/integration/):
  - Module runs correctly inside a minimal host pipeline
  - Output of this module is valid input for the next stage
  - Module is layer-parallel-safe (run 8 layers in parallel, verify no races)

Property tests (using proptest crate):
  - For geometry functions: output is always inside input boundary
  - For config validation: no panic on any config value within declared range
  - For path generation: all output paths have finite coordinates (no NaN/Inf)

Performance tests (using criterion crate):
  - Benchmark the stage function against a 20mm × 20mm square at target density
  - Assert median time < ./docs/00_project_overview.md performance targets
  - Measure WASM boundary crossing overhead for the module

NaN/Inf guards:
  - Every f32/f64 output field must be asserted finite in golden tests
  - Use assert_paths::assert_all_finite() helper on all ExtrusionPath3D outputs
```

---

## Docs SubAgent

### System Prompt

```
You are the Docs SubAgent for the ModularSlicer project.

Your responsibilities:
1. Keep ./docs/ in sync with implemented code.
2. Generate rustdoc comments for all public items.
3. Update ./docs/implementation_status.md after each completed task.
4. Write the module authoring guide (./docs/06_authoring_guide.md) as SDK is implemented.
5. Generate CHANGELOG entries for completed features.

Rules:
- Never change the architecture docs (./docs/00–05) without Planner approval.
- Rustdoc comments must include an example for every public function.
- If an implementation deviates from architecture docs, flag it to the Planner — do NOT silently update the doc to match the deviation.
- Keep ./docs/implementation_status.md current after every task.
```

---

## TDD Cycle (Enforced)

Every task follows this exact cycle. The Planner verifies each step before proceeding.

```
Step 1 — QA Agent writes tests
  ├─ Tests reference the types/functions that WILL exist
  ├─ Tests compile (may need stub types with todo!())
  └─ Tests FAIL (red) — verified by running cargo test

Step 2 — Coding Agent implements
  ├─ Writes implementation until tests pass
  ├─ No new code beyond what tests require
  └─ Tests PASS (green) — verified by running cargo test

Step 3 — Coding Agent refactors
  ├─ Clean up implementation while keeping tests green
  └─ No logic changes — only structure

Step 4 — QA Agent reviews and supplements
  ├─ Checks edge cases missed in Step 1
  ├─ Adds property tests if needed
  └─ Runs full test suite including new tests

Step 5 — Docs Agent updates
  ├─ Rustdoc comments on all new public items
  └─ Updates implementation_status.md
```

---

## Implementation Order

Tasks must be implemented in this order. Later tasks depend on earlier ones.

### Phase A — Foundation (no dependencies)

```
TASK-001  Create workspace Cargo.toml with all crate members
TASK-002  Create crates/slicer-ir/  — all IR structs (02_ir_schemas.md)
TASK-003  Create wit/ directory — all WIT files (03_wit_and_manifest.md)
TASK-004  Create crates/slicer-macros/ — proc-macro crate skeleton
TASK-005  Create crates/slicer-test/ — mock host + fixture builders
TASK-006  Create crates/slicer-sdk/ — re-exports + host service wrappers
```

### Phase B — Core Algorithms (depends on Phase A)

```
TASK-010  crates/slicer-core/: Clipper2 Rust + polygon operations
  Stage: Core Infrastructure
  Purpose: Provide deterministic polygon boolean and offset primitives used by slice generation and downstream geometry transforms.
  Depends on: TASK-002
  

TASK-011  crates/slicer-core/: TriangleMeshSlicer (slice_mesh_ex)
  Stage: Layer::Slice
  Purpose: Perform triangle/plane intersection and produce raw slice geometry for each layer from immutable mesh and layer planning inputs.
  Depends on: TASK-010
  

TASK-012  crates/slicer-core/: Loop chaining (chain_lines_by_triangle_connectivity)
  Stage: Layer::Slice
  Purpose: Chain segment outputs from mesh slicing into valid closed loops suitable for region construction and clipping.
  Depends on: TASK-011
  

TASK-013  crates/slicer-core/: Geometry helpers (segment_path, path_length, etc.)
  Stage: Core Infrastructure
  Purpose: Supply reusable geometric utility operations needed by perimeters, infill, support, and post-processing path transforms.
  Depends on: TASK-010
  

TASK-014  crates/slicer-core/: AABB tree for mesh queries
  Stage: Core Infrastructure
  Purpose: Build accelerated spatial queries over mesh data to support fast raycasts, bounds checks, and stage-level geometric lookups.
  Depends on: TASK-002
  

TASK-015  crates/slicer-core/: Point-in-polygon test for paint region queries (used by PaintRegionAnnotator and host::point_in_paint_region)
  Stage: Layer::SlicePostProcess
  Purpose: Classify contour points against precomputed paint polygons so boundary paint annotations can be written deterministically.
  Depends on: TASK-010, TASK-013
  
```

### Phase C — Host Scheduler (depends on A, B)

```
TASK-020  crates/slicer-host/: Manifest ingestion (parse TOML → LoadedModule)
    Stage: Core Infrastructure
    Purpose: Load and normalize module metadata, declared stage placement, access contracts, and compatibility requirements into host runtime records.
    Depends on: TASK-003
    

TASK-021  crates/slicer-host/: DAG construction (build_intra_stage_dag)
    Stage: Core Infrastructure
    Purpose: Build per-stage dependency graphs from declared reads/writes and ordering constraints before execution plan freezing.
    Depends on: TASK-020
    

TASK-022  crates/slicer-host/: DAG validation (all 13 passes from 04_host_scheduler.md)
    Stage: Core Infrastructure
    Purpose: Enforce claim uniqueness, cycle freedom, read/write legality, and compatibility constraints so runtime execution is deterministic.
    Depends on: TASK-021
    

TASK-023  crates/slicer-host/: Topological sort (Kahn's algorithm)
    Stage: Core Infrastructure
    Purpose: Produce deterministic in-stage module ordering from validated DAGs for every execution stage.
    Depends on: TASK-021
    

TASK-024  crates/slicer-host/: WASM instance pool (wasmtime integration)
    Stage: Core Infrastructure
    Purpose: Provide isolated, reusable module instances with parallel-safe pooling semantics aligned to stage/thread constraints.
    Depends on: TASK-020
    

TASK-025  crates/slicer-host/: ExecutionPlan builder (freeze after validation)
    Stage: Core Infrastructure
    Purpose: Materialize immutable runtime scheduling state combining validated DAG order, access masks, claims, and module loading decisions.
    Depends on: TASK-022, TASK-023, TASK-024
    

TASK-026  crates/slicer-host/: Blackboard + LayerArena
    Stage: Core Infrastructure
    Purpose: Implement host-owned immutable Blackboard IR storage and per-layer arena allocation lifetimes for safe parallel layer execution.
    Depends on: TASK-002
    

TASK-027  crates/slicer-host/: PrePass executor
    Stage: Core Infrastructure
    Purpose: Execute whole-model PrePass stages in fixed order and persist outputs to Blackboard for downstream per-layer reads.
    Depends on: TASK-025, TASK-026
    

TASK-028  crates/slicer-host/: MeshSegmentation stage executor
    Stage: PrePass::MeshSegmentation
    Purpose: Convert sub-facet paint strokes into whole-triangle assignments by clipping mesh facets before any later semantic analysis stages run.
    Depends on: TASK-011, TASK-027
    
          Purpose: Clips mesh triangles at sub-facet stroke boundaries.
          Acceptance criteria:
          - A mesh with no strokes passes through unchanged
          - A mesh with a stroke across one triangle produces two triangles
            with correct paint values
          - clip_triangle_at_stroke() has unit tests for all degenerate cases
            (stroke tangent to edge, stroke through vertex, etc.)
TASK-029  crates/slicer-host/: PaintSegmentation stage executor
    Stage: PrePass::PaintSegmentation
    Purpose: Compute per-layer paint regions for every semantic from segmented mesh and authoritative layer plan, storing results in PaintRegionIR.
    Depends on: TASK-028, TASK-027, TASK-015
    
          Purpose: Converts tagged mesh triangles → per-layer polygon regions
          for all semantics simultaneously.
          Acceptance criteria:
          - Material regions produce correct tool_index on ActiveRegion
          - SupportEnforcer/Blocker regions appear in PaintRegionIR
          - FuzzySkin regions appear in PaintRegionIR
          - Custom semantics are preserved with their module ID key
          - A layer with no paint produces an empty LayerPaintMap (not absent)
          - Integration test: two objects with different materials produce
            correct tool changes in output G-code
TASK-030  crates/slicer-host/: SlicePostProcess paint annotation executor (PaintRegionAnnotator)
    Stage: Layer::SlicePostProcess
    Purpose: Run point-in-polygon annotation last in SlicePostProcess and write contour-parallel boundary paint metadata used by later wall processing.
    Depends on: TASK-029, TASK-015
    
          Stage: Layer::SlicePostProcess (runs last within that stage)
          Purpose: Point-in-polygon tests against PaintRegionIR,
          writes boundary_paint onto SlicedRegion contour points.
          Acceptance criteria:
          - boundary_paint is empty for regions with no paint
          - boundary_paint entries are parallel to polygon contour points
          - Adding/removing polygon points invalidates the annotation
            (test that the host debug-build check catches this)
TASK-031   crates/slicer-host/: Per-layer parallel executor (rayon)
    Stage: Core Infrastructure
    Purpose: Execute all Tier-2 layer stages in parallel with deterministic stage ordering and isolated per-layer mutable state.
    Depends on: TASK-025, TASK-026, TASK-030
    

TASK-032  crates/slicer-host/: LayerFinalization executor
    Stage: PostPass::LayerFinalization
    Purpose: Run full-print sequential finalization over all layers, including synthetic layer insertion and cross-layer feature realization.
    Depends on: TASK-031
    

TASK-033  crates/slicer-host/: PostPass executor
    Stage: Core Infrastructure
    Purpose: Orchestrate sequential PostPass stage execution order and handoff between structured IR and optional text post-processing.
    Depends on: TASK-032
    

TASK-034  crates/slicer-host/: GCodeEmit (built-in serializer)
    Stage: PostPass::GCodeEmit
    Purpose: Serialize finalized layer collections into structured G-code command streams with deterministic tool, fan, and temperature sequencing.
    Depends on: TASK-032
    

TASK-035  crates/slicer-host/: Config schema query API (JSON over stdout)
    Stage: Core Infrastructure
    Purpose: Expose module-contributed config schema metadata to frontend clients via host query protocol.
    Depends on: TASK-020, TASK-025
    

TASK-036  crates/slicer-host/: Progress event emitter (JSON over stdout)
    Stage: Core Infrastructure
    Purpose: Emit structured runtime progress, warning, error, and completion events with deterministic ordering guarantees.
    Depends on: TASK-027, TASK-031, TASK-033
    
          Contract: must conform to `./docs/09_progress_events.md`
```

### Phase D — SDK Tooling (depends on A)

```
TASK-040  crates/slicer-macros/: #[slicer_module] proc-macro
  Stage: Core Infrastructure
  Purpose: Generate module boilerplate and ABI-safe exports so community modules bind to host contracts with minimal manual glue code.
  Depends on: TASK-004
  

TASK-041  crates/slicer-macros/: #[module_test] proc-macro
  Stage: Core Infrastructure
  Purpose: Generate test harness integration scaffolding for module-level tests that run without a live host runtime.
  Depends on: TASK-004
  

TASK-042  crates/slicer-sdk/: LayerModule trait + WIT bindings
  Stage: Core Infrastructure
  Purpose: Define typed SDK interfaces for per-layer module worlds and map WIT contracts to ergonomic Rust traits.
  Depends on: TASK-003, TASK-006, TASK-040
  

TASK-043  crates/slicer-sdk/: PrepassModule trait + WIT bindings
  Stage: Core Infrastructure
  Purpose: Define typed SDK interfaces for PrePass module worlds and enforce host contract fidelity at compile time.
  Depends on: TASK-003, TASK-006, TASK-040
  

TASK-044  crates/slicer-sdk/: PostpassModule trait + WIT bindings
  Stage: Core Infrastructure
  Purpose: Define typed SDK interfaces for PostPass module worlds with correct sequential execution expectations.
  Depends on: TASK-003, TASK-006, TASK-040
  

TASK-045  crates/slicer-test/: SliceRegionViewBuilder
  Stage: Core Infrastructure
  Purpose: Provide deterministic builders for constructing SliceIR view fixtures used in module and host unit tests.
  Depends on: TASK-005, TASK-042
  

TASK-046  crates/slicer-test/: PerimeterRegionViewBuilder
  Stage: Core Infrastructure
  Purpose: Provide deterministic builders for PerimeterIR fixture setup and contract-focused test scenarios.
  Depends on: TASK-005, TASK-042
  

TASK-047  crates/slicer-test/: ConfigViewBuilder
  Stage: Core Infrastructure
  Purpose: Provide reproducible config view fixtures for module config validation and behavior tests.
  Depends on: TASK-005, TASK-006
  

TASK-048  crates/slicer-test/: InfillOutputCapture + PerimeterOutputCapture
  Stage: Core Infrastructure
  Purpose: Capture generated outputs from module calls to support golden checks and contract assertions in tests.
  Depends on: TASK-005, TASK-042
  

TASK-049  crates/slicer-test/: assert_paths helpers
  Stage: Core Infrastructure
  Purpose: Validate geometry/path invariants including finite coordinates, ordering, and expected path-level shape constraints.
  Depends on: TASK-048
  

TASK-050  cli/slicer-cli/: `slicer new` scaffold command
  Stage: Core Infrastructure
  Purpose: Generate a standard module project scaffold aligned with SDK macros, WIT contracts, and manifest expectations.
  Depends on: TASK-040, TASK-042
  

TASK-051  cli/slicer-cli/: `slicer build` WASM compile command
  Stage: Core Infrastructure
  Purpose: Compile module crates into deployable WASM artifacts compatible with host discovery and manifest pairing rules.
  Depends on: TASK-050, TASK-003
  

TASK-052  cli/slicer-cli/: `slicer test` command
  Stage: Core Infrastructure
  Purpose: Execute module test workflows against the test harness and expose pass/fail feedback for TDD loops.
  Depends on: TASK-051, TASK-049
  

TASK-053  cli/slicer-cli/: `slicer validate` command
  Stage: Core Infrastructure
  Purpose: Validate module manifests and compatibility metadata against host rules before runtime loading.
  Depends on: TASK-020, TASK-022, TASK-051
  

TASK-054  cli/slicer-cli/: `slicer run` command
  Stage: Core Infrastructure
  Purpose: Launch full host execution from CLI inputs and stream runtime events/results through the host protocol.
  Depends on: TASK-033, TASK-034, TASK-050
  
```

### Phase E — MVP Core Modules & CLI (depends on A, B, C, D)

```
TASK-070  modules/core-modules/layer-planner-default/
  Stage: PrePass::LayerPlanning
  Purpose: Compute authoritative global Z-plane sequences and region/object layer-height planning for downstream layer execution.
  Depends on: TASK-011, TASK-027, TASK-043
  Claims: layer-planner

TASK-071  modules/core-modules/classic-perimeters/
  Stage: Layer::Perimeters
  Purpose: Generate wall loops, seam candidates, and perimeter geometry from slice regions under the perimeter generation contract.
  Depends on: TASK-030, TASK-031, TASK-042
  Claims: perimeter-generator

TASK-072  modules/core-modules/rectilinear-infill/
  Stage: Layer::Infill
  Purpose: Generate rectilinear infill paths for valid infill areas and emit deterministic InfillIR outputs.
  Depends on: TASK-071, TASK-031, TASK-042
  Claims: infill-generator

TASK-073  modules/core-modules/traditional-support/
  Stage: Layer::Support
  Purpose: Generate support geometry from slice and surface classification inputs with deterministic support eligibility logic.
  Depends on: TASK-029, TASK-031, TASK-042
  Claims: support-generator

TASK-074  crates/slicer-host/: CLI argument parsing (clap)
  Stage: Core Infrastructure
  Purpose: Parse run/build/config command arguments and map CLI inputs into validated host runtime options.
  Depends on: TASK-054
  

TASK-075  crates/slicer-host/: Main entry point + signal handling
  Stage: Core Infrastructure
  Purpose: Initialize host runtime, wire cancellation/shutdown handling, and coordinate end-to-end execution lifecycle.
  Depends on: TASK-074, TASK-033
  

TASK-076  crates/slicer-host/: File format loaders (STL, 3MF, OBJ) + admesh-based mesh repair integration
  Stage: Core Infrastructure
  Purpose: Ingest source model formats into canonical mesh structures and perform pre-slice mesh repair before PrePass execution.
  Depends on: TASK-075, TASK-010
  

TASK-077  crates/slicer-host/: Integration test: slice benchy.stl end-to-end
  Stage: Core Infrastructure
  Purpose: Validate complete pipeline execution from model load to emitted G-code using representative fixture coverage.
  Depends on: TASK-070, TASK-071, TASK-072, TASK-073, TASK-076, TASK-034
  
```

### Phase F — Post-MVP & Advanced Features (depends on Phase E)

```
TASK-080  modules/core-modules/arachne-perimeters/
  Stage: Layer::Perimeters
  Purpose: Generate variable-width perimeter loops using Arachne-style wall strategies under the perimeter generation contract.
  Depends on: TASK-071, TASK-042
  Claims: perimeter-generator

TASK-081  modules/core-modules/gyroid-infill/
  Stage: Layer::Infill
  Purpose: Generate gyroid infill paths for eligible regions while preserving deterministic path generation behavior.
  Depends on: TASK-072, TASK-042
  Claims: infill-generator

TASK-082  modules/core-modules/lightning-infill/
  Stage: Layer::Infill
  Purpose: Generate sparse lightning-style internal support paths optimized for reduced material usage.
  Depends on: TASK-072, TASK-042
  Claims: infill-generator

TASK-083  modules/core-modules/seam-placer/
  Stage: Layer::PerimetersPostProcess
  Purpose: Optimize seam placement over generated wall loops to reduce visible artifacts while preserving print constraints.
  Depends on: TASK-071, TASK-080, TASK-042
  Claims: seam-placer

TASK-084  modules/core-modules/tree-support/
  Stage: Layer::Support
  Purpose: Generate branching tree-style supports from overhang and region constraints under support generation rules.
  Depends on: TASK-073, TASK-042
  Claims: support-generator

TASK-085  modules/core-modules/support-surface-ironing/
  Stage: Layer::SupportPostProcess
  Purpose: Refine support top/interface geometry after initial support generation to improve supported surface quality.
  Depends on: TASK-073, TASK-084, TASK-042
  

TASK-086  modules/core-modules/mesh-segmentation/
    Stage: PrePass::MeshSegmentation
  Purpose: Clip triangles at sub-facet paint stroke boundaries so downstream stages consume a uniformly tagged mesh.
  Depends on: TASK-028, TASK-043
  
TASK-087  modules/core-modules/paint-segmentation/
    Stage: PrePass::PaintSegmentation
  Purpose: Compute per-layer polygon regions for all paint semantics and write deterministic PaintRegionIR outputs.
  Depends on: TASK-029, TASK-043
  

TASK-088  modules/core-modules/wipe-tower/
    Stage: PostPass::LayerFinalization
  Purpose: Generate purge tower structures for multi-tool transitions across the full print after per-layer paths are finalized.
  Depends on: TASK-032, TASK-071
  
    Acceptance criteria:
    - Generates purge paths for every tool change across all layers
    - Purge volume scales correctly with colour distance between tools
    - Tower position does not intersect any object's bounding box

TASK-089  modules/core-modules/skirt-brim/
    Stage: PostPass::LayerFinalization
  Purpose: Append skirt or brim extrusion entities around print footprints as cross-layer finalization features before G-code emission.
  Depends on: TASK-032
  

TASK-090  modules/core-modules/paint-region-annotator/
    Stage: Layer::SlicePostProcess
  Purpose: Write contour-parallel boundary_paint annotations from PaintRegionIR after all slice polygon modifications are complete.
  Depends on: TASK-030, TASK-029
  
    Note: This is a host-built-in finalization step within SlicePostProcess, not a standalone scheduler stage and not community-replaceable.
    It does not hold a claim. It always runs last in SlicePostProcess.

TASK-091  modules/core-modules/fuzzy-skin/
    Stage: Layer::PerimetersPostProcess
  Purpose: Apply selective outer-wall perturbation using propagated feature flags while preserving path/flag cardinality.
  Depends on: TASK-090, TASK-071, TASK-080
  
    Acceptance criteria:
    - Segments with feature_flags.fuzzy_skin = true are perturbed
    - Segments with feature_flags.fuzzy_skin = false are unchanged
    - apply-to-all = true perturbs all outer wall segments regardless of flags
    - No perturbation on inner walls when apply-to-all is false
    - Path point count and feature_flags remain parallel after perturbation
    - Property test: all output points have finite coordinates

TASK-092  modules/core-modules/classic-perimeters/ 
    Stage: Layer::Perimeters
    Purpose: Extend classic perimeter generation to propagate boundary paint flags and detect material boundary metadata on wall loops.
    Depends on: TASK-071, TASK-090, TASK-087
    Claims: perimeter-generator
    Add: propagate boundary_paint from SlicedRegion.boundary_paint
      → WallLoop.feature_flags when generating wall paths.
    Add: detect adjacent material regions via PaintRegionIR,
      set WallLoop.boundary_type = MaterialBoundary where applicable.

TASK-093 modules/core-modules/arachne-perimeters/ (UPDATE existing task)
    Stage: Layer::Perimeters
    Purpose: Extend Arachne perimeter generation with boundary paint propagation and material boundary tagging parity.
    Depends on: TASK-080, TASK-090, TASK-087
    Claims: perimeter-generator
    Same additions as TASK-069g for the Arachne generator.

TASK-094 modules/core-modules/traditional-support/ (UPDATE existing task)
    Stage: Layer::Support
    Purpose: Enforce support paint semantics by applying blocker/enforcer precedence before overhang-angle support checks.
    Depends on: TASK-073, TASK-087
    Claims: support-generator
    Add: read PaintRegionIR for SupportEnforcer and SupportBlocker.
    Add: apply enforcer/blocker priority rules before angle threshold check.
    Acceptance criteria:
    - A fully blocked region generates zero support paths
    - A fully enforced region generates support paths at 0° overhang
    - A region that is both blocked and enforced generates zero support
    - Existing overhang-angle behavior unchanged for unpainted regions

TASK-095 modules/core-modules/tree-support/ (UPDATE existing task)
    Stage: Layer::Support
    Purpose: Add paint-driven support eligibility rules to tree support generation with blocker precedence and deterministic fallback behavior.
    Depends on: TASK-084, TASK-087
    Claims: support-generator
    Same additions as TASK-069i for the tree support generator.
```

### Phase G — Pipeline Wiring & WASM Integration (depends on Phase F)

```
TASK-100  crates/slicer-host/: Wasmtime dependencies
    Stage: Core Infrastructure
    Purpose: Add `wasmtime` and `wit-bindgen` to dependencies.
    Depends on: TASK-024, TASK-040
    
TASK-101  crates/slicer-host/: WasmInstance wrapper
    Stage: Core Infrastructure
    Purpose: Implement `WasmInstance` wrapper mapping WASM components to internal API.
    Depends on: TASK-100, TASK-024, TASK-040

TASK-102  crates/slicer-host/: Concrete WASM stage runners
    Stage: Core Infrastructure
    Purpose: Implement `WasmPrepassRunner`, `WasmLayerRunner`, `WasmFinalizationRunner`, and `WasmPostpassRunner` that invoke exports via `wasmtime`.
    Depends on: TASK-101, TASK-027, TASK-031, TASK-032, TASK-033

TASK-103  crates/slicer-host/: WASM module compilation in ExecutionPlan
    Stage: Core Infrastructure
    Purpose: Update the `ExecutionPlan` builder (TASK-025) to actually load, compile and instantiate WASM modules into `wasmtime::component::Component`.
    Depends on: TASK-101, TASK-025

TASK-104  crates/slicer-host/: Python Post-Processing Bridge
    Stage: PostPass
    Purpose: Implement python script execution using `pyo3` and `wasmtime-py` for text post-processing.
    Depends on: TASK-102

TASK-105  crates/slicer-host/: PrePassMeshAnalysis Built-in Stage
    Stage: PrePass::MeshAnalysis
    Purpose: Implement host-built-in logic to analyze mesh and produce `SurfaceClassificationIR`.
    Depends on: TASK-027
    
TASK-106  crates/slicer-host/: PrePassRegionMapping Built-in Stage
    Stage: PrePass::RegionMapping
    Purpose: Implement `build_region_map` to populate `RegionMapIR` and execute it at the end of the prepass.
    Depends on: TASK-027

TASK-107  crates/slicer-host/: LayerSlice Pipeline Wiring
    Stage: Layer::Slice
    Purpose: Wire the `LayerSlice` host-built-in stage into `execute_single_layer`, calling `slice_mesh_ex` to produce `SliceIR` for each layer.
    Depends on: TASK-031

TASK-108  crates/slicer-host/: SlicePostProcess Paint Annotator Wiring
    Stage: Layer::SlicePostProcess
    Purpose: Wire the `execute_slice_postprocess_paint_annotation` logic to run at the end of the `LayerSlicePostProcess` stage.
    Depends on: TASK-030, TASK-031

TASK-109  crates/slicer-macros/: Implement WIT WASM bindings in #[slicer_module]
    Stage: Core Infrastructure
    Purpose: Update `slicer_module` proc-macro to generate actual `wit_bindgen::generate!` calls and proper export implementations, making modules valid WASM components.
    Depends on: TASK-040

TASK-110  modules/core-modules/: Add `.toml` manifests
    Stage: Core Infrastructure
    Purpose: Write proper `.toml` manifest files for all core modules required by the architecture.
    Depends on: None

TASK-111  modules/core-modules/: Apply #[slicer_module] macro
    Stage: Core Infrastructure
    Purpose: Update all core modules to use the `#[slicer_module]` macro so they export valid WIT interfaces.
    Depends on: TASK-109, TASK-110

TASK-112  crates/slicer-host/: Implement ConfigSchema CLI output
    Stage: Core Infrastructure
    Purpose: Update `main.rs` `HostCommands::ConfigSchema` to read loaded module manifests and output their JSON schemas.
    Depends on: TASK-110, TASK-035

TASK-113  crates/slicer-host/: Main binary integration
    Stage: Core Infrastructure
    Purpose: Update `main.rs` to ingest manifests, validate the DAG, and replace `Noop` runners with concrete WASM runners, enabling true end-to-end execution.
    Depends on: TASK-102, TASK-103, TASK-104, TASK-075, TASK-112
```

### Phase H — End-to-End Integration & Review (depends on Phase G)

```
TASK-120  tests/: Benchy End-to-End Slice Test
    Stage: End-to-End
    Purpose: Produce a fully sliced `.gcode` of the Benchy STL as a form of End-to-End Integration testing to ensure the MVP is functional.
    Depends on: TASK-113
```

---

## Implementation Status Template

The Docs SubAgent maintains this file after every task:

```markdown
# Implementation Status

Last updated: [DATE]

## Phase A — Foundation
- [x] TASK-001 Workspace Cargo.toml
- [ ] TASK-002 slicer-ir crate
- [ ] TASK-003 WIT files
- [ ] TASK-004 slicer-macros skeleton
- [ ] TASK-005 slicer-test crate
- [ ] TASK-006 slicer-sdk crate

## Phase B — Core Algorithms
- [ ] TASK-010 through TASK-014

## Phase C — Host Scheduler
- [ ] TASK-020 through TASK-036

## Phase D — SDK Tooling
- [ ] TASK-040 through TASK-054

## Phase E — MVP Core Modules & CLI
- [ ] TASK-060, TASK-062, TASK-066, TASK-080 through TASK-083

## Phase F — Post-MVP & Advanced Features
- [ ] TASK-061, TASK-063 through TASK-065, TASK-067 through TASK-079

## Phase G — Pipeline Wiring & WASM Integration
- [ ] TASK-100 through TASK-113

## Phase H — End-to-End Integration & Review
- [ ] TASK-120

## Known Deviations from Architecture Docs
[List any intentional deviations with justification]

## Blocked Tasks
[List tasks blocked on external decisions]
```

---

## Quality Gates

Before any Phase can be considered complete, all gates must pass:

**Architecture acceptance gate (release blocking):**
- Must pass rubric in `./docs/11_operational_governance_and_acceptance_gate.md`
- Categories required: determinism, recoverability, resource bounds, coupling control, compatibility, operability
- Any critical deviation requires explicit waiver by architecture owner and is release-blocking by default

**Phase A gate:**
- `cargo build --workspace` succeeds with zero warnings
- `cargo test --workspace` passes
- All IR structs have serde round-trip tests

**Phase B gate:**
- Slice of a known STL produces known polygon count (golden test)
- Loop chaining produces closed polygons for a manifold mesh
- All geometry helpers have property tests passing

**Phase C gate:**
- Scheduler rejects: claim conflicts, cycles, unfulfilled reads, version mismatches
- Per-layer parallel executor produces identical output to sequential for same input
- Blackboard is not mutated during per-layer execution (verified by Arc + read-only API)
- LayerFinalization executor enforces pool size 1 (verified by test: run with rayon thread count 16, confirm single instance used)
- Synthetic layers inserted by finalization modules appear in correct Z-sorted position in GCodeEmit output

**Phase D gate:**
- `slicer new` + `slicer build` + `slicer test` pipeline works end-to-end
- A scaffolded module compiles to valid WASM component
- Mock host captures all host service calls correctly

**Phase E gate:**
- TASK-080 through TASK-082 execute successfully in one end-to-end CLI run
- File import supports STL/OBJ/3MF and applies admesh-based mesh repair without fatal errors on standard fixtures
- TASK-060, TASK-062, and TASK-066 each pass module-level tests and produce valid staged outputs
- TASK-083 end-to-end benchy slice passes and emits valid G-code

**Phase F gate:**
- All remaining advanced modules/tasks (TASK-061, TASK-063 through TASK-065, TASK-067 through TASK-079) pass their test suites
- Advanced paint/annotation propagation and post-processing behaviors match architecture contracts
- Benchy.stl slices in < 2 seconds on reference machine
- Progress events conform to `./docs/09_progress_events.md`

**Phase G gate:**
- `slicer run` successfully executes end-to-end using real compiled WASM modules
- Wasmtime instance pooling parallel execution completes without errors on complex models
- Python text post-processing operates securely with WASM module outputs

**Phase H gate:**
- A fully sliced `.gcode` of the Benchy STL is produced successfully without errors.
- Visual inspection or G-code analysis of the Benchy output confirms that all enabled modules (perimeters, infill, supports) functioned correctly end-to-end.
- Architecture acceptance gate result is recorded in implementation status
