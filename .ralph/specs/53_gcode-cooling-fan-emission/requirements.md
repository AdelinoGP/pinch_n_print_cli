# Requirements: 53_gcode-cooling-fan-emission

## Packet Metadata

- Grouped task IDs:
  - `TASK-154` (new — "Emit M106/M107 from a live finalization-stage cooling module")
  - `TASK-152d` (new — "Supersede TASK-152c; permit cooling on the finalization surface")
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The live G-code emit path produces zero `M106`/`M107` commands. Verified via reconnaissance:

- `GCodeCommand::FanSpeed { value: u8 }` exists in IR.
- `crates/slicer-host/src/gcode_emit.rs:466` serializes `FanSpeed` to `M106 S<n>` when present.
- The only call sites that CONSTRUCT `FanSpeed` are in test fixtures (`crates/slicer-sdk/src/postpass_builders.rs:141`) and the macro/dispatch plumbing (`crates/slicer-macros/src/lib.rs:826, :2895`). No live pipeline module produces `FanSpeed`.
- `crates/slicer-host/src/config_schema.rs` has no cooling-profile keys.
- `docs/07_implementation_status.md` records **TASK-152c (Closed 2026-04-29)** with rationale: "packet 19 documents fan-speed and cooling overrides as intentionally unsupported on the live `Layer::PathOptimization` surface; rejection wording locked in `docs/05_module_sdk.md` § Layer Stage Module Surface Rejections".

This packet supersedes TASK-152c by introducing a DIFFERENT surface — the finalization stage, parallel to the existing `SkirtBrim` and `WipeTower` finalization modules (`docs/05_module_sdk.md` § "Finalization Stage Module Surface"). The rejection on the path-optimization surface remains in force; what changes is that the finalization surface is now the documented home for cooling.

This packet is the second remediation against DEV-009 (Benchy live output partially correct).

## In Scope

- New crate `modules/core-modules/cooling/` implementing `FinalizationModule`.
- Eight cooling-profile keys registered in `config_schema.rs`.
- Dispatcher wiring in `crates/slicer-host/src/dispatch.rs` (range `:2840-:2900` only).
- Doc updates: `docs/05_module_sdk.md` § "Layer Stage Module Surface Rejections" (clarification only — add a pointer to the accepted finalization surface, do NOT remove the path-optimization rejection); `docs/07` rows for TASK-152d + TASK-154 and supersession marker on TASK-152c; `docs/DEVIATION_LOG.md` supersession + DEV-009 progress.
- TDD tests in `crates/slicer-host/tests/gcode_cooling_fan_emission_tdd.rs`.

## Out of Scope

- Per-segment speed slowdown algorithm (defers to a follow-up packet; this packet only registers the keys).
- `FanMover` parity — fan command relocation toward role edges.
- Adaptive PA.
- Acceleration token emission.
- Any change to the path-optimization surface; TASK-152c rejection on that surface is preserved verbatim.

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage role and module loading. Delegate a SUMMARY for the "FinalizationStage" section.
- `docs/02_ir_schemas.md` — `GCodeCommand::FanSpeed`, `LayerCollectionIR` builder API. Delegate a SUMMARY.
- `docs/03_wit_and_manifest.md` — module manifest TOML schema. Load directly the schema section.
- `docs/05_module_sdk.md` § "Finalization Stage Module Surface" + § "Layer Stage Module Surface Rejections" — load directly the affected lines (≤ 60 lines combined).
- `docs/07_implementation_status.md` — delegate; insert TASK-152d + TASK-154 rows and supersession marker on TASK-152c.
- `docs/11_operational_governance_and_acceptance_gate.md` — delegate a SUMMARY of "output completeness" criteria.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — load directly; supersession entry + DEV-009 progress note.

## OrcaSlicer Reference Obligations

All reads delegated.

- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` — cooling decision algorithm. SUMMARY ≤ 200 words.
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp` — public API surface. FACT, names of the 2–3 public methods this packet emulates.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — verbatim defaults for the eight registered keys. FACT, one row per key.
- `OrcaSlicerDocumented/src/libslic3r/GCode/FanMover.cpp` — explicitly NOT ported in this slice. SUMMARY ≤ 80 words confirming what is skipped.

## Acceptance Summary

Positive outcomes:

- ≥ 1 `M106 S<n>` (n > 0) after layer 2 — proves the module emits.
- `M107` (or `M106 S0`) before end-gcode — proves shutdown.
- `disable_fan_first_layers = 2` is honored.
- `enable_overhang_fan + overhang_fan_speed` injects an `M106 S<255>` at overhang region start and restores prior speed after.
- Eight cooling keys registered with OrcaSlicer-derived defaults (verbatim values to be recorded in `design.md` after the Step 1 dispatch returns).
- The dispatcher invokes the cooling module in the finalization stage, after `SkirtBrim`, before serialization.

Negative outcomes:

- `fan_speed_max = 0` → zero M106 with n>0; exactly one M107 in preamble.
- Module missing from host → no M106; the required-presence regression test fails as designed.
- Malformed cooling config (non-bool for `enable_overhang_fan`, non-int for `disable_fan_first_layers`) → `ConfigValidationError` naming key.

Measurable outcomes:

- New crate compiles to a `.wasm` artefact: `modules/core-modules/cooling/cooling.wasm`.
- ≥ 8 test functions in `gcode_cooling_fan_emission_tdd.rs`.
- `dispatch.rs` gains exactly one new branch in the finalization match (within `:2840-:2900`).
- `config_schema.rs` gains exactly eight registered fields.

Cross-packet impact:

- Closes the cooling subset of DEV-009.
- Blocks future per-layer-time slowdown packet (which depends on packet 52's feedrate emission).
- TASK-152c is marked superseded; the path-optimization rejection remains as documentation.

## Verification Commands

- `cargo test -p slicer-host --test gcode_cooling_fan_emission_tdd` — primary acceptance.
- `cargo test -p slicer-host --test orca_comment_contract_tdd` — regression.
- `./modules/core-modules/build-core-modules.sh` — produces the cooling `.wasm`. Dispatch as FACT (success/failure).
- `cargo check --workspace`.
- `cargo clippy --workspace -- -D warnings`.

## Step Completion Expectations

See `implementation-plan.md` for per-step Precondition / Postcondition / Falsifying check / Files-allowed / Files-edit / Expected dispatches / Cost. No step is L.

## Context Discipline Notes

- `crates/slicer-host/src/dispatch.rs` is > 2800 lines. Range-read `:2840-:2900` only.
- `modules/core-modules/skirt-brim/src/lib.rs` is the canonical reference template — load directly (small).
- `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.*` — full algorithm is large; the SUMMARY return is ≤ 200 words. Do not request snippets.
- Likely temptation reads to skip: full `dispatch.rs`, the OrcaSlicer FanMover source, the full TASK-152c packet (packet 19) directory — its conclusion has already been read; do not re-read.
- Sub-agent return formats:
  - Cooling defaults: FACT, ≤ 12 lines, `key = <value>` per row.
  - CoolingBuffer algorithm: SUMMARY ≤ 200 words.
  - module build script: FACT pass/fail.
  - cargo test runs: FACT pass/fail, SNIPPETS on failure.
