# Overhang-quartile query: predicate narrowing + standing paired A/B

Type: task
Status: open
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 25

## Question

Does narrowing the `overhang_quartile` query's classification predicate so its
`eps = 0.0` boundary-tolerance pre-pass is skipped on the strict-containment
path — or, as the alternative shape, adding an inclusive per-polygon bbox
prefilter in the query so non-containing band polygons never reach the O(N)
scan — preserve the output exactly and improve median uninstrumented wall with
corroborating process CPU, per the map's standing paired ordinary + accelerated
A/B — or does it fail the fairness/keep gates?

Graduated from [Arachne graph-construction
attribution](25-arachne-graph-attribution.md)'s measured split (2026-10-01):
the Arachne module's fuel is 86.4% two guest per-vertex queries, of which
`overhang_quartile` is 38.1% of module fuel at ~1.64 ms/call ordinary.
Its classification goes through
`point_in_polygon_winding(polygon, x, y, 0.0)`
(`crates/slicer-ir/src/polygon_predicate.rs`), whose boundary-tolerance
pre-pass is **measured at 56.5-71.2% of the predicate's own cost** across four
preserved synthetic-ring runs
(`evidence/t25-arachne-attribution/probe_outputs/winding_pass_share.run{1..4}.txt`)
with identical verdicts against a winding-only replica
(`t25_probe_winding_pass_share_tdd`), while the predicate's own doc-comment
defines `0.0` as the strict-containment contract. The exact-on-edge verdict is
the open question: the pre-pass admits a point exactly on an edge, and a
straight skip is probably — not provably — value-preserving.

Fuel ceiling: the synthetic pre-pass share × the query's module share ≈
**21-27% of module fuel** — a bound derived from a cross-domain share, not a
measured fuel saving. Wall transfer unmeasured; the map's measured fuel→wall
transfer is 0-16%.

## Work (only after explicit human authorization of this take)

- Choose the shape: narrow `point_in_contour_winding` /
  `point_in_polygon_winding` (`crates/slicer-ir/src/polygon_predicate.rs`) so
  an `eps = 0.0` call skips the boundary pass outright — which needs an
  explicit decision on exactly-on-edge points, since the strict path currently
  admits them via `dist2 <= 0` — versus an inclusive per-polygon bbox
  prefilter inside `PerimeterSpatialContext::overhang_quartile` /
  `indexed_overhang_quartile` (`crates/slicer-core/src/perimeter_spatial.rs`),
  which is value-preserving by construction but whose saving is unmeasured
  (the y-interval lookups may already reject most candidates).
- Prove output equivalence first: the frozen supports-off Benchy Arachne
  reference (`7049a06d…`) is byte-identical for the ordinary and accelerated
  cells this take wants to claim; add a regression test that fails if the
  narrowed predicate ever disagrees with the full one on any in/out verdict.
- Then the standing measurement: paired ordinary + accelerated, interleaved
  repeats with starvation exclusion, uninstrumented probe-free runs, isolated
  host/guest snapshots, output disclosure per the fairness contract. Fuel
  alone is not a wall win — gate before keeping.
- Keep/drop recommendation returns to the human; no auto-commit.

## Not authorized

Any implementation, benchmark, or production change until the human explicitly
authorizes this take; the ticket 25 study authorized attribution only.

## Evidence

[Arachne re-attribution findings](../evidence/t25-arachne-attribution/FINDINGS.md),
probe preserved at `evidence/t25-arachne-attribution/probe.patch` +
`evidence/t25-arachne-attribution/*.rs.txt`, reduced captures at
`evidence/t25-arachne-attribution/captures/`, re-derivation in
`evidence/t25-arachne-attribution/verify-t25.py`.
