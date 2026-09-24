# Tree-planner substage attribution

Type: task
Status: open
Blocked by: 12, 24

## Question

Split `com.core.tree-support-planner`'s wall time — the largest measured serial
module (92.9–96.6 s on base, 3.3 s on benchy, per the 2026-09-05 refresh) —
into cache/batched-host work, guest work, and surrounding host
validation/commit.

Prerequisite inside this ticket: add the missing instrumentation to the batched
host services — `offset_polygons_batch`, `clip_polygons_batch`, and
`simplify_polygon_batch` record nothing today (only the two mesh-query batches
push `batch_calls` audit entries). ADR-0049 measured the collision-cache build
at 98.0% of planner runtime (603 independent `offset_polygons` calls) — but
whether that is still the cost **at HEAD is unmeasured**, and the batched
services' own share has never had a timing hook.

Attribution runs only (no wall claims); instrumentation kept greppable and
removed or waived per the probe discipline before any commit.

Deliverable: the substage split with the batched-service share quantified, and
a recommendation for/against an algorithmic candidate — feeding [Gap budget per
cell](12-gap-budget-per-cell.md) for the prepass-dominated cells (base especially).
