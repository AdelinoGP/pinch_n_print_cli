# Pinch 'n Print — Project Overview

**What this covers:** the project's goals, the four architectural decisions that
shape everything else, the crate layout, workspace dependency version
requirements, and the index of which doc answers which question.

**Who it's for:** anyone arriving at the project — contributors, module authors,
and reviewers — plus agents needing a first-hop index into `docs/`.

**Prerequisites:** none. This is the entry point. Read
`01_system_architecture.md` next for the pipeline in depth.

## Vision

Pinch 'n Print is a modular FDM 3D printer slicing engine where pipeline features are independently compiled, community-extensible modules where the stage contract permits. The Rust runtime hosts these modules alongside host-built-in pipeline stages. It has zero UI concern.

The primary failure mode of existing slicers (OrcaSlicer, PrusaSlicer) that this project solves:

<!-- VERIFY: These comparative claims are project motivation and are not verifiable from this workspace alone. -->

- Features are tightly coupled to the core, making community contributions require full C++ builds
- Post-processing workarounds (Python G-code scripts) exist because there are no proper pipeline hooks
- Configuration co-dependencies are implicit and fragile
- Adding a new feature can silently break existing features

---

## Goals

| Goal                        | Description                                                                               |
|-----------------------------|-------------------------------------------------------------------------------------------|
| **Modular pipeline**        | Pipeline stages accept independently compiled modules; some stages remain host-built-in  |
| **Stable ABI**              | Typed per-stage WIT packages and explicit host/IR compatibility checks define the boundary |
| **Safe parallelism**        | Per-layer processing uses rayon; each layer has an isolated `LayerArena`                  |
| **Config robustness**       | Module config is schema-driven and module access is scoped to declared keys               |
| **Fast iteration**          | Modules and guest components build independently from the host pipeline                   |
| **Community extensibility** | Modules ship as `.wasm` + `.toml` manifest; no host source access required                |
| **Testability**             | Modules can use the `slicer-sdk` test feature without a running host                      |
| **Clean separation**        | Core engine has zero GUI/frontend code; all UI is a separate process                      |

## Non-Goals

| Non-Goal                | Reason                                                                      |
|-------------------------|-----------------------------------------------------------------------------|
| Hot reload of modules   | Modules are loaded at slice-command startup; iteration cycle is fast enough |
| GUI / preview rendering | Separate frontend process communicates via CLI/socket API                   |
| <!-- VERIFY: SLA/resin support remains a product non-goal; no SLA pipeline is present in the current workspace. --> SLA / resin printing | Pipeline is FDM-first; SLA support is a future module set                   |

---

## Key Architectural Decisions

### Language: Rust (core host)

- Rust's ownership model avoids a tracing garbage collector in the host
- `rayon` for data-parallel layer processing
- `wasmtime` as the embedded WebAssembly Component Model runtime
- `clipper2-rust` for integer polygon operations; geometry algorithms live in `slicer-core`
- Compiles to native binary; no runtime dependency on Rust toolchain for users

### Module Format: WebAssembly (WASM) Component Model

- Typed, per-stage WIT packages define the host/module contract and are checked during component instantiation
- <!-- VERIFY: Language/toolchain breadth beyond the in-tree Rust guests is a design intent; this repository does not verify C, C++, or Python component builds. --> Modules can be written in Rust, C, C++, or any WASM-targeting language (including Python via CPython→WASM toolchains)
- Community modules ship as `.wasm` + `.toml` — no build toolchain required for users

### State Model: Host Blackboard + Per-Layer Arenas

- Whole-print and prepass IR (mesh, layer plan, surface classification, region map, and related products) lives in the host-owned `Blackboard` in `crates/slicer-runtime/src/blackboard.rs`.
- Each layer gets its own `LayerArena` in `crates/slicer-runtime/src/blackboard.rs`; staged IR is committed as a `LayerCollectionIR` for later finalization and G-code emission.
- Modules never own host geometry; they receive scoped IR/WIT views and the host enforces declared reads and writes.
- Per-layer arena state is released when the layer completes; committed layer collections are retained for PostPass.

### Pipeline Shape: DAG of Stages

- The scheduler's declared `STAGE_ORDER` in `crates/slicer-scheduler/src/execution_plan.rs` fixes validation order across the PrePass, Per-Layer, and PostPass tiers; finalization is a separate sequential step inside PostPass execution.
- Within each stage, module execution order is a topologically sorted DAG derived from manifest requirements and IR read/write declarations.
- `validate_startup_dag` in `crates/slicer-scheduler/src/validation.rs` checks stage IDs, claims, compatibility, cycles, and access contracts before execution.

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
| Support-preview JSON contract                             | `20_support_preview.md`                                   |
| Active architecture deviations                            | `DEVIATION_LOG.md`                                        |

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
│   ├── pnp-cli/              # Single binary `pnp_cli`: slice, profile, support-preview, visual-debug, module, mesh, dag
│   ├── pnp-cli-locator/      # Host-side test/bench helper for locating a fresh `pnp_cli`
│   ├── slicer-core/          # Core algorithms (slicing, Clipper ops, geometry)
│   ├── slicer-gcode/         # LayerCollectionIR → GCodeIR → G-code text
│   ├── slicer-model-io/      # STL / OBJ / 3MF ingestion; geometry-only writers
│   ├── slicer-ir/            # IR type definitions (shared between host and SDK)
│   ├── slicer-sdk/           # Module authoring SDK (imported by module crates; test harness under `test` feature)
│   ├── slicer-macros/        # Proc-macros (#[slicer_module], #[module_test])
│   ├── slicer-schema/        # Canonical stage/WIT mapping and WIT contract
│   │   └── wit/              #   The single canonical WIT source (root.wit and deps/)
│   └── slicer-helpers/       # Pre-pipeline mesh ops (repair, decimate, STEP import)
├── modules/
│   └── core-modules/         # Built-in module crates and guest components
├── xtask/                    # Dev tooling: build-guests, dist, test, gen-config-docs, check-deviations, compact-specs
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
| `slicer-wasm-host` | `crates/slicer-wasm-host/` | WIT / wasmtime marshalling and dispatch. Co-locates the per-stage `bindgen!` modules and shared host implementations so mapped WIT types retain Rust type identity — see ADR-0002 and ADR-0045. |
| `pnp_cli` (binary) | `crates/pnp-cli/` | The single CLI binary: `slice`, `profile`, `support-preview`, `visual-debug`, `module`, `mesh`, and `dag` verbs. Entry point `main` in `crates/pnp-cli/src/main.rs`. |
| `pnp-cli-locator` | `crates/pnp-cli-locator/` | Host-side dev/test helper exposing `pnp_cli_bin` and workspace freshness helpers. |
| `slicer-core` | `crates/slicer-core/` | Core algorithms (slicing, Clipper ops, geometry). |
| `slicer-gcode` | `crates/slicer-gcode/` | Pure-IR G-code emission: `LayerCollectionIR` → `GCodeIR` → G-code text. No wasmtime, scheduler, or blackboard dependency. |
| `slicer-model-io` | `crates/slicer-model-io/` | Host-side model ingestion (STL, OBJ, 3MF → `MeshIR`) and geometry-only 3MF/OBJ writers. |
| `slicer-ir` | `crates/slicer-ir/` | IR type definitions shared between host and SDK. |
| `slicer-sdk` | `crates/slicer-sdk/` | Module authoring SDK; module test harness under the `test` feature. |
| `slicer-macros` | `crates/slicer-macros/` | Proc-macros (`#[slicer_module]`, `#[module_test]`). |
| `slicer-schema` | `crates/slicer-schema/` | Canonical stage/WIT mapping and the WIT contract under `crates/slicer-schema/wit/`; scheduler owns manifest parsing. |
| `slicer-helpers` | `crates/slicer-helpers/` | Pre-pipeline mesh ops (repair, decimate, STEP import). |
| `xtask` | `xtask/` | Dev tooling (`build-guests`, `dist`, `test`, `gen-config-docs`, `check-deviations`, `compact-specs`). |

> **Packet 69 rename (history):** the former `slicer-host` library crate was
> renamed to `slicer-runtime`, and the former `slicer-cli` crate was deleted with
> its verbs absorbed into the `pnp_cli` binary. The names `slicer-host` /
> `slicer-cli` survive only in historical records (`docs/DEVIATION_LOG.md`,
> `docs/specs/`) and must not appear as
> live paths in the numbered reference docs.

---

## Technology Stack

Workspace version requirements live in the workspace `Cargo.toml`; resolved
versions live in `Cargo.lock`. The table below records the relevant manifest
requirements and notable lockfile resolutions.

| Component     | Technology                              | Manifest requirement / resolution                                      |
|---------------|-----------------------------------------|-------------------------------------------------------------------------|
| Host language | Rust                                    | 1.91.0 (edition 2021)                                                  |
| WASM runtime  | wasmtime                                | 43.0.0 workspace requirement; 43.0.1 in `Cargo.lock`                   |
| WIT tooling   | wit-bindgen                             | 0.57.1 workspace requirement and primary lockfile resolution           |
| Parallelism   | rayon                                   | 1.80 workspace requirement; 1.10 in runtime/core/wasm-host; 1.11.0 lock |
| Geometry      | clipper2-rust                           | 1.0.3                                                                    |
| Serialization | serde + postcard                        | 1.0.228, 1.1.3                                                          |
| Config format | TOML (manifests), JSON (runtime config) | —                                                                       |
| Testing       | cargo test                              | —                                                                       |
| CLI framework | clap                                    | 4.6.1                                                                    |

---

## Versioning Policy

- <!-- VERIFY: The migration-guide policy is stated by project documentation but is not enforced by the current source tree. --> **Host** follows semver. Major version bumps are rare and announced with migration guides.
- **WIT stage packages** are versioned independently (for example, `slicer:layer-perimeters@1.0.0`).
  The package identity selects the stage contract; typed wasmtime instantiation checks the
  qualified export, while IR schema ranges are checked separately. A change to one stage package
  affects modules for that stage, not every module in its tier. The legacy manifest `wit-world`
  key is tolerated and ignored (`docs/03_wit_and_manifest.md`).
- **IR schemas** carry a `schema_version: SemVer` field. Modules declare minimum and maximum
  compatible versions.
- **Module manifests** declare `min-host-version`. The host rejects modules requiring a newer host.
- **Config keys** use structured runtime namespaces where scope requires them:
  `object_config:<id>:<key>`, `paint_config:<semantic>:<key>`, and
  `tool_config:<tool_index>:<key>`. Module config keys themselves use snake_case; manifest
  metadata fields retain their documented TOML spellings. Core/global keys are bare names.

Operational governance (rollout checklist, compatibility policy, release-blocking architecture gate):

- `11_operational_governance_and_acceptance_gate.md`

---

## Performance Targets

| Metric                                           | Target                        |
|--------------------------------------------------|-------------------------------|
| <!-- VERIFY: docs/12 defines this bound, but the named Benchy reference fixture is not materialized; current evidence uses `regression_wedge.stl`. --> Slicing a 50-layer benchy (0.2mm layers) | < 10 seconds                  |
| <!-- VERIFY: No current source or gate document provides an evidence source for this per-layer target. --> Per-layer overhead (host scheduler, IR views) | < 5ms per layer               |
| <!-- VERIFY: No current source or gate document provides an evidence source for these WASM boundary targets. --> WASM boundary crossing cost (warm instance, p50) | < 0.5ms per module invocation |
| WASM boundary crossing cost (warm instance, p95) | < 1ms per module invocation   |
| <!-- VERIFY: docs/12 defines the RSS bound, but current instrumentation cannot measure WASM-inclusive peak RSS. --> Peak memory for a 500-layer model | < 512 MB                      |
| <!-- VERIFY: No current source or gate document defines this 20-second / 20-module target. --> Module load + validation at startup | < 20s for 20 modules          |

Operational budgeting note:

- The above targets assume host-call batching and bounded RegionMap/LayerCollection memory strategies as defined in `04_host_scheduler.md`.
- Performance gate fixture definitions and measurement protocol are defined in `12_architecture_gate_metrics.md`.
