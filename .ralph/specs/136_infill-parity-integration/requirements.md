# Requirements: 136_infill-parity-integration

## Packet Metadata

- Grouped task IDs:
  - `TASK-261`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packets 129–135 each proved their slice in isolation; nothing yet proves the composition —
that a modifier's density reaches a rewritten module through the per-region accessor, that
the linker's two branches behave on real pipeline output, and that the user-visible result
matches the OrcaSlicer reference behavior that motivated the roadmap. Meanwhile the golden
baseline has been carved since packet 131 (D6 decision: carve early, bless once) — the
longer it stays carved, the weaker the repo's regression signal. This packet closes both
gaps: end-to-end proof and a single justified re-bless.

## In Scope

- M3 e2e fixture: a cube with a centered infill-modifier volume (base 0.15 / modifier 0.40).
  Preference: extend `resources/cube_cilindrical_modifier.3mf` with the density delta in its
  modifier metadata if the loader's delta path supports it; otherwise author a new
  `resources/cube_infill_modifier.3mf` offline (packet-89/90 fixture precedent). `[FWD]` —
  decided at Step 1.
- E2e tests: AC-1 (one wall set + two spacings), AC-2 (containment + shared-arc anchoring +
  linkage), AC-3 (wedge linked-infill + `--report` artifact), AC-N1 (no-linker degraded
  guard).
- `infill_overlap` CLI binding (pattern: `crates/slicer-ir/tests/fill_holder_cli_binding_tdd.rs`)
  + its test.
- Golden restore: remove every `carved: infill-parity D6` `#[ignore]`; re-bless each restored
  expectation against verified output with a per-fixture closure-log justification (bless is
  gated on AC-2/AC-3 passing first — geometry before SHAs).
- Workspace acceptance ceremony via `cargo xtask test --workspace --summary` (permitted: the
  packet-close case), dispatched to a sub-agent with a FACT return.
- docs/07 closure sweep for TASK-254…TASK-261.

## Out of Scope

- Any algorithm change in any module or the linker — if integration exposes a defect, this
  packet records it and (if small) fixes it as a documented deviation; structural defects
  spawn a follow-up packet rather than silently expanding here.
- Lightning fixtures/blessing (137–140 own their sub-roadmap and its bless).
- Gyroid multi-role e2e (opt-in path) — include only if the M3 fixture extension makes it
  ≤ S extra cost; otherwise note as follow-up (spec M3 "if cheap" clause).

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 5 — load Phase 5 only.
- `docs/specs/modifier-region-infill.md` §Phase M3 — full (short).
- `docs/16_slicer_report.md` — delegate if needed for AC-3's artifact assert.
- `CLAUDE.md` §Test Discipline — the `cargo xtask test --workspace` dispatch contract.

## Acceptance Summary

- Positive cases: `AC-1`–`AC-5` in `packet.spec.md`. Refinements: AC-1's spacing-ratio
  tolerance (10%) absorbs solid-spacing adjustment and clipping effects; AC-2's
  0.5 × spacing reach threshold is the same operationalization packet 133 used (AC-7 there) —
  deliberately identical so the module-level and e2e-level asserts agree; AC-5's zero-marker
  grep makes restoration mechanically total.
- Negative cases: `AC-N1` — degraded-not-failed without the linker (ADR-0025's explicit
  trade-off, pinned at the integration level).
- Cross-packet impact: closes the D6 carve window; flips TASK-254…261; unblocks 137.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test e2e -- modifier_infill 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-1 + AC-2 | FACT + counts |
| `cargo test -p slicer-runtime --test e2e -- wedge_linked_infill_report 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-3 | FACT |
| `cargo test -p slicer-ir -- infill_overlap_cli_binding 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-4 | FACT |
| `cargo test -p slicer-runtime --test integration -- no_linker_module_degraded_raw_output 2>&1 \| tee target/test-output.log \| grep "^test result"` | AC-N1 | FACT |
| `rg -c 'carved: infill-parity D6' --glob '*.rs' \| wc -l` | AC-5 (expect 0) | FACT count |
| `cargo xtask test --workspace --summary` (sub-agent) | acceptance ceremony | FACT PASS/FAIL + failing names only |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT |
| `cargo xtask build-guests --check` | freshness before the ceremony | FACT |

## Step Completion Expectations

- Ordering: geometry ACs (1/2/3) MUST be green before any golden re-bless (Step 4) — a SHA
  blessed before geometry verification is exactly the anti-pattern the D6 gate exists to
  prevent.
- The workspace ceremony runs LAST, after restore, and its full output is never absorbed into
  the implementer's context (summary dispatch only; drill-down via Grep on
  `target/test-output.log`).

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated: all e2e test files
  (open only the tests being added/restored); `docs/16_slicer_report.md` (delegate);
  NEVER stream the workspace test output (the `--summary` contract exists for this).
- Likely temptation reads: the HTML report body — do not read it; AC-3 asserts existence +
  IR-level linkage, and the visual check is a human/closure-log step.
- Sub-agent return-format hints: the ceremony dispatch returns the `--summary` verdict block
  only; restore-bless dispatches return FACT per fixture (old SHA → new SHA + one-line
  justification).
