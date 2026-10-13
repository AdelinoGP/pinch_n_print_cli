# Prepared winding-coordinate cache scope

Type: grilling
Label: wayfinder:grilling
Status: resolved
Assignee: agpen (current OpenCode wayfinder scope session), 2026-10-05
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 49

## Question

Should the next bounded take test one exact prepared contour-coordinate cache
for signed-distance sign classification in ordinary and accelerated modes,
with independent query exactness checks followed by the standing
fresh-reference paired wall/CPU gate; or should this lead remain parked?

Recommendation: authorize that one candidate only on the same supports-off
Benchy Arachne attribution job. First settle the numerical/diagnostic contract
and incremental construction overhead, then implement an isolated experiment
only if the human approves it. No speed benefit has been established.

## Context and scope boundaries

[Signed-distance sign-versus-distance attribution](49-signed-distance-sign-versus-distance-attribution.md)
provides the fresh split and observer-qualified evidence. Winding classification
is substantial in ordinary and dominant in accelerated; the isolated cost of
its coordinate conversions is not measured. A cache could lose once
construction, allocation and memory effects are included.

The live exchange must settle whether to authorize the candidate, its scope,
exactness oracle and keep/drop protocol. Consult `grilling` and
`domain-modeling`; read the predecessor findings before prescribing an
implementation. Before the resolution below, this ticket provided no permission
to edit production code, run candidate builds or start a speed campaign.

Any authorized candidate must preserve:

- Both unchanged boundary/winding passes, including exact zero-epsilon edges,
  degenerate/repeated vertices, nonfinite/fallback behavior and contour-only
  sign semantics; holes still participate in distance, not sign.
- Winding's direct `units as f64 / 10_000.0` coordinates, distinct from
  distance's f32-rounded `f64::from(units_to_mm(...))` coordinates. No changed
  operation ordering/FMA assumption or reuse of distance endpoints without
  exactness proof.
- Signed zero, nearest-edge source-order ties, and observable fallback and
  diagnostic contracts. Keep an unchanged, independent predicate/full-scan
  oracle; a final-G-code match alone cannot falsify every query defect.
- Current post-repair output references, clean-output disclosure, both
  mode-specific freshness checks, and the standing probe-free paired
  ordinary/accelerated wall/CPU gate after exactness passes.

Do not bundle first-hit exits, bbox narrowing, quartile changes, distance
algorithm changes, broader fixture/generator campaigns or a default/adoption
switch. All other parked query work remains parked. A future KEEP/DROP
recommendation returns to the human; no automatic commit or retry.

## Answer — resolution comment, 2026-10-05

The human selected **Bounded gate (Recommended)**, then **Confirm (Recommended)**
on the execution contract. One eager, per-region-context prepared
boundary-contour coordinate cache is authorized for signed-distance sign
classification, in ordinary and accelerated modes, on the same supports-off
Benchy Arachne attribution job. This is not the matched-Orca scoreboard job.

The approved contract is:

- Prepare only direct-f64 winding coordinates once per context. Retain both
  existing predicate passes and their arithmetic/order. Preserve ordinary
  short-circuit behavior and accelerated source-ordered candidate evaluation,
  existing distance arithmetic, ties, fallback reasons and diagnostic behavior.
  The shared predicate and full-scan helper remain unchanged independent oracles;
  quartile, bridge and distance algorithms are outside the candidate.
- Prove exactness before speed evaluation: independent predicate comparisons and
  bit-exact full-query checks for exact edges, holes, repeated/degenerate vertices,
  large/rounding-sensitive coordinates, signed zero, distance ties, nonfinite
  queries and forced fallback paths. Include fixed expected edge verdicts so a
  shared bug cannot hide behind a differential oracle. Current real-output
  proof must be byte-identical in both modes before either timing batch.
- Account separately for incremental cache construction and storage. Judge total
  probe-free process wall and CPU, including preparation, allocation and teardown,
  under the unchanged standing paired-mode gate. Both modes must pass; a
  one-sided or CPU-only exception requires another human decision. Construction
  or query fuel alone is not an end-to-end win; storage accounting is not a new
  peak-RSS acceptance campaign.
- Stop on exactness failure or an inconclusive/failed speed gate and return the
  findings and KEEP/DROP recommendation to the human. No automatic retry,
  alternative-candidate substitution, commit, adoption retry or default change.
  Other parked query optimizations remain parked.

Execution is graduated to
[Prepared winding-coordinate cache: exactness + standing paired A/B](51-prepared-winding-coordinate-cache-experiment.md).
It is unclaimed and unblocked by this resolution. This session resolves only the
scope decision; it does not implement or execute that experiment.

Validation at this boundary: read-only source/coordinate-contract inspection and
`python docs/specs/perf-vs-orca/evidence/t49-signed-distance/verify.py --falsify`
passed, including negative controls. No candidate code, builds, tests, freshness
checks or new captures ran. Conversion savings, incremental overhead and
CPU/wall benefit remain unmeasured. Production sources/artifacts and unrelated
working-tree changes are preserved.
