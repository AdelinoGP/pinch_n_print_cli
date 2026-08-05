---
status: implemented
packet: 170-seam-livepath-audit
task_ids:
  - TASK-120c
---

# 170-seam-livepath-audit

## Goal

Prove (or fix) that seam rotation in `seam-placer` never erases sibling wall loops — every region entering `run_wall_postprocess` with N wall loops leaves with exactly N — via multi-wall and multi-region regression fixtures, then close or re-scope TASK-120c in `docs/07_implementation_status.md`.

## Problem Statement

The fork-gaps handoff item 8 claimed `seam-placer` ignored live seam candidates; grounding for the approved plan (`docs/specs/fork-gaps-wave1-plan.md`, Packet 8) corrected this: `run_wall_postprocess` already prefers `region.seam_candidates()` with `resolved_seam` fallback in the per-mode dispatch (in `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess`, contract comment near the `seam_target` computation). The remaining TASK-120c risk is narrower: when the seam-target wall loop is rotated and re-emitted, sibling wall loops in the same region could be erased unless the full region wall set is re-emitted every time. The current emission loop pushes every wall (`push_reordered_wall_loop` per index, rotation only on the target index), and the wall-preservation invariant is documented in-module — but no regression test pins it for multi-wall regions, multi-region calls, the tolerance-miss pristine path, or the post-180 aligned branch (which now includes `project_onto_wall_segment` continuous projection on top of the legacy vertex-snap path). This packet is a correctness audit: reproduce with fixtures, fix if falsified, and give TASK-120c an explicit disposition.

## Architecture Constraints

- The wall-preservation invariant is load-bearing downstream: dropping a region's walls propagates through `convert_perimeter_output` (no bucket → no `PerimeterRegion` entry) and corrupts the `(object_id, region_id)` pairing in the per-stage commit path (`layer_executor::apply` in `crates/slicer-runtime/src/layer_executor.rs`, ADR-0020) for multi-region prints. Tests must assert region-level pairing, not just loop counts. The in-module comments document this invariant; the historical "commit_layer_outputs" name is a pre-ADR-0020 legacy reference in the comments and is not a function in the current source — cite `layer_executor::apply` instead.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

(Coordinate-system snippet omitted: fixtures use the module's existing f32-mm point conventions; no mm/unit conversion or Orca constants are involved.)

## Data and Contract Notes

- IR/manifest contracts: none change. Output assertions go through the same builder-view API the existing dispatch tests use.
- WIT boundary: untouched.
- Determinism/scheduler constraints: fixtures pick `nearest` mode (deterministic min-by) and `aligned` (deterministic `aligned_seam_target` selection via its `min_by` over non-empty candidates); the empty-candidate `project_onto_wall_segment` fallback remains covered by packet 180 tests. Avoid `random` mode in count fixtures to keep failures reproducible.
- "Point-for-point identical" comparison must include `feature_flags` and `width_profile.widths` (the parallel arrays `rotate_wall_loop` maintains) so a partial-rotation bug cannot pass on points alone. The `rotate_wall_loop` debug-assert that parallelism holds during rotation is the in-module safety net this audit pins externally.

## Locked Assumptions and Invariants

- Wall-preservation invariant: every region entering `run_wall_postprocess` with N wall loops exits with exactly N, in the `nearest` and `aligned` modes covered by the regression fixtures, on every seam-resolution branch (hit, miss, none). This packet's tests become its permanent guard; the existing dispatch tests cover `rear`, `random`, and `aligned_back` at the single-wall level.
- Packet 180's `aligned` mode continuous projection and the host-injection of `resolved_seam` exist before AC-3 is written; the aligned branch is the post-180 form, not the pre-180 vertex-snap form. AC-3 specifically uses non-empty candidates, so it exercises `aligned_seam_target`; the empty-candidate `project_onto_wall_segment` fallback is outside this fixture.
- The historical `empty-seam graceful-handling fix` / `painted-seam consumer closure` / `painted-seam consumer gap` triad (registered retroactively in `docs/DEVIATION_LOG.md` on 2026-07-23) records the P108→P109 seam-placer correctness arc. The historical claim is that P109 corrected P108's "fatal on empty seam-candidates" carve-out (T-082) to graceful wall preservation; the audit's wall-preservation invariant is the codified form of that correction. `D-109-SEAM-FATAL-CORRECTED` (the pre-rename ID, before the slot was recognised as already taken by `D-109-SELF-CAPTURED-FIXTURES`) is the citation carried by `docs/05_module_sdk.md` and the in-module comment; the canonical log row is `empty-seam graceful-handling fix` to match the `the earlier beading sub-packet family` sub-row convention.

## Risks and Tradeoffs

- Expected-green audit: all fixtures may pass immediately, making the tests look vacuous. Mitigation: each test must be demonstrated RED-capable once by temporarily inverting its assertion locally (not committed) or by construction review in the exit condition; the packet report states which outcome occurred.
- AC-3 couples this packet to the post-180 aligned semantics; if a future packet changes the aligned target behavior, AC-3's fixture (0.3 mm offset with a non-empty `seam_candidates` list, exercising `aligned_seam_target`) is the one to re-derive. The fixture was chosen so the `aligned_seam_target` path (rather than the `project_onto_wall_segment` path) is exercised, decoupling the audit from any future projection-behavior change.
