# Requirements: 201-integrated-module-registry-tier5

## Packet Metadata

- Grouped task IDs: `ADR-0056` (Decision items 1–2). No `docs/07_implementation_status.md` TASK rows exist for this program — see `docs/specs/multi-edition-distribution-plan.md` §"Backlog anchoring [FWD]"; do not add rows to docs/07 in this packet.
- Backlog source: `docs/specs/multi-edition-distribution-plan.md`, Packet Queue row 2.
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

ADR-0056 decides that a module compiled into the host binary (an *integrated module*, CONTEXT.md glossary) stays a full citizen of the one existing module model: its manifest flows through the same ingestion, claims, DAG validation, and config-schema machinery as any disk module. Today that pipeline is disk-only: `load_modules_from_roots` (`crates/slicer-scheduler/src/manifest.rs`) discovers `*.toml` files, and `ingest_manifest` hard-fails without a same-stem `.wasm` (`ensure_same_stem_wasm_exists`, `LoadErrorKind::MissingWasm`). There is no provenance notion on `LoadedModule`, no tier beneath the four search-path tiers assembled by `assemble_search_roots` (`crates/slicer-scheduler/src/module_search_path.rs`), and no home for embedded manifests. This packet builds exactly that registration layer — nothing about how an integrated module *executes* (packet 202).

## In Scope

- `ModuleProvenance` enum (`External | Integrated`) on `LoadedModule`, defaulting to `External`, set via `LoadedModuleBuilder`; accessor `LoadedModule::provenance()`; re-exported from `slicer_scheduler` and from `slicer_runtime` alongside the existing loader re-exports.
- `IntegratedModuleRegistration { manifest_toml: &'static str, origin_label: &'static str }` in `crates/slicer-scheduler/src/manifest.rs` (registration carries no dispatch information — scheduling never learns "native", ADR-0056 Decision item 1).
- Refactor `ingest_manifest` into a text-source core (`ingest_manifest_text`) so disk and embedded manifests share one parser/validator; integrated entries skip `ensure_same_stem_wasm_exists` and `is_placeholder_wasm` (`placeholder_wasm = false`).
- `load_modules_from_roots_with_integrated(search_roots, integrated)` — disk roots first (tiers 1–4), integrated registrations last (tier 5), one shared `seen_ids` dedup; existing `load_modules_from_roots` delegates with an empty slice.
- Provenance-aware duplicate-id diagnostic: when the dedup loser is integrated and the winner external, the warning message becomes `external module 'X' shadows integrated module 'X'`; all other pairings keep the existing generic text.
- New workspace crate `crates/slicer-integrated-modules/`: per-module cargo features (named after the module directory, e.g. `classic-perimeters`) each gating an `include_str!` of that module's manifest TOML; `integrated_registrations() -> Vec<IntegratedModuleRegistration>`; default features empty. One feature-gated proving entry (`classic-perimeters`) with a feature-gated test.
- `load_live_modules_for_plan_with_integrated(...)` in `crates/slicer-wasm-host/src/execution_plan_live.rs`: the real body of `load_live_modules_for_plan_profiled` plus an `integrated` parameter; integrated-provenance modules get `LiveModuleBinding { wasm_component: None, .. }` and never reach `compile_module_component` (dispatching such a module hits the existing `DispatchPhase::MissingComponent` loud-failure path until 202 routes it natively).
- `crates/slicer-runtime/src/run.rs`: both live-loader call sites switch to the integrated-aware entry point, sourcing `slicer_integrated_modules::integrated_registrations()`.
- Doc edits per `packet.spec.md` §Doc Impact.

## Out of Scope

- Native execution of integrated modules, `#[slicer_module]` changes, dispatch routing, `NativeStageEntry` (packet 202; 202 will extend `load_live_modules_for_plan_with_integrated`'s signature with a native-entry table — do not pre-build it here).
- `--no-integrated-modules` CLI flag, `SliceRunOptions` changes, and provenance in `pnp_cli module` listing/diagnose/config-schema (packet 203; until then the `pnp_cli module` subcommands see only external modules).
- Pilot module integration, parity tests (204); edition dimension in `cargo xtask dist`, CI artifacts (205).
- Any change to `assemble_search_roots` / `assemble_search_roots_with` (tier 5 is not a filesystem root; it enters at the loader, not the root assembler), to `--no-default-module-paths` semantics, or to `dedup_same_claim_modules_with_wall_generator` / `ExecutionPlanError::DuplicateModuleBinding` (`crates/slicer-scheduler/src/execution_plan.rs`) / `validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`).
- Editing `docs/07_implementation_status.md`, `CONTEXT.md`, any `docs/adr/*`, or the plan file.

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — short; direct read.
- `docs/01_system_architecture.md` — long; §Module Search Path range only; delegate anything else.
- `docs/04_host_scheduler.md` — long; delegate (only §Phase 1 heading is edited).
- `docs/03_wit_and_manifest.md` — long; delegate §Module Manifest Schema facts if needed.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`. Refinement to AC-2: the surviving disk module must be byte-identical to what a plain `load_modules_from_roots` run yields for the same root.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: 202 consumes `ModuleProvenance`, `IntegratedModuleRegistration`, `load_modules_from_roots_with_integrated`, `load_live_modules_for_plan_with_integrated` (signature to be extended by 202), and the `slicer-integrated-modules` crate/feature scheme; 203 consumes the empty-slice disable seam (flag ⇒ pass `&[]`) and provenance accessors; 205 composes editions from the per-module feature list.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-scheduler --test scheduler_integration integrated_manifest_ingests_without_wasm` | AC-1 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-scheduler --test scheduler_integration external_root_overrides_integrated_tier` | AC-2 | FACT pass/fail |
| `rg -q 'load_live_modules_for_plan_with_integrated' crates/slicer-runtime/src/run.rs && rg -q 'integrated_registrations' crates/slicer-runtime/src/run.rs` | AC-3 | FACT pass/fail (exit code) |
| `cargo test -p slicer-integrated-modules --features classic-perimeters embedded_classic_perimeters_manifest_ingests` | AC-4 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration integrated_binding_skips_component_compile` | AC-5 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration external_shadow_diagnostic_names_integrated_loser` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration empty_integrated_registry_is_identity` | AC-N2 | FACT pass/fail |
| `rg -qi 'tier 5' docs/01_system_architecture.md && rg -q 'ModuleProvenance' docs/04_host_scheduler.md` | AC-N3 doc greps | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration` | manifest-ingestion regression (existing suite unchanged) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration live_module_loading` | live-loader regression | FACT pass/fail |
| `cargo check --workspace --all-targets` | whole-tree compile incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- Steps 1–2 (scheduler loader) must land before Step 4 (wasm-host live path) and Step 5 (run.rs wiring); Step 3 (registry crate) must land before Step 5.
- The shadow-diagnostic message string is written once in `manifest.rs` and asserted verbatim in the AC-N1 test — if wording changes during implementation, change both in the same step; do not weaken the assertion to a substring of convenience.
- `cargo test` output must tee to `target/test-output.log` per `CLAUDE.md` §Test Discipline; read the log instead of re-running.

## Context Discipline Notes

- `crates/slicer-scheduler/src/manifest.rs` is long (several times the 600-line direct-read cap) — ranged reads only: the loader region (`load_modules_from_roots` through `ensure_same_stem_wasm_exists`, roughly lines 566–860 at authoring time) and the `LoadedModule`/builder region (roughly lines 28–400). Never load the whole file.
- `crates/slicer-wasm-host/src/execution_plan_live.rs` — short (353 lines at authoring); locate the entry points and compile loop by symbol (`rg -n '^pub fn|compile_module_component'`), not by a pinned range.
- `docs/01_system_architecture.md` / `docs/04_host_scheduler.md` — edit by heading anchor; do not read either file end-to-end.
- Feature-gated test hazard: the `slicer-integrated-modules` test compiles to zero tests without `--features classic-perimeters` (same silent-green class as `CLAUDE.md` §"Feature-gated test files report green"); the AC command pins the feature explicitly — never drop it.
