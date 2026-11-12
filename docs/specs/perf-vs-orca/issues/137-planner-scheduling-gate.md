# Planner scheduling gate: apply the t117 four-cell form to t135/t136?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-12; human ruling — **extend the t117 four-cell form to
  t135 Stage 2 and t136, and generalize it as the standing rule for any
  representation-safe planner-only candidate** where supports-off is
  structurally inert; the 8/8 destination is unchanged. See ## Answer)
Assignee: dev (2026-11-12 — the human said "Continue the perf vs orca map"; 137
  was the first frontier ticket in order and was resolved the same session)
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

**Ruling (2026-11-12, human, grilling): extend the t117 four-cell gate form to
[t135](135-cross-layer-occupancy-batching.md) Stage 2 and
[t136](136-planner-clip-batch-fanout.md), and generalize it as the standing rule
for any representation-safe planner-only candidate (where the four supports-off
cells are structurally inert).** "Require a supports-off leg" and "Park the
planner scheduling program" are both declined.

1. **Standing gate rule for representation-safe planner-only candidates.**
   When a planner-only candidate (`PrePass::SupportGeometry` /
   `PrePass::SupportAnalysis` or `com.core.tree-support-planner`) is
   representation-safe and the four supports-off cells are structurally inert
   (`enable_support: false` declines before the candidate's work), the
   candidate runs under the four-cell form without requiring a separate gate
   ticket:
   - **Output exactness (all 8 cells):** fresh 16-cell reference check (or 8 × 2
     ordinary when the accelerated host build is blocked, with accelerated
     output parity verified separately per t142's disclosure). Candidate ≡
     baseline byte-for-byte in all eight cells on both arms, and equal to the
     active reference set (currently [t139](139-layer-plane-parity.md)'s set).
     Any output delta is a defect and stops the campaign.
   - **Paired improvement gate (4 supports-on cells):** in `benchy classic-on`,
     `benchy arachne-on`, `base classic-on`, and `base arachne-on`, run at
     least three interleaved baseline/candidate pairs with alternating order.
     Candidate median wall must improve by at least `0.80 %` of that cell's
     same-run baseline median wall, with median process CPU corroborating
     (decreasing). Measure the candidate's own batch occupancy / thread-to-wall
     conversion rather than borrowing a proxy constant.
   - **Supports-off no-regression gate (4 supports-off cells):** in
     `benchy classic-off`, `benchy arachne-off`, `base classic-off`, and
     `base arachne-off`, candidate median wall and median process CPU must not
     regress outside paired noise (or if the code path is proven unreached by
     construction when `enable_support: false` returns before the touched site,
     disclose the inertness proof alongside the eight-cell table as in
     [t132](132-planner-occupancy-memo.md)).
   - **Destination unchanged:** the map's destination remains **8 of 8 cells won
     against OrcaSlicer**. Supports-on tranches bank against the supports-on
     cells only; the supports-off cells and the remaining gap still require
     their own route steps.
2. **Unblocked tickets.** [t135](135-cross-layer-occupancy-batching.md) Stage 2
   and [t136](136-planner-clip-batch-fanout.md) have their `Blocked by: 137`
   cleared and are now unblocked AFK tickets on the frontier, with their
   reference-set citations updated to [t139](139-layer-plane-parity.md)'s active
   reference set.
3. **No production edit in this take.** Tracker-only update across this ticket,
   [t135](135-cross-layer-occupancy-batching.md),
   [t136](136-planner-clip-batch-fanout.md), and [map.md](../map.md).
