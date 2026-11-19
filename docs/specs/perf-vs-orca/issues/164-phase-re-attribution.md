# Phase re-attribution at HEAD: shares, walls and route arithmetic in one window

Type: task / measurement + attribution + route arithmetic
Label: wayfinder:task
Status: resolved (2026-11-18; measured board + instrumented split + arithmetic — the
  prepass is 67.0-87.2 % of every cell, all eight cells win at canonical prepass
  cost (0.396x-0.890x), and the binding cell moves from `base classic-on` to
  `benchy arachne-on` at 0.890x. See ## Answer)
Assignee: 2026-11-18 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" and, the frontier being empty, picked "Phase re-attribution
  at HEAD" from the fog.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the map's standing fog item **"The phase split must be
  re-attributed before any further route arithmetic"** (surfaced 2026-11-17 by
  [161](161-post-t158-scoreboard-refresh.md)). The split still in the map —
  prepass **63.5–82.4 %**, per-layer **14.4–29.6 %**, postpass **2.6–6.5 %**,
  outside **0.1–5.3 %** — was measured by
  [154](154-benchy-per-layer-carrier-attribution.md) on the t154 tree, before
  [158](158-needs-support-fast-predicate.md) landed a measured 4.23–6.37 s on
  the four base cells and before t150/t151 moved the benchy cells. The fog's
  candidate shape is "a one-window phase re-split (the same instrument t154
  used) run beside a fresh paired scoreboard so shares and walls come from the
  same window".

## Question

At HEAD, after t150, t151 and t158, what are the four phase shares on all eight
cells — and what does the destination arithmetic become when the shares and the
walls are taken from one window, the canonical-prepass substitution is
re-applied, and each cell's non-prepass requirement is re-derived?

Two things the map cannot currently state:

1. **The split.** Is the prepass still 63.5–82.4 % of wall, or did t158's
   per-layer saving on the four base cells move it? Every projection in the map
   is built on the t154 split.
2. **The requirement.** t154's only losing cell at canonical prepass cost was
   `base classic-on` at **1.051x**, needing **1.096 s** from the per-layer
   phase; t157/t158 then removed **4.676 s** of per-layer wall from that cell by
   measurement. On today's shares, which cell carries the residual, how large is
   it, and is the prepass rewrite still the only route to 8/8?

## Scope

1. **One window, two tools.** 8 cells × {OrcaSlicer 2.5.1, PnP ordinary}, 1
   warmup + 3 measured runs per cell, interleaved, matched keys asserted on every
   Orca run (the standing rig,
   `evidence/matched-pair/run_scoreboard.ps1 -Tools orca,pnp-ordinary`). These
   are the walls of record for this take.
2. **Instrumented passes, three per cell.** `pnp_cli slice
   --instrument-stderr`, `RAYON_NUM_THREADS=12`, the matched configs and 24
   external core modules. Every pass must exit 0, be `degraded=false` with
   `fatal_error_count=0` / `non_fatal_error_count=0`, and reproduce t151's
   active reference set in 8/8 (the currency). Three passes per cell give a
   share-stability figure; t154 carried 2–4.
3. **The split.** Reuse t154's reducer (four brackets — prepass, per-layer,
   postpass, validation — plus the outside residual, reconciling to
   `slice_complete`), and record the prepass and postpass stage brackets and the
   per-layer stage thread sums beside it, so the stage-level picture is not lost
   when the shares move.
4. **The arithmetic.** Implied phase wall = share × this window's wall of
   record. Required cut per cell from this window's own Orca and PnP medians.
   Canonical-prepass substitution with t110's canonical prepass costs held fixed
   (the same substitution t154 used), every other phase at this take's measured
   share, labelled `estimate`. Per cell: the projected result at canonical
   prepass, the implied prepass cut the cell needs, and what it still needs from
   non-prepass work.
5. **No production change.** This is a measurement and an arithmetic
   re-derivation.

## Pre-declared gate

**No performance gate** — nothing is adopted. Correctness bar:

- both the scoreboard's PnP runs and every instrumented pass reproduce t151's
  active set in 8/8 cells;
- the four brackets plus the outside residual reconcile to `slice_complete` in
  every cell, and the repeat spread of each phase share is reported, not
  assumed;
- an instrumented wall is never a wall claim; shares are attribution only;
- the box load is sampled through both campaigns and reported;
- no number is composed with t161's or t163's walls: the walls here come from
  this window only, and the canonical prepass costs are held at t110's values as
  a fixed input, not as a wall;
- the Orca executable identity is stamped (the t161 tool boundary).

## Acceptance

- A reduced artifact under `evidence/t164-phase-re-attribution/`: the
  scoreboard table, the per-cell split with repeat spreads, the stage brackets,
  the arithmetic, and a `verify.py` that re-derives every quoted number from the
  committed set and exits 0 on a plain checkout, with `--self-test` negative
  controls.
- The map's destination-status block updated with the new split and the new
  per-cell requirement, and the fog item marked answered.
- If the route arithmetic changes shape — a different cell becomes the binding
  one, or the prepass stops being the only route — that is recorded as the
  finding, not smoothed toward t154's conclusion.

## Answer

Resolved 2026-11-18 as a measured board, an instrumented split, and an
arithmetic re-derivation. Evidence:
[`evidence/t164-phase-re-attribution/`](../evidence/t164-phase-re-attribution/FINDINGS.md)
(`verify.py` PASS, every number re-derived from the committed set; `--self-test`
16/16 negative controls rejected).

**One window, two tools**, OrcaSlicer 2.5.1 (`5013b1bd…`) and PnP ordinary
(`2c44d38f…`, HEAD `c83d253b`), 8 cells x 1 warmup + 3 measured runs, 64 rows,
matched keys asserted on all 24 Orca runs. Every PnP run is byte-identical to
t151's active set in **8/8**; all runs `exit 0`, `degraded: false`, zero fatal
and non-fatal errors. Load 99 samples across both campaigns, min 6.8 % / median 66.6 % / max 100 % (45.9 % median during the scoreboard window).
**24 instrumented passes** (3 per cell) are also byte-identical **24/24**.

### The split at HEAD

Prepass **67.0-87.2 %**, per-layer **9.3-26.3 %**, postpass **3.2-6.8 %**,
outside **0.1-2.3 %**; the per-layer module-busy union equals its wall to within
1 ms, so there is still no idle pool. Repeat spread across the three passes is
0.72-3.91 pp. The prepass share **rose on all eight cells** against t154 -
**+14.85 pp** on base classic-off and **+11.51 pp** on base arachne-off, where
t158's predicate took 4.25 s / 4.23 s out of the per-layer phase of 22 s cells;
**+4.81 / +5.23 pp** on the base supports-on cells (t158: 4.68 s / 6.37 s against
69-71 s walls); **+1.07 to +6.00 pp** on the benchy cells. `MeshAnalysis` is now
687-697 ms on base and 86 ms on benchy. The prepass brackets are
`host:shell_classification` (11.2 s on base classic-off, 68.4 % of its prepass),
`SupportGeometry` (34.9 s on base classic-on, 54.9 %) and `SupportAnalysis`
(12.7 s there, 20.0 %).

### The board

**1.379x-2.884x, 0 of 8 cells won**, required cut **27.5 %-65.3 %**: benchy
1.978x / 2.884x / 1.894x / 2.784x, base 1.501x / 2.812x / 1.379x / 2.578x. This
corroborates t163's 24-hour-old board (1.389x-2.829x, cut 28.0-64.7 %) without
composing with it.

### The arithmetic

**All eight cells win at canonical prepass cost**: benchy **0.611x / 0.874x /
0.649x / 0.890x**, base **0.400x / 0.786x / 0.396x / 0.718x**. **No cell needs
anything from the non-prepass phases** (`needed_from_other = 0` everywhere), so
the prepass alone is sufficient under the estimate - the reversal of t154's
shape, where `base classic-on` was 1.051x and needed 1.096 s from the per-layer
phase.

**The binding cell is `benchy arachne-on` at 0.890x** (11.0 % margin), then
`benchy classic-on` at 0.874x; `base classic-on` is now **0.786x** because t158
removed 4.68 s from its per-layer phase. The constraint is not the projected
result but the **input**: the four supports-on cells each have only
**1.50x-1.75x** slack between their prepass budget and canonical's prepass cost,
and the two benchy supports-on cells have the least (0.874 s and 0.840 s of
budget against 0.550 s). The substitution implies a prepass cut of
**82.6 %-98.1 %**.

### Disclosures

The arithmetic is an **estimate**, not a ratio of record: shares and walls are
this window's, and the single external input is t110's canonical prepass cost
held fixed (`benchy-off` 50.5 ms is a bound, not a point). Shares are attribution
only; the instrumented passes ran in a faster box state than the scoreboard
(base classic-off 19.8 s instrumented against a 22.3 s median wall), so shares
are multiplied by the scoreboard wall and never substituted as a wall. No null
band is claimed: nothing was adopted. One window, one box, three runs per cell
per tool, per-cell wall spreads 1.7-9.9 % (PnP) and 0.3-11.7 % (Orca).
