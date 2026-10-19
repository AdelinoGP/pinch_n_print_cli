# Task Map: typed-modifier-kind

This single-task map is emitted because the queue assignment explicitly requires five packet files and because `TASK-569` spans IR, loader, resolver, geometry, and compatibility surfaces.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-569` | Steps 1–10 | Plan §Modifiers/owner decision 2; ADR-0070; ADR-0030; `docs/02_ir_schemas.md`; `docs/04_host_scheduler.md` | `slicer-ir` modifier IR; model loader; ten core/runtime match sites; common runtime config seam; focused tests/docs | `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` by function name through delegation | `M` | Proves typed four-kind routing, registry admission, old-scope deletion, an activation-derived one-minor MeshIR compatibility change, and visual geometry without adding layer range. |
