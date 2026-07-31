---
status: implemented
packet: 165_cli-binary-locator-extraction
task_ids:
  - TASK-146d
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 165_cli-binary-locator-extraction

## Goal

Collapse the **seven** copies of the `pnp_cli` binary locator into one new std-only host-side crate `crates/slicer-test-support`, with an ADR recording that home decision.

**Premise correction (found at implementation time, not part of the original authoring).** This packet was authored on the belief that the locator was triplicated at three sites. A verification sweep during implementation found **seven**. Packet 162 fixed the freshness bug at three of them:

- `crates/slicer-runtime/tests/common/slicer_cache.rs` (`pnp_cli_bin`)
- `crates/slicer-runtime/benches/gate_evidence.rs` (`pnp_cli_bin`)
- `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` (its copy is named `bin`)

Four more were never in 162's scope and carry **no freshness gate at all** (`staleness_reason` appears zero times in each) — the exact false-baseline trap 162 closed at the other three, still open:

- `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`
- `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`
- `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`
- `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`

Those four additionally branch on `std::env::var("PROFILE")`, which Cargo sets for **build scripts**, not for test binaries — so they always resolve `target/debug/pnp_cli` regardless of the profile the tests were built with. The packet is widened (user-approved) to migrate all seven, which both makes AC-2 satisfiable and extends 162's freshness gate to the four that never had one.

## Scope Boundaries

This is mostly a tidiness packet with one substantive correction: packet 162 already killed the staleness *bug* at three of the seven sites, and this packet removes the sevenfold duplication while extending that gate to the remaining four. It authors one ADR, creates one dependency-free crate exposing the locator (`pnp_cli_bin`, `staleness_reason`, `newest_source_mtime`, `workspace_root`), and points all seven sites at it via dev-dependencies. At the three 162-fixed sites the **freshness algorithm and scan scope are unchanged** — `newest_source_mtime`'s body moves byte-identically, `staleness_reason` keeps its three arms and its `newest_src_mtime > artifact_mtime` comparison direction, and `pnp_cli_bin` keeps its profile inference (`current_exe().parent().parent()` sibling lookup) — and it remains a mirror of `is_stale` in `xtask/src/build_guests.rs` (`xtask` is bin-only and cannot be depended on). The panic *message text* is **not** unchanged; it was deliberately reconciled across three divergent copies (see §Declared Deviation). At the four newly-discovered sites there **is** an intended behavior change: they gain the freshness gate and lose the inert `PROFILE` branch. It does not touch WIT, guests, the scheduler's production code, or any production code path.

### Declared Deviation — panic message text reconciled, not moved verbatim

**What changed.** Five strings differ from `git show HEAD:crates/slicer-runtime/tests/common/slicer_cache.rs`, all inside `crates/slicer-test-support/src/lib.rs`:

- `staleness_reason`'s absent arm: remedy ``cargo build --bin pnp_cli`` → ``cargo build -p pnp-cli``.
- `staleness_reason`'s stale arm: `"pnp_cli is stale at its resolved path; run \`cargo build --bin pnp_cli\` to rebuild it."` → `"pnp_cli is stale: it is older than crates/*/src/**; run \`cargo build -p pnp-cli\` to rebuild it."`
- `pnp_cli_bin`'s staleness panic gained a trailing hint: ``Note: a narrow `cargo test -p <crate>` does not rebuild another package's binary.``
- `pnp_cli_bin`'s unresolvable-exe panic: `integration-test executable` → `test executable`, `--bin pnp_cli` → `-p pnp-cli`.
- `workspace_root`'s `expect` text: `"repo root canonicalize"` → `"workspace root canonicalize"` (follows the rename of `repo_root` → `workspace_root`).

**Why a verbatim move was impossible.** The three pre-extraction copies were **not identical**, so there was no single verbatim text to move. Verified against `git show HEAD:crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`: that copy's `staleness_reason` stale arm read `"pnp_cli is older than crates/*/src/** and must be rebuilt."` — it did **not** contain the word `stale` and named no remedy at all. Packet 162's loudness contract (message names `pnp_cli`, contains `stale`, names a remedy) therefore genuinely held at only two of the three sites. Collapsing three divergent copies into one forces a choice of wording; the implementer chose the reconciled text.

**Why this strengthens rather than weakens 162's contract.** The reconciled text satisfies 162's loudness contract at all seven sites — including the dag copy that previously failed it and the four late-discovered sites that had no gate at all. It also names the narrower, correct remedy (`cargo build -p pnp-cli`, per AC-N1) everywhere, where one copy previously named the stale `--bin pnp_cli` form.

**Why 162's registered regression test still passes.** `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` asserts substring predicates, never an exact string: `older_binary_is_stale` requires `contains("pnp_cli")`, `contains("stale")`, and `contains("cargo build -p pnp-cli")`; `absent_binary_is_stale` requires `contains("pnp_cli")` and the same remedy substring; `fresh_binary_is_not_stale` requires `None`. The reconciled text satisfies every one of them, and the remedy assertions can only be satisfied by the reconciled wording — the pre-extraction `--bin pnp_cli` form would fail them. The tests were never coupled to the exact message text.

## Prerequisites and Blockers

- Depends on: `162_wit-lifecycle-export-removal` (queue #1) **implemented and landed**. 165 refactors the post-162 shape of the three sites (each carrying `staleness_reason` + freshness assert, no release/debug fallback loop). Precondition check (run before starting; the pre-162 tree fails it): `[ "$(rg -l 'staleness_reason' crates/slicer-runtime/tests/common/slicer_cache.rs crates/slicer-runtime/benches/gate_evidence.rs crates/slicer-scheduler/tests/integration/dag_cli_integration.rs | wc -l)" = "3" ] && echo READY || echo 'BLOCKED: 162 not landed'`. Packet 162's exports ledger (`docs/specs/adr-0045-per-stage-wit-packages-plan.md` §"Exports ledger" → "From #1") is the contract for what each site holds.
- Unblocks: nothing in the ADR-0045 queue. Independent of #2/#3 (163/164); may land before or after them.
- Activation blockers: none once the precondition check prints `READY`.

## Acceptance Criteria

- **AC-1. Given** no shared home exists for the locator, **when** `crates/slicer-test-support` is created and registered as a workspace member, **then** its `src/lib.rs` defines exactly one each of `pub fn pnp_cli_bin`, `pub fn staleness_reason`, `pub fn newest_source_mtime`, and `pub fn workspace_root`, its `Cargo.toml` declares zero `[dependencies]` entries, and the crate type-checks. | `cargo check -p slicer-test-support && python3 -c "import re; s=open('crates/slicer-test-support/src/lib.rs',encoding='utf-8').read(); c={n: len(re.findall(r'pub fn '+n+r'\b', s)) for n in ('pnp_cli_bin','staleness_reason','newest_source_mtime','workspace_root')}; t=open('crates/slicer-test-support/Cargo.toml',encoding='utf-8').read(); dep=re.search(r'\[dependencies\]\s*\n\s*\w', t); print('PASS' if all(v==1 for v in c.values()) and not dep else f'FAIL fns={c} has_deps={bool(dep)}')"`

- **AC-2. Given** the workspace holds seven `fn`-level copies of the locator today (see §Goal's premise correction), **when** the extraction lands, **then** exactly one file in `crates/` (the new crate's `src/lib.rs`) defines `fn staleness_reason(`, exactly one defines `fn pnp_cli_bin(`, and exactly one defines `fn newest_source_mtime(`. | `a=$(rg -l 'fn staleness_reason\(' crates/ | wc -l); b=$(rg -l 'fn pnp_cli_bin\(' crates/ | wc -l); c=$(rg -l 'fn newest_source_mtime\(' crates/ | wc -l); [ "$a" = "1" ] && [ "$b" = "1" ] && [ "$c" = "1" ] && echo PASS || echo "FAIL staleness=$a bin=$b mtime=$c"`

- **AC-3. Given** the seven consumer sites — `crates/slicer-runtime/tests/common/slicer_cache.rs`, `crates/slicer-runtime/benches/gate_evidence.rs`, `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`, `crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`, `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`, `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs` — **when** each is pointed at the shared crate, **then** each names `slicer_test_support` (either a qualified `slicer_test_support::pnp_cli_bin()` call or a `use slicer_test_support::pnp_cli_bin;` import — the check accepts both name-resolution-equivalent forms and therefore searches for the bare string), none defines a local `fn pnp_cli_bin(`, `fn staleness_reason(`, `fn newest_source_mtime(`, or (in `dag_cli_integration.rs`) `fn bin(`, and `slicer_cache.rs` re-exports the locator (`pub use slicer_test_support::`) so its downstream test callers are untouched. The caller population is smaller than earlier revisions of this packet claimed (they said "~30"): re-derive with `rg -l 'slicer_cache' crates/slicer-runtime/tests/`, which returns a single-digit file count, of which **exactly one** — `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs`, via `use crate::common::slicer_cache::staleness_reason` — consumes a re-exported *locator* symbol; the rest call the cache API (`cached_run`, `run_pnp_cli_uncached`, `expect_outcome`). The re-export's justification is unchanged and does not rest on the magnitude: it preserves packet 162's registered regression home without relocating that test (see §Rejected alternatives in `design.md`). | `python3 -c "import re; F=['crates/slicer-runtime/tests/common/slicer_cache.rs','crates/slicer-runtime/benches/gate_evidence.rs','crates/slicer-scheduler/tests/integration/dag_cli_integration.rs','crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs','crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs','crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs','crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs']; S=[open(p,encoding='utf-8').read() for p in F]; missing=[p for p,s in zip(F,S) if 'slicer_test_support' not in s]; local=[p for p,s in zip(F,S) if re.search(r'fn (pnp_cli_bin|staleness_reason|newest_source_mtime)\(', s)]; dagbin=bool(re.search(r'\bfn bin\(', S[2])); reexp='pub use slicer_test_support::' in S[0]; print('PASS' if not missing and not local and not dagbin and reexp else f'FAIL missing={missing} local={local} dag_fn_bin={dagbin} reexport={reexp}')"`

- **AC-4. Given** the dag CLI tests spawned via the third copy (test fns named `dag_stages_*`, `dag_stage_*`, `dag_depends_*`, `dag_claims_*` in `dag_cli_integration.rs` — 10 match the `dag_` filter today — the filter matches the module path `dag_cli_integration::`, so it sweeps the three `diagnose_*` fns as well as the seven `dag_*` ones), **when** they run against the shared locator, **then** `cargo test -p slicer-scheduler --test scheduler_integration -- dag_` passes with a non-zero test count (name filter — `0 passed` means the filter matched nothing and is a FAIL). | `mkdir -p target && (cargo test -p slicer-scheduler --test scheduler_integration -- dag_ 2>&1 | tee target/test-output.log | rg '^test result: ok\. [1-9][0-9]* passed') || echo 'FAIL: 0 dag_ tests ran'`

- **AC-5. Given** packet 162's freshness regression tests (`pnp_cli_freshness_tdd` in the slicer-runtime `integration` bucket) exercise `staleness_reason`'s three synthetic-mtime cases (stale binary ⇒ `Some` containing `pnp_cli` + `stale`; absent binary ⇒ `Some`; fresh binary ⇒ `None`), **when** the function moves to the shared crate (reached through `slicer_cache.rs`'s re-export), **then** the same tests still pass with a non-zero count. | `mkdir -p target && (cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 | tee target/test-output.log | rg '^test result: ok\. [1-9][0-9]* passed') || echo 'FAIL: 0 freshness tests ran'`

- **AC-6. Given** `gate_evidence.rs` is a `harness = false` bench target (bench targets receive dev-dependencies like any test target), **when** it imports the shared crate instead of its self-contained mirror, **then** it compiles. Compile-only — running it slices a 50-layer fixture and is deliberately excluded ("slow; not in CI" per `CLAUDE.md`). | `cargo bench -p slicer-runtime --bench gate_evidence --no-run > target/ac6.log 2>&1 && rg -q 'slicer_test_support' crates/slicer-runtime/benches/gate_evidence.rs && echo PASS || { echo 'FAIL: bench target does not compile, or does not import the shared crate'; tail -20 target/ac6.log; }`

- **AC-7. Given** no ADR governs host-side test support (ADR-0004 covers only guest-side support in `slicer-sdk`; `slicer-test` was deleted by packet 78, commit `c68f8973`), **when** the ADR is authored at the next free number (re-derive at write time: `ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1`, then +1; never trust a number frozen in this packet), **then** `docs/adr/<NNNN>-host-side-test-support-crate.md` exists with `Accepted` status, decides for `slicer-test-support`, and rejects at minimum: the `pnp-cli` lib `test-support` feature, `slicer-sdk` (ADR-0004), an `xtask` lib target, and reviving `slicer-test` (packet 78). | `f=$(ls docs/adr/*-host-side-test-support-crate.md 2>/dev/null | head -1) && python3 -c "s=open('$f',encoding='utf-8').read(); need=['Accepted','slicer-test-support','ADR-0004','packet 78','test-support','xtask']; miss=[n for n in need if n not in s]; print('PASS' if not miss else f'FAIL missing={miss}')" || echo 'FAIL: ADR file absent'`

- **AC-8. Given** the four locator copies discovered at implementation time (`crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs`, `crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs`, `crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs`, `crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs`) each define their own `fn pnp_cli_bin(` with no freshness gate and branch on `std::env::var("PROFILE")` — a variable Cargo sets for build scripts, not test binaries, so it always resolves `target/debug/pnp_cli`, **when** the premise correction is applied and all four are migrated to the shared crate, **then** none of the four defines a local `fn pnp_cli_bin(`, none mentions `PROFILE`, and each names `slicer_test_support` (qualified call or `use` import — both accepted). This criterion FAILs on the pre-migration tree, which is what makes it load-bearing. | `python3 -c "import re,sys; F=['crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs','crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs','crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs','crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs']; S=[open(p,encoding='utf-8').read() for p in F]; local=[p for p,s in zip(F,S) if re.search(r'fn pnp_cli_bin\(', s)]; prof=[p for p,s in zip(F,S) if 'PROFILE' in s]; noimp=[p for p,s in zip(F,S) if 'slicer_test_support' not in s]; print('PASS' if not local and not prof and not noimp else f'FAIL local={local} profile_branch={prof} missing_import={noimp}')"`

## Negative Test Cases

- **AC-N1. Given** packet 162 removed every release/debug fallback loop (the stale-binary trap) and mandated that the missing/stale-binary panic name the remedy `cargo build -p pnp-cli` rather than the too-broad `cargo build --workspace`, **when** the locator is centralized, **then** (a) no `for profile in ["release", "debug"]` / `["debug", "release"]` loop and no `.join("release")` probe exists in the new crate or any of the seven sites, (b) the remedy wording `cargo build -p pnp-cli` is present in `crates/slicer-test-support/src/lib.rs`, which is where the panic bodies of `pnp_cli_bin` and `staleness_reason` now live, (c) the string `cargo build --workspace` appears in **neither** `crates/slicer-test-support/src/lib.rs` **nor** `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`, and (d) all seven call sites name `slicer_test_support` — via a qualified path or a `use` import, both accepted (the check searches the bare string) — and the shared crate file exists. A regression on (a)–(c) re-opens the false-baseline trap 162 closed.

  **Deliberate omission — do not re-add.** An earlier revision of this criterion also required the literal `cargo build -p pnp-cli` to appear *inside* `dag_cli_integration.rs`. That was correct only while that file defined its own `fn bin()` carrying the panic text. The extraction deletes `fn bin()`; the executable remedy wording moves to `pnp_cli_bin` / `staleness_reason` in `crates/slicer-test-support/src/lib.rs`, and the only surviving occurrence in `dag_cli_integration.rs` is an explanatory comment above `fn workspace_root`. Asserting on that occurrence would pin a comment's wording in place while proving nothing about runtime behavior, and would still pass if the real panic message were wrong. The clause is therefore asserted against the shared crate only.

  **Load-bearing vs. pre-existing.** Load-bearing (can only pass because this packet's work landed): the existence of `crates/slicer-test-support/src/lib.rs`, clause (b) — the remedy wording living in the shared crate — and clause (d)'s seven-site `slicer_test_support` references. Already true before this packet (162 closed them at the three sites it covered, and the four late-discovered sites never had a fallback loop): clause (a)'s no-loop / no-`.join("release")` assertions and clause (c)'s absence of `cargo build --workspace`. | `python3 -c "import re,os; F=['crates/slicer-test-support/src/lib.rs','crates/slicer-runtime/tests/common/slicer_cache.rs','crates/slicer-runtime/benches/gate_evidence.rs','crates/slicer-scheduler/tests/integration/dag_cli_integration.rs','crates/slicer-runtime/tests/integration/no_linker_module_degraded_raw_output_tdd.rs','crates/slicer-runtime/tests/e2e/infill_overlap_changes_gcode_tdd.rs','crates/slicer-runtime/tests/e2e/modifier_infill_tdd.rs','crates/slicer-runtime/tests/e2e/wedge_linked_infill_report_tdd.rs']; missing=[p for p in F if not os.path.exists(p)]; S=[open(p,encoding='utf-8').read() if os.path.exists(p) else '' for p in F]; loop=[p for p,s in zip(F,S) if re.search(r'for\s+profile\s+in\s*\[\s*\"(release|debug)\"\s*,\s*\"(debug|release)\"\s*\]', s)]; probe=[p for p,s in zip(F,S) if re.search(r'\.join\(\s*\"release\"\s*\)', s)]; shared=[p for p,s in zip(F[1:],S[1:]) if 'slicer_test_support' not in s]; remedy='cargo build -p pnp-cli' in S[0]; ws=[p for p,s in ((F[0],S[0]),(F[3],S[3])) if 'cargo build --workspace' in s]; print('PASS' if not missing and not loop and not probe and not shared and remedy and not ws else f'FAIL missing={missing} loop={loop} probe={probe} not_using_shared_crate={shared} remedy_in_shared_crate={remedy} workspace_wording={ws}')"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && (cargo test -p slicer-runtime --test integration pnp_cli_freshness 2>&1 | tee target/test-output.log | rg '^test result: ok\. [1-9][0-9]* passed') || echo 'FAIL: 0 tests ran'` — the `rg '^test result: ok\. [1-9][0-9]* passed'` guard is **mandatory on every name-filtered `cargo test` gate in this packet**: it fails both when the filter matches nothing (`ok. 0 passed`) and when tests ran and failed (`FAILED. N passed; M failed`). The earlier `rg -v '0 passed'` form caught only the first case — a genuinely failing run passed it silently — and mis-fired on any count ending in 0 (`10 passed` contains `0 passed`). Unfiltered whole-binary runs do not need it.

## Authoritative Docs

- `docs/specs/adr-0045-per-stage-wit-packages-plan.md` (long; ranged reads only) - direct read of §"Grounding corrections" items 1, 4, 6 and §"Exports ledger" → "From #1" only.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (short; read whole) - direct read; the boundary the new ADR must not cross.
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
