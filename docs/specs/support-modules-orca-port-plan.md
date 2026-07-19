# Support Modules Orca Port Packet Plan

The approved source plan is maintained verbatim at
`docs/specs/support-modules-orca-port.md`. This file is the Batch Protocol
anchor and queue for packets 116 through 124; the source plan remains the
authority for scope, decisions, validation, and OrcaSlicer references.

## Packet Queue

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 116 | support-modules-doc-honesty-cleanup | Correct support-module documentation and dead config state while documenting the existing speed-normalization convention. | Unmapped source B1-B3 (`TASK-250`, `TASK-251`, `TASK-252` collide or lack canonical support rows) | P95 | generated | `.ralph/specs/116_support-modules-doc-honesty-cleanup/` |
| 117 | support-planner-geometric-correctness | Fix the support planner's tip-cone radius and replace its DIY polygon inflation through the guest-compatible SDK geometry seam. | Unmapped source B5-B6 (`TASK-254`, `TASK-255` collide with unrelated current rows) | 116 | generated | `.ralph/specs/117_support-planner-geometric-correctness/` |
| 118 | support-planner-typed-diagnostics | Add the typed prepass diagnostic channel and migrate all three support-planner warning paths. | TASK-163b-diagnostic; source B4 `TASK-253` excluded because its live row owns paint segmentation | 116 (shared-file edit order only) | generated | `.ralph/specs/118_support-planner-typed-diagnostics/` |
| 119 | support-validation-wedge-harness | Establish the current-contract wedge invariants and self-capture golden regression that gate Block C. | Unmapped source C1 (`TASK-260` collides with unrelated current rows) | 116, 117, 118 | generated | `.ralph/specs/119_support-validation-wedge-harness/` |
| 120 | support-modules-paint-segment-annotations-migration | Restore support paint enforcer/blocker behavior on post-P95 segment annotations. | TASK-261 | P95/P96/P97, 119 | pending | `.ralph/specs/120_support-modules-paint-segment-annotations-migration/` |
| 121 | support-planner-smooth-nodes | Add endpoint-fixed Laplacian branch smoothing and its curvature invariant. | TASK-262 | 117, 119, 120 | pending | `.ralph/specs/121_support-planner-smooth-nodes/` |
| 122 | support-planner-multi-neighbour-mst | Replace single-neighbour propagation with all-neighbour target synthesis and symmetry coverage. | TASK-263 | 117, 119, 120, 121 | pending | `.ralph/specs/122_support-planner-multi-neighbour-mst/` |
| 123 | support-planner-to-buildplate-pruning | Track build-plate reachability and reject unsupported model-resting branches when configured. | TASK-264 | 122 | pending | `.ralph/specs/123_support-planner-to-buildplate-pruning/` |
| 124 | support-plan-raft-plan-and-raftinfill-role | Add the object-level raft plan seam, raft infill role/claim, and C7 contract decision. | TASK-265, TASK-266 | 116, 119, 120 | pending | `.ralph/specs/124_support-plan-raft-plan-and-raftinfill-role/` |
