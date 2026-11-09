# Planner scheduling gate: apply the t117 four-cell form to t135/t136?

Type: grilling
Label: wayfinder:grilling
Status: open (unclaimed; human's decision)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Planner effective parallelism](134-planner-effective-parallelism.md)'s
  2026-11-09 gate finding: the literal all-8-cell 0.80 % form is structurally
  unreachable for any planner-only change.

## Question

Ticket 134 measured the planner-only scheduling shapes and found that the
literal all-8-cell 0.80 % gate cannot be met by any of them:

- On the four supports-off cells the **whole** `PrePass::SupportGeometry` stage
  is **266 / 211 / 37 / 35 ms** — **0.78x / 0.64x / 0.80x / 0.81x** the 0.80 %
  floor of t124's cell medians — and the planner module alone is
  **0.32–0.43x**. Even deleting the entire stage fails base classic-off and
  only ties benchy.
- The two live shapes are [t135](135-cross-layer-occupancy-batching.md) (exact
  occupancy batch, **11.2 s** = **11.1x** floor = **12.1 %** of base-on's
  required cut) and [t136](136-planner-clip-batch-fanout.md) (clip fan-out
  diagnosis, **10.7 s** = **10.6x** = **11.5 %**); both are supports-on
  tranches, not the route.
- [Ticket 117](117-strong-planner-carry-ruling.md) already amended the gate for
  one representation-safe planner candidate: the 0.80 % paired wall/CPU form on
  the four supports-on cells, byte-identical output in all eight, and no median
  wall/CPU regression in the four supports-off cells. That amendment was
  explicitly campaign-specific and does not automatically cover t135/t136.

Which is it:

- **Extend the t117 form to t135/t136.** Both proceed under the four-cell
  positive gate, all-eight byte-identical output, and the supports-off
  regression rule. The 8/8 destination is unchanged; the tranches are banked
  against the supports-on cells only.
- **Require a supports-off leg.** A planner scheduling candidate must be paired
  with a shape that moves a supports-off cell, so the combined candidate is
  judged on all eight. That makes the candidate larger and couples two
  independent schedules.
- **Park the planner scheduling program.** Bank the measurements, leave t135 and
  t136 closed as out of scope, and return the effort to the operand-volume /
  geometry route.

## Acceptance

- A recorded gate decision in this ticket and, if the amendment is granted, in
  the map's Notes.
- [t135](135-cross-layer-occupancy-batching.md) and
  [t136](136-planner-clip-batch-fanout.md) updated to the decided gate (and
  their `Blocked by` cleared or repointed).
- No production edit in this take.

## Answer

_To be recorded on resolution._
