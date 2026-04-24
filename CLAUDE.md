# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Build the entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Run clippy lints (required before committing)
cargo clippy --workspace -- -D warnings

# Run a single test
cargo test -p slicer-host --test core_module_ir_access_contract_tdd

# Build WASM core modules (requires wasm32 target)
./modules/core-modules/build-core-modules.sh

# Run the slicer CLI
cargo run --bin slicer-cli --release --slice --input model.stl --output model.gcode
```

## Project Structure

```
pinch_n_print/
├── crates/
│   ├── slicer-host/     # Main host binary, scheduler, WASM executor
│   ├── slicer-core/     # Core algorithms (slicing, clipper, mesh)
│   ├── slicer-ir/       # All IR schema definitions (MeshIR, SliceIR, etc.)
│   ├── slicer-sdk/      # SDK helpers for module authors
│   ├── slicer-macros/   # Procedural macros (#[slicer_module])
│   ├── slicer-schema/   # Config schema validation
│   └── slicer-helpers/  # Polygon/geometry utilities
├── cli/slicer-cli/      # CLI binary
├── modules/core-modules/ # 17 bundled WASM modules (perimeters, infill, support, etc.)
├── wit/                  # WIT world definitions (host-api, world-layer, etc.)
├── docs/                 # Architecture docs (01-14)
└── .ralph/              # Agent config and spec packets
```

## Architecture Overview

ModularSlicer is a WASM-powered 3D printing slicer with a four-phase scheduler:

**Phase 1-3 (Static):** Manifest ingestion → DAG construction → Validation (claim conflicts, cycles, IR versions)
**Phase 4 (Dynamic):** PrePass → Per-Layer (rayon parallel) → PostPass

### Pipeline Tiers

| Tier | Stages | Parallelism |
|------|--------|-------------|
| PrePass | MeshSegmentation → MeshAnalysis → LayerPlanning → PaintSegmentation → RegionMapping | Sequential |
| Per-Layer | Slice → SlicePostProcess → Perimeters → PerimetersPostProcess → Infill → InfillPostProcess → Support → SupportPostProcess → PathOptimization | rayon parallel |
| PostPass | LayerFinalization → GCodeEmit → GCodePostProcess → TextPostProcess | Sequential |

### IR Schemas (in `slicer-ir`)

All IR structs carry `schema_version: SemVer`. Key IRs:
- **MeshIR** — raw triangle mesh with facet paint data
- **SurfaceClassificationIR** — facet classification (normal, overhang, bridge, top/bottom)
- **LayerPlanIR** — global Z-plane sequence, catch-up layers, resolved config per region
- **PaintRegionIR** — per-layer semantic paint polygons (Material, FuzzySkin, SupportEnforcer, SupportBlocker, Custom)
- **SliceIR** → **PerimeterIR** → **InfillIR** → **SupportIR** → **LayerCollectionIR** → **GCodeIR**

### Module System

Modules are `.wasm` + `.toml` manifest pairs. They declare:
- `stage` — which pipeline stage
- `[ir-access].reads/writes` — what IR they access (enforced at runtime)
- `claims` — exclusive capability slots (e.g., `infill-generator`)
- `wit-world` — which WIT world (e.g., `slicer:world-layer@1.0.0`)

### Claim System

Claims prevent two modules from generating the same feature simultaneously:
- `perimeter-generator`, `infill-generator`, `support-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`
- Region overrides allow different holders per region
- Non-transitionable claims (perimeter-generator, seam-placer) must stay stable across layers

### WIT Boundary Rules

- All module access is validated at the WIT boundary
- Undeclared reads = fatal contract error
- Undeclared writes = fatal contract error
- Mesh geometry never crosses the boundary — modules query via host services (raycast, normal, bounds)

## Coordinate System

**1 unit = 100 nm (10⁻⁴ mm)**, NOT 1 nm like OrcaSlicer. Use `Point2::from_mm(x, y)` or `mm_to_units()`. Convert OrcaSlicer constants by dividing by 100.

## Current Development Status

Phase H (end-to-end integration) is active. See `docs/07_implementation_status.md` for the remediation backlog. The Architecture Acceptance Gate is blocked on TASK-120 (Benchy parity), TASK-125 (claim transition matrix), TASK-126 (write conflict ordering), and related tasks.

## Ralph Agent Workflow

This project uses spec packets under `.ralph/specs/` for implementation. Each packet contains `packet.spec.md`, `requirements.md`, `design.md`, and `implementation-plan.md`. The active packet is selected by `status: active` in its `packet.spec.md`. Backpressure gates require `cargo build`, `cargo test`, and `cargo clippy` to pass before packet completion.

## WIT/Type Changes Checklist
When modifying WIT types or interface definitions:
1. Search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type
2. Verify type identity matches across component boundaries (e.g., `list<object-id>` in one file and `list<MeshObjectView>` in another causes linking failures)
3. Run `cargo build --tests` after WIT changes
4. Update both inline WIT and external package references consistently

## Key Docs

- `docs/01_system_architecture.md` — pipeline tiers, data ownership, claim system
- `docs/02_ir_schemas.md` — all IR schema definitions
- `docs/03_wit_and_manifest.md` — WIT worlds, module manifest schema
- `docs/04_host_scheduler.md` — DAG validation, execution phases, error handling
- `docs/10_glossary_and_scenario_traces.md` — glossary, normative scenario traces
- `docs/07_implementation_status.md` — current phase, task backlog, deviation log
