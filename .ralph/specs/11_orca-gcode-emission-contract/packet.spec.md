---
status: implemented
packet: orca-gcode-emission-contract
task_ids:
  - TASK-119
  - TASK-119a
  - TASK-119b
  - TASK-119c
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: orca-gcode-emission-contract

## Goal

Define and implement one canonical OrcaSlicer-compatible GCode emission contract on the live host postpass path, including layer-change comments, role-to-` ;TYPE:` labeling, and the emitted serialization rules for fill, support, seam-started wall loops, retract/unretract, and travel moves when those entities or decisions are present on the final path.

## Scope Boundaries

- In scope:
  - enumerate the Orca-compatible layer/comment contract in one host-owned constant/mapping surface for `DefaultGCodeEmitter` and `DefaultGCodeSerializer`
  - emit `;LAYER_CHANGE`, `;Z:<value>`, and `;HEIGHT:<value>` in canonical order ahead of each emitted layer block
  - emit canonical `;TYPE:` boundaries for `ExtrusionRole::OuterWall`, `InnerWall`, `TopSolidInfill`, `BottomSolidInfill`, `SparseInfill`, `BridgeInfill`, `SupportMaterial`, `SupportInterface`, `Skirt`, and `WipeTower`
  - preserve seam-started wall-loop emission instead of reshuffling wall starts on the emit path
  - serialize retract/unretract, travel, and Z-hop moves in one deterministic emitted order on the final text path
  - whole-postpass compatibility regressions from `LayerCollectionIR` through `execute_postpass()` to final text output
- Out of scope:
  - generating top/bottom fill geometry or deciding where those entities come from (TASK-120a)
  - restoring support geometry generation (TASK-120b)
  - producing `PerimeterRegion.resolved_seam` values or teaching path optimization to consume them (TASK-120c, TASK-151)
  - deciding retract/no-retract policy ownership (TASK-120d1)
  - generating SkirtBrim or WipeTower geometry (TASK-142, TASK-143)

## Prerequisites and Blockers

- Depends on:
  - no producer packet is required for the fixture-level emit tests in this packet
  - downstream producer packets (`12` through `20`) are required before full Benchy parity can satisfy the broader TASK-120 acceptance run
- Unblocks:
  - TASK-120a, TASK-120b, TASK-120c, TASK-120d2, TASK-142, TASK-143, and TASK-135 by giving them one canonical emitted-text contract
- Activation blockers:
  - None. The packet is `draft` by default, not because scope is unresolved.

## Acceptance Criteria

- **Given** two consecutive `LayerCollectionIR` entries with `global_layer_index=7`, `z=1.4` and `global_layer_index=8`, `z=1.6`, **when** `DefaultGCodeEmitter` and `DefaultGCodeSerializer` emit text, **then** the first layer block begins with exactly `;LAYER_CHANGE`, `;Z:1.4`, and `;HEIGHT:0.2` in that order before the first emitted `G1` line for layer `7`. | `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_layer_headers_before_first_extrusion -- --exact --nocapture`
- **Given** one emitted layer whose `ordered_entities[*].path.role` sequence crosses `OuterWall -> TopSolidInfill -> SparseInfill -> SupportMaterial -> SupportInterface -> Skirt -> WipeTower`, **when** the host serializes the layer, **then** it inserts role-boundary comments with exact labels `;TYPE:Outer wall`, `;TYPE:Top surface`, `;TYPE:Sparse infill`, `;TYPE:Support`, `;TYPE:Support interface`, `;TYPE:Skirt/Brim`, and `;TYPE:Prime tower` at the first command of each contiguous role block and never duplicates a label inside the same contiguous block. | `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_type_comments_at_role_boundaries -- --exact --nocapture`
- **Given** a wall-loop entity whose first `Point3WithWidth` is already the resolved seam start `(20.0, 10.0, 0.2)`, **when** the host serializes that wall loop, **then** the first extruding move for that loop is emitted at `X20 Y10 Z0.2` and the emit path does not prepend a travel-only move that changes the loop start point. | `cargo test -p slicer-host --test gcode_emit_tdd preserves_seam_started_wall_loop_order_in_output -- --exact --nocapture`
- **Given** a postpass command sequence containing `Retract { length: 0.8, speed: 1800.0 }`, one travel move with no `E`, one Z-hop up to `0.6`, one Z-hop return to `0.4`, and `Unretract { length: 0.8, speed: 1800.0 }`, **when** the final text is serialized, **then** it contains `G1 E-0.8 F1800`, a hop-up `G1 Z0.6`, the XY travel move without any `E`, a hop-down `G1 Z0.4`, and `G1 E0.8 F1800` in that exact order. | `cargo test -p slicer-host --test gcode_emit_tdd serializes_retract_travel_and_z_hop_in_canonical_order -- --exact --nocapture`
- **Given** a synthetic `LayerCollectionIR` fixture containing layer headers, role changes, comments, raw commands, retract/unretract, and tool changes, **when** `execute_postpass()` runs end-to-end through `DefaultGCodeEmitter`, `DefaultGCodeSerializer`, and any `PostPass::GCodePostProcess` modules, **then** the final text preserves the canonical Orca comment/order contract byte-for-byte across repeated runs. | `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd full_postpass_pipeline_preserves_orca_emission_contract -- --exact --nocapture`

## Negative Test Cases

- **Given** a layer whose entities contain only `OuterWall` and `SparseInfill` roles and whose postpass queue contains no retracts, unretracts, or support entities, **when** the host serializes the final text, **then** the output contains no `;TYPE:Support`, no `;TYPE:Support interface`, no `;TYPE:Skirt/Brim`, no `;TYPE:Prime tower`, and no retract line matching `G1 E-`. | `cargo test -p slicer-host --test gcode_emit_tdd omits_absent_role_labels_and_retraction_lines -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_layer_headers_before_first_extrusion -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd emits_orca_type_comments_at_role_boundaries -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd preserves_seam_started_wall_loop_order_in_output -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd serializes_retract_travel_and_z_hop_in_canonical_order -- --exact --nocapture`
- `cargo test -p slicer-host --test gcode_emit_tdd omits_absent_role_labels_and_retraction_lines -- --exact --nocapture`
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd full_postpass_pipeline_preserves_orca_emission_contract -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — post-finalization ownership and deterministic pipeline constraints
- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `GCodeIR`, `GCodeCommand`, and `ExtrusionRole` contracts
- `docs/04_host_scheduler.md` — PostPass order, host-built-in emit/serialize responsibilities
- `docs/07_implementation_status.md` — TASK-119 / TASK-119a / TASK-119b / TASK-119c scope and ordering

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeWriter.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/PostProcessor.hpp`
- `OrcaSlicerDocumented/tests/fff_print/test_gcode.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`