# Requirements: 165_cli-binary-locator-extraction

## Packet Metadata

- Grouped task IDs: `TASK-146d`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

Packet 162 closed the stale-`pnp_cli` false-baseline trap at all three spawn sites (`slicer_cache.rs`, `gate_evidence.rs`, `dag_cli_integration.rs`) but — by explicit decision recorded in its `[FWD]` — fixed them **in place**, leaving the locator + freshness assert triplicated. Extraction was deferred because the shared home is an architecture decision requiring an ADR: ADR-0004 places only *guest-side* test support in `slicer-sdk` (a crate compiled into guest WASM — the wrong home for host process-spawning plumbing), and `slicer-test`, the crate that could have hosted it, was deleted by packet 78 (commit `c68f8973`). The residual risk 162 accepted is drift among the three copies — `gate_evidence.rs` produces DEV-026's 50-layer time evidence, so a drifted copy there silently invalidates governance evidence. The ADR-0045 plan queues this as row #4 precisely because "the kind of follow-up that historically evaporates" needed its own TASK id and row.

## In Scope

- A new ADR (`docs/adr/<NNNN>-host-side-test-support-crate.md`, number re-derived at write time) deciding the host-side test-support home: a new std-only crate.
- New workspace member `crates/slicer-test-support` (lib name `slicer_test_support`): `workspace_root()`, `pnp_cli_bin()`, `staleness_reason(...)`, `newest_source_mtime(...)`, moved verbatim-in-behavior from the post-162 `slicer_cache.rs`. Zero `[dependencies]`; `[lints] workspace = true`.
- Root `Cargo.toml`: add the member.
- `crates/slicer-runtime/Cargo.toml`: add `[dev-dependencies] slicer-test-support` (serves both the `tests/` tree and the `gate_evidence` bench — bench targets receive dev-dependencies).
- `crates/slicer-scheduler/Cargo.toml`: add the same dev-dependency.
- `crates/slicer-runtime/tests/common/slicer_cache.rs`: delete local `pnp_cli_bin`/`staleness_reason`/`newest_source_mtime` bodies; `pub use slicer_test_support::{pnp_cli_bin, staleness_reason, newest_source_mtime};` so `run_pnp_cli_uncached`, the e2e callers, and `pnp_cli_freshness_tdd` are untouched. `repo_root()` may delegate to `slicer_test_support::workspace_root()` or stay — implementer's choice; it is not part of the triplication.
- `crates/slicer-runtime/benches/gate_evidence.rs`: delete its self-contained `pnp_cli_bin` mirror (and its "Mirrors (does not import…)" justification comment, which becomes false); import from `slicer_test_support`. Its local `repo_root()` may likewise delegate to `workspace_root()`.
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`: delete `fn bin()`; call `slicer_test_support::pnp_cli_bin()`; `workspace_root()`/`core_modules_path()` may delegate or stay.
- `docs/07_implementation_status.md`: TASK-146d row.

## Out of Scope

- Any change to the freshness algorithm, panic messages, or scan scope established by 162. This packet moves code; it does not redesign it.
- Giving `xtask` a lib target or importing `build_guests::is_stale` — rejected by the plan's grounding correction 6; the shared crate remains a documented *mirror* of `is_stale`.
- `crates/pnp-cli/tests/e2e_integration_tdd.rs` — it correctly uses `env!("CARGO_BIN_EXE_pnp_cli")` (available only in the binary-defining package) and is not one of the three copies. Do not migrate it.
- The `xtask test` Step-1 `pnp_cli` rebuild gate (162's AC-9 surface) — unaffected.
- Packets 163/164's WIT/package surfaces; guest WASM; any production crate's `[dependencies]`.
- Moving `pnp_cli_freshness_tdd.rs` out of the slicer-runtime `integration` bucket — it stays as the regression home 162 registered.

## Authoritative Docs

- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` - long; ranged reads only (§"Grounding corrections" 1/4/6, §"Exports ledger" From #1, §"Packet Queue" row 4).
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - 72 lines; direct read.
- `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - ~196 lines; direct read of §"CLI freshness — three sites, fixed in place" and §"Open Questions" `[FWD]` only.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md`. Refinement: AC-3's re-export requirement exists so that the ~30 `slicer-runtime` e2e/integration files calling `common::slicer_cache::*` need zero edits — if any of them changes in the diff, the extraction went wrong.
- Negative: `AC-N1`.
- Cross-packet impact: none forward — 163/164 do not touch the three sites. Backward: 162's AC-8 grep contract (each site names `staleness_reason`, no fallback loop, dag panic names `cargo build -p pnp-cli`) must remain true post-extraction; AC-3/AC-N1 encode that.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | all targets (incl. bench + scheduler tests) still compile | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate incl. new crate | FACT pass/fail |
| AC-1 command (`cargo check -p slicer-test-support` + python fn/dep audit) | new crate shape | FACT PASS/FAIL line |
| AC-2 command (rg single-definition counts) | triplication actually gone | FACT PASS/FAIL line |
| AC-3 command (site-consumption python audit) | three sites consume the crate | FACT PASS/FAIL line |
| `(cargo test -p slicer-scheduler --test scheduler_integration -- dag_ 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed'` | site 3 exercised end-to-end (name filter matching the `dag_*` test fns in `dag_cli_integration.rs`, 7 today; 0 passed = FAIL) | FACT pass/fail + result line |
| `(cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed'` | staleness_reason contract survives the move (name filter; 0 passed = FAIL) | FACT pass/fail + result line |
| `cargo bench -p slicer-runtime --bench gate_evidence --no-run` | site 2 compiles against dev-deps (compile-only; never run the bench in this packet) | FACT pass/fail |
| `(cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed'` | baseline stays green — expect `12 passed; 0 failed; 11 ignored` (name filter; 0 passed = FAIL) | FACT pass/fail + result line |
| `(cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 \| tee target/test-output.log \| rg '^test result') \| rg -v '0 passed'` | baseline e2e that spawns via `slicer_cache` — expect `1 passed; 0 failed` (site 1 exercised) | FACT pass/fail + result line |
| AC-7 command (ADR existence + content audit) | the home decision is recorded | FACT PASS/FAIL line |
| AC-N1 command (fallback-resurrection audit) | 162's trap stays closed | FACT PASS/FAIL line |

No `cargo test --workspace` anywhere in this packet: the change surface is test-only plumbing in two crates plus one new dependency-free crate; the targeted runs above exercise every consumer, and `--all-targets` check/clippy proves compilation of everything else.

## Step Completion Expectations

- The ADR (Step 1) must be written **before** the crate (Step 2): the crate's rustdoc and the packet's diff cite the ADR by its derived number, and deriving the number after creating files invites the frozen-ledger-fact failure this queue has hit repeatedly.
- Between Step 3 and Step 4 the workspace must compile at every commit point — there is no intentionally-broken window in this packet; a step that leaves `cargo check --workspace --all-targets` red is incomplete.

## Context Discipline Notes

- `crates/slicer-runtime/tests/common/slicer_cache.rs` is ~348 lines pre-162 and will have grown; read only the locator block (locate `pnp_cli_bin`, `staleness_reason`, `newest_source_mtime` by name) plus the `use`/module header. The cache machinery (`cached_run`, `execute_slicer`, staging dirs) is out of scope — do not read it.
- Do not open `docs/07_implementation_status.md` directly; the TASK-146d row is added via a worker dispatch.
- Line numbers in this packet's lineage (162's design cites e.g. `:15-31`) are navigation hints only and were captured pre-162; every citation resolves by symbol name.
