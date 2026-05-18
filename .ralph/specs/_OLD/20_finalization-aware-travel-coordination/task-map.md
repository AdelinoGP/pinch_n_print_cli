# Task Map: finalization-aware-travel-coordination

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-152f` | Step 1 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/finalization_aware_travel_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`, `AvoidCrossingPerimeters.cpp` | Freezes the brim-aware travel transition and no-op boundary before implementation. |
| `TASK-152` | Step 2 | `docs/04_host_scheduler.md`, `docs/05_module_sdk.md` | `crates/slicer-host/src/gcode_emit.rs`, `crates/slicer-host/src/postpass.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`, `GCode.cpp` | Implements the host-side reconciliation pass that sees finalization geometry and travel hints together. |
| `TASK-152f` | Step 2 | `docs/04_host_scheduler.md` | `crates/slicer-host/tests/finalization_aware_travel_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` | Adds wipe-aware travel detour coverage. |
| `TASK-152f` (negative) | Step 3 | `docs/01_system_architecture.md` | `crates/slicer-host/tests/finalization_aware_travel_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` | Reconciliation must preserve model extrusion order while changing travel transitions only. |