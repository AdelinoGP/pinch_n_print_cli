# Task Map: 53_gcode-cooling-fan-emission

Bridges this packet's steps back to `docs/07_implementation_status.md`, to DEV-009 in `docs/DEVIATION_LOG.md`, and to the superseded TASK-152c.

This packet introduces two new task IDs and supersedes one prior:

- **TASK-154** (new) — "Emit M106/M107 from a live finalization-stage cooling module".
- **TASK-152d** (new) — "Supersede TASK-152c; permit cooling on the finalization surface; preserve the path-optimization rejection".
- **TASK-152c** (existing, Closed 2026-04-29) — marked `Superseded by TASK-152d` in Step 5.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-154` | Step 1 — Discovery | none direct | none (pure-dispatch) | `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` (SUMMARY), `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (FACT) | S | FACTs and SUMMARY recorded into design.md. |
| `TASK-154` | Step 2 — Config keys + first tests | none direct | `crates/slicer-host/src/config_schema.rs`, `crates/slicer-host/tests/gcode_cooling_fan_emission_tdd.rs` (new) | none | S | Two named tests green. |
| `TASK-154` | Step 3 — Scaffold cooling module | `docs/03_wit_and_manifest.md` (manifest schema) | `modules/core-modules/cooling/{Cargo.toml, cooling.toml, src/lib.rs}` (new), `crates/slicer-host/src/dispatch.rs` (range `:2840-:2900`), `modules/core-modules/build-core-modules.sh` | none | M | Build script produces `cooling.wasm`. Dispatcher loads the module. Split into 3a/3b if it trends to L. |
| `TASK-154` | Step 4 — Algorithm + remaining tests | `docs/02_ir_schemas.md` (SUMMARY) | `modules/core-modules/cooling/src/lib.rs`, `crates/slicer-host/tests/gcode_cooling_fan_emission_tdd.rs` | CoolingBuffer SUMMARY from Step 1 | M | All 6 ACs + 3 negative cases green. |
| `TASK-152d` | Step 5 — Docs hygiene | `docs/05_module_sdk.md`, `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | docs only | none | S | TASK-152c marked Superseded; new TASK-152d + TASK-154 rows. DEV-009 progress entry. |

Aggregate context cost: M (no row is L).

## Why this packet is sufficient evidence for TASK-154 and TASK-152d

- TASK-154: ACs collectively prove (a) M106 emission after layer-2; (b) M107 before end-gcode; (c) `disable_fan_first_layers` honored; (d) `enable_overhang_fan` + `overhang_fan_speed` produce a bump-and-restore pattern; (e) eight cooling keys registered with the OrcaSlicer-derived defaults; (f) module is invoked by the host's finalization dispatcher.
- Negative cases prove `fan_speed_max = 0` produces a phantom-free output, that the module is required (missing-module regression), and that malformed config is rejected with a key-named error.
- TASK-152d: Step 5 produces the doc supersession trail (`docs/05_module_sdk.md` pointer + `docs/07` Superseded marker + `docs/DEVIATION_LOG.md` entry). Together these constitute the supersession record; no code change in TASK-152d itself.

## Relationship to DEV-009 and TASK-152c

- DEV-009 ("Benchy Phase H output is only partially correct on the live path") closes the cooling subset upon this packet's completion. Other DEV-009 subsets are closed by packet 52 (feedrate) and packet 54 (skirt-brim + relative-E).
- TASK-152c's rejection of cooling on the path-optimization surface is preserved verbatim. TASK-152d clarifies (does NOT contradict) by pointing at the accepted finalization surface.
