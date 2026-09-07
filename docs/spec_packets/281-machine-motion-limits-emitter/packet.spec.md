---
status: draft
packet: 281-machine-motion-limits-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/54-author-packet-p47-printer-machine-motion-limits-emitter.md (wayfinder ticket 54)
context_cost_estimate: M
---

# Packet Contract: 281-machine-motion-limits-emitter

## Goal

Make the eight missing P47 motion-limit scalars (`machine_max_acceleration_x/y/z/e`, `machine_max_acceleration_retracting`, `machine_max_junction_deviation`, `machine_min_extruding_rate`, `machine_min_travel_rate`) drive live host-emitter behaviour by extending packet 267's machine-limit envelope and estimator clamp; nothing returns to the queue.

## Scope Boundaries

This packet adds eight `ResolvedConfig` scalar-global fields (canonical `coFloats` stride-2 pairs collapse to first-wins scalars per the ticket-140 precedent) plus `to_config_map` arms matching the ten live siblings, extends 267's envelope builder with the `M201` line, the `M204 R` component, and the `M205 J` line in canonical order and flavor forms, and extends `EstimatorLimits` with the two minimum-rate clamps. It does not declare `silent_mode` anywhere, adds no manifest row, no IR/WIT field, no module, and no CONFIG_BLOCK padding work.

## Prerequisites and Blockers

- Depends on: packet 267 (`docs/spec_packets/267-printer-machine-power-recovery-emitter/`) — the envelope builder (`M203`/`M204 P/T`/`M205` over the ten existing fields, the `emit_machine_limits_to_gcode` gate, the Marlin/Marlin2/RRF flavor gate) must exist in `crates/slicer-gcode` before this packet's lines can extend it. FORWARD-DEP: `emit_machine_limits_to_gcode` + the envelope builder ← produced by draft packet 267 (its AC-1 declares the gate, its AC-3/AC-4 the builder and order); names reconciled ✓.
- Unblocks: ticket 54's P47 packet-authoring closure; ticket 117 keeps the per-variant `silent_mode` model.
- Activation blockers: packet 267 implemented and merged; else this packet stays `draft` with `[BLOCK]` in `design.md`.

## Acceptance Criteria

- **AC-1. Given** `machine_max_acceleration_x = 1000`, `_y = 1000`, `_z = 500`, `_e = 5000` with flavor `marlin2` and `emit_machine_limits_to_gcode = true`, **when** `DefaultGCodeEmitter::emit_gcode` runs, **then** the command stream opens with the raw line `M201 X1000 Y1000 Z500 E5000` ahead of the `M203` line. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_m201 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** `machine_max_acceleration_extruding = 1500`, `machine_max_acceleration_retracting = 1200`, `machine_max_acceleration_travel = 800` with flavor `marlin2`, **when** `emit_gcode` runs, **then** the stream contains `M204 P1500 R1200 T800`; **when** flavor is `marlin` (legacy) the identical config emits `M204 P1500 R1200 T1500` (travel falls back to extruding). | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_m204r 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** `machine_max_junction_deviation = 0.05` with flavor `marlin2`, **when** `emit_gcode` runs, **then** the stream contains `M205 J0.05`; **when** the key is unset or `0`, no `M205 J` line is emitted. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_junction_deviation 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** a fixture with one 100 mm extruding move at programmed 100 mm/s, **when** estimated with `machine_min_extruding_rate = 0` versus `= 50`, **then** the second estimate clamps the segment to 50 mm/s and reports a strictly larger total time; the same holds for a travel move under `machine_min_travel_rate`. | `cargo test -p slicer-gcode --test estimator minimum_rate_clamp 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** all eight new fields set, **when** flavor is `klipper`, **then** none of `M201`/`M204 R`/`M205 J` is emitted; **when** `emit_machine_limits_to_gcode = false` under `marlin2`, none is emitted either. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_p47_flavor_gate 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** `machine_max_speed_x/y` set with flavor `reprapfirmware`, **when** `emit_gcode` runs, **then** the `M203` values are the configured mm/s values multiplied by 60 (mm/min) rounded with `+ 0.5`; the `M201` values are unscaled. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_rrf_factor 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-7. Given** `ResolvedConfig::default()` (all eight new fields `None`), **when** the default 20 mm-box slice runs before and after this packet, **then** the emitted G-code is byte-identical. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_p47_default_identity 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-8. Given** `ResolvedConfig::default()`, **when** its eight new fields are read, **then** each is `None`, each appears in `to_config_map` output only when set (absent when `None`), and the generated config reference documents all eight. | `cargo test -p slicer-ir --test resolved_config_defaults_tdd machine_motion_limits_defaults 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Negative Test Cases

- **AC-N1. Given** global configuration sets `machine_max_acceleration_x = -5` or `machine_max_junction_deviation = -0.1`, **when** resolution runs, **then** it fails with `slicer_ir::resolved_config::ConfigResolutionError::OutOfRange` rather than emitting a negative limit. | `cargo test -p slicer-scheduler --test integration machine_limits_range_rejection 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** `silent_mode` is returned to the queue (ticket 117), **then** no manifest table, `ResolvedConfig` field, or emitter read is added for it: `silent_mode` has zero occurrences in `crates/` and `modules/` after the packet. | `rg -q 'silent_mode' crates modules && echo FAIL || echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular-pipeline goals constraining the host-emitter owner choice)
- `docs/01_system_architecture.md` - delegated SUMMARY (host emitter seam vs module claim seams)
- `docs/specs/orca-feature-gap/map.md` - direct range read, Notes Authoring rules 1–6 + P47 queue rows

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` - regenerated host-keys section covers the eight new keys - `rg -q 'machine_max_acceleration_x' docs/15_config_keys_reference.md`
- `docs/07_implementation_status.md` - not edited by this packet (wayfinder queue owns the ledger; closure is recorded on ticket 54)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::print_machine_envelope` (canonical M201/M203/M204/M205 order, flavor gate, RRF ×60 factor, legacy-Marlin travel fallback)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::set_junction_deviation` (M205 J emission only when JD > 0) and the M204/M205 command forms
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — canonical defaults and minima for the eight new keys (accel x/y 1000, z 500, e 5000; retracting 1500; JD 0.01 max 0.3; min rates 0) and the `printer_options_with_variant_2` stride-2 set

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
