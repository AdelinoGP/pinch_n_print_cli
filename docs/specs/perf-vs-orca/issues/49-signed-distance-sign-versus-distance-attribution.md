# Signed-distance sign-versus-distance attribution

Type: task
Label: wayfinder:task
Status: resolved
Assignee: agpen (current OpenCode wayfinder attribution session), 2026-10-05
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 48

## Question

On the current supports-off Benchy Arachne job, how does
`signed_distance_to_boundary` cost divide between inside/outside classification
and nearest-edge distance in ordinary versus accelerated mode, and does that
split justify scoping an output-preserving candidate?

## Authorization

The human authorized this bounded paired-mode attribution in
[Signed-distance investigation scope](48-signed-distance-investigation-scope.md).
Temporary diagnostic probes and current-output checks are permitted. This is
not permission for a production optimization, a speed acceptance campaign,
broader fixtures/generators, commits, adoption retries, or default changes.
Claim before executing. Return any candidate recommendation to the human.

## Work

- Consult `debug-pipeline`, `docs/17_agent_debugging.md`, the map's measurement
  traps and evidence policy, and the scope ticket's verified code facts.
  Reinspect actual source and inputs; do not trust old hashes or missing
  scratch artifact paths as current references.
- Preserve unrelated working-tree changes. Isolate the temporary probes and
  paired mode snapshots; do not switch branches without the repo's required
  stash discipline. Record source/config/model/build identities. Check
  ordinary and accelerated guest freshness using their respective gates;
  one mode's clean check is not evidence for the other.
- Prepare and verify current post-repair outputs for the same matched
  supports-off Benchy Arachne job in both modes. Do not silently recreate or
  substitute historical frozen inputs. If required inputs are absent or
  current ordinary/accelerated output disagrees, record the blocker before
  interpreting an attribution capture. Preserve the fan-command reference
  boundary and output disclosure.
- Add non-overlapping guest-fuel probes around the ordinary full-scan helper
  `signed_distance_to_boundary` (`crates/slicer-core/src/perimeter_utils.rs`)
  and accelerated `indexed_inside` / `indexed_distance`
  (`crates/slicer-core/src/perimeter_spatial.rs`). Split sign candidate
  collection from exact winding evaluation, and distance seed/range/exact
  work without double-counting nested scopes. Account separately for
  `PerimeterSpatialContext::new` construction, wrappers, and fallbacks.
- Record query/context counts, sign candidate and predicate work, distance-edge
  evaluations, and fallback reasons. Reconcile leaf self-fuel plus wrapper
  self-fuel with inclusive public-query fuel; prove that any counted path was
  exercised rather than drawing conclusions from zero samples.
- Check probe outputs against the current mode-specific reference before
  reading their costs; disclose completion, degraded state, nonfatal errors,
  output bytes/TYPE counts and exact identity. Fuel is attribution, not a
  wall-win verdict. No instrumented wall enters an acceptance claim.
- Reduce headline evidence into `docs/specs/perf-vs-orca/evidence/` with an
  in-repo reducer/verifier that succeeds without gitignored raw captures.
  Do not commit the heavy models. Restore diagnostic-only source/artifact
  changes and verify restoration before finishing.
- Recommend one candidate question, or no candidate, based on the fresh split
  and its unknowns. Preserve existing zero-epsilon boundary verdicts,
  contour-only sign semantics, hole-edge distance, signed zero, source-order
  nearest-edge ties, fallbacks, and coordinate conversions. A candidate needs
  separate authorization and independent exactness proof before the standing
  fresh-reference paired ordinary/accelerated wall-and-CPU gate.

## Verification guidance

The existing `exact_queries_match_legacy` test in
`crates/slicer-core/tests/perimeter_spatial_tdd.rs` requires
`--features host-algos,perimeter-spatial-test-support`; a bare crate run can
silently omit it. If tests are needed for probe changes, run the narrow target
and tee combined output to `target/test-output.log`, then inspect that log.
Use the gated test entry point for any guest-touching broad run. No full
workspace test or adoption acceptance ceremony is required by this ticket.

## Resolution evidence

[Signed-distance findings](../evidence/t49-signed-distance/FINDINGS.md) hold the
full split, accounting, controls, output disclosure and validation.
`summary.json` is re-derived by `verify.py --falsify` using the in-repo reduced
captures, without the raw tree or licensed model.

## Answer — resolution comment, 2026-10-05

Fresh supports-off Benchy Arachne attribution is complete in both modes.
Excluding separately marked counter self fuel, ordinary signed-distance query
fuel splits **44.705% sign / 55.238% nearest-edge distance**, accelerated
**89.758% / 10.144%**; wrapper remainders are separately accounted. Exact
winding predicate self fuel holds 99.864% / 95.432% of sign respectively.
Context construction is a separate 4.120% / 7.556% of probed module fuel.
Self-fuel accounting reconciles exactly to inclusive query/context/module
fuel, without nested double-counting.

Both modes exercised 240 contexts and 110,293 public queries; accelerated
signed-distance fallbacks were zero. Ordinary evaluates 260,813,669 distance
edges; accelerated evaluates 2,327,888 (seed traversal, direct seed and range
counts reconcile). Sign-predicate calls are 192,918 / 187,986. Presented
contour-vertex counts are not exact predicate-loop iterations. Accelerated
evaluates 34,537 predicates after first true; its saved fuel was not separately
measured, and ordinary already short-circuits via `.any()`.

Probe observer effects are disclosed: unmodified-guest controls measure
module fuel **+0.500% ordinary / +1.142% accelerated** under the probes. These
are probe-build shares, not an assertion of identical production instruction
counts. No instrumented wall enters a speed claim.

Fresh baseline, probe and profile-control outputs in both modes all match the
current post-repair reference `5a61a106…`, 3,809,027 bytes, validated Arachne
marker, clean completion, zero fatal/nonfatal errors and unchanged TYPE
counts. The 1,110 unchanged open-loop warnings remain a limitation. Both
freshness gates passed before preparation and after restoration. All recorded
source inputs match pre-probe hashes; active Developer distributions are
byte-restored. The existing exact-query test passed on the probe tree; all
three `perimeter_spatial_tdd` tests passed after restoration with both required
features. The standing output validator and falsifiable evidence verifier pass.

**Recommendation:** scope one exact prepared contour-coordinate cache for the
signed-distance sign predicate in both modes, preserving the unchanged
boundary/winding arithmetic and independent oracle. Its isolated conversion
cost, cache construction/memory overhead, exactness and CPU/wall benefit are
unmeasured; this is not an implemented candidate or KEEP/DROP. The human's
authorization decision graduates separately. Other parked query work remains
parked. No production optimization, commit, adoption retry or default change
occurred.
