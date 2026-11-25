# Supports-on output is support-incomplete: what do the `exact-Z occupancy` body rejections drop?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-13; Phase-1 attribution — the rejections are a
  planner/validator **plane mismatch** amplified by an all-or-nothing gate. All
  33 rejected entries overlap the **print plane** and **none** overlaps the
  midpoint payload the planner carved: 1 benchy entry at **5.8342 mm²
  (2.276 %)**, 32 base entries at **8.4e-6–1.4706 mm²** (sum **6.8920 mm²**).
  The 44,474 base diagnostics are **(body, demand) pairs**, not unsupported
  overhangs: 2,892 unique demands and **zero** are uniquely dropped. The print
  is still support-incomplete: **32 complete base support layers** (runs 33–36
  and 38–41 plus 24 singles) and **1 benchy layer** are absent from the emitted
  G-code, **36,466.736 mm (10.2765 %)** of base support path and **192.601 mm
  (0.233 %)** of benchy path. **19/32** base and **1/1** benchy overhangs
  directly above a missing layer have Orca support at that plane and none in
  PnP. Classification **(b) defect**: the planner emits bodies carved at
  `layer.z − 0.5 h` while `validate_entry` queries `anchor_z = layer.z`; the
  zero-tolerance gate then drops the whole body for a sliver. **Ruling: the four
  supports-on ratios of record are not quotable as a matched-settings claim**; a
  fairness ticket [t148](148-supports-on-plane-carve.md) is graduated. See
  ## Answer
Assignee: 2026-11-13 OpenCode wayfinder session — the human said "continue the perf vs orca map, pick an AFK ticket"
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Benchy fixture replacement](145-benchy-fixture-rebaseline.md)'s
  2026-11-13 take. Every PnP supports-on run in the matched job reports
  `degraded: true` with code-1200 non-fatal diagnostics from
  `PrePass::SupportGeometry` / `com.core.tree-support-planner`, all of the form
  `support demand '<id>' unmet for body '<tree-body-…>': body rejected: exact-Z
  occupancy`. The counts are stable and fixture-independent: **101** per benchy
  supports-on cell and **44,474** per base supports-on cell, identical on the
  lost benchy export (t135, t142) and the replacement (t143, t144, t145), and
  zero on the four supports-off cells. The map's supports-on ratios therefore
  compare a PnP job that prints **less support than its own planner demanded**;
  the standing policy is to close asymmetries before a ratio of record, and the
  four supports-on cells carry four of the eight destination ratios.

## Question

Does the planner's exact-Z occupancy test reject support bodies it should keep,
or is the rejection correct and the unmet demand expected? Classify each family
of rejections and say whether the supports-on PnP output is support-incomplete
in a way that invalidates the wall ratio (less support printed = less work paid
than the matched job demands), and what would close it.

Decide and record, in order:

1. **Census the rejections.** On one benchy supports-on cell and one base
   supports-on cell, count rejected bodies per layer, their layer span, the
   demand ids they would have served, and the `exact-Z occupancy` payload that
   caused the rejection (the colliding exact-Z query key and the body whose
   occupancy it hit). State whether the failure is a contiguous band (DEV-174's
   signature) or scattered.
2. **Check the emitted geometry against the demand set.** For a sample of
   rejected demands, locate the overhang region they were meant to support in the
   PnP G-code: is there support there from another body, or is the region
   unsupported? Do the same on the Orca arm's output for the same layer. This is
   the check that decides "incomplete" vs "the demand was redundant".
3. **Classify.** (a) correct rejection — the body genuinely collides with the
   model at that exact Z; (b) a defect — the test, the body geometry, or the
   occupancy record is wrong; (c) a modelling choice — bodies are built
   optimistically and demands are expected to go unmet. Cite the code path
   (`plan_for_object`, the exact-Z occupancy query, and the body-rejection site
   in `modules/core-modules/tree-support-planner/src/lib.rs`).
4. **Price the fairness effect.** If support is missing, the supports-on wall is
   understated on the PnP side. Bound the missing support (path mm, coverage mm²)
   and say whether the four supports-on ratios of record remain quotable with a
   disclosure, or need an exception/ticket.

## Pre-declared gate

This is an attribution, not a candidate: no performance gate and no production
edit. The correctness bar is that the classification is falsifiable from the
evidence:

- the rejection census and the emitted-support check are re-derivable from an
  in-repo reduced set, with raw captures cited only as the fuller source;
- the sample of demands examined is named (ids, layers) so the "incomplete" or
  "redundant" verdict cannot be a single-example anecdote;
- every supports-on ratio quoted in the answer carries the degradation count and
  the verdict beside it.

If the answer is (b) and a fix is exact by construction, graduate a candidate
ticket; do not land a fix in this take.

## Acceptance

- A rejection census (per cell, per layer span, per demand family) and the
  emitted-support check, as an in-repo evidence directory with a `verify.py`.
- A written classification with the code paths named by symbol.
- A ruling on the four supports-on ratios of record: quotable with disclosure,
  or a new fairness asymmetry ticket.
- The map's Next-status and the supports-on disclosure updated.

## Answer

Resolved 2026-11-13 as a Phase-1 attribution. Evidence:
[`evidence/t147-supports-rejection/`](../evidence/t147-supports-rejection/FINDINGS.md)
(`verify.py` checks every number from `census.json`; 6/6 negative controls
rejected).

### 1. Census

| cell | diagnostics | rejected entries | rejected layers | unique demands | only-rejected | retained entries |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-on | 101 | 1 | 1 (L170) | 101 | **0** | 201 |
| base classic-on | 44,474 | 32 | 32 | 2,892 | **0** | 400 |

The diagnostics count `(body, demand)` pairs. A rejected entry carries every
demand its tree column serves, so one dropped layer emits thousands of records.
Every unique demand carried by a rejected entry is also carried by a retained
entry: **no demand is uniquely dropped**. The rejected entry is the only entry
at its layer in every case, so the layer itself is missing from the print. Base
loses 32 layers: `15, 18, 19, 27, 33, 34, 35, 36, 38, 39, 40, 41, 80, 99, 132,
155, 156, 177, 192, 216, 218, 259, 266, 316, 329, 351, 352, 362, 387, 395,
411, 414` (two four-layer runs, 33–36 and 38–41, plus 24 singles). Benchy loses
L170.

### 2. The emitted-support check

The probe G-code is byte-identical to the t145 currency (benchy `d9fc80ba…`,
base `585c6a8e…`). PnP prints **zero** support at every rejected layer; Orca
prints support at the corresponding plane on all 32 base layers and on the
benchy plane (432.6 mm).

For each rejected layer L, the demand whose overhang is at L+1 is the one whose
top interface should have printed at L. PnP coverage there is **0** in every
case. Orca coverage is:

- benchy candidate 68 (L171, 22.403 mm²): **36.85 %** at z = 34.2 (PnP 0);
  **63.8 %** at z = 34.0 (PnP 9.6 %);
- base: **19/32** candidates have Orca coverage > 5 % (0.019–0.996); the other
  13 are unsupported by both tools at that plane and are not counted either way;
- a second benchy candidate, 76 (L183, 214.7 mm²), is Orca-supported at
  36.0–36.4 mm (14.7–32.7 %) and PnP-unsupported at all four planes; it is
  carried by the same column but is not directly above L170.

**Missing path:** 36,466.736 mm base (10.2765 % of the PnP support path) and
192.601 mm benchy (0.233 %), by neighbour interpolation across each gap; the
solid-fill upper bound is 116,454 mm base. The path metric is the t147 parser
(Support + Support interface extrusion) on both arms, not the t145 metric.

### 3. Classification

**(b) a defect, on two independent layers.**

1. **Planner plane.** `layer_occupancy_payload` / `build_roles_batched`
   (`modules/core-modules/tree-support-planner/src/lib.rs`) carve against
   `SupportAnalysisIR::model_occupancy`, filled from `SliceIR` at
   `layer_sample_plane_z` = `layer.z − 0.5 h` (`crates/slicer-core/src/algos/prepass_slice.rs`).
   `validate_entry` (`crates/slicer-wasm-host/src/support_aggregation.rs`)
   queries exact-Z occupancy at `SupportPlanEntry::anchor_z = layer.z`. Every
   rejected entry overlaps the print plane and **none** overlaps the midpoint:
   benchy **5.8342 mm² (2.276 %)**, base **8.4e-6–1.4706 mm²** (sum
   **6.8920 mm²**, median 0.0805 mm², ≤ 0.069 % of body). Benchy L170 is a real collision (the cross-section gains 5.83 mm² between
   the two planes); base is a sliver family.
2. **All-or-nothing gate.** The zero-tolerance `SUPPORT_OVERLAP_TOLERANCE`
   rejects the whole entry, then `record_rejection` emits one unmet record per
   `(body, demand)` pair. Canonical `draw_circles` carves the overlap out
   (`avoid_object_remove_extra_small_parts`) and keeps the surviving part; here
   that would keep **250.5 of 256.3 mm²** on benchy and **≥ 99.93 %** of every
   base body. Dropping the whole layer turns a sliver into a missing 0.2 mm
   support layer.

Not (c): the demands are not redundant — the printed column loses its layer
and, at the overhang, its top interface. Not (a) for the base family: the body
does collide at the print plane, but the planner is what put it there.

### 4. Fairness price and ruling

The four supports-on ratios of record (t145 post-swap) and their degradation:

| cell | ratio | diagnostics | verdict |
| --- | ---: | ---: | --- |
| benchy classic-on | 2.683x | 101 | supports-incomplete, 1 layer / 0.19 m |
| benchy arachne-on | 2.580x | 101 | supports-incomplete, 1 layer / 0.19 m |
| base classic-on | 3.001x | 44,474 | supports-incomplete, 32 layers / 36.47 m |
| base arachne-on | 3.455x | 44,474 | supports-incomplete, 32 layers / 36.47 m |

**Ruling: they are not quotable as a matched-settings claim.** They stand as
historical numbers only with the disclosure above, and no **new** supports-on
ratio of record may be taken until the asymmetry is closed. The supports-off
cells have zero rejections and are unaffected. Because PnP paid less work than
the matched job demands, the ratios understate PnP's true cost and do not
rescue the destination arithmetic.

### 5. What would close it

Carve every emitted role against the **print-plane** occupancy at `anchor_z`.
The host already computes those planes for `validate_entry`'s prefetch
(`prefetch_cross_sections`), so the payload exists in the same take; publish it
to the planner (a print-plane occupancy map or the existing exact-Z query host
interface) and use it as the final carve set. Moving the validator to the
midpoint instead would hide the benchy L170 collision and print support through
the model. Keep the zero-tolerance gate as the tripwire, but record the overlap
area and clear area so a sliver is visible as a sliver. Re-run the four
supports-on cells and re-record the reference set; the supports-off cells need
no re-take. Graduated [t148](148-supports-on-plane-carve.md).
