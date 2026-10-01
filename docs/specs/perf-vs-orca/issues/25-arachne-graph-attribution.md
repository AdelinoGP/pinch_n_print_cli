# Arachne graph-construction attribution

Type: task
Status: resolved
Assignee: current OpenCode session (wayfinder), 2026-10-01
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

## Answer — the module's cost is two guest per-vertex queries, not the host pipeline

Measured 2026-10-01 on the frozen supports-off Benchy Arachne job (ordinary and
accelerated, every output byte-identical to the frozen reference `7049a06d…`).
Full evidence: [FINDINGS.md](../evidence/t25-arachne-attribution/FINDINGS.md);
probe preserved in `probe.patch` + the `*.rs.txt` copies; re-derivation in
`verify-t25.py` (exit 0).

- **The boundary the old study could not see.** Ticket 06 measured the
  *enclosing host pipeline interval* (75.3% preprocess+graph there) — that
  interval is only **~18% of the module's own elapsed** at HEAD. The rest is
  guest work the old probe never covered.
- **Where the module's fuel goes.** `PerimeterSpatialContext`'s two per-vertex
  spatial queries are **86.4% of module fuel**: `signed_distance_to_boundary`
  48.3% (112,606 calls) and `overhang_quartile` 38.1% (3,289 calls);
  `build_walls` plumbing 5.3%, the precise-outer-wall `offset` 4.1%, the region
  loop's own glue 3.8%, everything else ≤0.2%. The host service call is 0.04%
  of module fuel and 20.6% of module wall.
- **Not a tail.** 239/239 layers call both queries; the top 12 of 239 layers
  carry 17.3% of guest fuel. This is an every-layer, per-vertex cost.
- **Inside the host service** (the ticket's named targets, re-measured): the
  old study's two tied leads keep their shape — preprocess 43.3% (stage 1
  triple offset alone 41.6%, split evenly across `offset2_ex`'s two clipper
  passes and the follow-up `offset`) and graph construction 38.0% (boostvoronoi
  `Builder::build` 33.5%). Named stages close to 99.2% of the pipeline, so the
  364.932 ms remainder is answered. `connect_junctions` 5.8% and
  `reorder_by_region_order` 3.0% are the next two.
- **Modes.** Acceleration cuts module fuel 1.73x, the distance query 1.93x and
  the quartile query 1.71x, and the two queries still dominate at 82.0%
  accelerated — the ticket-18 shape re-measured on Arachne. The host split is
  mode-invariant (service wall ratio 1.13).

**One candidate: the `overhang_quartile` query (38.1% of module fuel,
~1.64 ms/call ordinary).** Its classification goes through
`point_in_polygon_winding(..., eps = 0.0)`, whose boundary-tolerance pre-pass is
**measured at 62.8-66.3% of the predicate's own cost** with identical verdicts
to a winding-only replica, while `0.0` is the documented strict-containment
contract. Fuel ceiling ≈ 24-25% of module fuel; wall unmeasured (the map's
measured fuel→wall transfer is 0-16%). Two value-preserving shapes (narrow the
predicate for `eps = 0.0`, or an inclusive bbox prefilter in the query) are
design work for the candidate's own take. No implementation, authorization,
A/B, keep/drop or commit is claimed here; the candidate joins the timing chain
awaiting its own scope plus the standing paired ordinary + accelerated A/B —
graduated as [Overhang-quartile query: predicate narrowing + standing paired
A/B](45-overhang-quartile-predicate-narrowing.md).
