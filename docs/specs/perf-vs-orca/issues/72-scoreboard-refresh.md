# Matched-pair scoreboard refresh

Type: task
Status: resolved (2026-10-18; refreshed scoreboard measured — Orca still wins
all 8 cells at 3.30x–18.03x, no cell within ~1.5x; supports-on taint retired
at the output level; two infrastructure evictions disclosed and recovered;
see ## Answer)
Assignee: this session (2026-10-17, claimed on creation from the human's route
decision — the frontier's other child
[Consume-only-when-read annotations](20-consume-only-when-read-annotations.md)
stays parked below-fold)
Blocked by: none (measurement-only; the t14 DEV-174 repair untainted the four
supports-on cells, and every structural prerequisite of the old rig is resolved)

## Question

Re-derive the per-cell gap between PNP and OrcaSlicer on the matched job at
current HEAD: refresh the 2026-09-22 scoreboard
([Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md))
— 8 cells × {Orca, PNP ordinary, PNP accelerated} under the unchanged
matched-pair protocol (1 warmup + 3 measured runs per tool per cell, rotating
tool order, per-run output-evidence validation, starvation-flagged medians,
`RAYON_NUM_THREADS=12`).

Why now: the standing scoreboard (`evidence/matched-pair/SCOREBOARD.md`,
2026-09-22) predates the DEV-174 support-correctness repair — its four
supports-on cells are marked **TAINTED, not won or lost** — and predates every
wall-moving keep since (27, 32, 44, 58, 65, 67, 70 and the small t19 step).
No fresh Orca-side numbers exist on the map; the last full-gap ledger fact on
record is the 2026-09-22 table (6.3x–26.24x, Orca winning all 8).

Deliverables:

1. Refreshed scoreboard document (this evidence set, in-repo reduced set per
   the map's evidence policy): per-cell medians (wall/CPU, cpu/wall per
   sample, retained-vs-starved counts), gaps against Orca in both PNP modes,
   accelerated/ordinary ratio, and the output disclosure (bytes, TYPE/FEATURE
   counts, degraded flag, non-fatals) — with the taint status of the four
   supports-on cells re-derived post-repair, not assumed.
2. Committed rows CSVs so a plain checkout re-derives the medians (mirroring
   the t11 `results/` precedent; raw captures stay under gitignored
   `target/matched-pair/`).
3. The [Gap budget per cell](12-gap-budget-per-cell.md) below-fold park
   condition ("a cell lands within ~1.5x of Orca") re-derived as a ledger
   fact — the route decision that gates lifting the park on ticket 20.

Constraints: no candidate, no production change, no keep/drop campaign, no
auto-commit of production sources; measurement-only take under the standing
scoreboard protocol. The 2026-09-22 SCOREBOARD.md is history — not edited,
superseded by a sibling document.

Evidence: `evidence/scoreboard-refresh-20261017/` (populated on completion;
batches `refresh-20261017-benchy` / `refresh-20261017-base`, summarizer output
+ rows CSVs + verification note).
## Answer

**Resolved 2026-10-18 as a measurement-only take — refreshed scoreboard
captured at HEAD `f067efa2` (slicer sources = the t70 keep `7127b4d6`), no
candidate, no production change, no keep/drop campaign.**

Headline (median wall s, n=3, zero starvation exclusions, all 72 measured
samples retained; full detail in
[`evidence/scoreboard-refresh-20261017/SCOREBOARD.md`](../evidence/scoreboard-refresh-20261017/SCOREBOARD.md)):

| Cell | Orca | PNP ord | PNP acc | gap ord | gap acc |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 4.27 | 21.96 | 18.52 | 5.14x | 4.34x |
| benchy arachne-off | 4.45 | 14.65 | 14.71 | 3.30x | 3.31x |
| benchy classic-on | 4.73 | 27.73 | 24.06 | 5.87x | 5.09x |
| benchy arachne-on | 4.38 | 20.87 | 20.30 | 4.76x | 4.63x |
| base classic-off | 18.84 | 216.28 | 173.41 | 11.48x | 9.20x |
| base arachne-off | 18.06 | 133.42 | 130.63 | 7.39x | 7.24x |
| base classic-on | 24.38 | 439.66 | 408.35 | 18.03x | 16.75x |
| base arachne-on | 23.78 | 372.16 | 368.58 | 15.65x | 15.50x |

1. **Destination status:** Orca still wins all 8 cells, 3.30x–18.03x on median
   uninstrumented wall; process CPU corroborates the ordering in every cell
   (PNP burns 4.5x–41.5x Orca's CPU). **No cell is within ~1.5x of Orca** —
   the [Gap budget per cell](12-gap-budget-per-cell.md) below-fold park
   condition is re-derived as a ledger fact and still binding. Best cell is
   benchy arachne supports-off at 3.30x.
2. **Change since 2026-09-22:** PNP ordinary wall improved in 7 of 8 cells
   (−13.9 % to −34.6 %; benchy classic-off +5.4 %, the one cell no
   wall-moving keep touched), consistent with the t27/t58/t65/t67/t70 keeps.
   Orca also measured slower today (+1.5 % base arachne-off to +45–47 % on
   benchy, where fixed GUI startup dominates a 3–5 s run), so part of the raw
   gap improvement is machine-state drift — disclosed, not claimed as PNP
   work. Ratios were compared today-vs-today; sample spreads are tight
   (Orca ≤ 1.2 s absolute on base, PNP arms ≤ 2.6 % wall).
3. **DEV-174 taint retired at the output level:** all 16 PNP tool groups
   report `degraded=false`, `non_fatal=0` in every measured run — the four
   supports-on cells are untainted in a scoreboard revision for the first
   time. PNP ordinary and accelerated G-code bytes are identical in all 8
   cells; Orca bytes and per-cell TYPE/FEATURE sections are disclosed in the
   evidence set.
4. **Protocol:** unchanged t11 rig (1 warmup + 3 measured per tool per cell,
   rotating order in interleaved batches, `RAYON_NUM_THREADS=12`, per-run
   config/dispatch/completion validation, starvation rule with **zero
   exclusions**). Both guest freshness gates exit 0 in every batch preflight;
   module diagnose pass=true 24/24 external both standing trees.
5. **Interruptions (disclosed):** the host evicted two long background
   batches — `refresh-20261017-base` at 17:20 mid `arachne-off` m3 and
   `refresh-20261017-base-rest` at ≈00:13 mid `classic-on` m2. Completed cell
   groups were kept; every interrupted cell was re-run whole as per-tool
   recovery chunks (never stitched from partial rounds); the evicted partial
   CSVs are preserved in the evidence set as `.interrupted.csv`. The two
   chunk-assembled cells (base classic-on/arachne-on) carry a disclosed
   protocol deviation: their tool arms were measured in separate windows, and
   their internal spreads bound the drift.
6. **Evidence:** `evidence/scoreboard-refresh-20261017/` — `SCOREBOARD.md`,
   `SUMMARY.txt` (full per-run dump), nine curated row CSVs,
   `REPRODUCE.md`, and `verify.py` (checkout-only; re-derives all 24 medians
   and 8 gap ratios from the committed CSVs, exit 0; `--self-test` proves it
   detects a perturbed expectation).

No production edit, no candidate, no adoption/default change. Route decision
returns to the human: [Consume-only-when-read annotations](20-consume-only-when-read-annotations.md)
remains the only open child, parked below-fold with its park condition
re-confirmed.
