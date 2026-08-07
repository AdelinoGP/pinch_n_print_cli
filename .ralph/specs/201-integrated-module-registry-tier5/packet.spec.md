---
status: draft
packet: 201-integrated-module-registry-tier5
task_ids:
  - ADR-0056
backlog_source: docs/specs/multi-edition-distribution-plan.md
context_cost_estimate: M
---

# Packet Contract: 201-integrated-module-registry-tier5

## Goal

Register integrated modules (embedded manifest TOML, no on-disk `.wasm`) as search tier 5 beneath the four existing search-path tiers, flowing through the one existing ingestion/claims/DAG pipeline with a `ModuleProvenance` marker and a provenance-aware shadow diagnostic, per ADR-0056 §1–2.

## Scope Boundaries

This packet touches manifest ingestion (`crates/slicer-scheduler/src/manifest.rs`), the live-plan loader (`crates/slicer-wasm-host/src/execution_plan_live.rs`), the production loader call in `crates/slicer-runtime/src/run.rs`, and a new registry crate `crates/slicer-integrated-modules/`. Native dispatch of integrated modules is packet 202; the `--no-integrated-modules` flag and `pnp_cli module` provenance surfacing are packet 203. The default integrated registry is empty, so every shipped behavior is unchanged.

## Prerequisites and Blockers

- Depends on: nothing (queue row 2 of `docs/specs/multi-edition-distribution-plan.md`; independent of packet 200).
- Unblocks: 202 (native adapter/dispatch), 203 (CLI/provenance), 204 (pilot/parity), 205 (editions).
- Activation blockers: none known; see `design.md` §Open Questions for [FWD] items.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** an `IntegratedModuleRegistration` whose `manifest_toml` is a valid module manifest with a unique `module.id` and **no** `.wasm` file anywhere on disk, **when** `load_modules_from_roots_with_integrated` runs with that registration and no search roots, **then** the report contains exactly one module with that id, `provenance()` returns `ModuleProvenance::Integrated`, `placeholder_wasm()` returns `false`, and no `LoadErrorKind::MissingWasm` error is produced. | `cargo test -p slicer-scheduler --test scheduler_integration integrated_manifest_ingests_without_wasm`
- **AC-2. Given** the same `module.id` provided both by a disk search root (tier 1) and by an integrated registration, **when** `load_modules_from_roots_with_integrated` runs, **then** the surviving module is the disk one with `provenance() == ModuleProvenance::External`, and the integrated entry is dropped (first-root-wins dedup by `module.id` unchanged). | `cargo test -p slicer-scheduler --test scheduler_integration external_root_overrides_integrated_tier`
- **AC-3. Given** the production slice path, **when** `crates/slicer-runtime/src/run.rs` loads live modules, **then** it calls the integrated-aware entry point and sources registrations from the registry crate. | `rg -q 'load_live_modules_for_plan_with_integrated' crates/slicer-runtime/src/run.rs && rg -q 'integrated_registrations' crates/slicer-runtime/src/run.rs` (bare symbol names, so all name-resolution-equivalent call forms match)
- **AC-4. Given** the registry crate built with `--features classic-perimeters`, **when** its embedded-manifest test ingests `integrated_registrations()` through `load_modules_from_roots_with_integrated` with no search roots, **then** the report contains module id `com.core.classic-perimeters` with `provenance() == ModuleProvenance::Integrated`. | `cargo test -p slicer-integrated-modules --features classic-perimeters embedded_classic_perimeters_manifest_ingests`
- **AC-5. Given** an integrated-provenance module that survives dedup, **when** `load_live_modules_for_plan_with_integrated` builds live bindings, **then** the module's `LiveModuleBinding` has `wasm_component: None` and component compilation (`compile_module_component`) is never attempted for it. | `cargo test -p slicer-runtime --test integration integrated_binding_skips_component_compile`

## Negative Test Cases

- **AC-N1. Given** module id `X` provided by a disk root and by an integrated registration, **when** `load_modules_from_roots_with_integrated` runs, **then** `LoadModulesReport.diagnostics` contains a `DiagnosticLevel::Warning` with `field: Some("module.id")` and message exactly `external module 'X' shadows integrated module 'X'` (with `X` substituted), replacing the generic duplicate-id text for this provenance pairing. | `cargo test -p slicer-scheduler --test scheduler_integration external_shadow_diagnostic_names_integrated_loser`
- **AC-N2. Given** an empty integrated registration slice, **when** `load_modules_from_roots_with_integrated(roots, &[])` and `load_modules_from_roots(roots)` both scan `modules/core-modules/`, **then** the two `LoadModulesReport` values are equal (same module ids in the same order, same diagnostics) — the disabled tier is a strict identity. | `cargo test -p slicer-scheduler --test scheduler_integration empty_integrated_registry_is_identity`
- **AC-N3 (doc grep).** Given the doc edits below, both greps pass. | `rg -qi 'tier 5' docs/01_system_architecture.md && rg -q 'ModuleProvenance' docs/04_host_scheduler.md`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-scheduler --test scheduler_integration integrated`

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; read directly; §1–2 are this packet's contract.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; read directly; flag/edition split confirms what stays out of scope.
- `docs/01_system_architecture.md` — §Module Search Path only (lines 912–1015 as of authoring; re-locate by heading); delegate anything else.
- `docs/04_host_scheduler.md` — §Phase 1 Manifest Ingestion heading only; delegate.
- `docs/03_wit_and_manifest.md` — §Module Manifest Schema heading only; delegate.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/01_system_architecture.md` §"Module Search Path": add tier 5 to "Priority tiers (highest first)", extend "Intra-root `module.id` deduplication" with the provenance-aware shadow diagnostic, note the tier is inert when the registry is empty. — `rg -qi 'tier 5' docs/01_system_architecture.md`
- `docs/04_host_scheduler.md` §"Phase 1 — Manifest Ingestion": one short paragraph — ingestion is generalized over manifest source; `LoadedModule` carries `ModuleProvenance`; claims/DAG machinery never inspects it. — `rg -q 'ModuleProvenance' docs/04_host_scheduler.md`

Doc greps are appended to the ACs as AC-N3.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
