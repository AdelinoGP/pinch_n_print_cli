# Emit-pass carve gate: representation safety + paired A/B

Type: task
Status: open
Blocked by: 22 (resolved)

## Question

Is the `carve_emitted_regions` bounding-box gate (the candidate [Tree-planner
substage attribution](22-tree-planner-substage-attribution.md) produced)
behaviour-preserving, and does it move wall under the standing acceptance
protocol?

The candidate, from measured class counts at the matched job: `carve_emitted_regions`
(`modules/core-modules/tree-support-planner/src/lib.rs`) differences each drawn
region against the whole collision set with no bbox rejection. A bbox-disjoint
pre-test classifies **46.2% (base) / 42.2% (benchy)** of the stage's singular
`clip_polygons` calls as disjoint — calls the clip set cannot affect as a set —
holding **26–27% of the measured clip wall** (52.6 s / 1.47 s of 194.0 s /
5.65 s; 149,201 / 22,255 avoided host calls).

**The safety question comes first, and it is measured, not assumed.** The
ticket-22 probe test (`evidence/t22-planner-substage/probe.patch`,
`t22_probe_diff_identity_tdd.rs`) established that a disjoint `Difference`
returns the subject **as a set but not verbatim** — clipper normalizes ring
order and winding. So the gate is only behaviour-preserving if the consumers of
the carve result are insensitive to that representation. Work:

1. Trace every consumer of `carve_emitted_regions`' output
   (`build_roles`' role regions → `SupportPlanRoleRegion.regions`
   → `SupportPlanIR` → host aggregation → per-layer support emission) and
   identify which ones read ring order / winding / start vertex. The
   `max_by` tie-break in `carve_emitted_regions` itself already compares
   `contour.points`, so representation is observable at least there.
2. If representation matters anywhere reachable, the gate must preserve the
   subject verbatim on the disjoint path (return the input clone) rather than
   accept clipper's normalization — state which and why in the design.
3. Then measure: paired ordinary + accelerated A/B per the standing metric
   (uninstrumented probe-free runs, interleaved repeats, starvation exclusion,
   isolated snapshots, `--module-dir modules/core-modules`), output disclosure
   per the fairness contract, keep/drop to the human, no auto-commit.

Constraints: intended geometry preserved exactly (the gate's whole claim);
`union_expolys` + simplify (the stage's second sub-term, 72.8 s base) is out of
scope for this ticket unless the gate alone comes back short.

Acceptance: the representation-safety finding first (a written answer, even if
it blocks the gate), then the paired A/B if it does not.

## Provenance

Candidate, class counts, and the measured representation caveat:
[Tree-planner substage attribution](22-tree-planner-substage-attribution.md)'s
`## Answer`; `evidence/t22-planner-substage/FINDINGS.md`. Timing chain position:
the acceptance slot after ticket 22 (map `Notes`).
