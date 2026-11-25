# t164 — Phase re-attribution at HEAD: the split, the walls, and the arithmetic in one window

Route: [Phase re-attribution at HEAD](../../issues/164-phase-re-attribution.md).
Graduated from: the map's standing fog item, *"the phase split must be
re-attributed before any further route arithmetic"*, surfaced by
[t161](../t161-scoreboard-refresh/FINDINGS.md).

**Headline: on the post-t158 tree the prepass is 67.0–87.2 % of every cell's
wall — it grew on all eight cells since [t154](../t154-benchy-per-layer/FINDINGS.md)
— and at canonical prepass cost all eight cells win: benchy 0.611x / 0.874x /
0.649x / 0.890x, base 0.400x / 0.786x / 0.396x / 0.718x. The binding cell is no
longer `base classic-on` (t154's 1.051x, needing 1.096 s from the per-layer
phase); it is `benchy arachne-on` at 0.890x. But the four supports-on cells each
carry less than 2x slack on canonical's own prepass cost, so the whole 8/8
result rests on one fixed input staying where t110 measured it.**

## 1. What was measured

One window, **two tools** — OrcaSlicer 2.5.1 (`orca-slicer.exe` `5013b1bd…`, the
t161 tool boundary) and PnP ordinary (`target/release/pnp_cli.exe`
`2c44d38f…`, HEAD `c83d253b`) — 8 cells × 1 warmup + 3 measured runs,
interleaved, **64 rows**, matched keys asserted on all 24 Orca runs from their
resolved-config appendix.

**Correctness is complete.** All 48 measured and 16 warmup rows exit 0,
`degraded: false`, zero fatal and non-fatal errors, and **every PnP run
reproduces t151's active reference set in 8/8 cells**. Box load sampled 99
times across both campaigns: minimum 6.8 %, median 66.6 %, maximum 100 %;
within the scoreboard window alone (61 samples) the median is 45.9 %.

Then **three instrumented passes per cell** (24 runs, `--instrument-stderr`,
`RAYON_NUM_THREADS=12`, the same configs and 24 external modules), each also
byte-identical to the same reference set, **24/24**. Shares come from these
passes; walls come from the scoreboard. An instrumented wall is never a wall
claim.

## 2. The split at HEAD

Shares are the per-phase median over each cell's three instrumented passes; the
repeat spread is the largest per-phase share movement across those passes.

| cell | slice | prepass | per-layer | postpass | outside | repeat spread |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3,189 ms | **70.46 %** | 22.51 % | 4.85 % | 2.29 % | 0.72 pp |
| benchy classic-on | 6,790 ms | **77.11 %** | 15.17 % | 6.60 % | 1.08 % | 1.88 pp |
| benchy arachne-off | 3,344 ms | **67.00 %** | 26.32 % | 4.85 % | 2.12 % | 3.91 pp |
| benchy arachne-on | 7,034 ms | **75.48 %** | 16.29 % | 6.82 % | 1.07 % | 1.94 pp |
| base classic-off | 19,826 ms | **82.75 %** | 13.24 % | 3.43 % | 0.40 % | 0.96 pp |
| base classic-on | 71,705 ms | **87.20 %** | 9.27 % | 3.38 % | 0.11 % | 1.04 pp |
| base arachne-off | 20,911 ms | **80.56 %** | 15.57 % | 3.47 % | 0.39 % | 1.15 pp |
| base arachne-on | 69,537 ms | **86.86 %** | 9.83 % | 3.19 % | 0.13 % | 1.88 pp |

Every cell reconciles: the four brackets plus the outside residual equal
`slice_complete` (`verify.py` C7). The per-layer phase's module-busy union
equals its wall to within **0–1 ms** in all eight cells, so there is still no
cross-worker idle pool to reclaim on that phase.

Inside the prepass, the stage brackets at HEAD (pass 1 of each cell):

- **base classic-off** (16,379 ms): `ShellClassification` **11,204 ms (68.4 %)**,
  `Slice` 2,224, `SeamPlanning` 1,675, `MeshAnalysis` 687, `SupportGeometry` 221,
  `OverhangAnnotation` 174 — a **host-side** wall (`host:shell_classification`
  11,203 ms of thread).
- **base classic-on** (63,549 ms): `SupportGeometry` **34,865 ms (54.9 %)**,
  `SupportAnalysis` 12,726 (20.0 %), `ShellClassification` 11,034 (17.4 %),
  `Slice` 2,075, `SeamPlanning` 1,816, `MeshAnalysis` 697.
- **benchy arachne-on** (5,409 ms): `SupportGeometry` 2,392, `ShellClassification`
  1,690, `SupportAnalysis` 734, `Slice` 253, `SeamPlanning` 180, `MeshAnalysis` 86.

`MeshAnalysis` is **687–697 ms** on base and **86 ms** on benchy, i.e. t150's
bridge retirement is present and the stage is now the smallest prepass bracket
but one on base.

## 3. What moved since t154

Share movement in percentage points, t164 minus t154. This is an attribution
diff between two trees and two windows — **not** a wall comparison.

| cell | prepass | per-layer | postpass | outside |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | **+6.00** | −3.55 | +0.71 | −2.97 |
| benchy classic-on | +1.70 | −2.17 | +0.48 | −0.01 |
| benchy arachne-off | +3.47 | −3.25 | +0.35 | −0.20 |
| benchy arachne-on | +1.07 | −1.72 | +0.35 | +0.03 |
| base classic-off | **+14.85** | **−15.91** | +0.80 | +0.09 |
| base classic-on | +4.81 | −5.14 | +0.29 | 0.00 |
| base arachne-off | **+11.51** | **−12.34** | +0.71 | +0.12 |
| base arachne-on | +5.23 | −5.80 | +0.55 | +0.03 |

The movement is exactly where the landed tickets predict it. On the two base
supports-off cells the per-layer phase shrank by 12–16 pp — that is **t158's**
fast `needs_support` predicate removing 4.25 s (classic) and 4.23 s (arachne) of
per-layer wall from cells that are 22.3 s and 22.6 s wide. On the two base
supports-on cells the per-layer drop is 5–6 pp, matching t158's 4.68 s and
6.37 s against 69–71 s walls. The benchy cells moved 1–6 pp, consistent with
t158's ~231–237 ms benchy ceiling and with t150/t151's smaller surfaces. **No
cell's prepass share fell**, because every landed ticket since t154 either
touched the per-layer phase (t158), the postpass fairness surfaces (t151), or a
prepass bracket small relative to the cell (t150's 3.7 s on base-off).

## 4. The arithmetic

Walls are this window's medians; shares are this take's instrumented split.
`prepass at canonical` replaces the prepass with t110's canonical cost
(0.0505 / 0.5498 / 2.1030 / 10.4448 s by class) and holds every other phase at
this take's measured share. It is an **estimate**, not a measurement of a
candidate.

| cell | ratio | required cut | prepass cut | prepass budget | canonical | slack | at canonical |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| base arachne-off | 1.379x | 27.5 % | 34.1 % | 12.014 s | 2.103 s | 5.71x | **0.396x** |
| base classic-off | 1.501x | 33.4 % | 40.3 % | 11.022 s | 2.103 s | 5.24x | **0.400x** |
| benchy classic-off | 1.978x | 49.5 % | 70.2 % | 0.791 s | 0.051 s | 15.67x | **0.611x** |
| benchy arachne-off | 1.894x | 47.2 % | 70.5 % | 0.780 s | 0.051 s | 15.44x | **0.649x** |
| base arachne-on | 2.578x | 61.2 % | 70.5 % | 18.228 s | 10.445 s | 1.75x | **0.718x** |
| base classic-on | 2.812x | 64.4 % | 73.9 % | 15.690 s | 10.445 s | 1.50x | **0.786x** |
| benchy classic-on | 2.884x | 65.3 % | 84.7 % | 0.874 s | 0.550 s | 1.59x | **0.874x** |
| benchy arachne-on | 2.784x | 64.1 % | 84.9 % | 0.840 s | 0.550 s | 1.53x | **0.890x** |

- **All eight cells win** at canonical prepass cost. The projected results span
  **0.396x–0.890x**.
- **No cell needs anything from the non-prepass phases** in this arithmetic
  (`needed_from_other = 0` everywhere): the prepass alone is sufficient, under
  the estimate, on every cell. That reverses t154's shape, where
  `base classic-on` needed **1.096 s** from the per-layer phase and could not be
  won by the prepass at all.
- **The binding cell is `benchy arachne-on` at 0.890x** (margin 11.0 %), followed
  by `benchy classic-on` at 0.874x. t154's "only loser" — `base classic-on` — is
  now **0.786x**, because t158 took 4.68 s out of its per-layer phase.
- **The four supports-on cells are the constraint on the input, not on the
  result.** Each has **1.50x–1.75x** slack between its prepass budget and
  canonical's prepass cost. The two benchy supports-on cells have the least:
  a budget of 0.874 s and 0.840 s against a canonical 0.550 s. If canonical's
  benchy supports-on prepass were **1.6x** t110's tap value, both cells would
  cease to win (`benchy arachne-on` at 1.0x needs 0.840 s against 0.550 s ×
  1.53). The supports-off cells have 5.2x–15.7x slack, so they are insensitive
  to that input.
- The prepass cut the substitution implies is **82.6 %–98.1 %**: the prepass has
  to fall to 1.9–17.4 % of today's cost. That is the same order as t129's
  finding that canonical's prepass is 15.7x / 9.8x cheaper on base; the benchy
  cells are the ones asking for ~90–98 %.

## 5. Reading it honestly

- **The estimate chains exactly one input.** Shares and walls are from the same
  window on the same tree; the substitution's only external number is t110's
  canonical prepass cost, held fixed. `benchy-off`'s 50.5 ms is a **bound**, not
  a point, and the supports-on cells' verdicts are the ones that would move if
  that input is wrong.
- **Shares are attribution, walls are not.** The instrumented passes happened to
  run in a faster box state than the scoreboard (base classic-off instrumented
  19.8 s against a 22.3 s median scoreboard wall), which is why the split is
  used as a *share* and multiplied by the scoreboard wall, never substituted as
  a wall. A share measured on a different box state transfers as a share; that
  is the assumption, and it is disclosed.
- **No null band is claimed.** This is a board and an arithmetic re-derivation,
  not a candidate: nothing was adopted and no paired delta is quoted.
- **No cross-window composition.** The board's required cuts are computed from
  this window's two medians. t161's and t163's boards are not composed with
  this one; their numbers appear only as independent corroboration of the same
  order (t163: 1.389x–2.829x, required cut 28.0–64.7 %; here 1.379x–2.884x,
  27.5–65.3 %).
- **One box, one window, load median 66.6 % across both campaigns (45.9 %
  during the scoreboard), max 100 %.** Three runs per cell
  per tool; per-cell wall spreads are **1.7–9.9 %** on PnP and **0.3–11.7 %** on
  Orca, and process-CPU spreads are **0.3–2.9 %** (`reduced/board.json`).
- **Orca identity is stamped** (`5013b1bd…`, 2.5.1). The rig's profile gate
  cannot see an executable swap; this take records the binary hash so a future
  campaign can.

## 6. What it changes for the map

1. **The split in the map is superseded.** Prepass is **67.0–87.2 %**, per-layer
   **9.3–26.3 %**, postpass **3.2–6.8 %**, outside **0.1–2.3 %** — not t154's
   63.5–82.4 / 14.4–29.6 / 2.6–6.5 / 0.1–5.3. Every projection built on the old
   split is stale.
2. **The route's shape changes.** Under the same substitution t154 used, the
   prepass rewrite now wins **8/8**, and the per-layer phase is no longer
   required on any cell. `base classic-on` stops being the destination's
   problem; **the two benchy supports-on cells** become the binding ones, and
   they are binding through the *prepass* input, not through the per-layer
   phase.
3. **The residual risk is a bound, not a shape.** The destination now rests on
   canonical's benchy supports-on prepass cost being within **1.6x** of t110's
   tap value. That is a fact to verify before the rewrite is scoped, not a
   candidate to build.

## 7. Evidence

- [`reduced/board.json`](reduced/board.json) — the 64-row paired board
  (per-run walls, CPU, hashes, matched keys, tool identity, load).
- [`reduced/instrumented.json`](reduced/instrumented.json) — the split: three
  reduced passes per cell, per-pass shares, the median share block, stage
  brackets, per-module thread sums and busy unions.
- [`reduced/arithmetic.json`](reduced/arithmetic.json) — the substitution and
  its summary.
- [`reduced/split-delta.json`](reduced/split-delta.json) — the t154 diff.
- [`reduced/load-samples.csv`](reduced/load-samples.csv) — the load trace.
- `python verify.py` → **PASS** (10 claim groups, every number re-derived from
  the committed set); `python verify.py --self-test` → **16/16** negative
  controls rejected.
