---
status: implemented
packet: 122_support-planner-multi-neighbour-mst
task_ids: [TASK-287]
---

# 122_support-planner-multi-neighbour-mst

## Goal

Replace `support-planner`'s single-neighbour MST propagation (the `nearest_neighbour` / `nearest_distance` lookup that picks exactly one MST neighbour per node as the move target) with multi-neighbour target synthesis matching OrcaSlicer's `TreeSupport::drop_nodes`: each node's move target is the reciprocal-distance-squared (1/d²) weighted aggregate of ALL its MST neighbours. Add a `merge_geometry_symmetric_for_n_branches` wedge invariant asserting merge points (nodes with ≥ 3 incoming MST edges) are approximately equidistant from contributing branches.

## Problem Statement

Single-neighbour propagation produced asymmetric branches for nodes with ≥ 3 MST neighbours. The source-plan C4 task's weighting formula was unresolved ("optionally — confirm the Orca formula"); implementation resolved it as normalized reciprocal-distance-squared (1/d²) with a degenerate zero-distance short-circuit. Source-plan `TASK-263` collided with unrelated ledger work (now Lightning DistanceField) — renumbered to `TASK-287`.

## Architecture Constraints

- Replacement is local to `modules/core-modules/support-planner/src/lib.rs`: the per-neighbour lookup (originally lines 671-682) and move-target synthesis (688-704) are replaced by an `aggregate_neighbour_targets` all-neighbours scan. `mst_edges: Vec<(a_idx, b_idx, distance)>` source data unchanged.
- Preserved: `max_move_xy` cap and `clamp_to_avoidance` enforcement — only the move DIRECTION changes.
- Degenerate `D_j < 1e-6 mm` collapses to that neighbour's position (dominant weight saturates; no division by zero).
- No IR, manifest, or WIT change; branch connectivity may change → goldens re-anchored.

## Data and Contract Notes

- Weights: `1.0 / (distance_j * distance_j)`, normalized to sum 1 (Orca `drop_nodes` non-`is_strong` path).
- Invariant 9 (wedge harness): `merge_geometry_symmetric_for_n_branches` — for every merge point (node with ≥ 3 incoming MST edges, equivalently a `branch_segments` endpoint shared by ≥ 3 segments), the standard deviation of distances from the merge point to its contributing endpoint XYs is ≤ 30% of the mean distance. Empirical threshold: loose enough for asymmetric smoothed branches, tight enough to catch old single-neighbour asymmetry.
- Degenerate single-neighbour case: aggregate over 1 element is that element — matches old behavior (AC-N1).
- Task renumber: source-plan `TASK-263` → `TASK-287`.

## Locked Assumptions and Invariants

- All prior wedge invariants (7 from packet 119 + curvature from packet 121) stay green.
- The packet does NOT touch tree-support/traditional-support.
- Golden re-anchor via `SUPPORT_WEDGE_REGEN_GOLDEN=1`; count drift ≤ 10%, endpoint Hausdorff ≤ 0.5 mm.

## Risks and Tradeoffs

- Merge-point changes alter branch connectivity — the symmetry invariant and golden review are the gates.
- The 1/d² formula is documented as the resolved choice in `docs/specs/support-modules-orca-port.md` §C4 (2026-08-05 doc audit removed the "optionally — confirm the Orca formula" wording).

## Implementation Deviations (recorded at close)

None. Doc Impact: `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list extension only.
