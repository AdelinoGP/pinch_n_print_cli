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

Close the remaining non-segmentation WIT-boundary gaps on live execution paths. Covers DEV-006. Specifically: pass real GCode command lists through `dispatch_postpass_gcode_call`, and add live-path boundary coverage for layer-world and finalization-world deep-copy behavior outside native fallback code.

## Scope Boundaries

- In scope: GCode command list boundary coverage (all `GCodeCommand` variants), layer-world deep-copy behavior, finalization-world deep-copy behavior, live-path regression for postpass WIT boundaries.
- Out of scope: Prepass segmentation boundary gaps (TASK-128 series), mesh-query host services (TASK-147/148), path-optimization behavior.

## Prerequisites and Blockers

- Depends on: TASK-124 (undeclared runtime access enforcement at WIT boundary), TASK-145 (WIT drift detection)
- Unblocks: TASK-120 (Benchy parity acceptance gate)
- Activation blockers: None

## Acceptance Criteria

- **Given** a GCodePostProcess module written via macro or raw WIT, **when** `dispatch_postpass_gcode_call` is invoked, **then** the `GCodeCommand` list passed across the boundary contains all command variants (Move with all field combinations, Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw) with correct field values preserved through the round trip. | `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
  - NOTE: On test pass, grep should confirm variants: `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw` appear in output.
- **Given** a `LayerCollectionIR` with `ordered_entities`, `tool_changes`, and `z_hops` is written by a layer-world module, **when** the layer-world deep-copy crosses the WIT boundary, **then** all entity fields (path points, role, region_key, topo_order, tool_change after_entity_index, z_hop hop_height) are preserved bit-for-bit. | `cargo test -p slicer-host --test layer_world_deep_copy_tdd 2>&1`
  - NOTE: On test pass, grep should confirm: `deep.copy.*pass` or `bit.for.bit` appears in output.
- **Given** a `LayerCollectionIR` is consumed by a finalization-world module, **when** the finalization-world deep-copy crosses the WIT boundary, **then** all layer indices, z values, ordered_entities, tool_changes, and z_hops are preserved bit-for-bit through the boundary. | `cargo test -p slicer-host --test finalization_world_deep_copy_tdd 2>&1`
  - NOTE: On test pass, grep should confirm: `finalization.*deep.copy.*pass` appears in output.
- **Given** `dispatch_postpass_gcode_call` processes a GCodeIR with `commands.len() > 0`, **when** the call returns, **then** the returned GCodeIR commands are in the same order with identical content and no command is silently dropped or mutated. | `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd 2>&1`
  - NOTE: On test pass, grep should confirm: `command.*preserved` or `order.*identical` appears in output.

## Negative Test Cases

- **Given** a GCodePostProcess module declares only `reads = ["GCodeIR.commands"]` but the live path passes an empty command list, **when** the module runs, **then** this is not a violation -- empty lists are valid and the module must handle them gracefully.

## Verification

- `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
  - NOTE: On pass, grep should confirm `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw` in output.
- `cargo test -p slicer-host --test layer_world_deep_copy_tdd 2>&1`
  - NOTE: On pass, grep should confirm `deep.copy.*pass` or `bit.for.bit` in output.
- `cargo test -p slicer-host --test finalization_world_deep_copy_tdd 2>&1`
  - NOTE: On pass, grep should confirm `finalization.*deep.copy.*pass` in output.
- `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd 2>&1`
  - NOTE: On pass, grep should confirm `command.*preserved` or `order.*identical` in output.
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
