# Task Map: layer-collection-builder-wit-surface

This packet introduces a new sub-task `TASK-152g` under the existing `TASK-152` parent in `docs/07_implementation_status.md`. The parent stays `[~]` (partial) until packet 33 finishes the migration. The packet does **not** reopen or close any other backlog row; it only adds the new surface.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-152g` (new) | Step 1 | `docs/03_wit_and_manifest.md` | `wit/deps/ir-types.wit` | `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp` | Declares `resource layer-collection-builder` with one method `set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>`. |
| `TASK-152g` | Step 2 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `wit/world-layer.wit` | — | Adds `collection: layer-collection-builder` parameter to `run-path-optimization`. |
| `TASK-152g` | Step 3 | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `crates/slicer-host/src/wit_host.rs` | — | Host backing data, trait impl, resource constructor; stores proposal on `HostExecutionContext`. |
| `TASK-152g` | Step 4 | `docs/02_ir_schemas.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/lib.rs` | — | `apply_entity_order_proposal` validates length → range → duplicates and applies permutation + reversal + topo_order reassignment atomically. |
| `TASK-152g` | Step 5 | `docs/02_ir_schemas.md` | `crates/slicer-host/tests/layer_collection_builder_tdd.rs` (new) | — | Six host-validation tests covering positive permutation, reversal, three rejection cases, and atomicity. |
| `TASK-152g` | Step 6 | `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs` | — | Live PathOptimization dispatch pushes the new resource and applies the proposal post-call. Fallback still active. |
| `TASK-152g` | Step 7 | `docs/05_module_sdk.md` | `crates/slicer-sdk/src/postpass_builders.rs` (or new sibling), `crates/slicer-sdk/src/lib.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs` | — | SDK guest type, trait-method signature, macro embedded WIT, drain helper. |
| `TASK-152g` | Step 8 | `docs/05_module_sdk.md` | `modules/core-modules/path-optimization-default/src/lib.rs`, other in-tree `LayerModule::run_path_optimization` impls, `test-guests/*` | — | Sweep-update signatures (parameter unused). Rebuild WASM via `build-core-modules.sh`. |
| `TASK-152g` | Step 9 | `docs/03_wit_and_manifest.md` | `crates/slicer-host/tests/wit_drift_detection_tdd.rs` | — | Drift-detection asserts the macro embeds the new resource and updated export. |
| `TASK-152g` | Step 10 | `docs/07_implementation_status.md` | — | — | Acceptance ceremony, document `TASK-152g` row as `[~]` (parent `TASK-152` stays `[~]`). |

## Cross-Packet Mapping

- Packet 18 (`18_path-optimization-entity-ordering`) — stays `implemented`. Its acceptance tests must remain green throughout this packet because the host fallback is preserved.
- Packet 33 (`33_path-optimization-module-ordering`) — depends on this packet. Will migrate `path-optimization-default` to call `set_entity_order`, remove `order_entities_by_nearest_neighbor` from `crates/slicer-host/src/layer_executor.rs`, mark packet 18 `superseded`, and close `TASK-152g`.

## Backlog Delta Summary

- Add row: `TASK-152g` under the `TASK-152` group in `docs/07_implementation_status.md`, status `[~]`. Wording: *"Add `layer-collection-builder` WIT resource (`set-entity-order(items: list<tuple<u32, bool>>)`) and wire it through host bindings, SDK, and the `LayerModule::run_path_optimization` trait. Host validates and applies the proposal; host fallback preserved. Module migration deferred to packet 33."*
- No other backlog row is modified by this packet.
