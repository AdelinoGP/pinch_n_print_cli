---
status: implemented
packet: 52_gcode-feedrate-emission
task_ids:
  - TASK-153
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 52_gcode-feedrate-emission

## Goal

Wire a per-role feedrate (F-token) into every emitted `G0`/`G1` print and travel move so the live emit path stops producing G-code whose only F value is the retract speed (`F25`). The Benchy reference output today carries no F on its 126,251 print moves; after this packet, every print and travel move either declares its own F or is preceded within the same role by an F directive.

## Scope Boundaries

- In scope:
  - Replace the three `f: None` assignments in `crates/slicer-host/src/gcode_emit.rs` (the print-move builder, the z-hop-up builder, the z-hop-down builder) with a resolved feedrate sourced from the per-role speed configuration.
  - Register per-role speed config keys in `crates/slicer-host/src/config_schema.rs` (outer_wall_speed, inner_wall_speed, thin_wall_speed, top_surface_speed, bottom_surface_speed, sparse_infill_speed, bridge_speed, internal_bridge_speed, support_speed, support_interface_speed, gap_infill_speed, ironing_speed, skirt_speed, wipe_tower_speed, prime_tower_speed, travel_speed, travel_speed_z, initial_layer_speed, initial_layer_infill_speed, initial_layer_travel_speed, wipe_speed, overhang_1_4_speed, overhang_2_4_speed, overhang_3_4_speed, overhang_4_4_speed, filament_ironing_speed) as `ConfigValue::Float` (mm/s) with OrcaSlicer-derived defaults (percentage-based defaults pre-resolved to absolute mm/s; overhang defaults = 0 = disabled; filament_ironing_speed = per-tool modifier defaulting to 0 = use global ironing_speed).
  - Consume `ExtrusionPath3D.speed_factor` (`slice_ir.rs:1297`) when resolving the F token, so module-side speed overrides flow through.
  - Add a TDD test file `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` covering positive and negative cases.
- Out of scope:
  - Cooling-driven speed modulation (acceleration/cooling — owned by packet 53).
  - Adaptive PA / dynamic per-move speed scaling beyond the existing `speed_factor` field.
  - Changes to `LayerPlanIR` / `LayerCollectionIR` field set; this packet reads the existing `speed_factor`, it does not introduce new IR fields.
  - Path-optimization or finalization mutation behavior (those are packets 33, 40, 41).

## Prerequisites and Blockers

- Depends on: none — surfaces verified to exist (`gcode_emit.rs:218`, `:282`, `:309`; `config_schema.rs:104-176`).
- Unblocks: packet 53 (cooling fan), which can then assume every move declares a baseline speed before the cooling module applies its `min_speed` clamp.
- Activation blockers: none open at draft time. The OrcaSlicer default-table reading in Step 1 is gated to a single delegated FACT dispatch, not a direct read.

## Acceptance Criteria

- **Given** the live Benchy emit path is run with the default speed profile, **when** every distinct F value in the produced G-code is enumerated, **then** the set contains at least two distinct values and at least one value > 600 mm/min. | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- distinct_feedrates_present --nocapture`
- **Given** any contiguous block of print moves in the produced G-code, **when** the block is scanned, **then** an F-token has been emitted within the preceding 200 lines. | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- f_token_within_200_lines --nocapture`
- **Given** a `ConfigView` that sets `outer_wall_speed = 30.0`, `inner_wall_speed = 60.0`, `sparse_infill_speed = 120.0`, **when** the serializer emits a perimeter region followed by an infill region, **then** the first `G1` of the outer-wall region carries `F1800`, the first `G1` of the inner-wall region carries `F3600`, and the first `G1` of the sparse-infill region carries `F7200`. | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- per_role_speed_resolves_to_f_token --nocapture`
- **Given** a `PrintEntity` whose `path.speed_factor = 0.5`, **when** that entity is serialized under a `outer_wall_speed = 60.0` config, **then** the emitted F is `F1800` (i.e. `60 * 60 * 0.5`). | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- speed_factor_modulates_role_speed --nocapture`
- **Given** a travel move carrying `f: Some(7200.0)` from an upstream module, **when** serialized, **then** the F-token equals `F7200` verbatim and the role-default `travel_speed` is not substituted. | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- module_supplied_f_wins --nocapture`
- **Given** `config_schema.rs`, **when** queried for `outer_wall_speed`, `inner_wall_speed`, `thin_wall_speed`, `top_surface_speed`, `bottom_surface_speed`, `sparse_infill_speed`, `bridge_speed`, `internal_bridge_speed`, `support_speed`, `support_interface_speed`, `gap_infill_speed`, `ironing_speed`, `skirt_speed`, `wipe_tower_speed`, `prime_tower_speed`, `travel_speed`, `travel_speed_z`, `initial_layer_speed`, `initial_layer_infill_speed`, `initial_layer_travel_speed`, `wipe_speed`, `overhang_1_4_speed`, `overhang_2_4_speed`, `overhang_3_4_speed`, `overhang_4_4_speed`, `filament_ironing_speed`, **then** each is registered as `ConfigValue::Float` with the OrcaSlicer default (mm/s) recorded in this packet's `requirements.md` Acceptance Summary. | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- speed_keys_registered_with_defaults --nocapture`
- **Given** a config where `filament_ironing_speed = 15.0` and `ironing_speed = 20.0`, **when** an Ironing role entity is serialized, **then** the emitted F uses `filament_ironing_speed` (F900) rather than `ironing_speed` (F1200). | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- filament_ironing_overrides_global_ironing --nocapture`
- **Given** a config where `wipe_speed = 96.0`, **when** a Custom("Wipe") role entity is serialized, **then** the emitted F is `F5760` (96 * 60). | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- wipe_speed_resolves_correctly --nocapture`

## Negative Test Cases

- **Given** the live emit path producing G-code whose only F value is `F25`, **when** the criteria above are evaluated, **then** the first criterion's assertion fails (set size ≥ 2 required). | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- rejects_only_retract_speed --nocapture`
- **Given** any print `G1` line that carries no F token AND has had no F directive within the preceding 200 lines, **when** scanned, **then** the second criterion's assertion fails (stale F at line N). | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- rejects_stale_f_window --nocapture`
- **Given** a non-bool / non-float value supplied for any of the registered speed keys, **when** the config is validated, **then** a `ConfigValidationError` with the offending key name is returned. | `cargo test -p slicer-host --test gcode_feedrate_emission_tdd -- rejects_non_float_speed_config --nocapture`

## Verification

- `cargo build -p slicer-host`
- `cargo test -p slicer-host --test gcode_feedrate_emission_tdd`
- `cargo test -p slicer-host --test gcode_emit_tdd` (regression: ensure existing G-code emission tests still pass with F-tokens)
- `cargo clippy -p slicer-host -- -D warnings`

## Authoritative Docs

- `docs/02_ir_schemas.md` — load lines covering `GCodeCommand::Move`, `TravelMove`, `ExtrusionPath3D.speed_factor`; delegate a SUMMARY if section > 200 lines.
- `docs/07_implementation_status.md` — delegate; locate the Phase H deviation entry (DEV-009) and confirm the new TASK-153 row insertion point.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — read directly (small); add a remediation-progress entry against DEV-009.
- `docs/08_coordinate_system.md` — read directly; confirm F-token formatting convention (mm/min for G-code output even though internal speed is mm/s).
- `docs/11_operational_governance_and_acceptance_gate.md` — delegate a SUMMARY of "objective output completeness" gate criteria.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — delegate; locate per-role `set_speed` call sites in `GCode::extrude_loop` / `extrude_path`.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — delegate; extract the `set_speed` formatting (rounding, mm/s → mm/min conversion).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` and `.cpp` — delegate; extract the default mm/s values for the speed keys listed in scope. Record verbatim in `requirements.md`.
- `OrcaSlicerDocumented/src/libslic3r/GCode/AdaptivePAProcessor.cpp` — delegate; confirm we are NOT porting adaptive PA in this packet (out-of-scope confirmation).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor `design.md`'s out-of-bounds list — `OrcaSlicerDocumented/`, `target/`, `Cargo.lock`, and unrelated crates must not be loaded directly;
- delegate every cargo run and every OrcaSlicer default-table lookup;
- stop reading at 60% context and hand off at 85%.

Aggregate context cost: M. No step is L. The largest single dispatch is the OrcaSlicer default-value lookup, which must return as a FACT block (≤ 16 lines) — never as snippets from the OrcaSlicer config tree.
