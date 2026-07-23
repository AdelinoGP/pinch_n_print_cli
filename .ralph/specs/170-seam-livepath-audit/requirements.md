# Requirements: 170-seam-livepath-audit

## Packet Metadata

- Grouped task IDs: `TASK-120c` (existing; standalone reopened row `- [~] TASK-120c Restore seam placement on real wall-loop seam candidates` in `docs/07_implementation_status.md` — also referenced by the TASK-120 / TASK-151 / TASK-159 rows; this packet reconciles that reopened row)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft` (depends on packets 178, 179, 180 — all `status: implemented`)
- Aggregate context cost: `S`

## Problem Statement

The fork-gaps handoff item 8 claimed `seam-placer` ignored live seam candidates; grounding for the approved plan (`docs/specs/fork-gaps-wave1-plan.md`, Packet 8) corrected this: `run_wall_postprocess` already prefers `region.seam_candidates()` with `resolved_seam` fallback in the per-mode dispatch (in `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess`, contract comment near the `seam_target` computation). The remaining TASK-120c risk is narrower: when the seam-target wall loop is rotated and re-emitted, sibling wall loops in the same region could be erased unless the full region wall set is re-emitted every time. The current emission loop pushes every wall (`push_reordered_wall_loop` per index, rotation only on the target index), and the wall-preservation invariant is documented in-module — but no regression test pins it for multi-wall regions, multi-region calls, the tolerance-miss pristine path, or the post-180 aligned branch (which now includes `project_onto_wall_segment` continuous projection on top of the legacy vertex-snap path). This packet is a correctness audit: reproduce with fixtures, fix if falsified, and give TASK-120c an explicit disposition.

## In Scope

- New regression test file `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` covering: 3-wall single-region rotation (AC-1), multi-region count/pairing preservation (AC-2), aligned-mode sibling survival (AC-3, using the `select_seam_candidate` path since `seam_candidates` is non-empty), and the tolerance-miss pristine path with no committed seam (AC-N1, in `nearest` mode to isolate from 180's continuous projection). Fixtures built with the existing `slicer_sdk::test_prelude` builders (`PerimeterRegionViewBuilder`, `seam_candidate`) as used by `tests/seam_placer_dispatch_tdd.rs` and the new packet-180 tests (`seam_continuous_projection_tdd.rs`, `seam_degraded_fallback_tdd.rs`).
- A fix in `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess` only if a fixture falsifies the invariant (expected outcome: already correct; the fix step is conditional).
- TASK-120c reconciliation in `docs/07_implementation_status.md`: update the existing reopened `[~]` row — whose reopened-gap text lists the already-fixed candidate-preference gap alongside the sibling-erasure risk this packet audits — to `[x]` closed with the audit finding, or `[ ]` re-scoped with the exact residual defect, referencing packet `170-seam-livepath-audit` (AC-4).

## Out of Scope

- The candidates-vs-`resolved_seam` preference order (already correct; verified against the per-mode dispatch in `run_wall_postprocess`).
- The known planner mesh-corner vs inset-boundary coordinate gap — packet 168 introduced the aligned-aligned compensation, packet 178 superseded 168, and packet 180's continuous projection closed the source-geometry half of `D-168-SEAM-PREPASS-SOURCE` in the deviation log (now `Closed — 2026-07-22`). The nearest/rear/random exact-match tolerance stays as-is here; AC-N1 only pins its graceful degradation in `nearest` mode.
- seam-planner-default, host injection/backfill paths (`crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-runtime/src/layer_executor.rs`), WIT, manifests, config keys.
- Any behavior change for regions with empty wall lists (existing `continue` early in the per-region loop unchanged).
- Packet 180's continuous projection, degraded fallback, or default-mode change — already implemented and tested; this audit piggybacks on its fixture idioms but does not modify the projection logic.

## Authoritative Docs

- `docs/07_implementation_status.md` — large; delegate; only the reopened TASK-120c row plus its referencing TASK-120 / TASK-151 / TASK-159 rows.
- `crates/slicer-sdk/src/builders.rs` — delegate FACT lookups of `begin_region` and `push_reordered_wall_loop` semantics if needed.

## Acceptance Summary

- Positive: `AC-1` through `AC-4`. Refinement: "point-for-point identical" in AC-1/AC-3 means equal `path.points`, equal `feature_flags`, and equal `width_profile.widths` vectors, and preserved closing-repeat convention (`path.is_closed()` unchanged).
- Negative: `AC-N1` (restricted to `nearest` mode to isolate from 180's continuous-projection behavior).
- Cross-packet impact: runs after packets 178/179/180; AC-3 exercises the aligned consumption branch in its post-180 form, so a regression here also guards 180's landing. The audit does not modify any of those packets' code. No other packets touch this crate.

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
- Mirror the per-test structure used by the packet-180 fixtures (`seam_continuous_projection_tdd.rs`, `seam_degraded_fallback_tdd.rs`) to stay consistent with the recent seam-placer test style: same `ir_point` / `ir_wall` / `aligned_region` helper shape, same `expect_err` / `assert_eq!` / `assert!` style.
