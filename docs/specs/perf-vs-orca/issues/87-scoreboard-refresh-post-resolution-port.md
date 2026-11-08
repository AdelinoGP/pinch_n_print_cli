# Matched-pair scoreboard refresh at post-resolution-port HEAD

Type: task
Label: wayfinder:task
Status: resolved (2026-10-25; measurement-only scoreboard refresh — Orca still
wins all 8 cells at 2.71x–9.36x; best cell moves 3.30x → 2.71x, so the ~1.5x
below-fold park stays binding; supports-on is now the concentration and
graduates [ticket 88](88-supports-on-increment-attribution.md); see ## Answer)
Assignee: 2026-10-25 OpenCode wayfinder session — the human's "continue the
perf vs orca map" named no ticket, the frontier re-derivation found 86/86
children resolved, and the human selected **Scoreboard refresh** from the
route options
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none (measurement-only; both guest freshness gates exit 0 and both
standing measurement trees are rebuilt from clean HEAD in this take)
Graduated from: [Matched-pair scoreboard refresh](72-scoreboard-refresh.md)
(the 2026-10-18 table this take supersedes)

## Question

What is the per-cell gap between PNP and OrcaSlicer on the matched job **at
current HEAD**, now that the job is fully resolution-matched and eight keeps
have landed since the 2026-10-18 table?

Why now:

1. **The last scoreboard predates every wall-moving keep of the last week.**
   [t72](72-scoreboard-refresh.md) measured on 2026-10-17/18 at `f067efa2`.
   Since then: [t74](74-layerfinalization-supports-on-view-cost.md) (KEEP),
   [t75](75-shellclassification-phase-b-commit.md) (KEEP),
   [t76](76-meshanalysis-footprint-union.md) (KEEP),
   [t20](20-consume-only-when-read-annotations.md) (KEEP),
   [t78](78-tree-support-planner-emit-batching.md) (KEEP),
   [t81](81-resolution-slicing-time-contour-simplification.md) (KEEP, a
   **geometry-changing** port that cut classic walls 9–22 %),
   [t85](85-seam-inset-offset-batching.md) (KEEP, −2.7…−7.5 % on the two base
   classic cells) and [t65](65-infill-linker-batch-execute-experiment.md)
   (KEEP). The t72 table cannot be carried forward: its absolute walls were
   taken under a different machine state (it disclosed Orca-side drift of
   +1.5 %…+47 %), so no arithmetic on its numbers is a scoreboard.
2. **The park condition is a ledger fact that only a refresh can re-derive.**
   [Gap budget per cell](12-gap-budget-per-cell.md)'s below-fold park lifts
   when "a cell lands within ~1.5x of Orca". Carry-forward arithmetic on mixed
   era numbers (not evidence, not to be quoted) puts the best cell near 2.1x
   rather than t72's 3.30x, so the park may be close to unblocking — or the
   arithmetic may be wrong. The refresh decides.
3. **This is the first fully resolution-matched scoreboard.** t80 disclosed
   that every pre-t81 ratio compared PnP at full contour detail against an
   Orca doing materially less geometric work; the t80/t81 correction factor
   applies to *pre-t81* numbers only. New numbers need no counterfactual
   column, and the map's fairness contract can drop the disclosure for them.

## Protocol (pre-declared, unchanged from t72 / ticket 11)

- 8 cells = {benchy, base} × {classic, arachne} × {supports off, on} ×
  3 tools = {Orca, PNP ordinary, PNP accelerated}.
- **1 warmup + 3 measured runs per tool per cell**, tools interleaved within a
  cell so each cell's three arms share one measurement window.
- `RAYON_NUM_THREADS=12`; uninstrumented runs only (instrumented walls are
  attribution-only and are never quoted as wall, `docs/17_agent_debugging.md`).
- Per-run validation via the rig's own gates: PNP runs through
  `resources/perimeter-acceptance/validate_measurement.ps1` (config,
  `slice_complete` counters, dispatch check); Orca runs against the resolved
  G-code appendix (`wall_generator`, `wall_loops`, `sparse_infill_density`,
  `sparse_infill_pattern`, `enable_support`, `support_type`, `layer_height`).
  Any validation failure aborts the batch.
- **Starvation rule (the rig's own, unchanged since ticket 11):** within a
  (fixture, cell, tool) group a sample is excluded when
  `cpu/wall < 0.75 × the group's best cpu/wall` (`PERF-HANDOFF` §10.3, as
  implemented in `summarize_scoreboard.ps1`). Excluded samples are still
  printed and never deleted; medians are over retained samples, and the
  retained/starved counts are disclosed per group.
- Verdict metric: median uninstrumented wall; process CPU must corroborate
  (never contradict). Rows carry cpu/wall per sample.
- Output disclosure per cell: G-code bytes, TYPE counts, `degraded` flag,
  `non_fatal` count; PNP ordinary vs accelerated bytes; Orca appendix presence.
  A PNP group with `degraded=true` or `non_fatal>0` is disclosed, not hidden.
- **By-product, not a parity claim:** record each PNP tool group's G-code SHA
  (one SHA per group expected) as the post-t81 reference set for future
  tickets. No byte-identity is claimed against any pre-t81 reference, because
  t81 changed geometry by design.

## Deliverables

1. `evidence/scoreboard-refresh-20261025/SCOREBOARD.md` — per-cell medians
   (wall/CPU, cpu/wall per sample, retained-vs-starved counts), gaps against
   Orca in both PNP modes, accelerated/ordinary ratio, output disclosure, and
   the t81-era status of each cell.
2. Committed rows CSVs so a plain checkout re-derives every median and gap.
   Raw captures stay under the gitignored `target/matched-pair/<BatchId>/`.
3. `verify.py` — checkout-only, exits 0 against the committed CSVs alone,
   re-deriving all medians and 8 gap ratios, with `--self-test` negative
   controls rejecting perturbed expectations.
4. `REPRODUCE.md` — the exact batch commands as run.
5. The **~1.5x below-fold park condition re-derived as a ledger fact** in the
   ticket's Answer and on the map.

## Constraints

Measurement-only: **no candidate, no production edit, no keep/drop campaign,
no auto-commit of production sources.** The map's standing paired-acceptance
and KEEP/DROP rules are unchanged. A batch eviction or machine-state anomaly is
disclosed and the affected cell re-run whole — never stitched from partial
rounds.

## Answer

**Resolved 2026-10-25 as a measurement-only take — no candidate, no production
change, no keep/drop campaign.** Measured at HEAD `6e71c1a1` (slicer sources =
the t85 keep `7ea7f056` + the t86 shim fix `4b1133f8`), both standing trees
rebuilt from clean HEAD before the campaign. Full detail:
[`evidence/scoreboard-refresh-20261025/SCOREBOARD.md`](../evidence/scoreboard-refresh-20261025/SCOREBOARD.md).

| Cell | Orca | PNP ord | PNP acc | gap ord | gap acc |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.79 | 7.68 | 7.56 | 2.75x | **2.71x** |
| benchy arachne-off | 2.88 | 7.90 | 7.89 | 2.74x | 2.74x |
| benchy classic-on | 3.53 | 12.30 | 12.33 | 3.48x | 3.49x |
| benchy arachne-on | 3.64 | 12.58 | 12.47 | 3.45x | 3.42x |
| base classic-off | 13.87 | 73.17 | 72.92 | 5.28x | 5.26x |
| base arachne-off | 14.14 | 73.25 | 73.23 | 5.18x | 5.18x |
| base classic-on | 18.37 | 171.07 | 171.71 | 9.31x | 9.35x |
| base arachne-on | 19.93 | 186.63 | 183.34 | **9.36x** | 9.20x |

1. **Destination status: Orca still wins all 8 cells, 2.71x–9.36x.** Process
   CPU corroborates the ordering in every cell (PnP burns 4.1x–21.3x Orca's
   CPU). **No cell is within ~1.5x**, so the
   [Gap budget per cell](12-gap-budget-per-cell.md) below-fold park condition is
   re-derived as a ledger fact and **still binding**; the release bound has
   moved from 3.30x to 2.71x.
2. **Supports-on is now the concentration.** Base supports-off is 5.18–5.28x,
   base supports-on 9.20–9.36x; every supports-on cell is the worse of its
   pair. The base increment is **+97.90 s / +113.38 s** for PnP against
   **+4.50 s / +5.79 s** for Orca (21.8x / 19.6x), while PnP emits **fewer**
   support FEATURE sections than Orca. This graduates
   [ticket 88](88-supports-on-increment-attribution.md) (open, unclaimed).
3. **Change since 2026-10-18:** PnP ordinary wall fell **39.7–66.2 %** in all
   eight cells (t74/t75/t76/t20/t78/t81/t85 + t65 on Arachne), while **Orca
   also measured 16.2–35.3 % faster today** — machine-state drift, disclosed,
   not claimed as PnP work. Ratios are today-vs-today.
4. **This is the first fully resolution-matched scoreboard.** t80's
   fairness-counterfactual correction applies to pre-t81 numbers only and is
   not carried into this table.
5. **Output disclosure:** PnP 16/16 groups `degraded=false`, `non_fatal=0`;
   PnP ordinary ≡ accelerated **byte for byte in all 8 cells**; one SHA per
   PnP group. The new post-t81 reference set is recorded in `SCOREBOARD.md`;
   the four base SHAs reproduce the references used by t83 (`01a1434c…`), t84
   (`292df501…` / `22200671…`) and t85 — an independent consistency check on the
   rebuilt tree.
6. **Protocol:** 96 rows (72 measured), 8 cells × 3 tools, 1 warmup + 3
   measured per group, tools interleaved and rotated within each cell,
   five per-cell batches, `RAYON_NUM_THREADS=12`, **zero starvation
   exclusions**, zero validation failures, both freshness gates exit 0 and both
   trees at 24/24 external modules before the campaign, machine quiet at start.
7. **Disclosures:** (a) an accidental duplicate benchy round caused by a failed
   shell pipeline is preserved as `rows-benchy-duplicate-round.csv` and is
   **not** ledger input; the two duplicate rounds agree within ~1 %, while the
   clean re-run that is ledger input is 4–6 % slower on PnP and 1–2 % slower on
   Orca, so **benchy between-round uncertainty is larger than base's** (base
   within-batch PnP spreads are 0.2–3.6 %); (b) Orca-side drift as in point 3;
   (c) the Orca base arachne-on group carries one cold first sample (21.98 s vs
   19.23 / 19.93 s; median 19.93 s), retained by the rig's starvation rule and
   disclosed.

Evidence: [`evidence/scoreboard-refresh-20261025/`](../evidence/scoreboard-refresh-20261025/)
(`SCOREBOARD.md`, `SUMMARY.txt`, five ledger row CSVs, the preserved duplicate
round, `REPRODUCE.md`, `verify.py` — checkout-only exit 0, `--self-test` 8/8
negative controls detected).

No production edit, no candidate, no adoption/default change. Frontier now:
[ticket 88](88-supports-on-increment-attribution.md) only; the take order is the
human's.
