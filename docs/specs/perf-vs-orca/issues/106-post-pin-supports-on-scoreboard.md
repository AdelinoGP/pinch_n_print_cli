# Post-pin supports-on scoreboard re-derivation

Type: task
Label: wayfinder:task
Status: resolved (2026-11-02; measurement-only. Post-pin ratios **6.02x / 6.11x**
base classic / arachne and **4.77x / 4.53x** benchy classic / arachne; park needs
**66.9–75.5 %** cuts. The t105 transfer agreed on two cells and missed two —
base arachne-on **−17.95 %**, benchy classic-on **+3.89 %** — because the pinned
job is byte-identical to t105's yet **1.9–9.6 % faster today**. See ## Answer)
Assignee: 2026-11-02 OpenCode wayfinder session — the human said "continue the
  perf vs orca map" (no ticket named); the frontier re-derivation at HEAD found
  this as the only open child.
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

## Answer

**Measured, quiet, and the record now names the pin.** One warmup + three
measured per cell, Orca 2.5.0-beta (`7663f5d3…`, the same binary t105 measured),
`support_style = tree_strong` asserted per row, load 16.4 % → 3.3 %, 12/12
measured samples retained.

| cell | t103 pre-pin | post-pin | delta | PnP ordinary | ratio | cut to park |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-on | 19.488 s | **26.706 s** | +37.04 % | 160.857 s | **6.02x** | 75.10 % |
| base arachne-on | 23.028 s (n=2) | **26.365 s** | +14.49 % | 161.139 s | **6.11x** | 75.46 % |
| benchy classic-on | 2.286 s | **2.600 s** | +13.74 % | 12.403 s | **4.77x** | 68.56 % |
| benchy arachne-on | 2.553 s | **2.698 s** | +5.69 % | 12.216 s | **4.53x** | 66.87 % |

**Does it agree with the t105 transfer within the cells' own spread?** On the
pre-registered rule (tolerance = the larger of the two takes' own measured
ranges, fixed before measuring): **two cells agree, two do not.**

- base classic-on — predicted 28.145 s, measured 26.706 s (−5.11 %, tol 2.120 s) — **agrees**
- benchy arachne-on — predicted 2.747 s, measured 2.698 s (−1.78 %, tol 0.151 s) — **agrees**
- benchy classic-on — predicted 2.503 s, measured 2.600 s (+3.89 %, **0.004 s past** a 0.093 s tolerance) — **disagrees, nominally**
- base arachne-on — predicted 32.135 s, measured 26.365 s (**−17.95 %**, tol 4.670 s) — **disagrees, materially**

**The disagreement is vintage, not the pin** — and the evidence for that is
output-shaped, not rhetorical. The pinned job is **byte-identical to t105's
`tree_strong` arm on all four cells** (44,803,778 / 43,802,018 / 9,481,130 /
10,331,360 B; +23.46–35.34 % over t103's pre-pin volumes; support-interface
sections 423 / 426 / 57 / 58), on a different day, in a separate campaign. The
same job ran **1.9–9.6 % faster today** than in t105's session. Meanwhile t103's
base arachne-on baseline sits **+12.25 %** above t105's *organic* arm and rests on
only **2** retained samples with a 4.67 s spread (t105 already called it "ticket
103's own noisy cell"). The transfer multiplies one vintage's median by another
vintage's ratio and compares to a third, so those two drifts add: **+12.25 % −
7.90 % is the whole miss.** The transfer is therefore **retired as an absolute
predictor**; a style price must be paired same-session, and an absolute median
must name its day.

Orca's g-code **SHA changes every run** (the appendix embeds timing), so SHA
cannot witness a job change on the Orca side — this take uses output **volume**
as the non-vacuity signal, and requires it to move ≥ 5 % against the pre-pin arm
(measured +23.46–35.34 %).

**Record updates.** The map's fairness contract now carries the post-pin table and
labels ticket 103's supports-on Orca medians **superseded (pre-pin)**; the t105
Decisions entry keeps its historical derived transfer, which the fairness
contract now marks retired as an absolute predictor.

**Disclosures.** No organic arm was re-measured (out of scope), so the
today-vs-t103 delta is not a new paired style price; it is reported as what it is
and corroborated against t105's paired price. The take started only after an
unrelated game process and a **sibling checkout's**
`cargo test -p slicer-runtime` e2e binary (`D:\slicerProject\pinch_n_print_cli`,
not this workspace) had both exited. `cargo xtask build-guests --check` exit 0.

Evidence: [`evidence/t106-post-pin-scoreboard/FINDINGS.md`](../evidence/t106-post-pin-scoreboard/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 75/75 claims, 9/9 negative
controls rejected).
