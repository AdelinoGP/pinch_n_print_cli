# Matched-pair scoreboard — 2026-10-25 (post-resolution-port HEAD)

Route: [ticket 87](../../issues/87-scoreboard-refresh-post-resolution-port.md).
Measured at HEAD `6e71c1a1` (slicer sources = the t85 keep `7ea7f056` + the
t86 shim fix `4b1133f8`). **This is the first fully resolution-matched
scoreboard**: PnP now implements OrcaSlicer's slicing-time `resolution`
simplification (t81), so the pre-t81 fairness-counterfactual column is not
applied to these numbers and is not carried forward.

Both standing measurement trees were rebuilt from clean HEAD before the
campaign (`cargo xtask dist --edition developer`,
`cargo xtask dist --accelerated --edition developer`). The binaries on disk
predated the t85 code commit and were replaced — no campaign arm was reused.

## Headline (median uninstrumented wall, s; n=3 per group)

| Cell | Orca | PnP ordinary | PnP accelerated | gap ord | gap acc |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.79 | 7.68 | 7.56 | **2.75x** | **2.71x** |
| benchy arachne-off | 2.88 | 7.90 | 7.89 | **2.74x** | **2.74x** |
| benchy classic-on | 3.53 | 12.30 | 12.33 | **3.48x** | **3.49x** |
| benchy arachne-on | 3.64 | 12.58 | 12.47 | **3.45x** | **3.42x** |
| base classic-off | 13.87 | 73.17 | 72.92 | **5.28x** | **5.26x** |
| base arachne-off | 14.14 | 73.25 | 73.23 | **5.18x** | **5.18x** |
| base classic-on | 18.37 | 171.07 | 171.71 | **9.31x** | **9.35x** |
| base arachne-on | 19.93 | 186.63 | 183.34 | **9.36x** | **9.20x** |

**Destination status: Orca still wins all 8 cells.** The best cell is
**benchy classic-off at 2.71x accelerated / 2.75x ordinary**; the worst is
**base arachne-on at 9.36x ordinary**. No cell is within ~1.5x of Orca, so
[Gap budget per cell](../../issues/12-gap-budget-per-cell.md)'s below-fold park
condition is re-derived as a ledger fact and **still binding**. The park's
release bound has moved from 3.30x (2026-10-18) to 2.71x — real progress, but
still short of 1.5x.

**Supports-on is now the whole problem at scale.** Base supports-off sits at
5.18–5.28x; base supports-on at 9.20–9.36x. Benchy shows the same shape:
2.71–2.75x off versus 3.42–3.49x on. Every supports-on cell is the worse of its
pair, on both fixtures and both modes.

## CPU corroboration (median, s) and per-sample cpu/wall

| Cell | Orca cpu | ord cpu | acc cpu | cpu/wall ord | cpu/wall acc | PnP/Orca cpu |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 9.27 | 49.70 | 48.66 | 6.46–6.49 | 6.44–6.47 | 5.4x / 5.3x |
| benchy arachne-off | 10.50 | 50.58 | 50.59 | 6.25–6.53 | 6.32–6.49 | 4.8x / 4.8x |
| benchy classic-on | 14.36 | 65.41 | 65.30 | 5.25–5.32 | 5.22–5.43 | 4.6x / 4.5x |
| benchy arachne-on | 16.03 | 66.64 | 66.44 | 5.28–5.34 | 5.31–5.37 | 4.2x / 4.1x |
| base classic-off | 33.20 | 708.59 | 704.50 | 9.65–9.74 | 9.65–9.73 | 21.3x / 21.2x |
| base arachne-off | 36.56 | 714.17 | 711.19 | 9.69–9.76 | 9.71–9.75 | 19.5x / 19.5x |
| base classic-on | 58.48 | 1034.34 | 1031.16 | 5.97–6.10 | 5.95–6.15 | 17.7x / 17.6x |
| base arachne-on | 65.00 | 1075.62 | 1071.00 | 5.74–5.77 | 5.83–5.87 | 16.5x / 16.5x |

Process CPU corroborates the wall ordering in every cell: PnP burns **4.1x–21.3x**
Orca's CPU, so no cell is a wall win hiding a CPU regression or vice versa.

## Output disclosure

- **PnP 16/16 groups: `degraded=false`, `non_fatal=0`.** Every measured run in
  every cell.
- **PnP ordinary ≡ PnP accelerated, byte for byte, in all 8 cells** (identical
  G-code bytes and identical SHA per cell). One SHA per group; every group's
  three measured runs and its warmup emit the same SHA.
- **New post-t81 reference set** (one SHA per PnP cell, both modes):

  | Cell | SHA-256 |
  | --- | --- |
  | benchy classic-off | `3672dc7e1649cab69a0ffab2ebd293233a9af0504d0f98666746da933d8c641d` |
  | benchy arachne-off | `5158539849b0ed1b6b1fb89a9f222480cfb30e68027ee4c284e2f60505eda85b` |
  | benchy classic-on | `eb4817b4be28c0a30e906bed9d960cd59282fa20d95afb509e48c8e39ab4b714` |
  | benchy arachne-on | `6b8a0074ca7ac4e8142af764dce669495bd411b45ad47f94a29dc57451146e05` |
  | base classic-off | `292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80` |
  | base arachne-off | `01a1434c6ab5bbe965e37985e78137b739b47dccf2d364a5d3dbd0156cad7e97` |
  | base classic-on | `22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d` |
  | base arachne-on | `0b29e1bf928f1c0a160ebdedf08245edfd1d0433b3807ad1118b4578a378981c` |

  The four base SHAs reproduce the post-t81 references used by t83 (`01a1434c…`),
  t84 (`292df501…` / `22200671…`) and t85 — an independent consistency check
  that the rebuilt HEAD tree is the tree those takes measured.
- Orca's G-code bytes vary slightly run-to-run in 6 of 8 cells (e.g. base
  classic-on `37,408,960 / 37,409,049 / 37,471,566`); its wall is unaffected.
  Per-cell TYPE/FEATURE section counts for the median run are in `SUMMARY.txt`.
- Output is **not** byte-comparable between engines (t81 changed PnP geometry by
  design; the engines also differ in arc fitting and support structure). The
  disclosure is the fairness contract, not a parity claim.

## Change since the 2026-10-18 scoreboard

Today-vs-today ratios against [t72](../../issues/72-scoreboard-refresh.md).
PnP walls fell much more than Orca's, but **Orca also measured 16.2–35.3 %
faster today** — machine-state drift, disclosed, not claimed as PnP work.

| Cell | Orca | PnP ord | PnP acc | gap ord | gap acc |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 4.27 → 2.79 (−34.7 %) | 21.96 → 7.68 (−65.0 %) | 18.52 → 7.56 (−59.2 %) | 5.14 → 2.75 | 4.34 → 2.71 |
| benchy arachne-off | 4.45 → 2.88 (−35.3 %) | 14.65 → 7.90 (−46.1 %) | 14.71 → 7.89 (−46.4 %) | 3.30 → 2.74 | 3.31 → 2.74 |
| benchy classic-on | 4.73 → 3.53 (−25.4 %) | 27.73 → 12.30 (−55.6 %) | 24.06 → 12.33 (−48.8 %) | 5.87 → 3.48 | 5.09 → 3.49 |
| benchy arachne-on | 4.38 → 3.64 (−16.9 %) | 20.87 → 12.58 (−39.7 %) | 20.30 → 12.47 (−38.6 %) | 4.76 → 3.45 | 4.63 → 3.42 |
| base classic-off | 18.84 → 13.87 (−26.4 %) | 216.28 → 73.17 (−66.2 %) | 173.41 → 72.92 (−57.9 %) | 11.48 → 5.28 | 9.20 → 5.26 |
| base arachne-off | 18.06 → 14.14 (−21.7 %) | 133.42 → 73.25 (−45.1 %) | 130.63 → 73.23 (−43.9 %) | 7.39 → 5.18 | 7.24 → 5.18 |
| base classic-on | 24.38 → 18.37 (−24.7 %) | 439.66 → 171.07 (−61.1 %) | 408.35 → 171.71 (−58.0 %) | 18.03 → 9.31 | 16.75 → 9.35 |
| base arachne-on | 23.78 → 19.93 (−16.2 %) | 372.16 → 186.63 (−49.9 %) | 368.58 → 183.34 (−50.3 %) | 15.65 → 9.36 | 15.50 → 9.20 |

The keeps that landed since t72 and are visible here: t74 (finalization view
cost), t75 (ShellClassification phase-B), t76 (MeshAnalysis union), t20
(consume-only annotations — both perimeter generators), t78 (tree-support
emit batching), t81 (resolution port), t85 (seam batching), plus t65's linker
batch on the Arachne cells. The two base supports-off cells moved the most in
relative terms because t20 and t81 both hit them and they carry no support
stages.

## Protocol and integrity

- **Rig:** the unchanged ticket-11
  [`run_scoreboard.ps1`](../matched-pair/run_scoreboard.ps1) /
  [`summarize_scoreboard.ps1`](../matched-pair/summarize_scoreboard.ps1).
  1 warmup + 3 measured runs per tool per cell, tools interleaved and rotated
  within each cell, `RAYON_NUM_THREADS=12`, uninstrumented.
- **Batches:** one per cell so the three tools share one measurement window:
  `refresh-20261025-benchy2`, `refresh-20261025-base-classic-off`,
  `refresh-20261025-base-arachne-off`, `refresh-20261025-base-classic-on`,
  `refresh-20261025-base-arachne-on`.
- **Rows:** 96 total, 72 measured (8 cells × 3 tools × 3). Every group has
  exactly 1 warmup + 3 measured.
- **Starvation:** the rig's rule (`cpu/wall < 0.75 ×` group best) flags nothing;
  **zero exclusions** in all 24 groups.
- **Validation:** per-run PnP validation (config, `slice_complete`,
  dispatch) and Orca appendix validation all passed — a failure aborts a batch,
  and all five exited 0.
- **Freshness:** both guest gates exit 0 before the campaign; both trees
  rebuilt from clean HEAD; `module diagnose` pass with 24/24 external modules
  for both.
- **Machine:** quiet at start (1.5–3.2 % total CPU), no Orca/PNP/cargo
  processes running.

## Disclosures

1. **A duplicate benchy round exists and is not ledger input.** The first
   benchy invocation was launched through a shell pipeline that failed to
   resolve `Tee-Object`; bash returned 127 but the already-started PowerShell
   process kept running, and a second invocation then ran the same batch ID
   again. `rows-benchy-duplicate-round.csv` preserves both complete rounds.
   They agree closely — PnP ordinary classic-off 7.18 vs 7.22 s, Orca 2.76 vs
   2.73 s, arachne-off 7.39 vs 7.37 s, classic-on 11.67 vs 11.66 s, arachne-on
   11.93 vs 11.86 s — so the duplicate is corroborating, not contradictory.
   The ledger uses the clean re-run `refresh-20261025-benchy2` (7.68 / 7.90 /
   12.30 / 12.58 s PnP ordinary), which is 4–6 % slower than both duplicate
   rounds while its Orca arm is only 1–2 % slower, i.e. the benchy cells carry
   the largest machine-state sensitivity of the eight. **Benchy between-round
   uncertainty is therefore larger than base's**: the three complete benchy
   rounds put PnP ordinary classic-off at 7.18 / 7.22 / 7.68 s (7.0 % span)
   against Orca at 2.76 / 2.73 / 2.79 s (2.2 % span), whereas each base cell
   was measured in one window whose three samples span 0.2–3.6 % on PnP.
2. **Orca-side machine drift.** Every Orca cell measured 16.2–35.3 % faster than
   the 2026-10-18 table. t72 disclosed drift in the opposite direction
   (+1.5 %…+47 %). Ratios are today-vs-today and are not a PnP-work claim.
3. **The base supports-on cells are the long poles.** Within-batch wall
   spreads: accelerated classic-on 167.8–174.0 s (3.6 %), accelerated
   arachne-on 183.2–183.6 s (0.2 %). The Orca arachne-on group carries one cold
   first sample (21.98 s vs 19.23 / 19.93 s; 13.8 % spread, median 19.93 s),
   retained by the rig's starvation rule and disclosed here.
4. **No candidate, no production edit, no keep/drop campaign.** Measurement-only
   take under the standing protocol.

## Reproduce

```bash
python docs/specs/perf-vs-orca/evidence/scoreboard-refresh-20261025/verify.py
python docs/specs/perf-vs-orca/evidence/scoreboard-refresh-20261025/verify.py --self-test
```

`verify.py` reads only the committed row CSVs and re-derives all 24 medians, the
16 wall-gap ratios, sample counts, the zero-starvation state, the PnP
output-flags and the ordinary≡accelerated byte identity. The batch commands are
in `REPRODUCE.md`.
