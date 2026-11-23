# Signed distance: sign versus nearest-edge distance

Route: [Signed-distance sign-versus-distance attribution](../../issues/49-signed-distance-sign-versus-distance-attribution.md).

**Fresh attribution supports scoping an exact prepared winding-coordinate
cache, not implementing it yet.** Ordinary query fuel splits **44.705% sign /
55.238% distance**; accelerated splits **89.758% sign / 10.144% distance**.
Classification's exact predicate, not candidate collection, is the shared
remaining lead. This is a diagnostic result, not a speed win or KEEP/DROP.

## Job and current-output gate

The authorized job is `benchy-arachne.json` in
`docs/specs/perf-vs-orca/evidence/t44-clip-universe-hoist/`: 0.5 mm nozzle,
0.2 mm layers, 3 walls, 25% infill, supports off, 12 Rayon threads. It is
**not** the matched-Orca scoreboard job. Model/config/compiler/source hashes
and actual snapshot inventories are in `identity.json`. No historical frozen
inputs were regenerated or substituted.

Ordinary and accelerated freshness checks returned exit 0 before baseline
preparation. Fresh baseline outputs, probe outputs, and unmodified-guest
profile controls all match byte-for-byte:

- SHA-256 `5a61a1066a4607c6968cc038eb221a38ed39d30b5d26d8448ad87895abc92c4a`;
  **3,809,027 bytes**, 240 layers; validated Arachne generator marker.
- Completion `ok`, `degraded=false`, zero fatal/nonfatal errors throughout.
- TYPE counts: Skirt 1, Brim 1, Outer wall 698, Inner wall 744, Bottom surface
  23, Bridge 18, Sparse infill 139, Internal solid infill 6, Top surface 32,
  Internal Bridge 12.
- **1,110 unchanged open-loop warnings** remain a correctness limitation.
  Clean completion does not establish closed geometry or canonical parity.

The standing `Test-MeasurementEvidence` validator
(`resources/perimeter-acceptance/validate_measurement.ps1`) also checked all
captured outputs; results are in `output-validation.json`. No fan stripping
or retired-reference comparison was used. The snapshots enforce external-only
module dispatch; only the Arachne component and diagnostic host differ from
each mode's baseline. Every manifest and other guest is byte-identical.

## Non-overlapping fuel split

`probe.patch` preserves the temporary instrumentation. It wraps
`PerimeterSpatialContext::signed_distance_to_boundary` and `new`
(`crates/slicer-core/src/perimeter_spatial.rs`), the independent full-scan
`signed_distance_to_boundary` (`crates/slicer-core/src/perimeter_utils.rs`),
and indexed sign/distance internals. The unchanged winding predicate is marked
at its callers; `crates/slicer-ir/src/polygon_predicate.rs` was not edited.

The reducer sums **self fuel**, never adds a child's inclusive fuel to its
parent. Percentages below use query self fuel excluding the explicit counter
bookkeeping bucket; context construction is separate.

| Query term | Ordinary self fuel | Ordinary share | Accelerated self fuel | Accelerated share |
| --- | ---: | ---: | ---: | ---: |
| Sign classification | 27,053,571,758 | 44.705% | 28,662,007,089 | 89.758% |
| Nearest-edge distance | 33,427,638,202 | 55.238% | 3,239,212,575 | 10.144% |
| Wrapper/reduction remainder | 34,411,416 | 0.057% | 31,319,813 | 0.098% |

Within sign, exact winding predicate self fuel is **99.864% ordinary /
95.432% accelerated**. Accelerated candidate collection self fuel is
1,173,670,501; the predicate is 27,352,717,559. Acceleration does not remove
the whole-contour boundary/winding predicate for each selected polygon.

Accelerated distance's non-overlapping self buckets are seed traversal
1,834,209,534; range traversal/reduction 1,021,863,711; exact edge arithmetic
302,625,440; indexed wrapper/radius/result 80,513,890. Exact edge arithmetic
is nested under both seed and range; it is counted once here. The final
distance square root and sign application stay in their enclosing wrapper.

Context construction measures 5,196,102,224 ordinary / 5,537,154,119 accelerated
inclusive fuel, **4.120% / 7.556% of the respective probed Arachne module**.
This is not assigned to either query's sign or distance. Ordinary `new` still
prepares indexes even though ordinary queries use the full scan; no change to
that behavior was tested.

Reconciliation passes exactly in both modes:

```text
sign self + distance self + wrappers self + query counter self = public inclusive
all t49 scope self = public inclusive + context inclusive
all module scope self + module self = module total
```

### Observer qualification

Additional marks/counters do compile to guest instructions even though the
host callbacks themselves burn no fuel. Unmodified-guest profile controls
therefore accompany the probes. Module total fuel changes from
125,503,350,296 to 126,130,773,503 ordinary (**+0.500%**), and from
72,453,494,474 to 73,280,584,408 accelerated (**+1.142%**). Explicit query
counter self fuel is 40,500,495 / 233,150,895; removing that bucket does
**not** remove all probe machinery or compilation effects.

These are measured **probe-build shares**, not an assertion that production
has those exact shares. The split is a strong localization signal with its
observer delta disclosed. Profile walls are accumulated/instrumented spans;
none is used as a wall-win claim. No acceptance CPU/wall campaign ran.

## Exercised work, not extrapolated work

Both modes constructed **240 contexts** and made **110,293 public queries**.
All accelerated queries completed indexed sign and distance, with **zero
signed-distance fallbacks**. Linear/other entry-reason scopes and fallback
scopes had no activations; this establishes absence only on this job, not
fallback safety on all inputs.

| Work count | Ordinary | Accelerated |
| --- | ---: | ---: |
| Exact sign-predicate calls | 192,918 | 187,986 |
| Contour vertices presented to those predicates | 173,033,046 | 175,177,655 |
| Exact distance-edge evaluations | 260,813,669 | 2,327,888 |

The sign vertex counter records contour sizes presented to the unchanged
predicate: **not actual boundary-pass/winding-pass iterations**. An early
boundary hit can terminate a pass. Predicate fuel measures executed work;
we do not turn the vertex counter into an exact loop count or cache-savings
estimate.

Accelerated sign sees 388,422 Y-envelope edge records, deduplicated to
187,986 source-ordered polygon candidates. Distance makes 530,491 exact edge
evaluations inside nearest-seed traversal, 110,293 direct seed re-evaluations,
and 1,687,104 range re-evaluations. Their sum equals the all-edge counter.

The indexed sign loop evaluates **34,537 predicates / 19,287,301 presented
vertices after its first true verdict**. A first-hit exit is therefore reached
on this workload, but its saved predicate fuel was **not separately measured**.
Ordinary already uses `.any()`, so first-hit alone is not a shared paired-mode
candidate. It remains an alternative hypothesis, not an approved change.

## Recommended next decision and preserved boundaries

Scope **one exact prepared contour-coordinate cache for signed-distance sign
classification in both modes**, then decide whether to authorize its
independent exactness proof and standing fresh-reference paired CPU/wall gate.
The fresh split supplies a reason to revisit this particular previously
parked query lead; it does not reopen every parked query optimization.

Why this lead: the same exact predicate dominates sign in both modes, and
sign remains substantial in ordinary while dominating accelerated. Its
per-edge integer-to-direct-f64-mm conversions are visible in
`point_in_contour_winding`, `dist2_point_to_segment_mm`, and `is_left`
(`crates/slicer-ir/src/polygon_predicate.rs`). **Their isolated cost and the
cache's benefit, construction cost, memory cost and wall transfer are still
unmeasured.** A prepared cache is a candidate question, not a demonstrated
optimization or a route that already closes the Orca gap.

Any authorized experiment must retain both predicate passes and exact-zero
boundary verdicts, contour-only sign semantics, hole-edge distance, signed
zero, source-order nearest-edge ties, fallbacks and diagnostic contracts.
Keep winding's `units as f64 / 10_000.0` distinct from distance's
`f64::from(units_to_mm(...))` (the latter rounds through f32). Do not reuse
distance endpoints for winding. Keep the unchanged predicate/full-scan oracle
available; unchanged final G-code alone does not prove all query semantics.
No bbox, first-hit, quartile or distance-algorithm change is bundled into this
recommendation.

## Validation, restoration and reproduction

- Narrow `exact_queries_match_legacy` passed with both required features on
  the probe tree. After restoration the full `perimeter_spatial_tdd` target
  passed **3 tests**; `restored-perimeter-spatial-tests.log` preserves the log.
  Its existing missing-crate-doc warning remains unchanged.
- Both diagnostic distribution builds and both production restoration builds
  completed. All recorded source inputs match their pre-probe hashes.
- Both post-restoration mode-specific freshness gates returned exit 0; active
  ordinary/accelerated Developer distributions match their baseline
  inventories byte-for-byte (`restoration.json`). Rebuilt hosts differed in
  PE metadata; the preserved baseline host copies were reinstated in the
  active distributions. Shared host/guest caches were rebuilt from restored
  source, not left with probe code. No source branch switch or stash occurred.
- `verify.py --falsify` passes from the in-repo reduced set, checking output,
  work populations, exact fuel conservation, recorded restoration and summary
  re-derivation. It rejects altered costs, absent counts, output drift and
  nonfatal status sabotage in both modes.
- No workspace suite, candidate implementation, commit, adoption retry or
  production-default change ran. No performance improvement is claimed.

Recheck from a plain checkout (no raw captures or models needed):

```bash
python docs/specs/perf-vs-orca/evidence/t49-signed-distance/verify.py --falsify
```

Optional fuller raw source:
`.local-artifacts/perimeter-reference-preparation/t49-signed-distance/`.
`capture.py reduce-existing` regenerates the reduced captures/disclosures from
it without running a slice; `verify.py --reduce` regenerates `summary.json`.
`validate-output.ps1` rechecks the raw outputs with the standing validator.
The heavy model was neither copied into tracked evidence nor committed.
