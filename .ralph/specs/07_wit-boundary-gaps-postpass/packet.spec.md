---
status: draft
packet: wit-boundary-gaps-postpass
task_ids:
  - TASK-129
  - TASK-129a
  - TASK-129b
  - TASK-129c
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: wit-boundary-gaps-postpass

## Goal

Close the remaining non-segmentation WIT-boundary gaps on live execution paths. Covers DEV-006. Specifically: widen the postpass WIT surface so full `GCodeCommand` payloads and explicit `Unretract` writes cross the boundary, widen the finalization WIT surface so completed layers expose `ordered_entities` and `z_hops`, and add live-path regression coverage for the existing layer-world builder-to-commit path.

## Scope Boundaries

- In scope: postpass WIT evolution for full `GCodeCommand` payload input, `gcode-output-builder` `push-unretract` support, layer-world live builder-to-commit boundary coverage, finalization-world `ordered_entities` and `z_hops` exposure, and live-path regression tests for each surface.
- Out of scope: Prepass segmentation boundary gaps (TASK-128 series), mesh-query host services (TASK-147/148), path-optimization behavior.

## Prerequisites and Blockers

- Depends on: TASK-124 (undeclared runtime access enforcement at WIT boundary), TASK-145 (WIT drift detection)
- Unblocks: TASK-120 (Benchy parity acceptance gate)
- Activation blockers: None

## Acceptance Criteria

- **Given** a GCodePostProcess module written via macro or raw WIT, **when** `dispatch_postpass_gcode_call` is invoked with representative `GCodeCommand` input, **then** the postpass WIT boundary carries all eight command variants (`Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw`) with their exact payload fields preserved into guest-visible input. | `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
  - NOTE: On test pass, grep should confirm variants: `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw` appear in output.
- **Given** a GCodePostProcess module emits `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, and `Raw` through `gcode-output-builder`, **when** the call returns, **then** the host preserves returned commands in the same order with identical content and no command is silently dropped or mutated. | `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd 2>&1`
  - NOTE: On test pass, grep should confirm: `command.*preserved` or `order.*identical` appears in output.
- **Given** a layer-world module writes entities via the existing builder surfaces and `Layer::PathOptimization` emits tool changes and z-hops, **when** the live layer-world path commits outputs into `LayerCollectionIR`, **then** `ordered_entities.path.points`, `role`, `region_key`, `topo_order`, `tool_changes.after_entity_index`, and `z_hops.hop_height` are preserved bit-for-bit. | `cargo test -p slicer-host --test layer_world_deep_copy_tdd 2>&1`
  - NOTE: On test pass, grep should confirm: `deep.copy.*pass` or `bit.for.bit` appears in output.
- **Given** a finalization-world module consumes completed layers through `layer-collection-view`, **when** the finalization-world deep-copy crosses the WIT boundary, **then** `layer_index`, `z`, `ordered_entities`, `tool_changes`, and `z_hops` are preserved bit-for-bit through the boundary. | `cargo test -p slicer-host --test finalization_world_deep_copy_tdd 2>&1`
  - NOTE: On test pass, grep should confirm: `finalization.*deep.copy.*pass` appears in output.

## Negative Test Cases

- **Given** a GCodePostProcess module declares only `reads = ["GCodeIR.commands"]`, **when** the live path passes an empty command list, **then** this is not a violation and the module handles the empty list without trapping or mutating output state. | `cargo test -p slicer-host --test postpass_gcode_empty_list_tdd 2>&1`

## Verification

- `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
  - NOTE: On pass, grep should confirm `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw` in output.
- `cargo test -p slicer-sdk --test postpass_module_tdd 2>&1`
- `cargo test -p slicer-sdk --test finalization_module_tdd 2>&1`
- `cargo test -p slicer-host --test layer_world_deep_copy_tdd 2>&1`
  - NOTE: On pass, grep should confirm `deep.copy.*pass` or `bit.for.bit` in output.
- `cargo test -p slicer-host --test finalization_world_deep_copy_tdd 2>&1`
  - NOTE: On pass, grep should confirm `finalization.*deep.copy.*pass` in output.
- `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd 2>&1`
  - NOTE: On pass, grep should confirm `command.*preserved` or `order.*identical` in output.
- `cargo test -p slicer-host --test postpass_gcode_empty_list_tdd 2>&1`
  - NOTE: On pass, grep should confirm `empty.*valid` in output.
- `cargo test -p slicer-host --test wit_drift_detection_tdd 2>&1`
- `cargo clippy --workspace -- -D warnings` (workspace gate)

## Authoritative Docs

- `docs/01_system_architecture.md` (PostPass Tier 3 section, lines ~292-326)
- `docs/02_ir_schemas.md` (GCodeIR struct lines ~738-770, LayerCollectionIR)
- `docs/03_wit_and_manifest.md` (WIT worlds, manifest contracts)
- `docs/04_host_scheduler.md` (PostPass execution, execute_postpass function)

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
