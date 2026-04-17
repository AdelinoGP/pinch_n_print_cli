# Task Map: prepass-segmentation-sdk-wit-alignment

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-128` | Steps 1–4 | `docs/01_system_architecture.md` | `crates/slicer-sdk/src/`, `crates/slicer-host/src/prepass/` | Check OrcaSlicerDocumented/ | Segmentation input-shape gap resolution |
| `TASK-128a` | Steps 1–2 | `docs/02_ir_schemas.md` (MeshIR) | `crates/slicer-sdk/src/prepass.rs` | None | MeshObjectView real geometry |
| `TASK-128b` | Steps 3–4 | `docs/02_ir_schemas.md` (PaintRegionIR) | `crates/slicer-sdk/src/prepass.rs` | None | PaintSegmentation real inputs |
| `TASK-130` | Steps 5–6 | `docs/05_module_sdk.md` | `crates/slicer-macros/src/` | None | Complete #[slicer_module] prepass bridge |
| `TASK-130a` | Step 7 | `docs/03_wit_and_manifest.md` (world-prepass.wit) | `crates/slicer-macros/src/`, `crates/slicer-host/src/prepass/` | None | PaintSegmentationOutput drainage |
| `TASK-130b` | Step 8 | `docs/05_module_sdk.md` | `crates/slicer-host/tests/` | None | End-to-end round-trip tests |