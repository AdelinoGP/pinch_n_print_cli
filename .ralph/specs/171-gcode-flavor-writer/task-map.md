# Task Map: 171-gcode-flavor-writer

This packet mints a new backlog task; this crosswalk owns the `docs/07_implementation_status.md` registration. TASK-276 is the next free ID after TASK-275 (highest ID in `docs/07` is TASK-272; wave-1 draft packets 166-170 have claimed TASK-273 through TASK-275). The row is appended to `docs/07_implementation_status.md` at closure via a worker dispatch, never a full-file read.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-276` (new: G-code flavor dialect layer, 5 flavors, config-honored + CONFIG_BLOCK echo) | Steps 1-5 | `docs/02_ir_schemas.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `crates/slicer-gcode/src/flavor.rs` (new), `crates/slicer-gcode/src/serialize.rs`, `crates/slicer-runtime/src/run.rs` | `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp`, `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` | M | Dialect matrix tests + CONFIG_BLOCK echo prove the flavor is parsed, applied, and reported; closes handoff item 5 (wave-2 plan) |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

Suggested docs/07 row text (for the closure dispatch): `TASK-276 — [ ] G-code flavor dialect layer: GcodeFlavor enum (marlin/marlin2/klipper/reprapfirmware/repetier) ported from OrcaSlicer GCodeWriter.cpp, honored from gcode_flavor config, echoed in CONFIG_BLOCK (packet 171).`
