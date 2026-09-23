# Support-correctness repair, DEV-174 class

Type: task
Status: open

## Question

Repair the DEV-174-class support defect so the supports-on cells of the matrix
compare honest work: `com.core.tree-support-planner` drops ~60 contiguous
support layers mid-print on base.stl behind 29,108 code-1200 routing
rejections (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §8 — its "DEV-167" label is stale; the
ledger row is **DEV-174** in `docs/DEVIATION_LOG.md`).

Work:

- Diagnose the code-1200 routing rejection path (the `diagnosing-bugs`
  discipline: verify root cause before editing).
- Fix without changing intended geometry elsewhere; the degraded flag must
  clear and the non-fatal error count must go to zero on base.stl supports-on
  runs.
- Correctness acceptance with the human (this is route work only because the
  full 8-cell matrix decides and the fairness contract disqualifies cells whose
  work is skipped — a degraded slice is not Orca's job). Performance
  non-regression is checked at stage level per the recipe, not by whole-slice
  single runs.

Until this lands, every scoreboard revision must disclose the degraded
supports-on condition (Q5's disqualify rule marks those four cells tainted
rather than won or lost).

Measured context (ticket 11 scoreboard, 2026-09-22, matched job): every
base.stl supports-on run in both PNP modes reports `degraded=true` with
**172,181** non-fatal errors (29,108 at the old 3-wall/0.5 mm config), and PNP
emits only 280 `Support` / 55 `Support interface` sections against Orca's
638–640 / 224–225 — the dropped-layer work-skipping is visible in the output
section counts themselves.
