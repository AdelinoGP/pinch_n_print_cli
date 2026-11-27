# Task Map: 282-resonance-avoidance-emitter

Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under map ticket 55. Single-task packet (no backlog slice); this crosswalk exists for the S0 structural gate and records the map provenance.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-000` (queue packet, `task_ids: []`) | Steps 1–3 | `docs/15_config_keys_reference.md` (generated); `docs/config/host-keys.toml`; `docs/DEVIATION_LOG.md` (DEV-174) | `crates/slicer-ir/src/resolved_config.rs` + `crates/slicer-gcode/src/emit.rs` + `crates/slicer-gcode/tests/resonance_avoidance_emission_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (declarations + defaults) + `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (`GCode::_extrude` resonance block) | M | P48: 3 Tier B keys, owner `crates/slicer-gcode` stands; Tier B sizing confirmed (all zero-occurrence, new emitter logic); scalar-global is parity (canonical scalar — no ticket-125 vector model); one intended behaviour change only at non-defaults (avoidance off → identity). |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
