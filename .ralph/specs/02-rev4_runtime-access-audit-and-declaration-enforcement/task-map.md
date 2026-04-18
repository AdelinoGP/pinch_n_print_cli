# Task Map: 02-rev4_runtime-access-audit-and-declaration-enforcement

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This packet reopens TASK-123c and TASK-124 (both previously claimed by 02-rev3) because the audit found two CRIT gaps: (1) the postpass read-performing test was never actually exercised by a real test runner, and (2) the dag validation audit helper was a simulation rather than live dispatch.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-123c` | Steps 1–3 | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/pipeline_tdd.rs` | None | Add read-performing postpass test `access_audits_live_path_read_performing` with `PostpassModuleReadingPostpassRunner`; verify write-only `access_audits_live_path` still passes. |
| `TASK-124` | Step 4 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Replace `collect_dispatch_audit` simulation with live-dispatch helper using `WasmRuntimeDispatcher`. |
