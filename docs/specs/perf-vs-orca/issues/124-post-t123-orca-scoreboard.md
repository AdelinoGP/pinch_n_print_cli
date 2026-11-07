# Post-t123 Orca scoreboard: re-derive the eight matched-cell ratios

Type: task / measurement
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
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
