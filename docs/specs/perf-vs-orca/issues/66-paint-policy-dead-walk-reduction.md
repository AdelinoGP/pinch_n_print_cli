# Support paint-policy dead-walk reduction

Type: task
Label: wayfinder:task
Status: resolved (2026-10-14; standing paired gate measured GATE NOT MET on the wall clause — human-approved DROP, candidate retired, no production edit; see ## Answer)
Assignee: 2026-10-14 OpenCode wayfinder session (map resumption; human route decision)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Can the support paint-policy query's dead geometry walk be eliminated exactly
— without changing any produced path, any paint veto, any off-grid anchor or
any role — and does the candidate pass the standing paired ordinary +
accelerated uninstrumented wall/CPU gate?

[Tree-support renderer self-fuel and scan-fill attribution](64-tree-support-renderer-attribution.md)
measured `PaintRegionLayerView::paint_policy_for`
(`crates/slicer-sdk/src/traits.rs`) at **34.697 B guest instructions, 79.7% of
all tree-support renderer fuel** (43.510 B dispatch, 836 expoly renders, 240
dispatches, both matched classic/arachne supports-on cells: the split is
generator-invariant). Per query on the matched job: one SliceIR region (the
support-glue view is rebuilt from the dispatch's own regions, ~2.4 polygons /
1,883 edges), ~427.7 caller contour vertices tried, ~1,284.7 ring walks,
~893,291 edge tests at ~46.5 metered instructions each — and **cover_hits = 0
of 836, eligibility checks = 0**: the walk never finds a covering region and
`support_eligibility` never runs, so every query returns `DefaultEligible`.
The wall context is attribution-only: renderer accumulated worker time is
6.79–8.35 s over 12-thread phases of 8.2–8.7 s (6.5–8.5% machine occupancy),
0–2 ms on the runs' last-finishing layer, so the freeable share is bounded by
~0.45–0.55 s of phase wall in the probed runs — the paired gate decides the
real number.

## Candidate shapes (exactness argument required per shape)

1. **Annotations-empty fast path.** When the view's SliceIR regions carry no
   support-blocker/support-enforcer annotation values, `support_eligibility`
   can only return `DefaultEligible` for every covering region; returning
   `DefaultEligible` without the geometry walk is value-identical. This
   covers every unpainted (auto-support) job — i.e. every matched cell. The
   emptiness test itself must be cheap (a per-view precheck at construction or
   a per-query scan over the same region set the walk would cover), and must
   not silently change behavior for painted jobs.
2. **Ring bbox prefilter.** A conservative per-ring bounding-box gate before
   `point_in_polygon_ring` removes ring walks that cannot contain the vertex;
   shapes the painted-job path. Exact if the gate can only ever reject
   vertices outside the ring; disclose how the boxes are computed and cached.

The shapes are not mutually exclusive; the ticket's experiment scope decides
which to implement (or documents that neither survives exactness scrutiny).

## Scope and decision evidence

1. Implement the selected shape(s) as a candidate with the standard
   qualification: independent engine/unit tests for the fast path (unpainted
   returns value-identical; painted behavior unchanged), output identity
   against the preserved references for all matched cells in
   **both modes**, and disclosure of any reference needing refresh per the
   [Frozen-job output drift boundary](46-frozen-job-output-drift-boundary.md)
   (the current t58 references are post-t47 and current).
2. Preserve every documented semantic from the t64 take: painted vetoes,
   wall/fill separation, widths/flows, off-grid anchors, roles and output
   order are untouched; no regularization skipped and no planned geometry
   simplified to reduce the walk.
3. The change lives in `slicer-sdk` (guest + native halves); keep the two legs
   behavior-identical (mirror the packet-241-style view-repair discipline) and
   run the repo's narrow parity gates around it.
4. The standing paired ordinary + accelerated uninstrumented wall/CPU gate (6
   retained pairs per cell, starved samples excluded, cpu/wall quoted, outputs
   disclosed); KEEP/DROP returned to the human; candidates never auto-committed.
5. Timing campaigns must not overlap other open tickets' campaigns (t63's
   scheduling-baseline rule): a renderer-cost change does not alter linker or
   scheduling work, but the fresh-baseline rule from
   [Independent low-layer dispatch](63-low-layer-dispatch-granularity.md)
   applies to any campaign started alongside it.

## Acceptance and boundaries

Attribution take t64 graduated this candidate; this ticket is implementation
+ qualification only. No WIT or manifest change is expected (the view change
is internal geometry/gating code in the shared SDK); if a WIT edit becomes
necessary, stop and re-run the tracker's route decision. A failing gate ends
in a documented DROP with candidate retired to a re-appliable patch and
production restored (both freshness gates green), following the t51/t57
restore discipline.

Evidence: [t64 findings](../evidence/t64-tree-support-renderer/FINDINGS.md),
its captures, reducer, verifier and `probe.patch`.

## Comments

### Take pre-declaration — 2026-10-14 (recorded before any measurement)

**Candidate.** The annotations-empty fast path (t64's shape 1): a new
`slicer_core::paint_policy::has_support_paint` predicate plus an early
`DefaultEligible` return in `PaintRegionLayerView::paint_policy_for`
(`crates/slicer-sdk/src/traits.rs`) when no region of the attached slice
carries a `Some(_)` value under `SupportBlocker`/`SupportEnforcer`.
Exactness argument: `support_eligibility` classifies from exactly those two
map entries plus the region-area floor, so when the predicate is false for
every region, every covering region is `DefaultEligible` and the aggregate
is `DefaultEligible` regardless of the region-ownership walk; painted views
(predicate true) still take the walk unchanged. Shape 2 (per-ring bbox
prefilter) is **not** implemented in this take: it shapes only the painted
path, which has no representative workload or fixture (the map's
painted-wall fog), while the fast path already covers every matched cell.
The shape stays documented here as available work.

**Qualification (before timing).** The 16-cell output-identity matrix —
{benchy, base} × {classic, arachne} × {supports off, on} × {ordinary,
accelerated} — baseline and candidate once per cell; outputs must be
byte-identical to each other and to the preserved t58 references; clean
completion (`ok`, not degraded, zero fatal/non-fatal), non-vacuous
wall/infill/support role counts.

**Timed cells (the standing four-cell set).** `ordinary-benchy-classic-on`,
`accelerated-benchy-classic-on`, `ordinary-base-classic-on`,
`accelerated-base-classic-on`. Rationale: the dead walk lives in the
supports-on renderer (t64: 836 queries per benchy supports-on slice; the
supports-off cells never reach the query), and the renderer split is
measured generator-invariant (classic vs arachne agree to ≤0.003 B per
scope, same dispatch/instruction shape), so arachne-on cells carry the same
effect class; their outputs remain covered by the 16-cell qualification. Any
extension to the arachne cells is a new pre-declaration.

**Protocol.** Per timed cell: 1 proof pair (baseline + candidate once, output
hashes must match), then 1 warm-up per arm, then 6 baseline/candidate pairs
with odd pairs baseline-first and even pairs candidate-first; 12 rayon
threads; process wall and CPU via `GetProcessTimes`; a whole pair is excluded
when either arm's cpu/wall ratio falls below half that arm's median (never
resampled; fewer than 6 retained pairs = inconclusive, not a pass); paired
medians of retained per-pair ratios; gate = wall delta < 0 % AND CPU delta
≤ 0 % in every timed cell.

**Builds.** Baseline = HEAD snapshot, candidate = HEAD + this take's diff,
each built as complete ordinary (`cargo xtask dist --edition developer`) and
accelerated (`--accelerated --edition developer`) snapshots; runs use
`--module-dir <snapshot>/modules --no-default-module-paths
--no-integrated-modules` with `SLICER_MODULE_PATH` cleared; `build-guests
--check` must exit 0 in the arm's tree before its build.

**Artifacts.** Scripts and reducers: `evidence/t66-paint-walk/`; raw runs and
snapshots under the ignored
`.local-artifacts/perimeter-reference-preparation/t66-paint-walk/`. The
candidate stays uncommitted until the human's KEEP/DROP; both guest freshness
gates are green at the candidate tree.

## Answer

**Measured GATE NOT MET — human-approved DROP (2026-10-14); candidate retired,
no production edit.**

The annotations-empty fast path (shape 1:
`slicer_core::paint_policy::has_support_paint` plus the early `DefaultEligible`
return in `PaintRegionLayerView::paint_policy_for`,
`crates/slicer-sdk/src/traits.rs`) was implemented exactly as pre-declared and
passed the full 16-cell output-identity qualification: baseline == candidate
== preserved t58 references in every cell, all runs clean (`build-
qualification.py`: 16/16, `pass=True`; `verify-references.py`: 16/16,
`pass=True`; `qualification.json`).

The standing paired gate then ran the four pre-declared classic supports-on
cells (2 warm-ups + 6 alternating pairs per cell; 6/6 pairs retained
everywhere; zero starvation exclusions; all 56 runs `ok`, not degraded, and
byte-identical): **benchy wall +0.0017 % (ordinary) / +0.1272 %
(accelerated)** — the pre-declared wall < 0 % clause measured and missed —
while **base wall −0.4977 % (ordinary) / −2.1098 % (accelerated)** and
**CPU −0.56 %…−1.22 % in all four cells** passed. Read plainly: the dead
walk's CPU cost is real and removed everywhere; its wall transfer
materializes on the base fixtures only and sits inside paired noise on the
~23–27 s benchy slices (consistent with t64's 0.45–0.55 s
per-layer-phase freed-wall bound and 0–16 % fuel-to-wall transfer range).
The human ruled **DROP** on 2026-10-14.

Disposition per the standing DROP discipline (t51/t57 pattern): the three
source edits and the new test file are retired to `HEAD` (none remain in the
tree; a retirement stash holds the working-tree copies). The re-appliable
artifacts are committed with this ticket —
[`candidate.patch`](../evidence/t66-paint-walk/candidate.patch) (sha256
`8ccfbeb1e81888956c29455c04aa80193b75772d5c95b0efb1bee01b66d8fe52`,
byte-identical to the as-measured `git diff`) plus
[`candidate-new-file-paint_policy_for_tdd.rs`](../evidence/t66-paint-walk/candidate-new-file-paint_policy_for_tdd.rs)
(sha256 `d949a9324bb862dbdf7c205620f402fb0a67d8888280dee5ee58ab038ae72ec3`).
No adoption retry, one-sided exception or further retry. Re-entry of the
shape requires a new pre-declaration; the measured effect class is recorded
(CPU −0.6…−1.2 %, base wall −0.5…−2.1 %, benchy wall ≈0).

Recovery disclosures (campaign-side only, none a slicer failure): the proof
driver was killed 2026-10-14 12:37:34 (−03:00) by an opencode
location-services eviction mid-cell-14 (candidate stderr frozen just past
`phase_start` for `prepass` at 25,842 bytes, no panic — the same signature
t58 recorded); the partial cell was archived under
`.local-artifacts/perimeter-reference-preparation/t66-paint-walk/interrupted-2026-10-14T1237-accelerated-base-classic-on/`
and re-ran from scratch per the one-shot-guard discipline; the campaign was
relaunched detached from the agent process tree
(`evidence/t66-paint-walk/launch-detached.ps1`, the t58 remedy) and ran
through two further harness evictions (15:36, 16:37) with no cell boundary
disrupted. Background load disclosed (Opera/Discord resident); zero
starvation exclusions materialized. The standing dist dirs were restored
from the baseline snapshots (`4fd7f3d6…` / `f80f16ac…`) and both guest
freshness gates are green at the restored tree (guests rebuilt per mode
after the revert).

Evidence:
[FINDINGS.md](../evidence/t66-paint-walk/FINDINGS.md) (`qualification.json`,
`ab-reduction.json`, campaign tooling incl. the detached launcher, candidate
patch).