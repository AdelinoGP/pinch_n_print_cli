# Task Map: skirt-brim-finalization-live-path

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-142` | Step 1 | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` | Freezes exact `run_finalization()` builder expectations before code changes. |
| `TASK-142` | Step 2 | `docs/05_module_sdk.md` | `modules/core-modules/skirt-brim/src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/Brim.hpp` | Ports legacy geometry helpers onto the canonical finalization module surface. |
| `TASK-142` | Step 3 | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/tests/finalization_live_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` | Proves the host merges finalization output and no longer depends on the legacy helper. |
| `TASK-142` (negative) | Step 1 | `docs/05_module_sdk.md` | `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` | Disabled or empty inputs must emit no finalization pushes. **Note:** this test passes immediately with the default no-op `run_finalization()` — the no-op is already correct behavior for disabled or empty inputs. |