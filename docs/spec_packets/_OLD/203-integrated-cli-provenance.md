---
status: implemented
packet: 203-integrated-cli-provenance
task_ids:
  - ADR-0056
  - ADR-0057
---

# 203-integrated-cli-provenance

## Goal

Add the `--no-integrated-modules` flag (slice verb plus every manifest-loading CLI verb) that disables the integrated tier entirely via packet 201's disable seam (pass `&[]` registrations and no native entries), and surface module provenance — integrated vs external, plus the shadow diagnostic — in `pnp_cli module diagnose`, `module config-schema`, and the `dag` verbs, per ADR-0056 consequences and ADR-0057.

## Problem Statement

After packets 201/202 land, `run_slice` assembles integrated modules into tier 5 and dispatches them natively, but no user or agent can (a) turn the integrated tier off (ADR-0057 requires `--no-integrated-modules` so module developers can test pure-external setups on Hybrid/Integrated binaries), or (b) see which loaded module is integrated vs external, or that an external copy is shadowing an integrated one. The `pnp_cli module` and `dag` verbs still load through the external-only `load_modules_from_roots`, so the CLI's introspection surface disagrees with what a slice actually runs. This packet closes both gaps in one coherent CLI/provenance slice; it deliberately ships before the pilot modules (204), so with default features every new behavior is inert (empty registry) and observable only under the test-only `integrated-classic-perimeters` feature.

## Architecture Constraints

- **Flag composition:** ADR-0057 §Decision states exactly two things here — `--no-integrated-modules` "disables the integrated tier entirely", and `--no-default-module-paths` "keeps its current meaning (drops the config-dir and exe-dir tiers only); the flags compose". Everything below is a **derived consequence**, not ADR text, and its basis is named so a reviewer can re-check it:
  - *Derived from ADR-0057 + `assemble_search_roots` (`crates/slicer-scheduler/src/module_search_path.rs`):* `--no-default-module-paths` drops the config-dir and exe-dir tiers and does NOT touch tier 5; `--no-integrated-modules` drops only tier 5 (by passing empty registrations — 201's documented disable seam — and, on the slice path, no native entries). Since each flag names a disjoint tier set, **neither implies the other**.
  - *Derived from the tier list in `assemble_search_roots`:* the `SLICER_MODULE_PATH` env tier is neither a default-path tier nor tier 5, so it is untouched by both flags. Consequence for this packet: every new test must clear that variable (`assert_cmd` `.env_remove("SLICER_MODULE_PATH")`), or a developer's exported path silently injects external modules. Re-derive the tier list from `assemble_search_roots` at implementation time rather than trusting this bullet.
- **Disable = empty inputs, not a new code path (packet 201's contract):** the loader entry points never learn about the flag; disabling is achieved purely by what the callers pass (`&[]`). Do not add a `bool` parameter to any 201/202 loader signature. Source: `201/design.md` §Code Change Surface item 5 — "add an entry point, never a parameter, to keep existing call sites untouched" — and `201/packet.spec.md` AC-N2, which makes `load_modules_from_roots_with_integrated(roots, &[])` a strict identity with `load_modules_from_roots(roots)`. (This rule is **not** in ADR-0056; Decision item 1 "One model" is about manifest ingestion via `include_str!`. No ADR is contradicted, so no `docs/DEVIATION_LOG.md` row is owed.)
- **Feature-gated test green-blindness (CLAUDE.md §Feature-gated test files):** `integrated_provenance_tdd` carries `required-features = ["integrated-classic-perimeters"]`, so a bare `cargo test -p pnp-cli` skips it silently and prints a clean green wall. Every AC command therefore spells the `--features` flag; the acceptance ceremony must use those exact commands, never the bare form.
- **Guest-artifact precondition (CLAUDE.md §Guest WASM Staleness):** this packet edits no path that feeds guest WASM, so no rebuild is triggered by its changes — but AC-2 slices with `--module-dir modules/core-modules` and AC-N2's diagnose requires every manifest's companion `.wasm` to exist on disk (`load_modules_from_roots` hard-errors on a missing companion — see docs/17 §Diagnose exit code 2). Run `cargo xtask build-guests --check` (rebuild if `STALE:`) before attributing any AC-2/AC-N2 failure to this packet's edits.
- **Schema/version constants:** none touched. The diagnose JSON is a CLI output contract documented only in `docs/17_agent_debugging.md`, not a versioned IR schema; adding the `modules` array is additive and consumers per docs/17 parse named fields.

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
