# Task Map: 52_gcode-feedrate-emission

Bridges this packet's steps back to `docs/07_implementation_status.md` and to DEV-009 in `docs/DEVIATION_LOG.md`.

This packet introduces **TASK-153** (new). The row will be appended under Phase H during Step 5; the slug is "Per-role feedrate emission on live G-code path".

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-153` | Step 1 — Discovery | `docs/02_ir_schemas.md` (SUMMARY) | none (pure-dispatch) | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (FACT), `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` (FACT) | S | Discovery proves the config handle type and records the twenty-six Orca default mm/s values + rounding rule into this packet's `design.md`. |
| `TASK-153` | Step 2 — Failing TDD tests | `docs/02_ir_schemas.md` (SUMMARY for `GCodeCommand`, `ExtrusionPath3D`) | `crates/slicer-host/tests/gcode_feedrate_emission_tdd.rs` (new) | none | M | Tests are at red. ≥ 8 tests covering the 6 ACs + 3 negative cases from `packet.spec.md`. |
| `TASK-153` | Step 3 — Register config keys | none beyond the design assumptions | `crates/slicer-host/src/config_schema.rs` | none | S | Two named tests (`speed_keys_registered_with_defaults`, `rejects_non_float_speed_config`) flip to green. |
| `TASK-153` | Step 4 — Resolver + wiring | `docs/08_coordinate_system.md` | `crates/slicer-host/src/gcode_emit.rs` (`resolve_feedrate` helper + three call-site edits at `:228`, `:282`, `:309`) | rounding rule from Step 1 | M | Remaining six tests + three negative tests flip to green. `gcode_emit_tdd` (covers the former `orca_comment_contract_tdd` `emits_orca_*` cases) regression remains green. |
| `TASK-153` | Step 5 — Backlog hygiene | `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | docs only | none | S | Adds TASK-153 row and DEV-009 progress entry. Status flip on `packet.spec.md` happens at the Packet Completion Gate. |

Aggregate context cost: M (no row is L).

## Why this packet is sufficient evidence for TASK-153

- TASK-153 is defined as "every print and travel move declares an F-token resolved from a registered per-role speed config key".
- Acceptance criteria in `packet.spec.md` collectively prove: (a) distinct F set ≥ 2 with at least one > 600 mm/min; (b) no stale F window > 200 lines; (c) role-to-key mapping resolves correctly for the four primary roles + travel; (d) `speed_factor` modulates correctly; (e) upstream module `f: Some(...)` is preserved; (f) all twenty-six keys are registered with correct defaults; (g) non-float values are rejected with the offending key name in the error.
- Negative cases prove the failure mode most likely to regress silently (a return to "only F25").

## Relationship to DEV-009

DEV-009 is the open deviation entry for "Benchy Phase H output is only partially correct on the live path". TASK-153 closes the speed-token subset of DEV-009. Subsequent packets (53 cooling, 54 skirt-brim + relative-E) close further DEV-009 subsets. The deviation entry remains open until all DEV-009 subsets are closed.
