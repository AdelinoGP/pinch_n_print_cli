# Task Map: 175-m73-progress

Single-task packet; emitted because TASK-279 is minted by this packet (row added to `docs/07_implementation_status.md` at closure) and to record the hard FORWARD-DEP on packet 169 (TASK-275).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-279` | `Step 1` | `.ralph/specs/169-time-estimator-slice-stats/design.md` (SUMMARY) | `crates/slicer-gcode/src/estimator.rs` | — | S | Per-command elapsed vector; 169 tests stay green |
| `TASK-279` | `Step 2` | — | `crates/slicer-gcode/src/m73.rs`, `lib.rs`, `tests/m73.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp`, `GCode.cpp` (delegate) | M | M73 pairs + filament/time comment block |
| `TASK-279` | `Step 3` | `docs/15_config_keys_reference.md` (grep) | `crates/slicer-ir/src/resolved_config.rs` | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (delegate) | S | `disable_m73` bool key; guest-staleness check |
| `TASK-279` | `Step 4` | — | `crates/slicer-gcode/src/emit.rs` (:757-758 site), `crates/pnp-cli/tests/m73_progress_tdd.rs` | — | M | Emit-site wiring + e2e; proves AC-4/AC-N1 |
| `TASK-279` | `Step 5` | `docs/15_config_keys_reference.md`, `docs/ORCA_CONFIG_REFERENCE.md` | docs only | — | S | AC-5 doc greps |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M. Prerequisite crosswalk: TASK-275 (packet 169) must be closed in `docs/07_implementation_status.md` before this packet activates.
