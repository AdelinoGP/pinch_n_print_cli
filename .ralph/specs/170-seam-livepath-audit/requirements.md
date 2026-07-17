# Requirements: 170-seam-livepath-audit

## Packet Metadata

- Grouped task IDs: `TASK-120c` (existing; standalone reopened row `- [~] TASK-120c` at `docs/07_implementation_status.md:92`, also referenced by the TASK-120/TASK-151/TASK-159 rows at lines 87/99/100 — this packet reconciles that reopened row)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft` (depends on packet `168-seam-aligned-modes`)
- Aggregate context cost: `S`

## Problem Statement

The fork-gaps handoff item 8 claimed `seam-placer` ignored live seam candidates; grounding for the approved plan (`docs/specs/fork-gaps-wave1-plan.md`, Packet 8) corrected this: `run_wall_postprocess` already prefers `region.seam_candidates()` with `resolved_seam` fallback (`modules/core-modules/seam-placer/src/lib.rs:242-252`, contract comment at `lib.rs:208-218`). The remaining TASK-120c risk is narrower: when the seam-target wall loop is rotated and re-emitted, sibling wall loops in the same region could be erased unless the full region wall set is re-emitted every time. The current loop at `lib.rs:260-275` appears to emit every wall (`push_reordered_wall_loop` per index, rotation only on the target index), and the HIGH-2 wall-preservation invariant is documented in-module — but no regression test pins it for multi-wall regions, multi-region calls, the tolerance-miss pristine path, or packet 168's new aligned snap branch. This packet is a correctness audit: reproduce with fixtures, fix if falsified, and give TASK-120c an explicit disposition.

## In Scope

- New regression test file `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` covering: 3-wall single-region rotation (AC-1), multi-region count/pairing preservation (AC-2), aligned-mode snap branch sibling survival (AC-3), and the tolerance-miss pristine path with no committed seam (AC-N1). Fixtures built with the existing `slicer_sdk::test_prelude` builders (`PerimeterRegionViewBuilder`, `seam_candidate`) as used by `tests/seam_placer_dispatch_tdd.rs`.
- A fix in `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess` only if a fixture falsifies the invariant (expected outcome: already correct; the fix step is conditional).
- TASK-120c reconciliation in `docs/07_implementation_status.md`: update the existing reopened `[~]` row at line 92 — whose reopened-gap text lists the already-fixed candidate-preference gap alongside the sibling-erasure risk this packet audits — to `[x]` closed with the audit finding, or `[ ]` re-scoped with the exact residual defect, referencing packet `170-seam-livepath-audit` (AC-4).

## Out of Scope

- The candidates-vs-`resolved_seam` preference order (already correct; verified).
- The known planner mesh-corner vs inset-boundary coordinate gap itself (`lib.rs:210-214`) — packet 168 addresses it for aligned modes; the nearest/rear/random exact-match tolerance stays as-is here (AC-N1 only pins its graceful degradation).
- seam-planner-default, host injection/backfill paths (`crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-runtime/src/layer_executor.rs`), WIT, manifests, config keys.
- Any behavior change for regions with empty wall lists (existing `continue` at `lib.rs:222-224` unchanged).

## Authoritative Docs

- `docs/07_implementation_status.md` — large; delegate; only the reopened TASK-120c row (line 92) plus its referencing rows (lines 87/99/100).
- `crates/slicer-sdk/src/builders.rs` — delegate FACT lookups of `begin_region` (`builders.rs:266`) / `push_reordered_wall_loop` (`builders.rs:337`) semantics if needed.

## Acceptance Summary

- Positive: `AC-1` through `AC-4`. Refinement: "point-for-point identical" in AC-1/AC-3 means equal `path.points`, equal `feature_flags`, and equal `width_profile.widths` vectors, and preserved closing-repeat convention (`path.is_closed()` unchanged).
- Negative: `AC-N1`.
- Cross-packet impact: runs after packet 168 in the same module; AC-3 exercises 168's snap branch, so a regression here also guards 168's landing. No other packets touch this crate.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p seam-placer --test seam_sibling_walls_tdd 2>&1 \| grep '^test result'` | AC-1/2/3/N1 suite | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-placer 2>&1 \| grep '^test result'` | whole-module regression | FACT pass/fail |
| AC-4 grep (see `packet.spec.md`) | docs/07 disposition row | FACT PASS/absent |
| `cargo xtask build-guests --check` | only if `src/lib.rs` was edited (conditional fix step) | FACT clean/STALE |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- The fixture step must land RED-capable assertions before any fix: if all fixtures pass immediately, the packet records "invariant verified, no fix needed" and proceeds to the docs/07 disposition — the conditional fix step is skipped, not silently absorbed.
- If `src/lib.rs` is edited, the guest rebuild must happen before re-running any host-integration suite.

## Context Discipline Notes

- `docs/07_implementation_status.md` is never read in full — the disposition edit goes through a worker dispatch with exact anchor text.
- Reuse `tests/seam_placer_dispatch_tdd.rs` builder idioms by reading that file once; do not explore `crates/slicer-sdk` broadly for builder internals — delegate a FACT if a builder signature is unclear.
