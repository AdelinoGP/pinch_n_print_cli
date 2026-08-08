# Requirements: 203-integrated-cli-provenance

## Packet Metadata

- Grouped task IDs: `ADR-0056`, `ADR-0057` (no `docs/07_implementation_status.md` TASK rows exist for this program — see `docs/specs/multi-edition-distribution-plan.md` §"Backlog anchoring [FWD]"; do not create them in this packet)
- Backlog source: `docs/specs/multi-edition-distribution-plan.md` (queue row 4)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After packets 201/202 land, `run_slice` assembles integrated modules into tier 5 and dispatches them natively, but no user or agent can (a) turn the integrated tier off (ADR-0057 requires `--no-integrated-modules` so module developers can test pure-external setups on Hybrid/Integrated binaries), or (b) see which loaded module is integrated vs external, or that an external copy is shadowing an integrated one. The `pnp_cli module` and `dag` verbs still load through the external-only `load_modules_from_roots`, so the CLI's introspection surface disagrees with what a slice actually runs. This packet closes both gaps in one coherent CLI/provenance slice; it deliberately ships before the pilot modules (204), so with default features every new behavior is inert (empty registry) and observable only under the test-only `integrated-classic-perimeters` feature.

## In Scope

- New `--no-integrated-modules` clap flag on `Cmd::Slice`, `ModuleCmd::Diagnose`, `ModuleCmd::ConfigSchema`, and all four `DagCmd` variants (`Stages`, `Stage`, `Depends`, `Claims`) in `crates/pnp-cli/src/main.rs`; composes with `--no-default-module-paths`, which keeps its current meaning (drops config-dir and exe-dir tiers only — ADR-0057).
- New `SliceRunOptions.no_integrated_modules: bool` field (`crates/slicer-runtime/src/run.rs`) plus the run-path seam: when set, `run_slice` passes `&[]` registrations and no native entries to `load_live_modules_for_plan_with_integrated` (201's documented disable seam, extended by 202's `native_entries` parameter); when unset, it sources `slicer_integrated_modules::integrated_registrations()` / `native_entries()` as 201/202 wired.
- Struct-literal blast radius of the new field: every `SliceRunOptions { .. }` construction site (13 sites across 11 files at authoring, none using `..Default::default()`; re-derive both numbers at implementation — `design.md` §Architecture Constraints).
- Integrated-aware loading for CLI introspection: `load_dag_modules` and the `ModuleCmd::ConfigSchema` arm switch from `load_modules_from_roots` to `load_modules_from_roots_with_integrated` (FORWARD-DEP, `crates/slicer-scheduler/src/manifest.rs`), sourcing registrations from `slicer_integrated_modules::integrated_registrations()` unless the flag disables them; shared helper in `main.rs`.
- `run_diagnose` (`crates/slicer-runtime/src/diagnose.rs`): third parameter `no_integrated_modules: bool`; loads via the integrated-aware entry point; output gains a `modules` array with one `{id, provenance}` entry per surviving module (`provenance` serialized as `"integrated"` / `"external"` via a local mapping over `ModuleProvenance` — FORWARD-DEP); the shadow diagnostic flows through the existing `diagnostics` array unchanged.
- `crates/pnp-cli/Cargo.toml`: direct dependency on `slicer-integrated-modules` (FORWARD-DEP crate), a passthrough feature `integrated-classic-perimeters = ["slicer-integrated-modules/classic-perimeters"]`, and a `[[test]]` target `integrated_provenance_tdd` with `required-features = ["integrated-classic-perimeters"]`.
- New test file `crates/pnp-cli/tests/integrated_provenance_tdd.rs` driving the real binary via `assert_cmd` (fixture authored in this packet; covers AC-2 … AC-5, AC-N1, AC-N2).
- Doc edits to `docs/17_agent_debugging.md` §Diagnose and §DAG introspection (see `packet.spec.md` §Doc Impact).

## Out of Scope

- Pilot-module integration, `native_entries()` population, and parity gates (packet 204). Until 204, `native_entries()` arms are empty (202's contract), which is why AC-2's slice runs never dispatch an integrated module natively — the external copy shadows it.
- Editions, `cargo xtask dist`, and CI artifacts (packet 205).
- `Cmd::SupportPreview` and `prepare_prepass_context` (`crates/slicer-runtime/src/run.rs`): no `--no-integrated-modules` flag there. Rationale: `prepare_prepass_context` is shared by support-preview and visual-debug with multiple callers, and threading the flag through them is outside this packet's CLI surface.

  **KNOWN GAP (tracked, not argued away).** 201's `design.md` item 6 plans that *both* live-loader call sites switch to the integrated-aware entry point passing `slicer_integrated_modules::integrated_registrations()`. If 201 lands as planned, `support-preview` and `visual-debug` load the integrated tier with **no way to disable it**, which contradicts ADR-0057's normative clause that the flag "disables the integrated tier entirely" and this packet's own locked invariant ("tier 5 contributes nothing"). The tier is inert under *default* features today, but not in the Hybrid or Integrated editions ADR-0057 defines — which is precisely the flag's stated use case. **This gap is deliberately left open here and must be closed before any non-Developer edition ships.** No packet in the 200–205 queue currently owns it: 205 builds the first Hybrid/Integrated artifact but puts `--no-integrated-modules` out of scope, so the gap is *tracked but unassigned*. Assign an owner at the 200-series close — either by widening 205 or by adding a follow-on packet. Raised by the packet-203 preflight gate, 2026-08-07.
- Provenance fields inside `dag` JSON output shapes (`StagesOut`/`StageOut`/`DependsOut`/`ClaimsOut` in `crates/slicer-scheduler/src/dag_cli.rs`): the dag verbs gain integrated-aware *loading* and the flag, but their output schemas are unchanged.
- Any edit to `modules/core-modules/**`, `crates/slicer-scheduler/**`, `crates/slicer-wasm-host/**`, or `crates/slicer-integrated-modules/**` (201/202 own those surfaces).
- Any change to the shadow-diagnostic string or `LoadModulesReport` shape (201's contract; this packet only displays them).

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — 122 lines; direct read.
- `docs/adr/0057-three-editions-and-integrated-tier.md` — 55 lines; direct read.
- `docs/17_agent_debugging.md` — 287 lines; read only §Diagnose and §DAG introspection; delegate the rest.
- `docs/spec_packets/201-integrated-module-registry-tier5/packet.spec.md`, `docs/spec_packets/202-native-adapter-and-dispatch/packet.spec.md` — direct read; other packet files via SUMMARY dispatch only.
- `docs/specs/multi-edition-distribution-plan.md` — short; Exports ledger is the FORWARD-DEP source of truth.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: 205 consumes the flag when verifying edition artifacts (an Integrated-edition binary with `--no-integrated-modules` must degrade to external-only loading); 204's parity work is independent of this packet.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo run -p pnp-cli --bin pnp_cli -- slice --help \| rg -q -- '--no-integrated-modules' && rg -q 'no_integrated_modules' crates/slicer-runtime/src/run.rs && rg -q 'no_integrated_modules' crates/pnp-cli/src/main.rs` | AC-1 flag + plumbing | FACT pass/fail |
| `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd slice_flag_disables_integrated_tier 2>&1 \| tee target/test-output.log` | AC-2 slice A/B (shadow warning present/absent) | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd diagnose_lists_integrated_provenance 2>&1 \| tee target/test-output.log` | AC-3 provenance listing | FACT pass/fail |
| `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd dag_stages_sees_integrated_tier 2>&1 \| tee target/test-output.log` | AC-4 dag integrated-aware + disable | FACT pass/fail |
| `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd config_schema_includes_integrated_module 2>&1 \| tee target/test-output.log` | AC-5 config-schema integrated-aware + disable | FACT pass/fail |
| `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd no_integrated_modules_empties_diagnose 2>&1 \| tee target/test-output.log` | AC-N1 flag removes the tier | FACT pass/fail |
| `cargo test -p pnp-cli --features integrated-classic-perimeters --test integrated_provenance_tdd diagnose_shows_external_shadowing_integrated 2>&1 \| tee target/test-output.log` | AC-N2 shadow display correctness | FACT pass/fail |
| `rg -q -- '--no-integrated-modules' docs/17_agent_debugging.md && rg -qi 'provenance' docs/17_agent_debugging.md` | AC-N3 doc greps | FACT pass/fail |
| `cargo xtask build-guests --check` | test precondition — AC-2/AC-N2 scan/dispatch real guest artifacts | FACT clean/STALE |
| `cargo check --workspace --all-targets` | compile gate incl. blast-radius sweep | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- The `SliceRunOptions` field lands with its full struct-literal blast radius in the same step (Step 1); later steps assume `cargo check --workspace --all-targets` is already green.
- The feature-gated test file (Step 4) requires Steps 1–3 complete and fresh guests (`cargo xtask build-guests --check` clean) before its first run.
- `run_diagnose`'s signature change (Step 2) and its `main.rs` caller update must land in the same step — the only caller is the `ModuleCmd::Diagnose` arm.

## Context Discipline Notes

- `crates/slicer-runtime/src/run.rs` is 1175 lines: read only the `SliceRunOptions` struct block and the loader call region (locate by `rg -n 'load_live_modules' crates/slicer-runtime/src/run.rs`); never the whole file.
- `crates/pnp-cli/src/main.rs` is 762 lines: read the verb enums and the arms being edited; skip the mesh/visual-debug arms.
- Re-derive the `SliceRunOptions { .. }` literal-site list at implementation time (`rg -n 'SliceRunOptions \{' crates/`) — it is a ledger fact; the parallel struct-literal plan (`docs/specs/struct-literal-churn-gate-plan.md`, packets 194–199) may have converted test sites to FRU by then, shrinking the sweep.
