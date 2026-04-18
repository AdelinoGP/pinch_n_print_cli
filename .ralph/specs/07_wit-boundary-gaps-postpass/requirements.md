# Requirements: wit-boundary-gaps-postpass

## Packet Metadata

- Grouped task IDs:
  - `TASK-129` (parent — umbrella, closes when all children land)
  - `TASK-129a` (GCode command list boundary coverage)
  - `TASK-129b` (layer-world deep-copy boundary coverage)
  - `TASK-129c` (finalization-world deep-copy boundary coverage)
- Backlog source: `docs/07_implementation_status.md` (Workstream 2, lines 64-67)
- Packet status: `draft`

## Problem Statement

DEV-006 identifies that postpass GCode command content and executable WIT-boundary coverage still have live gaps. The three sub-tasks target distinct boundary surfaces:

- TASK-129a: `dispatch_postpass_gcode_call` in `crates/slicer-host/src/dispatch.rs` currently passes an empty `&[]` slice into the WIT call (line 707: `&[], own(output_handle)`) instead of the real command list. All `GCodeCommand` variant content (Move, Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw) must survive the WIT round trip.

- TASK-129b: Layer-world modules write `LayerCollectionIR` via output builders, but the reverse path (reading back a layer collection through the WIT boundary) has no live-path regression for deep-copy behavior. The `HostExecutionContext` layer-collection view and the per-layer arena must preserve all entity fields through the boundary.

- TASK-129c: Finalization-world modules consume `Vec<LayerCollectionIR>` through `world-finalization.wit`, but the deep-copy path from host IR to WIT view has no live-path regression. The full print collection must survive the boundary with all layer indices, z values, ordered_entities, tool_changes, and z_hops intact.

## In Scope

- GCodePostProcess WIT boundary: all 8 `GCodeCommand` variants (`Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw`) crossing the `dispatch_postpass_gcode_call` boundary in both directions
- Layer-world deep-copy boundary: `LayerCollectionIR` fields (`ordered_entities`, `tool_changes`, `z_hops`, `global_layer_index`, `z`, `annotations`) crossing from host arena through WIT view back to host
- Finalization-world deep-copy boundary: `Vec<LayerCollectionIR>` crossing through `world-finalization.wit` deep-copy interface
- Live-path regression for each boundary surface
- Negative case: empty command list is valid and must not cause a contract violation

## Out of Scope

- Prepass segmentation boundary gaps (TASK-128 series)
- Mesh-query host services (TASK-147 `raycast_z_down`, TASK-148 `surface_normal_at`)
- Path-optimization behavior (TASK-152 series)
- TextPostProcess boundary coverage (not yet on live path; covered by TASK-137)
- Legacy `Noop*Runner` fallback paths (TASK-139)

## Authoritative Docs

- `docs/01_system_architecture.md` (Tier 3 PostPass section, lines ~292-326)
- `docs/02_ir_schemas.md` (GCodeIR struct lines ~738-770, LayerCollectionIR lines ~1235-1252)
- `docs/03_wit_and_manifest.md` (WIT worlds: `world-postpass.wit`, `world-layer.wit`, `world-finalization.wit`)
- `docs/04_host_scheduler.md` (PostPass execution: `execute_postpass`, `dispatch_postpass_gcode_call`)
- `crates/slicer-host/src/dispatch.rs` (`dispatch_postpass_gcode_call` line 626)
- `crates/slicer-host/src/postpass.rs` (PostPass executor, `execute_postpass` line 163)
- `wit/deps/ir-types.wit` (gcode-output-builder lines 98-116)

## Acceptance Summary

### Positive Cases

- AC-1 (TASK-129a): All 8 `GCodeCommand` variants with all fields preserved through the WIT round trip when passed through `dispatch_postpass_gcode_call`
- AC-2 (TASK-129b): `LayerCollectionIR` entity fields (`path.points`, `role`, `region_key`, `topo_order`, `tool_change.after_entity_index`, `z_hop.hop_height`) preserved bit-for-bit through layer-world WIT boundary
- AC-3 (TASK-129c): `Vec<LayerCollectionIR>` all fields preserved bit-for-bit through finalization-world WIT boundary
- AC-4 (TASK-129a): Command order and content identical after round trip; no silent drop or mutation

### Negative Cases

- Empty `GCodeCommand` list is valid; module must handle gracefully (no contract violation)

### Measurable Outcomes

- Four new TDD test files (`postpass_gcode_boundary_tdd`, `layer_world_deep_copy_tdd`, `finalization_world_deep_copy_tdd`, `postpass_gcode_command_preservation_tdd`) passing with exact assertion content matching AC text
- `dispatch_postpass_gcode_call` passes real `GCodeCommand` list (not `&[]`) through the WIT boundary
- All commands survive round trip with identical content and order

## Verification Commands

- `cargo test -p slicer-host --test postpass_gcode_boundary_tdd`
- `cargo test -p slicer-host --test layer_world_deep_copy_tdd`
- `cargo test -p slicer-host --test finalization_world_deep_copy_tdd`
- `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd`
- `cargo clippy --workspace -- -D warnings` (workspace gate before commit)

## Step Completion Expectations

### Step 1 (TASK-129a): GCode command list real-data pass

- Precondition: `dispatch_postpass_gcode_call` passes `&[]` as the gcode command list
- Postcondition: `dispatch_postpass_gcode_call` passes the real `gcode_ir.commands` slice through the WIT boundary
- Falsifying check: `postpass_gcode_boundary_tdd` fails if any command variant is dropped or mutated

### Step 2 (TASK-129a): Round-trip regression test

- Precondition: No test exercises all 8 GCodeCommand variants crossing the WIT boundary
- Postcondition: `postpass_gcode_boundary_tdd` passes with exact field assertions for all variants
- Falsifying check: Any missing variant assertion causes test to fail

### Step 3 (TASK-129a): Command preservation regression test

- Precondition: No test confirms order/content preservation through `dispatch_postpass_gcode_call`
- Postcondition: `postpass_gcode_command_preservation_tdd` passes with order and content identical checks
- Falsifying check: Any reordering or mutation causes test to fail

### Step 4 (TASK-129b): Layer-world deep-copy live-path test

- Precondition: Layer-world deep-copy behavior only covered by native fallback code (not on live WASM path)
- Postcondition: `layer_world_deep_copy_tdd` passes with bit-for-bit field assertions for all `LayerCollectionIR` fields
- Falsifying check: Any field truncation or mutation causes test to fail

### Step 5 (TASK-129c): Finalization-world deep-copy live-path test

- Precondition: Finalization-world deep-copy behavior only covered by native fallback code (not on live WASM path)
- Postcondition: `finalization_world_deep_copy_tdd` passes with bit-for-bit field assertions for all `Vec<LayerCollectionIR>` fields across all layers
- Falsifying check: Any layer index, z value, or entity field change causes test to fail
