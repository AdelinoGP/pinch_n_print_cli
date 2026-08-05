# Task Map: 174-graceful-cancel

Single new task, mapped for the docs/07 crosswalk (TASK-278 is minted by this packet at closure; it has no pre-existing `docs/07_implementation_status.md` row).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-278` | `Step 1` | `docs/09_progress_events.md` | `crates/slicer-runtime/src/progress_events.rs`, `run.rs` | none | M | `cancelled` event + `cancel_flag` option field |
| `TASK-278` | `Step 2` | `docs/09_progress_events.md` | `crates/slicer-runtime/src/pipeline.rs`, `layer_executor.rs`, `run.rs` | none | M | checkpoint in the real per-layer loop + `Cancelled` variant |
| `TASK-278` | `Step 3` | `docs/specs/fork-gaps-wave2-plan.md` | `crates/pnp-cli/src/main.rs`, `Cargo.toml`, `tests/slice_cancel_tdd.rs` | none | M | signals + stdin-EOF flag + exit 130 prove the task end-to-end |
| `TASK-278` | `Step 4` | `docs/09_progress_events.md` | docs only | none | S | additive schema row + cancellation sequence |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
