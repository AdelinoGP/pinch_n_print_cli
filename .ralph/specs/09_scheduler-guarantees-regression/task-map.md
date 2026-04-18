# Task Map: scheduler-guarantees-regression

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This file is required when the packet spans more than one task ID.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-131` | Step 1 | `docs/04_host_scheduler.md` (lines 492-510) | `crates/slicer-host/tests/resolve_active_regions_o1_contract_tdd.rs` | None | Regression guard for O(1) `resolve_active_regions` contract. Guards scheduler performance budget for runtime-budget evidence (TASK-156). |
| `TASK-132` | Step 2 | `docs/04_host_scheduler.md` (lines 512-530) | `crates/slicer-host/tests/region_map_overflow_tdd.rs`, `crates/slicer-host/tests/region_map_at_cap_tdd.rs` | None | Structured RegionMap overflow coverage with 1000-entry cap, top-contributor diagnostics, and remediation hints. Provides DEV-026 evidence for architecture acceptance gate. |
| `TASK-133` | Step 3 | `docs/01_system_architecture.md` (lines 46-49) | `crates/slicer-host/tests/layer_parallel_safe_false_serialization_tdd.rs` | None | Pool-behavior test proving `layer_parallel_safe = false` serializes concurrent WASM acquisition. Guards instance-pool concurrency contract in docs/04. |
| `TASK-134` | Step 4 | `docs/01_system_architecture.md` (lines 117-136), `docs/02_ir_schemas.md` (lines 274-278) | `crates/slicer-host/tests/catchup_layer_propagation_tdd.rs` | None | Catch-up layer field propagation test across all nine per-layer stages. Guards documented catch-up-layer propagation contract. |
| `TASK-131` (negative) | Step 1 | `docs/04_host_scheduler.md` (lines 492-510) | `crates/slicer-host/tests/resolve_active_regions_empty_tdd.rs` | None | Empty-region resolution returns empty slice, not an error. |
| `TASK-132` (negative) | Step 2 | `docs/04_host_scheduler.md` (lines 512-530) | `crates/slicer-host/tests/region_map_at_cap_tdd.rs` | None | At-cap boundary test: exactly 1000 entries succeeds without overflow. |