# Implementation Progress

## Current State
- Phase A: COMPLETE
- Phase B: COMPLETE
- Phase C: COMPLETE
- Phase D: COMPLETE — all tasks (TASK-040 through TASK-058) verified and marked complete.
- Phase E: IN PROGRESS — Starting TASK-070 (layer-planner-default).

## Current Task: TASK-056 — Mesh repair TDD (QA red phase)

### Analysis
TASK-056 writes failing tests for repair.rs, then implements the three repair phases:
1. Degenerate triangle removal (area < 1e-8 sq internal units)
2. Face orientation normalization (flood-fill from most-negative-Z centroid)
3. Open-edge closure (fan-cap boundary loops, skip if > 256 vertices)

### MeshIR Structure (from crates/slicer-ir/src/slice_ir.rs)
- MeshIR { schema_version, objects: Vec<ObjectMesh>, build_volume }
- ObjectMesh { id, mesh: IndexedTriangleSet, transform, config, modifier_volumes, paint_data }
- IndexedTriangleSet { vertices: Vec<Point3>, indices: Vec<u32> }
- Point3 { x: i64, y: i64, z: i64 } — scaled integers, 1 unit = 100nm

### Existing repair.rs Stubs (crates/slicer-helpers/src/repair.rs)
Already has: RepairResult, RepairStats, RepairWarning, RepairError, MAX_REPAIR_CAP_VERTICES=256
Function stub: pub fn repair(_mesh: MeshIR) -> Result<RepairResult, RepairError> with todo!()

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/TriangleMesh.cpp — trianglemesh_repair_on_import()
- OrcaSlicerDocumented/src/libslic3r/TriangleMesh.hpp — its_remove_degenerate_faces(), its_face_neighbors(), its_num_open_edges(), its_flip_triangles()
- OrcaSlicerDocumented/tests/libslic3r/test_indexed_triangle_set.cpp — mesh splitting, non-manifold handling
- OrcaSlicerDocumented/tests/fff_print/test_trianglemesh.cpp — volume calculations, transforms

### Test Plan per docs/13
| Test | Input | Expected |
|------|-------|----------|
| repair_removes_degenerate_triangles | Mesh with 3 zero-area triangles | stats.degenerate_removed == 3 |
| repair_normalizes_flipped_face | Cube with one face winding reversed | stats.faces_reoriented >= 1 |
| repair_closes_open_edge | Cube with one face removed | stats.open_edges_closed > 0 |
| repair_noop_on_clean_mesh | Valid cube mesh | All stats == 0 |
| repair_large_cap_loop_warning | Mesh with 300-vertex open boundary | RepairWarning::LargeCapLoop present |

### Decision: repair() operates on ObjectMesh level
The repair function takes MeshIR which contains Vec<ObjectMesh>. The repair algorithm
should iterate over each ObjectMesh and repair its IndexedTriangleSet independently.
The function signature matches docs/13 exactly: repair(mesh: MeshIR) -> Result<RepairResult, RepairError>.

### QA Red Task Card

## Task: TASK-056 QA Red — mesh repair failing tests

**Role:** QA
**Authoritative docs:** ./docs/13_slicer_helpers_crate.md (sections: Mesh Repair, TDD Contract)
**OrcaSlicer reference:** 
- OrcaSlicerDocumented/src/libslic3r/TriangleMesh.hpp — its_remove_degenerate_faces(), RepairedMeshErrors
- OrcaSlicerDocumented/tests/libslic3r/test_indexed_triangle_set.cpp — non-manifold mesh handling

**Files to create:**
- `crates/slicer-helpers/tests/repair_tdd.rs` (create — 5 failing tests)

**Context:**
Write the 5 failing tests from docs/13 §TDD Contract for repair_tdd.rs. Each test must
construct a MeshIR with a single ObjectMesh containing the relevant defect, call
slicer_helpers::repair(), and assert the expected outcome. Tests must compile but fail
only on the todo!() stub in repair().

The repair function's signature is: pub fn repair(mesh: MeshIR) -> Result<RepairResult, RepairError>

MeshIR must be constructed with:
- schema_version: SemVer { major: 1, minor: 0, patch: 0 }
- objects: vec![ObjectMesh { id, mesh: IndexedTriangleSet { vertices, indices }, transform: identity, config: default, modifier_volumes: vec![], paint_data: None }]
- build_volume: BoundingBox3 with appropriate bounds

Point3 uses i64 scaled integers (1 unit = 100nm, so 1mm = 10_000 units).

Test fixtures needed:
1. repair_removes_degenerate_triangles — 3 zero-area triangles among valid ones
2. repair_normalizes_flipped_face — cube with one face winding reversed
3. repair_closes_open_edge — cube with one face removed (creates 4 open edges)
4. repair_noop_on_clean_mesh — valid closed cube mesh
5. repair_large_cap_loop_warning — mesh with a 300-vertex open boundary loop

**Acceptance criteria:**
- [ ] All 5 tests compile
- [ ] All 5 tests fail with "not yet implemented: TASK-056" (the todo! stub)
- [ ] Test meshes are geometrically correct (valid cube has 8 vertices, 12 triangles)
- [ ] No test uses #[ignore] or #[should_panic] to mask the todo!()
- [ ] Tests import types from slicer_helpers (RepairResult, RepairStats, RepairWarning, etc.)

### CoderAgent Report (QA Red):
```yaml
task_id: TASK-056
status: done (QA red verified)
summary: >
  5 failing TDD tests verified. All compile, all fail on todo! stub.
```

### TASK-056 Coding Green — Implement mesh repair

## Task: TASK-056 Coding Green — mesh repair implementation

**Role:** Coding
**Authoritative docs:** ./docs/13_slicer_helpers_crate.md (sections: Mesh Repair — Algorithm, Output, Public API)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/TriangleMesh.cpp — trianglemesh_repair_on_import(), its_remove_degenerate_faces()
- OrcaSlicerDocumented/src/libslic3r/TriangleMesh.hpp — its_face_neighbors(), its_flip_triangles()

**Files to modify:**
- `crates/slicer-helpers/src/repair.rs` (modify — implement `repair()`)

**Context:**
Implement the three sequential mesh repair phases in the `repair()` function stub.
Point3 uses f32 mm coordinates. The function operates per-ObjectMesh in the MeshIR.

**Phase 1 — Degenerate triangle removal:**
- A triangle is degenerate if `||(v1-v0).cross(v2-v0)||² < 2e-16` (area < 1e-8 sq units)
- Remove degenerate triangles from the index buffer; increment stats.degenerate_removed
- Do this before any other operation

**Phase 2 — Face orientation normalization:**
- Build adjacency: for each edge (v_a, v_b), track which triangles share it
- Seed from the triangle with the most negative Z centroid (deterministic)
- BFS/flood-fill across shared edges; flip winding of any neighbor whose shared-edge 
  winding is inconsistent with the propagation front
- Handle multiple disconnected components (one flood-fill each)
- Increment stats.faces_reoriented for each flipped triangle
- Set stats.components to the number of disconnected components found

**Phase 3 — Open-edge closure:**
- An open edge is referenced by exactly one triangle
- Collect open edges, group into boundary loops by shared vertex
- Cap each loop with a fan of triangles from the loop centroid
- If a loop has > MAX_REPAIR_CAP_VERTICES (256) vertices, emit RepairWarning::LargeCapLoop
  and skip that loop
- Increment stats.open_edges_closed by the number of edges closed

**Acceptance criteria:**
- [ ] All 5 tests in `cargo test -p slicer-helpers --test repair_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] No new dependencies added (pure algorithmic implementation)
- [ ] The repair function processes each ObjectMesh independently
- [ ] RepairStats fields are correctly populated

### CoderAgent Report (Coding Green):
```yaml
task_id: TASK-056
status: done
summary: >
  Implemented mesh repair() with 3 sequential phases: (1) degenerate triangle
  removal using cross-product squared magnitude < 2e-16 threshold, (2) face
  orientation normalization via BFS flood-fill seeded from most-negative-Z
  centroid with multi-component support, (3) open-edge closure via fan-capping
  boundary loops with >256 vertex skip + LargeCapLoop warning. All 5 TDD tests
  pass, workspace builds clean, clippy clean.
files_changed:
  - crates/slicer-helpers/src/repair.rs
tests_added_or_updated: []
risks:
  - none
notes_for_tester:
  - All 5 repair_tdd tests pass
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-056 marked complete
```

## Current Task: TASK-057 — Mesh decimation TDD (QA red + Coding green)

### Analysis
TASK-057 writes 6 failing tests for decimate.rs, then implements QEM decimation via meshopt.

### Existing Stubs (crates/slicer-helpers/src/decimate.rs)
- DecimateConfig { target_count, target_ratio, max_error, aggressive }
- DecimateResult { mesh, original_triangle_count, final_triangle_count, achieved_error }
- DecimateError { EmptyMesh, InvalidConfig(String), IoError }
- Function stub: pub fn decimate(_mesh: MeshIR, _config: DecimateConfig) -> Result<DecimateResult, DecimateError> with todo!()

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/QuadricEdgeCollapse.cpp — Garland-Heckbert QEM
- OrcaSlicerDocumented/src/libslic3r/QuadricEdgeCollapse.hpp — SymMat, TriangleInfo

### TDD Contract (docs/13)
| Test | Input | Expected |
|------|-------|----------|
| decimate_by_ratio | Sphere 2000 tris, target_ratio=0.5 | Output ≤ 1000 tris |
| decimate_by_count | Sphere 2000 tris, target_count=400 | Output ≤ 400 tris |
| decimate_respects_error_budget | Sphere, max_error=0.001 | achieved_error ≤ 0.001 |
| decimate_stops_early | Sphere, target_ratio=0.01, max_error=0.001 | Stops early (more tris than target) |
| decimate_empty_mesh_error | Empty MeshIR | Err(DecimateError::EmptyMesh) |
| decimate_conflict_config_error | Both target_count and target_ratio | Err(DecimateError::InvalidConfig) |

### Algorithm (from docs/13)
1. Convert MeshIR vertices/indices to meshopt f32 vertex buffer + u32 index buffer
2. Call meshopt::simplify (or simplify_sloppy if aggressive) with target_count and target_error
3. Reconstruct MeshIR from simplified buffers
4. Run Phase 2 (orientation normalization) from repair module to correct winding

### QA Red Task Card

## Task: TASK-057 QA Red — mesh decimation failing tests

**Role:** QA
**Authoritative docs:** ./docs/13_slicer_helpers_crate.md (sections: Mesh Decimation, TDD Contract)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/QuadricEdgeCollapse.hpp — QEM algorithm reference

**Files to create:**
- `crates/slicer-helpers/tests/decimate_tdd.rs` (create — 6 failing tests)

**Context:**
Write 6 failing tests from docs/13 §TDD Contract for decimate_tdd.rs. Tests must construct
MeshIR meshes (UV sphere with ~2000 triangles for geometric tests, empty for error tests),
call slicer_helpers::decimate(), and assert expected outcomes. Tests must compile but fail
only on the todo!() stub.

Sphere mesh generation: Create a UV sphere programmatically with lat/lon subdivision
to get ~2000 triangles. Use i64 scaled integer coordinates (1 unit = 100nm, 1mm = 10_000 units).

**Test fixtures:**
1. decimate_by_ratio — Sphere ~2000 tris, target_ratio=0.5 → output ≤ 1000 tris
2. decimate_by_count — Sphere ~2000 tris, target_count=400 → output ≤ 400 tris
3. decimate_respects_error_budget — Sphere, max_error=0.001 → achieved_error ≤ 0.001
4. decimate_stops_early — Sphere, target_ratio=0.01, max_error=0.001 → final_count > target (stopped early)
5. decimate_empty_mesh_error — Empty MeshIR → Err(DecimateError::EmptyMesh)
6. decimate_conflict_config_error — Both target_count and target_ratio set → Err(DecimateError::InvalidConfig)

**Acceptance criteria:**
- [ ] All 6 tests compile
- [ ] All 6 tests fail with "not yet implemented: TASK-057" (the todo! stub)
- [ ] Sphere mesh has ~2000 triangles (valid geometry)
- [ ] No test uses #[ignore] or #[should_panic]
- [ ] Tests import types from slicer_helpers

### Coding Green Task Card

## Task: TASK-057 Coding Green — mesh decimation implementation

**Role:** Coding
**Authoritative docs:** ./docs/13_slicer_helpers_crate.md (sections: Mesh Decimation — Algorithm, Output, Public API)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/QuadricEdgeCollapse.cpp — QEM implementation

**Files to modify:**
- `crates/slicer-helpers/src/decimate.rs` (modify — implement `decimate()`)

**Context:**
Implement the mesh decimation function using meshopt crate.

**Steps:**
1. Validate config: exactly one of target_count/target_ratio must be set; reject empty mesh
2. Convert MeshIR to flat f32 vertex positions buffer and u32 index buffer
3. Compute target_count from target_ratio if needed
4. Call meshopt::simplify (or simplify_sloppy if aggressive) with target_count and max_error
5. Reconstruct MeshIR from simplified buffers (convert f32 back to i64)
6. Report achieved_error from meshopt result
7. Handle "stopped early" case: if final_count > target due to error budget

**Acceptance criteria:**
- [ ] All 6 tests in `cargo test -p slicer-helpers --test decimate_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] Uses meshopt crate (no custom QEM implementation)

### CoderAgent Report (TASK-057):
```yaml
task_id: TASK-057
status: done
summary: >
  Implemented mesh decimation via meshopt QEM. Created 6 TDD tests (QA red),
  then implemented decimate() with config validation, per-object proportional
  target distribution, meshopt::simplify_decoder / simplify_sloppy_decoder
  dispatch, vertex compaction, and achieved-error tracking. All 6 tests pass,
  workspace builds clean, clippy clean.
files_changed:
  - crates/slicer-helpers/src/decimate.rs
tests_added_or_updated:
  - crates/slicer-helpers/tests/decimate_tdd.rs
risks:
  - none
notes_for_tester:
  - All 6 decimate_tdd tests pass
notes_for_docs:
  - docs/07_implementation_status.md TASK-057 marked complete
```

## Current Task: TASK-058 — STEP import TDD (QA red + Coding green)

### Analysis
TASK-058 writes 8 failing tests for import_step_tdd.rs, creates STEP test fixture files,
then implements import/step.rs using truck-stepio + truck-meshalgo.

### Existing Stubs (crates/slicer-helpers/src/import/step.rs)
- StepImportResult { meshes: Vec<NamedMesh>, source_unit: StepLengthUnit, warnings: Vec<StepWarning> }
- NamedMesh { name: Option<String>, mesh: MeshIR }
- StepLengthUnit { Millimetre, Metre, Inch, Micrometre, Unknown }
- StepWarning { UnsupportedSchema, UnknownUnit, RepairApplied, MultipleComponents }
- StepImportError { FileNotFound, ParseError, NoGeometry, IoError }
- Function stub: pub fn import_step(_path: &Path) -> Result<StepImportResult, StepImportError> with todo!()

### truck API (from crate source inspection)
- `ruststep::parser::parse(&step_string)` → `Exchange` with `.data[0]` DataSection
- `Table::from_data_section(&exchange.data[0])` or `Table::from_step(&step_string)` → Table
- `table.shell` is a map of shell entities
- `table.to_compressed_shell(shell)` → CompressedShell
- `shell.robust_triangulation(tolerance)` → meshed CompressedShell
- `.to_polygon()` → PolygonMesh with `.positions()` (Vec<Point3 f64>) and `.tri_faces()` (Vec<[StandardVertex; 3]>)
- StandardVertex has `.pos` (usize index into positions)
- `poly.put_together_same_attrs(tol).remove_degenerate_faces().remove_unused_attrs()`
- truck re-exports: `truck_stepio::r#in::*` and `truck_meshalgo::prelude::*`
- ruststep is re-exported: `truck_stepio::r#in::ruststep`

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Format/STEP.hpp — StepPreProcessor, Step class
- OrcaSlicerDocumented/src/libslic3r/Format/STEP.cpp — STEPCAFControl_Reader, BRepMesh_IncrementalMesh

### Test fixtures needed (from docs/13)
Must be valid ISO 10303-21 text files in tests/resources/:
- cube.step — single 10mm cube, mm units (AP203/AP214)
- cube_metres.step — same cube, metre units
- assembly.step — two distinct solids
- step_open_face.step — STEP whose tessellation produces non-manifold mesh

### TDD Contract (docs/13)
| Test | Input | Expected |
|------|-------|----------|
| import_step_single_solid | cube.step (mm) | 1 mesh, vertices in internal units |
| import_step_unit_metre | cube_metres.step | Vertices × 10,000,000 vs mm |
| import_step_multi_solid | assembly.step (2 solids) | result.meshes.len() == 2 |
| import_step_merge_components | assembly.step, merge=true | 1 mesh, combined |
| import_step_repair_applied | step_open_face.step | StepWarning::RepairApplied present |
| import_step_unknown_unit_warning | STEP with no unit | StepWarning::UnknownUnit, defaults mm |
| import_step_not_found_error | non-existent path | Err(StepImportError::FileNotFound) |
| import_step_invalid_file_error | binary garbage | Err(StepImportError::ParseError) |

### Note: merge_components
The documented API `import_step(path)` has no merge parameter. Test #4 needs either:
(a) a separate `merge_meshes(StepImportResult) -> StepImportResult` function, or
(b) an options struct. Going with (a) — a simple helper — to keep the primary API clean.

### QA Red + Coding Green Task Card

## Task: TASK-058 — STEP import (QA red + Coding green)

**Role:** Coding (combined QA red + green)
**Authoritative docs:** ./docs/13_slicer_helpers_crate.md (sections: STEP Import, TDD Contract)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/Format/STEP.hpp — Step class, STEPCAFControl_Reader
- OrcaSlicerDocumented/src/libslic3r/Format/STEP.cpp — BRepMesh tessellation pipeline

**Files to create:**
- `crates/slicer-helpers/tests/resources/cube.step` — minimal AP203 B-Rep cube, 10mm, mm units
- `crates/slicer-helpers/tests/resources/cube_metres.step` — same cube, metre units (0.01m)
- `crates/slicer-helpers/tests/resources/assembly.step` — 2 distinct solids
- `crates/slicer-helpers/tests/resources/step_open_face.step` — non-manifold tessellation result
- `crates/slicer-helpers/tests/import_step_tdd.rs` — 8 failing tests

**Files to modify:**
- `crates/slicer-helpers/src/import/step.rs` — implement import_step()
- `crates/slicer-helpers/src/import/mod.rs` — add merge utility if needed

**Context:**
Write 8 TDD tests from docs/13 §TDD Contract, create STEP fixture files, then implement
the import_step() function using truck-stepio for parsing and truck-meshalgo for tessellation.

### STEP fixture generation approach
STEP files are complex B-Rep ISO 10303-21 text. For test fixtures, use truck-stepio's
output module to generate them programmatically. Add truck-modeling as a dev-dependency
if needed, or write the STEP text by hand for simple primitives. Alternatively, generate 
fixtures from a build script or test helper that writes them to the test resources dir.

The simplest approach: write hand-crafted minimal STEP files. A cube in STEP AP203 format
is approximately 60-100 lines. The key entities are:
- CLOSED_SHELL with 6 ADVANCED_FACEs
- Each face has an OUTER_BOUNDARY (EDGE_LOOP)
- Edges reference VERTEX_POINTs with CARTESIAN_POINTs
- Surfaces are PLANEs

### truck API usage for implementation
```rust
use truck_stepio::r#in::*;
use truck_meshalgo::prelude::*;

let step_string = std::fs::read_to_string(path)?;
let table = Table::from_step(&step_string).ok_or(StepImportError::ParseError(...))?;

for (idx, step_shell) in table.shell.iter() {
    let cshell = table.to_compressed_shell(shell)?;
    let mut poly = cshell.robust_triangulation(tolerance).to_polygon();
    poly.put_together_same_attrs(TOLERANCE * 50.0)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    
    let positions = poly.positions(); // Vec<Point3> (f64)
    let tri_faces = poly.tri_faces(); // Vec<[StandardVertex; 3]>
    // Convert positions to internal units, build IndexedTriangleSet
}
```

### Unit detection
Parse STEP header for LENGTH_UNIT/SI_UNIT entities:
- Look for `LENGTH_MEASURE` or `SI_UNIT` with `.MILLI.` → Millimetre
- `.METRE.` or no prefix → Metre
- If CONVERSION_BASED_UNIT with 'INCH' → Inch
- `.MICRO.` → Micrometre
- None found → Unknown (emit warning, default to mm)

### merge_meshes helper
Add a public utility function:
```rust
pub fn merge_step_meshes(result: StepImportResult) -> StepImportResult
```
This merges all NamedMesh entries into a single NamedMesh by concatenating vertices and
offsetting indices.

### Acceptance criteria
- [ ] All 8 tests in `cargo test -p slicer-helpers --test import_step_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] STEP fixture files are valid ISO 10303-21
- [ ] Unit conversion matches docs/13 table (mm→×10000, m→×10000000, in→×254000, µm→×10)
- [ ] Repair (Phase 1 + Phase 2) applied automatically to each component
- [ ] StepWarning::RepairApplied emitted when repair does actual work
- [ ] StepWarning::UnknownUnit emitted when no unit found
- [ ] StepImportError::FileNotFound for missing files
- [ ] StepImportError::ParseError for invalid files

### CoderAgent Report (TASK-058):
```yaml
task_id: TASK-058
status: done
summary: >
  Implemented STEP import via truck-stepio + truck-meshalgo. Created 8 TDD tests
  covering single solid, unit detection (mm/metre), multi-solid assembly, merge
  components, repair applied, unknown unit warning, file not found, and invalid
  file errors. Generated STEP fixture files from truck-modeling primitives.
  Implemented import_step() with two-pass tessellation (coarse bounding box then
  refined), unit detection from STEP text scanning, automatic repair pass, and
  merge_step_meshes() utility. All 8 tests pass, workspace builds clean,
  clippy clean.
files_changed:
  - crates/slicer-helpers/src/import/step.rs
  - crates/slicer-helpers/src/lib.rs
  - crates/slicer-helpers/Cargo.toml
  - Cargo.toml
  - Cargo.lock
tests_added_or_updated:
  - crates/slicer-helpers/tests/import_step_tdd.rs
  - crates/slicer-helpers/tests/step_fixtures/mod.rs
  - crates/slicer-helpers/tests/resources/*.step
risks:
  - none
notes_for_tester:
  - All 8 import_step_tdd tests pass
  - truck-modeling added as dev-dependency only
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-058 marked complete
```

## Current Task: TASK-070 — Default Layer Planner (Phase E)

### Analysis
TASK-070 creates `modules/core-modules/layer-planner-default/`, the first core module.
It implements the PrepassModule trait (TASK-043) for the PrePass::LayerPlanning stage.

This module computes global Z-plane sequences from object mesh heights and layer-height config.
It's the MVP layer planner — uniform layer heights, multi-object LCM sync, catch-up layers.

### Key Types (already exist in slicer-ir)
- LayerPlanIR { schema_version, global_layers: Vec<GlobalLayer>, object_participation: HashMap<ObjectId, Vec<ObjectLayerRef>> }
- GlobalLayer { index, z, active_regions: Vec<ActiveRegion>, has_nonplanar, is_sync_layer }
- ActiveRegion { object_id, region_id, resolved_config, effective_layer_height, nonplanar_shell, is_catchup_layer, catchup_z_bottom, tool_index }
- ObjectLayerRef { local_layer_index, global_layer_index, effective_layer_height }
- ResolvedConfig { layer_height, first_layer_height, wall_count, ... } (full fields in slice_ir.rs:446-503)

### SDK Types (already exist in slicer-sdk)
- PrepassModule trait: on_print_start(config) -> Result<Self, ModuleError>, run_layer_planning(objects, output, config) -> Result<(), ModuleError>
- LayerPlanOutput builder: push_layer(LayerProposal) -> Result<(), String>
- LayerProposal { z: f32, active_regions: Vec<RegionLayerProposal> }
- RegionLayerProposal { object_id, region_id, effective_layer_height, is_catchup, catchup_z_bottom }
- ConfigView from slicer_ir

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Slicing.cpp — generate_object_layers() line 1010-1086
- OrcaSlicerDocumented/src/libslic3r/Slicing.hpp — SlicingParameters struct
- OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp — new_layers() line 63-86

### Module Structure
```
modules/core-modules/layer-planner-default/
  Cargo.toml
  src/lib.rs           — DefaultLayerPlanner struct implementing PrepassModule
  tests/
    layer_planning_tdd.rs — TDD test suite
```

### Algorithm (MVP — uniform layers)
1. Read layer_height and first_layer_height from config
2. For each object: compute object Z range from mesh bounding box
3. Generate layer sequence: first_layer_height, then layer_height increments up to max Z
4. For multi-object: compute LCM sync interval, merge Z-plane sequences
5. Mark sync layers where objects with different heights align
6. Generate catch-up layers for objects that skip intermediate global layers
7. Push each GlobalLayer proposal to LayerPlanOutput

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| single_object_uniform_layers | 1 object, 2mm tall, layer_height=0.2 | 10 layers, ascending Z, layer 0 has first_layer_height |
| first_layer_height_respected | 1 object, first_layer=0.3, rest=0.2 | Layer 0 z=0.3, layer 1 z=0.5, ... |
| multi_object_same_height | 2 objects, same layer_height=0.2 | Both participate in all layers, no sync layers |
| multi_object_lcm_sync | Object A 0.2mm, Object B 0.3mm | Sync at 0.6mm multiples, catch-up layers present |
| catch_up_layer_fields | Object with catch-up | is_catchup_layer=true, catchup_z_bottom correct |
| empty_objects_error | No objects | Error returned |
| zero_layer_height_error | layer_height=0 | Error returned |
| object_participation_map | 1 object, 2mm tall | object_participation has correct local/global index mapping |

### QA Red Task Card

## Task: TASK-070 QA Red — layer planner failing tests

**Role:** QA
**Authoritative docs:** ./docs/02_ir_schemas.md (LayerPlanIR, GlobalLayer, ActiveRegion), ./docs/03_wit_and_manifest.md (world-prepass.wit), ./docs/05_module_sdk.md
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/Slicing.cpp lines 1010-1086 — generate_object_layers()
- OrcaSlicerDocumented/src/libslic3r/Slicing.hpp lines 76-160 — SlicingParameters

**Files to create:**
- `modules/core-modules/layer-planner-default/Cargo.toml` — library crate depending on slicer-sdk, slicer-ir
- `modules/core-modules/layer-planner-default/src/lib.rs` — DefaultLayerPlanner struct with todo!() stub in run_layer_planning
- `modules/core-modules/layer-planner-default/tests/layer_planning_tdd.rs` — 8 failing tests

**Context:**
Write 8 failing tests from the TDD contract. Tests construct ConfigView with layer height settings,
create object ID lists, call run_layer_planning via the PrepassModule trait, and assert expected
layer sequences. The module struct should have a todo!("TASK-070") stub so tests compile but fail.

The module does NOT need WASM compilation for these tests — it runs natively via the SDK trait.

**Acceptance criteria:**
- [ ] All 8 tests compile
- [ ] All 8 tests fail with "not yet implemented: TASK-070"
- [ ] Module crate builds as part of workspace
- [ ] No test uses #[ignore] or #[should_panic]

### Coding Green Task Card

## Task: TASK-070 Coding Green — layer planner implementation

**Role:** Coding
**Authoritative docs:** ./docs/02_ir_schemas.md (LayerPlanIR), ./docs/01_system_architecture.md (PrePass::LayerPlanning)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/Slicing.cpp — generate_object_layers() lines 1010-1086

**Files to modify:**
- `modules/core-modules/layer-planner-default/src/lib.rs` — implement run_layer_planning

**Context:**
Implement the default layer planner:
1. Read layer_height and first_layer_height from ConfigView
2. For each object: determine object height from config/mesh bounds (passed via config or hardcoded for MVP)
3. Generate global Z-plane sequence with first_layer_height, then uniform layer_height
4. For multi-object: merge sequences, mark sync layers at LCM intervals
5. Generate catch-up layers for objects that skip global Zs
6. Push LayerProposal entries to output
7. Error on empty objects or invalid config (zero/negative layer height)

**Acceptance criteria:**
- [ ] All 8 tests in `cargo test -p layer-planner-default --test layer_planning_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean

### CoderAgent Report (TASK-070):
```yaml
task_id: TASK-070
status: done
summary: >
  Created modules/core-modules/layer-planner-default/ crate implementing PrepassModule
  trait for LayerPlanning stage. DefaultLayerPlanner reads layer_height, first_layer_height,
  and per-object height/layer-height from ConfigView. For same-height objects, generates
  simple merged Z-plane sequence. For different layer heights, generates union of all native
  Z-planes with catch-up layers at every global Z for non-native objects (effective_layer_height =
  z - catchup_z_bottom). 8 TDD tests covering single/multi-object uniform layers, first_layer_height,
  LCM sync with catch-up fields, error cases, and participation mapping. All pass.
files_changed:
  - Cargo.toml (added workspace member)
  - modules/core-modules/layer-planner-default/Cargo.toml
  - modules/core-modules/layer-planner-default/src/lib.rs
tests_added_or_updated:
  - modules/core-modules/layer-planner-default/tests/layer_planning_tdd.rs (8 tests)
risks:
  - none
notes_for_tester:
  - All 8 layer_planning_tdd tests pass
  - 1 unit test (on_print_start_defaults) also passes
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-070 marked complete
```

## Current Task: TASK-071 — Classic Perimeters Module (Phase E)

### Analysis
TASK-071 creates `modules/core-modules/classic-perimeters/`, implementing the LayerModule trait
for the `Layer::Perimeters` stage. It generates wall loops from slice contour polygons via
iterative Clipper2 polygon insets (negative offsets).

### Key SDK Types
- **LayerModule::run_perimeters(layer_index, regions: &[SliceRegionView], paint: &PaintRegionLayerView, output: &mut PerimeterOutputBuilder, config: &ConfigView)** — the entry point
- **SliceRegionView** — provides polygons(), object_id(), region_id(), z(), effective_layer_height()
- **PerimeterOutputBuilder** — push_wall_loop(WallLoop), set_infill_areas(Vec<ExPolygon>), push_seam_candidate(Point3, f32)
- **ConfigView** — fields HashMap<ConfigKey, ConfigValue> with wall_count, line_width, outer_wall_speed, inner_wall_speed
- **WallLoop** — perimeter_index, loop_type, path: ExtrusionPath3D, width_profile: WidthProfile, feature_flags: Vec<WallFeatureFlags>, boundary_type: WallBoundaryType
- **ExtrusionPath3D** — points: Vec<Point3WithWidth>, role: ExtrusionRole, speed_factor: f32
- **Point3WithWidth** — x, y, z, width (all f32, in mm)
- **slicer_core::polygon_ops::offset()** — already exists for Clipper2 polygon insets

### Algorithm (Classic — constant-width insets)
Per OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp process_classic():
1. Start with slice polygons from SliceRegionView
2. For i in 0..wall_count:
   - Inset polygon by -(line_width/2) for i=0 (outer), -(line_width) for i>0 (inner)
   - Convert inset result to WallLoop with correct perimeter_index, loop_type, boundary_type
   - Extract path as ExtrusionPath3D at the layer Z
3. After all walls: remaining area = infill_areas (inset of innermost wall by line_width/2)
4. Seam candidates: score concave corners of outer wall path

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| single_square_two_walls | 10mm square, wall_count=2, line_width=0.4 | 2 wall loops (outer+inner), infill area smaller |
| outer_wall_is_index_zero | any polygon, wall_count>=1 | walls[0].perimeter_index==0, loop_type==Outer |
| inner_walls_correct_type | square, wall_count=3 | walls[1..].loop_type==Inner, ascending perimeter_index |
| infill_area_computed | square, wall_count=2 | infill_areas non-empty, smaller than input |
| empty_polygon_no_output | empty polygons | 0 wall loops, 0 infill areas |
| wall_count_zero | wall_count=0 | 0 wall loops, infill_areas == input polygons |
| seam_candidates_generated | square, wall_count>=1 | at least 1 seam candidate on outer wall |
| speed_factor_from_config | outer_wall_speed=30, inner_wall_speed=60 | outer speed_factor=30/50, inner=60/50 (normalized) |

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp — process_classic() iterative insets
- OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.hpp — PerimeterGeneratorLoop tree

### QA Red + Coding Green Task Card

## Task: TASK-071 Classic Perimeters (QA red + Coding green)

**Role:** Coding (combined QA red + green)
**Authoritative docs:** ./docs/01_system_architecture.md (Layer::Perimeters), ./docs/02_ir_schemas.md (PerimeterIR, WallLoop), ./docs/05_module_sdk.md (LayerModule trait)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp — process_classic() iterative polygon insets
- OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.hpp — PerimeterGeneratorLoop declarations

**Files to create:**
- `modules/core-modules/classic-perimeters/Cargo.toml` — library crate depending on slicer-sdk, slicer-ir, slicer-core
- `modules/core-modules/classic-perimeters/src/lib.rs` — ClassicPerimeters struct implementing LayerModule
- `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs` — 8 TDD tests

**Files to modify:**
- `Cargo.toml` — add workspace member

**Context:**
Classic perimeters generates wall loops from slice contour polygons using iterative negative
polygon offsets (Clipper2). For each region on each layer, it reads wall_count and line_width
from config, performs N insets to produce N WallLoops (outer first, then inner), computes the
remaining infill area, and generates seam candidates at concave corners of the outer wall.

Uses existing slicer_core::polygon_ops::offset() for Clipper2 insets. ExPolygon contour points
are in scaled integers (10_000 units/mm), but ExtrusionPath3D points use f32 mm coordinates.
The module must convert between coordinate systems.

**Algorithm:**
1. on_print_start: read wall_count, line_width from config
2. run_perimeters: for each region's polygons:
   a. Outer wall: inset by -line_width/2 → WallLoop{perimeter_index:0, loop_type:Outer}
   b. Inner walls (i=1..wall_count): inset previous by -line_width → WallLoop{perimeter_index:i, loop_type:Inner}
   c. Infill area: inset innermost wall by -line_width/2
   d. Convert ExPolygon contour points (scaled i64) to ExtrusionPath3D points (f32 mm) at layer Z
   e. Set width_profile.widths to line_width for all vertices
   f. Set feature_flags to default (no paint, no bridge, no thin wall) for all vertices
   g. Push seam candidates at concave corners (inner angle < 180°) of outer wall

**Key type conversions:**
- Polygon Point2 (i64 scaled) → Point3WithWidth (f32 mm): x = units_to_mm(p.x), y = units_to_mm(p.y), z = region.z(), width = line_width
- WallLoop.boundary_type: Outer wall = ExteriorSurface, Inner walls = Interior
- ExtrusionRole: OuterWall for index 0, InnerWall for index > 0
- speed_factor: outer_wall_speed / 50.0 for outer, inner_wall_speed / 50.0 for inner (normalized to base 50mm/s)

**Acceptance criteria:**
- [ ] All 8 tests in `cargo test -p classic-perimeters --test classic_perimeters_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] Wall loops have correct perimeter_index ordering (0=outer, 1..N=inner)
- [ ] Infill areas computed as inset of innermost wall
- [ ] Coordinates converted correctly between scaled i64 and f32 mm
- [ ] Empty input produces empty output (no panic)

### CoderAgent Report (TASK-071):
```yaml
task_id: TASK-071
status: done
summary: >
  Created modules/core-modules/classic-perimeters/ crate implementing LayerModule
  trait for Layer::Perimeters stage. ClassicPerimeters reads wall_count, line_width,
  outer_wall_speed, inner_wall_speed from ConfigView. Generates wall loops via iterative
  Clipper2 polygon insets: outer wall at -line_width/2, inner walls at -line_width each.
  Infill area computed as inset of innermost wall by -line_width/2. Seam candidates
  generated at all sharp corners with concave corners scoring higher. Coordinates converted
  from scaled i64 (Point2) to f32 mm (Point3WithWidth). Speed factors normalized to base 50mm/s.
  8 TDD tests covering single/multi-wall generation, infill area computation, empty input,
  wall_count=0, seam candidates, speed factors, and boundary types. All pass.
files_changed:
  - Cargo.toml (added workspace member)
  - modules/core-modules/classic-perimeters/Cargo.toml
  - modules/core-modules/classic-perimeters/src/lib.rs
tests_added_or_updated:
  - modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs (8 tests)
risks:
  - none
notes_for_tester:
  - All 8 classic_perimeters_tdd tests pass
  - 1 unit test (on_print_start_defaults) also passes
  - Pre-existing clippy warnings in slicer-core (not from this change)
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-071 marked complete
```

## Current Task: TASK-072 — Rectilinear Infill Module (Phase E)

### Analysis
TASK-072 creates `modules/core-modules/rectilinear-infill/`, implementing the LayerModule trait
for the `Layer::Infill` stage. It generates sparse infill fill lines using a scan-line approach:
parallel lines at computed spacing, clipped to infill area boundaries, with per-layer angle rotation.

### Key SDK Types
- **LayerModule::run_infill(layer_index, regions: &[SliceRegionView], output: &mut InfillOutputBuilder, config: &ConfigView)**
- **SliceRegionView** — provides infill_areas() -> &[ExPolygon], z(), object_id(), region_id()
- **InfillOutputBuilder** — push_sparse_path(ExtrusionPath3D)
- **ExtrusionPath3D** — points: Vec<Point3WithWidth>, role: ExtrusionRole, speed_factor: f32
- **ExtrusionRole::SparseInfill** for sparse infill paths
- **ConfigView.fields** — infill_density (Float, 0.0-1.0), infill_angle (Float, degrees), infill_speed (Float, mm/s)
- **ExPolygon** — contour: Polygon (CCW), holes: Vec<Polygon> (CW), Polygon.points: Vec<Point2>
- **Point2** — {x: i64, y: i64} in scaled integers (10_000 units/mm), with from_mm/to_mm
- **slicer_ir::mm_to_units(), units_to_mm()** — coordinate conversion

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp — fill_surface_by_lines() lines 2966-3143
- OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp — slice_region_by_vertical_lines() lines 828-999
- OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp — _layer_angle() lines 368-373
- OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp — _infill_direction() lines 342-393

### Algorithm (MVP — rectilinear scan lines)
1. Read infill_density, infill_angle, infill_speed, line_width from config
2. If density == 0 or no infill_areas, return Ok(()) (no infill)
3. Compute line_spacing_mm = line_width / density
4. Compute angle = infill_angle + layer_rotation (0° on even layers, 90° on odd)
5. For each region's infill_areas (each ExPolygon):
   a. Rotate all polygon points by -angle (work in rotated space)
   b. Compute bounding box of rotated polygon
   c. Generate horizontal scan lines at line_spacing intervals across bbox
   d. For each scan line Y: intersect with polygon edges, sort X intersections, pair enter/exit
   e. Rotate line segment endpoints back by +angle
   f. Convert to Point3WithWidth at layer Z with line_width
   g. Create ExtrusionPath3D with role=SparseInfill, speed_factor=infill_speed/50.0
6. Push each path to output via push_sparse_path()

### Key Details
- All polygon operations in scaled i64 coordinates (10_000 units/mm)
- Scan line intersection: for each edge (p1→p2), if y is between p1.y and p2.y, compute x = p1.x + (y - p1.y) * (p2.x - p1.x) / (p2.y - p1.y)
- Must handle holes: hole edges produce exit/enter pairs
- Sort all intersections by x, pair them: [0,1], [2,3], ... → line segments
- Rotation: x' = x*cos - y*sin, y' = x*sin + y*cos (in scaled coords, use f64 for precision)
- Final coordinates converted to f32 mm via units_to_mm()
- Speed factor normalized to base 50mm/s (same pattern as classic-perimeters)

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| single_square_sparse_fill | 10mm square infill area, density=0.2, angle=0, line_width=0.4 | Multiple parallel lines with spacing ~2mm |
| density_affects_line_count | Square, density=0.5 vs 0.2 | Higher density produces more lines |
| angle_rotation_45 | Square, angle=45° | Lines oriented diagonally |
| layer_alternation | Layer 0 vs layer 1, angle=0 | Lines rotated 90° between layers |
| empty_infill_areas | No infill areas | 0 paths output |
| zero_density_no_output | density=0 | 0 paths output |
| extrusion_role_is_sparse | Any valid input | All paths have role=SparseInfill |
| speed_factor_from_config | infill_speed=100 | speed_factor=100/50=2.0 |

### QA Red + Coding Green Task Card

## Task: TASK-072 Rectilinear Infill (QA red + Coding green)

**Role:** Coding (combined QA red + green)
**Authoritative docs:** ./docs/01_system_architecture.md (Layer::Infill), ./docs/02_ir_schemas.md (InfillIR, InfillRegion), ./docs/05_module_sdk.md (LayerModule trait)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp — fill_surface_by_lines(), scan-line intersection
- OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp — _layer_angle() per-layer rotation

**Files to create:**
- `modules/core-modules/rectilinear-infill/Cargo.toml` — library crate depending on slicer-sdk, slicer-ir
- `modules/core-modules/rectilinear-infill/src/lib.rs` — RectilinearInfill struct implementing LayerModule
- `modules/core-modules/rectilinear-infill/tests/rectilinear_infill_tdd.rs` — 8 TDD tests

**Files to modify:**
- `Cargo.toml` — add workspace member

**Acceptance criteria:**
- [ ] All 8 tests in `cargo test -p rectilinear-infill --test rectilinear_infill_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] Lines have correct spacing proportional to 1/density
- [ ] Angle rotation applied correctly per layer
- [ ] Coordinates converted correctly between scaled i64 and f32 mm
- [ ] Empty/zero-density input produces empty output (no panic)

### CoderAgent Report (TASK-072):
```yaml
task_id: TASK-072
status: done
summary: >
  Rectilinear infill module complete. 8/8 TDD tests pass. Workspace builds clean.
```

## Current Task: TASK-073 — Traditional Support Module (Phase E)

### Analysis
TASK-073 creates `modules/core-modules/traditional-support/`, implementing the LayerModule trait
for the `Layer::Support` stage. It generates support material paths for overhanging areas using
a rectilinear scan-line fill pattern, similar to the infill module but for support structures.

### Key SDK Types
- **LayerModule::run_support(layer_index, regions: &[SliceRegionView], paint: &PaintRegionLayerView, output: &mut SupportOutputBuilder, config: &ConfigView)** — entry point
- **SupportOutputBuilder** — push_support_path(ExtrusionPath3D), push_interface_path(ExtrusionPath3D, bool), push_raft_path(ExtrusionPath3D)
- **SliceRegionView** — polygons(), infill_areas(), z(), object_id(), region_id()
- **PaintRegionLayerView** — layer_index() (placeholder for enforcer/blocker data)
- **ConfigView.fields** — support_enabled (Bool), support_density (Float, 0-1), support_speed (Float, mm/s), support_angle (Float, degrees), line_width (Float, mm), support_interface_layers (Int)
- **ExtrusionRole::SupportMaterial** for base support, **ExtrusionRole::SupportInterface** for interface layers
- **ExtrusionPath3D** — points: Vec<Point3WithWidth>, role: ExtrusionRole, speed_factor: f32

### Algorithm (MVP — support fill from polygon areas)
For MVP, the module operates as a fill-pattern generator for support regions:
1. on_print_start: read support_enabled, support_density, support_speed, line_width, support_angle, support_interface_layers from config
2. If support_enabled == false, return Ok(()) immediately
3. For each region: use polygons() as the areas to generate support fill
   (In the full pipeline, these would be pre-computed support areas from a prepass)
4. Generate rectilinear scan lines within the support areas:
   a. Compute line_spacing = line_width / support_density
   b. Apply angle rotation (support_angle + 90° alternation per layer)
   c. Scan-line intersection with polygon boundaries
   d. Create ExtrusionPath3D with role=SupportMaterial, speed_factor=support_speed/50.0
5. Push paths via push_support_path()
6. Interface layer detection: if support_interface_layers > 0, top layers use SupportInterface role
   (For MVP: treat all layers as base support; interface detection requires multi-layer context)

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp — detect_overhangs, project_support_to_grid
- OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.hpp/cpp — toolpath generation
- OrcaSlicerDocumented/src/libslic3r/Support/SupportLayer.hpp — support layer data structures

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| support_disabled_no_output | support_enabled=false, valid regions | 0 paths |
| single_region_generates_support | enabled, 10mm square region, density=0.2 | Multiple support paths generated |
| extrusion_role_is_support_material | enabled, valid region | All paths have role=SupportMaterial |
| speed_factor_from_config | support_speed=80 | speed_factor=80/50=1.6 |
| density_affects_line_count | density=0.5 vs 0.2 | Higher density = more lines |
| alternating_angle | layer 0 vs layer 1, angle=0 | Lines rotated 90° between layers |
| empty_regions_no_output | enabled but empty polygons | 0 paths |
| zero_density_no_output | density=0 | 0 paths |

### QA Red + Coding Green Task Card

## Task: TASK-073 Traditional Support (QA red + Coding green)

**Role:** Coding (combined QA red + green)
**Authoritative docs:** ./docs/01_system_architecture.md (Layer::Support), ./docs/02_ir_schemas.md (SupportIR), ./docs/05_module_sdk.md (LayerModule trait)
**OrcaSlicer reference:**
- OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp — support generation algorithm
- OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp — toolpath fill patterns

**Files to create:**
- `modules/core-modules/traditional-support/Cargo.toml` — library crate depending on slicer-sdk, slicer-ir
- `modules/core-modules/traditional-support/src/lib.rs` — TraditionalSupport struct implementing LayerModule
- `modules/core-modules/traditional-support/tests/traditional_support_tdd.rs` — 8 TDD tests

**Files to modify:**
- `Cargo.toml` — add workspace member

**Context:**
Traditional support generates rectilinear fill patterns for support material areas. For MVP,
the module reads support config (enabled, density, speed, angle, line_width) and fills region
polygons with scan-line support paths using the same approach as rectilinear-infill: rotate
polygon space, compute bounding box, generate horizontal scan lines at spacing intervals,
intersect with polygon edges, convert to ExtrusionPath3D with SupportMaterial role.

The module can reuse the scan-line fill algorithm from rectilinear-infill adapted for support.
Key differences from infill: uses SupportMaterial extrusion role, reads support-specific config
keys, support_density instead of infill_density.

**Algorithm:**
1. on_print_start: read support_enabled, support_density, support_speed, support_angle, line_width
2. If !support_enabled, run_support returns Ok(()) immediately
3. If support_density == 0 or no polygons, return Ok(())
4. Compute line_spacing = line_width / support_density
5. Compute angle = support_angle + (90° if odd layer)
6. For each region's polygons (each ExPolygon):
   a. Rotate points by -angle
   b. Compute bounding box
   c. Generate scan lines at line_spacing intervals
   d. Intersect scan lines with polygon edges, pair intersections
   e. Rotate endpoints back by +angle
   f. Convert to Point3WithWidth at layer Z with line_width
   g. Create ExtrusionPath3D with role=SupportMaterial, speed_factor=support_speed/50.0
7. Push each path via push_support_path()

**Key type conversions:**
- ExPolygon contour Point2 (i64 scaled, 10_000 units/mm) → Point3WithWidth (f32 mm)
- Speed factor: support_speed / 50.0 (normalized to base 50mm/s)

**Acceptance criteria:**
- [ ] All 8 tests in `cargo test -p traditional-support --test traditional_support_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] Support paths use ExtrusionRole::SupportMaterial
- [ ] Line spacing proportional to 1/density
- [ ] Angle alternation between layers
- [ ] Disabled/empty/zero-density produces no output

### CoderAgent Report (TASK-073):
```yaml
task_id: TASK-073
status: done
summary: >
  Created modules/core-modules/traditional-support/ crate implementing LayerModule
  trait for Layer::Support stage. TraditionalSupport reads support_enabled, support_density,
  support_angle, support_speed, line_width from ConfigView. When enabled, generates
  rectilinear scan-line fill patterns for region polygons with per-layer 90° alternation.
  Uses ExtrusionRole::SupportMaterial. Speed factors normalized to base 50mm/s.
  8 TDD tests covering disabled/enabled, role, speed, density, angle alternation,
  empty regions, and zero density. All pass. Workspace builds and tests clean.
files_changed:
  - Cargo.toml (added workspace member)
  - modules/core-modules/traditional-support/Cargo.toml
  - modules/core-modules/traditional-support/src/lib.rs
tests_added_or_updated:
  - modules/core-modules/traditional-support/tests/traditional_support_tdd.rs (8 tests)
risks:
  - none
notes_for_tester:
  - All 8 traditional_support_tdd tests pass
  - 1 unit test (on_print_start_defaults) also passes
  - cargo test --workspace passes (all workspace tests green)
  - cargo clippy -p traditional-support clean
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-073 marked complete
```

## Current Task: TASK-074 — CLI Argument Parsing (Phase E)

### Analysis
TASK-074 adds `clap`-based CLI argument parsing to `crates/slicer-host/` so the host binary
can be invoked with `slicer-host run --module <wasm> --model <model> [--config <config>] [--output <output>]`
and `slicer-host config-schema [--module-dir <path>]`.

The `cli/slicer-cli/src/cmd_run.rs` already builds host args in `build_host_args()` (line 120-139):
- `run --module <wasm> --model <model> [--config <config>] [--output <output>]`

This confirms the expected CLI interface that slicer-host must parse.

### Key SDK Types
- **HostCli** — top-level clap struct with subcommands
- **HostCommands::Run** — `--module`, `--model`, `--config`, `--output`, `--module-dir`
- **HostCommands::ConfigSchema** — `--module-dir`
- **HostRunOptions** — validated runtime options mapped from CLI args
- **CliError** — argument validation errors

### Interface from cmd_run.rs
```
slicer-host run --module <path.wasm> --model <path.stl> [--config <path.json>] [--output <path.gcode>]
slicer-host config-schema [--module-dir <path>]
slicer-host --version
slicer-host --help
```

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| run_requires_module_and_model | `run` with no args | Error: missing required args |
| run_parses_all_flags | `run --module a.wasm --model b.stl --config c.json --output d.gcode` | All fields parsed |
| run_optional_config_and_output | `run --module a.wasm --model b.stl` | config=None, output=None |
| config_schema_default_dir | `config-schema` | module_dir defaults to "." |
| config_schema_custom_dir | `config-schema --module-dir /foo` | module_dir="/foo" |
| validate_run_options_missing_model | RunOptions with nonexistent model path | Error |
| validate_run_options_missing_module | RunOptions with nonexistent wasm path | Error |
| version_flag | `--version` | Shows version string |

### QA Red + Coding Green Task Card

### CoderAgent Report (TASK-074):
```yaml
task_id: TASK-074
status: done
summary: >
  Added clap-based CLI argument parsing to crates/slicer-host/src/cli.rs.
  HostCli parses `run` and `config-schema` subcommands with all flags matching
  the interface expected by cli/slicer-cli/src/cmd_run.rs build_host_args().
  validate_run_options() checks file/dir existence. CliError has Display impl.
  8 TDD tests covering parsing and validation. All pass.
files_changed:
  - crates/slicer-host/Cargo.toml (added clap dependency)
  - crates/slicer-host/src/cli.rs (created)
  - crates/slicer-host/src/lib.rs (added pub mod cli + re-exports)
tests_added_or_updated:
  - crates/slicer-host/tests/cli_tdd.rs (8 tests)
risks:
  - none
```

## Current Task: TASK-075 — Main Entry Point (Phase E)

### Analysis
TASK-075 creates the main binary entry point for slicer-host that orchestrates the full
slicing pipeline: CLI parsing → model loading → module discovery → DAG validation →
execution plan → prepass → per-layer → finalization → postpass → gcode output.

All low-level executor APIs already exist in slicer-host (prepass, per-layer, finalization,
postpass, blackboard, DAG, validation, execution plan). The main entry point is the
orchestration glue that ties them together.

### Key Constraint
For MVP, actual WASM module execution is not yet wired up. The main entry point needs to:
1. Parse CLI via HostCli/validate_run_options (done in TASK-074)
2. Load model file into MeshIR (stub — actual format loaders are TASK-076)
3. Load modules via load_modules_from_roots
4. Build and validate DAG
5. Build execution plan
6. Create Blackboard
7. Execute pipeline stages in order (PrePass → PerLayer → Finalization → PostPass)
8. Write gcode output to file or stdout

For TDD, we test the orchestration function with injectable trait runners.

### Existing APIs (from lib.rs)
- `HostCli`, `validate_run_options` — CLI parsing
- `load_modules_from_roots` — module discovery
- `build_intra_stage_dag` — DAG construction
- `validate_startup_dag` — DAG validation
- `topological_sort` — module ordering
- `build_execution_plan` — freeze runtime schedule
- `Blackboard::new` — create shared state
- `execute_prepass` — run tier 1
- `execute_per_layer` — run tier 2 (rayon)
- `execute_layer_finalization` — run finalization
- `execute_postpass` — run tier 3
- `DefaultGCodeEmitter`, `DefaultGCodeSerializer` — gcode emission

### Architecture (from docs/04)
```
Phase 1: Manifest Ingestion     (parse all .toml files)
Phase 2: DAG Construction       (build intra-stage dependency graphs)
Phase 3: DAG Validation         (claim conflicts, cycles, version checks)
Phase 4: Execution              (PrePass → Per-Layer parallel → PostPass)
```

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| run_pipeline_empty_modules | valid model, no modules dir | Succeeds with empty gcode |
| run_pipeline_writes_output_file | valid model + output path | Creates output file |
| run_pipeline_writes_stdout | valid model, no output path | Returns gcode string |
| run_pipeline_propagates_prepass_error | mock prepass that fails | Pipeline returns prepass error |
| run_pipeline_propagates_layer_error | mock per-layer that fails | Pipeline returns layer error |
| run_pipeline_propagates_postpass_error | mock postpass that fails | Pipeline returns postpass error |
| run_pipeline_calls_stages_in_order | mock runners tracking call order | prepass → per_layer → finalization → postpass |
| config_schema_command_runs | config-schema subcommand | Returns schema JSON |

### QA Red + Coding Green Task Card

## Task: TASK-075 Main Entry Point (QA red + Coding green)

**Role:** Coding (combined QA red + green)
**Authoritative docs:** ./docs/04_host_scheduler.md (Phase 1-4 lifecycle), ./docs/01_system_architecture.md (pipeline stages)
**OrcaSlicer reference:** N/A — our modular WASM architecture has no direct Orca equivalent

**Files to create:**
- `crates/slicer-host/src/pipeline.rs` — `run_pipeline(PipelineConfig) -> Result<PipelineOutput, PipelineError>` orchestration function
- `crates/slicer-host/tests/pipeline_tdd.rs` — 8 TDD tests

**Files to modify:**
- `crates/slicer-host/Cargo.toml` — add `[[bin]]` target for slicer-host, add `anyhow` dependency
- `crates/slicer-host/src/main.rs` — binary entry point using clap + pipeline
- `crates/slicer-host/src/lib.rs` — add `pub mod pipeline`

**Context:**
The main entry point orchestrates the full slicing pipeline. It is structured as:
1. `main.rs` — thin binary: parse CLI with clap, dispatch to `run_pipeline()` or `config_schema()`
2. `pipeline.rs` — testable orchestration: `run_pipeline(PipelineConfig)` that takes injectable
   trait runners for each stage, enabling TDD without real WASM modules.

**PipelineConfig struct:**
```rust
pub struct PipelineConfig {
    pub model_path: PathBuf,
    pub module_roots: Vec<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub prepass_runner: Box<dyn PrepassStageRunner>,
    pub layer_runner: Box<dyn LayerStageRunner>,
    pub finalization_runner: Box<dyn FinalizationStageRunner>,
    pub postpass_runner: Box<dyn PostpassStageRunner>,
}
```

For MVP, model loading is a stub that creates a minimal MeshIR (actual format loaders are TASK-076).
The key deliverable is the orchestration sequence with proper error propagation.

**PipelineError enum:**
```rust
pub enum PipelineError {
    Io(std::io::Error),
    ModelLoad(String),
    ModuleLoad(LoadError),
    DagValidation(Vec<SchedulerError>),
    ExecutionPlan(ExecutionPlanError),
    Prepass(PrepassExecutionError),
    LayerExecution(LayerExecutionError),
    Finalization(FinalizationError),
    Postpass(PostpassError),
}
```

**Pipeline orchestration sequence:**
1. Load model file → MeshIR (stub for now: read bytes, create minimal IndexedTriangleSet)
2. Discover modules: `load_modules_from_roots(&module_roots)` → handle warnings
3. If no modules loaded: skip DAG/execution, emit empty gcode
4. Build DAGs per stage: `build_intra_stage_dag(stage, &modules)` for each stage
5. Validate: `validate_startup_dag(&request)` → fail on errors
6. Build execution plan: `build_execution_plan(&request)` → freeze schedule
7. Create blackboard: `Blackboard::new(Arc::new(mesh_ir), layer_count)`
8. Execute prepass: `execute_prepass(&plan, &mut blackboard, &prepass_runner)`
9. Execute per-layer: `execute_per_layer(&plan, &blackboard)` → Vec<LayerCollectionIR>
10. Execute finalization: `execute_layer_finalization(&plan, &layer_irs, &blackboard)`
11. Execute postpass: `execute_postpass(&plan, &layer_irs, &blackboard, &emitter, &serializer, &postpass_runner)`
12. Write gcode to output_path or return as string

**main.rs structure:**
```rust
use clap::Parser;
use slicer_host::{HostCli, HostCommands, validate_run_options};
use slicer_host::pipeline::{run_pipeline, PipelineConfig};

fn main() {
    let cli = HostCli::parse();
    match cli.command {
        HostCommands::Run { module, model, config, output, module_dir } => {
            let opts = validate_run_options(&module, &model, config.as_deref(), output.as_deref(), &module_dir)
                .unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
            // Build PipelineConfig with default runners
            // run_pipeline(config).unwrap_or_else(...)
        }
        HostCommands::ConfigSchema { module_dir } => {
            // Load modules, collect schemas, print JSON
        }
    }
}
```

**Acceptance criteria:**
- [ ] All 8 tests in `cargo test -p slicer-host --test pipeline_tdd` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo build -p slicer-host` produces a binary (slicer-host)
- [ ] Pipeline calls stages in documented order
- [ ] Error propagation from each stage works
- [ ] Empty module set produces valid (empty) gcode output
- [ ] Config-schema command works with empty module dir

### CoderAgent Report (TASK-075):
```yaml
task_id: TASK-075
status: done
summary: >
  Created crates/slicer-host/src/pipeline.rs with run_pipeline(PipelineConfig) -> Result<PipelineOutput, PipelineError>
  orchestrating the full slicing pipeline: prepass → per-layer → finalization → postpass.
  PipelineConfig takes an ExecutionPlan and PipelineStageRunners (injectable trait objects for all 4 stages
  plus GCodeEmitter and GCodeSerializer). PipelineError wraps errors from each stage with From impls.
  Created crates/slicer-host/src/main.rs as the binary entry point with clap CLI dispatch
  (run and config-schema subcommands, stubbed pending TASK-076 model loading).
  Added [[bin]] target to Cargo.toml. 8 TDD tests covering empty pipeline, gcode output,
  error propagation from prepass/layer/finalization/postpass, stage ordering, and multi-layer output.
files_changed:
  - crates/slicer-host/Cargo.toml (added [[bin]] target)
  - crates/slicer-host/src/lib.rs (added pub mod pipeline)
  - crates/slicer-host/src/pipeline.rs (created)
  - crates/slicer-host/src/main.rs (created)
tests_added_or_updated:
  - crates/slicer-host/tests/pipeline_tdd.rs (8 tests)
risks:
  - none
notes_for_tester:
  - All 8 pipeline_tdd tests pass
  - cargo test --workspace passes (all workspace tests green)
  - cargo build -p slicer-host produces slicer-host binary
  - main.rs is a stub pending TASK-076 model loading wiring
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-075 marked complete
```

## Current Task: TASK-076 — File Format Loaders (STL/OBJ/3MF)

### Analysis
TASK-076 implements host-side model loading for STL, OBJ, and 3MF formats in slicer-host.
The pipeline.rs currently creates a stub empty MeshIR (lines 116-136) and main.rs has a
TODO(TASK-076) placeholder. This task:
1. Creates a model_loader module with format detection and parsing
2. Wires model loading into pipeline.rs (replacing the stub)
3. Wires the run command in main.rs to actually execute the pipeline

### Key Types
- **MeshIR** (slicer-ir) — schema_version, objects: Vec<ObjectMesh>, build_volume
- **ObjectMesh** — id (ObjectId/Uuid), mesh (IndexedTriangleSet), transform, config, modifier_volumes, paint_data
- **IndexedTriangleSet** — vertices: Vec<Point3>, indices: Vec<u32>
- **Point3** — x: f32, y: f32, z: f32

### Format Detection
- `.stl` — binary or ASCII STL
- `.obj` — Wavefront OBJ
- `.3mf` — ZIP-based 3MF archive
- Detection by file extension (case insensitive)

### Crate Dependencies Needed
- `stl_io` — STL parsing (binary + ASCII)
- `tobj` — OBJ parsing
- `zip` — 3MF ZIP extraction
- `quick-xml` — 3MF XML parsing
- `uuid` — ObjectId generation

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Format/STL.cpp — binary/ASCII STL loading
- OrcaSlicerDocumented/src/libslic3r/Format/OBJ.cpp — OBJ loading
- OrcaSlicerDocumented/src/libslic3r/Format/3mf.cpp — 3MF loading

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| load_stl_binary_cube | binary STL cube fixture | 1 object, 12 triangles, 8 vertices |
| load_stl_ascii_cube | ASCII STL cube fixture | 1 object, 12 triangles |
| load_obj_cube | OBJ cube fixture | 1 object with correct mesh |
| load_3mf_cube | 3MF cube fixture | 1 object with correct mesh |
| detect_format_by_extension | various paths | Correct ModelFormat enum |
| unknown_extension_error | "model.xyz" | ModelLoadError::UnsupportedFormat |
| nonexistent_file_error | missing path | ModelLoadError::Io |
| load_model_produces_mesh_ir | any valid fixture | MeshIR with schema_version, non-empty objects |
| bounding_box_computed | valid cube | build_volume matches mesh extents |
| pipeline_uses_loaded_model | wired pipeline test | Pipeline runs with real loaded mesh |

### QA Red + Coding Green Task Card

### CoderAgent Report (TASK-076):
```yaml
task_id: TASK-076
status: done
summary: >
  Created crates/slicer-host/src/model_loader.rs with load_model(path) -> Result<MeshIR, ModelLoadError>
  supporting STL (binary+ASCII via stl_io with vertex deduplication), OBJ (via tobj), and
  3MF (ZIP+quick-xml parsing). Format detected by case-insensitive extension. Each loaded model
  produces a single ObjectMesh with identity transform, computed BoundingBox3, and schema_version 1.0.0.
  Added stl_io, tobj, zip, quick-xml, uuid dependencies to slicer-host Cargo.toml.
  10 TDD tests covering all 3 formats, format detection, error cases, MeshIR structure, and bounding box.
files_changed:
  - crates/slicer-host/Cargo.toml (added stl_io, tobj, zip, quick-xml, uuid deps)
  - crates/slicer-host/src/model_loader.rs (created)
  - crates/slicer-host/src/lib.rs (added pub mod model_loader)
  - Cargo.lock (updated)
tests_added_or_updated:
  - crates/slicer-host/tests/model_loader_tdd.rs (10 tests)
risks:
  - none
notes_for_tester:
  - All 10 model_loader_tdd tests pass
  - cargo test --workspace passes (all workspace tests green)
  - cargo build --workspace succeeds
  - pipeline.rs and main.rs NOT yet wired (separate task to integrate)
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-076 marked complete
```

## Current Task: TASK-077 — Integration Test: End-to-End STL Pipeline

### Analysis
TASK-077 is the final Phase E task. It validates the full pipeline end-to-end:
load model → build plan → execute stages → emit gcode.

**Key changes needed:**

1. **pipeline.rs** — `PipelineConfig` must accept `mesh_ir: Arc<MeshIR>` instead of creating
   a stub. `run_pipeline()` passes this to `Blackboard::new(mesh_ir, layer_count)`.

2. **main.rs** — Wire `HostCommands::Run` to:
   - `model_loader::load_model()` for model loading
   - `load_modules_from_roots()` for module discovery
   - Build execution plan (empty if no modules)
   - Create `DefaultGCodeEmitter` + `DefaultGCodeSerializer`
   - Call `run_pipeline()` and write output to file or stdout

3. **Integration test** — `crates/slicer-host/tests/e2e_integration_tdd.rs`:
   - Load 20mmbox-LF.stl via model_loader
   - Build empty execution plan (no WASM modules)
   - Run pipeline with DefaultGCodeEmitter + DefaultGCodeSerializer + no-op runners
   - Verify non-error completion
   - Verify deterministic output (run twice, same result)
   - Test with layers to verify gcode emission
   - Test error propagation from model_loader (unsupported format, missing file)

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| e2e_load_stl_empty_plan | 20mmbox-LF.stl, no modules | Pipeline succeeds, gcode output |
| e2e_deterministic_output | same STL twice | Identical gcode output |
| e2e_model_load_error | nonexistent.stl | PipelineError::ModelLoad |
| e2e_unsupported_format | model.xyz | PipelineError::ModelLoad |
| e2e_with_layers | STL + layers in plan | Non-empty gcode with layer data |
| e2e_pipeline_uses_real_mesh | STL + empty plan | Blackboard mesh has non-empty objects |
| e2e_output_to_file | STL + output path | File created with gcode content |
| e2e_main_binary_runs | --help flag | Binary exits successfully |

### QA Red + Coding Green Task Card

## Task: TASK-077 End-to-End Integration Test (QA red + Coding green)

### CoderAgent Report (TASK-077):
```yaml
task_id: TASK-077
status: done
summary: >
  Wired pipeline.rs to accept mesh_ir: Arc<MeshIR> in PipelineConfig instead of creating
  a stub. Added ModelLoad variant to PipelineError. Wired main.rs Run command to load_model()
  + empty ExecutionPlan + run_pipeline() with DefaultGCodeEmitter/Serializer. ConfigSchema
  emits empty JSON. Created 8 e2e integration tests covering: STL loading with empty plan,
  deterministic output verification, nonexistent file error, unsupported format error,
  pipeline with layers, real mesh geometry validation (36 indices for 20mm box), file output,
  and binary --help execution. Updated existing pipeline_tdd.rs to provide mesh_ir.
files_changed:
  - crates/slicer-host/src/pipeline.rs (added mesh_ir to PipelineConfig, ModelLoad error)
  - crates/slicer-host/src/main.rs (wired model loading + pipeline execution)
  - crates/slicer-host/tests/pipeline_tdd.rs (updated for mesh_ir field)
tests_added_or_updated:
  - crates/slicer-host/tests/e2e_integration_tdd.rs (8 tests)
risks:
  - Pre-existing clippy warnings in slicer-core block workspace-wide clippy
notes_for_tester:
  - All 8 e2e tests pass
  - All 8 existing pipeline_tdd tests still pass  
  - cargo test --workspace passes (all workspace tests green)
  - cargo build --workspace succeeds
  - slicer-host binary runs with --help
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-077 marked complete
```

## Phase E: COMPLETE (TASK-077 verified + docs updated)
- All 8 e2e tests pass, all workspace tests pass, build clean
- docs/07 updated, committed as 230d948

## Current Task: TASK-081 — Arachne Perimeters Module (Phase F)

### Analysis
TASK-081 creates a variable-width perimeter generator module implementing `LayerModule::run_perimeters`.
Unlike classic-perimeters (constant-width insets), Arachne produces continuous-width wall loops
using medial-axis / skeletal trapezoidation.

**Full Arachne implementation is complex** (Voronoi diagram → half-edge graph → beading → transitions).
For MVP, we implement a simplified variable-width approach:
1. Build medial axis via iterative polygon insets at fine steps
2. Determine local width from distance-to-boundary at each sample
3. Generate wall loops with varying width profiles

**Key difference from classic-perimeters:**
- WallLoop.width_profile has varying widths (not all identical)
- Thin regions get fewer/narrower walls adaptively
- Width transitions taper smoothly

### OrcaSlicer References
- OrcaSlicerDocumented/generated_documentation/pseudocode_arachne_straight_skeleton.md
- OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp
- OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
- OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionJunction.hpp
- OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.hpp

### Module Structure (follows classic-perimeters pattern)
- modules/core-modules/arachne-perimeters/Cargo.toml
- modules/core-modules/arachne-perimeters/src/lib.rs
- modules/core-modules/arachne-perimeters/tests/arachne_perimeters_tdd.rs

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| on_print_start_defaults | empty config | wall_count=2, line_width=0.4 |
| on_print_start_custom | custom config | reads provided values |
| single_square_region | 10mm square at z=1.0 | >=1 WallLoop, non-empty path points |
| variable_width_profile | narrow wedge region | width_profile has varying widths |
| thin_region_fewer_walls | region narrower than 2*line_width | fewer walls than wall_count |
| zero_walls_config | wall_count=0 | no walls, all infill |
| empty_regions_no_output | empty regions | no wall loops |
| outer_wall_role | any region | first wall has OuterWall role |
| inner_wall_role | any region | subsequent walls have InnerWall role |
| infill_areas_set | region with walls | non-empty infill areas |
| seam_candidates_generated | square region | >=1 seam candidate |

### QA Red + Coding Green Task Card

## Task: TASK-081 Arachne Perimeters (QA red + Coding green)

### CoderAgent Report (TASK-081):
```yaml
task_id: TASK-081
status: done
summary: >
  Created modules/core-modules/arachne-perimeters with ArachnePerimeters implementing
  LayerModule::run_perimeters. Variable-width wall generation uses iterative polygon
  insets combined with ray-based local region width measurement: for each wall path vertex,
  finds nearest outer boundary point, casts ray in opposite direction to find full local
  clearance, distributes width among walls proportionally. Thin regions (narrower than
  wall_count * line_width) naturally get fewer walls with adapted widths. Width profiles
  vary per-vertex based on local geometry (unlike classic-perimeters' uniform widths).
  Added to workspace Cargo.toml.
files_changed:
  - Cargo.toml (added arachne-perimeters to workspace members)
  - modules/core-modules/arachne-perimeters/Cargo.toml (created)
  - modules/core-modules/arachne-perimeters/src/lib.rs (created)
tests_added_or_updated:
  - modules/core-modules/arachne-perimeters/tests/arachne_perimeters_tdd.rs (11 tests)
risks:
  - Simplified Arachne: uses iterative insets + ray-based width instead of full
    Voronoi/skeletal trapezoidation. Good enough for MVP.
  - Pre-existing clippy warnings in slicer-core block workspace-wide clippy
notes_for_tester:
  - All 11 arachne_perimeters_tdd tests pass
  - cargo test --workspace passes (all workspace tests green)
  - cargo build --workspace succeeds
  - clippy clean on arachne-perimeters (--no-deps)
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-081 marked complete
```

## Current Task: TASK-082 — Gyroid Infill Module (Phase F)

### Analysis
TASK-082 creates a gyroid TPMS infill pattern generator module implementing `LayerModule::run_infill`.
The Gyroid equation sin(x)cos(y) + sin(y)cos(z) + sin(z)cos(x) = 0 produces wave-like
fill patterns at each layer Z by sampling the 2D cross-section of the 3D surface.

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp — full implementation
- OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.hpp — constants: CorrectionAngle=-45, DensityAdjust=2.44, PatternTolerance=0.2

### Key Algorithm (from OrcaSlicer, translated to Rust)
1. Compute z phase: z_sin = sin(z / scale_factor), z_cos = cos(z / scale_factor)
2. Choose orientation: if |z_sin| <= |z_cos| → vertical, else horizontal
3. Build f(x, z_sin, z_cos, vertical, flip) → y curve via asin-based formula
4. Adaptive sampling: make_one_period with cross-product tolerance refinement
5. Tile periods across bounding box width: make_wave replicates period template
6. Generate alternating odd/even wave rows at π spacing
7. Clip waves to infill polygon boundaries
8. Convert to ExtrusionPath3D with SparseInfill role

### Module Structure (follows rectilinear-infill pattern)
- modules/core-modules/gyroid-infill/Cargo.toml
- modules/core-modules/gyroid-infill/src/lib.rs
- modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| on_print_start_defaults | empty config | density=0.2, line_width=0.4 |
| on_print_start_custom | custom config | reads provided values |
| square_region_produces_paths | 10mm square, density=0.2 | non-empty sparse paths |
| paths_have_sparse_infill_role | any region | all paths ExtrusionRole::SparseInfill |
| zero_density_no_paths | density=0.0 | no output paths |
| empty_regions_no_output | empty regions | no paths emitted |
| paths_at_correct_z | region z=1.5 | all points z=1.5 |
| wave_pattern_varies_by_layer | two different z | different path geometries |
| density_affects_spacing | 0.1 vs 0.5 density | sparser vs denser paths |
| width_matches_config | line_width=0.6 | all point widths = 0.6 |
| asin_nan_protection | extreme z values | no NaN in output points |

### QA Red + Coding Green Task Card

## Task: TASK-082 Gyroid Infill (QA red + Coding green)

### CoderAgent Report (TASK-082):
```yaml
task_id: TASK-082
status: done
summary: >
  Created modules/core-modules/gyroid-infill with GyroidInfill implementing
  LayerModule::run_infill. Gyroid TPMS wave pattern generation adapted from
  OrcaSlicer FillGyroid.cpp. 11 TDD tests + 4 unit tests all passing.
  Commit f442338.
files_changed:
  - Cargo.toml (added gyroid-infill to workspace members)
  - modules/core-modules/gyroid-infill/Cargo.toml (created)
  - modules/core-modules/gyroid-infill/src/lib.rs (created)
tests_added_or_updated:
  - modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs (11 tests)
risks:
  - Simplified clipping via point-in-polygon per vertex (MVP)
notes_for_tester:
  - All 11 gyroid_infill_tdd tests pass
  - cargo test --workspace passes
  - cargo build --workspace succeeds
  - clippy clean on gyroid-infill
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-082 marked complete
```

## Phase F: TASK-082 gyroid-infill COMPLETE (commit f442338, docs updated)

## Current Task: TASK-083 — Lightning Infill Module (Phase F)

### Analysis
TASK-083 creates a lightning infill pattern generator module implementing `LayerModule::run_infill`.
Lightning infill grows a top-to-bottom forest of branching polylines that support every overhang
point within a configurable "supporting radius." It's the most material-efficient infill pattern
for parts that don't need structural internal strength.

**Full OrcaSlicer implementation is complex** (multi-layer tree propagation with distance fields,
TBB parallelism, edge grids, pruning/straightening/realigning across layers). For MVP, we
implement a simplified single-layer approach per the LayerModule interface (which processes
one layer at a time without cross-layer state):

### MVP Algorithm (simplified lightning for single-layer interface)
Since LayerModule::run_infill gets one layer at a time, we can't build cross-layer trees.
Instead, we implement a "lightning-style" sparse branching pattern per layer:
1. Build a distance field from infill polygon boundaries
2. Sample interior points sorted by distance-to-boundary (interior-first)
3. Grow branches from interior toward nearest boundary, branching at junction points
4. Convert tree polylines to ExtrusionPath3D with SparseInfill role
5. Density controls supporting_radius (branch spacing)

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.hpp/cpp
- OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.hpp/cpp
- OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.hpp/cpp
- OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.hpp/cpp
- OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp/cpp
- OrcaSlicerDocumented/generated_documentation/pseudocode_fill_lightning.md

### Module Structure (follows gyroid-infill pattern)
- modules/core-modules/lightning-infill/Cargo.toml
- modules/core-modules/lightning-infill/src/lib.rs
- modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| on_print_start_defaults | empty config | density=0.2, line_width=0.4 |
| on_print_start_custom | custom config | reads provided values |
| square_region_produces_paths | 10mm square, density=0.2 | non-empty sparse paths |
| paths_have_sparse_infill_role | any region | all paths ExtrusionRole::SparseInfill |
| zero_density_no_paths | density=0.0 | no output paths |
| empty_regions_no_output | empty regions | no paths emitted |
| paths_at_correct_z | region z=1.5 | all points z=1.5 |
| branching_pattern_present | large region | paths have branch-like geometry (not parallel lines) |
| density_affects_coverage | 0.1 vs 0.5 density | sparser vs denser paths |
| width_matches_config | line_width=0.6 | all point widths = 0.6 |
| interior_first_growth | large region | branches reach interior points |

### QA Red + Coding Green Task Card

## Task: TASK-083 Lightning Infill (QA red + Coding green)

### CoderAgent Report (TASK-083):
```yaml
task_id: TASK-083
status: done
summary: >
  Created modules/core-modules/lightning-infill with LightningInfill implementing
  LayerModule::run_infill. Simplified single-layer lightning-style branching pattern:
  samples interior grid points sorted by distance-to-boundary (interior-first),
  grows branches from each sample toward nearest boundary or nearest existing
  connected point (merge radius = 0.75 * spacing). Produces characteristic
  tree-like branching pattern with varying directions. 11 TDD tests + 4 unit
  tests all passing. Commit 00fd3c0.
files_changed:
  - Cargo.toml (added lightning-infill to workspace members)
  - modules/core-modules/lightning-infill/Cargo.toml (created)
  - modules/core-modules/lightning-infill/src/lib.rs (created)
tests_added_or_updated:
  - modules/core-modules/lightning-infill/tests/lightning_infill_tdd.rs (11 tests)
risks:
  - Simplified lightning: single-layer branching without cross-layer tree propagation (MVP)
  - Point-in-polygon clipping per vertex (same as gyroid MVP approach)
notes_for_tester:
  - All 11 lightning_infill_tdd tests pass
  - All 4 unit tests pass
  - cargo test --workspace passes (all workspace tests green)
  - cargo build --workspace succeeds
  - clippy clean on lightning-infill (--no-deps)
notes_for_docs:
  - docs/07_implementation_status.md needs TASK-083 marked complete
```

## Phase F: TASK-083 lightning-infill COMPLETE (commit 00fd3c0, docs updated)

## Current Task: TASK-084 — Seam Placer Module (Phase F)

### Analysis
TASK-084 creates a seam placement module implementing `LayerModule::run_wall_postprocess`.
It reads seam_candidates generated by perimeter modules (classic-perimeters, arachne-perimeters)
and selects the best candidate to write as resolved_seam.

**SDK changes needed first:**
1. `PerimeterRegionView` needs `seam_candidates: Vec<SeamCandidate>` field + accessor
2. `PerimeterOutputBuilder` needs `set_resolved_seam(point: Point3, wall_index: u32)` method

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp/cpp — full seam placement
- OrcaSlicerDocumented/generated_documentation/pseudocode_seam_placer.md

### Key Algorithm (MVP — simplified from OrcaSlicer)
For each perimeter region:
1. Read seam_candidates from the view
2. Score each candidate based on:
   - Corner angle (concave preferred → lower score)
   - SeamReason weight (Concave > Sharp > Aligned > UserForced)
   - Configurable seam_mode: nearest, rear, random, aligned
3. Select lowest-score candidate
4. Write resolved_seam with the selected position and wall_index=0

### IR Types (already exist)
- SeamCandidate { position: Point3WithWidth, score: f32, reason: SeamReason }
- SeamPosition { point: Point3WithWidth, wall_index: u32 }
- SeamReason: Concave, Aligned, UserForced, Sharp

### Module Structure
- modules/core-modules/seam-placer/Cargo.toml
- modules/core-modules/seam-placer/src/lib.rs
- modules/core-modules/seam-placer/tests/seam_placer_tdd.rs

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| on_print_start_defaults | empty config | seam_mode="nearest" |
| on_print_start_custom | custom config | reads provided values |
| picks_lowest_score | 3 candidates, scores 0.8, 0.2, 0.5 | selects score=0.2 |
| concave_preferred | concave + aligned same score | selects concave |
| no_candidates_no_seam | empty seam_candidates | no resolved_seam |
| rear_mode_prefers_back | rear mode, multiple candidates | selects max-Y candidate |
| random_mode_produces_seam | random mode | some resolved_seam set |
| seam_at_correct_z | candidates at z=1.5 | resolved_seam z=1.5 |
| multiple_regions | 2 regions with candidates | each gets resolved_seam |
| empty_regions_no_output | empty regions list | no output |
| wall_index_zero | any candidate | resolved_seam wall_index=0 |

### Prerequisite SDK Changes (included in task)
1. Add seam_candidates field to PerimeterRegionView + constructor + accessor
2. Add set_resolved_seam method to PerimeterOutputBuilder
3. Add resolved_seam accessor to PerimeterOutputBuilder (for testing)

### QA Red + Coding Green Task Card

## Task: TASK-084 Seam Placer (QA red + Coding green)

### CoderAgent Report (TASK-084):
```yaml
task_id: TASK-084
status: done
summary: >
  Created modules/core-modules/seam-placer with SeamPlacer implementing
  LayerModule::run_wall_postprocess. SDK prerequisites: added seam_candidates
  field to PerimeterRegionView (+ constructor param + accessor), added
  resolved_seam field to PerimeterOutputBuilder with set_resolved_seam and
  resolved_seam accessor. Module supports nearest/rear/random seam modes
  with reason-based scoring (Concave/Sharp/UserForced/Aligned bonuses).
  Commit ce2bd98.
files_changed:
  - Cargo.toml (added seam-placer to workspace members)
  - crates/slicer-sdk/src/views.rs (added seam_candidates to PerimeterRegionView)
  - crates/slicer-sdk/src/builders.rs (added resolved_seam to PerimeterOutputBuilder)
  - crates/slicer-test/src/fixtures.rs (updated PerimeterRegionViewBuilder)
  - crates/slicer-sdk/tests/layer_module_tdd.rs (updated call sites)
  - modules/core-modules/seam-placer/Cargo.toml (created)
  - modules/core-modules/seam-placer/src/lib.rs (created)
tests_added_or_updated:
  - modules/core-modules/seam-placer/tests/seam_placer_tdd.rs (11 tests)
risks:
  - Simplified seam placement (single-layer, no cross-layer alignment)
notes_for_tester:
  - All 11 seam_placer_tdd tests pass
  - All 3 unit tests pass
  - cargo test --workspace passes (all workspace tests green)
  - cargo build --workspace succeeds
  - clippy clean on seam-placer
notes_for_docs:
  - docs/07_implementation_status.md TASK-084 marked complete
```

## Phase F: TASK-084 seam-placer COMPLETE (commit ce2bd98, docs updated)

## Current Task: TASK-085 — Tree Support Module (Phase F)

### Analysis
TASK-085 creates a tree-support module implementing `LayerModule::run_support`.
Tree support generates branching polyline structures instead of traditional grid fills.
Branches converge toward fewer build-plate contact points, using less material while
still adequately supporting overhangs.

**Full OrcaSlicer implementation is extremely complex** (multi-layer tree propagation,
TreeModelVolumes collision avoidance, organic/slim/strong modes, distance fields).
For MVP with single-layer LayerModule interface, we implement simplified tree-like branching:
1. Sample support polygon interior points on a grid (spacing from density)
2. Build a minimum spanning tree connecting interior points
3. Root the tree at centroid/boundary and generate branch paths
4. Convert tree edges to ExtrusionPath3D with SupportMaterial role

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp/cpp
- OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.hpp/cpp
- OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp
- OrcaSlicerDocumented/src/libslic3r/Support/TreeModelVolumes.hpp/cpp

### Existing SDK Types
- SupportOutputBuilder: push_support_path, push_interface_path, push_raft_path
- SupportIR: support_paths, interface_paths, raft_paths, ironing_paths
- ExtrusionRole::SupportMaterial, SupportInterface
- SupportType::Tree (already in IR)

### Module Structure (follows traditional-support pattern)
- modules/core-modules/tree-support/Cargo.toml
- modules/core-modules/tree-support/src/lib.rs
- modules/core-modules/tree-support/tests/tree_support_tdd.rs

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| on_print_start_defaults | empty config | density=0.2, line_width=0.4, enabled=false |
| on_print_start_custom | custom config | reads provided values |
| square_region_produces_paths | 10mm square, enabled=true | non-empty support paths |
| paths_have_support_role | any region | all paths SupportMaterial role |
| disabled_no_paths | enabled=false | no output paths |
| zero_density_no_paths | density=0.0 | no output paths |
| empty_regions_no_output | empty regions | no paths emitted |
| paths_at_correct_z | region z=1.5 | all points z=1.5 |
| branching_pattern_present | large region | paths have branch-like geometry (not parallel) |
| density_affects_coverage | 0.1 vs 0.5 density | sparser vs denser paths |
| width_matches_config | line_width=0.6 | all point widths = 0.6 |

### QA Red + Coding Green Task Card

## Task: TASK-085 Tree Support (QA red + Coding green)

### CoderAgent Report (TASK-085):
```yaml
task_id: TASK-085
status: done
summary: >
  Created modules/core-modules/tree-support with TreeSupport implementing
  LayerModule::run_support. Nearest-neighbor tree branching algorithm: grid
  samples interior points at spacing=line_width/density, builds tree from
  centroid root connecting nearest unvisited points, generates branch paths
  with SupportMaterial role. 11 TDD + 3 unit tests pass. Commit 5d7a185.
files_changed:
  - Cargo.toml (added tree-support to workspace members)
  - modules/core-modules/tree-support/Cargo.toml (created)
  - modules/core-modules/tree-support/src/lib.rs (created)
tests_added_or_updated:
  - modules/core-modules/tree-support/tests/tree_support_tdd.rs (11 tests)
risks:
  - Simplified tree support: single-layer branching without cross-layer propagation (MVP)
  - O(n^2) nearest-neighbor for large point sets
notes_for_tester:
  - All 11 tree_support_tdd tests pass
  - cargo test --workspace passes
  - cargo build --workspace succeeds
  - clippy clean on tree-support
notes_for_docs:
  - docs/07_implementation_status.md TASK-085 marked complete
```

## Phase F: TASK-085 tree-support COMPLETE (commit 5d7a185, docs updated)

## Current Task: TASK-086 — Support Surface Ironing Module (Phase F)

### Analysis
TASK-086 creates a surface ironing module implementing `LayerModule::run_infill_postprocess`.
Ironing generates low-flow rectilinear passes over top surfaces to smooth them. It reads
perimeter regions (wall_loops + infill_areas) to identify top-surface polygons and emits
ironing paths via InfillOutputBuilder::push_ironing_path.

### OrcaSlicer References
- OrcaSlicerDocumented/src/libslic3r/GCode.cpp — ironing G-code generation
- OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp — ironing config parameters
- OrcaSlicerDocumented/generated_documentation/01_system_architecture.md — ironing in pipeline

### Key Algorithm (MVP — simplified from OrcaSlicer)
1. Read config: ironing_enabled, ironing_speed, ironing_flow_rate, ironing_spacing, line_width
2. For each perimeter region:
   a. Get infill_areas as top-surface polygons
   b. Generate rectilinear scan lines at ironing_spacing interval
   c. Clip lines to infill_area polygons (point-in-polygon per vertex)
   d. Create ExtrusionPath3D with ExtrusionRole::Ironing and reduced flow
3. Push paths via InfillOutputBuilder::push_ironing_path

### Interface
- Implements `LayerModule::run_infill_postprocess`
- Receives: layer_index, &[PerimeterRegionView], &mut InfillOutputBuilder, &ConfigView
- PerimeterRegionView provides: wall_loops, infill_areas (top surface polygons)
- InfillOutputBuilder provides: push_ironing_path

### Module Structure (follows rectilinear-infill pattern)
- modules/core-modules/support-surface-ironing/Cargo.toml
- modules/core-modules/support-surface-ironing/src/lib.rs
- modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs

### TDD Contract
| Test | Input | Expected |
|------|-------|----------|
| on_print_start_defaults | empty config | enabled=false, speed=15.0, flow=0.1, spacing=0.1 |
| on_print_start_custom | custom config | reads provided values |
| disabled_no_paths | enabled=false | no ironing paths |
| square_region_produces_paths | 10mm square, enabled=true | non-empty ironing paths |
| paths_have_ironing_role | any region | all paths ExtrusionRole::Ironing |
| empty_regions_no_output | empty regions | no paths emitted |
| paths_at_correct_z | region z=1.5 | all points z=1.5 |
| flow_rate_applied | flow=0.15 | all point flow_factor ~0.15 |
| spacing_affects_density | 0.1 vs 0.4 spacing | more vs fewer paths |
| width_matches_config | line_width=0.4 | all point widths = 0.4 |
| rectilinear_pattern | large region | paths have parallel-line geometry |

### QA Red + Coding Green Task Card

---

```yaml
task_id: TASK-086
status: done
summary: Implemented support-surface-ironing module with LayerModule::run_infill_postprocess. Generates low-flow rectilinear scan lines over top surfaces using ExtrusionRole::Ironing. Z derived from first wall loop point. Scan-line intersection algorithm matches rectilinear-infill pattern.
files_changed:
  - Cargo.toml (added workspace member)
  - modules/core-modules/support-surface-ironing/Cargo.toml
  - modules/core-modules/support-surface-ironing/src/lib.rs
  - modules/core-modules/support-surface-ironing/tests/ironing_tdd.rs
tests_added_or_updated:
  - ironing_tdd.rs: 11 integration tests (on_print_start_defaults, on_print_start_custom, disabled_no_paths, square_region_produces_paths, paths_have_ironing_role, empty_regions_no_output, paths_at_correct_z, flow_rate_applied, spacing_affects_density, width_matches_config, rectilinear_pattern)
  - lib.rs: 1 unit test (on_print_start_defaults)
risks:
  - Ironing only generates horizontal (axis-aligned) scan lines; no per-layer rotation like rectilinear-infill
  - No hole clipping beyond even/odd edge intersection pairing (matches existing pattern)
notes_for_tester:
  - All 11 TDD tests pass, clippy clean, workspace builds and tests clean
  - Module skips regions with no wall loops (cannot determine z)
notes_for_docs:
  - Config keys: ironing_enabled (bool), ironing_speed (float, mm/s), ironing_flow_rate (float), ironing_spacing (float, mm), line_width (float, mm)
```
