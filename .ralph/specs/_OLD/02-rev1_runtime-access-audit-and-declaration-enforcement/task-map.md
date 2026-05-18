# Task Map: 02-rev1_runtime-access-audit-and-declaration-enforcement

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

This packet reopens TASK-123a/123b/123c and TASK-124 (read audit plumbing is incomplete),
and TASK-126 (orderable semantics and positive test coverage missing).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-123a` | Steps 1–2 | `docs/01_system_architecture.md` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/prepass.rs` | None | Reopened: prepass read audit plumbing missing |
| `TASK-123b` | Steps 1–3 | `docs/01_system_architecture.md` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/layer_executor.rs` | None | Reopened: per-layer read audit plumbing missing |
| `TASK-123c` | Steps 1–4 | `docs/01_system_architecture.md` | `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/postpass.rs` | None | Reopened: postpass read audit plumbing missing |
| `TASK-124` | Steps 5–6 | `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/validation.rs` | None | Reopened: read enforcement cannot fire without read audits |
| `TASK-126` | Steps 7–8 | `docs/04_host_scheduler.md` | `crates/slicer-host/src/validation.rs`, `crates/slicer-host/tests/dag_validation_tdd.rs` | None | Reopened: positive orderable test case missing; semantics unclear |
