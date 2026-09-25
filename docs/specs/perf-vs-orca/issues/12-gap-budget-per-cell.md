# Gap budget per cell

Type: task
Status: open
Blocked by: 11

## Question

Convert the scoreboard gap into per-cell budgets: for each of the 8 matrix
cells, how much wall must move — and from which stage — for PNP to overtake
Orca?

Work:

- Combine [Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md)'s
  per-cell gaps with existing attribution (the phase walls of
  `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §5, the fuel splits of tickets 08/09) into an Amdahl
  budget per stage and generator: which stages must shrink by what factor per
  cell, and which cells bind (the hardest cell decides the route).
- Re-rank the optimization tickets (19–25) against the finish line: promote,
  demote, or kill candidates below irrelevance for the binding cells, and state
  the new take order (the timing chain may be re-wired accordingly).
- State where acceleration already pays (ticket 09's −35.8% fuel) versus where
  the remaining gap actually lives per cell.

Analysis only — no timing runs. Deliverable: the budgeted, ranked route.
