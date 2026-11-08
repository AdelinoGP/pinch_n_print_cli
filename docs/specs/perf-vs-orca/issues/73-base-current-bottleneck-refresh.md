# Base current bottleneck refresh after the clipper-vendoring keep

Type: task
Label: wayfinder:task
Status: resolved (2026-10-18; fresh eight-capture base attribution — prepass
dominated at post-t70 HEAD, three tickets graduated, t20 park unchanged; see
## Answer)
Assignee: 2026-10-18 OpenCode wayfinder session — human selected the
base-cell bottleneck refresh after the frontier re-derivation found only the
parked [Consume-only-when-read annotations](20-consume-only-when-read-annotations.md)
open
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

At the post-t70 HEAD, which current **base** costs justify new, non-duplicated
investigation tickets toward the map's matched-job destination?

The 2026-10-17 scoreboard refresh re-derived the four base cells as the
destination's far end — 7.39x–18.03x median wall against OrcaSlicer, with
216–440 s of PNP ordinary wall in a single cell — while the freshest base
attribution predates the wall-moving keeps that have landed since: the
clone-once finalization (t58), the infill-linker batch execute (t65, Arachne),
the region-preparation derivation sharing (t67) and the in-tree clipper
hot-path candidate (t70). The 2026-10-09 refresh and the t60 `host:native`
split are the right shape but the wrong tree; their bucket shares cannot be
quoted as current. Re-derive the base profile rather than reusing either.

Run the four matched base cells (classic/arachne, supports off/on) through
fresh ordinary-mode instrumented fuel profiles, including per-call detail to
localize long-tail layers. Corroborate the stage shape of any cell whose
profile is ambiguous with profiling-off instrumented runs. Qualify the
current release build, guest freshness, external-module provenance and output
evidence before interpreting any capture. Compare against the preserved
t58 qualification references and the scoreboard's fresh wall/CPU context, and
disclose any drift instead of normalizing it away.

This is attribution and route expansion, not a regression repair or
optimization experiment: no candidate, acceptance campaign, adoption retry,
default switch or commit is authorized by this take. Profile/worker elapsed
is not process CPU or uninstrumented wall. No fresh Orca comparison or
matrix-win claim is intended.

## Acceptance

- Durable reduced evidence and a checkout-only verifier, with raw-capture
  hashes and an in-repo reducer for optional reconstruction.
- Clean completions and disclosed bytes, TYPE counts and reference identity for
  every fresh run; any mismatch investigated before using the run.
- Existing leads updated rather than duplicated; new children ask precise
  questions and carry explicit scope, evidence and blocking edges.
- Resolve only this diagnostic ticket; leave optimization decisions to later
  sessions and the human's keep/drop ruling.

## Comments

### Take start — 2026-10-18

Prerequisites checked before any capture: the matched configs and both heavy
fixtures are present (`tmp/base.stl`, `tmp/3dbenchy.stl`; raw captures stay
under `target/`, never committed), `cargo xtask build-guests --check` exits 0,
and the preserved t58 qualification carries all four ordinary base references
(`20e7204c…` classic-off, `f60e5e9b…` classic-on, `3ba16419…` arachne-off,
`a1bc8110…` arachne-on). No candidate or timing campaign is part of this take.

## Answer

**Resolved 2026-10-18 as a measurement-only take — eight fresh base captures at
HEAD `86c15182` (slicer sources = the t70 keep `7127b4d6`), no candidate, no
production change, no timing campaign.**

Method: four ordinary-mode fuel profiles (`--profile --profile-verbose
--instrument-stderr`) plus four instrumentation-only corroborations, one per
generator × support state, 12 rayon threads, the map's matched configs. Every
output is byte-identical to its t58 qualification reference (all four base
cells, both observer modes); every completion is `ok`, `degraded=false`, zero
fatal/non-fatal. Guest freshness exit 0, 24/24 external modules. Evidence:
[`evidence/perf-refresh-base-2026-10-18/`](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md)
(`captures/`, `summary.json`, `reduce.py`, `verify.py` exits 0 with four
negative controls rejected, `analyze.py`, `run-refresh-base.ps1`).

Headline (instrumented observer; process wall s / prepass / per-layer /
postpass):

| base cell | wall | prepass | per-layer | postpass |
| --- | ---: | ---: | ---: | ---: |
| classic supports-off | 214.5 | 115.0 | 94.3 | 2.4 |
| classic supports-on | 408.7 | 237.6 | 98.2 | 69.1 |
| arachne supports-off | 130.2 | 109.2 | 16.1 | 1.4 |
| arachne supports-on | 360.2 | 259.9 | 19.8 | 76.6 |

1. **The base cells are prepass-dominated**, not per-layer-dominated: the
   common floor (MeshAnalysis + Slice + OverhangAnnotation +
   ShellClassification + SeamPlanning) is 104–115 s on every cell, and the
   supports-on cells add 134–149 s of SupportAnalysis + SupportGeometry.
   `PrePass::Slice` (40.0–44.9 s) remains under ticket 59's
   no-representation-safe-reduction verdict.
2. **The supports-on finalization residual is the largest un-attributed term**:
   four whole-print module brackets of 10.1–12.5 s each (45.0–48.5 s total)
   against a supports-off control of 0.30–1.39 s, while their combined guest
   fuel is 5.45 B of 12,964 B (0.042 %). The classifier early-return audit
   (ticket 20) still holds, so the brackets are host-side view/dispatch/merge,
   not module compute. Graduated:
   [LayerFinalization supports-on whole-print view cost](74-layerfinalization-supports-on-view-cost.md)
   (recommended first).
3. **ShellClassification's serial phase-B commit is still 26.0–31.4 s** in all
   four base cells; ticket 27 measured the serial commit at 25.07–30.29 s and
   named the `internal_bridge_angles` input pre-reduction. Graduated:
   [ShellClassification phase-B serial commit reduction](75-shellclassification-phase-b-commit.md)
   (recommended second).
4. **MeshAnalysis is a serial exact union**: 19.4–22.2 s of stage wall,
   `host:mesh_analysis`'s only scope `polygon_ops::clip_polygons` at
   19.2–20.1 s accumulated (concurrency ≈ 1x), over 326,546 projected facets.
   Ticket 27's bbox replacement is unsafe; the surviving question is an exact,
   representation-preserving reduction. Graduated:
   [MeshAnalysis exact footprint union reduction](76-meshanalysis-footprint-union.md)
   (recommended third).

Existing leads updated, not duplicated: ticket 20 received the base-classic
context (per-layer 94.3/98.2 s; module 98.8/95.0 % of fuel; 10,573 B self fuel;
t18's 57.9 % base scan share) with the below-fold park **unchanged** (best cell
still 3.30x); ticket 59's Slice verdict stands; the planner lineage
(22/53/55/57) remains native-clipper-bound with no new shape at HEAD (t70
already took the hot loop); the t64 renderer is only 2.1 s accumulated on base
supports-on; ticket 63's low-layer tail is 0 ms on base; ticket 68's round-trip
bucket is already 97–99 % native ops.

Limits: ordinary mode only; no accelerated, Orca or fresh uninstrumented
comparison; profile brackets inflate phase walls (classic per-layer 110.6 vs
94.3 s) and are never acceptance timings; the supports-on module brackets'
internal split is ticket 74's job. No speedup, regression, scoreboard win,
geometry-correctness verdict, default change or production-source edit is
claimed. This take resolves only this refresh; the three new tickets are
unclaimed investigations and ticket creation is not execution approval.
