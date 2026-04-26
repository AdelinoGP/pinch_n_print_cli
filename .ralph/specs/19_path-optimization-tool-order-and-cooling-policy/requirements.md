# Requirements: path-optimization-tool-order-and-cooling-policy

## Packet Metadata

- Grouped task IDs:
  - `TASK-152` — expand path optimization beyond comment-only output for the tool-ordering slice
  - `TASK-152b` — emit deterministic tool-change ordering for mixed-tool layers
  - `TASK-152c` — close cooling override policy explicitly
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Workstream 3 still lacks deterministic mixed-tool ordering, and TASK-152c is still unresolved. This packet resolves both together without widening scope into a new cooling API.

After packet `33`, entity-ordering ownership lives in `path-optimization-default` via `layer-collection-builder.set-entity-order`, and the host-side `order_entities_by_nearest_neighbor` no longer exists. Tool ordering is the same shape of work as entity ordering — it groups entities by `tool_index`, picks group order, and emits a single `ToolChange` record at each group boundary — so the natural home is the same module, using the same surfaces. The module computes a per-tool grouping permutation, calls `set_entity_order(items)` once with the grouped permutation, and calls `push_tool_change(from_tool, to_tool)` once per group boundary. The host applies the validated proposal (packet 32 already provides the validation and application logic) and the existing PathOptimization commit path appends `ToolChange` records to `LayerCollectionIR.tool_changes`. No new host helper, no new WIT member.

The cooling-override decision is closed on the documentation rejection path because the live module surface has no clean fan-speed/cooling control contract and adding one would reopen the postpass control plane.

## In Scope

- mixed-tool grouping and deferred `ToolChange` sequencing **inside `modules/core-modules/path-optimization-default/src/lib.rs`**, using `set_entity_order` for the per-tool entity grouping and `push_tool_change` for the deferred `ToolChange` records
- docs-driven rejection path for cooling overrides

## Out of Scope

- generic entity ordering (covered by packets `18`, `32`, `33`)
- retraction policy and Z hops
- finalization-aware travel coordination
- adding new fan-speed/cooling config keys or WIT members
- any host-side ordering or tool-grouping helper in `crates/slicer-host/src/layer_executor.rs`

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/03_wit_and_manifest.md`
- `docs/04_host_scheduler.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp`

## Acceptance Summary

### Positive Cases

- Mixed-tool layers emit deterministic grouped tool ordering and deferred `ToolChange` entries.
- Single-tool layers emit no synthetic tool changes.
- Docs explicitly state the rejection path for cooling overrides.

### Negative Cases

- Canonical or single-tool sequences do not emit redundant tool changes.

### Measurable Outcomes

- Acceptance tests assert exact tool order and exact `ToolChange` sequence.
- The docs rejection path is verified by exact text grep, not implied prose.

### Cross-Packet Impact

- Packet `32` provides the `layer-collection-builder` resource and `apply_entity_order_proposal` helper this packet's grouping permutation flows through. Must be `implemented` first.
- Packet `33` provides the module-side ordering pattern this packet extends with per-tool grouping, and confirms `order_entities_by_nearest_neighbor` is gone from `slicer-host`. Must be `implemented` first.
- Packet `20` assumes tool ordering is deterministic when wipe geometry is present.
- Packet `21` uses this packet's decisions when asserting final Benchy travel and tool-change evidence.

## Verification Commands

- `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`
- `! grep -RIn "tool.*group\\|group.*by_tool\\|order_entities_by_tool" crates/slicer-host/src/layer_executor.rs`
- `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the mixed-tool or docs policy surface is isolated
- Postcondition: one exact tool-order or docs contract is observable
- Falsifying check: a focused sequence or grep assertion fails if the rule regresses