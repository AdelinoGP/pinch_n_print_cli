# Support paint-policy dead-walk reduction

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Can the support paint-policy query's dead geometry walk be eliminated exactly
— without changing any produced path, any paint veto, any off-grid anchor or
any role — and does the candidate pass the standing paired ordinary +
accelerated uninstrumented wall/CPU gate?

[Tree-support renderer self-fuel and scan-fill attribution](64-tree-support-renderer-attribution.md)
measured `PaintRegionLayerView::paint_policy_for`
(`crates/slicer-sdk/src/traits.rs`) at **34.697 B guest instructions, 79.7% of
all tree-support renderer fuel** (43.510 B dispatch, 836 expoly renders, 240
dispatches, both matched classic/arachne supports-on cells: the split is
generator-invariant). Per query on the matched job: one SliceIR region (the
support-glue view is rebuilt from the dispatch's own regions, ~2.4 polygons /
1,883 edges), ~427.7 caller contour vertices tried, ~1,284.7 ring walks,
~893,291 edge tests at ~46.5 metered instructions each — and **cover_hits = 0
of 836, eligibility checks = 0**: the walk never finds a covering region and
`support_eligibility` never runs, so every query returns `DefaultEligible`.
The wall context is attribution-only: renderer accumulated worker time is
6.79–8.35 s over 12-thread phases of 8.2–8.7 s (6.5–8.5% machine occupancy),
0–2 ms on the runs' last-finishing layer, so the freeable share is bounded by
~0.45–0.55 s of phase wall in the probed runs — the paired gate decides the
real number.

## Candidate shapes (exactness argument required per shape)

1. **Annotations-empty fast path.** When the view's SliceIR regions carry no
   support-blocker/support-enforcer annotation values, `support_eligibility`
   can only return `DefaultEligible` for every covering region; returning
   `DefaultEligible` without the geometry walk is value-identical. This
   covers every unpainted (auto-support) job — i.e. every matched cell. The
   emptiness test itself must be cheap (a per-view precheck at construction or
   a per-query scan over the same region set the walk would cover), and must
   not silently change behavior for painted jobs.
2. **Ring bbox prefilter.** A conservative per-ring bounding-box gate before
   `point_in_polygon_ring` removes ring walks that cannot contain the vertex;
   shapes the painted-job path. Exact if the gate can only ever reject
   vertices outside the ring; disclose how the boxes are computed and cached.

The shapes are not mutually exclusive; the ticket's experiment scope decides
which to implement (or documents that neither survives exactness scrutiny).

## Scope and decision evidence

1. Implement the selected shape(s) as a candidate with the standard
   qualification: independent engine/unit tests for the fast path (unpainted
   returns value-identical; painted behavior unchanged), output identity
   against the preserved references for all matched cells in
   **both modes**, and disclosure of any reference needing refresh per the
   [Frozen-job output drift boundary](46-frozen-job-output-drift-boundary.md)
   (the current t58 references are post-t47 and current).
2. Preserve every documented semantic from the t64 take: painted vetoes,
   wall/fill separation, widths/flows, off-grid anchors, roles and output
   order are untouched; no regularization skipped and no planned geometry
   simplified to reduce the walk.
3. The change lives in `slicer-sdk` (guest + native halves); keep the two legs
   behavior-identical (mirror the packet-241-style view-repair discipline) and
   run the repo's narrow parity gates around it.
4. The standing paired ordinary + accelerated uninstrumented wall/CPU gate (6
   retained pairs per cell, starved samples excluded, cpu/wall quoted, outputs
   disclosed); KEEP/DROP returned to the human; candidates never auto-committed.
5. Timing campaigns must not overlap other open tickets' campaigns (t63's
   scheduling-baseline rule): a renderer-cost change does not alter linker or
   scheduling work, but the fresh-baseline rule from
   [Independent low-layer dispatch](63-low-layer-dispatch-granularity.md)
   applies to any campaign started alongside it.

## Acceptance and boundaries

Attribution take t64 graduated this candidate; this ticket is implementation
+ qualification only. No WIT or manifest change is expected (the view change
is internal geometry/gating code in the shared SDK); if a WIT edit becomes
necessary, stop and re-run the tracker's route decision. A failing gate ends
in a documented DROP with candidate retired to a re-appliable patch and
production restored (both freshness gates green), following the t51/t57
restore discipline.

Evidence: [t64 findings](../evidence/t64-tree-support-renderer/FINDINGS.md),
its captures, reducer, verifier and `probe.patch`.