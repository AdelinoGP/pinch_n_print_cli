# Task Map: 245-lock-aware-infill-consumers

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-355` | `Step 1` | `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` | `modules/core-modules/infill-linker/src/orchestrate.rs` | none | `M` | locked passthrough + swept-footprint carve |
| `TASK-355` | `Step 2` | `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` | `modules/core-modules/path-optimization-default/src/lib.rs` | none | `S` | locked-block nearest-neighbor candidate |
| `TASK-355` | `Step 3` | `docs/specs/wave-overhangs-bridge-fill-plan.md` | `crates/slicer-gcode/src/emit.rs` | none | `S` | locked bypass of D-P + min-segment |
| `TASK-355` | `Step 4` | `docs/02_ir_schemas.md`, `CONTEXT.md` | tests + `docs/adr/0063-*` | none | `S` | all-`None` parity + ADR-0063 + doc amendments |
