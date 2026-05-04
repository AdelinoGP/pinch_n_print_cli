---
status: implemented
packet: retraction-mode-firmware-vs-gcode
task_ids:
  - TASK-120d2
  - TASK-135
backlog_source: docs/07_implementation_status.md
supersedes:
  - 21_benchy-acceptance-evidence
---

## Goal

Add an OrcaSlicer-style retraction-mode toggle (`retract_mode`: `gcode` | `firmware`, default `gcode`) so the slicer can either emit travel retracts as inline E-axis moves (`G1 E-<len> F<speed>`) or as firmware retract/unretract opcodes (`G10` / `G11`). Reframe the currently failing E2E acceptance assertion to validate the default G-code path correctly, and add new tests that prove the firmware path emits `G10` / `G11` when the toggle is flipped. Match OrcaSlicer parity: M207/M208 are firmware *configuration* setters and are NOT emitted by this slicer; the firmware mode produces exactly `G10` (retract) and `G11` (unretract).

## Scope Boundaries

In scope:
- New `RetractMode` enum in `crates/slicer-ir/src/slice_ir.rs` and a `mode: RetractMode` field on `GCodeCommand::Retract` and `GCodeCommand::Unretract`.
- New `retract_mode` config field in `modules/core-modules/path-optimization-default/path-optimization-default.toml` and its read in `modules/core-modules/path-optimization-default/src/lib.rs`.
- Branch in `crates/slicer-host/src/gcode_emit.rs` that writes either `G1 E[-]<len> F<speed>` or `G10` / `G11` based on the per-command mode.
- Reframe of the existing failing test at `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_balanced_retract_and_unretract_pairs` to assert balanced G-code-mode retracts under the default config.
- Two new tests: one E2E proving firmware-mode emission of `G10`/`G11` with a config override, one negative test proving default mode emits no `G10`/`G11`.

Out of scope:
- Changing retract length, retract speed, z-hop, wipe, or deretraction-speed semantics (those remain on packet 15's contract).
- Adding `retraction_minimum_travel`, `retract_before_wipe`, or `retract_restart_extra` (separate future packets).
- Emitting `M207`/`M208` configuration setters (OrcaSlicer leaves those to printer start G-code; this packet preserves that convention).
- Per-extruder retraction state machines or multi-tool retract coordination.
- Path-optimization travel-ordering changes.

## Prerequisites and Blockers

- Packet 15 (`live-travel-retraction-policy`) is `implemented` and ships the current G-code-mode emission. This packet **extends** packet 15 with a firmware-mode branch and a config toggle; packet 15's behavior remains the default.
- Packet 21 (`benchy-acceptance-evidence`) carried the failing assertion as AC-3 with an incorrect M207/M208 expectation. Packet 21 will be marked `superseded` and packet 34 absorbs its retract/unretract acceptance evidence.

## Acceptance Criteria

- **AC-1 (default G-code mode emits balanced inline retracts):** **Given** an unmodified core-module config (no `retract_mode` override), **when** the Benchy E2E fixture runs through the live host pipeline and produces G-code, **then** the line count of `G1 E-` retract lines (lines starting with literal `G1 E-`) is `> 0`, the line count of paired unretract lines (`G1 E<positive> F<num>` lines emitted by `gcode_emit.rs:419-426` immediately following or balanced against retracts) is `> 0`, the two counts are exactly equal, and the file contains zero `G10` and zero `G11` lines. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture`

- **AC-2 (firmware mode emits balanced G10/G11):** **Given** the same Benchy fixture with the path-optimization-default module config overridden to `retract_mode = "firmware"`, **when** the live host pipeline runs, **then** the count of lines that equal exactly `G10` is `> 0`, the count of lines that equal exactly `G11` is `> 0`, the two counts are equal, AND the file contains zero `G1 E-` retract-style lines. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_firmware_retraction_emits_balanced_g10_g11 -- --exact --nocapture`

- **AC-3 (IR carries the mode):** **Given** the path-optimization-default module reads `retract_mode = "firmware"` from its config, **when** `run_path_optimization` pushes retract/unretract commands at `modules/core-modules/path-optimization-default/src/lib.rs:269-292`, **then** every emitted `GCodeCommand::Retract { mode, .. }` and `GCodeCommand::Unretract { mode, .. }` has `mode == RetractMode::Firmware`; conversely with `retract_mode = "gcode"` (or unset), every retract/unretract command has `mode == RetractMode::Gcode`. | `cargo test -p path-optimization-default retract_mode_propagates_into_ir_commands -- --exact --nocapture`

- **AC-4 (config schema enforces the enum):** **Given** the module manifest at `modules/core-modules/path-optimization-default/path-optimization-default.toml`, **when** the host loads the manifest at module-load time, **then** `[config.schema.retract_mode]` exists with `type = "enum"`, `values = ["gcode", "firmware"]`, `default = "gcode"`, and a `display` string; an attempt to set `retract_mode = "marlin"` (or any value outside the enum) at config-validation time produces a config-validation error that names the field `retract_mode` and the offending value. | `cargo test -p slicer-host config_schema_rejects_unknown_retract_mode -- --exact --nocapture`

- **AC-5 (emitter dispatches per command, not per file):** **Given** a synthetic `LayerCollectionIR` containing one `GCodeCommand::Retract { mode: RetractMode::Gcode, .. }` followed by one `GCodeCommand::Retract { mode: RetractMode::Firmware, .. }` (acceptance check only — production code never mixes), **when** `DefaultGCodeEmitter::emit_gcode` runs at `crates/slicer-host/src/gcode_emit.rs:96`, **then** the output contains exactly one `G1 E-` line and exactly one `G10` line, in that order, with no cross-mode bleed. | `cargo test -p slicer-host gcode_emit_dispatches_per_command_retract_mode -- --exact --nocapture`

## Negative Test Cases

- **NC-1 (no firmware-opcode bleed under default):** Default G-code-mode runs MUST NOT emit any `G10` or `G11` line; the assertion is `gcode.lines().filter(|l| l.trim() == "G10" || l.trim() == "G11").count() == 0`. Covered by AC-1's third clause and the dedicated assertion line in `benchy_gcode_contains_balanced_retract_and_unretract_pairs`.

- **NC-2 (no inline-E retract bleed under firmware mode):** Firmware-mode runs MUST NOT emit any `G1 E-` retract-style line; assertion `gcode.lines().filter(|l| l.starts_with("G1 E-")).count() == 0`. Covered by AC-2's fourth clause.

- **NC-3 (invalid enum value rejected at config load):** Config validation MUST reject `retract_mode = "marlin"` (or any non-enum value) with a diagnostic that names the field and value; covered by AC-4.

## Verification

Workspace-level checks (run after step-level verifications pass):

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd` (full Benchy E2E suite — guards against unintended regressions in other Benchy assertions while reframing one of them)

## Authoritative Docs

- `docs/02_ir_schemas.md` — `GCodeCommand` variants, enum versioning rules, and IR additivity.
- `docs/03_wit_and_manifest.md` — Module Manifest Schema (TOML), Config Field Types Reference (specifically the `enum` type with `values` array), and config-validation enforcement at the host boundary.
- `docs/05_module_sdk.md` — module config-read lifecycle (`on_print_start`) and how new fields are surfaced to the SDK.
- `docs/08_coordinate_system.md` — confirm retract `length` is mm at the IR boundary (already so per packet 15) so the new mode field is purely orthogonal.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `use_firmware_retraction` boolean definition (≈ line 5139). Confirms enum-equivalent toggle name; we use a string enum for richer future modes (e.g., `g10_with_m207_setter`) without breaking the manifest contract.
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `_retract()` branch on `use_firmware_retraction()`: firmware branch emits `G10` (retract) and `G11` (unretract); G-code branch emits inline E moves. This packet mirrors that branching.
- `OrcaSlicerDocumented/src/libslic3r/Extruder.cpp` / `Extruder.hpp` — confirms M207/M208 are NOT emitted by OrcaSlicer; firmware retraction parameters are configured via the printer's start G-code. This packet preserves that convention and intentionally does NOT emit `M207`/`M208`.

## Packet Files

- `packet.spec.md` (this file)
- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
