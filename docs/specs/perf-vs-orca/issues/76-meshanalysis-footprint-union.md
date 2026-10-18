# MeshAnalysis exact footprint union reduction

Type: task
Label: wayfinder:task
Status: open
Assignee: unclaimed — graduated by the 2026-10-18 base refresh; ticket creation
is not execution approval
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 73

## Question

Can `compute_xy_footprint`'s exact union over the projected facets be made
cheaper while preserving the downstream footprint representation exactly?

The fresh base captures measure `PrePass::MeshAnalysis` at **19.4–22.2 s** on
every base cell, and `host:mesh_analysis`'s only scope is
`polygon_ops::clip_polygons` at **19.2–20.1 s accumulated** — a serial union
(concurrency ≈ 1x) ([findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md)).
Ticket 27 localized it to `compute_xy_footprint`
(`crates/slicer-core/src/algos/mesh_analysis.rs`): the exact Clipper union over
326,546 projected facets, ~75 % of the stage. Its proposed bbox/coverage
replacement is **unsafe as stated** — `region_needs_support`
(`crates/slicer-core/src/algos/mesh_analysis.rs`) uses the footprint through an
exact intersection, and the premise audit verified that consumer. Bbox-only
substitution is out of scope for this ticket.

Deliverables:

1. Re-derive the union's current cost and input shape at HEAD (the t27 probe
   predates the t70 clipper keep; `clipper2-rust` is now vendored in-tree).
2. Investigate only exact, representation-preserving reductions: a spatial
   partition with a deterministic merge order, parallel union of disjoint
   tiles, or an exact input reduction that provably preserves the downstream
   footprint. Any change must survive the 16-cell output-identity matrix against
   the preserved references; a representation-equivalence argument alone is not
   sufficient.
3. If a candidate survives, run the standing paired ordinary + accelerated A/B
   on the pre-declared cells. Keep/drop is the human's.

Constraints: exactness first; no production edit before a measured owner; no
bbox substitution, no approximation; no default switch or auto-commit.
Profile/worker elapsed is not process CPU or uninstrumented wall.

## Acceptance

- A current-head union cost and input census, with the parallel/reduction shape
  stated precisely enough to implement.
- Either a candidate passing the standing gate, or a measured no-safe-reduction
  verdict with the refuted shapes recorded.
- Durable reduced evidence and a checkout-only verifier, per the map's
  evidence policy.
