# Task Map: 283-printer-timing-emitter

Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under map ticket 56. Single-task packet (no backlog slice); this crosswalk exists for the S0 structural gate and records the map provenance.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-000` (queue packet, `task_ids: []`) | Steps 1–4 | `docs/15_config_keys_reference.md` (generated); `docs/config/host-keys.toml`; `docs/DEVIATION_LOG.md` (DEV-175) | `crates/slicer-ir/src/resolved_config.rs` + `crates/slicer-gcode/src/estimator.rs` + `crates/slicer-gcode/src/m73.rs` + `crates/slicer-gcode/src/emit.rs` + `crates/slicer-gcode/tests/printer_timing_stats_tdd.rs` + lock-test arms | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (declarations + defaults) + `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` (`process_filament_change` both overloads) + `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`update_print_estimated_stats`) | M | P49: 4 Tier B keys, owner `crates/slicer-gcode` stands; Tier B sizing confirmed (all zero-occurrence, new estimator/footer logic); scalar-global is parity (canonical scalar `coFloat` × 4 — no ticket-125 vector model); `None`-omitted shape keeps CONFIG_BLOCK byte-stable at defaults; behaviour changes only at non-defaults (toolchange seconds, cost line). |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
