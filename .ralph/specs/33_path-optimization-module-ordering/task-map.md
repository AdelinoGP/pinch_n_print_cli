# Task Map: path-optimization-module-ordering

This packet introduces a new sub-task `TASK-152h` and closes the prior packet-32 sub-task `TASK-152g` (whose surface is now actually consumed end-to-end). It also marks packet 18 (`18_path-optimization-entity-ordering`) `superseded` and records the deviation log entry. The `TASK-152` parent stays `[~]` because 152b/c/f remain open.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-152h` (new) | Step 1 | `docs/05_module_sdk.md` | `modules/core-modules/path-optimization-default/src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` | Port packet-18 NN algorithm into the module. Read entities via `collection.get_ordered_entities()` (returns `&[OrderedEntityView]`); call `collection.set_entity_order(items)` exactly once. Reversal flag stays `false`. |
| `TASK-152h` | Step 2 | `docs/04_host_scheduler.md` | `crates/slicer-host/tests/path_ordering_tdd.rs` | — | Rewrite the same-object acceptance test against live `WasmRuntimeDispatcher` dispatch of `path-optimization-default.wasm`. |
| `TASK-152h` | Step 3 | `docs/02_ir_schemas.md` | `crates/slicer-host/tests/path_ordering_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` | Rewrite cross-object, bridge-priority, determinism, and no-op tests against live dispatch. |
| `TASK-152h` | Step 4 | `docs/01_system_architecture.md` | `crates/slicer-host/tests/path_ordering_tdd.rs` | — | Add `no_module_proposal_leaves_raw_assembled_order` (currently fails — Step 5 makes it pass by removing the fallback). |
| `TASK-152h` | Step 5 | `docs/01_system_architecture.md` | `crates/slicer-host/src/layer_executor.rs`, `crates/slicer-host/src/lib.rs`, `crates/slicer-host/tests/path_ordering_tdd.rs` | — | Delete `order_entities_by_nearest_neighbor` and remove its re-export. Both call sites in `execute_single_layer` use raw `assemble_ordered_entities` directly. Also delete the obsolete packet-32 test `reordered_sequence_is_consumed_by_path_optimization_stage` (its host-pre-stages-NN contract no longer exists). |
| `TASK-152h` | Step 6 | `docs/14_deviation_audit_history.md` | `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | — | Mark packet 18 `superseded`. Record the deviation. |
| `TASK-152g` (close) | Step 7 | `docs/07_implementation_status.md` | `docs/07_implementation_status.md` | — | Close `TASK-152g` — its packet-32 surface is now actually consumed end-to-end. |
| `TASK-152h` (close) | Step 7 | `docs/07_implementation_status.md` | `docs/07_implementation_status.md` | — | Close `TASK-152h` — algorithm now lives in the module, helper is deleted from host. |
| `TASK-152h` | Step 8 | `docs/07_implementation_status.md` | — | — | Acceptance ceremony. Confirm all acceptance commands and the workspace clippy/build are green. |

## Cross-Packet Mapping

- Packet 32 (`32_layer-collection-builder-wit-surface`) — must be `implemented` before this packet activates. This packet consumes packet 32's WIT resource (`set-entity-order` write + `get-ordered-entities` read), the `OrderedEntityView` SDK type, host validation helper (`apply_entity_order_proposal`), host read helper (`project_ordered_entities`), SDK builder, and macro plumbing.
- Packet 18 (`18_path-optimization-entity-ordering`) — flipped from `implemented` to `superseded` in Step 6. The NN algorithm is preserved verbatim; only its location moved. Packet 18's task closures (152a/d/e) stay `[x]` because the work they represented is still done.
- Packet 19 (mixed-tool ordering) — unblocked by this packet because the path-optimization module now owns the entity ordering surface end-to-end.
- Packet 21 (Benchy evidence) — unblocked because path-optimization output is now observable beyond comment markers (the `set_entity_order` call yields a different `LayerCollectionIR.ordered_entities` than raw assembly).

## Backlog Delta Summary

- Add row: `TASK-152h` under the `TASK-152` group in `docs/07_implementation_status.md`. Wording: *"Move the deterministic NN entity-ordering algorithm from `slicer-host::layer_executor::order_entities_by_nearest_neighbor` into `path-optimization-default::run_path_optimization` using the `layer-collection-builder` surface from packet 32. Delete the host helper. Mark packet 18 superseded."* Set status to `[x]` after Step 7.
- Update row: `TASK-152g` (introduced by packet 32, status `[~]`). Set status to `[x]` after Step 7 with a close note pointing at packet 33.
- Update row: `TASK-152` parent stays `[~]` (152b/c/f remain open).
- No other backlog row is modified by this packet.
