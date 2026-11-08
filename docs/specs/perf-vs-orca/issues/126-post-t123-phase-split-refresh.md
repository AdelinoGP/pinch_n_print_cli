# Post-t123 phase split refresh and per-layer/emit attribution

Type: task / measurement + Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-08; measurement + Phase-1 attribution — **the split is
  re-derived on all eight cells; the prepass is 62.1–87.1 %, the per-layer phase
  10.6–33.3 %, and even a zero prepass leaves benchy classic-off at 1.02x**.
  `PrePass::Slice` fell 41,472 → 2,110 ms on base classic-off; the required
  prepass removal is **72–101 %**. One exact per-layer shape is graduated
  (raft-fill dispatch outside the raft band, 61–63 ms on all 495 base layers
  while its body early-outs; ceiling 2.64x–8.25x the floor). Emit is not a
  route. See ## Answer)
Assignee: 2026-11-08 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named; all 125 children resolved), then selected
  "Refresh split, then per-layer/emit" from the offered fog routes
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the map's **Not yet specified** entry "The per-layer/emit
  program is the next stage after the prepass rewrite (t111, 2026-11-03)", plus
  the stale phase split it depends on. [ticket 104](104-prepass-vs-per-layer-attribution.md)
  measured the only phase split on 2026-11-02, before [t121](121-post-merge-per-layer-dispatch-regression.md)'s
  per-layer fix and [t123](123-facet-bridge-path-retirement.md)'s prepass cut;
  [t124](124-post-t123-orca-scoreboard.md) then measured base PnP walls
  **−20…−26 %** below t121's. [t125](125-strong-planner-occupancy-offset-batching.md)
  closed the planner occupancy batch as a DROP and explicitly left the
  per-layer emit-check half untried, with no automatic rescue status.

## Question

On today's post-t123 tree, what are the phase shares and the per-stage and
per-module composition of the eight matched cells' walls — and which per-layer
or emit shapes are priced above the 0.80 % floor once the split is re-derived
against t124's ratios of record?

## Scope

1. **Build and prove the arm.** Build `pnp_cli` release from the current tree,
   confirm both guest-freshness checks exit 0, and record the binary SHA-256.
   The arm must be the t123 candidate artifact
   (`7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0`) unless
   the tree has moved; if it has moved, say so and re-derive.
2. **One instrumented pass per cell, all eight cells.** `pnp_cli slice
   --instrument-stderr`, ordinary mode, `RAYON_NUM_THREADS=12`, the standing
   matched-pair configs (`evidence/matched-pair/configs/pnp-*.json`), 24
   external modules. t104 measured four classic cells and disclosed Arachne
   unmeasured; this take closes that gap because half the destination's cells
   are Arachne.
3. **Validate every pass:** exit 0, `degraded=false`, `fatal_error_count=0`,
   `non_fatal_error_count=0`, and G-code SHA-256 equal to the t123 candidate
   reference set (the current reference set). An instrumented wall is never a
   wall claim; shares are the product.
4. **Re-derive the split:** per cell, the four phase brackets (`validation`,
   `prepass`, `per_layer`, `postpass`), the outside-phases residual, prepass
   stage costs, postpass stage costs, per-layer stage thread sums (labelled
   thread-accumulated, never wall), per-module brackets with call counts, and —
   from the `module_start`/`module_complete` timestamps — the per-layer phase's
   module-busy union versus host-gap residual.
5. **Re-run the counterfactual arithmetic against t124** (not t103): what
   remains if the prepass is zero, if the per-layer phase is zero, and if the
   two dominators are zero, in all eight cells, with each cell's cut to beat
   Orca and to reach 1.5x recomputed from the t124 medians.
6. **Attribute the per-layer and emit terms.** Name the carrying modules and
   stages, give their accumulated cost and call counts, and price any named
   shape against 0.80 % of that cell's t124 median wall. The t125 caveat is
   binding: the untried per-layer emit-check occupancy half must not inherit
   either a win or a failure claim from the per-row role-carve batch; if it is
   named, it needs its own price.
7. **Stop rule.** If no per-layer or emit shape clears the floor, record
   measured-dead, produce no candidate ticket, and make no production edit.

## Acceptance

- Eight instrumented passes, all clean and byte-identical to the t123
  reference set (ordinary), raw streams preserved under gitignored
  `target/t126-instrumented/`.
- Shares reconcile: the four phase brackets plus the outside residual equal
  `slice_complete` within the reducer's stated tolerance.
- The counterfactual table is denominated by t124's medians, and every quoted
  share is from this take, never carried from t104.
- `evidence/t126-post-t123-phase-split/` holds `FINDINGS.md`, `REPRODUCE.md`,
  `run-instrumented.py`, `reduce.py`, `arithmetic.py`, the committed reduced
  `summary.json`, and `verify.py`; `verify.py` exits 0 checkout-only and its
  `--self-test` rejects every negative control.
- The map's destination status and **Not yet specified** fog are updated, and
  the take is recorded in **Decisions so far**.
- No production edit and no G-code reference move.

## Evidence starts

- [`evidence/t104-phase-split/`](../evidence/t104-phase-split/FINDINGS.md) —
  the only phase split, and its reducer/reproduce scripts.
- [`evidence/t124-post-t123-scoreboard/SCOREBOARD.md`](../evidence/t124-post-t123-scoreboard/SCOREBOARD.md)
  — the ratios of record this take must use.
- [`evidence/t123-facet-retirement/reduced/summary.json`](../evidence/t123-facet-retirement/reduced/summary.json)
  — the current output reference set.
- [`evidence/t125-occupancy-offset-batching/FINDINGS.md`](../evidence/t125-occupancy-offset-batching/FINDINGS.md)
  — the planner batch DROP and the untried per-layer half.
- [`evidence/t121-per-layer-dispatch/FINDINGS.md`](../evidence/t121-per-layer-dispatch/FINDINGS.md)
  — the per-layer transport fix and the module-bracket method.

## Answer

**Resolved 2026-11-08 as a measurement + Phase-1 attribution. The split is
re-derived on all eight cells against ticket 124's medians, and it re-prices the
map's next stage.**

| cell | prepass | per-layer | postpass | outside | ratio of record | if prepass 0 | if per-layer 0 | prepass needed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 62.1 % | 31.3 % | 2.9 % | 3.7 % | 2.70x | **1.02x** | 1.86x | **101 %** |
| benchy classic-on | 76.4 % | 18.5 % | 4.2 % | 0.9 % | 3.61x | 0.85x | 2.95x | 95 % |
| benchy arachne-off | 62.2 % | 33.3 % | 2.9 % | 1.5 % | 2.50x | 0.94x | 1.67x | 96 % |
| benchy arachne-on | 73.4 % | 20.9 % | 4.9 % | 0.7 % | 3.59x | 0.95x | 2.84x | 98 % |
| base classic-off | 78.3 % | 19.1 % | 2.4 % | 0.2 % | 2.95x | 0.64x | 2.38x | 84 % |
| base classic-on | 87.1 % | 10.6 % | 2.3 % | 0.1 % | 4.63x | 0.60x | 4.14x | 90 % |
| base arachne-off | 79.8 % | 18.2 % | 1.8 % | 0.2 % | 2.36x | 0.48x | 1.93x | 72 % |
| base arachne-on | 86.7 % | 10.9 % | 2.2 % | 0.1 % | 4.65x | 0.62x | 4.14x | 91 % |

**Four rulings follow directly.**

1. **The prepass is still the only phase that can reach the destination.**
   Deleting the whole per-layer phase leaves 1.67x–4.14x in every cell.
2. **The destination requires removing 72–101 % of the prepass, and benchy
   classic-off cannot be won by prepass deletion alone** — its prepass share
   (62.1 %) is below its required cut (63.0 %), so the counterfactual lands at
   1.02x. This is the map's first arithmetic evidence that the 8/8 destination
   is out of reach without a prepass rewrite (or a redraw), and it is graduated
   as [Destination arithmetic after t123: re-aim, rewrite, or park?](128-destination-arithmetic-ruling.md).
3. **`PrePass::Slice` is no longer a term:** 41,472 → 2,110 ms on base
   classic-off (−94.9 %), 2,301 → 205 ms on benchy classic-off, 44,558 → 2,117 ms
   on base classic-on. The supports-off prepass is now `ShellClassification` +
   `OverhangAnnotation` (24.4 s of a 42.5 s cell on base classic-off); the
   supports-on prepass is `SupportGeometry` + `SupportAnalysis` (68.9 s of a
   126.3 s cell on base classic-on). No single stage dominates any more.
4. **Per-layer and emit programs cannot close the map.** The per-layer carriers
   are perimeters (3.82x–15.84x floor ceiling), tree-support on supports-on
   (4.66x–9.63x), raft-default (2.64x–8.25x) and infill-linker (1.17x–8.28x).
   The postpass is 1.8–4.9 % of every cell and its largest bracket is
   0.85–2.61x floor. **One exact shape is graduated:** the raft-fill dispatch
   pays 61–63 ms on every one of the 495 base layers while its body returns
   before doing any work (`raft_layers = 2`), so declining it outside the raft
   band is output-neutral by construction —
   [Raft-fill dispatch outside the raft band](127-raft-fill-dispatch-outside-band.md).

**Acceptance evidence:**
- Eight instrumented passes, all exit 0, `degraded=false`,
  `fatal_error_count=0`, `non_fatal_error_count=0`, and every G-code SHA-256
  equal to ticket 123's candidate-ordinary reference set. Ordinary arm only.
- Both guest freshness checks exit 0; the ordinary binary is
  `7737a74fd9f7ea012681ba83e9afdcf511d067fc50a0f919cef8b877b35759b0` and
  `git diff c598f098..HEAD` touches docs only.
- Phase shares reconcile to 1 in every cell; the phase brackets plus the
  outside residual equal `slice_complete`; prepass stage brackets reconcile with
  the prepass phase within 2 % of that phase.
- The per-layer phase has **no globally idle wall**: the module-busy union
  equals the phase wall on all four base cells and leaves 2.0–2.3 % (33–44 ms,
  0.47–0.90x floor) on benchy. This bounds cross-worker idle only; a per-worker
  host cost between dispatches is not resolved by this instrument.
- Committed under
  [`evidence/t126-post-t123-phase-split/`](../evidence/t126-post-t123-phase-split/FINDINGS.md);
  `verify.py` holds 617/617 claims checkout-only and `--self-test` rejects
  10/10 negative controls.
- No production edit and no G-code reference move.

Disclosures: instrumented walls are attribution only (base classic-off +3.3 %
over its t124 median); machine load sampled 0.8–7.5 % (median ≈2.1 %); one pass
per cell, no share-stability repeat; `ratio_if_X_zero` is a bound, not a
prediction; cross-take deltas are directional only (t104's base-on pass carries
a disclosed +43.6 % instrumented inflation). Full detail:
[`FINDINGS.md`](../evidence/t126-post-t123-phase-split/FINDINGS.md).
