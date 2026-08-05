---
status: implemented
packet: 121_support-planner-smooth-nodes
task_ids: [TASK-286]
---

# 121_support-planner-smooth-nodes

## Goal

Port OrcaSlicer's `TreeSupport::smooth_nodes` (100-iteration three-point Laplacian smoothing on branch chains) to `support-planner` so per-layer branch-column positions are smoothed into continuous curves rather than the raw stairstep positions from the per-layer `clamp_to_avoidance` snap. Add a `branch_curvature_below_threshold` invariant to the wedge harness gating the smoothing against regression.

## Problem Statement

The planner's per-layer clamping produced jagged stairstep branches. Orca runs 100 iterations of three-point Laplacian smoothing on each branch chain (`TreeSupport.cpp` `smooth_nodes`); the source-plan C3 task was open and the source-plan `TASK-262` collided with unrelated ledger work (now LightningTreeGen) — renumbered to `TASK-286`.

## Architecture Constraints

- Data shape: `SupportPlanEntry.branch_segments: Vec<ExtrusionPath3D>` where each `ExtrusionPath3D` is typically a 2-point MST-edge segment. The "chain" spans layers: `group_branches_into_columns` groups by `(object_id, region_id)` ordered by `global_layer_index` descending; the smoother operates on the per-layer (x, y) sequence of each column.
- Only per-layer (x, y) and width are smoothed; z, role, speed_factor are NOT. Endpoints (highest z tip and lowest z root) held fixed.
- Width (radius) clamped to `[0.0, MAX_BRANCH_RADIUS_MM = 6.0]` after each iteration.

## Data and Contract Notes

- Delivered shape (differs from source-plan C3 wording): `support-planner::smooth_branches(entries: &mut Vec<SupportPlanEntry>, iterations: usize)` at the tail of `plan_for_object` (between the propagation loop and the final `entries_in_order` emit); 5 mm sub-chain boundaries at inter-tree gaps (mirrors the golden-reconstruction break).
- `need_extra_wall` flag interactions NOT ported (future work).
- Invariant 8 (wedge harness): `branch_curvature_below_threshold` — no consecutive (x,y) segment pair across all `branch_segments[*].points[*]` exceeds 30° turn angle (empirical: loose enough for legitimately-curved smoothed branches, tight enough to catch unsmoothed stairsteps).
- Golden re-capture via `SUPPORT_WEDGE_REGEN_GOLDEN=1`; commit documents the algorithmic shift; reviewers verify "smoother, not warped".
- Task renumber: source-plan `TASK-262` → `TASK-286`.

## Locked Assumptions and Invariants

- Columns with < 3 points are no-ops (AC-N1); empty entries no-op without panicking (AC-N2).
- Branch count and connectivity are preserved; the existing 7 wedge invariants stay green after smoothing lands.
- Does NOT touch tree-support/traditional-support — they consume smoothed `SupportPlanIR` via the existing `support_plan_segments_for` path.

## Risks and Tradeoffs

- Golden shift is expected and reviewed; the curvature invariant is the regression gate.
- No IR/WIT/manifest/configurable-iteration contract introduced — planner behavior only.

## Implementation Deviations (recorded at close)

None. Doc Impact: `docs/specs/support-modules-orca-port.md` §Validation Strategy invariant list extension only (source-plan §C3's `smooth_chains/PlannedSupportNode` wording reconciled in the 2026-08-05 doc audit).
