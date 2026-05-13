---
status: draft
packet: 53_gcode-cooling-fan-emission
task_ids:
  - TASK-154
  - TASK-152d   # supersedes TASK-152c (closed 2026-04-29)
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 53_gcode-cooling-fan-emission

## Goal

Emit `M106 S<n>` and `M107` part-cooling fan commands on the live G-code emit path according to a cooling profile, by introducing a new finalization-stage WASM module (`cooling`) that consumes the finalized `LayerCollectionIR` and inserts `GCodeCommand::FanSpeed` entries on layer boundaries. This supersedes the prior decision (TASK-152c, closed 2026-04-29) that placed cooling on the rejected `Layer::PathOptimization` surface — the new surface is the finalization stage, parallel to `SkirtBrim` and `WipeTower`.

## Scope Boundaries

- In scope:
  - New crate `modules/core-modules/cooling/` containing a `FinalizationModule` implementation that reads a cooling profile from `ConfigView` and produces `GCodeCommand::FanSpeed { value }` events on the `LayerCollectionIR` at the correct layer boundaries.
  - Register cooling-profile keys in `crates/slicer-host/src/config_schema.rs`: `fan_speed_min`, `fan_speed_max`, `disable_fan_first_layers`, `enable_overhang_fan`, `overhang_fan_speed`, `slow_down_for_layer_cooling`, `slow_down_min_speed`, `slow_down_layer_time` — all `ConfigValue` of the appropriate primitive type with OrcaSlicer-derived defaults.
  - Wire the new module into the dispatcher in `crates/slicer-host/src/dispatch.rs` alongside the existing `dispatch_finalization_call` (at `:2854`).
  - Update `docs/05_module_sdk.md` § "Layer Stage Module Surface Rejections" to point at the new accepted surface and reference TASK-152d / packet 53.
  - Update `docs/07_implementation_status.md`: mark TASK-152c `Superseded by TASK-152d` and add a TASK-152d + TASK-154 row.
  - Update `docs/DEVIATION_LOG.md` with the supersession + DEV-009 remediation progress note.
  - Add TDD tests `crates/slicer-host/tests/gcode_cooling_fan_emission_tdd.rs` covering positive and negative cases.
- Out of scope:
  - Per-segment speed slowdown (`slow_down_for_layer_cooling` decision logic) — the keys are registered but the slowdown algorithm itself stays out (defer to a follow-up). M106/M107 emission is the entire delivery here.
  - Adaptive PA, fan-mover post-processing parity beyond first-layer-disable + overhang-bump.
  - Per-role acceleration tokens (M204).
  - Path-optimization-surface cooling hooks — explicitly rejected (TASK-152c stands on that surface).

## Prerequisites and Blockers

- Depends on:
  - Packet 52 (feedrate emission) — recommended but not strictly required. The cooling module mutates layer-boundary fan commands, not per-move speeds; however, the cross-packet `slow_down_*` follow-up will need packet 52 to be in place.
- Unblocks:
  - Future per-layer-time slowdown packet.
- Activation blockers:
  - Confirm with the user that updating `docs/05_module_sdk.md` § "Layer Stage Module Surface Rejections" to point at the new surface is the chosen path versus leaving the rejection in place and adding a new permitted section. (See `design.md` Open Questions.)

## Acceptance Criteria

- **Given** a print of > 5 layers run through the live emit path with `fan_speed_max = 255` and `disable_fan_first_layers = 1`, **when** the produced G-code is scanned, **then** at least one `M106 S<n>` with `n > 0` appears after the `;LAYER_CHANGE` marker for layer 2 (zero-indexed: appears after the second layer-change comment). | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- m106_present_after_layer_2 --nocapture`
- **Given** the last layer of the same print, **when** the tail of the G-code is scanned, **then** an `M107` (or `M106 S0`) appears before the end-gcode preamble. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- fan_off_before_end_gcode --nocapture`
- **Given** `disable_fan_first_layers = 2`, **when** the produced G-code is scanned for layers 0 and 1, **then** no `M106 S<n>` with `n > 0` appears in those layers. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- fan_disabled_on_first_layers --nocapture`
- **Given** `enable_overhang_fan = true` and `overhang_fan_speed = 100` (percent) with an overhang-marked region in the layer, **when** the G-code is scanned around that region, **then** an `M106 S255` (100% of 255) appears at the start of the overhang region and the previous fan speed is restored after the region. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- overhang_fan_bumped --nocapture`
- **Given** `config_schema.rs` queried for `fan_speed_min`, `fan_speed_max`, `disable_fan_first_layers`, `enable_overhang_fan`, `overhang_fan_speed`, `slow_down_for_layer_cooling`, `slow_down_min_speed`, `slow_down_layer_time`, **when** the registration is inspected, **then** each key exists with the type and OrcaSlicer-derived default recorded in this packet's `requirements.md`. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- cooling_keys_registered --nocapture`
- **Given** the new `cooling` module loaded into the host dispatcher, **when** a finalization-stage slice is run, **then** the dispatcher invokes the module's entry point after `SkirtBrim` and before `GCodeIR` serialization, and the layer-collection augmentation produced is non-empty whenever `fan_speed_max > 0`. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- cooling_module_invoked_in_finalization --nocapture`

## Negative Test Cases

- **Given** a config with `fan_speed_max = 0` (cooling effectively disabled), **when** the G-code is scanned, **then** zero `M106 S<n>` lines with `n > 0` appear and exactly one `M107` appears in the preamble (initial fan-off). | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- rejects_phantom_fan_when_disabled --nocapture`
- **Given** the live path WITHOUT the cooling module loaded (regression scenario for the prior intentionally-unsupported state), **when** the test loads a host with the module list missing `cooling`, **then** the produced G-code carries zero `M106` lines AND `gcode_cooling_fan_emission_tdd::m106_present_after_layer_2` fails — this is the "module-required" assertion. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- rejects_cooling_missing_when_required --nocapture`
- **Given** a malformed cooling config (e.g. `fan_speed_max = "high"`), **when** validation runs, **then** `ConfigValidationError` returns naming the offending key. | `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd -- rejects_malformed_cooling_config --nocapture`

## Verification

- `cargo build -p slicer-host -p cooling` (the new module crate).
- `./modules/core-modules/build-core-modules.sh` — required to produce the `.wasm` artefact for `cooling`.
- `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd`.
- `cargo test -p slicer-host --test orca_comment_contract_tdd` (regression).
- `cargo clippy --workspace -- -D warnings`.

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage role; delegate a SUMMARY for the "FinalizationModule" section.
- `docs/02_ir_schemas.md` — `GCodeCommand::FanSpeed`, `LayerCollectionIR`. Delegate a SUMMARY.
- `docs/03_wit_and_manifest.md` — WIT manifest TOML schema; read directly to author the new module manifest.
- `docs/05_module_sdk.md` — Layer Stage Module Surface Rejections section; load directly the affected lines (≤ 30) to author the supersession edit.
- `docs/07_implementation_status.md` — delegate; locate TASK-152c row and insert TASK-152d + TASK-154.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — load directly; record supersession + DEV-009 remediation.
- `docs/11_operational_governance_and_acceptance_gate.md` — delegate a SUMMARY of "output completeness" gate criteria.

## OrcaSlicer Reference Obligations

All reads delegated.

- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` — cooling decision algorithm: which fan speed to apply per layer given layer time, overhang, and slowdown thresholds.
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp` — public API + state machine.
- `OrcaSlicerDocumented/src/libslic3r/GCode/FanMover.cpp` — fan command placement (relocation toward edges to minimize transient). This packet does NOT port FanMover; mention as future work.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — verbatim defaults for the eight cooling keys above. Required as a FACT return.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

- `OrcaSlicerDocumented/` MUST be delegated.
- `crates/slicer-host/src/dispatch.rs` is large (> 2800 lines). Range-read `:2840-:2900` only; delegate any cross-file symbol lookup.
- `modules/core-modules/skirt-brim/src/lib.rs` is the reference template; load directly (small file per reconnaissance).
- Sub-agent return formats:
  - OrcaSlicer cooling-defaults: FACT, ≤ 12 lines, one row per key.
  - CoolingBuffer algorithm: SUMMARY ≤ 200 words; the packet does not port the whole algorithm in this slice.
  - cargo test runs: FACT pass/fail, SNIPPETS on failure ≤ 20 lines.

Aggregate context cost: M. No step is L. If Step 3 (the new module crate) trends toward L during implementation, split it into 3a (manifest + scaffolding) and 3b (algorithm) before proceeding.
