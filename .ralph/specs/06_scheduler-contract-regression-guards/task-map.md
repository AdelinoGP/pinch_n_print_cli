# Task Map: scheduler-contract-regression-guards

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-131` | Step 1 | `docs/04_host_scheduler.md` (resolve_active_regions O(1) contract) | `crates/slicer-host/src/scheduler/` | None | O(1) regression guard |
| `TASK-132` | Step 2 | `docs/04_host_scheduler.md` (RegionMap Memory Budget) | `crates/slicer-host/src/scheduler/` | None | RegionMap overflow structured diagnostics |
| `TASK-133` | Step 3 | `docs/04_host_scheduler.md` (WASM Instance Pool) | `crates/slicer-host/src/scheduler/` | None | layer_parallel_safe=false serialization proof |
| `TASK-134` | Step 4 | `docs/01_system_architecture.md` (Catch-Up Layer Semantics), `docs/04_host_scheduler.md` | `crates/slicer-host/src/scheduler/` | None | Catch-up layer propagation through all stages |