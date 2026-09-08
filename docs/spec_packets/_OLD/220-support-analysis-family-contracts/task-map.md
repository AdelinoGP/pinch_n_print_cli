# Task Map: support-analysis-family-contracts

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-331` | Steps 1-4 | `docs/specs/support-families-anchored-entities-plan.md`, `docs/adr/0059-support-families-and-anchored-entities.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-ir`, `crates/slicer-scheduler`, `crates/slicer-wasm-host`, `crates/slicer-schema`, `crates/slicer-sdk`, `crates/slicer-macros`, support manifests | Delegate `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` and `TreeSupport.cpp` locations only | M | Consumes TASK-330 anchored event contracts and unblocks TASK-332/TASK-333.
