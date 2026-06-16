# Task Map: 64_paint-native-migration

This packet spans new work not covered by any pre-existing open `TASK-###` in `docs/07_implementation_status.md`; it is therefore assigned the new primary task ID **`TASK-204`** (created by this packet at closure). It consolidates completed infrastructure (TASK-130c, TASK-181, TASK-128b through TASK-130b, TASK-180b) from WASM modules into host-native paths. TASK-136 (open — E2E progress-event coverage for paint-annotation failure codes 501-504) is tangentially relevant because the always-on host annotator now exercises the code 504 warning path in production.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | Context cost | Notes |
| --- | --- | --- | --- | --- | --- |
| TASK-136 (open) | Step 3, Step 9 | `docs/04_host_scheduler.md` | `layer_executor.rs`, `slice_postprocess.rs` | M | The always-on host annotator exercises code 503/504 paths previously only hit when the WASM module was absent. Per-point parallelism touches the same annotation loop that emits code 504 warnings. |
| TASK-204 (new — primary) | All 11 steps | `docs/04_host_scheduler.md`, `docs/07_implementation_status.md` | 22 files (see `design.md` §Files in Scope) | M | This packet creates the `TASK-204` row in `docs/07_implementation_status.md` with status `[x]` upon completion. The row references `64_paint-native-migration` and summarizes the consolidation. |

The `TASK-136` row in `docs/07` should have its notes updated to reflect that the code 504 warning path is now exercised by the always-on host annotator (not just in E2E progress-event tests). The new `TASK-204` row for this packet should appear after the latest entry in `docs/07` (currently `TASK-203`).
