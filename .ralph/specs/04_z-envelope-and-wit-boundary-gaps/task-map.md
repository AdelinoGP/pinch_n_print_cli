# Task Map: z-envelope-and-wit-boundary-gaps

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-127` | Steps 1–2 | `docs/01_system_architecture.md` (Z Envelope Rules) | `crates/slicer-host/src/scheduler/` | None | Enforce Z envelope at output-commit |
| `TASK-129` | Steps 3–4 | `docs/04_host_scheduler.md` (PostPass Execution) | `crates/slicer-host/src/postpass/` | None | Close postpass GCode boundary gaps |
| `TASK-129a` | Steps 3–4 | `docs/03_wit_and_manifest.md` (world-postpass.wit) | `crates/slicer-host/src/postpass/` | None | Real GCode command lists crossing WIT |
| `TASK-129b` | Step 5 | `docs/04_host_scheduler.md` (LayerCollectionIR Lifecycle) | `crates/slicer-host/src/scheduler/` | None | Layer-world deep-copy coverage |
| `TASK-129c` | Step 6 | `docs/04_host_scheduler.md` (LayerCollectionIR Lifecycle) | `crates/slicer-host/src/scheduler/` | None | Finalization-world deep-copy coverage |