# Requirements: 165_cli-binary-locator-extraction

## Packet Metadata

- Grouped task IDs: `TASK-146d`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

**Premise correction (recorded at implementation time; the original authoring got the count wrong).** This packet was written on the belief that the `pnp_cli` locator existed in three copies. A verification sweep during implementation found **seven**. Packet 162 closed the stale-`pnp_cli` false-baseline trap at three spawn sites (`crates/slicer-runtime/tests/common/slicer_cache.rs`, `crates/slicer-runtime/benches/gate_evidence.rs`, `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`, whose copy is named `bin`). Four further copies were never in 162's scope and carry **no freshness gate at all** — `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`, `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`, `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`. Each of those four also branches on `std::env::var("PROFILE")`, a variable Cargo sets for build scripts rather than test binaries, so the branch is inert and always resolves `target/debug/pnp_cli`. The packet is widened (user-approved) to all seven: this makes AC-2 satisfiable inside the declared scope — the original scope forbade editing the trees the four live in, contradicting AC-2 — and extends 162's freshness gate to the four sites that never had one.

Packet 162 closed the trap at its three spawn sites but — by explicit decision recorded in its `[FWD]` — fixed them **in place**, leaving the locator + freshness assert duplicated. Extraction was deferred because the shared home is an architecture decision requiring an ADR: ADR-0004 places only *guest-side* test support in `slicer-sdk` (a crate compiled into guest WASM — the wrong home for host process-spawning plumbing), and `slicer-test`, the crate that could have hosted it, was deleted by packet 78 (commit `c68f8973`). The residual risk 162 accepted is drift among the three copies — `gate_evidence.rs` produces DEV-026's 50-layer time evidence, so a drifted copy there silently invalidates governance evidence. The ADR-0045 plan queues this as row #4 precisely because "the kind of follow-up that historically evaporates" needed its own TASK id and row.

## In Scope

- A new ADR (`docs/adr/<NNNN>-host-side-test-support-crate.md`, number re-derived at write time) deciding the host-side test-support home: a new std-only crate.
- New workspace member `crates/slicer-test-support` (lib name `slicer_test_support`): `workspace_root()`, `pnp_cli_bin()`, `staleness_reason(...)`, `newest_source_mtime(...)`, moved from the post-162 `slicer_cache.rs` with the freshness algorithm and scan scope unchanged; panic message text is reconciled across the divergent copies per §Out of Scope and `packet.spec.md` §"Declared Deviation". Zero `[dependencies]`; `[lints] workspace = true`.
- Root `Cargo.toml`: add the member.
- `crates/slicer-runtime/Cargo.toml`: add `[dev-dependencies] slicer-test-support` (serves both the `tests/` tree and the `gate_evidence` bench — bench targets receive dev-dependencies).
- `crates/slicer-scheduler/Cargo.toml`: add the same dev-dependency.
- `crates/slicer-runtime/tests/common/slicer_cache.rs`: delete local `pnp_cli_bin`/`staleness_reason`/`newest_source_mtime` bodies; `#[allow(unused_imports)] pub use slicer_test_support::{pnp_cli_bin, staleness_reason};` so `run_pnp_cli_uncached`, the e2e callers, and `pnp_cli_freshness_tdd` are untouched. Two symbols, not three: `newest_source_mtime` has zero consumers anywhere outside `slicer-test-support` (re-derive: `rg -n 'newest_source_mtime' crates/ --glob '!crates/slicer-test-support/**'` returns only the doc-comment in `slicer_cache.rs` explaining its absence), so re-exporting it would add an unused name; a future caller imports it from `slicer_test_support` directly. The `#[allow(unused_imports)]` is required, not stylistic: `slicer_cache.rs` is `#[path]`-included as a *private* module into several test binaries, and a `pub use` inside a private module does fire `unused_imports` in binaries that use only some of the re-exported names. Measured: dropping the attribute fails `cargo clippy --workspace --all-targets -- -D warnings` with `unused import: staleness_reason` in the `arachne_wall_sequence_e2e_tdd` test target. `repo_root()` may delegate to `slicer_test_support::workspace_root()` or stay — implementer's choice; it is not part of the triplication.
- `crates/slicer-runtime/benches/gate_evidence.rs`: delete its self-contained `pnp_cli_bin` mirror (and its "Mirrors (does not import…)" justification comment, which becomes false); import from `slicer_test_support`. Its local `repo_root()` may likewise delegate to `workspace_root()`.
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`: delete `fn bin()`; call `slicer_test_support::pnp_cli_bin()`; `workspace_root()`/`core_modules_path()` may delegate or stay.
- The four locator copies found by the premise correction — `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`, `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`, `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`: in each, delete the local `pnp_cli_bin` and the inert `PROFILE` branch it contains, and add `use slicer_test_support::pnp_cli_bin;` (a qualified `slicer_test_support::pnp_cli_bin()` call is equally acceptable). No manifest edit for these — the `slicer-runtime` dev-dependency already covers the whole `tests/` tree. Their `repo_root()`/`core_modules_dir()`/`core_modules_root()` helpers may delegate to `slicer_test_support::workspace_root()` or stay; they are not the duplicated locator.
- `docs/07_implementation_status.md`: TASK-146d row.

## Out of Scope

- Any change to the **freshness algorithm** or the **scan scope** established by 162. `newest_source_mtime`'s body moves byte-identically; `staleness_reason` keeps its three arms and its `newest_src_mtime > artifact_mtime` comparison direction; `pnp_cli_bin` keeps its `current_exe().parent().parent()` profile inference. This packet moves that logic; it does not redesign it.
- Any *weakening* of the panic messages. Reconciling their **text** is explicitly **in** scope and was done: the three pre-extraction copies were not identical (the `dag_cli_integration.rs` copy's stale arm contained neither the word `stale` nor a remedy), so a verbatim move was impossible and one wording had to be chosen. The reconciled text must satisfy 162's loudness contract at all seven sites — name `pnp_cli`, contain `stale`, name the resolved path, and name the remedy `cargo build -p pnp-cli` — which AC-N1 and `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` both assert. See `packet.spec.md` §"Declared Deviation — panic message text reconciled, not moved verbatim". What remains out of scope is dropping any of those four required elements, or reintroducing the too-broad `cargo build --workspace` / stale `cargo build --bin pnp_cli` remedies.
- Giving `xtask` a lib target or importing `build_guests::is_stale` — rejected by the plan's grounding correction 6; the shared crate remains a documented *mirror* of `is_stale`.
- `crates/pnp-cli/tests/e2e_integration_tdd.rs` — it correctly uses `env!("CARGO_BIN_EXE_pnp_cli")` (available only in the binary-defining package) and is not one of the seven copies. Do not migrate it.
- Every file under `crates/slicer-runtime/tests/e2e/**` and `crates/slicer-runtime/tests/integration/**` **except the four named locator-copy sites listed in §In Scope**. The exclusion is narrowed, not lifted: the `pub use` re-export in `slicer_cache.rs` exists precisely so the *other* caller files in those trees need zero edits, and if any of them appears in the diff the extraction went wrong. (Magnitude, re-derived — earlier revisions inflated this to "~30": `rg -l 'slicer_cache' crates/slicer-runtime/tests/` returns a single-digit file count, and only `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` consumes a re-exported *locator* symbol. The exclusion holds regardless of the count.) The four are in scope only because each defines its own locator copy, not because it calls one.
- The `xtask test` Step-1 `pnp_cli` rebuild gate (162's AC-9 surface) — unaffected.
- Packets 163/164's WIT/package surfaces; guest WASM; any production crate's `[dependencies]`.
- Moving `pnp_cli_freshness_tdd.rs` out of the slicer-runtime `integration` bucket — it stays as the regression home 162 registered.

## Authoritative Docs

- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` - long; ranged reads only (§"Grounding corrections" 1/4/6, §"Exports ledger" From #1, §"Packet Queue" row 4).
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - short; read whole.
- `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - long; ranged reads only — direct read of §"CLI freshness — three sites, fixed in place" and §"Open Questions" `[FWD]` only.

## Acceptance Summary

- Positive: `AC-1` through `AC-8` in `packet.spec.md` (`AC-8` added by the premise correction). Refinement: AC-3's re-export requirement exists so that the `slicer-runtime` e2e/integration files calling `common::slicer_cache::*` need zero edits — if any of them *other than the four named locator-copy sites* changes in the diff, the extraction went wrong. The count is single-digit, not the "~30" earlier revisions claimed (re-derive: `rg -l 'slicer_cache' crates/slicer-runtime/tests/`); the requirement does not depend on it.
- Negative: `AC-N1`.
- Cross-packet impact: none forward — 163/164 do not touch the three sites. Backward: 162's AC-8 grep contract (each site names `staleness_reason`, no fallback loop, the missing/stale-binary panic names `cargo build -p pnp-cli` and never `cargo build --workspace`) must remain true post-extraction; AC-3/AC-N1 encode that. Post-extraction the remedy wording is asserted in `crates/slicer-test-support/src/lib.rs` (`pnp_cli_bin` / `staleness_reason`), not in `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` — that file's `fn bin` is deleted and its only remaining occurrence of the string is an explanatory comment above `fn workspace_root`, which is not behavior and must not be pinned by a grep.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | all targets (incl. bench + scheduler tests) still compile | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate incl. new crate | FACT pass/fail |
| AC-1 command (`cargo check -p slicer-test-support` + python fn/dep audit) | new crate shape | FACT PASS/FAIL line |
| AC-2 command (rg single-definition counts) | triplication actually gone | FACT PASS/FAIL line |
| AC-3 command (site-consumption python audit) | all seven sites consume the crate | FACT PASS/FAIL line |
| AC-8 command (premise-correction python audit over the four discovered sites) | the four newly-found copies are gone, the inert `PROFILE` branch is gone, and each names `slicer_test_support` | FACT PASS/FAIL line |
| `(cargo test -p slicer-scheduler --test scheduler_integration -- dag_ 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. [1-9][0-9]* passed')` | site 3 exercised end-to-end (name filter matching the `dag_*` test fns in `dag_cli_integration.rs`; 10 today — the filter matches the module path, so `diagnose_*` fns count too) | FACT pass/fail + result line |
| `(cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. [1-9][0-9]* passed')` | staleness_reason contract survives the move (name filter; 0 passed = FAIL) | FACT pass/fail + result line |
| `cargo bench -p slicer-runtime --bench gate_evidence --no-run` | site 2 compiles against dev-deps (compile-only; never run the bench in this packet) | FACT pass/fail |
| `(cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. [1-9][0-9]* passed')` | baseline stays green — expect `3 passed; 0 failed; 0 ignored` (name filter; 0 passed = FAIL) | FACT pass/fail + result line |
| `(cargo test -p slicer-runtime --test e2e -- legacy_zero_matches_golden 2>&1 \| tee target/test-output.log \| rg '^test result: ok\. [1-9][0-9]* passed')` | baseline e2e that spawns via `slicer_cache` — expect `1 passed; 0 failed` (site 1 exercised) | FACT pass/fail + result line |
| AC-7 command (ADR existence + content audit) | the home decision is recorded | FACT PASS/FAIL line |
| AC-N1 command (fallback-resurrection + remedy-wording audit) | 162's trap stays closed; remedy wording lives in the shared crate, `--workspace` wording in neither the shared crate nor the dag test | FACT PASS/FAIL line |

No `cargo test --workspace` anywhere in this packet: the change surface is test-only plumbing in two crates plus one new dependency-free crate; the targeted runs above exercise every consumer, and `--all-targets` check/clippy proves compilation of everything else.

## Step Completion Expectations

- The ADR (Step 1) must be written **before** the crate (Step 2): the crate's rustdoc and the packet's diff cite the ADR by its derived number, and deriving the number after creating files invites the frozen-ledger-fact failure this queue has hit repeatedly.
- Between Step 3 and Step 4 the workspace must compile at every commit point — there is no intentionally-broken window in this packet; a step that leaves `cargo check --workspace --all-targets` red is incomplete.

## Context Discipline Notes

- `crates/slicer-runtime/tests/common/slicer_cache.rs` is long; ranged reads only. Read only the locator block (locate `pnp_cli_bin`, `staleness_reason`, `newest_source_mtime` by name) plus the `use`/module header. The cache machinery (`cached_run`, `execute_slicer`, staging dirs) is out of scope — do not read it.
- Do not open `docs/07_implementation_status.md` directly; the TASK-146d row is added via a worker dispatch.
- Line numbers in this packet's lineage (162's design cites e.g. `:15-31`) are navigation hints only and were captured pre-162; every citation resolves by symbol name.
