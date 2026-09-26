# Emit-pass carve gate: representation safety + paired A/B

Type: task
Status: resolved (2026-09-25; human accepted KEEP; commit pending)
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

## Answer

Resolved 2026-09-25 (AFK agent session). **Representation safety: the gate is
safe with two constraints, and unsafe without them; the constrained gate is
implemented and regression-pinned. Timing: see below for the paired A/B
verdict.**

Evidence: [`evidence/t32-carve-gate/FINDINGS.md`](../evidence/t32-carve-gate/FINDINGS.md);
regression suite `t32_gate_*` in the planner's `mod tests`
(`modules/core-modules/tree-support-planner/src/lib.rs`); firing counts from a
temporary probe (removed).

**1. What the skipped clip returns.** On six representative disjoint shapes,
`Difference` is set-equal but never verbatim (clipper normalizes winding and
start vertex). It is a byte-wise *fixed point* for canonical CCW rings; a CW
input is not normalized by a second pass, so winding is an observable input
property.

**2. The chain erases representation for canonical input.** The full
`with_areas` chain (carve → `union_expolys` → optional `expolygons_simplify` →
final set-wide `Difference`) is byte-identical between the production gate and
the unconditional carve across 29 shapes × 3 configs (0 mismatches). Two
guards make that true, and both are load-bearing:

- **Winding guard** — non-canonical rings are never gated
  (`region_has_canonical_winding`); under Clipper's NonZero rule a verbatim CW
  ring can describe a different set.
- **Lone-region repair** — when the gate fired, exactly one region survives,
  and a simplify tolerance is active, the lone region is re-clipped
  (`carve_emitted_regions`). `simplify_ring`'s Douglas–Peucker anchors at
  `points[0]`, so a verbatim ring can select a different kept-vertex set than
  the clip-normalized one: measured on a 16-gon disc at
  `avg_node_per_layer = 500`, area 50,000,000 → 49,989,460 units² (−0.021%).

Without both guards the naive gate diverges on 6 of the 87 combinations; with
them, zero.

**3. Reachable inputs.** Every producer feeding the carve is canonical CCW —
60/60 `node_ellipse` radius/movement/resolution combinations, and
`swept_region`'s monotone-chain hull by construction. The guards are therefore
safety nets, not hot-path filters; on both real fixtures `repaired=0`.

**4. Firing counts (probe-instrumented run).** benchy: gated 11,018,
repaired 0, carved 10,694. base: gated 78,832, repaired 0, carved 69,161 —
exactly ticket 22's predicted carve classes, so the gate fires on the intended
calls. Both runs clean (`degraded=false`, `non_fatal=0`).

**5. Consumer trace.** `build_roles` → `SupportPlanRoleRegion` carries no
representation fields; host aggregation
(`crates/slicer-wasm-host/src/support_aggregation.rs`) is set/area/bbox based;
the tree renderer (`modules/core-modules/tree-support/src/lib.rs`) runs every
role through `union_ex` before reading vertices; traditional-support never
receives tree entries. The in-planner chain is the last place representation
is observable, so the repair there is sufficient.

**6. Timing.** Machine conditions, complete paired rows (including per-sample
cpu/wall and TYPE histograms), artifact provenance, and protocol detail:
[`evidence/t32-carve-gate/FINDINGS.md`](../evidence/t32-carve-gate/FINDINGS.md)
and [`ab-rows.csv`](../evidence/t32-carve-gate/ab-rows.csv). Changing external
load affected every batch (cpu/wall 1.90–4.96 vs quiet reference ~6.5), so
wall is load-qualified; process CPU is less sensitive to descheduling and
corroborates the direction in every completed pair.

- **benchy, 10 paired repeats:** process CPU median −1.71 s / mean −2.05 s,
  range [−4.14, −0.59], **candidate faster in 10/10 pairs** (paired mean /
  standard error ≈ −5.1). Wall (load-qualified) median −6.28 s / mean
  −3.17 s, 8/10 pairs. Every run `degraded=false`, `non_fatal=0`; gcode-size
  ranges overlap and sit inside the known same-binary variance band.
- **ordinary base, 4 completed pairs** (two interrupted batches plus one
  complete short batch): CPU paired median −35.35 s / mean −41.58 s, **4/4**
  negative; wall median −38.95 s, **4/4** negative but the wall mean includes
  one severely load-distorted baseline run. Only complete pairs were kept.
- **accelerated benchy, 10 paired repeats:** CPU paired median −1.48 s,
  **10/10** negative; wall median −1.77 s, **8/10** negative.
- **accelerated base, 3 paired repeats in alternating-order short batches:**
  CPU paired median −38.39 s, **3/3** negative; wall median −33.13 s,
  **3/3** negative. Matched-source accelerated planner WASMs differ from their
  ordinary builds, so the accelerated measurements are independent.

**Human decision: KEEP the guarded candidate** (accepted 2026-09-25). The
output-marker differences are confined to `Inner wall`
count variation also seen within an arm; support and other markers match,
and every accepted run completed non-degraded with zero non-fatal errors.
This is a load-qualified median-wall win corroborated by process CPU, **not**
a quiet-machine wall confirmation. A stricter acceptance requires fresh
interleaved pairs on a quiet machine; ordinary benchy's `<2.0` starvation
exclusion is asymmetric (three baseline samples, no candidates). The explicit
human acceptance authorizes committing this candidate despite that caveat.

## Provenance

Candidate, corrected carve-specific class counts, and the limited probe-test caveat:
[Tree-planner substage attribution](22-tree-planner-substage-attribution.md)'s
`## Answer`; `evidence/t22-planner-substage/FINDINGS.md`. Timing chain position:
the acceptance slot after ticket 22 (map `Notes`).
