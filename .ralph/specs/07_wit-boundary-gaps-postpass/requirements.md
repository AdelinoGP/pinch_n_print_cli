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

- TASK-129a: `dispatch_postpass_gcode_call` in `crates/slicer-host/src/dispatch.rs` currently passes an empty `&[]` slice into the WIT call instead of the real command list, and the postpass WIT surface does not currently expose payload-bearing command input or explicit `Unretract` write support. This task widens `world-postpass.wit`, the shared `gcode-output-builder`, and the host/macro/SDK mirrors so all eight `GCodeCommand` variants can cross the live boundary with exact payloads.

- TASK-129b: Layer-world modules do not have a `LayerCollectionIR` read resource; they write through builder surfaces and the host commits those outputs into `LayerCollectionIR`. This task adds live-path regression coverage for that existing builder-to-arena-to-commit path so entity fields, tool changes, and z-hops are proven to survive the production layer-world boundary.

- TASK-129c: Finalization-world modules consume `Vec<LayerCollectionIR>` through `world-finalization.wit`, but the current `layer-collection-view` only exposes metadata (`layer-index`, `z`, `entity-count`, `tool-changes`). This task widens the finalization WIT surface and its mirrors so `ordered_entities` and `z_hops` cross the live boundary with exact field preservation.

## In Scope

- Postpass WIT evolution: payload-bearing command input for all 8 `GCodeCommand` variants and explicit `push-unretract` support on `gcode-output-builder`
- Postpass live-path runtime wiring: `dispatch_postpass_gcode_call` passes real `GCodeCommand` input instead of `&[]`
- Layer-world live builder/arena/commit boundary coverage for `ordered_entities`, `tool_changes`, and `z_hops`
- Finalization-world WIT evolution: `layer-collection-view` exposure for `ordered_entities` and `z_hops`
- Finalization-world deep-copy boundary coverage for completed-layer input
- Live-path regression for each boundary surface
- Negative case: empty command list is valid and must not cause a contract violation

## Out of Scope

- Prepass segmentation boundary gaps (TASK-128 series)
- Mesh-query host services (TASK-147 `raycast_z_down`, TASK-148 `surface_normal_at`)
- Path-optimization behavior (TASK-152 series)
- TextPostProcess boundary coverage (not yet on live path; covered by TASK-137)
- Legacy `Noop*Runner` fallback paths (TASK-139)
- Adding a new layer-world read resource for `LayerCollectionIR`

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

- AC-1 (TASK-129a): The postpass WIT input surface carries all 8 `GCodeCommand` variants with exact payload fields into guest-visible input
- AC-2 (TASK-129a): The postpass output builder carries `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, and `Raw` back to the host with identical order and content
- AC-3 (TASK-129b): The live layer-world builder/arena/commit path preserves `ordered_entities.path.points`, `role`, `region_key`, `topo_order`, `tool_changes.after_entity_index`, and `z_hops.hop_height`
- AC-4 (TASK-129c): The widened finalization-world boundary preserves `layer_index`, `z`, `ordered_entities`, `tool_changes`, and `z_hops` bit-for-bit

### Negative Cases

- Empty `GCodeCommand` list is valid; module must handle gracefully (no contract violation) | `cargo test -p slicer-host --test postpass_gcode_empty_list_tdd 2>&1`

### Measurable Outcomes

- `dispatch_postpass_gcode_call` passes real `GCodeCommand` input (not `&[]`) through the postpass WIT boundary
- `gcode-output-builder` exposes and commits explicit `Unretract` output on the live path
- Finalization `layer-collection-view` exposes `ordered_entities` and `z_hops` on the live path
- `postpass_gcode_boundary_tdd`, `postpass_gcode_command_preservation_tdd`, `postpass_gcode_empty_list_tdd`, `layer_world_deep_copy_tdd`, and `finalization_world_deep_copy_tdd` pass with exact assertion content matching AC text

## Verification Commands

- `cargo test -p slicer-host --test wit_drift_detection_tdd`
- `cargo test -p slicer-sdk --test postpass_module_tdd`
- `cargo test -p slicer-sdk --test finalization_module_tdd`
- `cargo test -p slicer-host --test postpass_gcode_boundary_tdd`
- `cargo test -p slicer-host --test layer_world_deep_copy_tdd`
- `cargo test -p slicer-host --test finalization_world_deep_copy_tdd`
- `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd`
- `cargo test -p slicer-host --test postpass_gcode_empty_list_tdd`
- `cargo clippy --workspace -- -D warnings` (workspace gate before commit)

## Step Completion Expectations

### Step 1 (TASK-129a): Postpass WIT surface widened

- Precondition: `world-postpass.wit`, host inline WIT, macro inline WIT, and SDK postpass types do not yet express payload-bearing postpass command input or explicit `Unretract` output
- Postcondition: canonical and mirrored WIT surfaces can express all eight `GCodeCommand` variants and `gcode-output-builder` includes `push-unretract`
- Falsifying check: `wit_drift_detection_tdd` or `postpass_module_tdd` fails if any mirrored WIT surface drifts or misses `Unretract`

### Step 2 (TASK-129a): Live postpass runtime wiring

- Precondition: `dispatch_postpass_gcode_call` still passes `&[]` or does not convert full command payloads
- Postcondition: the host passes real `gcode_ir.commands` through the widened WIT boundary and collects returned output including `Unretract`
- Falsifying check: `postpass_gcode_boundary_tdd` or `postpass_gcode_command_preservation_tdd` fails if any payload field, order, or `Unretract` output is lost

### Step 3 (TASK-129a): Postpass regressions

- Precondition: No focused regression proves live-path input payload preservation, output preservation, and empty-list handling together
- Postcondition: `postpass_gcode_boundary_tdd`, `postpass_gcode_command_preservation_tdd`, and `postpass_gcode_empty_list_tdd` pass on the live WASM path
- Falsifying check: Any missing variant, payload mismatch, order mutation, or empty-list trap fails the step

### Step 4 (TASK-129b): Layer-world builder-to-commit regression

- Precondition: No focused live-path regression proves that layer-world builder output survives the production arena/commit path into `LayerCollectionIR`
- Postcondition: `layer_world_deep_copy_tdd` passes with exact assertions for entity fields, tool changes, and z-hops on the real layer-world commit path
- Falsifying check: Any field truncation, mutation, drop, or reorder beyond documented `topo_order` semantics causes test to fail

### Step 5 (TASK-129c): Finalization WIT surface widened

- Precondition: `world-finalization.wit`, host inline WIT, and macro inline WIT expose only metadata-level `layer-collection-view` reads
- Postcondition: finalization modules can read `ordered_entities` and `z_hops` through widened `layer-collection-view` methods with mirrored host/macro bindings
- Falsifying check: `finalization_module_tdd` or `wit_drift_detection_tdd` fails if any widened method is missing or drifted

### Step 6 (TASK-129c): Finalization-world deep-copy live-path test

- Precondition: No focused live-path regression proves that widened finalization input preserves full completed-layer content across the WIT boundary
- Postcondition: `finalization_world_deep_copy_tdd` passes with bit-for-bit assertions for `layer_index`, `z`, `ordered_entities`, `tool_changes`, and `z_hops`
- Falsifying check: Any layer index, z value, entity field, tool-change field, or z-hop field change causes test to fail
