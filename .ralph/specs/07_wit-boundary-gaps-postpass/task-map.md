# Task Map: 07_wit-boundary-gaps-postpass

Use this file because the packet spans four task IDs and closes DEV-006 on the live execution path.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-129a` | Step 0 (signature threading) | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/postpass.rs`, `wit/world-postpass.wit` | `dispatch_postpass_gcode_call` — add `commands: &[GCodeCommand]` param; `WasmRuntimeDispatcher::run_gcode_postprocess` — pass `&gcode_ir.commands` | None | This is not a logic change; it makes the commands available at the WIT boundary. The actual empty vs. real data behavior is verified by TDD tests in subsequent steps. |
| `TASK-129a` | Step 1 (real-data pass) | `crates/slicer-host/src/dispatch.rs` | `dispatch_postpass_gcode_call` line 724: change `&[]` to `commands` | None | One-line fix verified by TDD tests; the slice is already threaded through Step 0. |
| `TASK-129a` | Step 2 (boundary test) | `docs/02_ir_schemas.md`, `wit/deps/ir-types.wit` | `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs` (new) | None | All 8 GCodeCommand variants must appear in assertions; test name matches acceptance criterion command. |
| `TASK-129a` | Step 3 (preservation test) | `docs/02_ir_schemas.md`, `crates/slicer-host/src/dispatch.rs` | `crates/slicer-host/tests/postpass_gcode_command_preservation_tdd.rs` (new) | None | Order and content identical; no silent drop or mutation. Test name matches acceptance criterion command. |
| `TASK-129a` | Step 4 (empty list negative test) | `crates/slicer-host/src/dispatch.rs`, `wit/world-postpass.wit` | `crates/slicer-host/tests/postpass_gcode_empty_list_tdd.rs` (new) | None | Empty list is valid; module must handle gracefully; no contract violation. |
| `TASK-129b` | Step 5 (layer-world test) | `docs/02_ir_schemas.md`, `crates/slicer-host/src/dispatch.rs` | `crates/slicer-host/tests/layer_world_deep_copy_tdd.rs` (new) | None | Bit-for-bit field preservation for all LayerCollectionIR fields through layer-world WIT boundary. |
| `TASK-129c` | Step 6 (finalization-world test) | `docs/02_ir_schemas.md`, `crates/slicer-host/src/dispatch.rs` | `crates/slicer-host/tests/finalization_world_deep_copy_tdd.rs` (new) | None | Bit-for-bit preservation across Vec<LayerCollectionIR> through finalization-world WIT boundary. |
| `TASK-129a`, `TASK-129b`, `TASK-129c` | Step 7 (workspace gate) | `CLAUDE.md` | None | None | Final workspace gate before packet completion. |
