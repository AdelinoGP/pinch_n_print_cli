# Task Map: 23-rev1_prepass-seam-planning-orca-parity

Use this file to track how packet steps map back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-159` | Steps 1-4, 6-7 | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `wit/world-prepass.wit`, `dispatch.rs`, `slicer-macros/src/lib.rs`, `seam-planner-default/src/lib.rs`, WASM rebuild | None | TASK-159 is "Add PrePass::SeamPlanning plus canonical SeamPlanIR blackboard contract". This packet fixes the root cause (geometry never reaches the module). |
| `TASK-135` | Step 7 (downstream) | `docs/07_implementation_status.md` | N/A — downstream verification only | None | TASK-135 is "Add Benchy regression assertions for supports, top/bottom fills, seams". This packet unblocks TASK-135 by making `SeamPlanIR` produce non-zero entries. |
