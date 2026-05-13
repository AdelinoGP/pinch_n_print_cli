# Requirements: 52_gcode-feedrate-emission

## Packet Metadata

- Grouped task IDs:
  - `TASK-153` (new — to be inserted into `docs/07_implementation_status.md` under Phase H against DEV-009)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The live G-code emit path in `crates/slicer-host/src/gcode_emit.rs` constructs every print-move and z-hop `GCodeCommand::Move` with `f: None`. Verified at file:line:

- `gcode_emit.rs:218-228` — print-move builder (comment: "Feed rate could be calculated, but tests don't require it").
- `gcode_emit.rs:282` — z-hop up builder.
- `gcode_emit.rs:309` — z-hop down builder.

The serializer (`gcode_emit.rs:424-426`) writes an `F` token only when the field is `Some`, so the produced G-code carries an F-token only on travel moves that received an upstream `Some(...)` and on retract/unretract (`F25`). For Benchy this collapses to a single distinct F value (`F25`) — leaving firmware speed undefined for every print move.

Two upstream sources of speed exist but are unused at emit time:

- `ExtrusionPath3D.speed_factor: f32` at `crates/slicer-ir/src/slice_ir.rs:1297` — set by the layer/region executor (`layer_executor.rs:607` `drain_region_to_print_entities`) but never read.
- No per-role speed keys are registered in `crates/slicer-host/src/config_schema.rs` today (`config_schema.rs:104-176` defines only the `ConfigValue` enum and generic validation; no `outer_wall_speed`-style keys).

This packet closes the gap end-to-end: register the speed keys, resolve them in the emit builder using `(role, speed_factor)`, and serialize the F token on every print and travel move.

This packet does not reopen any prior packet. It is the first remediation against DEV-009 ("Benchy Phase H output is only partially correct on the live path") for the speed-token subset.

## In Scope

- Per-role speed config registration in `config_schema.rs`.
- F-token resolution in `gcode_emit.rs` for print, travel, and z-hop builders.
- Consumption of `ExtrusionPath3D.speed_factor` as a multiplier on the resolved role speed.
- New TDD test file `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs`.
- Documentation: DEV-009 progress note in `docs/DEVIATION_LOG.md` and a new TASK-153 row in `docs/07_implementation_status.md`.

## Out of Scope

- Cooling-profile speed modulation (packet 53).
- Adaptive PA (`AdaptivePAProcessor.cpp` parity).
- Acceleration tokens (`M204`) — distinct concern.
- Per-segment dynamic speed scaling beyond `speed_factor`.
- IR schema changes in `crates/slicer-ir/`.
- Path-optimization or finalization stage changes.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `GCodeCommand::Move`, `TravelMove`, `ExtrusionPath3D.speed_factor` definitions. Delegate a SUMMARY if the relevant section > 200 lines.
- `docs/08_coordinate_system.md` — load directly (small); confirm G-code feedrate unit convention (mm/min) versus internal mm/s.
- `docs/07_implementation_status.md` — delegate; insert TASK-153 row, do not load the full backlog.
- `docs/11_operational_governance_and_acceptance_gate.md` — delegate a SUMMARY of objective output-completeness gates.
- `docs/14_deviation_audit_history.md` and `docs/DEVIATION_LOG.md` — load directly; add DEV-009 progress entry.

## OrcaSlicer Reference Obligations

All reads delegated.

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — borrow the per-role speed lookup pattern in `GCode::extrude_loop` / `extrude_path`; we are NOT borrowing the AdaptivePA hooks.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — borrow the `set_speed` mm/s → mm/min conversion (`* 60`) and integer rounding rule. Record verbatim in design.md.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp`/`.cpp` — borrow the default values for the twenty-six registered keys. The delegation MUST return a FACT block of the form `key = <number> mm/s` for each.
- `OrcaSlicerDocumented/src/libslic3r/GCode/AdaptivePAProcessor.cpp` — explicitly NOT ported; the SUMMARY return must confirm we are skipping it.

## Acceptance Summary

Positive outcomes (each falsifiable in `gcode_feedrate_emission_tdd.rs`):

- Distinct F values in live Benchy output ≥ 2 and contain at least one value > 600 mm/min.
- Every print move has an F-token within 200 lines.
- `outer_wall_speed = 30 mm/s` produces `F1800` on the first wall move; `inner_wall_speed = 60 mm/s` → `F3600`; `sparse_infill_speed = 120 mm/s` → `F7200`. Conversion rule: `f_value_mm_per_min = round(speed_mm_per_s * 60 * speed_factor)`.
- `speed_factor = 0.5` halves the resolved F value.
- Module-supplied `f: Some(...)` is preserved verbatim and not overridden by the role default.
- All twenty-six speed keys registered as `ConfigValue::Float` with OrcaSlicer defaults (defaults recorded in `design.md`; percentage-based defaults pre-resolved to absolute mm/s since the frontend handles derivation; overhang defaults = 0 = disabled; filament_ironing_speed = per-tool modifier defaulting to 0 = use global ironing_speed). Keys: outer_wall_speed, inner_wall_speed, thin_wall_speed, top_surface_speed, bottom_surface_speed, sparse_infill_speed, bridge_speed, internal_bridge_speed, support_speed, support_interface_speed, gap_infill_speed, ironing_speed, skirt_speed, wipe_tower_speed, prime_tower_speed, travel_speed, travel_speed_z, initial_layer_speed, initial_layer_infill_speed, initial_layer_travel_speed, wipe_speed, overhang_1_4_speed, overhang_2_4_speed, overhang_3_4_speed, overhang_4_4_speed, filament_ironing_speed.

Negative outcomes:

- The "only F25" pattern fails the test.
- A stale F-window (> 200 lines without an F) fails the test.
- A non-float config value for any speed key returns `ConfigValidationError` naming the offending key.

Measurable outcomes:

- File: `gcode_feedrate_emission_tdd.rs` — at least 8 test functions, one per acceptance criterion and negative case above.
- `gcode_emit.rs` — the three `f: None` literals at `:228`, `:282`, `:309` are gone; replaced by `resolve_feedrate(role, speed_factor, &config)` helper that maps all 13 ExtrusionRole variants + travel to the 26 registered speed keys.
- `config_schema.rs` — twenty-six new `ConfigField` entries, each with `default: ConfigValue::Float(_)` and validation rejecting non-float values.

Cross-packet impact:

- Unblocks packet 53 (cooling): cooling module can now read a baseline F-token to clamp.
- Does not block packet 54 (skirt-brim + relative-E).

## Verification Commands

- `cargo test -p slicer-host --test gcode_feedrate_emission_tdd` — primary acceptance gate.
- `cargo test -p slicer-host --test gcode_emit_tdd` — regression; ensures `;TYPE:` labels and OrcaSlicer-canonical headers are still emitted alongside the new F-tokens (covers former TASK-119 / `emits_orca_*` cases).
- `cargo check --workspace` — fast type-check gate.
- `cargo clippy -p slicer-host -- -D warnings`.

All listed commands are delegation-friendly: the test binaries print one failing assertion per test on failure; on pass they print only the summary line.

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition / Postcondition / Falsifying check / Files-allowed / Files-edit / Expected dispatches / Cost — defined per step in `implementation-plan.md`.
- No step is `L`.

## Context Discipline Notes

- `OrcaSlicerDocumented/` MUST be delegated. The OrcaSlicer default-table lookup is the highest-risk dispatch — it MUST return as a FACT block listing `key = <number> mm/s` for each of the twenty-six keys, never as a code snippet.
- `crates/slicer-host/src/gcode_emit.rs` is > 600 lines; the implementer must range-read around `:200-:320` (move builders) and `:380-:480` (serializer) — never load in full.
- `crates/slicer-ir/src/slice_ir.rs` is > 1500 lines; range-read `:1280-:1330` (ExtrusionPath3D) and `:1460-:1530` (TravelMove + LayerCollectionIR) only.
- Likely temptation reads to skip: `crates/slicer-host/src/dispatch.rs` (large; not on this packet's path), `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer*` (unrelated), the full `docs/07_implementation_status.md` (delegate via subject query only).
- Sub-agent return-format hints:
  - OrcaSlicer default lookup → FACT, ≤ 16 lines, format `key = <number> mm/s` per row, no prose.
  - `cargo test` runs → FACT (pass) or SNIPPETS (≤ 20 lines per failing assertion).
  - Backlog row insertion → no return value other than `EDITED` / `NOT-EDITED`.
