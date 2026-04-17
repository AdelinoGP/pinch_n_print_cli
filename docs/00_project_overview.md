# ModularSlicer — Project Overview

## Vision

ModularSlicer is a high-performance, modular FDM/SLA 3D printer slicing engine where every slicing feature is a first-class, independently compiled, community-extensible module. The core engine acts as a host/runner for these modules. It has zero UI concern.

The primary failure mode of existing slicers (OrcaSlicer, PrusaSlicer) that this project solves:

- Features are tightly coupled to the core, making community contributions require full C++ builds
- Post-processing workarounds (Python G-code scripts) exist because there are no proper pipeline hooks
- Configuration co-dependencies are implicit and fragile
- Adding a new feature can silently break existing features

---

## Goals

| Goal                        | Description                                                                               |
|-----------------------------|-------------------------------------------------------------------------------------------|
| **Modular pipeline**        | Every slicing feature is a separate compiled module assigned to a specific pipeline stage |
| **Stable ABI**              | Modules compiled once run on any future host version within the same major version        |
| **Safe parallelism**        | Per-layer processing parallelized via rayon; modules cannot cause data races              |
| **Config robustness**       | Adding/removing a module never breaks existing configurations                             |
| **Fast iteration**          | Modules compile independently; no full-project rebuild needed                             |
| **Community extensibility** | Modules ship as `.wasm` + `.toml` manifest; no host source access required                |
| **Testability**             | Every module is unit-testable in isolation without a running host                         |
| **Clean separation**        | Core engine has zero GUI/frontend code; all UI is a separate process                      |

## Non-Goals

| Non-Goal                | Reason                                                                      |
|-------------------------|-----------------------------------------------------------------------------|
| Hot reload of modules   | Modules are loaded at slice-command startup; iteration cycle is fast enough |
| GUI / preview rendering | Separate frontend process communicates via CLI/socket API                   |
| SLA resin printing      | Pipeline is FDM-first; SLA support is a future module set                   |

---

## Key Architectural Decisions

### Language: Rust (core host)

- Zero GC pauses — critical for predictable per-layer timing
- `rayon` for data-parallel layer processing
- `wasmtime` as the embedded WASM runtime
- `nalgebra` / `geo` for geometry
- Compiles to native binary; no runtime dependency on Rust toolchain for users

### Module Format: WebAssembly (WASM) Component Model

- Stable ABI across compiler versions, platforms, and languages
- Modules can be written in Rust, C, C++, or any WASM-targeting language
- Python modules supported via a host-side Python bridge (for GCode post-processing tier)
- Community modules ship as `.wasm` + `.toml` — no build toolchain required for users

### State Model: ECS-inside-Blackboard

- Global state (mesh, layer plan, surface classification) lives in a host-owned Blackboard
- Per-layer state is modeled as an ECS world (layer = entity, sliced data = components)
- Modules never own geometry; they receive scoped borrow tokens from the host
- All geometry allocated in per-layer arenas; freed after each layer completes

### Pipeline Shape: DAG of Stages

- Fixed stage ordering (PrePass → Per-Layer → PostPass)
- Within each stage, module execution order is a topologically-sorted DAG derived from IR read/write declarations
- Full DAG validation at startup — zero runtime surprises mid-slice

---

## Terminology (Canonical)

- Detailed glossary plus normative edge-case traces are defined in `./docs/10_glossary_and_scenario_traces.md`.

## Normative Document Map (LLM/Reviewer Fast Index)

Use this table as the first-hop index when answering architecture or implementation questions.

| Question type                                             | Canonical doc                                             |
|-----------------------------------------------------------|-----------------------------------------------------------|
| Stage order, ownership, claims, paint propagation         | `./docs/01_system_architecture.md`                        |
| IR fields, IDs, config merge, determinism rules           | `./docs/02_ir_schemas.md`                                 |
| WIT worlds, manifest contracts, module compatibility      | `./docs/03_wit_and_manifest.md`                           |
| Scheduler validation, DAG execution, RegionMapIR behavior | `./docs/04_host_scheduler.md`                             |
| SDK usage, host service wrappers, test workflow           | `./docs/05_module_sdk.md`                                 |
| Live agent orchestration and role instructions            | `../ralph.yml`                                            |
| Current sequencing, progress, and gate status             | `./docs/07_implementation_status.md`                      |
| Coordinate scaling and porting rules                      | `./docs/08_coordinate_system.md`                          |
| Runtime event schema and ordering guarantees              | `./docs/09_progress_events.md`                            |
| Canonical terms and scenario traces                       | `./docs/10_glossary_and_scenario_traces.md`               |
| Governance and acceptance gate policy                     | `./docs/11_operational_governance_and_acceptance_gate.md` |
| Numeric acceptance thresholds                             | `./docs/12_architecture_gate_metrics.md`                  |
| slicer-helpers crate (repair, decimate, STEP import)      | `./docs/13_slicer_helpers_crate.md`                       |
| Active architecture deviations                            | `./docs/DEVIATION_LOG.md`                                 |
| Audit provenance and retired XML crosswalk                | `./docs/14_deviation_audit_history.md`                    |

Operational agent orchestration and validation gates live in `../ralph.yml`; architecture
conflicts are still resolved by the precedence order below.

Precedence rule for conflicts:

1. `01_system_architecture.md`, `02_ir_schemas.md`, `03_wit_and_manifest.md`
2. `04_host_scheduler.md`, `09_progress_events.md`
3. `05_module_sdk.md`
4. `00_project_overview.md` and status/governance summaries

---

## Repository Structure

```
modular-slicer/
├── crates/
│   ├── slicer-host/          # Main binary: CLI, WASM runtime, scheduler
│   ├── slicer-core/          # Core algorithms (slicing, Clipper ops, geometry)
│   ├── slicer-ir/            # IR type definitions (shared between host and SDK)
│   ├── slicer-sdk/           # Module authoring SDK (imported by module crates)
│   ├── slicer-test/          # Test harness for module unit tests
│   └── slicer-macros/        # Proc-macros (#[slicer_module], #[test_module])
├── cli/
│   └── slicer-cli/           # `slicer new`, `slicer build`, `slicer test`, `slicer validate`
├── modules/
│   ├── core-modules/         # Built-in modules (arachne walls, rectilinear infill, etc.)
│   └── example-modules/      # Reference implementations for SDK documentation
├── wit/
│   ├── deps/                 # Shared WIT type definitions
│   ├── world-layer.wit       # Per-layer module world
│   ├── world-prepass.wit     # PrePass module world
│   └── world-postpass.wit    # PostPass module world
├── ./docs/                     # This documentation set
└── tests/
    ├── integration/          # Full-pipeline integration tests
    └── fixtures/             # STL/3MF test models
```

---

## Technology Stack

| Component     | Technology                              | Version              |
|---------------|-----------------------------------------|----------------------|
| Host language | Rust                                    | 1.78+ (edition 2021) |
| WASM runtime  | wasmtime                                | 20.0+                |
| WIT tooling   | wit-bindgen                             | 0.24+                |
| Parallelism   | rayon                                   | 1.10+                |
| Geometry      | geo, nalgebra                           | latest stable        |
| Polygon ops   | clipper2-rust                           | latest stable        |
| Serialization | serde + bincode                         | latest stable        |
| Config format | TOML (manifests), JSON (runtime config) | —                    |
| Testing       | cargo test + nextest                    | —                    |
| Python bridge | pyo3 + wasmtime-py                      | —                    |
| CLI framework | clap                                    | 4.x                  |

---

## Versioning Policy

- **Host** follows semver. Major version bumps are rare and announced with migration guides.
- **WIT interfaces** are versioned independently (`slicer:world-layer@1.0.0`). Minor bumps are additive.
- **IR schemas** carry a `schema_version: SemVer` field. Modules declare minimum required version.
- **Module manifests** declare `min-host-version`. The host rejects modules requiring a newer host.
- **Config keys** contributed by modules are namespaced: `com.community.tpms-infill.density`. Core keys have no namespace prefix.

Operational governance (rollout checklist, compatibility policy, release-blocking architecture gate):

- `./docs/11_operational_governance_and_acceptance_gate.md`

---

## Performance Targets

| Metric                                           | Target                        |
|--------------------------------------------------|-------------------------------|
| Slicing a 50-layer benchy (0.2mm layers)         | < 10 seconds                  |
| Per-layer overhead (host scheduler, IR views)    | < 5ms per layer               |
| WASM boundary crossing cost (warm instance, p50) | < 0.5ms per module invocation |
| WASM boundary crossing cost (warm instance, p95) | < 1ms per module invocation   |
| Peak memory for a 500-layer model                | < 512 MB                      |
| Module load + validation at startup              | < 20s for 20 modules          |

Operational budgeting note:

- The above targets assume host-call batching and bounded RegionMap/LayerCollection memory strategies as defined in `./docs/04_host_scheduler.md`.
- Performance gate fixture definitions and measurement protocol are defined in `./docs/12_architecture_gate_metrics.md`.
