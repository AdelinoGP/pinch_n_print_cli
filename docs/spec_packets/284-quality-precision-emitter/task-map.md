# Task Map: 284-quality-precision-emitter

Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under map ticket 58. Single-task packet (no backlog slice); this crosswalk exists for the S0 structural gate and records the map provenance.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-000` (queue packet, `task_ids: []`) | Steps 1–4 | `docs/15_config_keys_reference.md` (generated); `docs/config/host-keys.toml`; `docs/DEVIATION_LOG.md` (DEV-176) | `crates/slicer-ir/src/resolved_config.rs` + `crates/slicer-gcode/src/serialize.rs` + `crates/slicer-gcode/src/emit.rs` + `crates/slicer-gcode/tests/quality_precision_arc_resolution_tdd.rs` + lock-test arms | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (declarations + bounds) + `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` (`apply_print_config`, arc density) + `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (emission selection) + `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` (`0.2 *` tightening) | M | P51: 2 Tier B keys, owner `crates/slicer-gcode` stands; Tier B sizing confirmed (both zero-occurrence as behaviour, new tolerance + arc logic); scalar-global is parity (canonical scalar `coBool`/`coFloat` — no ticket-125 vector model); one intended default value change (`resolution` padding `0.012` shadowed by live `0.01`, count unchanged), geometry byte-identical at defaults; behaviour changes only at non-defaults (large simplification, arcs, tightening). |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
