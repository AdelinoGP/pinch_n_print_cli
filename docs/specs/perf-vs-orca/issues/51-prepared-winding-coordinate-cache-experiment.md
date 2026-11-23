# Prepared winding-coordinate cache: exactness + standing paired A/B

Type: task
Label: wayfinder:task
Status: resolved (2026-10-06; human-approved DROP; candidate source/tests retired, patch preserved, active artifacts restored)
Assignee: agpen (current OpenCode implementation/evaluation session), 2026-10-06 — closed by human-approved DROP
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 50

## Question

Does one eager per-context boundary-contour winding-coordinate cache preserve
signed-distance query semantics exactly and improve total uninstrumented wall
with corroborating process CPU in both modes, after including its incremental
preparation and teardown cost?

## Authorization and context

The human approved this bounded implementation and evaluation in
[Prepared winding-coordinate cache scope](50-prepared-winding-coordinate-cache-scope.md).
Its resolution owns the numerical/diagnostic contract, exactness requirements,
stop rules and exclusions. Claim before work; return KEEP/DROP to the human.

Read [Signed-distance findings](../evidence/t49-signed-distance/FINDINGS.md) for
the fresh cost split and observer qualification. Conversion savings and cache
overhead are unmeasured. This candidate is not already speed-approved.

## Work

1. **Preflight and isolation.** Read the map's Notes, the scope resolution,
   `docs/08_coordinate_system.md` and relevant architecture/test-quality docs.
   Reinspect current source and available inputs. Preserve unrelated work and
   capture source/config/model/toolchain identities. Check both ordinary and
   accelerated guest freshness independently before preparing isolated external
   Developer baseline snapshots. Use the attribution job configuration at
   `docs/specs/perf-vs-orca/evidence/t44-clip-universe-hoist/benchy-arachne.json`;
   re-derive current output identities rather than pinning an old SHA. If inputs
   are missing or fresh mode outputs disagree, stop and record the blocker.
2. **Implement only the authorized cache.** Inspect
   `PerimeterSpatialContext::new`, its `signed_distance_to_boundary` dispatch and
   `indexed_inside` (`crates/slicer-core/src/perimeter_spatial.rs`). Keep
   `point_in_contour_winding` (`crates/slicer-ir/src/polygon_predicate.rs`) and
   `signed_distance_to_boundary` (`crates/slicer-core/src/perimeter_utils.rs`)
   unchanged as independent oracles. Route ordinary context queries through an
   isolated candidate path without silently changing the full-scan oracle.
   Cache contour coordinates, not holes or f32-rounded distance endpoints.
   Retain query traversal, fallback and diagnostic semantics per the scope.
3. **Independent exactness.** Add falsifiable regressions covering the approved
   adversarial inputs and nonvacuous execution in both modes. Compare predicate
   verdicts to the unchanged predicate and full results bitwise to the unchanged
   full scan; check signed zero, source-order distance ties and forced fallback
   behavior. Use fixed analytical edge expectations as an additional oracle.
   A final-G-code match alone is insufficient. Verify test target features from
   Cargo and source before trusting results; `perimeter_spatial_tdd` requires
   `host-algos,perimeter-spatial-test-support`. Tee each test run's combined
   output to `target/test-output.log` and inspect it before the next run.
4. **Preparation accounting and output proof.** Record actual cache construction
   work, allocation/storage and query reuse separately from query work; include
   their limitations. Diagnostic probes, if needed for this accounting, stay
   separate from timing builds and use non-overlapping accounting. Prepare
   candidate snapshots that differ from each mode's baseline only in the Arachne
   guest. Pass both mode-specific freshness gates and both current byte-identical
   output proofs before either timing batch. Disclose status, bytes, TYPE counts,
   generator marker and unchanged warnings; preserve the fan-reference boundary.
5. **Standing paired A/B.** Use the unchanged protocol in
   `docs/specs/perf-vs-orca/evidence/t45-quartile-bbox/protocol.json` as the
   measurement template, changing only candidate-specific identity/purpose.
   It defines repetitions/order, starvation exclusion, minimum retained pairs
   and paired median wall/CPU keep clauses. Runs must be probe-free, external-only
   subprocesses. Charge all process preparation and teardown; do not subtract
   construction or infer wall savings from query fuel. Stop on exactness failure
   or inconclusive/failed speed gate; no automatic retry or one-mode exception.
6. **Evidence and handback.** Keep headline evidence and a falsifiable
   reducer/verifier under `docs/specs/perf-vs-orca/evidence/`, usable without
   gitignored raw captures. Leave heavy models uncommitted. Run workspace build,
   all-targets Clippy and literal/touched-test-quality gates for the code change;
   no full workspace suite is required by this ticket. Restore diagnostic code
   and active production distributions from preserved baseline snapshots, then
   verify restoration and both freshness gates. Preserve the candidate patch
   and tests for the human's KEEP/DROP decision; no auto-commit or adoption/default
   switch. State exactness, measurement, construction/storage findings and gaps,
    bounded to this attribution job rather than claiming an Orca scoreboard win.

## Execution update

The initial campaign completed with bit-identical outputs but insufficient
retained pairs under the unchanged starvation rule (see
[`summary.json`](../evidence/t51-winding-cache/summary.json)). Work stopped at the
gate and returned to the human. The human then explicitly requested **"Try one
more run"**: one additional complete campaign, with identical snapshots, protocol
and job. This is a human-authorized exception to the no-automatic-retry rule, not
a threshold change. Preserve both campaigns separately; do not pool them or
schedule further retries. KEEP/DROP remains with the human.

## Measured answer — human-approved DROP (2026-10-06)

**Exactness passes; the standing both-modes speed gate does not. DROP approved by the human.**
The candidate is implemented and all seven exactness tests pass in both build
modes. The independent precision negative control fails as intended, and every
real proof/timing output is byte-identical. Construction/storage is measured
separately; workspace build, all-targets Clippy and literal/touched-test-quality
gates pass.

The initial campaign was inconclusive (2 ordinary pairs and 1 accelerated pair
retained). The one human-authorized repeat retains all 6 pairs in each mode.
Ordinary passes wall/CPU; accelerated fails wall despite lower CPU. No pooling,
one-mode exception, automatic retry or Orca scoreboard claim is made.

Evidence and reproducible verification:
[Prepared-cache findings](../evidence/t51-winding-cache/FINDINGS.md),
[initial summary](../evidence/t51-winding-cache/summary.json),
[repeat summary](../evidence/t51-winding-cache/retry-1-summary.json).

### Cleanup record — DROP and restore

The human selected **DROP and restore**. Candidate production code and tests
were retired together to the preserved pre-experiment tree; both Rust files
match `HEAD` and the archived `source-before` copies exactly. The preserved
[`candidate.patch`](../evidence/t51-winding-cache/candidate.patch) passes
`git apply --check` against the restored tree. Rebuilt ordinary and accelerated
Arachne guests are byte-identical to their archived pre-experiment baseline
snapshots (`e93985ff…` / `cc54cc8b…`), both mode-specific freshness checks
return exit 0, and the restored original spatial suite passes all three tests
with the required features explicitly enabled. Active Developer distributions
already matched their archived baseline inventories and remain restored. The
machine-checkable cleanup record and hashes are in
[`cleanup.json`](../evidence/t51-winding-cache/cleanup.json). No commit, adoption
retry, default switch or new ticket take occurred.
