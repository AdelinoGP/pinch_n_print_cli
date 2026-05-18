# Task Map: live-top-bottom-surface-fill

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-120a` | Step 1 | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` | Freezes exact role expectations before implementation. |
| `TASK-120a` | Step 2 | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `modules/core-modules/rectilinear-infill/src/lib.rs` | `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.hpp` | Restores top/bottom/bridge role generation on the canonical infill module. |
| `TASK-120a` | Step 3 | `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/layer_executor.rs`, `crates/slicer-host/tests/live_top_bottom_fill_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` | Proves the live host path preserves those roles through final layer assembly. |
| `TASK-120a` (negative) | Step 1 | `docs/02_ir_schemas.md` | `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` | Sparse-only regions must not fabricate top/bottom/bridge roles. |