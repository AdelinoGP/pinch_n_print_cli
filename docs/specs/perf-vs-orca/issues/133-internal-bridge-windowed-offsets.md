# Internal-bridge offsets on a candidate-bbox operand: bounded-delta build under the 16-cell contract

Type: task / measured candidate
Label: wayfinder:task
Status: resolved (2026-11-09; **KEEP, exact** — windowing the **negative**
  `deep_infill_area` erosion only is byte-identical in all 16 output cells and
  clears the 0.80 % floor in **8/8** at 1.66x–10.94x with CPU corroborating.
  Windowing both offsets is rejected: it moves 8 benchy bridge layers by up to
  22.09 mm because `internal_bridge_angles` reads the positive clip area
  whole-layer. See ## Answer)
Assignee: 2026-11-09 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 133 is the first unblocked, unclaimed
  AFK frontier ticket in order (130 is the human's grilling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Common-stage per-op cost decomposition](131-common-stage-per-op-cost-decomposition.md)'s
  2026-11-09 S5 probe: the two whole-layer `deep_infill_area` offsets cost
  **22,964 ms** of thread-time on 130 base classic-off layers and
  **1,937 ms** on a candidate-bbox-windowed operand (16.06x / 8.81x), but the
  windowed result is not byte-identical.

## Question

Does clipping the two `deep_infill_area` offsets to the internal-bridge
candidates' bbox — the shape t129 left unpriced — clear the standing gate
under the 16-cell contract with a bounded, audited delta?

[t131](../evidence/t131-per-op-decomposition/FINDINGS.md) measured the
ceiling and the risk:

- whole-layer offsets **22,964 ms** thread-time over 130 base classic-off
  layers; windowed **1,937 ms**; wall-equivalent **2,520 ms** at the stage's
  8.34x parallelism (**7.4x** the 340 ms floor, **9.1 %** of the cell's
  required prepass cut).
- **not exact**: positive pass exact on **115/130** layers and negative on
  **36/42**; max **0.71 mm²** / **0.59 mm²** per layer, net **−1.27 mm² /
  +1.18 mm²** (**−0.039 % / +0.076 %** of the clip area). The operands are
  NonZero winding sets, so clipping them re-normalises the fill.

## Scope

1. Decide the formulation: clip the **operand** to the candidate bbox expanded
   by the offset distance (the measured shape), or find an exact window
   (clip the result — no saving — or prove a fill-normalisation-free operand).
2. Run the full qualification (`qualify_internal_bridge_surface`,
   `internal_bridge_angles`) on the windowed operand and compare the committed
   `internal_bridge_areas` / `internal_bridge_angles_deg` per layer against
   the whole-layer run.
3. Produce the 16-cell output set and the structural delta audit **before**
   timing, per t111's contract. The delta is expected to be non-zero; the
   audit must show it is confined to internal-bridge geometry and its angles.
4. Do not weaken the gate to accept the delta: the standing paired wall/CPU
   gate at 0.80 % in all eight cells applies, and a DROP is a legitimate
   outcome.

## Acceptance

- A candidate under the 16-cell contract with its delta audited, or a measured
  DROP with the non-equivalence recorded.
- The per-layer area delta re-measured in the candidate (t131's
  115/130 + 36/42 exactness and 0.71 / 0.59 mm² maxima are the screen).
- No production edit until the delta audit is signed off.

## Answer

**KEEP, and the shape is exact: window the negative erosion only. The candidate
is byte-identical in all eight cells and clears the 0.80 % floor in 8/8 at
1.66x–10.94x with process CPU corroborating in every cell.** The positive
`deep_infill_clip_area` must stay whole-layer; windowing it changes bridge
directions, which is the measured reason the original two-offset shape is
rejected.

**Formulation.** `internal_bridge_offset_window` (in
`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) returns the union
bounding box of the layer's `internal_solid_fill` candidates grown by
`(2 x 4.5 + 1) x qualify_spacing_mm`. The erosion operand becomes
`deep_infill_area ∩ window`; the erosion itself (`-4.5 x spacing`, miter,
limit 2.0) is unchanged. Both consumers of the eroded set — `qualified.retain`
and the `internal_unsupported_area` filter in `internal_bridge_angles` — read it
only inside a candidate, and the window boundary is `spacing` beyond the
erosion's `2 x 4.5 x spacing` reach, so the covered set inside the candidate bbox
is unchanged. The full qualification and both committed fields
(`internal_bridge_areas`, `internal_bridge_angles_deg`) are therefore identical
on every layer.

**Rejected: windowing both offsets.** The first candidate clipped the positive
operand too. It moved G-code on all four classic cells sampled; on benchy
classic-off it changed **8 Internal Bridge layers**, 992 geometry lines, maximum
paired coordinate shift **22.09 mm**; on base classic-off, 5 Internal Bridge
lines. The ticket-123 auditor finds **0 structural geometry lines** on both, so
the delta is confined to the bridge role, but it is a real direction change:
`internal_bridge_angles` intersects the layer's sparse `expansion_area` with
`deep_infill_clip_area` and samples that whole-layer boundary anchor set, so
clipping the positive operand moves bridge directions even though every
candidate is local. The artifact and both audits are preserved under
`evidence/t133-windowed-bridge-offsets/rejected-full-window/`.

**Output (16-cell contract).** Output was taken before timing: 8 cells x 4 arms
= 32 runs, each cell emitting **one** hash across its four arms, every hash equal
to ticket 123's candidate-ordinary set. The delta audit is equality: zero moved
lines, structural or fill. The host binary differs between the arms and the
module tree is shared, as a host-only change requires.

**Paired gate (ordinary arms, interleaved, `RAYON_NUM_THREADS=12`).**

| cell | pairs | median wall | Δ | floor multiple | Δ CPU |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3 | 4.83 → 4.64 s | +0.19 s (+3.88 %) | **4.85x** | +0.55 s |
| benchy classic-on | 3 | 8.84 → 8.71 s | +0.13 s (+1.45 %) | **1.81x** | +0.30 s |
| benchy arachne-off | 3 | 5.01 → 4.87 s | +0.14 s (+2.84 %) | **3.55x** | +0.75 s |
| benchy arachne-on | 6 | 8.92 → 8.79 s | +0.13 s (+1.49 %) | **1.86x** | +0.93 s |
| base classic-off | 3 | 42.51 → 39.82 s | +2.69 s (+6.34 %) | **7.92x** | +7.64 s |
| base classic-on | 12 | 102.45 → 101.09 s | +1.36 s (+1.33 %) | **1.66x** | +4.48 s |
| base arachne-off | 3 | 43.82 → 39.98 s | +3.84 s (+8.76 %) | **10.94x** | +10.75 s |
| base arachne-on | 3 | 106.95 → 102.18 s | +4.77 s (+4.46 %) | **5.58x** | +8.94 s |

A strict majority of paired runs favours the candidate in every cell; base
classic-on is thinnest at 7/12 (CPU positive in 12/12). The two extended cells
were pre-declared before their extra pairs were taken, and every recorded pair
enters the median; no pair was discarded.

**Ledger correction to t131.** The S5 probe converted the erosion's thread-time
at the stage's parallelism (5.25x / 5.26x / 8.34x / 8.78x). Measured, the
conversion is **2.80x / 3.96x / 3.27x / 6.42x** (benchy-off / -on, base-off /
-on): the erosion was partly serial, so the probe understated benchy classic-off
by 1.7x and base classic-off by 2.3x, and was close on base classic-on. The
measured savings above, not the probe equivalents, are the campaign's result.

**Route arithmetic.** A banked keep, not a destination step: base classic-off
gains **2.69 s = 9.7 %** of its **27.7 s** required prepass cut; base classic-on
gains **1.36 s = 1.5 %** of its **92.5 s** cut; benchy gains 0.19 s / 0.13 s,
where the prepass is already too small for a prepass-only win. The destination
stays 8/8 and the next frontier is the planner's effective parallelism (134)
plus the human's brim ruling (130).

**Disclosures.** Box load was **21.2 %** before the campaign (DWM, Chrome, Xbox
service, Discord) and **10.9 %** at a later idle sample; 6 physical / 12 logical
cores. Each cell is internally paired with alternating order, but the background
load is part of the error bar. Three measurement windows (output, A/B, extended
pairs); no ratio spans windows. No Orca arm: the scoreboard of record remains
ticket 124's, and output identity means only its PnP wall column moves, by these
deltas. The window is per region, so exactness holds for multi-object jobs; the
saving shrinks when a region's candidates span most of its deep-infill extent.
Both guest freshness checks exit 0; the candidate changes no guest.

Evidence: [`evidence/t133-windowed-bridge-offsets/`](../evidence/t133-windowed-bridge-offsets/FINDINGS.md)
(`verify.py` 357 checks, 17/17 negative controls rejected; `reduce_t133.py`
reproduces `reduced/summary.json`; `rejected-full-window/` holds the
positive-offset experiment and its audits).
