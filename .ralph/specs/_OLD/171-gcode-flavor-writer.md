---
status: implemented
packet: 171-gcode-flavor-writer
task_ids:
  - TASK-276
---

# 171-gcode-flavor-writer

## Goal

Port OrcaSlicer's `GCodeWriter.cpp` per-flavor emission logic for five flavors (marlin, marlin2, klipper, reprapfirmware, repetier) into a `GcodeFlavor` dialect layer in `crates/slicer-gcode`, honored from the `gcode_flavor` config key (default marlin) and echoed as a real key in the CONFIG_BLOCK instead of the padded `"marlin"` literal.

## Problem Statement

PNP's G-code emission is pure Marlin: `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs:555-744`) hardcodes `M104`/`M109`, `M106 S`, `M82`/`M83`, `T<n>`, and `G10`/`G11` literals, and the only occurrence of `gcode_flavor` in the workspace is the cosmetic padding entry `("gcode_flavor", "marlin")` in `ORCA_CONFIG_PADDING` (`serialize.rs:403`). The OrcaSlicer fork frontend targets printers running Klipper, RepRapFirmware, Repetier, and modern Marlin; without a dialect layer, RRF printers receive wrong temperature commands and every future accel/jerk/pressure-advance emission feature would be born Marlin-only. This packet ports OrcaSlicer's `GCodeWriter.cpp` flavor branching for the five flavors the fork exposes, wired from config, in one coherent slice (handoff item 5, wave-2 plan `docs/specs/fork-gaps-wave2-plan.md`).

## Architecture Constraints

- Config key strings are snake_case: `gcode_flavor`, value strings `marlin|marlin2|klipper|reprapfirmware|repetier` (matching OrcaSlicer's config-enum spellings for the five supported variants).
- The dialect layer is a pure string-rendering layer over existing `GCodeCommand` variants — it must not change `GCodeIR`, WIT contracts, or any guest-visible schema. No guest WASM is rebuilt by this packet.
- Pure G-code text work: no geometry or mm/unit conversion is involved (coord-system snippet deliberately omitted).

## Data and Contract Notes

- IR/manifest contracts: `GCodeIR` and `GCodeCommand` are untouched; flavor is applied only at serialization.
- WIT boundary: none crossed; no guest rebuild required.
- Canonical divergence table (verified via delegated Orca survey this session; cite by file+function only):
  - `set_temperature`: RRF uses `G10` with `P<tool>` and appends `M116` on wait; Marlin/Marlin2/Klipper/Repetier use `M104`/`M109` with `T<tool> S<temp>` (`GCodeWriter.cpp::set_temperature`).
  - `set_acceleration_internal`: Repetier `M201 X.. Y..` (travel: `M202 X.. Y..`); Marlin2/RRF `M204 P..` (travel `M204 T..`); Klipper `SET_VELOCITY_LIMIT ACCEL=..`; legacy Marlin `M204 S..`.
  - `supports_separate_travel_acceleration`: true only for gcfRepetier, gcfMarlinFirmware, gcfRepRapFirmware.
  - `set_jerk_xy`: Klipper `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY=..`; Repetier `M207 X..`; others `M205 X.. Y..`.
  - `set_junction_deviation`: `M205 J..`, gcfMarlinFirmware only.
  - `set_pressure_advance`: Klipper `SET_PRESSURE_ADVANCE ADVANCE=..`; RRF `M572 D0 S..`; Repetier `M233 X.. Y..`; Marlin/Marlin2 `M900 K..`.
  - Fan `M106 S`, toolchange `T<n>`, `M82`/`M83`, firmware retract `G10`/`G11`, bed `M140`/`M190` are uniform across the five supported flavors (divergences exist only in flavors this packet excludes, e.g. MakerWare `M127`/`M126`, Machinekit `G22`/`G23`).
- Determinism: output remains a pure function of (IR, config); flavor comes from config only.

## Locked Assumptions and Invariants

- Default flavor is `Marlin` everywhere a serializer is constructed without config; all pre-existing emitted bytes are unchanged under the default (AC-6 falsifies this).
- Unknown `gcode_flavor` values never abort a slice — warn-and-default is locked behavior (AC-N1).
- The textual collision between firmware-retract `G10` and RRF's `G10 P.. S..` temperature command is accepted as canonical Orca behavior (RRF disambiguates by parameters); do not remap retract commands for RRF.

## Risks and Tradeoffs

- Merge risk with packet 167: both edit `ORCA_CONFIG_PADDING` and `serialize_config_block` call sites. Entries touched are disjoint (`gcode_flavor` here; speed/accel/jerk/`printer_model` there); the second packet to land rebases textually.
- RRF wait semantics: Orca appends `M116` (wait-for-all) rather than a blocking `G10 R`; the port copies Orca exactly — printer-side behavior differences are out of scope.
- `set_temperature` returning a multi-line string for RRF-wait must end each line with `\n` exactly once to keep golden-diff tooling stable; the dialect unit tests pin the exact strings.
- `serialize_config_block` signature change ripples into `ThumbnailAwareSerializer`; contained within `serialize.rs`.
