# Overhang-quartile bbox prefilter — DROP approved

Route: [Overhang-quartile query: predicate narrowing + standing paired A/B](../../issues/45-overhang-quartile-predicate-narrowing.md).

**DROP approved by the human (2026-10-05).** The candidate preserves the tested
output, but fails the standing paired-mode speed gate. Ordinary median wall increases while CPU falls;
accelerated wall and CPU medians fall slightly. Neither mode has strict sample
range separation. No adoption retry, default switch, or commit occurred.

## Candidate and correctness boundary

Skipping the entire zero-tolerance boundary pass is not behavior-preserving:
`point_in_contour_winding` (`crates/slicer-ir/src/polygon_predicate.rs`)
accepts a square's right-edge midpoint through its zero-distance boundary
check, while winding alone rejects it. The predicate remains unchanged.

The experiment instead prepares inclusive contour-only bounds once in
`PerimeterSpatialContext::new` (`crates/slicer-core/src/perimeter_spatial.rs`)
and filters before the unchanged exact predicate in both ordinary and indexed
quartile queries. Bounds cover direct-f64 raw endpoints, computed segment
endpoints `a + (b - a)`, and outward roundoff slack. Nonfinite queries bypass
the filter. Rings with fewer than three vertices still reject; repeated
vertices, exact edges, and the predicate's intentional hole-ignore semantics
are preserved. Distance and bridge queries are unchanged.

Three candidate regressions, preserved in `candidate.patch`'s changes to
`crates/slicer-core/tests/perimeter_spatial_tdd.rs` (not active after DROP):

- `quartile_bbox_preserves_edges_holes_degenerate_rings_and_nonfinite_queries`
  uses analytically fixed verdicts, including exact square edges and an ignored
  hole.
- `quartile_bbox_prunes_same_y_disjoint_x_without_changing_winding_verdicts`
  requires one exact evaluation among 32 same-Y disjoint-X polygons, then
  compares query output against the unchanged production predicate.
- `quartile_bbox_matches_unchanged_predicate_at_large_and_rounding_sensitive_coordinates`
  compares directly against that predicate at integer extrema, cancellation
  geometry and adjacent-f32 query values, with positive and negative populations.

These test the candidate's equivalence/pruning, not Orca geometry parity or an
exhaustive proof over every possible polygon. The separate analytical inspection
found no counterexample to the numerical envelope; its argument is that the
configured `32uM` slack exceeds the estimated `12uM` winding-sign overrun
(`u = 2^-53`, `M = max(abs(a), abs(b), 1)`). This is an analytical error-bound
argument, not a measured performance result or exhaustive executable search.

## Real-output proof and isolation

Baseline ordinary and accelerated Developer distributions were built and copied
before production edits. Candidate arms copy their mode's baseline and replace
**only** `modules/arachne-perimeters/arachne-perimeters.wasm`; host executable,
all manifests and every other component are byte-identical. Inventories were
recorded before timing and rechecked by the reducer afterward. The experiment
isolates the Arachne guest; it is not a full-distribution performance claim for
every consumer of the shared core code.

Both proof pairs passed before either measurement batch. Every proof, warmup
and measured output has SHA-256
`5a61a1066a4607c6968cc038eb221a38ed39d30b5d26d8448ad87895abc92c4a`,
3,809,027 bytes, validated Arachne marker, clean completion, no degraded flag,
zero fatal/non-fatal errors, and unchanged 1,110 open-loop warnings.
TYPE counts: Skirt 1, Brim 1, Outer wall 698, Inner wall 744, Bottom surface 23,
Bridge 18, Sparse infill 139, Internal solid infill 6, Top surface 32,
Internal Bridge 12. The unchanged warnings remain a correctness limitation;
clean completion does not prove closed geometry.

The baseline matches the current post-repair reference established by
[Frozen-job output drift boundary](../../issues/46-frozen-job-output-drift-boundary.md).
No fan stripping, retired-reference substitution, or reference overwrite was
used. The job is the ticket-44 attribution configuration: 0.5 mm nozzle,
0.2 mm layers, 3 walls, 25% infill, Arachne, supports off—not the matched-Orca
scoreboard configuration.

## Predeclared paired timing

[`protocol.json`](protocol.json) follows the standing ticket-44 protocol:
12 threads, one excluded warmup per arm, six interleaved AB/BA pairs per mode,
ordinary then accelerated, probe-free external-only subprocesses. Wall is
creation-to-exit `GetProcessTimes`; CPU is kernel+user time. Below 0.75 of the
best CPU/wall ratio within the same arm/mode excludes the whole pair; at least
five retained pairs are required, with no automatic retry. **No exclusions.**

| Mode | Baseline median wall | Candidate median wall | Wall change | Baseline median CPU | Candidate median CPU | CPU change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Ordinary | 13.722599 s | 13.816157 s | +0.682% | 69.210938 s | 67.265625 s | -2.811% |
| Accelerated | 14.246030 s | 13.963873 s | -1.981% | 66.164062 s | 65.875000 s | -0.437% |

Paired median candidate-minus-baseline deltas: ordinary wall +0.076500 s,
CPU -1.507812 s; accelerated wall -0.282156 s, CPU -0.117188 s. Ordinary favors
the candidate on wall in 2/6 pairs and CPU in 5/6; accelerated in 5/6 and 4/6.
The ordinary wall clause fails. Accelerated passes this experiment's
median/delta clauses, but neither metric strictly separates ranges in either
mode. This does not satisfy the stricter accelerated-adoption gate and is not
evidence of beating Orca.

Every sample, CPU/wall ratio, warning/output disclosure and inventory lives in
[`evidence.json`](evidence.json); headline figures are re-derived in
[`summary.json`](summary.json), not inferred from console rounding.

## Validation and reproduction

Passed during this take:

- `cargo test -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd`: 6 tests.
- `cargo xtask test --accelerated --summary -p slicer-core --features host-algos,perimeter-spatial-test-support --test perimeter_spatial_tdd`: 6 tests.
- Both ordinary and accelerated `cargo xtask dist --edition developer` builds,
  before and after the candidate; both mode-specific `build-guests --check`
  gates returned exit 0 before proof/timing.
- `cargo build --workspace`.
- `cargo clippy --workspace --all-targets --features slicer-core/perimeter-spatial-test-support -- -D warnings`.
- `cargo xtask check-literals` and touched-file `check-test-quality --report`:
  no findings.
- Evidence verifier and its altered-proof-output negative control.

Each test run's combined output was captured in `target/test-output.log` and
read before the next run. The workspace-wide test suite was not run. Build
output includes dependency future-incompatibility advisories and a staged
test-guest mtime warning even though the content/artifact freshness checks
returned clean; no guest-integration failure was observed or attributed.

Verify headline evidence from tracked files alone:

```bash
python docs/specs/perf-vs-orca/evidence/t45-quartile-bbox/verify-t45.py --falsify
```

Optional full raw source diff, snapshots, CSVs, G-code and logs:
`target/t45-ab/`. Re-reduction never launches slices:

```bash
python docs/specs/perf-vs-orca/evidence/t45-quartile-bbox/verify-t45.py --reduce target/t45-ab --write-summary
```

The re-applicable [`candidate.patch`](candidate.patch) preserves the exact
experiment source and tests. [`run-ab.ps1`](run-ab.ps1) refuses existing capture
directories; it does not build, retry, commit, or switch production mode.

## Recommendation and limits

**DROP/inconclusive, accepted by the human.** A one-sided or
CPU-only acceptance would be a new explicit human exception, not a pass under
the standing gate. No claim for Classic, base, supports-on, other thread counts,
native full-slice equivalence, fuel reduction, or memory impact. No explanation
of the small wall differences is established by this uninstrumented campaign.

## Human-approved cleanup

The human chose **DROP and restore** on 2026-10-05. Both touched Rust files now
match the pre-experiment tree exactly; the candidate and its three tests are
retired together and preserved, without assertion weakening, in
`candidate.patch`. `git apply --check` passes for that patch.

Rebuilt accelerated and ordinary distributions from the restored tree, then
rebuilt the normal workspace/debug binaries. Every rebuilt guest component and
manifest matched the archived baseline; rebuilt executable bytes differed, so
the active distribution executables were copied back from their exact pre-edit
baseline snapshots rather than attributing the difference without evidence.
Both complete active distribution inventories now match the baseline snapshots
exactly, recorded in [`restoration.json`](restoration.json) and checked by the
headline verifier. Both mode-specific freshness checks returned exit 0 after
restoration. Raw experiment snapshots remain preserved separately.

The restored-tree targeted spatial suite passed all **3 original tests**. The
candidate run passed **6** because it additionally carried the **3** now-retired
candidate tests; both runs use the same explicit required features, so this
count difference is not feature-gate blindness. Candidate controlled-test output
is preserved in [`candidate-accelerated-tests.log`](candidate-accelerated-tests.log).
The restored original file again emits its pre-existing missing-crate-doc
warning; no documentation-only change was kept in production/test code.

No production source change remains, no commit was made, ordinary stays the
default, and no second map ticket was taken.
