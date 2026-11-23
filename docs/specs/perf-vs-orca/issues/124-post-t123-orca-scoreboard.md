# Post-t123 Orca scoreboard: re-derive the eight matched-cell ratios

Type: task / measurement
Label: wayfinder:task
Status: resolved (2026-11-07; measurement-only — **PnP loses all 8 cells at
  2.36x–4.65x ordinary / 2.40x–4.67x accelerated**, zero starvation exclusions,
  all output/provenance/oracle gates green; required cut to beat Orca
  **58%–79%** of today's PnP wall, **72%–86%** for a 1.5x margin. Every cell's
  ratio improved against t121's pre-t123 2.78x–5.11x, but the move mixes both
  arms — see ## Answer)
Assignee: 2026-11-07 OpenCode wayfinder session — the human said "Continue the perf vs orca map, pick up the next afk ticket" (no ticket named; 117 is the human's grilling, so 124 is the first AFK frontier ticket in order)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 123](123-facet-bridge-path-retirement.md)'s BUILD
  (2026-11-07). Ticket 123 moved the PnP side of every matched cell — median
  wall **−2.73 % … −22.23 %** — so every ratio in
  [ticket 121](121-post-merge-per-layer-dispatch-regression.md)'s scoreboard is
  pre-t123.

## Question

What are the eight matched-cell PnP-vs-Orca ratios after ticket 123, under the
standing pinned rig, and which cells are still lost?

## Scope

1. Rebuild both PnP arms from the current tree and run the standing
   matched-pair scoreboard: `{benchy, base} × {classic, arachne} × {supports
   off, on}` × {Orca, PnP ordinary, PnP accelerated}, one warmup + three
   measured, interleaved. Follow
   [`evidence/matched-pair/run_scoreboard.ps1`](../evidence/matched-pair/run_scoreboard.ps1)
   and the t121 scoreboard's method; the Orca rig is the repaired
   version-gated vendor tree from
   [`evidence/orca-rig-repair/`](../evidence/orca-rig-repair/FINDINGS.md),
   with the t103 pins and the t105 `tree_strong` support-style pin asserted
   per run.
2. Keep the standing proof gates: 16/16 PnP groups `degraded=false` /
   `non_fatal=0`, PnP ordinary ≡ accelerated bytes in all eight cells, both
   guest-freshness checks exit 0, and the three pinned values
   (`enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`,
   `brim_width = 0`) plus the filament leg asserted on every Orca run.
3. Record the eight ratios, the per-cell medians, the required cut to reach
   ~1.5x, and the adhesion asymmetry disclosure (PnP `brim_width = 8` vs
   pinned Orca `0`). Do not quote a ratio from before t123.

## Acceptance

- Fresh scoreboard from today's tree, no production edit.
- All proof gates above pass; any starvation exclusion is named, and any
  spoiled cell is preserved rather than silently averaged.
- The scoreboard, CSV and verifier are committed under
  `evidence/t124-post-t123-scoreboard/`; `verify.py --self-test` rejects its
  negative controls and exits 0 checkout-only.
- The map's destination status is updated: how many of the eight cells PnP
  wins, and the required cut if it does not.

## Evidence starts

- [`evidence/t121-per-layer-dispatch/SCOREBOARD.md`](../evidence/t121-per-layer-dispatch/SCOREBOARD.md)
  — the current scoreboard of record (pre-t123).
- [`evidence/t121-per-layer-dispatch/summary.json`](../evidence/t121-per-layer-dispatch/summary.json)
  — per-cell ordinary/accelerated walls and hashes.
- [`evidence/matched-pair/run_scoreboard.ps1`](../evidence/matched-pair/run_scoreboard.ps1)
  and [`evidence/orca-rig-repair/prepare_orca_vendor.ps1`](../evidence/orca-rig-repair/prepare_orca_vendor.ps1).
- [`evidence/t123-facet-retirement/FINDINGS.md`](../evidence/t123-facet-retirement/FINDINGS.md)
  — the measured PnP wall deltas this scoreboard must reflect.

## Answer

**Resolved 2026-11-07 as a measurement. PnP loses all eight matched cells at
2.36x–4.65x (ordinary) and 2.40x–4.67x (accelerated); the destination's 8/8 is
not met.**

| cell | Orca | PnP ordinary | PnP accelerated | ordinary/Orca | accelerated/Orca | cut to beat |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.14 s | 5.77 s | 5.26 s | 2.70x | 2.46x | 63% |
| benchy classic-on | 2.77 s | 10.01 s | 9.74 s | 3.61x | 3.52x | 72% |
| benchy arachne-off | 2.14 s | 5.34 s | 5.37 s | 2.50x | 2.52x | 60% |
| benchy arachne-on | 2.77 s | 9.96 s | 9.61 s | 3.59x | 3.47x | 72% |
| base classic-off | 14.44 s | 42.53 s | 41.06 s | 2.95x | 2.84x | 66% |
| base classic-on | 27.28 s | 126.27 s | 121.31 s | 4.63x | 4.45x | 78% |
| base arachne-off | 17.45 s | 41.18 s | 41.90 s | 2.36x | 2.40x | 58% |
| base arachne-on | 26.24 s | 122.10 s | 122.50 s | 4.65x | 4.67x | 79% |

**Acceptance evidence:**
- Fresh scoreboard from the current tree, **no production edit**. Both PnP arms
  hash to t123's candidate artifacts (`7737a74f…` ordinary, `a6ae2adc…`
  accelerated) and `git diff c598f098..HEAD` touches docs only.
- **Proof gates:** 96 rows = 8 cells × 3 tools × (1 warmup + 3 measured); PnP
  ordinary ≡ accelerated bytes in 8/8; every PnP cell hash equals the t123
  candidate set; 64/64 PnP runs `degraded=false` / `non_fatal=0`; 32/32 Orca
  runs resolve `enable_arc_fitting = disabled`,
  `elefant_foot_compensation = 0`, `brim_width = 0` and the repaired filament
  leg (`1.24 / 12 / 220`); both guest-freshness checks exit 0. **Zero
  starvation exclusions** (all 24 groups n=3).
- **Spoiled cell preserved, not averaged:** the first batch was killed by the
  host mid-cell (79 of 96 runs); its `base classic-on` partial is recorded in
  `discarded_partial_sources` and the cell was re-taken in its own interleaved
  batch, as was `base arachne-on`. Each cell is internally paired, so no ratio
  spans windows.
- Committed under
  [`evidence/t124-post-t123-scoreboard/`](../evidence/t124-post-t123-scoreboard/FINDINGS.md)
  (`verify.py --self-test` exits 0 checkout-only, 10/10 negative controls
  rejected).
- The map's destination status now records **0 of 8 cells won** and the
  required cut.

**Reading:** every ratio improved against t121's pre-t123
**2.78x–5.11x**; the base supports-off PnP medians fell **−24.2% / −25.7%**,
consistent with t123's paired **−22.2% / −20.9%**. The Orca side also moved
across the windows (**−12.3% … +11.5%**), so the cross-take comparison mixes
both arms and **no causal claim is taken from it** — t123's interleaved A/B
remains the causal measurement. The standing adhesion asymmetry (PnP 8 mm brim
vs Orca 0) is disclosed. Full detail:
[`SCOREBOARD.md`](../evidence/t124-post-t123-scoreboard/SCOREBOARD.md).
