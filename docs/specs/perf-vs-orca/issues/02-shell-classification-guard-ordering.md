# shell_classification guard ordering

Type: task
Status: resolved

## Question

Where does `host:shell_classification`'s 223 s on base.stl actually go, and can
it be removed without changing intended geometry?

## Answer

Landed 2026-09-04 (`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §9). The cost was **not** where the
module's doc comment claimed (Pass 1 is ~4%, not dominant) and not in
`apply_opening`: ~91% of the stage was `gate_internal_bridge_sites`, and ~81%
of *that* was two full-layer `offset` calls on `deep_infill_area` computed
before the `internal_solid_fill_empty` / `unsupported_empty` early-outs that
discard them.

Changes (ordering/windowing/parallelism only, no intended-geometry change):

- Guards hoisted first in their original relative order (skip histogram
  unchanged); `internal_unsupported_area` computed lazily only when filtering
  survivors.
- Depth windowing via `depth_window_start` / `filled_window_start`
  (`slicer_core::algos::bridge_over_infill`) in both callers — removes a
  quadratic that had not yet dominated (kept as strictly less work).
- Bridge-gate loop parallelized (`par_iter_mut` over slices: 533 → 84 ms on
  benchy); `gate_internal_bridge_sites`' per-layer loop split into parallel
  qualification + serial commit.

Results: benchy stage 17.8 s → 4.26 s (guards) → **2.96 s** (+ parallel);
base.stl direction clear but the figure is bracketed by wall noise (two final
runs 529 s / 416 s around one intermediate 486 s run — §3.6).

Along the way a pre-existing e2e failure was found and fixed: the staircase
ironing assertion in `slicing_promotion_e2e_dispatch_regression_tdd.rs` assumed
origin-centred coordinates and broke after bare meshes were placed on the bed;
re-anchored through `slicer_model_io::bed_center_mm` with bounds unchanged.
Other origin-centred e2e assertions may be latently wrong the same way.

Verification at completion: five test buckets green (unit/contract/executor/
integration/e2e), `slicer-core --features host-algos` green, clippy
`--all-targets -D warnings` clean, `cargo xtask check-literals` 0 violations,
guest freshness exit 0. `cargo test --workspace` not run (per test discipline).
