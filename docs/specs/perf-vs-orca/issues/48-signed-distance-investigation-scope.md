# Signed-distance investigation scope

Type: grilling
Label: wayfinder:grilling
Status: resolved
Assignee: agpen (OpenCode wayfinder scope decision), 2026-10-05
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 18, 25, 45

## Question

What bounded investigation should turn the remaining
`signed_distance_to_boundary` cost into a concrete, output-preserving candidate,
without silently reopening the parked query optimizations or authorizing a
timing/adoption campaign?

Recommendation: fresh attribution first on the supports-off Benchy Arachne job,
ordinary and accelerated, separating context construction, inside/outside
classification, and nearest-edge distance. Decide whether a candidate deserves
its own exactness and standing paired-mode speed gate only after that split.
Do not infer ordinary-mode internals from accelerated historical evidence.

## Comments

### Claim and static scope inspection — 2026-10-05

Claimed for scoping after the human requested continuation. The session left
[offset2_ex call reduction](19-offset2-ex-call-reduction.md) parked and released
its route-only claim. The continuation was not treated as permission to edit
production code, add profiling probes, build artifacts, run slices, retry
adoption, switch defaults, or commit.

Verified code facts:

- `PerimeterSpatialContext::signed_distance_to_boundary`
  (`crates/slicer-core/src/perimeter_spatial.rs`) dispatches ordinary mode to
  the independent full-scan helper `signed_distance_to_boundary`
  (`crates/slicer-core/src/perimeter_utils.rs`). The indexed path computes
  distance through `indexed_distance` and sign through `indexed_inside`,
  retaining observable legacy fallback paths.
- `indexed_inside` (`crates/slicer-core/src/perimeter_spatial.rs`) collects
  polygon indices from Y-only edge envelopes into a source-ordered `BTreeSet`.
  It evaluates every candidate even after finding an inside polygon. The
  full-scan helper's sign calculation uses `.any()` instead. A first-hit exit
  is a possible lead, not an implemented or measured candidate; any change
  must account for fallback and diagnostic behavior as well as returned sign.
- `point_in_polygon_winding` / `point_in_contour_winding`
  (`crates/slicer-ir/src/polygon_predicate.rs`) classify the outer contour
  without subtracting holes and still run the boundary check at zero epsilon.
  Distance evaluation includes hole edges. Preserve those existing semantics,
  signed zero, nearest-edge source-order ties, and both coordinate-conversion
  domains; do not turn this performance take into a parity repair.
- `exact_queries_match_legacy`
  (`crates/slicer-core/tests/perimeter_spatial_tdd.rs`) checks query results,
  source ordinals, signed zero, and nonvacuous indexed execution. Its Cargo
  target requires both `host-algos` and `perimeter-spatial-test-support`
  (`crates/slicer-core/Cargo.toml`). These tests were inspected, not run.

Historical evidence, not a current-tree measurement:

- [Accelerated residual query findings](../evidence/t18-accelerated-residual/FINDINGS.md)
  split accelerated candidate collection from exact winding scans and locate
  the large residual in the scans. They do not provide the ordinary-mode
  internal split for this job.
- [Arachne re-attribution findings](../evidence/t25-arachne-attribution/FINDINGS.md)
  establish signed distance as a large ordinary guest query term, but aggregate
  its sign and distance work. Their retired output reference is historical,
  not this investigation's acceptance oracle.
- [Overhang-quartile predicate narrowing](45-overhang-quartile-predicate-narrowing.md)
  rules out blanket zero-epsilon boundary-pass removal. Its bbox DROP does not
  establish the signed-distance lead's cost or exactness.

If fresh attribution is authorized, require a non-overlapping fuel accounting
and work counters for classification candidates/predicate work, distance-edge
evaluations, contexts, and fallback reasons. Instrumented wall is not an
acceptance metric. Check both mode-specific guest freshness gates and disclose
current post-repair output identity before interpreting captures. Keep headline
evidence and its reducer in-repo; heavy models remain uncommitted.

Alternatives still needing the human's scope choice: broader Benchy/base and
Classic/Arachne attribution now, or defer new captures entirely. Cached winding
coordinates, 2-D bbox filters, and first-hit exits remain hypotheses, not
authorized implementations. No candidate KEEP/DROP or speed claim is made.

Validation so far: read-only code/evidence inspection. No builds, tests, guest
freshness checks, profiling captures, or timing runs; production/artifacts
unchanged at the scope-decision boundary.

## Answer — resolution comment, 2026-10-05

The human selected **Attribution (Recommended)** in the live scope exchange:
fresh attribution on supports-off Benchy Arachne in ordinary and accelerated
modes, with temporary probes separating sign classification from nearest-edge
distance and checking current output. The broader fixture/generator campaign
was not selected.

This authorizes diagnosis only. It does not authorize candidate implementation,
a speed acceptance campaign, commits, accelerated-adoption retries, default
changes, or reuse of retired output references. The old query-polish park and
the `offset2_ex` park remain unchanged.

Execution is graduated to [Signed-distance sign-versus-distance attribution](49-signed-distance-sign-versus-distance-attribution.md).
No captures ran in this scoping session; closing this decision uses this
session's single-ticket resolution allowance, rather than silently resolving
the diagnostic as well.
