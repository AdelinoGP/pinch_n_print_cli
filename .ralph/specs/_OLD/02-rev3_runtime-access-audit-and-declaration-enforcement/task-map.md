# Task Map: 02-rev3_runtime-access-audit-and-declaration-enforcement

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This packet reopens TASK-123a, TASK-123b, TASK-123c, and TASK-124 (all previously claimed by 02-rev1 and 02-rev2) because the postpass wiring was incomplete and test assertions were missing.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-123c` | Steps 1–4 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/postpass.rs`, `crates/slicer-host/tests/pipeline_tdd.rs` | None | Postpass dispatch returns `runtime_reads` alongside result; `execute_postpass` uses them for audit population; test asserts `LayerCollectionIR` in reads. |
| `TASK-123a` | Step 5 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/pipeline_tdd.rs` | None | New `prepass_audits_live_path` test asserts `"MeshIR"` in `prepass_audits` from live `run_pipeline` run. |
| `TASK-123b` | Step 6 | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/pipeline_tdd.rs` | None | New `layer_audits_live_path` test asserts `"SliceIR.regions.polygons"` in `layer_audits` from live `run_pipeline` run. |
| `TASK-124` | Step 7 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Manual `earlier_live_audit` construction replaced with live-path execution; test uses live `runtime_reads` data for undeclared-read assertion. |