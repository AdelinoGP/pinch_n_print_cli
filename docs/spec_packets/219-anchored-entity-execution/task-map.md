# Task Map: anchored-entity-execution

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-330` | Steps 1-8 | `docs/specs/support-families-anchored-entities-plan.md`, `docs/adr/0059-support-families-and-anchored-entities.md`, `docs/adr/0009-raft-as-layer-infill-role.md`, `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` | `crates/slicer-ir`, `crates/slicer-scheduler`, `crates/slicer-runtime`, `crates/slicer-sdk`, `crates/slicer-schema`, `crates/slicer-macros` | Delegate `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` and `Print.cpp` locations only | M | Establishes the generic substrate consumed by TASK-331.
