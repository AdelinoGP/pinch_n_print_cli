---
status: implemented
packet: 203-integrated-cli-provenance
task_ids:
  - ADR-0056
  - ADR-0057
backlog_source: docs/specs/multi-edition-distribution-plan.md
context_cost_estimate: M
---

# Packet Contract: 203-integrated-cli-provenance

## Goal

Add the `--no-integrated-modules` flag (slice verb plus every manifest-loading CLI verb) that disables the integrated tier entirely via packet 201's disable seam (pass `&[]` registrations and no native entries), and surface module provenance — integrated vs external, plus the shadow diagnostic — in `pnp_cli module diagnose`, `module config-schema`, and the `dag` verbs, per ADR-0056 consequences and ADR-0057.

## Scope Boundaries

This packet touches the clap verb tree and loader helpers in `crates/pnp-cli/src/main.rs`, the `SliceRunOptions` struct and the integrated-tier seam in `crates/slicer-runtime/src/run.rs`, and `run_diagnose` in `crates/slicer-runtime/src/diagnose.rs`. Pilot-module integration and parity gates are packet 204; editions/xtask are 205; the `SupportPreview` verb and `prepare_prepass_context` keep their current loader behavior (see `requirements.md` §Out of Scope).

## Prerequisites and Blockers

- Depends on: `201-integrated-module-registry-tier5` and `202-native-adapter-and-dispatch` (both draft at authoring — FORWARD-DEP: `ModuleProvenance`, `IntegratedModuleRegistration`, `load_modules_from_roots_with_integrated`, `load_live_modules_for_plan_with_integrated`, `slicer_integrated_modules::{integrated_registrations, native_entries}`, the `classic-perimeters` registry feature, the shadow-diagnostic string, and the documented disable seam `&[]`; names/shapes reconciled against 201/202's specs and the plan's Exports ledger).
- Unblocks: 205 (editions/xtask; needs the flag for edition verification).
- Activation blockers: 201 and 202 must be implemented first (this packet consumes their loader entry points and diagnostics contract).

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the built CLI, **when** `pnp_cli slice --help` runs, **then** the help text lists `--no-integrated-modules`, and the flag reaches the runtime as the `SliceRunOptions.no_integrated_modules` field bound in the `Cmd::Slice` arm. | `cargo run -p pnp-cli --bin pnp_cli -- slice --help | rg -q -- '--no-integrated-modules' && rg -q 'no_integrated_modules' crates/slicer-runtime/src/run.rs && rg -q 'no_integrated_modules' crates/pnp-cli/src/main.rs` (bare-token greps, so all name-resolution-equivalent binding forms match)
- **AC-2. Given** `pnp_cli` built with `--features integrated-classic-perimeters` and fresh guest WASM under `modules/core-modules/`, **when** `slice --model resources/test_stl/ASCII/20mmbox-LF.stl --module-dir modules/core-modules --no-default-module-paths` runs twice — once without and once with `--no-integrated-modules` (`SLICER_MODULE_PATH` cleared) — **then** both runs exit `0`; the first run's stderr contains `shadows integrated module com.core.classic-perimeters` and the second run's stderr contains no occurrence of `shadows integrated module`. | `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd slice_flag_disables_integrated_tier`
- **AC-3. Given** the same feature build and no search roots (`--no-default-module-paths`, no `--module-dir`, `SLICER_MODULE_PATH` cleared), **when** `pnp_cli module diagnose` runs, **then** it exits `0` and its stdout JSON has `"modules_loaded": 1` and a `modules` array containing exactly one entry `{"id": "com.core.classic-perimeters", "provenance": "integrated"}`. | `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd diagnose_lists_integrated_provenance`
- **AC-4. Given** the same feature build and no search roots, **when** `pnp_cli dag stages --no-default-module-paths` runs, **then** stdout contains stage id `Layer::Perimeters` (a `StageSummary.id` value in `run_dag_stages` output, `crates/slicer-scheduler/src/dag_cli.rs`); **and when** re-run with `--no-integrated-modules`, stdout contains no `Layer::Perimeters`. | `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd dag_stages_sees_integrated_tier`
- **AC-5. Given** the same feature build and no search roots, **when** `pnp_cli module config-schema --no-default-module-paths` runs, **then** stdout JSON has a `schema` array containing an entry whose **`module`** field parses to exactly `com.core.classic-perimeters`; **and when** re-run with `--no-integrated-modules`, stdout contains no occurrence of the substring `classic-perimeters`. (Shape verified at authoring: `build_config_schema_json` (`crates/slicer-scheduler/src/manifest.rs`) emits `{"schema_version": …, "schema": [{"module": m.id, "fields": [...]}]}` — the key is `module`, not `name`, and the value is the manifest `id`, not the module directory name. Assert on the parsed field per Step 4's parsed-value rule; the negative half stays a substring check.) | `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd config_schema_includes_integrated_module`

AC verification command rule: AC-2 through AC-5 and AC-N1/N2 run through the new `crates/pnp-cli/tests/integrated_provenance_tdd.rs` binary, authored by this packet (documented in `requirements.md` §In Scope) with `required-features = ["integrated-classic-perimeters"]`; pnp-cli tests drive the real binary via `assert_cmd` (existing pattern: `crates/pnp-cli/tests/slice_cancel_tdd.rs`, `m73_progress_tdd.rs`), so the binary can drive every asserted behavior. Without the feature the test target is skipped silently — the AC commands therefore always pass `--features integrated-classic-perimeters` (see `design.md` §Architecture Constraints on green-blindness).

## Negative Test Cases

- **AC-N1. Given** the feature build and no search roots, **when** `pnp_cli module diagnose --no-default-module-paths --no-integrated-modules` runs, **then** stdout JSON has `"modules_loaded": 0`, an empty `modules` array, and no occurrence of `com.core.classic-perimeters` — the flag removes the integrated tier from the assembled module set entirely. | `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd no_integrated_modules_empties_diagnose`
- **AC-N2. Given** the feature build and `--module-dir <workspace>/modules/core-modules` (fresh guests on disk), **when** `pnp_cli module diagnose --no-default-module-paths --module-dir …` runs, **then** the `modules` array lists `com.core.classic-perimeters` exactly once with `"provenance": "external"` (the external copy won first-root-wins dedup), and `diagnostics` contains a `"level": "warning"` entry whose message is exactly `external module com.core.classic-perimeters shadows integrated module com.core.classic-perimeters`. | `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd diagnose_shows_external_shadowing_integrated`
- **AC-N3 (doc grep). Given** the doc edits below, both greps pass. | `rg -q -- '--no-integrated-modules' docs/17_agent_debugging.md && rg -qi 'provenance' docs/17_agent_debugging.md`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd`

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; read directly; Decision item 2 (tier/dedup) and the provenance-aware-diagnostic consequence are this packet's contract.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; read directly; defines the flag's semantics and its composition with `--no-default-module-paths`.
- `docs/17_agent_debugging.md` — 287 lines; read only §"Diagnose" and §"DAG introspection" (the two edited sections); delegate anything else.
- `docs/spec_packets/201-integrated-module-registry-tier5/packet.spec.md` and `docs/spec_packets/202-native-adapter-and-dispatch/packet.spec.md` — direct read (FORWARD-DEP contracts); their other files only via SUMMARY dispatch.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/17_agent_debugging.md` §"Diagnose": document the new `modules` array (`id` + `provenance` per surviving module), the `--no-integrated-modules` flag, and that the provenance-aware shadow warning appears in `diagnostics`. — `rg -q -- '--no-integrated-modules' docs/17_agent_debugging.md`
- `docs/17_agent_debugging.md` §"DAG introspection": extend the flag list ("All `dag` subcommands take …") with `--no-integrated-modules`. — `rg -qi 'provenance' docs/17_agent_debugging.md`

Doc greps are appended to the ACs as AC-N3. The CLI help surface itself is owned by the clap doc comments edited in `crates/pnp-cli/src/main.rs` (AC-1); no other doc changes.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
