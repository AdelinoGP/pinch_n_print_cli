# Requirements: support-planner-multi-neighbour-mst

## Packet Metadata

- Grouped task IDs:
  - `TASK-287` (renumbered from source-plan `TASK-263`; `TASK-263` is now used by `docs/07_implementation_status.md` for Lightning DistanceField per packet 138).
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support-planner`'s propagation block at `modules/core-modules/support-planner/src/lib.rs:669-704` (current line numbers have drifted; symbol names are stable identifiers) uses a single `nearest_neighbour` lookup per node — each surviving node moves toward exactly one MST neighbour (the lowest-distance one). For nodes with ≥ 3 MST neighbours, the result is asymmetric: a "fan" of three branches converging on one node produces a chain that veers toward whichever fan-arm happens to have the smallest edge weight, ignoring the other two. The output is visibly skewed where Orca's `drop_nodes` produces a centered merge.

This packet replaces the single-neighbour lookup with multi-neighbour aggregation matching OrcaSlicer's pattern: the move target is the reciprocal-distance-squared (1/d²) weighted aggregate of ALL MST neighbours of the node. Adds a symmetry invariant to the wedge harness.

## In Scope

- Replace the `nearest_neighbour` + `nearest_distance` lookup blocks in `modules/core-modules/support-planner/src/lib.rs:671-682` (current line numbers have drifted) with a per-node aggregate computation.
- For each node `i`: collect all MST edges incident on `i`; let `D_j` be the distance to neighbour `j`; the move target is `sum(neighbour_j_position / (D_j * D_j)) / sum(1 / (D_j * D_j))` (reciprocal-distance-squared, 1/d² — matches Orca's `TreeSupport::drop_nodes` non-`is_strong` path).
- Apply the existing `max_move_xy` cap to the displacement (line 695-704 of the prior version, current line numbers have drifted) AFTER the aggregate is computed; apply the existing `clamp_to_avoidance` post-cap (line 727 of the prior version, current line numbers have drifted).
- Add the wedge harness invariant `merge_geometry_symmetric_for_n_branches` (the 9th invariant; the 8th is the curvature invariant from packet 121).
- Add `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` with AC-2, AC-3, AC-N1, AC-N2 unit tests.
- Regenerate the wedge goldens via `SUPPORT_WEDGE_REGEN_GOLDEN=1` and `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance`.
- Update `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list to add `merge_geometry_symmetric_for_n_branches`.

## Out of Scope

- Replacing Prim with a heap-based MST.
- Removing the `max_branches_per_layer` cap.
- Re-doing the merge-detection logic (which nodes are dropped). The merging rule (drop higher-index endpoint of edges shorter than `merge_distance`) is preserved.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C4 — directly.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::TreeSupport::drop_nodes` (~line 2625) — confirm aggregation formula and weighting scheme.

## Acceptance Summary

- Positive cases: AC-1 through AC-7.
- Negative cases: AC-N1 (single-neighbour degenerate), AC-N2 (zero distance — no panic).
- Cross-packet impact: goldens re-anchored; invariant list extended.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | WASM gate. | FACT pass/fail |
| `cargo test -p support-planner --test multi_neighbour_mst_tdd 2>&1 \| tee target/test-output.log` | AC-2, AC-3, AC-N1, AC-N2. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_invariants_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-4, AC-5, AC-7. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance 2>&1 \| tee target/test-output.log` | AC-6 re-anchor. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd 2>&1 \| tee target/test-output.log` | AC-6 (no-env tolerance). | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate. | FACT pass/fail |

## Step Completion Expectations

- The `max_move_xy` cap and `clamp_to_avoidance` snap continue to apply AFTER the multi-neighbour aggregate is computed (lines 695-707). The new aggregate replaces the *direction* of the move, not the cap or the avoidance enforcement.
- Goldens are regenerated AFTER multi-neighbour lands. Pre-regenerated goldens against single-neighbour would fail the tolerance gate.
- Symmetric merge detection: a "merge point" is a node with ≥ 3 MST edges. The wedge invariant counts such nodes and asserts each merge's endpoints are approximately equidistant. The 30% threshold (stddev / mean) is empirical — picked by Step 4 against the post-multi-neighbour planner output.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `modules/core-modules/support-planner/src/lib.rs` — range-read the propagation block (lines 669-704). 1590 lines total.
  - OrcaSlicer `TreeSupport.cpp::drop_nodes` — delegate SUMMARY only.
- Likely temptation reads (skip these):
  - OrcaSlicer's `MinimumSpanningTree.cpp` — out of scope (we keep Prim).
- Sub-agent return-format hints: SUMMARY ≤ 200 words for Orca; FACT pass/fail for cargo runs.
