# Requirements: 194-check-literals-gate

## Packet Metadata

- Grouped task IDs: `TASK-316` (new row; registered in `docs/07_implementation_status.md` by the implementing swarm — see `task-map.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

## Problem Statement

Adding one field to a widely-constructed struct forces a workspace-wide sweep of exhaustive struct literals in test code. Measured in `docs/specs/struct-literal-churn-gate-plan.md`: commit `a579fc18` (packet 193) touched 165 files, ~90% one-line `overhang_distance_mm: None` filler in test files after `Point3WithWidth` gained a field; `383b633b` swept 26 `LayerCollectionIR` sites; `defb4b19` re-edited every test constructing `SliceRunOptions`. The prior fix (`docs/specs/_OLD/default-builder-migration.md`) added `Default` impls but produced no ongoing rule, so later packets freshly wrote exhaustive literals (re-derived 2026-08-07: 103 test files still construct `Point3WithWidth` literals). Production `src/` literals are deliberately exempt: in `a579fc18` the marshal/producer sites received real logic for the new field — exhaustive literals there are compiler-enforced propagation checkpoints, and FRU there would have silently dropped `overhang_distance_mm` at the WIT boundary.

This packet builds the enforcement tool and its documentation. It does not convert any call site and does not flip enforcement on.

## In Scope

- New `xtask/src/check_literals.rs` module implementing:
  - Watchlist auto-derivation at run time: every `pub` struct with ≥ 5 named fields defined under `crates/*/src/**` (tuple structs excluded; `pub(crate)` and narrower visibilities excluded; no manual ledger). The watchlist scan always covers all of `crates/*/src` regardless of any path filter.
  - Enforced scope: `crates/*/tests/**`, `modules/core-modules/*/tests/**`, `crates/*/benches/**` (whole file each), and `#[cfg(test)]` mod subtrees inside `crates/*/src/**` (cfg-test-only mode). `crates/slicer-wasm-host/test-guests/*/src` is exempt by construction (not under `crates/*/src` or `crates/*/tests`) and documented as intentionally exempt (WIT adapter shims must break loudly on new fields).
  - Violation definition: a struct-literal expression whose path's last segment matches a watched name, with no `..` rest and no waiver. Handles `Self { }` via impl-target tracking, multi-segment paths (e.g. `slicer_ir::PrintEntity`), and literals inside macro token trees (`vec!`, `assert_eq!`, …) via token-stream scanning for a watched `Ident` followed by a brace group lacking a top-level `..`.
  - Waiver: an inline comment `// exhaustive: <reason>` (reason mandatory, non-empty) on the literal's opening line or the line immediately above.
  - CLI: `cargo xtask check-literals [--report] [PATH...]` — enforce mode exits 1 on any violation; `--report` prints the same output and always exits 0; positional `PATH` prefixes restrict which enforced files are scanned (component-aware prefix match). Unknown flag exits 2 with usage.
  - Output: one line per violation (`<ws-relative-path>:<line>: exhaustive literal of watched type \`<Name>\``, forward-slash paths) plus a final summary line `check-literals: <N> violation(s) in <M> file(s) (watchlist: <K> types)`.
- `xtask/src/main.rs` dispatch arm + `USAGE` text for the subcommand; `xtask/Cargo.toml` gains `syn` (features `full`, `visit`) and `proc-macro2` (feature `span-locations`).
- Unit tests for the checker itself, driven by in-memory `.rs` fixture strings (watchlist derivation, each violation class, waiver acceptance/rejection, FRU pass, enum-variant non-firing, `Self` tracking, macro-embedded literals, the documented macro range-expression blind spot).
- New docs page `docs/21_data_defaults_and_fixtures.md` (number re-derived 2026-08-07: highest existing page is `docs/20_support_preview.md`) covering: the rule; production-exemption rationale (cite `a579fc18`); watchlist derivation; waiver format; fixture policy pointing at `slicer_sdk::test_support` (authored by packet 195); `clippy::needless_update` guidance (omit default-equal fields; never spell-all+FRU); known blind spots.
- `.claude/doc-index.md` bullet for the new page.
- `docs/00_project_overview.md`: one row in the "Normative Document Map (LLM/Reviewer Fast Index)" table registering the new page (bare-filename style relative to `docs/`, like the existing `20_support_preview.md` row).
- CLAUDE.md: short MUST section describing the rule and the command, explicitly marked "not yet a required gate — enforcement flips on in packet 199".

## Out of Scope

- Wiring `check-literals` into `cargo xtask test` preflight or CLAUDE.md's required-before-commit command list (packet 199).
- Any call-site conversion of existing exhaustive literals (packets 196–198).
- Adding `Default` impls or fixture bases (packet 195).
- Widening the watchlist to `pub(crate)` structs, macro-generated struct definitions, or `modules/core-modules/*/src` definitions (locked decision; see `design.md` §Open Questions for the recorded `[FWD]`).
- Adding the new page to `docs/00_project_overview.md`'s precedence rule for conflicts — it is a test-code conventions page, not a normative architecture doc; only the Normative Document Map table row is in scope.

## Authoritative Docs

- `docs/specs/struct-literal-churn-gate-plan.md` - short; direct read (locked decisions and measured evidence).
- `CLAUDE.md` - large; read only §Test Discipline and §In-Tree Citation Style while authoring the new section.
- `.claude/doc-index.md` - 27 lines; direct read (bullet list format to follow).

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`.
- Negative: `AC-N1` (waiver reason mandatory), `AC-N2` (macro range blind spot locked), `AC-N3` (unknown-flag usage error).
- Cross-packet impact: the CLI shape (`--report`, positional path filter), waiver format, violation/summary line formats, and watchlist rule are consumed by packets 195–199; changing any of them later invalidates those packets' verification commands.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask check-literals --report > /tmp/cl194.txt 2>&1; ec=$?; tail -1 /tmp/cl194.txt; test $ec -eq 0 && echo OK` | AC-1 report mode exit + summary | FACT pass/fail + 1 summary line |
| `cargo xtask check-literals > /tmp/cl194e.txt 2>&1; echo "exit=$?"; head -3 /tmp/cl194e.txt` | AC-2 enforce exit 1 + line format | FACT pass/fail; SNIPPETS ≤ 5 lines |
| `cargo xtask check-literals --report crates/slicer-ir > /tmp/cl194p.txt 2>&1; grep -E '\.rs:[0-9]+: exhaustive literal' /tmp/cl194p.txt \| grep -vc '^crates/slicer-ir/'` | AC-3 path filter (expect `0`) | FACT single count |
| `cargo xtask check-literals --bogus > /tmp/cl194u.txt 2>&1; echo "exit=$?"` | AC-N3 usage error (expect `exit=2`) | FACT single line |
| `mkdir -p target && cargo test -p xtask 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-4..7, AC-N1, AC-N2 checker unit suite | FACT pass/fail; on failure grep log per CLAUDE.md |
| `rg -qF '// exhaustive:' docs/21_data_defaults_and_fixtures.md && rg -q 'a579fc18' docs/21_data_defaults_and_fixtures.md && rg -q 'needless_update' docs/21_data_defaults_and_fixtures.md && echo OK` | AC-8 docs page anchors | FACT pass/fail |
| `rg -q '21_data_defaults_and_fixtures' .claude/doc-index.md && rg -q '21_data_defaults_and_fixtures' docs/00_project_overview.md && echo OK` | AC-8 doc-index entry + docs/00 map row | FACT pass/fail |
| `rg -q 'not yet a required gate' CLAUDE.md && rg -q 'packet 199' CLAUDE.md && rg -q 'check-literals' CLAUDE.md && echo OK` | AC-9 CLAUDE.md section | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- The pure scanning function (`scan_source`) must be authored and unit-tested before the file-system walker and CLI wiring, so every violation class is provable from in-memory strings without touching the tree.
- The live-run ACs (AC-1..3) are meaningful only after CLI wiring (Step 4); do not attempt them earlier.
- Doc steps (5–6) depend on the final CLI shape and waiver format; if Step 4 changes either, the docs must reflect the shipped shape, not this spec's draft.

## Context Discipline Notes

- `docs/specs/_OLD/default-builder-migration.md` is 1449 lines — this packet does not need it; do not open it (packet 195 owns its §3.6/§5 criteria).
- Never dump the full violation list into context: `check-literals` output on the pre-sweep tree may exceed 200 lines; always pipe through `tail -1`, `head -3`, or `grep -c` as in the matrix above.
- Checker unit tests run via `cargo test -p xtask`; always tee to `target/test-output.log` per CLAUDE.md and read the log instead of re-running.
