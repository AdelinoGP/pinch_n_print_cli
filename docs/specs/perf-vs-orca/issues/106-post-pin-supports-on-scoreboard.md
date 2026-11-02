# Post-pin supports-on scoreboard re-derivation

Type: task
Label: wayfinder:task
Status: open
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 105](105-supports-on-engine-style-fairness.md)'s human
ruling (2026-11-02, **pin Orca to `tree_strong`**)

## Question

The standing rig now pins Orca's tree style to `tree_strong` on every
supports-on run, but **ticket 103's supports-on Orca medians were measured
before the pin**. Ticket 105's ratio transfer (`8.25x → 5.71x`, `7.00x → 5.01x`,
`5.43x → 4.96x`, `4.78x → 4.45x`) is **derived** from the measured style
multiplier, not measured on the pinned rig. What are the four supports-on Orca
medians, output stats and PnP ratios under the pin, and does the measured
post-pin table agree with the t105 transfer within the cells' own spread?

## Scope

- **Cells:** the four supports-on cells ({benchy, base} × {classic, arachne}).
  Supports-off cells are unaffected by the pin and are not re-measured.
- **Tool:** Orca only (`-Tools orca`), with the standing rig's new default
  (`-OrcaSupportStyle tree_strong`). No PnP runs: PnP's work is unchanged.
- **Schedule:** 1 warmup + 3 measured per cell, quiet machine, load recorded.
  The rig asserts `support_style = tree_strong` and `enable_support = 1` on
  every row; a row that resolves otherwise aborts the batch.
- **Evidence:** per-run bytes, SHA, TYPE/support/interface counts, wall and CPU;
  a reduced `summary.json` + `verify.py --self-test` under
  `evidence/t106-post-pin-scoreboard/`.

## Deliverable

1. A measured post-pin Orca table per supports-on cell, compared with t103's
   pre-pin medians and with the t105 transfer column.
2. The post-pin PnP ratios (t103 PnP medians unchanged), with the cut-to-park.
3. A one-line update to the map's fairness contract and, if the archive scoreboard
   is regenerated, a disclosure that its supports-on Orca arm is post-pin while
   the supports-off arm is unchanged.

## Stop rule

Measurement + record update only. No production edit, no PnP config change, no
parity port. If the post-pin medians disagree with the t105 transfer beyond the
cell's own spread, the answer discloses the disagreement rather than substituting
one for the other or re-measuring until they match.

## Method

Drive the standing rig per cell (the `-File` invocation cannot pass array
parameters; one invocation per cell is the established pattern from the t105
smoke), reduce and verify under `evidence/t106-post-pin-scoreboard/`, then update
the map's fairness-contract sentence and the ticket-105 transfer reference.
