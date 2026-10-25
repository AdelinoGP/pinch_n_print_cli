# Supports-on increment attribution at post-resolution-port HEAD

Type: task
Label: wayfinder:task
Status: open (graduated 2026-10-25 from the scoreboard refresh; unclaimed)
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Matched-pair scoreboard refresh at post-resolution-port HEAD](87-scoreboard-refresh-post-resolution-port.md)

## Question

Where does PnP's **supports-on increment** go at current HEAD, and is any of it
a batchable/parallelizable shape rather than the support geometry itself?

The 2026-10-25 scoreboard
([`evidence/scoreboard-refresh-20261025/SCOREBOARD.md`](../evidence/scoreboard-refresh-20261025/SCOREBOARD.md))
shows every supports-on cell is the worse of its pair, and the base increment is
an order of magnitude larger for PnP than for Orca:

| pair | PnP ordinary increment | Orca increment | PnP / Orca |
| --- | ---: | ---: | ---: |
| base classic | **+97.90 s** (73.17 → 171.07) | +4.50 s (13.87 → 18.37) | 21.8x |
| base arachne | **+113.38 s** (73.25 → 186.63) | +5.79 s (14.14 → 19.93) | 19.6x |
| benchy classic | +4.62 s (7.68 → 12.30) | +0.74 s (2.79 → 3.53) | 6.2x |
| benchy arachne | +4.68 s (7.90 → 12.58) | +0.76 s (2.88 → 3.64) | 6.2x |

PnP also emits **fewer** support FEATURE sections than Orca (base classic-on:
Support 431 vs 642, Support interface 70 vs 228), so the increment is not
explained by more printed support. Ticket 73 measured 134–149 s of support
stages on the two base supports-on cells at the pre-t74/t78 HEAD; t74 removed
the finalization churn (61.6 → 2.8 s) and t78 took the tree-support clip core
from 79.7 s to ~30.7 s, but no post-t81/t85 split of the remaining increment
exists.

Deliverables:

1. A fresh attribution of the supports-on increment on at least the two base
   cells (both modes may be ordinary-only if mode invariance is stated), with
   instrumented captures whose outputs are byte-identical to the post-t81
   reference set (`22200671…` classic-on, `0b29e1bf…` arachne-on).
2. The split by stage (prepass support geometry, per-layer support,
   support post-process, finalization, aggregation) and by owner
   (`tree-support-planner`, renderer/paint policy, other), with the probe
   reverted and both freshness gates green.
3. One graduated candidate per surviving reducible shape, each with its own
   pre-declared floor; shapes measured dead are recorded with their measured
   ceilings. No candidate is implemented in this take.
4. **Price the `tree_support_branch_distance` parity difference** (Orca 5 mm vs
   PnP 1 mm, supports-on) as a work driver: measure how much of the increment
   it accounts for, or bound it. **Changing the key is parity work and needs a
   separate human ruling** — this ticket measures, it does not port.

## Constraints

Measurement-only: no production edit, no candidate, no keep/drop campaign. The
standing paired ordinary + accelerated gate governs any candidate a later ticket
builds. Instrumented walls are attribution only and are never quoted as speed
claims (`docs/17_agent_debugging.md`).
