# Overhang-quartile query: predicate narrowing + standing paired A/B

Type: task
Status: resolved (2026-10-05; human-approved DROP/inconclusive; source and active artifacts restored)
Assignee: agpen (current OpenCode wayfinder session), 2026-10-05 — human authorized full gated take
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 25 (46 resolved 2026-10-05 — gate prerequisite satisfied; see ## Work)

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

**Authorization recorded 2026-10-05:** human selected the full gated take:
behavior-preserving implementation experiment and equivalence proof first,
then paired ordinary + accelerated wall/CPU measurement only if correctness
passes, using fresh post-repair baselines. Preserve existing exactly-on-edge
verdicts; return KEEP/DROP to the human. No automatic commit, adoption retry,
or default-mode switch.

Gate prerequisite surfaced by [Clip-universe preparation hoist](44-clip-universe-hoist.md):
**resolved by [Frozen-job output drift boundary](46-frozen-job-output-drift-boundary.md)
(2026-10-05)** — the drift is intended (config-scope-resolution + the t47 fan
repair), the boundary is current refs `5a61a106…` (t44 job) / `e1088b3d…`+alias
(classic-off), and frozen `7049a06d…` / t44-era `1b71f83d…` retire as timing
baselines for this job. A fresh-baseline gate for this take may proceed against
the current references with the t47 fan-command disclosure; the hoist's
exception is still not map-wide.

- Choose the shape: narrow `point_in_contour_winding` /
  `point_in_polygon_winding` (`crates/slicer-ir/src/polygon_predicate.rs`) so
  an `eps = 0.0` call skips the boundary pass outright — which needs an
  explicit decision on exactly-on-edge points, since the strict path currently
  admits them via `dist2 <= 0` — versus an inclusive per-polygon bbox
  prefilter inside `PerimeterSpatialContext::overhang_quartile` /
  `indexed_overhang_quartile` (`crates/slicer-core/src/perimeter_spatial.rs`),
  which is value-preserving by construction but whose saving is unmeasured
  (the y-interval lookups may already reject most candidates).
- Prove output equivalence first against freshly captured current-tree,
  post-repair supports-off Benchy Arachne references in each mode (the retired
  frozen `7049a06d…` is a historical control, not this take's oracle); add a
  regression test that fails if the
  narrowed predicate ever disagrees with the full one on any in/out verdict.
- Then the standing measurement: paired ordinary + accelerated, interleaved
  repeats with starvation exclusion, uninstrumented probe-free runs, isolated
  host/guest snapshots, output disclosure per the fairness contract. Fuel
  alone is not a wall win — gate before keeping.
- Keep/drop recommendation returns to the human; no auto-commit.

## Not authorized

Any automatic commit, adoption retry, or default-mode switch. The human's
2026-10-05 authorization above covers this take only; the ticket 25 study
authorized attribution only.

## Resolution comment — human-approved DROP/inconclusive (2026-10-05)

Human-authorized full gated take completed. Straight zero-epsilon boundary-pass
removal changes existing edge verdicts and was not implemented. The alternate
inclusive contour-bbox prefilter preserves both real proof pairs and every
measured output against the current post-repair reference. Ordinary median wall
increases despite lower CPU; accelerated medians improve slightly, with sample
ranges overlapping in both modes. The standing paired-mode gate fails, so
**DROP approved by the human**, not a CPU-only or one-sided KEEP exception.

Detail, reduced samples/inventories, re-derivation, negative control and
re-applicable candidate patch:
[Quartile bbox findings](../evidence/t45-quartile-bbox/FINDINGS.md).
The human selected **DROP and restore**. Candidate production code and tests
were removed together; both Rust files match the pre-experiment tree exactly.
The preserved patch passes `git apply --check`. Restored ordinary/accelerated
active distribution inventories match their archived baseline snapshots, both
guest freshness checks return exit 0, and normal workspace/debug binaries were
rebuilt from the restored tree. The restored original spatial suite passes all
three tests with the required features explicitly enabled. No commit, adoption
retry, default switch or second ticket take occurred.

## Originating attribution evidence

[Arachne re-attribution findings](../evidence/t25-arachne-attribution/FINDINGS.md),
probe preserved at `evidence/t25-arachne-attribution/probe.patch` +
`evidence/t25-arachne-attribution/*.rs.txt`, reduced captures at
`evidence/t25-arachne-attribution/captures/`, re-derivation in
`evidence/t25-arachne-attribution/verify-t25.py`.
