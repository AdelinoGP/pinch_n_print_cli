---
status: draft
packet: 194-check-literals-gate
task_ids:
  - TASK-316
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 194-check-literals-gate

## Goal

Implement `cargo xtask check-literals` — a syn-based scanner that flags exhaustive struct literals of watched types in test code (report mode, path filter, exit 1 on violations) — and author its rule documentation (`docs/21_data_defaults_and_fixtures.md`, `.claude/doc-index.md` entry, CLAUDE.md MUST section explicitly marked gate-off until packet 199).

## Scope Boundaries

One new xtask module (`xtask/src/check_literals.rs`) plus its `main.rs` dispatch arm and `xtask/Cargo.toml` deps; one new docs page and two small doc edits. No wiring into `cargo xtask test` preflight, no CLAUDE.md required-before-commit change, and no call-site conversion anywhere — those belong to packets 199 and 196–198 respectively.

## Prerequisites and Blockers

- Depends on: nothing (queue row #1 of `docs/specs/struct-literal-churn-gate-plan.md`).
- Unblocks: packets 195 (uses `--report` to enumerate), 196–198 (use the path filter for per-area green), 199 (flips enforcement on).
- Activation blockers: none known.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** the current pre-sweep tree, **when** `cargo xtask check-literals --report` runs, **then** it exits 0 and its final stdout line matches `check-literals: <N> violation(s) in <M> file(s) (watchlist: <K> types)` with N ≥ 1 and K ≥ 1. | `cargo xtask check-literals --report > /tmp/cl194.txt 2>&1; ec=$?; tail -1 /tmp/cl194.txt; test $ec -eq 0 && tail -1 /tmp/cl194.txt | grep -qE '^check-literals: [1-9][0-9]* violation' && echo PASS`
- **AC-2. Given** the current pre-sweep tree, **when** `cargo xtask check-literals` runs with no arguments (enforce mode), **then** it exits 1 and prints at least one violation line of the form `<ws-relative-path>.rs:<line>: exhaustive literal of watched type \`<TypeName>\`` (forward-slash paths). | `cargo xtask check-literals > /tmp/cl194e.txt 2>&1; ec=$?; test $ec -eq 1 && grep -qE '\.rs:[0-9]+: exhaustive literal of watched type' /tmp/cl194e.txt && echo PASS`
- **AC-3. Given** the path filter `crates/slicer-ir`, **when** `cargo xtask check-literals --report crates/slicer-ir` runs, **then** every printed violation line's path starts with `crates/slicer-ir/`, and the summary line is still printed. | `cargo xtask check-literals --report crates/slicer-ir > /tmp/cl194p.txt 2>&1; bad=$(grep -E '\.rs:[0-9]+: exhaustive literal' /tmp/cl194p.txt | grep -vc '^crates/slicer-ir/'); test "$bad" -eq 0 && grep -qE '^check-literals: ' /tmp/cl194p.txt && echo PASS`
- **AC-4. Given** an in-memory fixture source defining a `pub` struct with 5 named fields, a `pub(crate)` struct with 6 named fields, a `pub` tuple struct with 6 fields, and a `pub` struct with 4 named fields, **when** watchlist derivation parses it, **then** only the first struct's name is watched. | `mkdir -p target && cargo test -p xtask watchlist_includes_pub_ge5_named_structs_only 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** test-scope fixture sources containing (i) a watched-type literal with `..Default::default()`, (ii) an exhaustive watched literal with `// exhaustive: reason` on its opening line, (iii) the same waiver on the line immediately above, and (iv) `Self { ..base }` inside `impl <WatchedType>`, **when** scanned, **then** zero violations are reported. | `mkdir -p target && cargo test -p xtask scan_passes_fru_and_waivered_literals 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** test-scope fixture sources containing (i) a plain exhaustive watched literal, (ii) exhaustive `Self { }` inside `impl <WatchedType>`, (iii) a multi-segment path literal `slicer_ir::<WatchedType> { }`, and (iv) a watched literal inside `vec![...]` / `assert_eq!(...)` token trees without a top-level `..`, **when** scanned, **then** each is reported as a violation carrying the fixture's 1-based line number. | `mkdir -p target && cargo test -p xtask 'scan_flags_' 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** fixture sources containing (i) an enum struct-variant literal `SomeEnum::Variant { .. }` whose variant name is unwatched and (ii) an exhaustive watched literal in plain (non-`#[cfg(test)]`) src position scanned in cfg-test-only mode, **when** scanned, **then** zero violations are reported for both. | `mkdir -p target && cargo test -p xtask scan_ignores_enum_variants_and_non_test_src 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-8. Given** the docs deliverables, **when** grepped, **then** `docs/21_data_defaults_and_fixtures.md` exists containing the waiver format, the production-exemption rationale citing `a579fc18`, and the `clippy::needless_update` guidance; `.claude/doc-index.md` lists the page; and `docs/00_project_overview.md`'s "Normative Document Map" table has a row for it. | `rg -qF '// exhaustive:' docs/21_data_defaults_and_fixtures.md && rg -q 'a579fc18' docs/21_data_defaults_and_fixtures.md && rg -q 'needless_update' docs/21_data_defaults_and_fixtures.md && rg -q '21_data_defaults_and_fixtures' .claude/doc-index.md && rg -q '21_data_defaults_and_fixtures' docs/00_project_overview.md && echo PASS`
- **AC-9. Given** the CLAUDE.md rule section, **when** grepped, **then** it names `cargo xtask check-literals`, points at `docs/21_data_defaults_and_fixtures.md`, and contains the literal text `not yet a required gate` plus a reference to packet 199. | `rg -q 'check-literals' CLAUDE.md && rg -q '21_data_defaults_and_fixtures' CLAUDE.md && rg -q 'not yet a required gate' CLAUDE.md && rg -q 'packet 199' CLAUDE.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a fixture source whose waiver comment has no reason text (`// exhaustive:` followed by nothing or only whitespace), **when** scanned, **then** the exhaustive watched literal is still reported as a violation. | `mkdir -p target && cargo test -p xtask scan_requires_waiver_reason 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** a fixture source with an exhaustive watched literal inside a macro token tree that contains a top-level range expression (`field: 0..2`), **when** scanned, **then** the current implementation misses it (0 violations) — a test locks this documented blind spot so a future fix must consciously flip the assertion. | `mkdir -p target && cargo test -p xtask scan_macro_range_blind_spot_documented 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N3. Given** an unknown flag, **when** `cargo xtask check-literals --bogus` runs, **then** it exits 2 and prints the usage text. | `cargo xtask check-literals --bogus > /tmp/cl194u.txt 2>&1; test $? -eq 2 && grep -q 'USAGE' /tmp/cl194u.txt && echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p xtask 2>&1 | tee target/test-output.log | grep -E '^test result'` (full checker unit suite)

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read; locked decisions 1, 2, 5 govern this packet.
- `CLAUDE.md` §Test Discipline, §In-Tree Citation Style - direct read of named sections only (doc rules the new CLAUDE.md section must not contradict).

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/21_data_defaults_and_fixtures.md` (new page): rule, production-exemption rationale (`a579fc18` evidence), watchlist derivation, waiver format, fixture policy pointer to `slicer_sdk::test_support` (packet 195), `clippy::needless_update` guidance, known blind spots - `rg -qF '// exhaustive:' docs/21_data_defaults_and_fixtures.md`
- `.claude/doc-index.md` new bullet for the page - `rg -q '21_data_defaults_and_fixtures' .claude/doc-index.md`
- `docs/00_project_overview.md` section "Normative Document Map (LLM/Reviewer Fast Index)": one table row for the new page (bare-filename style, like the existing `20_support_preview.md` row); the precedence rule for conflicts is NOT touched (conventions page, not a normative architecture doc) - `rg -q '21_data_defaults_and_fixtures' docs/00_project_overview.md`
- `CLAUDE.md` new MUST section (rule + command, explicitly gate-off until packet 199) - `rg -q 'not yet a required gate' CLAUDE.md`

Doc greps are appended to the ACs (AC-8, AC-9).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
