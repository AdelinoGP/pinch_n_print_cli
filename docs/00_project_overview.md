# Pinch 'n Print — Project Overview

**What this covers:** the project's goals, the four architectural decisions that
shape everything else, the crate layout, pinned dependency versions, and the
index of which doc answers which question.

**Who it's for:** anyone arriving at the project — contributors, module authors,
and reviewers — plus agents needing a first-hop index into `docs/`.

**Prerequisites:** none. This is the entry point. Read
`01_system_architecture.md` next for the pipeline in depth.

## Vision

Pinch 'n Print is a high-performance, modular FDM/SLA 3D printer slicing engine where every slicing feature is a first-class, independently compiled, community-extensible module. The core engine acts as a host/runner for these modules. It has zero UI concern.

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
- Modules can be written in Rust, C, C++, or any WASM-targeting language (including Python via CPython→WASM toolchains)
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

- The project glossary is defined in `../CONTEXT.md`; normative edge-case traces are in `10_scenario_traces.md`.

## Normative Document Map (LLM/Reviewer Fast Index)

Use this table as the first-hop index when answering architecture or implementation questions.

Paths below are relative to this file (`docs/`).

| Question type                                             | Canonical doc                                             |
|-----------------------------------------------------------|-----------------------------------------------------------|
| Stage order, ownership, claims, paint propagation         | `01_system_architecture.md`                               |
| IR fields, IDs, config merge, determinism rules           | `02_ir_schemas.md`                                        |
| WIT worlds, manifest contracts, module compatibility      | `03_wit_and_manifest.md`                                  |
| Scheduler validation, DAG execution, RegionMapIR behavior | `04_host_scheduler.md`                                    |
| SDK usage, host service wrappers, test workflow           | `05_module_sdk.md`                                        |
| Packet authoring, preflight gating, and agent orchestration | `../.claude/skills/` (`spec-packet-generator`, `spec-review`, `swarm`) |
| Current sequencing, progress, and gate status             | `07_implementation_status.md`                             |
| Coordinate scaling and porting rules                      | `08_coordinate_system.md`                                 |
| Runtime event schema and ordering guarantees              | `09_progress_events.md`                                   |
| Canonical terms (glossary)                                | `../CONTEXT.md`                                            |
| Scenario traces                                           | `10_scenario_traces.md`                                   |
| Governance and acceptance gate policy                     | `11_operational_governance_and_acceptance_gate.md`        |
| Numeric acceptance thresholds                             | `12_architecture_gate_metrics.md`                         |
| slicer-helpers crate (repair, decimate, STEP import)      | `13_slicer_helpers_crate.md`                              |
| Catalogue of all recognised config keys                   | `15_config_keys_reference.md`                             |
| Slicer HTML debugging report (opt-in)                     | `16_slicer_report.md`                                     |
| Slice timing, DAG, and manifest diagnosis                 | `17_agent_debugging.md`                                   |
| Visual-debug bundles (stage/layer PNG evidence)           | `19_visual_debug.md`                                      |
| Active architecture deviations                            | `DEVIATION_LOG.md`                                        |
| Audit provenance and retired XML crosswalk                | `14_deviation_audit_history.md`                           |

Operational agent orchestration and validation gates live in the repo skills under
`.claude/skills/` (`spec-packet-generator` authors packets, `spec-review` gates them,
`swarm` executes them); architecture conflicts are still resolved by the precedence
order below.

Precedence rule for conflicts:

1. `01_system_architecture.md`, `02_ir_schemas.md`, `03_wit_and_manifest.md`
2. `04_host_scheduler.md`, `09_progress_events.md`
3. `05_module_sdk.md`
4. `00_project_overview.md` and status/governance summaries

---

## Repository Structure

```
pinch_n_print_cli/
├── crates/
│   ├── slicer-runtime/       # Library: pipeline execution, blackboard, run_slice() API (no binary)
│   ├── slicer-scheduler/     # Static planning: manifests, config resolution, DAG build + validate
│   ├── slicer-wasm-host/     # wasmtime/WIT marshalling and dispatch
│   ├── pnp-cli/              # Single binary `pnp_cli`: slice, visual-debug, module, mesh, dag verbs
│   ├── slicer-core/          # Core algorithms (slicing, Clipper ops, geometry)
│   ├── slicer-gcode/         # LayerCollectionIR → GCodeIR → G-code text
│   ├── slicer-model-io/      # STL / OBJ / 3MF ingestion; geometry-only writers
│   ├── slicer-ir/            # IR type definitions (shared between host and SDK)
│   ├── slicer-sdk/           # Module authoring SDK (imported by module crates; test harness under `test` feature)
│   ├── slicer-macros/        # Proc-macros (#[slicer_module], #[module_test])
│   ├── slicer-schema/        # Shared config/manifest schema types + canonical WIT contract
│   │   └── wit/              #   The single canonical WIT source (deps/, root.wit, world-*)
│   └── slicer-helpers/       # Pre-pipeline mesh ops (repair, decimate, STEP import)
├── modules/
│   └── core-modules/         # Built-in modules (arachne walls, rectilinear infill, etc.)
├── xtask/                    # Dev tooling: build-guests, dist, test, gen-config-docs, check-deviations
├── resources/                # STL / 3MF / OBJ test fixtures
└── docs/                     # This documentation set
```

> The phantom top-level `wit/` directory was deleted in packet 72; the canonical
> WIT contract now lives only under `crates/slicer-schema/wit/`. Do not recreate
> the top-level directory.

### Code Map (canonical crate ↔ path identity)

This table is the single authoritative home for crate identity. When a doc cites
a source file, the crate name and path resolve here — do not restate crate
identity elsewhere. Renames change this table once, not every citing doc.

| Crate / binary | Path | Role |
|----------------|------|------|
| `slicer-runtime` (lib) | `crates/slicer-runtime/` | Pipeline execution (prepass / per-layer / postpass), blackboard and layer arenas, host built-ins, `run_slice()` API. Re-exports the `slicer-scheduler` planning APIs. Rust module path `slicer_runtime::`. |
| `slicer-scheduler` | `crates/slicer-scheduler/` | Static planning, wasmtime-free: manifest ingestion, config resolution, DAG construction + validation, execution-plan compilation, DAG-CLI introspection. |
| `slicer-wasm-host` | `crates/slicer-wasm-host/` | WIT / wasmtime marshalling and dispatch. Holds all four `bindgen!` invocations (layer / prepass / finalization / postpass) so they share Rust type identity — see ADR-0002. |
| `pnp_cli` (binary) | `crates/pnp-cli/` | The single CLI binary: `slice`, `visual-debug`, `module`, `mesh`, `dag` verbs. Entry point `crates/pnp-cli/src/main.rs`. |
| `slicer-core` | `crates/slicer-core/` | Core algorithms (slicing, Clipper ops, geometry). |
| `slicer-gcode` | `crates/slicer-gcode/` | Pure-IR G-code emission: `LayerCollectionIR` → `GCodeIR` → G-code text. No wasmtime, scheduler, or blackboard dependency. |
| `slicer-model-io` | `crates/slicer-model-io/` | Host-side model ingestion (STL, OBJ, 3MF → `MeshIR`) and geometry-only 3MF/OBJ writers. |
| `slicer-ir` | `crates/slicer-ir/` | IR type definitions shared between host and SDK. |
| `slicer-sdk` | `crates/slicer-sdk/` | Module authoring SDK; module test harness under the `test` feature. |
| `slicer-macros` | `crates/slicer-macros/` | Proc-macros (`#[slicer_module]`, `#[module_test]`). |
| `slicer-schema` | `crates/slicer-schema/` | Config/manifest schema types **and** the canonical WIT under `crates/slicer-schema/wit/`. |
| `slicer-helpers` | `crates/slicer-helpers/` | Pre-pipeline mesh ops (repair, decimate, STEP import). |
| `xtask` | `xtask/` | Dev tooling (`build-guests`, `dist`, `test`, `gen-config-docs`, `check-deviations`, `compact-specs`). |

> **Packet 69 rename (history):** the former `slicer-host` library crate was
> renamed to `slicer-runtime`, and the former `slicer-cli` crate was deleted with
> its verbs absorbed into the `pnp_cli` binary. The names `slicer-host` /
> `slicer-cli` survive only in historical records (`docs/DEVIATION_LOG.md`,
> `docs/14_deviation_audit_history.md`, `docs/specs/`) and must not appear as
> live paths in the numbered reference docs.

---

## Technology Stack

Pinned versions live in the workspace `Cargo.toml`; the table below records the
minimum/current pin for each component.

| Component     | Technology                              | Pinned version       |
|---------------|-----------------------------------------|----------------------|
| Host language | Rust                                    | 1.91.0 (edition 2021)|
| WASM runtime  | wasmtime                                | 43.0.0               |
| WIT tooling   | wit-bindgen                             | 0.57.1               |
| Parallelism   | rayon                                   | 1.80                 |
| Geometry      | geo, nalgebra                           | 0.28, 0.32           |
| Polygon ops   | clipper2-rust                           | 1.0.3                |
| Serialization | serde + postcard                        | 1.0.228, 1.1.3       |
| Config format | TOML (manifests), JSON (runtime config) | —                    |
| Testing       | cargo test                              | —                    |
| CLI framework | clap                                    | 4.6.1                |

---

## Versioning Policy

- **Host** follows semver. Major version bumps are rare and announced with migration guides.
- **WIT interfaces** are versioned independently (`slicer:world-layer@2.0.0`). The version lives
  solely in the `.wit` `package` line and is a changelog annotation: it is erased from guest
  binaries at compile time and is not part of module identity (`docs/03`). Every world change is
  currently breaking for every module bound to that world, because a guest must satisfy the
  world's entire export surface (`docs/05` §SDK Versioning).
- **IR schemas** carry a `schema_version: SemVer` field. Modules declare minimum required version.
- **Module manifests** declare `min-host-version`. The host rejects modules requiring a newer host.
- **Config keys** contributed by modules are namespaced: `com.community.tpms-infill.density`. Core keys have no namespace prefix.

Operational governance (rollout checklist, compatibility policy, release-blocking architecture gate):

- `11_operational_governance_and_acceptance_gate.md`

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

- The above targets assume host-call batching and bounded RegionMap/LayerCollection memory strategies as defined in `04_host_scheduler.md`.
- Performance gate fixture definitions and measurement protocol are defined in `12_architecture_gate_metrics.md`.
