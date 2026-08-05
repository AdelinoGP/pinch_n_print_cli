---
status: implemented
packet: 171-gcode-flavor-writer
task_ids:
  - TASK-276
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 171-gcode-flavor-writer

## Goal

Port OrcaSlicer's `GCodeWriter.cpp` per-flavor emission logic for five flavors (marlin, marlin2, klipper, reprapfirmware, repetier) into a `GcodeFlavor` dialect layer in `crates/slicer-gcode`, honored from the `gcode_flavor` config key (default marlin) and echoed as a real key in the CONFIG_BLOCK instead of the padded `"marlin"` literal.

## Scope Boundaries

This packet adds a `GcodeFlavor` enum plus dialect functions in `crates/slicer-gcode` (new `flavor.rs`, wired into `serialize.rs`) and threads the parsed flavor from `config_source` at the serializer construction site in `crates/slicer-runtime/src/run.rs`. It covers the commands PNP emits today (M104/M109, M106, M82/M83, T\<n\>, G10/G11 firmware retract) and the flavor-divergent commands PNP does not emit yet (acceleration/jerk/pressure-advance family) so future emit features are flavor-correct. It does not add any new emission call sites for the not-yet-emitted commands, does not touch the padding-key cleanup owned by packet 167, and does not implement flavors beyond the five listed. Full lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: future accel/jerk/pressure-advance emission features; flavor-correct M73 work (packet 175 scope-adjacent, no hard dependency).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the strings `"marlin"`, `"marlin2"`, `"klipper"`, `"reprapfirmware"`, `"repetier"`, **when** `GcodeFlavor::from_config_str` parses each, **then** it returns the matching variant (`Marlin`, `Marlin2`, `Klipper`, `RepRapFirmware`, `Repetier`), and `GcodeFlavor::default()` is `Marlin`. | `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd -- flavor_parses_five_config_strings 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-2. Given** a `DefaultGCodeSerializer` constructed with `GcodeFlavor::RepRapFirmware`, **when** it serializes `GCodeCommand::Temperature { tool: 0, celsius: 210.0, wait: false }` and `{ wait: true }`, **then** the non-wait output line is `G10 P0 S210` (no `M104`) and the wait output is `G10 P0 S210` followed by an `M116` line (no `M109`), per canonical `GCodeWriter.cpp::set_temperature`. | `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd -- rrf_temperature_uses_g10_and_m116 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-3. Given** the dialect layer's `set_acceleration` for print acceleration 1000 mm/s², **when** each flavor renders it, **then** the outputs are exactly: Marlin `M204 S1000`, Marlin2 `M204 P1000`, RepRapFirmware `M204 P1000`, Repetier `M201 X1000 Y1000`, Klipper `SET_VELOCITY_LIMIT ACCEL=1000`, per canonical `GCodeWriter.cpp::set_acceleration_internal`. | `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd -- acceleration_dialect_per_flavor 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-4. Given** the dialect layer's travel-acceleration capability, **when** `supports_separate_travel_acceleration()` is queried, **then** it returns `true` for exactly `Repetier`, `Marlin2`, `RepRapFirmware` and `false` for `Marlin` and `Klipper`, per canonical `GCodeWriter.cpp::supports_separate_travel_acceleration`. | `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd -- travel_acceleration_capability_matrix 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-5. Given** a pipeline run whose `raw_config` contains `gcode_flavor = klipper` (string), **when** the CONFIG_BLOCK is serialized, **then** the block between `; CONFIG_BLOCK_START` and `; CONFIG_BLOCK_END` contains exactly one `; gcode_flavor = klipper` line and no `; gcode_flavor = marlin` line, and the `("gcode_flavor", "marlin")` entry is removed from `ORCA_CONFIG_PADDING`. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_flavor_config_block 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-6. Given** no `gcode_flavor` key in config, **when** G-code is emitted, **then** the default dialect is Marlin: temperature lines remain `M104 T.. S..` / `M109 T.. S..`, extrusion mode remains `M82`/`M83`, and the CONFIG_BLOCK contains `; gcode_flavor = marlin`, so all pre-existing golden emit output is byte-identical. | `mkdir -p target && cargo test -p slicer-gcode --test golden_emit_tdd 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-7. Given** the new `crates/slicer-gcode/src/flavor.rs`, **when** its header is inspected, **then** it begins with the standard OrcaSlicer porting attribution header per `docs/ORCASLICER_ATTRIBUTION.md` and cites `GCodeWriter.cpp` by file+function names only (no line-number citations anywhere in the file). | `cd F:/slicerProject/pinch_n_print && head -20 crates/slicer-gcode/src/flavor.rs | grep -qi "OrcaSlicer" && ! grep -nE "GCodeWriter\.cpp:[0-9]" crates/slicer-gcode/src/flavor.rs && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** `gcode_flavor = smoothie` (an unsupported flavor string), **when** `GcodeFlavor::from_config_str` parses it, **then** it falls back to `Marlin` (a `log::warn!` naming the rejected value is emitted) and the serialized output still uses `M104`/`M109` — the run never fails on an unknown flavor. | `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd -- unknown_flavor_falls_back_to_marlin 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd 2>&1 | tee target/test-output.log | grep "^test result"`

## Authoritative Docs

- `docs/ORCASLICER_ATTRIBUTION.md` - direct read (short); exact porting header text.
- `docs/07_implementation_status.md` - delegated; TASK-276 minted at closure via `task-map.md`.
- `docs/02_ir_schemas.md` - delegated bounded lookup of the CONFIG_BLOCK / GCodeIR command sections only.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "CONFIG_BLOCK" (or the G-code serialization subsection that documents `gcode_flavor`) — document that `gcode_flavor` is now a real honored key (5 supported values, default marlin) rather than cosmetic padding - `rg -q 'gcode_flavor' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — per-flavor emission: `set_temperature` (RRF `G10 P<tool>`/`M116`), `set_acceleration_internal` (M204 S/P, M201/M202, SET_VELOCITY_LIMIT), `set_jerk_xy` (M205 X/Y, M207 X, SQUARE_CORNER_VELOCITY), `set_pressure_advance` (M900/M572/M233/SET_PRESSURE_ADVANCE), `supports_separate_travel_acceleration`, `set_junction_deviation` (Marlin2-only `M205 J`), plus the `FLAVOR_IS`/`FLAVOR_IS_NOT` branching macros.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `enum GCodeFlavor` variant list and the config-string spellings for the five supported flavors.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
