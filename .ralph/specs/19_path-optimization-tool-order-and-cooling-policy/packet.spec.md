---
status: draft
packet: path-optimization-tool-order-and-cooling-policy
task_ids:
  - TASK-152
  - TASK-152b
  - TASK-152c
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: path-optimization-tool-order-and-cooling-policy

## Goal

Add deterministic mixed-tool ordering inside `path-optimization-default` and close the cooling-override decision explicitly on the documentation rejection path. Tool ordering is implemented module-side using the `layer-collection-builder.set-entity-order` surface introduced by packet 32 (for the per-tool entity grouping) and the existing `gcode-output-builder.push-tool-change` method (for the deferred `ToolChange` records). The host-side `apply_entity_order_proposal` helper from packet 32 applies the proposal. Fan-speed and cooling overrides are documented as intentionally unsupported on the live `Layer::PathOptimization` surface.

## Scope Boundaries

- In scope:
  - deterministic per-tool entity grouping computed inside `modules/core-modules/path-optimization-default/src/lib.rs`, emitted via `LayerCollectionBuilder::set_entity_order`
  - deferred `LayerCollectionIR.tool_changes` population by calling `GcodeOutputBuilder::push_tool_change(from_tool, to_tool)` at each tool-group boundary the module produces
  - one explicit decision for TASK-152c: cooling and fan-speed overrides remain intentionally unsupported on the live path-optimization surface
  - documentation updates in `docs/05_module_sdk.md` and `docs/07_implementation_status.md` that lock the rejection path for cooling overrides
- Out of scope:
  - generic entity ordering heuristics (packets `18`, `32`, `33`)
  - retract/no-retract policy (packet `15`)
  - finalization-aware wipe/brim travel coordination (packet `20`)
  - implementing a new cooling or fan-control WIT/config surface
  - any host-side ordering logic in `crates/slicer-host/src/layer_executor.rs` (deleted by packet `33` and not reintroduced here)

## Prerequisites and Blockers

- Depends on:
  - packet `32` (`32_layer-collection-builder-wit-surface`) — provides the `layer-collection-builder` resource, host validation, dispatch wiring, SDK builder, and macro plumbing this packet's tool grouping uses
  - packet `33` (`33_path-optimization-module-ordering`) — establishes the module-side ordering pattern that this packet extends with per-tool grouping; also confirms `order_entities_by_nearest_neighbor` is gone from `slicer-host`
  - existing `gcode-output-builder.push-tool-change(from-tool, to-tool)` method already accepted by host PathOptimization dispatch (commits to `LayerCollectionIR.tool_changes`)
- Unblocks:
  - packet `20` and packet `21`, which need deterministic tool sequencing when wipe-related geometry is present
- Activation blockers:
  - packets `32` and `33` must be `implemented` before this packet activates. Until then this packet stays `draft`.

## Acceptance Criteria

- **Given** a layer fixture whose raw entities use tool indices `0`, `2`, `1` (in that raw assembly order), **when** `path-optimization-default.wasm` dispatches through `WasmRuntimeDispatcher`, **then** the resulting `LayerCollectionIR.ordered_entities[*].tool_index` (or equivalent module-tool tag) sequence is exactly tool `0` entities first, then tool `1`, then tool `2`, and the deferred `LayerCollectionIR.tool_changes` sequence is exactly `[0→1, 1→2]`. | `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
- **Given** a layer fixture whose entities all use tool `0`, **when** the live module dispatches, **then** `LayerCollectionIR.tool_changes` is empty (length 0) and `ordered_entities` is unchanged from raw assembly order. | `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
- **Given** the module's tool-grouping is computed inside `modules/core-modules/path-optimization-default/src/lib.rs`, **when** the slicer-host source is grepped for any host-side tool-grouping helper, **then** zero matches are found in `crates/slicer-host/src/layer_executor.rs`. | `! grep -RIn "tool.*group\\|group.*by_tool\\|order_entities_by_tool" crates/slicer-host/src/layer_executor.rs`
- **Given** TASK-152c is closed on the rejection path, **when** `docs/05_module_sdk.md` and `docs/07_implementation_status.md` are inspected, **then** both docs state that fan-speed and cooling overrides are intentionally unsupported on the live `Layer::PathOptimization` surface and no new live-path cooling override surface is introduced in this packet. | `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`

## Negative Test Cases

- **Given** a single-tool layer or a mixed-tool layer already grouped in canonical order, **when** the ordering helper runs, **then** it does not emit redundant tool changes or reorder the already canonical tool grouping. | `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
- `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`
- `! grep -RIn "tool.*group\\|group.*by_tool\\|order_entities_by_tool" crates/slicer-host/src/layer_executor.rs`
- `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh`

## Authoritative Docs

- `docs/01_system_architecture.md` — path-optimization and tool-change ownership; module owns ordering after packet `33`
- `docs/03_wit_and_manifest.md` — `gcode-output-builder.push-tool-change` and `layer-collection-builder.set-entity-order` contracts
- `docs/04_host_scheduler.md` — deferred tool-change queue behavior on the PathOptimization stage
- `docs/05_module_sdk.md` — module-surface documentation for rejection path
- `docs/07_implementation_status.md` — TASK-152b and TASK-152c closure notes

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`