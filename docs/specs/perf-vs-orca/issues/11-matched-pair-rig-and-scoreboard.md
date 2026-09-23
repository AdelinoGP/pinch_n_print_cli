# Matched-pair rig and first scoreboard

Type: task
Status: resolved

## Question

Build the matched-configuration OrcaSlicer/PNP comparison rig and run the first
8-cell scoreboard — the map's measuring stick.

Concretely:

- **Matched pair** per `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §3.1: same nozzle (0.4 mm), layer
  height (0.20 mm), wall and infill counts, wall generator, and supports state
  on both sides. Cells: {benchy, base} × {classic, arachne} × {supports off,
  on}; calicat stays a smoke fixture.
- **Both sides measured identically**: uninstrumented wall for verdicts,
  process CPU corroborating (external, after exit), cpu/wall ratio quoted per
  sample with starved samples excluded; repeats (base.stl needs them — ±13%
  wall spread), warmups and interleaved ordering.
- **PNP in both modes**: ordinary release build and the accelerated snapshot
  from `cargo xtask dist --accelerated` run via the complete
  `target/dist-accelerated/developer/` snapshot (bare artifacts dir silently
  loads integrated modules; dual `--module-dir` does not shadow), always with
  `--module-dir modules/core-modules`, `cargo xtask build-guests --check` exit
  0 first.
- **Output disclosure per run** (fairness contract): G-code bytes, `;TYPE:`
  section counts, degraded flag, non-fatal error count. Validate actual
  generator dispatch per run (stderr + G-code evidence — labels prove nothing,
  cf. the ticket-07 harness bug).
- Reuse/extend the `docs/specs/perf-vs-orca/evidence/alloc-bench/run_bench.ps1` lineage for the external
  measurement harness.

Deliverable: the scoreboard table (Orca / PNP-ordinary / PNP-accelerated per
cell: median wall, median CPU, output stats) with per-cell gaps stated and
DEV-174's degraded supports-on condition disclosed (repair is
[Support-correctness repair](14-support-correctness-repair.md)). No
optimization is selected or authorized by this ticket — it establishes the
route's evidence base.

## Answer

The rig is landed and the first 8-cell scoreboard is measured (batches
`s1-benchy` + `s2-base`, 96 runs = 24 warmups + 72 measured, 2026-09-22; 3
samples starved-excluded per the §10.3 ratio rule, medians n=3 otherwise).

- **Matched job** enforced both sides — 0.4 mm nozzle, 0.20/0.20 mm layers,
  2 walls, 20% gyroid sparse infill, classic/arachne per cell, supports off/on
  (tree(auto)) — and validated per run from output evidence (labels prove
  nothing): Orca's G-code resolved-config appendix, PNP's
  `resources/perimeter-acceptance/validate_measurement.ps1` plus a gyroid
  dispatch check on the stderr event stream.
- **OrcaSlicer wins all 8 cells** on median uninstrumented wall, process CPU
  corroborating in every cell: gaps 6.3x–26.2x (largest: base supports-on).
  Accelerated mode buys 0–16% wall (0–1% on benchy and base supports-on, ~15%
  on base supports-off) against its −35.8% guest-fuel cut — it does not move
  the gap; ordinary/accelerated G-code byte-parity holds.
- **DEV-174 disclosure**: every base.stl supports-on run (both modes) came back
  `degraded=true` with 172,181 non-fatal errors and roughly half of Orca's
  support sections — those four cells are tainted per Q5 until
  [Support-correctness repair](14-support-correctness-repair.md) lands.
- Scoreboard and protocol: `../evidence/matched-pair/SCOREBOARD.md`; raw rows:
  `../evidence/matched-pair/results/s1-benchy.csv`, `s2-base.csv`; rig:
  `../evidence/matched-pair/run_scoreboard.ps1`, `summarize_scoreboard.ps1`,
  `configs/`. No optimization selected or authorized.
