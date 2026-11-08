# Prepared winding-coordinate cache — DROP approved

## Result

The implemented candidate passed the independent exactness checks and preserved
the current real G-code byte-for-byte, but **did not pass the standing
both-modes end-to-end wall/CPU gate**. No OrcaSlicer scoreboard win is established.

The initial campaign was inconclusive after starvation exclusion. Work stopped
and returned to the human, who explicitly requested **"Try one more run"**.
That one additional complete campaign retained all pairs: ordinary passed,
accelerated failed wall despite lower CPU. The human then approved **DROP** —
not an ordinary-only KEEP or CPU-only exception. No further retry is authorized.

## Campaigns, kept separate

Same supports-off Benchy Arachne attribution job, 12 threads, unchanged protocol
from ticket 45. Each campaign has fresh output proofs, excluded warmups, and six
alternating AB/BA measured pairs per mode. Timing uses external-only, probe-free
subprocesses; GetProcessTimes charges total creation-to-exit wall and process CPU,
including preparation, allocation and teardown. Nothing is subtracted.

| Campaign | Ordinary pairs retained | Accelerated pairs retained | Verdict |
|---|---:|---:|---|
| Initial | 2 / 6 | 1 / 6 | Inconclusive; each mode needs at least 5 |
| Human-authorized repeat | 6 / 6 | 6 / 6 | Both-modes gate fails on accelerated wall |

Repeat medians and paired median candidate-minus-baseline deltas, in seconds:

| Mode | Baseline wall | Candidate wall | Paired wall delta | Baseline CPU | Candidate CPU | Paired CPU delta | Gate |
|---|---:|---:|---:|---:|---:|---:|---|
| Ordinary | 14.427373 | 14.233015 | -0.134447 | 69.945313 | 68.492188 | -1.171875 | Pass |
| Accelerated | 12.528308 | 12.697882 | +0.042139 | 64.609375 | 63.859375 | -0.789063 | Fail |

Accelerated median wall and paired median wall delta are both unfavorable. This
is a failed directional gate, not a claim of statistically significant harm.
The ordinary median changes are -1.35% wall / -2.08% CPU; accelerated changes are
+1.35% wall / -1.16% CPU. Wall improves in only 3 of 6 ordinary pairs and 2 of 6
accelerated pairs; CPU improves in 5 of 6 pairs in each mode. Ordinary observed
wall ranges are 12.871–15.898 s baseline and 13.742–14.790 s candidate;
accelerated ranges are 12.447–13.805 s baseline and 12.530–13.198 s candidate.
The ranges overlap. No statistical significance test or isolated explanation
for the wall/CPU mismatch was performed; lower CPU cannot override the gate.
The initial under-populated timings remain descriptive only. Neither campaign
is pooled with, replaces, or selectively repairs the other.

Sources: [`summary.json`](summary.json),
[`retry-1-summary.json`](retry-1-summary.json),
[`evidence.json`](evidence.json), [`retry-1-evidence.json`](retry-1-evidence.json).
The repeat authorization binds original evidence hashes, the unchanged protocol,
candidate patch and fresh mode checks in
[`retry-1-authorization.json`](retry-1-authorization.json).

## Implementation and exactness

`PerimeterSpatialContext::new` and `PreparedWindingContour::contains`
(`crates/slicer-core/src/perimeter_spatial.rs`) prepare only boundary-contour
coordinates with direct `units as f64 / 10_000.0`. Both predicate passes,
arithmetic ordering, zero-epsilon boundary behavior, ordinary short circuit and
source-ordered accelerated candidate traversal are retained. Holes remain
distance-only. Distance endpoint conversion stays f32-rounded. Fallbacks retain
the unchanged full-scan helper.

Independent oracles remain unchanged:
`point_in_contour_winding` (`crates/slicer-ir/src/polygon_predicate.rs`) and
`signed_distance_to_boundary` (`crates/slicer-core/src/perimeter_utils.rs`). The
ordinary candidate has a separate full-query path so differential tests do not
silently compare the new algorithm to itself.

All seven tests in `perimeter_spatial_tdd`
(`crates/slicer-core/tests/perimeter_spatial_tdd.rs`) pass with the required
`host-algos,perimeter-spatial-test-support` features under ordinary and controlled
accelerated builds. They exercise analytical edges, holes, repeated/degenerate
vertices, raw-unit rounding-sensitive and large coordinates, signed zero,
source-order distance ties, nonfinite inputs, and forced fallbacks. Large-coordinate
full-context tests assert actual indexed execution, not a silent fallback.

A deliberate mutation replacing direct-f64 winding endpoints with f32-rounded
endpoints fails the precision regression at raw-unit x = 16,777,217 and query
`(1677.7216999999998, -1.0)`; it reports inside where the unchanged predicate
reports outside. The mutation was removed before candidate builds. Evidence:
[`precision-negative-control.log`](precision-negative-control.log),
[`exactness-tests.log`](exactness-tests.log),
[`accelerated-exactness-tests.log`](accelerated-exactness-tests.log).

Every proof and timing output in both campaigns has SHA-256
`5a61a1066a4607c6968cc038eb221a38ed39d30b5d26d8448ad87895abc92c4a`,
3,809,027 bytes, identical TYPE counts and 1,110 unchanged open-loop warnings.
The generator is validated as Arachne; status is `ok`, `degraded=false`, with
zero fatal/nonfatal errors. TYPE counts are disclosed in each summary. These
are current post-repair references; no pre-fan-repair baseline is used.

## Incremental preparation and storage

Separate diagnostic captures measure the cache, not acceptance timings:

| Quantity | Ordinary | Accelerated |
|---|---:|---:|
| Contexts | 240 | 240 |
| Contours | 575 | 575 |
| Prepared vertices | 395,668 | 395,668 |
| Public queries | 110,293 | 110,293 |
| Mean queries per context | 459.554 | 459.554 |
| Retained capacity bytes, summed over contexts | 6,337,588 | 6,337,588 |
| Maximum single-context retained capacity bytes | 60,052 | 60,052 |
| Backing buffers, summed over contexts | 814 | 814 |
| Cache construction self fuel | 14,114,655 | 14,114,131 |
| Construction share of probed module fuel | 0.011531% | 0.020362% |
| Separately marked bookkeeping self fuel | 5,017,905 | 5,017,905 |
| Probe-minus-control module fuel delta | +0.008906% | +1.378241% |

Retained capacity counts outer contour-buffer storage and cached point-buffer
storage; it excludes allocator metadata and is neither peak RSS nor an allocator
call count. Buffers and vertices are summed across context lifetimes, not claimed
to be simultaneously resident. One context per observed guest invocation is
verified before quoting the maximum. Query counts include every public query,
not a claim that every query reuses the cache.

Construction and bookkeeping scopes are non-overlapping and conserve module
fuel. The observed accelerated instrumentation delta qualifies diagnostic fuel;
these shares cannot establish uninstrumented wall savings. Teardown is charged
by total process measurements but not separately profiled. No peak-RSS campaign
or query-conversion savings measurement ran.

Source: construction accounting in both summaries, derived from the four
`*-control-profile.json` / `*-probe-profile.json` files and preserved
[`probe.patch`](probe.patch). Diagnostic code was removed, both guests rebuilt,
and their hashes matched the preserved probe-free candidates before timing.

## Validation, evidence and cleanup boundary

Workspace build, all-targets Clippy with the test-support feature, literal gate
and touched-test-quality report passed for the candidate. No full workspace test
suite was required or run. Original gate commands, exit observations and log
hashes are archived in [`validation.json`](validation.json), alongside the logs.
The build/Clippy logs contain an mtime-based test-guest staleness warning and
dependency future-incompatibility warnings; these are disclosed rather than
described as warning-free builds. Both mode-specific artifact freshness gates
passed again
before the repeat. Source, compiler and snapshot identities matched the first
campaign; each candidate differs from its baseline only in
`modules/arachne-perimeters/arachne-perimeters.wasm`.

The verifier works against the in-repo evidence without raw captures:

```bash
python docs/specs/perf-vs-orca/evidence/t51-winding-cache/verify-t51.py --falsify
python docs/specs/perf-vs-orca/evidence/t51-winding-cache/verify-t51.py --campaign retry-1 --falsify
```

Negative controls reject output drift, dirty status, altered ratio arithmetic
and host substitution. Synthetic insufficient-pair and CPU-only wins cannot
pass the gate. The two campaigns use independent populations; the original
evidence and summary hashes are preserved.

Optional raw captures are under
`.local-artifacts/perimeter-reference-preparation/t51-winding-cache/`, with the
repeat under `retry-1/`. Reduction reads existing captures, never reruns slices:

```bash
python docs/specs/perf-vs-orca/evidence/t51-winding-cache/verify-t51.py --campaign retry-1 --reduce .local-artifacts/perimeter-reference-preparation/t51-winding-cache --falsify
```

**DROP approved and executed by the human (2026-10-06):** candidate production
code and tests were retired together to the preserved pre-experiment tree; both
files match `HEAD`. The preserved re-applicable [`candidate.patch`](candidate.patch)
passes `git apply --check` against the restored tree. Diagnostic code was already
absent. Rebuilt ordinary and accelerated shared guest artifacts are byte-identical
to their archived pre-experiment baseline snapshots, both mode-specific freshness
checks return exit 0, and the restored original spatial suite passes all three
tests with the required features explicitly enabled. Active ordinary and
accelerated Developer distributions already matched their archived baseline
inventories and remain restored. Machine-checkable hashes and the restored-suite
log are in [`cleanup.json`](cleanup.json) and
[`cleanup-restored-suite.log`](cleanup-restored-suite.log). No commit,
adoption/default switch, new ticket or further retry occurred. Unrelated
working-tree changes and heavy model fixtures are preserved.
