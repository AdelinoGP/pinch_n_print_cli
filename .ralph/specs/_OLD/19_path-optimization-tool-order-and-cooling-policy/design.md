# Design: path-optimization-tool-order-and-cooling-policy

## Controlling Code Paths

- Module ordering and tool-change emission: `modules/core-modules/path-optimization-default/src/lib.rs` — `run_path_optimization` body, alongside the entity-NN helper introduced by packet `33`.
- Host-side surfaces consumed (no edits expected): `crates/slicer-host/src/dispatch.rs` (`apply_entity_order_proposal` from packet `32`; the `Layer::PathOptimization` commit path that already appends `push-tool-change` to `LayerCollectionIR.tool_changes`).
- WIT surfaces consumed (no edits): `wit/deps/ir-types.wit` (`layer-collection-builder.set-entity-order`, `gcode-output-builder.push-tool-change`).
- SDK surfaces consumed (no edits): `slicer_sdk::LayerCollectionBuilder::set_entity_order`, `slicer_sdk::postpass_builders::GcodeOutputBuilder::push_tool_change`.
- Documentation surfaces: `docs/05_module_sdk.md`, `docs/07_implementation_status.md`.
- Neighboring tests or fixtures: new `crates/slicer-host/tests/tool_ordering_tdd.rs` (drives `path-optimization-default.wasm` through `WasmRuntimeDispatcher`, mirroring the live-dispatch pattern packet `33` established in `path_ordering_tdd.rs`).
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` (per-tool grouping shape) and `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp` (the rejected cooling control plane).

## Architecture Constraints

- Selected approach: implement mixed-tool ordering **inside `path-optimization-default`**, layered on top of the entity-NN helper from packet `33`. The module computes a stable per-tool grouping permutation (group entities by `tool_index` ascending, preserving within-group NN order from packet `33`), then calls `collection.set_entity_order(items)` exactly once with the grouped permutation. For each group transition in the resulting sequence the module calls `output.push_tool_change(from_tool, to_tool)`.
- The host owns nothing new in this packet. `apply_entity_order_proposal` (packet `32`) validates and applies the permutation. The existing `Layer::PathOptimization` commit path appends `ToolChange` records to `LayerCollectionIR.tool_changes` from the `gcode-output-builder` queue.
- The packet must not add a new cooling/fan WIT or config surface.
- The packet must not reintroduce host-side ordering or tool-grouping helpers in `crates/slicer-host/src/layer_executor.rs` — that file's only role on the PathOptimization path is the raw `assemble_ordered_entities` call (post-packet-`33` state).
- The deferred `LayerCollectionIR.tool_changes` queue remains the only live tool-change emission surface.

## Code Change Surface

- Selected approach:
  - extend `modules/core-modules/path-optimization-default/src/lib.rs` with a per-tool grouping step that runs **after** the existing entity-NN computation introduced in packet `33`. The module produces one final permutation that combines NN ordering within each tool group with per-tool group order globally.
  - emit one `push_tool_change(from_tool, to_tool)` per group transition in the final permutation.
  - add focused live-dispatch tests for mixed-tool ordering and redundant-tool-change suppression.
  - update docs to close TASK-152c explicitly on the rejection path.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs` — extend `run_path_optimization`. Likely shape:
    1. cluster regions by `tool_index`
    2. for each cluster, compute a within-cluster NN permutation (reusing the algorithm packet `33` ports into the module)
    3. concatenate clusters in ascending `tool_index` order to form the final permutation
    4. call `collection.set_entity_order(items)` once
    5. walk the permutation; whenever the tool index changes between consecutive entities, call `output.push_tool_change(prev_tool, next_tool)`
  - `crates/slicer-host/tests/tool_ordering_tdd.rs` (new) — driving `path-optimization-default.wasm` through `WasmRuntimeDispatcher`, asserting on the resulting `LayerCollectionIR.ordered_entities[*].tool_index` sequence (or equivalent module-tool tag) and `LayerCollectionIR.tool_changes` records.
  - `docs/05_module_sdk.md` — add the cooling-override rejection text (exact wording in implementation-plan Step 3).
  - `docs/07_implementation_status.md` — close TASK-152b and TASK-152c with packet-19 close notes; leave TASK-152 parent `[~]` until TASK-152f closes (packet `20`).
- Files explicitly **not** expected to change:
  - `crates/slicer-host/src/layer_executor.rs` (post-packet-`33` it owns no ordering logic)
  - `crates/slicer-host/src/dispatch.rs` (the PathOptimization commit path already accepts `push-tool-change`)
  - `wit/deps/ir-types.wit`, `wit/world-layer.wit` (no WIT widening)
- Rejected alternatives:
  - implementing tool grouping on the host alongside or instead of the module: rejected because packet `33` deleted the host's last ordering helper and the module already owns entity ordering. Reintroducing host-side grouping would re-create the architectural inversion that motivated packets `32`/`33`.
  - adding a `set-tool-order(items: list<u32>)` method to `layer-collection-builder`: rejected because the existing `set-entity-order` already carries enough information (the module's grouped permutation implicitly carries the tool ordering), and `push-tool-change` already exists for the per-boundary records.
  - adding a new live cooling override surface: rejected because it would widen the packet into postpass control and fan-speed semantics.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.ordered_entities` — populated post-call by `apply_entity_order_proposal` from the module's grouped permutation
  - `LayerCollectionIR.tool_changes` — populated by the existing host commit path for `push-tool-change` calls
  - docs-only rejection path for cooling overrides
- WIT boundary considerations:
  - no WIT widening; both methods used (`set-entity-order`, `push-tool-change`) are already declared in `wit/deps/ir-types.wit`
- Determinism or scheduler constraints:
  - identical mixed-tool inputs must produce identical `ordered_entities` and `tool_changes` sequences
  - within-tool ordering must remain deterministic and identical to packet `33`'s NN behavior on a single-tool subset
  - tool-group order is ascending `tool_index`; ties (same tool) collapse into one group

## Locked Assumptions and Invariants

- Tool ordering and entity ordering both live inside `path-optimization-default`. There is no host-side ordering helper after packet `33`; this packet does not reintroduce one.
- Cooling overrides are intentionally unsupported on the live path-optimization surface; the rejection text is the canonical answer.
- A single-tool layer produces zero `ToolChange` records.
- Already-grouped layers (entities already arranged in tool-ascending order) produce no extra `ToolChange` records beyond the necessary group boundaries.

## Risks and Tradeoffs

- Risk: per-tool grouping interacts with the within-cluster NN ordering and produces an implementation that differs from the single-tool packet-`33` result on a single-tool input. Mitigation: the `single_tool_layer_emits_no_synthetic_tool_changes` test pins the single-tool case; the implementation must reduce to packet-`33` NN behavior when only one tool is present.
- Risk: mixed-tool ordering can regress redundant tool changes. Mitigation: keep the negative suppression test `canonical_or_single_tool_sequences_emit_no_redundant_tool_changes`.
- Risk: docs-only rejection could drift from code behavior. Mitigation: grep-based acceptance and explicit wording in both docs surfaces.
- Risk: packet `19` activates before packets `32`/`33` close, breaking the assumed module-side surface. Mitigation: activation blocker is explicit in `packet.spec.md`; preflight should refuse to activate this packet until both predecessors are `implemented`.

## Open Questions

- None. The packet chooses module-side implementation for tool ordering (consistent with packets `32`/`33`) and documentation rejection for cooling overrides.
