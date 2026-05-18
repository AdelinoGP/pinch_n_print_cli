# Task Map: path-optimization-tool-order-and-cooling-policy

This packet activates only after packets `32` (`32_layer-collection-builder-wit-surface`) and `33` (`33_path-optimization-module-ordering`) are `implemented`. Packet `32` provides the WIT surface (`layer-collection-builder.set-entity-order`) and host-side `apply_entity_order_proposal` helper. Packet `33` moves entity ordering into `path-optimization-default` and removes `order_entities_by_nearest_neighbor` from `slicer-host`. This packet extends the module's ordering with a per-tool grouping step and emits the deferred `ToolChange` sequence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-152b` | Step 1 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/tool_ordering_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` | Live-dispatch tests pinning grouped tool order and `ToolChange` records. Drive `path-optimization-default.wasm` through `WasmRuntimeDispatcher`, mirroring packet `33`'s pattern in `path_ordering_tdd.rs`. |
| `TASK-152b` | Step 2 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md` | `modules/core-modules/path-optimization-default/src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp` | Per-tool grouping inside the module: cluster by `tool_index`, NN within each cluster, ascending tool order globally; one `set_entity_order` call plus one `push_tool_change` per real boundary. |
| `TASK-152c` | Step 3 | `docs/05_module_sdk.md`, `docs/07_implementation_status.md` | `docs/05_module_sdk.md`, `docs/07_implementation_status.md` | `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp` | Closes the cooling override question explicitly on the documentation rejection path. |
| `TASK-152` | Steps 1-3 | `docs/07_implementation_status.md` | `modules/core-modules/path-optimization-default/src/lib.rs`, `docs/05_module_sdk.md`, `docs/07_implementation_status.md` | All of the above | The umbrella task closes only when both the module-side tool-ordering slice and the explicit cooling decision land. Parent stays `[~]` until TASK-152f closes (packet `20`). |
| `TASK-152b` (negative) | Step 1 | `docs/04_host_scheduler.md` | `crates/slicer-host/tests/tool_ordering_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` | Canonical or single-tool sequences must not emit redundant `ToolChange` entries. |

## Cross-Packet Mapping

- Packet `32` (`32_layer-collection-builder-wit-surface`) — must be `implemented`. Provides `set-entity-order` and `apply_entity_order_proposal`.
- Packet `33` (`33_path-optimization-module-ordering`) — must be `implemented`. Provides the module-side NN ordering this packet extends with per-tool grouping; confirms host has no ordering helpers left.
- Packet `20` (`20_finalization-aware-travel-coordination`) — depends on this packet for deterministic tool sequencing when wipe-related geometry is present.
- Packet `21` (Benchy evidence) — uses this packet's decisions when asserting final Benchy travel and tool-change evidence.

## Backlog Delta Summary

- Close `TASK-152b`: status `[x]` after Step 2 lands. Close note: *"Closed 2026-MM-DD — packet 19 implements per-tool grouping inside `path-optimization-default` using `set-entity-order` and `push-tool-change`; tests drive live WASM dispatch."*
- Close `TASK-152c`: status `[x]` after Step 3 lands. Close note: *"Closed 2026-MM-DD — packet 19 documents fan-speed and cooling overrides as intentionally unsupported on the live `Layer::PathOptimization` surface; rejection wording locked in `docs/05_module_sdk.md` and `docs/07_implementation_status.md`."*
- `TASK-152` parent: stays `[~]` after this packet (TASK-152f remains open and is the responsibility of packet `20`).
- No other backlog row is modified by this packet.
