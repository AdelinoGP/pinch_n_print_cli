---
status: implemented
packet: path-optimization-tool-order-and-cooling-policy
task_ids:
  - TASK-152
  - TASK-152b
  - TASK-152c
---

# 19_path-optimization-tool-order-and-cooling-policy

## Goal

Add deterministic mixed-tool ordering inside `path-optimization-default` and close the cooling-override decision explicitly on the documentation rejection path. Tool ordering is implemented module-side using the `layer-collection-builder.set-entity-order` surface introduced by packet 32 (for the per-tool entity grouping) and the existing `gcode-output-builder.push-tool-change` method (for the deferred `ToolChange` records). The host-side `apply_entity_order_proposal` helper from packet 32 applies the proposal. Fan-speed and cooling overrides are documented as intentionally unsupported on the live `Layer::PathOptimization` surface.

## Problem Statement

Workstream 3 still lacks deterministic mixed-tool ordering, and TASK-152c is still unresolved. This packet resolves both together without widening scope into a new cooling API.

After packet `33`, entity-ordering ownership lives in `path-optimization-default` via `layer-collection-builder.set-entity-order`, and the host-side `order_entities_by_nearest_neighbor` no longer exists. Tool ordering is the same shape of work as entity ordering — it groups entities by `tool_index`, picks group order, and emits a single `ToolChange` record at each group boundary — so the natural home is the same module, using the same surfaces. The module computes a per-tool grouping permutation, calls `set_entity_order(items)` once with the grouped permutation, and calls `push_tool_change(from_tool, to_tool)` once per group boundary. The host applies the validated proposal (packet 32 already provides the validation and application logic) and the existing PathOptimization commit path appends `ToolChange` records to `LayerCollectionIR.tool_changes`. No new host helper, no new WIT member.

The cooling-override decision is closed on the documentation rejection path because the live module surface has no clean fan-speed/cooling control contract and adding one would reopen the postpass control plane.

## Architecture Constraints

- Selected approach: implement mixed-tool ordering **inside `path-optimization-default`**, layered on top of the entity-NN helper from packet `33`. The module computes a stable per-tool grouping permutation (group entities by `tool_index` ascending, preserving within-group NN order from packet `33`), then calls `collection.set_entity_order(items)` exactly once with the grouped permutation. For each group transition in the resulting sequence the module calls `output.push_tool_change(from_tool, to_tool)`.
- The host owns nothing new in this packet. `apply_entity_order_proposal` (packet `32`) validates and applies the permutation. The existing `Layer::PathOptimization` commit path appends `ToolChange` records to `LayerCollectionIR.tool_changes` from the `gcode-output-builder` queue.
- The packet must not add a new cooling/fan WIT or config surface.
- The packet must not reintroduce host-side ordering or tool-grouping helpers in `crates/slicer-host/src/layer_executor.rs` — that file's only role on the PathOptimization path is the raw `assemble_ordered_entities` call (post-packet-`33` state).
- The deferred `LayerCollectionIR.tool_changes` queue remains the only live tool-change emission surface.

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
