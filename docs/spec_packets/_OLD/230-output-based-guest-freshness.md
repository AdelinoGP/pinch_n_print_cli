---
status: implemented
packet: 230-output-based-guest-freshness
task_ids:
  - TASK-341
---

# 230-output-based-guest-freshness

## Goal

Wire packet 229's artifact verifier into `cargo xtask build-guests --check` so guest WIT staleness is answered by decoding each artifact — with the stage resolved from the artifact and cross-checked against the core guest's manifest `[stage] id` — while `check_command` returns the stale list, `test_command` rebuilds only stale guests, `wasm-tools`-missing is a distinct infrastructure exit code, and the `v2-` fingerprint is written only after final verification succeeds.

## Problem Statement

`cargo xtask build-guests --check` answers "did any tracked input change?" instead of "does this artifact still agree with canonical?". `is_stale` unions a per-guest fingerprint with `compute_shared_freshness`, whose `shared_input_paths` charges every guest with all of `crates/slicer-schema/wit/**` plus the `slicer-{macros,sdk,ir,schema,core}` sources. One byte changed in any of them marks all 42 guests (21 core + 21 test) `STALE:`, `cargo xtask test` rebuilds all 42, and the first guest that fails to compile aborts the whole suite — so a WIT change made by one agent surfaces as another agent's tests breaking.

The semantic answer already exists but is unreachable from the gate: `verify_embedded_world` is called only from `build_one`, i.e. only *after* a build. All 42 artifacts are present on disk and `wasm-tools 1.250.0` resolves on `PATH`.

Four defects block simply calling the verifier from `check_command`, and this packet is the slice that fixes exactly those four:

1. **Stage resolution.** `build_one` resolves the WIT dir through `module_stage_wit_dir`, which needs a sibling module manifest — test guests have none, so their comparison runs against the ambiguity-stripped shared set. Resolution must come from the artifact. But resolution *solely* from the artifact makes the check self-referential (R5-4): a guest exporting the wrong stage declares its own stage and is judged against it, comparing equal. The manifest `[stage] id` therefore stays as an independent expectation for core guests.
2. **API shape.** `check_command` returns a bare `i32`, so `test_command` learns only "something is stale" and must rebuild everything.
3. **Reporting.** Freshness is asserted downstream by grepping for `STALE:`. A `wasm-tools`-missing failure prints no `STALE:` line and therefore reads as PASS (R5-3). The contract must be exit-code based, with a distinct code for infrastructure failure.
4. **Fingerprint lifecycle and content.** The sidecar is written at the end of `build_one_inner` — *before* `build_one`'s verification runs — so a guest that fails verification still leaves a fingerprint claiming freshness. Its content also omits the workspace-root `Cargo.toml` (which pins `wit-bindgen`, consumed as `wit-bindgen.workspace = true`), the guest's own `Cargo.lock`, and the rustc version, any of which can change the emitted bindings with byte-identical WIT (R5-2).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- This packet changes the staleness gate itself, so the usual "trust `--check`" reflex is suspended for its own steps: an unexpected `STALE:` report may be a bug in this packet's logic rather than a genuine stale guest. Confirm by decoding the named artifact before rebuilding.
- **The fingerprint version prefix moves `v1-` → `v2-`.** That is a public-ish version constant for the sidecar format: every guest invalidates exactly once, forcing one full rebuild. The step that lands the prefix owns that rebuild and owns updating any test that asserts on the `v1-` literal — do not defer it to a later `cargo check`.
- **The check must not be self-referential (R5-4).** An artifact declaring its own stage and then being judged only against that declaration compares equal by construction. The core guest's manifest `[stage] id` is the independent expectation that breaks the circle. `module_stage_wit_dir`'s own doc comment records the packet-164 regression that arose the last time manifest-derived resolution was lost.
- **Freshness is asserted by exit code, never by grepping for `STALE:` (R5-3).** Therefore `StaleReason`'s and `Drift`'s `Display` output must never contain the substring `STALE:`, and the infrastructure code must be distinct from both `0` and `1`.
- **No fail-open remains.** Every "cannot tell" outcome is either staleness (artifact-side: undecodable, unresolvable, missing) or an infrastructure error (tooling-side: `wasm-tools` absent, canonical unusable). Nothing maps to fresh.
- `GuestSpec` is a `pub` struct with 7 named fields, so the struct-literal churn gate applies to every new test literal: use `..` rest or an `// exhaustive: <reason>` waiver, per `docs/21_data_defaults_and_fixtures.md`; `cargo xtask check-literals` enforces it.
- `xtask` has no `[lib]` and no `xtask/tests/` directory; all tests are inline `#[cfg(test)] mod tests`, so AC commands are `cargo test -p xtask <module>::tests::<name> -- --exact`.

## Data and Contract Notes

- IR/manifest contracts: module manifests are read only for `[stage] id`, through the surviving `parse_stage_id_from_module_manifest`. No manifest key is added, renamed or removed. Manifest section headers and runtime key strings remain snake_case.
- WIT boundary: no canonical `.wit` file is edited. The stage↔package mapping is read from `slicer_schema::STAGES`, which stays the single source of truth (ADR-0006's stage table, ADR-0045's per-stage versioned packages). Resolution *adds* a consumer of that table; it does not create a parallel one.
- Determinism/scheduler constraints: `check_command` iterates `discover_guests`' order and prints one marker line per stale guest, so output is stable and diffable run to run. `CheckContext` is built once so all 42 comparisons see identical canonical input.
- Sidecar format: `target/guest-fingerprints/{crate_name}.fingerprint`, content `v2-{:016x}{:016x}`. The `v1-` → `v2-` change is deliberately not backward compatible; a `v1-` sidecar reads as a mismatch and triggers exactly one rebuild per guest.

## Locked Assumptions and Invariants

- For a core guest, `GuestSpec.stage_id` is authoritative as the *expectation*; the artifact's resolved stage must equal it. This asymmetry is the whole anti-self-reference argument and must not be "simplified" later.
- Exit codes are locked: `0` fresh, `1` stale, `EXIT_INFRA_ERROR` infrastructure. Downstream automation (packet 232's snippet rewrite, CI) depends on those three being distinct.
- No `Display` impl on `StaleReason`, `Drift` or `StageResolutionError` may contain the substring `STALE:`.
- The fingerprint is written only after final verification, and its absence is always safe (it means "rebuild"), while its presence is a positive claim that verification passed.
- `build_command` keeps its current signature and full-rebuild behaviour for `xtask/src/dist.rs` and CI.

## Risks and Tradeoffs

- **Decode cost per `--check` is unmeasured on this machine.** AC-16 forces a measurement rather than an assumption. If the measured "after" figure is materially worse than "before", report it — do not bury it, and do not quote the plan's earlier unmeasured `~38ms`/`~2s` figures.
- **First run after the `v2-` prefix lands marks all 42 guests stale.** Expected and one-time; must not be misread as a regression in the new comparison.
- **A pre-existing wrong-stage guest would now surface as `StageMismatch`.** That is the intended catch (R5-4), but it may appear as a surprising failure on an artifact that has been "working". Investigate the artifact; do not relax the cross-check.
- **`check_command` now depends on `wasm-tools` for a *check*, not just a build.** `.github/workflows/ci.yml` installs it via `taiki-e/install-action` in both the `test` and `dist-editions` jobs; the `test` job is the one that runs `cargo xtask build-guests --check`, and `dist-editions` runs `cargo xtask dist` (which reaches `build_command`). Both therefore have the tool available, so CI is safe; a developer without it gets `EXIT_INFRA_ERROR` with an actionable message rather than a silent pass.
- **`test_command`'s new seam adds an indirection** to a hot path in developer workflow. Accepted: without it, AC-9, AC-10 and AC-N4 can only be verified by spawning real builds, which is not delegation-friendly.
