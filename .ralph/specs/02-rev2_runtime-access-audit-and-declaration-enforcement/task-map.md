# Task Map: 02-rev2_runtime-access-audit-and-declaration-enforcement

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This packet reopens TASK-123a/123b/123c (runtime_reads extraction not wired) and TASK-124
(live-path undeclared-read enforcement not validated).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-123a` | Steps 2–3 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/prepass.rs`, `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Sufficient when prepass audits prove `"MeshIR"` survives harvesting into `ModuleAccessAudit`. |
| `TASK-123b` | Steps 2–3 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/layer_executor.rs`, `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Sufficient when per-layer audits prove `"SliceIR.regions.polygons"` survives harvesting into `ModuleAccessAudit`. |
| `TASK-123c` | Steps 2, 4 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/postpass.rs`, `crates/slicer-host/tests/pipeline_tdd.rs` | None | Sufficient when postpass audits prove `"LayerCollectionIR"` is recorded for read-performing modules while write-only modules stay empty. |
| `TASK-124` | Steps 2, 5 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Sufficient when the undeclared-read test asserts `AccessKind::Read` and path `"SliceIR.regions.undeclared"` from a live audit path, not manual injection. |
