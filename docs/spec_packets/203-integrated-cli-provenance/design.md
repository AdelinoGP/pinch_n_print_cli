# Design: 203-integrated-cli-provenance

## Controlling Code Paths

- Primary code path: `Cmd::Slice` arm → `SliceRunOptions` → `run_slice`'s live-loader call region (`crates/slicer-runtime/src/run.rs`, currently `load_live_modules_for_plan_profiled`, becoming `load_live_modules_for_plan_with_integrated` per 201/202 — FORWARD-DEP); secondary paths: `load_dag_modules` + `dag_producers` + the `ModuleCmd::ConfigSchema` arm (`crates/pnp-cli/src/main.rs`), and `run_diagnose` (`crates/slicer-runtime/src/diagnose.rs`).
- Neighboring tests/fixtures: `crates/pnp-cli/tests/slice_cancel_tdd.rs` / `m73_progress_tdd.rs` (binary-driving `assert_cmd` pattern), `crates/pnp-cli/tests/module_search_path_tdd.rs` (search-path semantics), `resources/test_stl/ASCII/20mmbox-LF.stl` (small slice fixture used by `e2e_integration_tdd.rs`).
- OrcaSlicer comparison: none — no OrcaSlicer behavior is involved; the orca-delegation snippet is deliberately absent.

## Architecture Constraints

- **Flag composition:** ADR-0057 §Decision states exactly two things here — `--no-integrated-modules` "disables the integrated tier entirely", and `--no-default-module-paths` "keeps its current meaning (drops the config-dir and exe-dir tiers only); the flags compose". Everything below is a **derived consequence**, not ADR text, and its basis is named so a reviewer can re-check it:
  - *Derived from ADR-0057 + `assemble_search_roots` (`crates/slicer-scheduler/src/module_search_path.rs`):* `--no-default-module-paths` drops the config-dir and exe-dir tiers and does NOT touch tier 5; `--no-integrated-modules` drops only tier 5 (by passing empty registrations — 201's documented disable seam — and, on the slice path, no native entries). Since each flag names a disjoint tier set, **neither implies the other**.
  - *Derived from the tier list in `assemble_search_roots`:* the `SLICER_MODULE_PATH` env tier is neither a default-path tier nor tier 5, so it is untouched by both flags. Consequence for this packet: every new test must clear that variable (`assert_cmd` `.env_remove("SLICER_MODULE_PATH")`), or a developer's exported path silently injects external modules. Re-derive the tier list from `assemble_search_roots` at implementation time rather than trusting this bullet.
- **Disable = empty inputs, not a new code path (packet 201's contract):** the loader entry points never learn about the flag; disabling is achieved purely by what the callers pass (`&[]`). Do not add a `bool` parameter to any 201/202 loader signature. Source: `201/design.md` §Code Change Surface item 5 — "add an entry point, never a parameter, to keep existing call sites untouched" — and `201/packet.spec.md` AC-N2, which makes `load_modules_from_roots_with_integrated(roots, &[])` a strict identity with `load_modules_from_roots(roots)`. (This rule is **not** in ADR-0056; Decision item 1 "One model" is about manifest ingestion via `include_str!`. No ADR is contradicted, so no `docs/DEVIATION_LOG.md` row is owed.)
- **Feature-gated test green-blindness (CLAUDE.md §Feature-gated test files):** `integrated_provenance_tdd` carries `required-features = ["integrated-classic-perimeters"]`, so a bare `cargo test -p pnp-cli` skips it silently and prints a clean green wall. Every AC command therefore spells the `--features` flag; the acceptance ceremony must use those exact commands, never the bare form.
- **Guest-artifact precondition (CLAUDE.md §Guest WASM Staleness):** this packet edits no path that feeds guest WASM, so no rebuild is triggered by its changes — but AC-2 slices with `--module-dir modules/core-modules` and AC-N2's diagnose requires every manifest's companion `.wasm` to exist on disk (`load_modules_from_roots` hard-errors on a missing companion — see docs/17 §Diagnose exit code 2). Run `cargo xtask build-guests --check` (rebuild if `STALE:`) before attributing any AC-2/AC-N2 failure to this packet's edits.
- **Schema/version constants:** none touched. The diagnose JSON is a CLI output contract documented only in `docs/17_agent_debugging.md`, not a versioned IR schema; adding the `modules` array is additive and consumers per docs/17 parse named fields.

## Code Change Surface

- Selected approach: plumb one `bool` end to end; keep provenance rendering local to the display layer.
  1. `crates/pnp-cli/src/main.rs`: add `#[arg(long = "no-integrated-modules")] no_integrated_modules: bool` to `Cmd::Slice`, `ModuleCmd::Diagnose`, `ModuleCmd::ConfigSchema`, `DagCmd::{Stages, Stage, Depends, Claims}` (doc comment = the help surface: "Disable the integrated-module tier (tier 5) entirely."). Add a file-local helper `cli_integrated_registrations(no_integrated_modules: bool)` returning `slicer_integrated_modules::integrated_registrations()` or empty; `load_dag_modules` gains the third parameter and calls `load_modules_from_roots_with_integrated(&search_roots, &regs)`; the `ConfigSchema` arm does the same inline.
  2. `crates/slicer-runtime/src/run.rs`: add `pub no_integrated_modules: bool` to `SliceRunOptions` (doc comment cites ADR-0057); at the live-loader call region, compute the integrated inputs (`integrated_registrations()` / `native_entries()` from `slicer_integrated_modules`, or empty when the flag is set) and pass them to `load_live_modules_for_plan_with_integrated` — exact argument order re-derived from the landed 201/202 signature at implementation time.
  3. `crates/slicer-runtime/src/diagnose.rs`: `run_diagnose(module_dir, no_default_module_paths, no_integrated_modules)`; switch `load_modules_from_roots` → `load_modules_from_roots_with_integrated`; extend `DiagnoseOut` with `modules: Vec<DiagnoseModuleOut>` where `DiagnoseModuleOut { id: String, provenance: &'static str }` and provenance maps `ModuleProvenance::Integrated → "integrated"`, `::External → "external"` (local `match` — do not add serde/Display to 201's enum).
  4. `crates/pnp-cli/Cargo.toml`: dependency `slicer-integrated-modules = { path = "../slicer-integrated-modules" }`; feature `integrated-classic-perimeters = ["slicer-integrated-modules/classic-perimeters"]`; explicit `[[test]] name = "integrated_provenance_tdd" path = "tests/integrated_provenance_tdd.rs" required-features = ["integrated-classic-perimeters"]` (precedent for explicit `[[test]]` beside auto-discovered tests: `crates/slicer-scheduler/Cargo.toml`).
  5. New `crates/pnp-cli/tests/integrated_provenance_tdd.rs`: six `assert_cmd` tests (AC-2…AC-5, AC-N1, AC-N2), all with `.env_remove("SLICER_MODULE_PATH")`; JSON assertions via `serde_json` (existing dev-dep); workspace paths from `env!("CARGO_MANIFEST_DIR")` + `../../` (pattern: `e2e_integration_tdd.rs::stl_fixture_path`).
- Exact functions/tests touched: `Cmd::Slice` match arm, `load_dag_modules`, `dag_producers` callers, `ModuleCmd::ConfigSchema` arm, `run_diagnose`, `DiagnoseOut`/`DiagnosticOut`, `SliceRunOptions` + its literal sites.
- Rejected alternatives:
  - Re-exporting `integrated_registrations` through `slicer-runtime` instead of a direct pnp-cli dependency — rejected: the direct dep is what lets 205 select editions via pnp-cli features, and it mirrors how `run.rs` sources the registry after 201.
  - A `pub` seam function in `run.rs` unit-tested in `tests/unit/` — rejected as the primary evidence: with default (empty) registry both branches return empty, so the test is vacuous; the feature-build E2E (AC-2) exercises the real seam non-vacuously because feature unification makes `integrated_registrations()` non-empty inside the pnp-cli test build.
  - Enabling registry features via `slicer-runtime` dev-dependencies — rejected: feature unification would flip the integrated tier on for every existing `slicer-runtime` test (and, once 204 lands, silently switch their dispatch to native), an unacceptable suite-wide behavior change.
  - Adding provenance fields to the `dag_cli` output structs — deferred; loading parity is what agents need first, and the JSON schema churn belongs with a consumer.

## Files in Scope (read + edit)

Three primary files plus the mandated blast radius and test/manifest additions:

- `crates/pnp-cli/src/main.rs` - role: verb tree + loader helpers; expected change: 7 clap args, helper, 3 loader-call updates.
- `crates/slicer-runtime/src/run.rs` - role: options struct + slice seam; expected change: 1 field + loader-input computation.
- `crates/slicer-runtime/src/diagnose.rs` - role: diagnose output; expected change: signature, loader, `modules` array.
- `crates/pnp-cli/Cargo.toml` - role: dep/feature/test target; justification: unavoidable manifest edit for the feature-gated test.
- `crates/pnp-cli/tests/integrated_provenance_tdd.rs` (new) - role: AC evidence; justification: packet-authored fixture per the AC command rule.
- `docs/17_agent_debugging.md` - role: doc impact; expected change: two section edits.
- Blast radius (Step 1 only; re-derive list at implementation): measured 2026-08-10 — 14 `SliceRunOptions` construction sites across 12 files, of which 11 already end in `..Default::default()` or an FRU base and need **no** edit (`tests/visual_debug_agent_overhead_tdd.rs`, `tests/unit/slice_run_options_default_tdd.rs`, `tests/e2e/mm_real_fixture_gcode_tdd.rs`, `tests/e2e/run_slice_api_tdd.rs`, `tests/executor/cube_4color_sparse_infill_per_painted_region_tdd.rs`, `tests/executor/cube_4color_phase5_tdd.rs`, `tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs`, `tests/executor/cube_4color_gcode_output_tdd.rs` ×3, `tests/executor/cube_4color_arachne.rs`). The three full literals that need the new field: `crates/pnp-cli/src/main.rs:514`, `tests/unit/profile_flag_tdd.rs:41`, `tests/unit/cancel_flag_tdd.rs:32` — plus the `impl Default for SliceRunOptions` body in `crates/slicer-runtime/src/run.rs` (a `Self { … }` full literal; the `rg 'SliceRunOptions \{'` re-derive grep does **not** match `Self {`, so this site must be added by hand).

## Read-Only Context

- `crates/slicer-runtime/src/run.rs` - `SliceRunOptions` block and the loader call region only (locate by symbol; file is 1175 lines) - purpose: exact field ordering and call-site shape.
- `crates/pnp-cli/src/main.rs` - verb enums + edited arms only (762 lines) - purpose: clap patterns.
- `crates/slicer-scheduler/src/module_search_path.rs` - `assemble_search_roots` doc comment - purpose: confirm which tiers `--no-default-module-paths` drops.
- `crates/slicer-scheduler/src/dag_cli.rs` - `StagesOut`/`StageSummary` only - purpose: AC-4 asserted field.
- `docs/spec_packets/201-*/packet.spec.md`, `docs/spec_packets/202-*/packet.spec.md` - FORWARD-DEP contracts.

## Out-of-Bounds Files

- `docs/spec_packets/200-*`, `docs/spec_packets/201-*` (beyond packet.spec.md), `docs/spec_packets/202-*` (beyond packet.spec.md), `docs/spec_packets/194-*`…`199-*` — never modify; inspect other packet files only via SUMMARY dispatch.
- `crates/slicer-scheduler/src/manifest.rs`, `crates/slicer-wasm-host/**`, `crates/slicer-integrated-modules/**` — 201/202 own these; delegate symbol lookups.
- `modules/core-modules/**` — never edit (204's neighborhood; also guest-WASM inputs).
- `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, generated code — never load.

## Expected Sub-Agent Dispatches

- Question: what exact signature did `load_live_modules_for_plan_with_integrated` land with (parameter names/order incl. 202's `native_entries`), and did `prepare_prepass_context`'s loader stay external-only or switch (201's [FWD])?; scope: `crates/slicer-wasm-host/src/execution_plan_live.rs`, `crates/slicer-runtime/src/run.rs`; return: `FACT` (≤5 lines); purpose: Step 1.
- Question: current `SliceRunOptions { .. }` literal sites; scope: `crates/`; return: `LOCATIONS` (≤20 entries); purpose: Step 1 blast radius re-derivation.
- Question: does `integrated_registrations()` return `Vec<IntegratedModuleRegistration>` or `&'static [..]`, and what re-exports exist?; scope: `crates/slicer-integrated-modules/src/lib.rs`; return: `FACT`; purpose: Step 1/3 helper signature.
- Question: run `cargo xtask build-guests --check`; scope: repo root; return: `FACT` clean/STALE list; purpose: Step 4 precondition.

## Data and Contract Notes

- IR/manifest contracts: untouched. The diagnose JSON gains an additive `modules` array; `pass`/`modules_loaded`/`stages`/`diagnostics` keep exact current semantics (docs/17 §Diagnose exit codes unchanged).
- WIT boundary: untouched — no WIT, macro, SDK, or guest change.
- Determinism/scheduler constraints: first-root-wins dedup and tier ordering are 201's locked behavior; this packet only chooses which registrations are offered. The shadow-warning A/B in AC-2 is deterministic because the warning is emitted during module loading, before any dispatch.
- Config keys: none added; the flag is CLI-only and never becomes a config key (no snake_case surface).

## Locked Assumptions and Invariants

- `--no-integrated-modules` semantics are locked to "tier 5 contributes nothing" — it must never also drop default search paths or env-tier roots.
- Diagnose provenance strings are locked to lowercase `"integrated"` / `"external"` (203's display contract; 205's edition verification may grep them).
- The shadow-diagnostic message text is 201's contract; this packet asserts it verbatim (AC-N2) and must not restate or alter it in code.

## Risks and Tradeoffs

- AC-2 runs two real slices of `20mmbox-LF.stl` with all core modules in a debug test build — the heaviest test in the packet (cost precedent: `slice_cancel_tdd.rs` / `m73_progress_tdd.rs` already slice in pnp-cli tests). Accepted: it is the only non-vacuous end-to-end proof that the slice verb's flag reaches the loader.
- The `integrated-classic-perimeters` feature couples this packet's tests to 201's registry feature name. That name is now pinned on both sides: 201's `[FWD]` is closed to the bare `classic-perimeters`, because 205 composes edition features as `integrated-<name> = ["slicer-integrated-modules/<name>"]` and a prefixed registry feature would break its AC-7. So this packet's `pnp-cli` passthrough is `integrated-classic-perimeters = ["slicer-integrated-modules/classic-perimeters"]`. `implementation-plan.md` Step 3 still requires *verifying* the landed name before writing it — but any mismatch is now a 201 defect to report, not a 203 adaptation to absorb.
- Until 204, nothing exercises the `native_entries()` half of the disable seam with non-empty input; AC-2 proves the registrations half. 204's parity suite plus 205's edition checks close the residual.
- The blast-radius list is a ledger fact; if the parallel 194–199 plan lands FRU defaults first, some listed edits become no-ops. Re-derive; never assume.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1 — field + struct-literal sweep of 13 sites across 11 files; re-derive both numbers at implementation)
- Highest-risk dispatch and required return format: the 201/202 landed-signature check — `FACT`, ≤5 lines; a wrong guess here mis-shapes the seam edit.

## Open Questions

- [FWD] Re-derive at implementation: exact landed parameter order of `load_live_modules_for_plan_with_integrated(..)` after 202 appends `native_entries`, and whether `integrated_registrations()` landed as `Vec` or `&'static [..]` (201 [FWD]) — adjust the helper's return type accordingly.
- [FWD] Did 201 leave `prepare_prepass_context` integrated-aware or external-only (201's own [FWD] + FACT-dispatch outcome)? Either answer keeps SupportPreview out of scope here; record which one held in the Step 1 commit message.
- [FWD] If `ModuleProvenance` gained `serde::Serialize`/`Display` during 201 implementation, `diagnose.rs` may use it instead of the local match — prefer the landed form; the JSON strings stay `"integrated"`/`"external"` either way.
- [FWD] If 204 lands before this packet is implemented, `native_entries()` will be non-empty under the pnp-cli test feature; AC-2's A/B remains valid (the external copy shadows in both runs), but confirm no new pilot feature is accidentally enabled by the passthrough.
