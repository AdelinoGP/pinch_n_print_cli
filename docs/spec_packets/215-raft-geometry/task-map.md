# Task Map: raft-geometry

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-324` | Step 1 | `docs/02_ir_schemas.md`; ADR-0009 | Scheduled IR definitions, literals, conversions, assertions, preserved sentinels | None; no numerical parity claim | M | Inventory before editing; owns struct-literal and test-assertion fallout. |
| `TASK-324` | Step 2 | `docs/02_ir_schemas.md`; remediation plan | `GlobalLayer.index`, signed runtime/host/visual-debug schedule and capture | None | M | Makes negative selectors representable without unsigned casts. |
| `TASK-324` | Step 3 | `docs/03_wit_and_manifest.md` | `LayerModule::run_infill`, macro glue, host/runtime boundary, SDK guests and tests | None | M | Owns the full WIT `s32` to SDK `i32` migration surface. |
| `TASK-324` | Step 4 | ADR-0009; `docs/08_coordinate_system.md` | `raft-default` synthesizer, rectilinear `claim:raft-fill`, focused tests | None; no numerical parity claim | M | Uses existing scan-line algorithm. |
| `TASK-324` | Step 5 | `docs/19_visual_debug.md`; architecture/schema/manifest docs | Negative-prefix dispatch, visual request fixtures, typed capture, docs | None | M | PNG support is conditional; typed `raft_paths` is the fallback gate. |
