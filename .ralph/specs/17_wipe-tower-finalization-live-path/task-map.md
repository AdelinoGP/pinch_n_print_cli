# Task Map: wipe-tower-finalization-live-path

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-143` | Step 1 | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` | Freezes exact `run_finalization()` builder expectations before code changes. |
| `TASK-143` | Step 2 | `docs/05_module_sdk.md` | `modules/core-modules/wipe-tower/src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.hpp`, `WipeTower2.cpp` | Ports legacy purge logic onto the canonical finalization module surface. |
| `TASK-143` | Step 3 | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `crates/slicer-host/tests/finalization_live_tdd.rs` (extend existing file with new test function) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` | Extends existing `finalization_live_tdd.rs` from packet 16. Host test uses real `wipe-tower.wasm`; requires WASM rebuild after Step 2. |
| `TASK-143` (negative) | Step 1 | `docs/05_module_sdk.md` | `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` | Disabled or no-tool-change inputs must emit no finalization pushes. |