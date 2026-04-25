# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`docs/` is the canonical source for architecture, IR schemas, WIT contracts, scheduler behavior, and the coordinate system. **Read the relevant doc before answering architecture questions or modifying contracts** — do not rely on summaries here.

## Build & Test Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings              # required before committing
cargo test -p slicer-host --test core_module_ir_access_contract_tdd   # single test example
./modules/core-modules/build-core-modules.sh         # build WASM core modules (needs wasm32 target)
cargo run --bin slicer-cli --release --slice --input model.stl --output model.gcode
```

## Coordinate System Hazard

**1 unit = 100 nm (10⁻⁴ mm)**, NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` / `mm_to_units()`. Full porting checklist in `docs/08_coordinate_system.md`.

## WIT/Type Changes Checklist

When modifying WIT types or interface definitions:
1. Search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type.
2. Verify type identity matches across component boundaries (e.g., `list<object-id>` in one file and `list<MeshObjectView>` in another causes linking failures).
3. Run `cargo build --tests` after WIT changes.
4. Update both inline WIT and external package references consistently.

## Ralph Agent Workflow

Implementation work is organized into spec packets under `.ralph/specs/<NN>_<slug>/`, each containing `packet.spec.md`, `requirements.md`, `design.md`, and `implementation-plan.md`. The active packet is the one whose `packet.spec.md` has `status: active` (grep for it). Backpressure gates require `cargo build`, `cargo test`, and `cargo clippy` to pass before a packet can be closed.

## Doc Index

Read these directly rather than relying on summaries — they are kept current and authoritative.

- `docs/00_project_overview.md` — vision and scope.
- `docs/01_system_architecture.md` — pipeline tiers, data ownership, claim system, memory model, module search path.
- `docs/02_ir_schemas.md` — every IR struct (`MeshIR`, `LayerPlanIR`, `SliceIR`, `SupportPlanIR`, etc.) with versioning rules.
- `docs/03_wit_and_manifest.md` — WIT worlds, host-boundary enforcement, module manifest TOML schema, config validation.
- `docs/04_host_scheduler.md` — DAG validation, four-phase execution, error handling.
- `docs/05_module_sdk.md` — SDK helpers, `#[slicer_module]` macro, builder lifecycles for module authors.
- `docs/07_implementation_status.md` — current phase, task backlog, deviation log. **Source of truth for what's blocked / in progress.**
- `docs/08_coordinate_system.md` — unit system, Z convention, OrcaSlicer porting hazards.
- `docs/09_progress_events.md` — structured runtime event contract.
- `docs/10_glossary_and_scenario_traces.md` — terminology and normative scenario traces.
- `docs/11_operational_governance_and_acceptance_gate.md` — Architecture Acceptance Gate criteria and release governance.
- `docs/12_architecture_gate_metrics.md` — objective thresholds for the gate.
- `docs/13_slicer_helpers_crate.md` — polygon/geometry utilities in `slicer-helpers`.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — registered deviations from architecture docs.
