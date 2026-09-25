# Arachne graph-construction attribution

Type: task
Status: open
Blocked by: 12, 21

## Question

Re-attribute Arachne's hot path at HEAD and propose at most one optimization
candidate for it.

Targets: `preprocess_input_outline` / `run_nine_stage_pipeline`
(`crates/slicer-core/src/arachne/preprocess.rs`) and
`SkeletalTrapezoidationGraph::from_polygons`
(`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`) — the 2026-09-07
study ([Perimeter attribution study](06-perimeter-attribution-study.md))
attributed 75.3% of the enclosing interval to preprocess/graph construction,
but those shares predate the committed
[Wall-flags annotation-free fast path](07-wall-flags-annotation-fast-path.md)
and a measured 364.932 ms remainder was never explained.

The arachne cells of the matrix count toward the destination, so this is route
work; [Gap budget per cell](12-gap-budget-per-cell.md) sets the required factor.

Constraints: preserve canonical stage ordering, topology, ties, and graph
invariants (the `getNextUnconnected` / `BeadingPropagation` semantics depend on
faithful `next`/`prev`/`twin` structure); attribution first, pass removal or
indexing changes only with measured justification.

If a candidate emerges it joins the timing chain and returns a keep/drop
recommendation to the human. Timing chain position 7.
