# Task Map: 129_clip-polylines

Single-task-ID packet (`TASK-254`); the map is retained because the preflight gate (S0)
requires all five contract files. Backlog row: `TASK-254` in `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-254` | `Step 1` | `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 0 (lines 124-176) | `crates/slicer-core/tests/polygon_ops_tdd.rs` (+8 tests), `crates/slicer-core/src/polygon_ops.rs` (stub) | none | S | RED suite pins the six geometric guarantees + two negative cases named by the backlog row. |
| `TASK-254` | `Step 2` | `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 0 (lines 124-176) | `crates/slicer-core/src/polygon_ops.rs` (`clip_polylines` on `Clipper64::add_open_subject` + `execute(…, solution_open)`) | none | S | Implements exactly the op TASK-254 names; green suite is the closure evidence. |
| `TASK-254` | `Step 3` | `docs/05_module_sdk.md` lines 55-75 | `docs/05_module_sdk.md` (1 line, §Guest Build Invariants primitives list) | none | S | Doc Impact grep + workspace/guest gates; docs/07 row checked off via worker dispatch. |

Aggregate context cost: `S` (S + S + S). No step rated `L`.
