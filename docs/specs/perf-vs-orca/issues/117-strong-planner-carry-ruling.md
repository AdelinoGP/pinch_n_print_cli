# Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?

Type: grilling
Label: wayfinder:grilling
Status: open
Assignee: (unclaimed)
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
