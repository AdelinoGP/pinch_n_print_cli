# Internal-bridge offsets on a candidate-bbox operand: bounded-delta build under the 16-cell contract

Type: task / measured candidate
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
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

_To be recorded on resolution._
