# Requirements: 171-gcode-flavor-writer

## Packet Metadata

- Grouped task IDs: `TASK-276` (new; minted at closure via `task-map.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

PNP's G-code emission is pure Marlin: `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs:555-744`) hardcodes `M104`/`M109`, `M106 S`, `M82`/`M83`, `T<n>`, and `G10`/`G11` literals, and the only occurrence of `gcode_flavor` in the workspace is the cosmetic padding entry `("gcode_flavor", "marlin")` in `ORCA_CONFIG_PADDING` (`serialize.rs:403`). The OrcaSlicer fork frontend targets printers running Klipper, RepRapFirmware, Repetier, and modern Marlin; without a dialect layer, RRF printers receive wrong temperature commands and every future accel/jerk/pressure-advance emission feature would be born Marlin-only. This packet ports OrcaSlicer's `GCodeWriter.cpp` flavor branching for the five flavors the fork exposes, wired from config, in one coherent slice (handoff item 5, wave-2 plan `docs/specs/fork-gaps-wave2-plan.md`).

## In Scope

- New `GcodeFlavor` enum in `crates/slicer-gcode` (new `src/flavor.rs`, re-exported from `lib.rs`): variants `Marlin` (Orca gcfMarlinLegacy; default), `Marlin2` (gcfMarlinFirmware), `Klipper`, `RepRapFirmware`, `Repetier`; `from_config_str` accepting exactly `"marlin"`, `"marlin2"`, `"klipper"`, `"reprapfirmware"`, `"repetier"`, warning and defaulting to `Marlin` on anything else.
- Dialect functions on `GcodeFlavor` for currently-emitted commands: `set_temperature(tool, celsius, wait)` (RRF: `G10 P<tool> S<temp>` + `M116` on wait; others: `M104`/`M109 T<tool> S<temp>`), fan (`M106 S` — uniform across the five), tool change (`T<n>` — uniform), extrusion mode (`M82`/`M83` — uniform), firmware retract/unretract (`G10`/`G11` — uniform).
- Dialect functions for flavor-divergent commands PNP does not emit yet, ported so future features are flavor-correct from day one: `set_acceleration` / `set_travel_acceleration` (M204 S / M204 P / M204 T / M201+M202 / `SET_VELOCITY_LIMIT ACCEL=`), `supports_separate_travel_acceleration` (true only for Repetier, Marlin2, RRF), `set_jerk_xy` (M205 X/Y; Repetier M207 X; Klipper `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY=`), `set_junction_deviation` (Marlin2-only `M205 J`), `set_pressure_advance` (Marlin M900 K; RRF M572 D0 S; Repetier M233 X Y; Klipper `SET_PRESSURE_ADVANCE ADVANCE=`), `set_bed_temperature` (`M140`/`M190` — uniform, ported for completeness).
- `DefaultGCodeSerializer` gains a `flavor: GcodeFlavor` field with a `with_flavor(...)` builder; the `Temperature` match arm (`serialize.rs:718-725`) routes through the dialect. Other current arms stay literal where the five flavors agree.
- Flavor threading: parse `gcode_flavor` from `config_source` next to the existing `use_relative_e_distances` read in `crates/slicer-runtime/src/run.rs:619-637` and construct the serializer with it.
- CONFIG_BLOCK echo: `serialize_config_block` (`serialize.rs:283`) emits the real resolved flavor as `; gcode_flavor = <value>` (raw_config value wins when present; resolved default otherwise); the `("gcode_flavor", "marlin")` padding entry at `serialize.rs:403` is removed.
- OrcaSlicer attribution header on `flavor.rs` per `docs/ORCASLICER_ATTRIBUTION.md`; all canonical citations by file+function name.
- New tests: `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs` (unit-level dialect matrix) and `crates/slicer-runtime/tests/integration/gcode_flavor_config_block_tdd.rs` (CONFIG_BLOCK echo), registered in the integration bucket harness.
- `docs/02_ir_schemas.md` note that `gcode_flavor` is now honored.

## Out of Scope

- Any new emission call sites for accel/jerk/pressure-advance/bed-temp (no module or emitter change generates these commands yet; the dialect layer just makes them available).
- Flavors beyond the five listed (no gcfSmoothie, gcfTeacup, gcfMakerWare, gcfSailfish, gcfMach3, gcfMachinekit, gcfNoExtrusion, BBL).
- The padding-table speed/accel/jerk key cleanup and `printer_model` synthesis — owned by packet `167-config-block-viewer-keys`; this packet touches only the `gcode_flavor` padding entry.
- M73 progress emission (packet 175) and time-estimator work (packet 169).
- Klipper macro-level features (START_PRINT macros, firmware retraction config) beyond command spelling.
- Changing `RetractMode` semantics or when firmware retraction is selected.

## Authoritative Docs

- `docs/ORCASLICER_ATTRIBUTION.md` - short; direct read for the exact header text.
- `docs/02_ir_schemas.md` - large; delegate a bounded lookup of the G-code serialization / CONFIG_BLOCK subsection.
- `docs/07_implementation_status.md` - always delegated; TASK-276 row added at closure.
- `docs/specs/fork-gaps-wave2-plan.md` - packet-171 section only (lines 23-27).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — per-flavor emission: `set_temperature` (RRF `G10 P<tool>`/`M116`), `set_acceleration_internal` (M204 S/P, M201/M202, SET_VELOCITY_LIMIT), `set_jerk_xy` (M205 X/Y, M207 X, SQUARE_CORNER_VELOCITY), `set_pressure_advance` (M900/M572/M233/SET_PRESSURE_ADVANCE), `supports_separate_travel_acceleration`, `set_junction_deviation` (Marlin2-only `M205 J`), plus the `FLAVOR_IS`/`FLAVOR_IS_NOT` branching macros.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `enum GCodeFlavor` variant list and the config-string spellings for the five supported flavors.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md`. Refinement: AC-6's byte-identity claim covers every pre-existing `slicer-gcode` serializer test, not only the golden file — the flavor field default must be `Marlin` in `Default`, `new()`, and `with_extrusion_mode()` constructors.
- Negative: `AC-N1`.
- Cross-packet impact: packet 167 also edits `ORCA_CONFIG_PADDING` in `serialize.rs`; whichever packet lands second rebases its padding-table edit (both remove/replace disjoint entries, so the merge is textual only). Packet 169/175 fixtures may later set `gcode_flavor` explicitly.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-gcode --test gcode_flavor_dialect_tdd 2>&1 \| tee target/test-output.log \| grep "^test result"` | Full dialect matrix: parse, temperature, accel, jerk, pressure-advance, capability, fallback | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p slicer-gcode 2>&1 \| tee target/test-output.log \| grep "^test result"` | All pre-existing serializer/emit tests still pass (default-Marlin byte identity) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_flavor_config_block 2>&1 \| tee target/test-output.log \| grep "^test result"` | CONFIG_BLOCK echoes real flavor; padding literal gone | FACT pass/fail |
| `cd F:/slicerProject/pinch_n_print && head -20 crates/slicer-gcode/src/flavor.rs \| grep -qi "OrcaSlicer" && ! grep -nE "GCodeWriter\.cpp:[0-9]" crates/slicer-gcode/src/flavor.rs && echo PASS \|\| echo FAIL` | Attribution header present; no line-pinned citations | FACT pass/fail |
| `cargo check --workspace --all-targets` | Whole-workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

The dialect layer (Step 2) must exist and be unit-tested before serializer wiring (Step 3) so wiring is a pure routing change; the CONFIG_BLOCK echo (Step 4) must land in the same packet as the runtime threading so the echoed value can never diverge from the dialect actually used.

## Context Discipline Notes

- `crates/slicer-gcode/src/serialize.rs` is 807 lines — read only the ranges named in `design.md`, never the whole file.
- `crates/slicer-runtime/src/run.rs` is large — read only lines 600-660 (serializer construction site).
- Orca `GCodeWriter.cpp` behavior is fully summarized in `design.md` §Data and Contract Notes; re-delegate only if an exact parameter spelling is in doubt.
