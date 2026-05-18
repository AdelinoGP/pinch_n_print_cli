# Task Map: runtime-access-audit-and-declaration-enforcement

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-123` | Steps 1–4 | `docs/01_system_architecture.md` | `crates/slicer-host/src/scheduler/` | None | Feed access audits through all three execution tiers |
| `TASK-123a` | Step 2 | `docs/01_system_architecture.md` | `crates/slicer-host/src/scheduler/` | None | Prepass audit plumbing |
| `TASK-123b` | Step 3 | `docs/01_system_architecture.md` | `crates/slicer-host/src/scheduler/` | None | Per-layer audit plumbing |
| `TASK-123c` | Step 4 | `docs/01_system_architecture.md` | `crates/slicer-host/src/scheduler/` | None | Postpass audit plumbing + live-path regression |
| `TASK-124` | Steps 5–6 | `docs/03_wit_and_manifest.md` | `crates/slicer-host/src/wit/` | None | Enforce undeclared read/write at WIT boundary |
| `TASK-125` | Step 7 | `docs/01_system_architecture.md` (Claim Transition Matrix) | `crates/slicer-host/src/scheduler/` | None | Non-transitionable claim enforcement |
| `TASK-126` | Step 8 | `docs/04_host_scheduler.md` (WriteConflict) | `crates/slicer-host/src/scheduler/` | None | Fix orderable semantics |