# Matched-pair rig and first scoreboard

Type: task
Status: open

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
