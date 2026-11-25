# Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-07; human ruling — **amend this representation-safe
  candidate's gate** to the four supports-on cells, with byte-identical output in
  all eight and no median wall/CPU regression in the four supports-off cells;
  the 8/8 destination is unchanged. See ## Answer)
Assignee: 2026-11-07 OpenCode wayfinder session — the human said "Continue the perf vs orca map" (no ticket named; this is the first frontier ticket in order)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 113](113-strong-planner-phase1.md)'s Phase-1
resolution (2026-11-04: the largest planner term is a representation-safe
occupancy-offset batch shape, but no planner-only shape can clear the 0.80 %
floor on the four supports-off cells).

## Question

How should the Strong planner's occupancy-inflation batching be carried, given
that it clears the 0.80 % floor on all four supports-on cells but cannot move the
four supports-off cells at all?

The options are:

1. **Fold it into the `PrePass::Slice` campaign.** [Ticket 116](116-prep-slice-canonical-bridge-classification.md)'s
   P1 runs in all eight cells and its ceiling is 19.1x–62.5x the floor, so a
   combined candidate can satisfy the all-8-cell gate with P1 carrying the
   supports-off cells and the occupancy batch adding a second increment on the
   supports-on cells. This is the least contract-violating route, but it couples
   two independent shapes and makes the G-code delta audit larger.
2. **Amend the planner-scoped gate.** Read the 0.80 % floor as applying to the
   four supports-on cells where the stage runs, with a no-regression bound on the
   other four. This keeps the shapes separable and gives the planner its own
   campaign, but it narrows the destination's all-8-cell contract for one stage.
3. **Park the shape.** Keep the decomposition as the record and re-enter when the
   prepass program reaches its second stage. This preserves the all-8-cell
   contract and spends nothing now.

## Context — what t113 established

- The planner's largest term is **model-occupancy inflation**
  (`inflate_model_occupancy`, `modules/core-modules/tree-support-planner/src/lib.rs`),
  **56.2 % / 46.0 %** of the planner on base / benchy classic-on and
  **51.4 % / 41.4 %** on the two Arachne-on cells. It is one singular
  `host::offset_polygons` per call, split almost evenly between the per-layer
  emit check and the per-origin role carve.
- Batching those offsets through `offset_polygons_batch` is exact by
  construction: the host runs the same `slicer_core::polygon_ops::offset` per
  request and returns the i-th result for the i-th request (t65/t78 precedent).
  The output delta is expected to be zero; no canonical-parity exception is
  needed.
- The measured 2.6x clamp gives **12.58x / 7.44x / 7.97x / 4.76x** the floor on
  base classic-on / benchy classic-on / base arachne-on / benchy arachne-on;
  the ideal 12x gives 18.73x / 11.08x / 11.87x / 7.09x.
- The whole `PrePass::SupportGeometry` stage on the supports-off cells is
  **300 ms = 0.22 %** of base classic-off and **46 ms = 0.51 %** of benchy
  classic-off, below the 0.80 % floor even if deleted. The planner module alone
  is **141 / 23 ms = 0.13x / 0.32x** the floor.
- The probe was output-neutral against the HEAD baseline on base classic-on,
  benchy classic-on and benchy arachne-on.
- If the ruling is option 1, the candidate implementation is **blocked by
  ticket 116** (the Slice baseline must land first); if option 2, it is
  unblocked once the gate wording is fixed; option 3 creates no ticket.

## Deliverable

A ruling on the carry route, and if option 1 or 2, the exact gate wording the
next candidate ticket must use. No production edit; no candidate until the
ruling lands.

## Answer

**Resolved 2026-11-07 by human ruling: amend this candidate's acceptance gate.**
The Strong-planner occupancy-offset batch proceeds as a representation-safe,
four-supports-on improvement campaign with explicit supports-off no-regression
requirements. The destination remains **8 of 8 cells won against Orca**; only
this candidate's positive improvement-floor scope changes.

### Why the original carry options changed

- The original option 1 is no longer executable: the P1 and whole-set
  classification candidates have landed, so their savings are already in the
  baseline and cannot carry a new paired candidate. The wave-expansion take also
  stopped the old prepass expansion framing.
- The exactness argument survives: `inflate_model_occupancy`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) can submit the same
  requests through `HostExecutionContext::offset_polygons_batch`
  (`crates/slicer-wasm-host/src/host.rs`), which applies the same
  `slicer_core::polygon_ops::offset` per request in input order. No
  canonical-parity exception or output delta is needed.
- The structural supports-off conclusion also survives: the planner declines
  before this emit work when supports are disabled, so this candidate contributes
  no positive improvement there. The old wall percentages and floor multiples
  are historical measurements, not carried into the new gate.

### Exact gate wording

> **Campaign scope:** the positive 0.80 % improvement floor applies to the four
> supports-on matched cells: `benchy classic-on`, `benchy arachne-on`,
> `base classic-on` and `base arachne-on`.
>
> **Output:** capture a fresh 16-cell reference set before timing. Baseline
> ordinary ≡ baseline accelerated ≡ the current post-t123 output; candidate
> ordinary ≡ candidate accelerated; and candidate ≡ baseline byte-for-byte in
> all eight cells. Because this shape is exact by construction, any output delta
> is a defect and stops the campaign.
>
> **Paired improvement gate:** in each supports-on cell, run at least three
> interleaved baseline/candidate pairs with alternating order. The candidate's
> median wall must improve by at least 0.80 % of that cell's same-run baseline
> median wall, and its median process CPU must decrease. Report every pair,
> medians, ranges and floor multiples. Measure this request set's own batch
> occupancy; do not adopt the existing emit-batching campaign's 2.6x as its
> price.
>
> **Supports-off no-regression gate:** in `benchy classic-off`,
> `benchy arachne-off`, `base classic-off` and `base arachne-off`, use the same
> interleaved protocol. Candidate median wall and median process CPU must not
> regress in any cell. Output remains byte-identical as required above.
>
> **Disclosure and accounting:** report the full eight-cell wall/CPU table and
> disclose any spoiled pair or starvation exclusion. Passing this campaign does
> not close the destination; the four supports-off cells and the remaining Orca
> gap still require other route steps.

The candidate is graduated as [Strong planner occupancy-offset batching: representation-safe four-cell campaign](125-strong-planner-occupancy-offset-batching.md),
open and unclaimed. No production code was edited in this ruling session.
