# Supports-on increment attribution at post-resolution-port HEAD

Type: task
Label: wayfinder:task
Status: resolved (2026-10-25; measurement-only attribution — the increment is
~69 % support geometry and ~31 % host shapes; two candidates graduated
([89](89-support-plan-exact-z-validation-parallel.md),
[90](90-postpass-layer-collection-ownership.md)); the
`tree_support_branch_distance` arm is a 6.4 %-of-increment driver, not the
reason; see ## Answer)
Assignee: 2026-10-25 OpenCode wayfinder session — frontier ticket in order
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

## Answer

**Resolved 2026-10-25 as a measurement-only attribution — no candidate
implemented, no production edit, no keep/drop campaign.** Full detail:
[`evidence/t88-supports-increment/FINDINGS.md`](../evidence/t88-supports-increment/FINDINGS.md)
(`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls
rejected; `probe.patch` reverted, both guest gates exit 0).

Measured at HEAD `5c7fa03e` on the matched base supports-on job, ordinary
external mode, 12 threads. The primary set is four quiet (5 % machine load)
instrumented captures; all four outputs are byte-identical to the post-t81
references (`292df501…` off, `22200671…` classic on, `0b29e1bf…` arachne on).
The first pass ran under 68–73 % desktop load; its walls are disclosed as
attribution-only and are not the split.

**The increment is 106.8 s classic / 109.7 s arachne** (instrumented same-run
slice walls; the t87 uninstrumented medians measure +97.9 / +113.4 s). The
increment is almost generator-independent:

| owner | classic on | arachne on | share of the classic increment |
| --- | ---: | ---: | ---: |
| tree-support planner (support geometry) | 55.52 s | 55.31 s | 52.0 % |
| exact-Z validation (host, serial) | **15.24 s** | **13.94 s** | **14.3 %** |
| support analysis (canonical kernels, parallel) | 12.85 s | 13.02 s | 12.0 % |
| per-layer tree render (parallel) | 5.91 s wall | 5.08 s wall | 5.5 % |
| finalization clone + drop | 6.76 s | 6.74 s | 6.3 % |
| G-code emission (whole-collection drop) | 6.26 s | 6.00 s | 5.9 % |
| serialize + post-process | 0.57 s | 0.81 s | 0.5 % |
| postpass teardown / reconcile gap | 4.58 s | 4.21 s | 4.3 % |
| common-stage drift | −0.96 s | +4.43 s | −0.9 / +4.0 % |

**Verdict: ~69 % is the support geometry itself; ~31 % is host-side and
reducible without touching geometry.** Two shapes clear the map's 0.80 %
screening floor:

1. [Parallelize the support-plan exact-Z validation](89-support-plan-exact-z-validation-parallel.md)
   — the 15.24 s / 13.94 s serial loop is 99.3 % of the post-planner gap;
   every `validate_entry` is independent over an immutable service. Ceiling
   **8.28 % / 7.42 %** of the cell.
2. [Single-ownership layer collection through finalization and emission](90-postpass-layer-collection-ownership.md)
   — one 6.96 s / 6.92 s finalization seam, one 6.34 s / 6.09 s emission seam
   (the probe's `SystemTime` marks show the clone is ~5 ms and the 6 s is the
   drop), and a 4.58 s / 4.21 s teardown gap. Upper bound **9.18 % / 8.60 %**;
   net upper **5.84 % / 5.47 %** after the one remaining drop.

Measured dead / geometry-bound, with their ceilings: the SupportAnalysis
eligibility pre-pass fold (2.24 s / 2.28 s gross, 1.21 %, but the predicate
work remains and the surviving scaffolding is not resolved above the floor);
the contact-detection kernel (10.16 s / 10.30 s, 5.5 %, parallel canonical
geometry); the planner (55.5 s / 55.3 s, 30 %, support geometry with t78's
clip batching in place); the per-layer render (51.96 s / 50.58 s accumulated,
5.9 / 5.1 s wall, already parallel).

**`tree_support_branch_distance` bound:** a config-only 5 mm arm measured
**−7.25 s slice wall (−3.94 %)** and **−6.8 s of the support increment
(6.4 %)**, with output bytes −0.83 % and a changed SHA. It is not the driver;
changing it is geometry-changing parity work and returns to the human, not to a
performance ticket.

The measurement also corrected a probe trap: an `Instant` clock read
disagreed with the event timestamps by the width of an earlier phase, so the
`ge.clone` bracket claimed 4.09 s while the `SystemTime` marks showed 5 ms. All quoted
probe claims use the `SystemTime` marks and the stage brackets. The probe build
is reverted; both guest freshness gates exit 0.
