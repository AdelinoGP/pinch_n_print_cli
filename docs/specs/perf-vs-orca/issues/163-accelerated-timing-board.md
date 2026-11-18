# Accelerated timing arm: three-tool eight-cell board on the current tree

Type: task / measured scoreboard
Label: wayfinder:task
Status: resolved (2026-11-18; measured board - the accelerated arm is
  output-exact 8/8 and worth -13.2 %...+4.4 % wall, median cell -1.1 %; it does
  not move the destination and it is worse on the tightest cell. See ## Answer)
Assignee: 2026-11-18 OpenCode wayfinder session — the human said "Run the
  accelerated next", after [162](162-accelerated-host-verbatim-path.md) restored
  the accelerated host build.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none (the accelerated arm builds: t162)
Graduated from: [t162](162-accelerated-host-verbatim-path.md)'s "not claimed":
  the arm is available and output-identical 16/16, but no accelerated timing
  number exists. The map has not measured the accelerated arm since it broke at
  packet 142.

## Question

The accelerated arm has been unavailable since packet 142, so every timing
statement in this tracker is ordinary-only, and the destination's
matched-settings claim has never been corroborated on the accelerated build.
Now that the arm builds and is output-exact:

**What does the accelerated arm measure, on all eight cells, in one window
against Orca and against the ordinary arm — and does it change any ratio of
record?**

## Scope

1. **One campaign, one window, three tools.** 8 cells × {Orca, PnP ordinary,
   PnP accelerated}, 1 warmup + 3 measured runs per cell, interleaved, same rig
   and fixtures as t145/t161 (`evidence/matched-pair/run_scoreboard.ps1`).
2. **Output identity first.** Both PnP arms must reproduce t151's active
   reference set in **8/8** cells (t162 proved 16/16 in a single run; this
   re-proves it under three measured runs each).
3. **The table.** Per cell: ordinary wall + process CPU, accelerated wall +
   process CPU, Orca wall, the two ratios against Orca, and the
   accelerated-vs-ordinary paired delta with its per-cell spread.
4. **One window only.** All three tools in the same interleaved window, so no
   number here composes across the t161 tool boundary (OrcaSlicer 2.5.1) or
   across any earlier wall.
5. **No route arithmetic.** This is a board, not a candidate: it does not
   re-derive the phase split, the required cut, or any projection. It records
   what the two arms measure.

## Pre-declared gate

**No performance gate** — this is a measurement, not an adoption. Correctness
bar:

- both PnP arms byte-identical to t151's active set in 8/8 cells, every run
  `exit 0`, `degraded: false`, zero `exact-Z occupancy` diagnostics on the
  four supports-on cells;
- Orca matched keys asserted per run, exactly as t161;
- per-cell spread disclosed for all three tools;
- box-load samples recorded;
- no causal claim from any cross-window movement.

## Acceptance

- A reduced artifact with the 24-cell table (8 cells × 3 tools), the identity
  check and the disclosures, plus `verify.py` re-deriving every quoted number
  and exiting 0 on a plain checkout.
- The map's destination-status block updated with the accelerated arm's position
  and the accelerated-vs-ordinary delta named per cell.
- If the accelerated arm is materially slower anywhere, that is recorded as the
  finding — not smoothed over.

## Answer

Resolved 2026-11-18 as a measured board. Evidence:
[`evidence/t163-accelerated-board/`](../evidence/t163-accelerated-board/FINDINGS.md)
(`verify.py` PASS 366/366, `--self-test` 14/14 mutations rejected).

One campaign, one window, three tools (OrcaSlicer 2.5.1, PnP ordinary, PnP
accelerated), 8 cells x 1 warmup + 3 measured runs, 96 data rows. **Both PnP arms
are byte-identical to t151's active set in 8/8 cells**, every run `exit 0`,
`degraded: false`, matched keys asserted on all 24 Orca runs.

| cell | ordinary | accelerated | Orca | ord/Orca | acc/Orca | acc vs ord |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.900 s | 3.846 s | 2.189 s | 1.782x | 1.757x | -1.4 % |
| benchy classic-on | 8.241 s | 7.951 s | 2.941 s | 2.802x | 2.703x | -3.5 % |
| benchy arachne-off | 4.849 s | 4.209 s | 2.500 s | 1.940x | 1.684x | -13.2 % |
| benchy arachne-on | 8.269 s | 8.199 s | 2.923 s | 2.829x | 2.805x | -0.8 % |
| base classic-off | 25.283 s | 26.391 s | 18.200 s | 1.389x | 1.450x | +4.4 % |
| base classic-on | 77.537 s | 78.683 s | 28.607 s | 2.710x | 2.751x | +1.5 % |
| base arachne-off | 25.861 s | 25.311 s | 17.755 s | 1.456x | 1.426x | -2.1 % |
| base arachne-on | 76.732 s | 76.517 s | 29.310 s | 2.618x | 2.611x | -0.3 % |

**The arm is real but it is not a lever.** Wall effect -13.2 %...+4.4 %, median
cell -1.1 %, mean -1.9 %, and per-cell spreads 2.9-21.6 % mean only two cells'
deltas exceed their own spread. Process CPU is the tight channel and it is lower
in 6 of 8 cells (-4.3...+1.4 %). **0 of 8 cells won on either arm**; required cut
28.0 %-64.7 % ordinary and 29.9 %-64.3 % accelerated. On the tightest cell
(`base classic-off`, 1.389x ordinary) the arm is **worse** at 1.450x.

The take also provides an unplanned cross-window control: both arms moved
together against t161 (~20 h earlier), leaving the paired ratio nearly intact
(benchy classic-off -0.8 % for 3.5-4.3 % wall movement; base classic-off -2.8 %
for 11.8-15.1 %), which independently confirms t161's method claim. No absolute
wall in this take is compared with t161's.

Disclosed: no null band (board, not candidate); box load 114 samples, min 1 % /
median 76 % / max 100 %; one window only, so no cross-window or cross-tool-version
composition.
