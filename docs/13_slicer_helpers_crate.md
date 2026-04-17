# ModularSlicer — slicer-helpers Crate

## Purpose

`slicer-helpers` is a library crate providing **pre-pipeline mesh processing operations**. It runs before any WASM module is loaded and before the slicing pipeline starts. Its outputs are `MeshIR` values (or modified `MeshIR` values) consumed by the host's standard pipeline entry point.

These operations are hosted here because they require native libraries or algorithms that cannot be expressed inside the WASM sandbox, produce or transform `MeshIR` that the pipeline then consumes, and are invoked directly via host CLI subcommands rather than through the module scheduler.

---

## Scope

**In scope:**

| Feature         | CLI subcommand | Description                                                                       |
|-----------------|----------------|-----------------------------------------------------------------------------------|
| Mesh repair     | `pnp repair`   | Manifold fixing: degenerate removal, orientation normalization, open-edge closure |
| Mesh decimation | `pnp decimate` | QEM triangle-count reduction with configurable error budget                       |
| STEP import     | `pnp import`   | STEP/STP → triangulated `MeshIR`, including unit normalization                    |

**Out of scope:**

| Item                              | Reason                                                          |
|-----------------------------------|-----------------------------------------------------------------|
| STL / OBJ / 3MF import            | Handled by the host's existing format loaders in `slicer-host`  |
| Per-layer geometry operations     | Pipeline module concerns using `slicer-core` and Clipper        |
| WASM module execution             | Owned by `slicer-host` scheduler                                |
| Boolean modifier volume execution | Handled per-layer by `slicer-core` Clipper ops (pipeline stage) |
| Any rendering or preview code     | Frontend (Unity) concern                                        |

---

## Crate Structure

```
crates/slicer-helpers/
├── Cargo.toml
└── src/
    ├── lib.rs               — public API surface; re-exports from sub-modules
    ├── repair.rs            — mesh manifold repair
    ├── decimate.rs          — QEM mesh decimation
    └── import/
        ├── mod.rs           — shared import utilities, unit conversion
        └── step.rs          — STEP/STP → MeshIR pipeline
```

Test files follow the project-wide TDD convention (tests fail before implementation):

```
crates/slicer-helpers/
└── tests/
    ├── repair_tdd.rs
    ├── decimate_tdd.rs
    └── import_step_tdd.rs
```

---

## Dependency Rules

`slicer-helpers` must obey the following dependency constraints:

| Dependency      | Allowed | Reason                                                              |
|-----------------|---------|---------------------------------------------------------------------|
| `slicer-ir`     | Yes     | Reads and writes `MeshIR`                                           |
| `nalgebra`      | Yes     | Geometry math for repair and decimation                             |
| `meshopt`       | Yes     | QEM decimation (see §Decimation)                                    |
| `truck-stepio`  | Yes     | STEP parser (see §STEP Import)                                      |
| `truck-meshing` | Yes     | BRep triangulation (see §STEP Import)                               |
| `slicer-core`   | No      | Core is a peer crate; helpers must not create circular dependencies |
| `slicer-host`   | No      | Host depends on helpers, not the reverse                            |
| `wasmtime`      | No      | No WASM runtime in this crate                                       |
| Any GUI crate   | No      | Zero UI code                                                        |

New workspace dependencies required in root `Cargo.toml`:

```toml
meshopt    = "0.3"
truck-stepio  = "0.2"
truck-meshing = "0.2"
```

---

## Coordinate System Contract

All operations in this crate **input and output values in Pinch_n_Print's internal coordinate system**:

```
1 internal unit = 100 nm = 0.0001 mm
```

The STEP importer is responsible for converting from the STEP file's declared units to internal units before populating `MeshIR`. All other operations (repair, decimate) receive and emit already-converted coordinates and must not apply any unit conversion.

Reference: `./docs/08_coordinate_system.md` — normative unit definitions.

Unit conversion table for STEP import:

| STEP declared unit       | Factor to internal units |
|--------------------------|--------------------------|
| Millimetre (most common) | × 10,000                 |
| Metre                    | × 10,000,000             |
| Inch                     | × 254,000                |
| Micrometre               | × 10                     |

If the STEP file declares no unit, the importer must default to millimetres and emit a structured warning.

---

## Feature: Mesh Repair

### Purpose

Fixes non-manifold geometry in imported meshes so the slicer pipeline always receives a closed, consistently oriented triangle mesh. Equivalent to OrcaSlicer's admesh-based repair pipeline applied at import time.

### Algorithm (Three Phases, Sequential)

**Phase 1 — Degenerate triangle removal**

A triangle is degenerate if its area is below `1e-8` square internal units (approximately 1 nm² in real space). Degenerate triangles are removed before any other operation because they poison normal computation.

Criterion: `||(v1 - v0).cross(v2 - v0)||² < 2e-16`

**Phase 2 — Face orientation normalization**

Starting from the triangle with the most negative Z centroid (chosen for determinism), flood-fill across shared edges and flip any neighbouring triangle whose shared-edge winding is inconsistent with the propagation front.

If the mesh has multiple disconnected components, run one flood-fill per component. Orientation of each component is resolved independently; the final orientation of a component is set so its outward normals point away from its centroid.

**Phase 3 — Open-edge closure**

An open edge is an edge referenced by exactly one triangle. After Phases 1 and 2, collect all open edges, group them into boundary loops by shared vertex, and cap each loop with a fan of triangles originating at the loop centroid.

If a boundary loop contains more than `MAX_REPAIR_CAP_VERTICES = 256` vertices, the repair emits a non-fatal `RepairWarning::LargeCapLoop` and skips that loop (it is too large to fan-cap reliably without introducing self-intersections). The caller receives the partial result with `repaired = false` on the affected component.

### Output

```rust
pub struct RepairResult {
    pub mesh: MeshIR,
    pub stats: RepairStats,
}

pub struct RepairStats {
    pub degenerate_removed: usize,
    pub faces_reoriented: usize,
    pub open_edges_closed: usize,
    pub components: usize,
    pub warnings: Vec<RepairWarning>,
}

pub enum RepairWarning {
    LargeCapLoop { vertex_count: usize },
    MultipleComponents { count: usize },
}
```

### Public API

```rust
/// Repair a mesh in place. Returns a RepairResult.
/// Input mesh may be non-manifold. Output mesh is manifold unless warnings
/// indicate skipped loops.
pub fn repair(mesh: MeshIR) -> Result<RepairResult, RepairError>
```

### CLI Subcommand: `pnp repair`

```
pnp repair --input <path> --output <path> [--format <stl|obj|3mf>] [--stats]

Options:
  --input     Input mesh file (STL, OBJ, 3MF, or STEP after conversion)
  --output    Output mesh file path
  --format    Output format (default: same as input)
  --stats     Print repair statistics to stderr as JSON
```

Exit codes:

| Code | Meaning                                                                |
|------|------------------------------------------------------------------------|
| 0    | Repair succeeded; mesh is fully manifold                               |
| 1    | Repair partially succeeded; some loops were skipped (warnings present) |
| 2    | Input file not found or unreadable                                     |
| 3    | Input mesh is empty                                                    |

---

## Feature: Mesh Decimation

### Purpose

Reduces triangle count via quadric error metric (QEM) edge collapse. Used to reduce high-resolution imported meshes (photogrammetry scans, STEP tessellations) to a size the slicer pipeline can process efficiently without losing print-relevant detail.

### Library: `meshopt`

Decimation is implemented via the `meshopt` crate (Rust bindings to meshoptimizer), which provides `simplify` (quality-preserving) and `simplify_sloppy` (faster, aggressive) functions. `meshopt` was chosen over a custom QEM implementation because:

- Battle-tested in game engine production use cases
- The `simplify` function implements the same Garland-Heckbert QEM algorithm used in OrcaSlicer's `QuadricEdgeCollapse.cpp`
- Pure C with no LGPL/GPL — clean licensing for redistribution
- No additional geometry library required beyond nalgebra for pre/post-processing

### Algorithm

1. Convert `MeshIR` vertices and indices into `meshopt`'s flat `f32` vertex buffer and `u32` index buffer.
2. Call `meshopt::simplify` with `target_count` and `target_error` derived from CLI arguments.
3. Reconstruct a `MeshIR` from the simplified buffers.
4. Run a single pass of Phase 2 (orientation normalization) from the repair module to correct any winding inconsistencies introduced by edge collapse.

Coordinates are converted to `f32` for meshopt processing and back to `i64` (internal units) on output. Precision loss from `f32` rounding is bounded by the meshopt error budget and is acceptable at typical decimation ratios.

### Configuration

| Parameter      | Type    | Default | Description                                                                                                |
|----------------|---------|---------|------------------------------------------------------------------------------------------------------------|
| `target_count` | `usize` | —       | Absolute target triangle count. Mutually exclusive with `target_ratio`.                                    |
| `target_ratio` | `f32`   | —       | Fraction of original count to retain (0.0–1.0). Mutually exclusive with `target_count`.                    |
| `max_error`    | `f32`   | `0.01`  | Maximum allowed quadric error in internal units. Decimation stops early if this would be exceeded.         |
| `aggressive`   | `bool`  | `false` | Use `simplify_sloppy` instead of `simplify`. Faster but may produce lower-quality results near boundaries. |

Exactly one of `target_count` or `target_ratio` must be specified.

### Output

```rust
pub struct DecimateResult {
    pub mesh: MeshIR,
    pub original_triangle_count: usize,
    pub final_triangle_count: usize,
    pub achieved_error: f32,
}
```

### Public API

```rust
pub fn decimate(mesh: MeshIR, config: DecimateConfig) -> Result<DecimateResult, DecimateError>
```

### CLI Subcommand: `pnp decimate`

```
pnp decimate --input <path> --output <path>
             (--target-count <n> | --target-ratio <0.0–1.0>)
             [--max-error <f32>]
             [--aggressive]
             [--stats]

Options:
  --input          Input mesh file
  --output         Output mesh file path
  --target-count   Absolute target triangle count
  --target-ratio   Fraction of triangles to retain (e.g. 0.25 = keep 25%)
  --max-error      Maximum quadric error budget (default: 0.01)
  --aggressive     Use sloppy simplification (faster, lower quality)
  --stats          Print result statistics to stderr as JSON
```

Exit codes:

| Code | Meaning                                                                           |
|------|-----------------------------------------------------------------------------------|
| 0    | Decimation succeeded; target was reached                                          |
| 1    | Decimation stopped early (max_error budget exhausted before target count reached) |
| 2    | Input file not found or unreadable                                                |
| 3    | Input mesh is empty or has fewer triangles than target                            |

---

## Feature: STEP Import

### Purpose

Converts STEP (ISO 10303) files to triangulated `MeshIR`. STEP is common for mechanical CAD parts (gears, enclosures, brackets) that users may wish to print. Unity has no STEP support; the CLI handles conversion before the mesh is passed to the frontend or pipeline.

### Library: `truck`

STEP import is implemented using the `truck` crate ecosystem (pure Rust CAD kernel):

- `truck-stepio`: STEP AP203/AP214 parser — reads B-Rep solids from `.step`/`.stp` files
- `truck-meshing`: triangulates B-Rep shells into indexed triangle meshes

`truck` was chosen over an OpenCASCADE FFI binding because:

- Pure Rust — no C++ build dependency, cross-compiles cleanly to all target platforms
- AP203 and AP214 coverage is sufficient for mechanical FDM print use cases
- Maintained actively as of 2026
- No LGPL entanglement

**Limitation:** `truck-stepio` does not support AP242 (the newer STEP standard used by Siemens NX and CATIA for assemblies with PMI). If an AP242-specific construct is encountered, the importer emits a non-fatal `StepWarning::UnsupportedSchema` and attempts to parse the geometry portions anyway.

### Pipeline

```
.step / .stp file
       │
       ▼
truck-stepio::read()          — parse STEP entities into B-Rep shell(s)
       │
       ▼
unit normalization             — read LENGTH_UNIT from STEP header,
       │                         apply conversion factor to all vertices
       ▼
truck-meshing::triangulate()  — tessellate each B-Rep shell into
       │                         indexed triangle mesh; tolerance = 100 nm
       ▼
component merging              — if STEP file contains multiple solids,
       │                         each becomes a separate MeshIR (array output)
       ▼
repair pass                    — Phase 1 + Phase 2 of mesh repair applied
       │                         to each component automatically
       ▼
Vec<MeshIR>                   — one MeshIR per solid in the STEP file
```

### Tessellation Tolerance

The triangulation tolerance passed to `truck-meshing` is fixed at **100 nm** (1 internal unit). This matches the coordinate system resolution and ensures no geometric detail finer than 1 internal unit is lost during tessellation.

Finer tolerances produce more triangles without slicing benefit. Coarser tolerances may lose sharp edges on small features. The value is not user-configurable at the CLI level; use `pnp decimate` afterward to reduce triangle count if needed.

### Output

```rust
pub struct StepImportResult {
    pub meshes: Vec<NamedMesh>,
    pub source_unit: StepLengthUnit,
    pub warnings: Vec<StepWarning>,
}

pub struct NamedMesh {
    pub name: Option<String>,   // STEP entity label if present
    pub mesh: MeshIR,
}

pub enum StepLengthUnit {
    Millimetre,
    Metre,
    Inch,
    Micrometre,
    Unknown,                    // triggers default-to-mm warning
}

pub enum StepWarning {
    UnsupportedSchema { schema: String },
    UnknownUnit,
    RepairApplied { component_index: usize, stats: RepairStats },
    MultipleComponents { count: usize },
}
```

### Public API

```rust
/// Import a STEP file. Returns one MeshIR per solid found in the file.
/// Repair (Phase 1 + Phase 2) is applied automatically to each component.
pub fn import_step(path: &Path) -> Result<StepImportResult, StepImportError>
```

### CLI Subcommand: `pnp import`

```
pnp import --input <path.step|path.stp>
           --output <path> [--output-format <stl|obj|3mf>]
           [--merge-components]
           [--no-repair]
           [--stats]

Options:
  --input             Input STEP or STP file
  --output            Output mesh file path. If the STEP file contains multiple
                      solids and --merge-components is not set, output path is
                      used as a stem: <stem>_0.stl, <stem>_1.stl, etc.
  --output-format     Output format (default: stl)
  --merge-components  Merge all solids into a single MeshIR before output
  --no-repair         Skip the automatic repair pass (not recommended)
  --stats             Print import statistics to stderr as JSON
```

Exit codes:

| Code | Meaning                                              |
|------|------------------------------------------------------|
| 0    | Import succeeded; all solids converted               |
| 1    | Import partially succeeded; some solids had warnings |
| 2    | Input file not found or unreadable                   |
| 3    | STEP file contains no recognisable geometry          |
| 4    | Parse error — file is not valid STEP                 |

---

## Error Types (Normative)

Each operation has its own error enum. All errors implement `std::error::Error` and are structured for programmatic consumption by the host.

```rust
pub enum RepairError {
    EmptyMesh,
    IoError(std::io::Error),
}

pub enum DecimateError {
    EmptyMesh,
    InvalidConfig(String),   // e.g. both target_count and target_ratio specified
    IoError(std::io::Error),
}

pub enum StepImportError {
    FileNotFound(PathBuf),
    ParseError(String),
    NoGeometry,
    IoError(std::io::Error),
}
```

Warnings are **not** errors. Operations that produce warnings still return `Ok(result)` with the warnings embedded in the result struct. The CLI prints warnings to stderr and uses exit code 1 to indicate their presence.

---

## Integration with Host CLI

The host binary (`slicer-host`) exposes `repair`, `decimate`, and `import` as top-level subcommands via `clap`. Each subcommand calls directly into the corresponding `slicer-helpers` function. No WASM runtime is initialized for these subcommands.

```
pnp slice    — full slicing pipeline (WASM modules, scheduler)
pnp repair   — slicer-helpers::repair()
pnp decimate — slicer-helpers::decimate()
pnp import   — slicer-helpers::import_step()
```

These subcommands must serialize all progress and result data as line-delimited JSON to stdout, matching the event protocol defined in `./docs/01_system_architecture.md §Inter-Process Communication`, so the Unity frontend can consume them uniformly.

Example output for `pnp repair --stats`:

```jsonc
{"event": "done", "operation": "repair", "degenerate_removed": 14,
 "faces_reoriented": 3, "open_edges_closed": 0, "warnings": []}
```

---

## TDD Contract

Tests must be written and confirmed failing before any implementation begins. Each test file maps to one feature module.

### `tests/repair_tdd.rs`

| Test                                  | Input                               | Expected                                                                |
|---------------------------------------|-------------------------------------|-------------------------------------------------------------------------|
| `repair_removes_degenerate_triangles` | Mesh with 3 zero-area triangles     | `stats.degenerate_removed == 3`                                         |
| `repair_normalizes_flipped_face`      | Cube with one face winding reversed | `stats.faces_reoriented >= 1`, output is manifold                       |
| `repair_closes_open_edge`             | Cube with one face removed          | `stats.open_edges_closed > 0`, output is closed                         |
| `repair_noop_on_clean_mesh`           | Valid cube mesh                     | All stats == 0, output identical to input                               |
| `repair_large_cap_loop_warning`       | Mesh with 300-vertex open boundary  | `RepairWarning::LargeCapLoop` present, `repaired == false` on component |

### `tests/decimate_tdd.rs`

| Test                             | Input                                              | Expected                               |
|----------------------------------|----------------------------------------------------|----------------------------------------|
| `decimate_by_ratio`              | Sphere with 2000 triangles, `target_ratio = 0.5`   | Output has ≤ 1000 triangles            |
| `decimate_by_count`              | Sphere with 2000 triangles, `target_count = 400`   | Output has ≤ 400 triangles             |
| `decimate_respects_error_budget` | Sphere, tight `max_error = 0.001`                  | `achieved_error ≤ 0.001`               |
| `decimate_stops_early`           | Sphere, `target_ratio = 0.01`, `max_error = 0.001` | Exit code 1 (budget hit before target) |
| `decimate_empty_mesh_error`      | Empty `MeshIR`                                     | `Err(DecimateError::EmptyMesh)`        |
| `decimate_conflict_config_error` | Both `target_count` and `target_ratio` set         | `Err(DecimateError::InvalidConfig(_))` |

### `tests/import_step_tdd.rs`

| Test                               | Input                                           | Expected                                           |
|------------------------------------|-------------------------------------------------|----------------------------------------------------|
| `import_step_single_solid`         | `tests/resources/cube.step` (mm units)          | 1 mesh, vertices scaled to internal units          |
| `import_step_unit_metre`           | `tests/resources/cube_metres.step`              | Vertices × 10,000,000 vs mm equivalent             |
| `import_step_multi_solid`          | `tests/resources/assembly.step` (2 solids)      | `result.meshes.len() == 2`                         |
| `import_step_merge_components`     | `tests/resources/assembly.step`, `merge = true` | 1 mesh, combined vertex count                      |
| `import_step_repair_applied`       | `tests/resources/step_open_face.step`           | `StepWarning::RepairApplied` present               |
| `import_step_unknown_unit_warning` | STEP file with no unit declaration              | `StepWarning::UnknownUnit` present, defaults to mm |
| `import_step_not_found_error`      | Non-existent path                               | `Err(StepImportError::FileNotFound(_))`            |
| `import_step_invalid_file_error`   | Binary garbage file                             | `Err(StepImportError::ParseError(_))`              |

Test fixture files required in `tests/resources/`:

- `cube.step` — single 10mm cube, millimetre units
- `cube_metres.step` — same cube, metre units
- `assembly.step` — two distinct solids in one STEP file
- `step_open_face.step` — STEP file whose tessellation produces a non-manifold mesh

---

## Implementation Tasks

These tasks extend the Phase B sequence in `./docs/07_implementation_status.md`.

| Task ID  | Description                                                                                                                  | Phase |
|----------|------------------------------------------------------------------------------------------------------------------------------|-------|
| TASK-055 | Create `crates/slicer-helpers/` workspace member; add `meshopt`, `truck-stepio`, `truck-meshing` to root `Cargo.toml`        | D     |
| TASK-056 | Write failing tests in `repair_tdd.rs`; implement `repair.rs` (all three phases); all tests pass                             | D     |
| TASK-057 | Write failing tests in `decimate_tdd.rs`; implement `decimate.rs` via meshopt; all tests pass                                | D     |
| TASK-058 | Create STEP test fixtures; write failing tests in `import_step_tdd.rs`; implement `import/step.rs` via truck; all tests pass | D     |

TASK-076 in Phase E ("File format loaders + admesh-based mesh repair integration") is superseded by TASK-056 for the repair component. TASK-076 retains responsibility for STL/OBJ/3MF host-side loaders only.
