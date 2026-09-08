# Implementation Plan: 194-check-literals-gate

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Deps, module skeleton, watchlist derivation (TDD)

- Task IDs: `TASK-316`
- Objective: Add `syn`/`proc-macro2` to xtask, create `check_literals.rs` with `derive_watchlist` and the `Violation` type, and prove the watchlist rule from an in-memory fixture.
- Precondition: clean tree; `cargo xtask check-literals` is an unknown subcommand (exit 2).
- Postcondition: `cargo test -p xtask watchlist_includes_pub_ge5_named_structs_only` passes; the fixture covers: `pub` 5-named-field struct → watched; `pub(crate)` 6-field → excluded; `pub` tuple 6-field → excluded; `pub` 4-field → excluded; a `pub` ≥5-field struct inside an inline `mod` → watched. `derive_watchlist` is factored so the parse-and-collect core takes source text (unit-testable) and the walker only feeds it files.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/check_deviations.rs` - lines 1-60 (run-signature and stderr conventions)
  - `xtask/src/main.rs` - full (160 lines)
- Files allowed to edit (at most 3):
  - `xtask/Cargo.toml`
  - `xtask/src/check_literals.rs` (new)
  - `xtask/src/main.rs` (only `mod check_literals;` with a temporary `#[allow(dead_code)]` on the mod, removed in Step 4)
- Files explicitly out of bounds:
  - everything under `crates/` and `modules/` (fixtures are in-memory strings)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant): not applicable — no workspace struct or constant changes in this step.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 2 (watchlist rule)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p xtask watchlist_includes_pub_ge5_named_structs_only 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - `cargo check -p xtask` - FACT pass/fail
- Exit condition: the watchlist test fails if any of the four exclusion classes leaks into the watchlist or the inline-mod struct is missed; `cargo check -p xtask` compiles with the two new deps resolved.

### Step 2: `scan_source` AST visitor — literals, Self, cfg(test), waivers (TDD)

- Task IDs: `TASK-316`
- Objective: Implement the pure `scan_source(file_label, src, mode, watch)` visitor covering `ExprStruct` violations, impl-target `Self` resolution, `CfgTestOnly` gating, and waiver detection with mandatory reason.
- Precondition: Step 1 merged; `derive_watchlist` green.
- Postcondition: these unit tests pass, each driven by in-memory `.rs` fixture strings: `scan_passes_fru_and_waivered_literals` (FRU rest; waiver same line; waiver line above; `Self { ..base }` in `impl Watched`), `scan_flags_exhaustive_watched_literals` (plain exhaustive literal, asserting the reported 1-based line number), `scan_flags_self_in_impl_blocks` (exhaustive `Self { }` inside `impl Watched` flagged; `Self { }` inside `impl Unwatched` not flagged), `scan_ignores_enum_variants_and_non_test_src` (`SomeEnum::Variant { .. }` with unwatched variant name; exhaustive watched literal outside a `#[cfg(test)]` mod under `ScanMode::CfgTestOnly`), `scan_requires_waiver_reason` (`// exhaustive:` with empty/whitespace remainder does not suppress).
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/check_literals.rs` - full (own module)
- Files allowed to edit (at most 3):
  - `xtask/src/check_literals.rs`
- Files explicitly out of bounds:
  - `xtask/src/main.rs` (wiring is Step 4), all workspace crates
- Blast-radius discipline: not applicable — no workspace struct or constant changes.
- Expected sub-agent dispatches: none.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decisions 1-2 (violation and waiver semantics)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p xtask scan_ 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (macro tests arrive in Step 3; filter matches only existing tests)
- Exit condition: any of the five named tests failing, or a violation reported without a line number, fails the step; waiver acceptance must be proven for both placements and rejected for reason-less waivers.

### Step 3: Macro token-tree scanning and multi-segment paths (TDD)

- Task IDs: `TASK-316`
- Objective: Extend `scan_source` to scan `Macro` token streams (watched `Ident` + brace `Group` lacking top-level `..`) and to match multi-segment paths by last segment; lock the range-expression blind spot.
- Precondition: Step 2 green.
- Postcondition: `scan_flags_macro_embedded_and_multisegment_literals` passes (watched literal inside `vec![…]` flagged; inside `assert_eq!(…)` flagged; `slicer_ir::<Watched> { }` plain-AST literal flagged; macro-embedded literal *with* top-level `..` passes) and `scan_macro_range_blind_spot_documented` passes (exhaustive watched literal whose macro token tree contains `field: 0..2` at top level is currently NOT flagged; the test asserts 0 violations and its doc comment marks this as the locked blind spot).
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/check_literals.rs` - full (own module)
- Files allowed to edit (at most 3):
  - `xtask/src/check_literals.rs`
- Files explicitly out of bounds:
  - `xtask/src/main.rs`, all workspace crates
- Blast-radius discipline: not applicable — no workspace struct or constant changes.
- Expected sub-agent dispatches: none.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 2 (macro token trees, last-path-segment matching)
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p xtask 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (full checker suite to catch Step-2 regressions)
- Exit condition: the macro tests failing, or any Step-2 test regressing, fails the step; the blind-spot test must assert the *miss* (0 violations), not the fix.

### Step 4: Enforced-file walker, CLI wiring, live-run ACs

- Task IDs: `TASK-316`
- Objective: Implement `collect_enforced_files` (scopes + component-aware path filters), `run()` (ordering, output lines, summary, exit codes), wire the `check-literals` match arm + USAGE into `main.rs`, and remove the temporary `#[allow(dead_code)]`.
- Precondition: Steps 1-3 green; `main.rs` still has the temporary allow.
- Postcondition: AC-1 (report exit 0, summary line with N ≥ 1), AC-2 (enforce exit 1, violation line format), AC-3 (path filter restricts to `crates/slicer-ir/`), and AC-N3 (unknown flag → exit 2 + USAGE) all pass; violation output is sorted by (path, line) with forward-slash ws-relative paths.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` - only the `workspace_root` fn (navigation hint: near top of file)
  - `xtask/src/main.rs` - full
- Files allowed to edit (at most 3):
  - `xtask/src/check_literals.rs`
  - `xtask/src/main.rs`
- Files explicitly out of bounds:
  - all workspace crates and `modules/` (the live run reads them; the step never edits them)
- Blast-radius discipline: not applicable — no workspace struct or constant changes.
- Expected sub-agent dispatches:
  - Question: run the four live-run AC commands (AC-1, AC-2, AC-3, AC-N3 from `packet.spec.md`) and `cargo check --workspace --all-targets` + `cargo clippy --workspace --all-targets -- -D warnings`; scope: repo root; return: `FACT` — one PASS/FAIL line per command, ≤ 20 error lines on any failure.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 2 (CLI: enforce/report/path filter)
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals --report > /tmp/cl194.txt 2>&1; ec=$?; tail -1 /tmp/cl194.txt; test $ec -eq 0 && echo OK` - FACT
  - `cargo xtask check-literals > /tmp/cl194e.txt 2>&1; echo "exit=$?"; head -3 /tmp/cl194e.txt` - FACT (expect exit=1)
  - `cargo xtask check-literals --report crates/slicer-ir > /tmp/cl194p.txt 2>&1; grep -E '\.rs:[0-9]+: exhaustive literal' /tmp/cl194p.txt | grep -vc '^crates/slicer-ir/'` - FACT (expect 0)
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail (delegated)
- Exit condition: any live-run AC failing, a backslash appearing in printed paths, the summary line missing, or clippy failing on the removed-allow warning surface fails the step.

### Step 5: Author `docs/21_data_defaults_and_fixtures.md` + doc-index and docs/00 map entries

- Task IDs: `TASK-316`
- Objective: Write the rule page and register it in the doc index and in `docs/00_project_overview.md`'s Normative Document Map table.
- Precondition: Step 4 shipped the final CLI shape and waiver format (the page documents what shipped, not the draft).
- Postcondition: the page exists and covers, in order: the rule (test-code literals of watched types need a `..` rest or a waiver); production-exemption rationale citing `a579fc18` (marshal/producer sites are compiler-enforced propagation checkpoints; FRU there would have silently dropped `overhang_distance_mm` at the WIT boundary); watchlist derivation (`pub`, ≥ 5 named fields, `crates/*/src`, derived at run time); waiver format `// exhaustive: <reason>` with placement rules; fixture policy pointing at `slicer_sdk::test_support` (bases authored by packet 195); `clippy::needless_update` guidance (omit default-equal fields; never spell-all+FRU); known blind spots (macro range expressions, macro-generated struct definitions such as `ResolvedConfig`, enum-variant name collisions, `#[cfg(any(test, feature = "test"))]` mods). `.claude/doc-index.md` gains one bullet in the existing bullet style. `docs/00_project_overview.md`'s "Normative Document Map (LLM/Reviewer Fast Index)" table gains one row (bare-filename style like the `20_support_preview.md` row, inserted before the `DEVIATION_LOG.md` row); the precedence rule for conflicts is not touched.
- Files allowed to read, with ranges when over 300 lines:
  - `.claude/doc-index.md` - full (short)
  - `docs/specs/struct-literal-churn-gate-plan.md` - full (short)
  - `docs/00_project_overview.md` - the "Normative Document Map (LLM/Reviewer Fast Index)" table region only (symbol-anchored window around the table)
- Files allowed to edit (at most 3):
  - `docs/21_data_defaults_and_fixtures.md` (new)
  - `.claude/doc-index.md`
  - `docs/00_project_overview.md` (one table row only)
- Files explicitly out of bounds:
  - `CLAUDE.md` (Step 6), all code; within `docs/00_project_overview.md`, everything outside the Normative Document Map table (in particular the precedence rule for conflicts)
- Blast-radius discipline: not applicable — docs only.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 5 (page contents)
- OrcaSlicer refs: none.
- Verification:
  - `rg -qF '// exhaustive:' docs/21_data_defaults_and_fixtures.md && rg -q 'a579fc18' docs/21_data_defaults_and_fixtures.md && rg -q 'needless_update' docs/21_data_defaults_and_fixtures.md && rg -q '21_data_defaults_and_fixtures' .claude/doc-index.md && rg -q '21_data_defaults_and_fixtures' docs/00_project_overview.md && echo OK` - FACT
- Exit condition: any AC-8 grep failing, or the page describing a CLI/waiver shape different from what Step 4 shipped, fails the step.

### Step 6: CLAUDE.md MUST section (gate-off)

- Task IDs: `TASK-316`
- Objective: Add a short MUST section to CLAUDE.md describing the rule and command, explicitly marked not yet enforced.
- Precondition: Steps 4-5 complete.
- Postcondition: CLAUDE.md contains a new section (placed near the existing test-discipline material) that: states the test-code FRU-or-waiver rule in ≤ 6 lines; names `cargo xtask check-literals` and the `--report` mode; points at `docs/21_data_defaults_and_fixtures.md`; and contains the literal sentence fragment `not yet a required gate — enforcement flips on in packet 199`. It must NOT add the command to the required-before-commit list.
- Files allowed to read, with ranges when over 300 lines:
  - `CLAUDE.md` - the Build & Test Commands and Test Discipline sections only
- Files allowed to edit (at most 3):
  - `CLAUDE.md`
- Files explicitly out of bounds:
  - all code, all other docs
- Blast-radius discipline: not applicable — docs only.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decisions 4-5 (gate-off wording; wiring reserved for packet 199)
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'not yet a required gate' CLAUDE.md && rg -q 'packet 199' CLAUDE.md && rg -q 'check-literals' CLAUDE.md && rg -q '21_data_defaults_and_fixtures' CLAUDE.md && echo OK` - FACT
- Exit condition: any AC-9 grep failing, or the section adding `check-literals` to required-before-commit commands, fails the step.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | deps + watchlist + 1 test |
| Step 2 | M | visitor core, 5 tests |
| Step 3 | M | token-tree scan, 2 tests |
| Step 4 | M | walker + CLI + live ACs + workspace gates |
| Step 5 | S | docs page + index |
| Step 6 | S | CLAUDE.md section |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch (register TASK-316 per `task-map.md`), never a full backlog read.
- Reconcile reopened/superseded status transitions: none for this packet.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (the two `[FWD]` blind-spot notes in `design.md`).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile (workspace-level gates; `-p xtask` unit runs target the bin's own tests).
