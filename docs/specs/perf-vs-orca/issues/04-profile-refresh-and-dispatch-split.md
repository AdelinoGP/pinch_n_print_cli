# Profile-first refresh and dispatch split

Type: task
Status: resolved

## Question

After the guard fix, where does the time go on the current tree — and which
investigation does the user authorize next?

## Answer

Executed 2026-09-05 (`docs/specs/perf-vs-orca/evidence/perf-refresh/FINDINGS.md`, `MEASUREMENTS.md`), then
the user-directed follow-up (`docs/specs/perf-vs-orca/evidence/perf-split/FINDINGS.md`).

- Refreshed phase walls (instrumented, base run1 / repeat / benchy): prepass
  283.8 / 258.2 / 11.9 s, per_layer 171.9 / 176.0 / 12.8 s, postpass ~5.9 /
  ~5.9 / 0.7 s. `com.core.tree-support-planner` is the largest serial module
  (96.6 / 92.9 / 3.3 s); `com.core.classic-perimeters` dominates accumulated
  worker elapsed (1,725.9 s base — **not** CPU seconds).
- Baseline medians retained as load-qualified observations only (the machine
  did not stay quiet); no controlled speedup or instrumentation-overhead claim
  was made. Output corrections recorded: TYPE counts are not universally
  stable (`Inner wall` varied 525–527 base / 244–251 benchy), and base runs
  stayed degraded with 29,108 non-fatal errors (DEV-174).
- **User decision**: take the **perimeter and dispatch split** next (rather
  than the planner recommendation). The split attributed ~97% of the clustered
  per-layer modules' dispatch cost to host-side input marshalling —
  `derive_needs_support` and region-field conversion re-executed by every
  module receiving slice regions — producing ranked proposal #1 (memoize
  per-region host data), landed as
  [Prepared-region arena cache](05-prepared-region-arena-cache.md).
- Timing correction propagated into the handoff: every per-layer figure
  historically labelled "CPU" is accumulated worker wall-clock.
