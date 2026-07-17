# Task Map: 169-time-estimator-slice-stats

This packet mints a new backlog entry: **TASK-275 does not yet exist in `docs/07_implementation_status.md`** (verified 2026-07-17; highest existing ID is TASK-271; TASK-272/273/274 are minted by wave-1 sibling packets 166/167/168). At packet closure, a worker dispatch adds the TASK-275 row to `docs/07_implementation_status.md` in the doc's row format, e.g.:

`- [x] TASK-275 — acceleration-aware trapezoidal print-time estimator (post-emit pass in slicer-gcode), slice_stats progress event (schema 1.2.0), layer_count on phase_start(per_layer) (packet 169) — Closed <date> — closed by packet 169-time-estimator-slice-stats.`

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-275` | `Step 1` | `docs/09_progress_events.md` | `crates/slicer-ir/src/resolved_config.rs` | none | S | Optional machine-limit/density config fields the estimator reads |
| `TASK-275` | `Step 2` | `docs/specs/fork-gaps-wave1-plan.md` (Packet A) | `crates/slicer-gcode/src/{estimator.rs,emit.rs,lib.rs}` | none (fresh Marlin-style model, no port) | M | Estimator fills the hardcoded `estimated_print_time_s: 0` |
| `TASK-275` | `Step 3` | `docs/09_progress_events.md` | `crates/slicer-runtime/src/{progress_events.rs,run.rs,postpass.rs}` | none | M | slice_stats 1.2.0 + creates the production slice_complete emission (none exists today) |
| `TASK-275` | `Step 4` | `docs/09_progress_events.md` | `crates/slicer-runtime/src/{progress_instrumentation.rs,pipeline.rs}` | none | S | layer_count on phase_start(per_layer) via additive PipelineInstrumentation method |
| `TASK-275` | `Step 5` | `docs/09_progress_events.md` | `docs/09_progress_events.md` | none | S | 1.2.0 row shipped; instrumented-version (1.1.0-const vs 1.3.0-doc) divergence resolved to 1.2.0 |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
