# Supports-on plane carve: align the planner's collision occupancy with the print plane

Type: task / fairness fix + re-record
Label: wayfinder:task
Status: resolved (2026-11-13; fairness fix landed — the planner now carves
  emitted roles against the print plane the exact-Z gate validates, published as
  `SupportAnalysisIR::print_plane_occupancy` (schema 2.1.0, WIT
  `print-plane-occupancy`, package `2.1.0`). All four supports-on cells:
  diagnostics **101 / 101 / 44,474 / 44,474 → 0**, `degraded: false`, the **33
  missing layers reappear** (32 base + 1 benchy) with none lost, support path
  **+36,700.688 / +36,700.716 mm** base and **+294.871 mm** benchy. Supports-off
  stays byte-identical to the t144 active set (4/4). Ratios of record
  re-derived: **2.705x / 2.829x / 3.356x / 3.248x**, CPU corroborating. See
  ## Answer and [`evidence/t148-print-plane-carve/`](../evidence/t148-print-plane-carve/FINDINGS.md)
  (`verify.py` 347/347, 8/8 negative controls).
Assignee: 2026-11-13 OpenCode wayfinder session — the human said "continue the perf vs orca map, pick an AFK ticket" (t148 was the first unblocked, unclaimed AFK frontier ticket in order; 146 is the human's HITL grilling)
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

## Answer

Resolved 2026-11-13. Evidence:
[`evidence/t148-print-plane-carve/`](../evidence/t148-print-plane-carve/FINDINGS.md)
(`verify.py` 347/347 checks, 8/8 negative controls; `reduce_t148.py`
regenerates `census.json`; `reference.json` is the new currency).

### 1. The payload and the carve

The planner-side route, as the ticket preferred, with a contract change:

- `SupportAnalysisIR::print_plane_occupancy: Vec<SupportAnalysisPrintPlaneEntry>`
  (`crates/slicer-ir/src/slice_ir.rs`) carries one object-wide model
  cross-section per `(global layer, object)` at the layer's **print plane**
  (`layer.z`). Additive schema **2.1.0**; WIT `print-plane-occupancy` on
  `support-analysis-view`, package `slicer:prepass-support-geometry@2.1.0`.
- `collect_print_plane_occupancy`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) batches one
  `slice_mesh_ex` pass per object over its print planes. The plane is quantized
  through `mm_to_units → units_to_mm` because that is the exact argument
  `validate_entry` hands the query service; slicing at the unquantized
  `layer.z` would leave a sub-unit plane difference and a sliver for the
  zero-tolerance gate to reject.
- `print_plane_occupancy_payload` and the tree planner's emit loop chain the
  inflated print-plane set into `collision_with_territory` and `role_collision`
  (`modules/core-modules/tree-support-planner/src/lib.rs`), so the final
  set-wide difference covers both planes. `prefetch_occupancy_memo` batches the
  chunk's print-plane offsets beside the midpoint offsets, so no serial per-layer
  host call was added.
- The validator is unchanged: still exact-Z at `anchor_z`, still
  `SUPPORT_OVERLAP_TOLERANCE = 0`. The midpoint payload is kept as the
  conservative extra the ticket allowed.

### 2. Correctness

| cell | diagnostics | support path before → after (mm) | Δ (mm) | layers | recovered | lost |
| --- | ---: | --- | ---: | --- | ---: | ---: |
| benchy classic-on | 101 → **0** | 82,644.645 → 82,939.516 | **+294.871** | 201 → 202 | L170 (z 34.2) | 0 |
| benchy arachne-on | 101 → **0** | 82,644.645 → 82,939.516 | **+294.871** | 201 → 202 | L170 (z 34.2) | 0 |
| base classic-on | 44,474 → **0** | 354,855.077 → 391,555.765 | **+36,700.688** | 400 → 432 | 32 planes | 0 |
| base arachne-on | 44,474 → **0** | 354,855.049 → 391,555.765 | **+36,700.716** | 400 → 432 | 32 planes | 0 |

Recovered planes are the set difference of the support-layer Z lists, not a
count: benchy **34.2**; base **3.2, 3.8, 4.0, 5.6, 6.8, 7.0, 7.2, 7.4, 7.8, 8.0,
8.2, 8.4, 16.2, 20.0, 26.6, 31.2, 31.4, 35.6, 38.6, 43.4, 43.8, 52.0, 53.4,
63.4, 66.0, 70.4, 70.6, 72.6, 77.6, 79.2, 82.4, 83.0**. Recovered path is
36,723.3 mm base against t147's 36,466.7 mm estimate (+0.70 %); the remaining
22.6 mm of the net base delta is existing layers losing a sliver to the new
carve. Every run is `degraded: false`.

**Candidate-coverage clause, two readings.** Plane presence is **20/20**: every
candidate t147 recorded as Orca-supported above a dropped layer now sits on a
plane that prints support. Polygon coverage on t147's > 5 % metric is **11/19
base, 0/1 benchy**; it is recorded and **not** used as a pass gate because the
retained-layer control (candidates on never-dropped layers) reads the same
0.000–0.021 — PnP tree interfaces sit under node footprints, not across the whole
overhang polygon, which is a pre-existing geometry-parity gap. The support-path
half of the clause is met.

### 3. Identity and currency

The four supports-off cells are byte-identical to the t144/t145 active set and
carry zero diagnostics (`1ae0cfa8…`, `8faf02b1…`, `c0b9014f…`, `17f63e93…`).
The four supports-on hashes move and are the new active set in
[`reference.json`](../evidence/t148-print-plane-carve/reference.json):
`37788518…` / `f86d8897…` / `eccbd523…` / `1811425f…`. t144/t145's supports-on
hashes are recorded as superseded; every supports-on number built on them is
historical.

### 4. Ratios of record (ordinary arm, supports-on)

| cell | Orca wall / CPU (s) | PnP wall / CPU (s) | ratio | t145 (superseded) |
| --- | --- | --- | ---: | ---: |
| benchy classic-on | 2.709 / 11.422 | 7.327 / 39.281 | **2.705x** | 2.683x |
| benchy arachne-on | 2.565 / 11.984 | 7.256 / 41.219 | **2.829x** | 2.580x |
| base classic-on | 21.598 / 95.047 | 72.487 / 588.656 | **3.356x** | 3.001x |
| base arachne-on | 22.341 / 103.875 | 72.570 / 592.828 | **3.248x** | 3.455x |

**Ruling: the four supports-on ratios are quotable as a matched-settings claim**
again, with the disclosures below. The supports-off ratios are untouched.

Disclosures: the window is not t145's, and Orca's own medians moved between them
(base classic-on 23.414 → 21.598 s, base arachne-on 25.822 → 22.341 s), so the
cross-take wall delta is **not** a measured cost of the fix. PnP process CPU
rises visibly (base classic-on 492.7 → 588.7 s median) because the job now
prints the support it always demanded plus one batched print-plane mesh pass and
its offsets; no causal wall claim is taken and there is no wall bar. No
accelerated timing arm (`libmimalloc-sys` build failure, t142/t145); 3 measured
runs per cell; classic-cell hashes were stable this window (four identical
hashes per cell), which is not guaranteed (DEV-093 class).
