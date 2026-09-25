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
region against the whole collision set with no bbox rejection. Its in-guest
probe counts **78,832 (base) / 11,018 (benchy) bbox-disjoint carve calls**,
or **53.3% / 50.7% of the carve's calls** (45.9% / 38.0% of all stage singular
clips). These calls are set-preserving *if* the disjoint-difference semantics
hold for their inputs; their removal's net wall effect has not been measured.
The host's stage-wide disjoint class counts 79,369 / 12,237 calls and records
52.59 / 1.78 s of **core boolean-call** wall, including 537 / 1,219 disjoint
clips **outside** the carve. Do not treat that wall as this gate's saving or
the whole host-call cost; the clip bracket excludes both conversions.

**The safety question comes first, and it is measured, not assumed.** The
ticket-22 probe test (`evidence/t22-planner-substage/probe.patch`,
`t22_probe_diff_identity_tdd.rs`) observed **equal area but different contour
representation for one disjoint rectangle**. It does not independently prove
general set equivalence or whether winding versus starting vertex changed.
The current operation may normalize the output, so skipping it is only
behaviour-preserving if the downstream consumer cannot distinguish the result,
or the gate reproduces that normalization. Work:

1. Trace every consumer of `carve_emitted_regions`' output
   (`build_roles`' role regions → `SupportPlanRoleRegion.regions`
   → `SupportPlanIR` → host aggregation → per-layer support emission) and
   identify which ones read ring order / winding / start vertex. The
   `max_by` tie-break in `carve_emitted_regions` itself already compares
   `contour.points`, so representation is observable at least there.
2. Check set equivalence and exact output representation on representative
   disjoint inputs (including holes and noncanonical contours), then pin the
   reachable downstream contract in a regression test. A verbatim input clone
   is acceptable **only if** the observable output remains equivalent. If
   representation matters, reproduce the existing normalized result or reject
   the gate; a verbatim clone would *change* the current baseline.
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

Candidate, corrected carve-specific class counts, and the limited probe-test caveat:
[Tree-planner substage attribution](22-tree-planner-substage-attribution.md)'s
`## Answer`; `evidence/t22-planner-substage/FINDINGS.md`. Timing chain position:
the acceptance slot after ticket 22 (map `Notes`).
