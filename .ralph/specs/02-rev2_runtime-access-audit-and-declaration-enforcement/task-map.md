# Task Map: 02-rev2_runtime-access-audit-and-declaration-enforcement

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This packet reopens TASK-123a/123b/123c (runtime_reads extraction not wired) and TASK-124
(live-path undeclared-read enforcement not validated).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-123a` | Steps 2–3 | `docs/01_system_architecture.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/prepass.rs` | None | Reopened: dispatch returns HostExecutionContext; extract runtime_reads |
| `TASK-123b` | Steps 2, 4 | `docs/01_system_architecture.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/layer_executor.rs` | None | Reopened: same pattern for per-layer execution |
| `TASK-123c` | Steps 2, 5 | `docs/01_system_architecture.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/postpass.rs` | None | Reopened: same pattern for postpass execution |
| `TASK-124` | Steps 6–7 | `docs/03_wit_and_manifest.md` | `crates/slicer-host/tests/pipeline_tdd.rs`, `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Reopened: live-path test for undeclared-read enforcement |
