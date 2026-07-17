---
status: draft
packet: 165_cli-binary-locator-extraction
task_ids:
  - TASK-146d
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 165_cli-binary-locator-extraction

## Goal

Collapse the three copies of the `pnp_cli` binary locator + freshness assert (left in place, deliberately triplicated, by packet 162) into one new std-only host-side crate `crates/slicer-test-support`, with an ADR recording that home decision.

## Scope Boundaries

This is a tidiness packet: packet 162 already killed the staleness *bug* at all three sites; this packet only removes the triplication. It authors one ADR, creates one dependency-free crate exposing the locator (`pnp_cli_bin`, `staleness_reason`, `newest_source_mtime`, `workspace_root`), and points the three sites at it via dev-dependencies. No behavior change: the same panics, the same messages, the same freshness algorithm (still a mirror of `is_stale` in `xtask/src/build_guests.rs` — `xtask` is bin-only and cannot be depended on). It does not touch WIT, guests, the scheduler, or any production code path.

## Prerequisites and Blockers

- Depends on: `162_wit-lifecycle-export-removal` (queue #1) **implemented and landed**. 165 refactors the post-162 shape of the three sites (each carrying `staleness_reason` + freshness assert, no release/debug fallback loop). Precondition check (run before starting; the pre-162 tree fails it): `cd F:/slicerProject/pinch_n_print_cli && [ "$(rg -l 'staleness_reason' crates/slicer-runtime/tests/common/slicer_cache.rs crates/slicer-runtime/benches/gate_evidence.rs crates/slicer-scheduler/tests/integration/dag_cli_integration.rs | wc -l)" = "3" ] && echo READY || echo 'BLOCKED: 162 not landed'`. Packet 162's exports ledger (`docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Exports ledger" → "From #1") is the contract for what each site holds.
- Unblocks: nothing in the ADR-0045 queue. Independent of #2/#3 (163/164); may land before or after them.
- Activation blockers: none once the precondition check prints `READY`.

## Acceptance Criteria

- **AC-1. Given** no shared home exists for the locator, **when** `crates/slicer-test-support` is created and registered as a workspace member, **then** its `src/lib.rs` defines exactly one each of `pub fn pnp_cli_bin`, `pub fn staleness_reason`, `pub fn newest_source_mtime`, and `pub fn workspace_root`, its `Cargo.toml` declares zero `[dependencies]` entries, and the crate type-checks. | `cd F:/slicerProject/pinch_n_print_cli && cargo check -p slicer-test-support && python3 -c "import re; s=open('crates/slicer-test-support/src/lib.rs',encoding='utf-8').read(); c={n: len(re.findall(r'pub fn '+n+r'\b', s)) for n in ('pnp_cli_bin','staleness_reason','newest_source_mtime','workspace_root')}; t=open('crates/slicer-test-support/Cargo.toml',encoding='utf-8').read(); dep=re.search(r'\[dependencies\]\s*\n\s*\w', t); print('PASS' if all(v==1 for v in c.values()) and not dep else f'FAIL fns={c} has_deps={bool(dep)}')"`

- **AC-2. Given** the workspace holds three `fn`-level copies of the locator today, **when** the extraction lands, **then** exactly one file in `crates/` (the new crate's `src/lib.rs`) defines `fn staleness_reason(`, exactly one defines `fn pnp_cli_bin(`, and exactly one defines `fn newest_source_mtime(`. | `cd F:/slicerProject/pinch_n_print_cli && a=$(rg -l 'fn staleness_reason\(' crates/ | wc -l); b=$(rg -l 'fn pnp_cli_bin\(' crates/ | wc -l); c=$(rg -l 'fn newest_source_mtime\(' crates/ | wc -l); [ "$a" = "1" ] && [ "$b" = "1" ] && [ "$c" = "1" ] && echo PASS || echo "FAIL staleness=$a bin=$b mtime=$c"`

- **AC-3. Given** the three consumer sites — `crates/slicer-runtime/tests/common/slicer_cache.rs`, `crates/slicer-runtime/benches/gate_evidence.rs`, `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` — **when** each is pointed at the shared crate, **then** each contains `slicer_test_support::`, none defines a local `fn pnp_cli_bin(`, `fn staleness_reason(`, `fn newest_source_mtime(`, or (in `dag_cli_integration.rs`) `fn bin(`, and `slicer_cache.rs` re-exports the locator (`pub use slicer_test_support::`) so its ~30 downstream test callers are untouched. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "import re; F=['crates/slicer-runtime/tests/common/slicer_cache.rs','crates/slicer-runtime/benches/gate_evidence.rs','crates/slicer-scheduler/tests/integration/dag_cli_integration.rs']; S=[open(p,encoding='utf-8').read() for p in F]; missing=[p for p,s in zip(F,S) if 'slicer_test_support::' not in s]; local=[p for p,s in zip(F,S) if re.search(r'fn (pnp_cli_bin|staleness_reason|newest_source_mtime)\(', s)]; dagbin=bool(re.search(r'\bfn bin\(', S[2])); reexp='pub use slicer_test_support::' in S[0]; print('PASS' if not missing and not local and not dagbin and reexp else f'FAIL missing={missing} local={local} dag_fn_bin={dagbin} reexport={reexp}')"`

- **AC-4. Given** the dag CLI tests spawned via the third copy (test fns named `dag_stages_*`, `dag_stage_*`, `dag_depends_*`, `dag_claims_*` in `dag_cli_integration.rs` — 7 match the `dag_` filter today; there is no test named `dag_cli`), **when** they run against the shared locator, **then** `cargo test -p slicer-scheduler --test scheduler_integration -- dag_` passes with a non-zero test count (name filter — `0 passed` means the filter matched nothing and is a FAIL). | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && (cargo test -p slicer-scheduler --test scheduler_integration -- dag_ 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 dag_ tests ran'`

- **AC-5. Given** packet 162's freshness regression tests (`pnp_cli_freshness_tdd` in the slicer-runtime `integration` bucket) exercise `staleness_reason`'s three synthetic-mtime cases (stale binary ⇒ `Some` containing `pnp_cli` + `stale`; absent binary ⇒ `Some`; fresh binary ⇒ `None`), **when** the function moves to the shared crate (reached through `slicer_cache.rs`'s re-export), **then** the same tests still pass with a non-zero count. | `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && (cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 freshness tests ran'`

- **AC-6. Given** `gate_evidence.rs` is a `harness = false` bench target (bench targets receive dev-dependencies like any test target), **when** it imports the shared crate instead of its self-contained mirror, **then** it compiles. Compile-only — running it slices a 50-layer fixture and is deliberately excluded ("slow; not in CI" per `CLAUDE.md`). | `cd F:/slicerProject/pinch_n_print_cli && cargo bench -p slicer-runtime --bench gate_evidence --no-run 2>&1 | tail -3`

- **AC-7. Given** no ADR governs host-side test support (ADR-0004 covers only guest-side support in `slicer-sdk`; `slicer-test` was deleted by packet 78, commit `c68f8973`), **when** the ADR is authored at the next free number (re-derive at write time: `ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1`, then +1; never trust a number frozen in this packet), **then** `docs/adr/<NNNN>-host-side-test-support-crate.md` exists with `Accepted` status, decides for `slicer-test-support`, and rejects at minimum: the `pnp-cli` lib `test-support` feature, `slicer-sdk` (ADR-0004), an `xtask` lib target, and reviving `slicer-test` (packet 78). | `cd F:/slicerProject/pinch_n_print_cli && f=$(ls docs/adr/*-host-side-test-support-crate.md 2>/dev/null | head -1) && python3 -c "s=open('$f',encoding='utf-8').read(); need=['Accepted','slicer-test-support','ADR-0004','packet 78','test-support','xtask']; miss=[n for n in need if n not in s]; print('PASS' if not miss else f'FAIL missing={miss}')" || echo 'FAIL: ADR file absent'`

## Negative Test Cases

- **AC-N1. Given** packet 162 removed every release/debug fallback loop (the stale-binary trap), **when** the locator is centralized, **then** no `for profile in ["release", "debug"]` / `["debug", "release"]` loop and no `.join("release")` probe exists in the new crate or the three sites, and `dag_cli_integration.rs`'s panic path still names `cargo build -p pnp-cli` (not `cargo build --workspace`). A regression here re-opens the false-baseline trap 162 closed. | `cd F:/slicerProject/pinch_n_print_cli && python3 -c "import re; F=['crates/slicer-test-support/src/lib.rs','crates/slicer-runtime/tests/common/slicer_cache.rs','crates/slicer-runtime/benches/gate_evidence.rs','crates/slicer-scheduler/tests/integration/dag_cli_integration.rs']; S=[open(p,encoding='utf-8').read() for p in F]; loop=[p for p,s in zip(F,S) if re.search(r'for profile in \[\"(release|debug)\", \"(debug|release)\"\]', s)]; probe=[p for p,s in zip(F,S) if '.join(\"release\")' in s]; d=S[3]; print('PASS' if not loop and not probe and 'cargo build -p pnp-cli' in d and 'cargo build --workspace' not in d else f'FAIL loop={loop} probe={probe}')"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cd F:/slicerProject/pinch_n_print_cli && mkdir -p target && (cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 | tee target/test-output.log | rg '^test result') | rg -v '0 passed' || echo 'FAIL: 0 tests ran'` — the `rg -v '0 passed'` guard is mandatory on every name-filtered `cargo test` gate in this packet.

## Authoritative Docs

- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` (long; ranged reads only) - direct read of §"Grounding corrections" items 1, 4, 6 and §"Exports ledger" → "From #1" only.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (72 lines) - direct read; the boundary the new ADR must not cross.
- `.ralph/specs/162_wit-lifecycle-export-removal/design.md` §"CLI freshness — three sites, fixed in place" - direct read; the post-162 shape being extracted.
- `CLAUDE.md` §"Test Discipline", §"Ledger Facts Must Be Re-derived, Not Quoted" - direct read.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/adr/<NNNN>-host-side-test-support-crate.md` (new; number re-derived at write time per AC-7) — the home decision. Verification grep: `ls docs/adr/*-host-side-test-support-crate.md`
- `docs/07_implementation_status.md` — record TASK-146d per the existing TASK-119a/TASK-194a sub-lettering convention. Verification grep: `rg -q 'TASK-146d' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
