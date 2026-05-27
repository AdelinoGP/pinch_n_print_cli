# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`docs/` is the canonical source for architecture, IR schemas, WIT contracts, scheduler behavior, and the coordinate system. **Read the relevant doc before answering architecture questions or modifying contracts** — do not rely on summaries here.

## Build & Test Commands

```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings              # required before committing
cargo test -p slicer-host --test core_module_ir_access_contract_tdd   # narrow, targeted run (preferred)
./modules/core-modules/build-core-modules.sh         # build WASM core modules (needs wasm32 target)
cargo run --bin slicer-cli --release --slice --input model.stl --output model.gcode
```

### Benchmarks (slow; not in CI)

```bash
# Native — fast, no WASM needed:
cargo bench -p slicer-core    --bench polygon_ops
cargo bench -p slicer-helpers --bench mesh_ops
# Host:
cargo bench -p slicer-host    --bench pipeline       # instrumentation overhead
cargo bench -p slicer-host    --bench per_stage      # plan-freeze serial-edge helpers
cargo bench -p slicer-host    --bench wasm_modules   # v1 stub; needs ./modules/core-modules/build-core-modules.sh
```

### HTML slicer report (debugging)

```bash
cargo run --bin slicer-host --release -- run \
    --model resources/benchy.stl \
    --module-dir modules/core-modules \
    --output /tmp/out.gcode \
    --report /tmp/slicer-report.html         # opt-in; zero overhead when absent
```

See `docs/16_slicer_report.md` for format, allocator contract, and known v1 limitations.

## Test Discipline (agents must follow)

**Do not run `cargo test --workspace` by default.** The full suite is >1000 tests and takes ≥11 minutes — running it speculatively or "to be safe" wastes time and tokens.

Default to the narrowest test that proves the change:
- A single test:        `cargo test -p <crate> --test <file> -- <test_name> --nocapture`
- One test file:        `cargo test -p <crate> --test <file>`
- One crate:            `cargo test -p <crate>`
- Type-check only:      `cargo check --workspace` (seconds, not minutes)

`cargo test --workspace` is permitted **only** when:
1. The user explicitly asks for it, OR
2. A packet's acceptance ceremony / completion gate (`packet.spec.md` / `implementation-plan.md`) requires it for closure, AND every narrower verification command on that packet has already passed.

When a packet does require it, dispatch it to a sub-agent with a `FACT pass/fail` return — never absorb the full output. See `.claude/skills/swarm/SKILL.md` and `.claude/skills/spec-review/SKILL.md` for the dispatch contract.

## Coordinate System Hazard

**1 unit = 100 nm (10⁻⁴ mm)**, NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` / `mm_to_units()`. Full porting checklist in `docs/08_coordinate_system.md`.

## Config Key Naming Convention

All config key strings in Rust code (both host-side and module-side) must use **snake_case** (underscores), never kebab-case (hyphens).

- Correct: `config.get("apply_to_all")`, `ConfigKey::from("fuzzy_skin.apply_to_all")`
- Wrong:   `config.get("apply-to-all")`, `ConfigKey::from("fuzzy_skin.apply-to-all")`

Module manifest TOML section headers (`[config.schema.apply_to_all]`) already use snake_case. Runtime key strings must match.

## Guest WASM Staleness (MUST follow)

Guest `.wasm` artifacts under `modules/core-modules/*/` and `test-guests/*.component.wasm` are **not** rebuilt by `cargo build` or `cargo test`. Stale guests fail typed instantiation at runtime and surface as test failures that look unrelated to your edits but are not.

**You MUST run both freshness checks before attributing any guest, component, host-integration, or module-dispatch test failure to your changes, to "flaky tests", to "a separate workstream", or to "unrelated infrastructure":**

```bash
./modules/core-modules/build-core-modules.sh --check
./test-guests/build-test-guests.sh --check
```

If either reports `STALE:`, you MUST rebuild (drop the `--check` flag) and re-run the failing test before drawing any conclusion about the failure's cause.

**You MUST run `--check` (and rebuild if stale) after editing any of the following paths**, because the build scripts treat them as guest-WASM inputs:

- `wit/**/*.wit` — invalidates every guest's bindgen output
- `crates/slicer-macros/**`, `crates/slicer-sdk/**`, `crates/slicer-ir/**`, `crates/slicer-schema/**` — universal guest deps baked into every guest `.wasm`
- `modules/core-modules/*/src/**` and `modules/core-modules/*/Cargo.toml` — the `#[slicer_module]` impl bodies
- `modules/core-modules/*/wit-guest/**` — the per-module guest shim
- `test-guests/*/src/**` and `test-guests/*/Cargo.toml` — test guest sources

**Prohibited claims unless `--check` was just run and returned clean:** "the wasm rebuild is a separate workstream", "this is unrelated to my changes", "the build scripts are out of scope for this packet", or any equivalent deflection. Treat a stale guest as your bug until `--check` proves otherwise.

## WIT/Type Changes Checklist

When modifying WIT types or interface definitions:
1. Search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for the affected type.
2. Verify type identity matches across component boundaries (e.g., `list<object-id>` in one file and `list<MeshObjectView>` in another causes linking failures).
3. Run `cargo build --tests` after WIT changes.
4. Update both inline WIT and external package references consistently.

## Ralph Agent Workflow

Implementation work is organized into spec packets under `.ralph/specs/<NN>_<slug>/`, each containing `packet.spec.md`, `requirements.md`, `design.md`, and `implementation-plan.md`. The active packet is the one whose `packet.spec.md` has `status: active` (grep for it). Backpressure gates require `cargo build`, the packet's narrow verification commands, and `cargo clippy` to pass before a packet can be closed; the full `cargo test --workspace` runs only at the packet-close acceptance ceremony, not during implementation iterations (see Test Discipline above).

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
- `docs/17_agent_debugging.md` — agent-facing guide for `slicer-host run --instrument-stderr`, `dag <subcommand>`, and `diagnose`. Paired skill: `.claude/skills/debug-pipeline/SKILL.md`; subagent: `.claude/agents/debug-pipeline.md`.
