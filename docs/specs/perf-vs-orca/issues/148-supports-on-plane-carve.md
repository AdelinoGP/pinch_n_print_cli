# Supports-on plane carve: align the planner's collision occupancy with the print plane

Type: task / fairness fix + re-record
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Supports-on output is support-incomplete](147-supports-on-body-rejection-attribution.md)'s
  2026-11-13 attribution. The rejections are a plane mismatch, not redundant
  demand: the planner carves `SupportAnalysisIR::model_occupancy` at
  `layer.z − 0.5 h`, the host validates `SupportPlanEntry::anchor_z` at
  `layer.z`, and the zero-tolerance gate drops a whole body for a sliver. Base
  loses 32 complete support layers (36.47 m of support path) and benchy 1
  (0.19 m); 19/32 base and 1/1 benchy overhangs above a missing layer have Orca
  support and none in PnP.

## Question

What is the smallest exact change that makes the planner emit bodies disjoint
from the **print-plane** model occupancy, and what does it cost in output and
wall once the four supports-on cells are re-recorded?

Decide and land, in order:

1. **The payload.** Prefer a planner-side final carve against the print-plane
   occupancy. The host already computes those cross-sections for
   `validate_entry`'s `prefetch_cross_sections`, so either publish them on
   `SupportAnalysisIR` as a print-plane map (keyed like `model_occupancy`) or
   call the existing exact-Z query host interface from the planner. Do **not**
   move the validator to the midpoint: that would hide the real benchy L170
   collision and print support through the model. Keep the midpoint
   payload as a conservative extra if the union is cheaper than a second query.
2. **The final carve.** Apply the print-plane set in the planner's final
   set-wide difference (after simplification, before role assembly), to every
   role (`SupportBody`, `TopInterface`, `BaseInterface`, `BottomInterface`).
   The host gate stays zero-tolerance as the tripwire.
3. **Re-record the four supports-on cells.** Expect **zero** `exact-Z
   occupancy` diagnostics on benchy classic-on / arachne-on and base
   classic-on / arachne-on, the 33 missing layers to reappear, and the support
   path to rise by ~36.47 m base / ~0.19 m benchy (measured by the t147
   parser; the exact number is the take's). The supports-off cells must be
   byte-identical to the active reference set and keep zero rejections.
4. **Re-derive the ratios.** The four supports-on ratios of record are
   supports-incomplete today; measure the matched pairs on the new reference
   set and re-derive all four. The expected direction is a small PnP wall
   increase (it now prints the support it planned); the size is unmeasured.
5. **Diagnostics clarity (optional, cheap).** If the host gate ever fires
   again, record the overlap area and the clear area per rejected entry rather
   than only one `(body, demand)` record per pair.

## Pre-declared gate

Fairness fix, not a performance candidate: no wall-improvement bar. Correctness
bars:

- zero `exact-Z occupancy` diagnostics on all four supports-on cells;
- the 33 missing layers present in the G-code and the `19/32 + 1/1` Orca-
  supported candidates covered by PnP support at the same plane;
- supports-off output byte-identical to the active set, with zero rejections;
- the four supports-on cells re-recorded under the t145 ceremony (fresh hashes,
  matched keys asserted, the new reference set linked from the map), then the
  four ratios re-derived on that set.

If the print-plane carve cannot be made exact without a contract change, the
ticket may choose the host-side carve-and-keep instead, but it must price the
change to the validation contract and keep a zero-tolerance assertion on the
carved result. Do not weaken the gate to a tolerance.

## Acceptance

- An in-repo evidence directory with a `verify.py`: zero rejections, the 33
  recovered layers, the coverage check, the supports-off byte-identity, the new
  supports-on reference hashes, and the four re-derived ratios.
- The map's supports-on disclosure replaced by the new reference set (or the
  disclosure explicitly carried forward if the fix is deferred).
- The four supports-on ratios of record re-derived, with the old t145 set
  marked historical.
