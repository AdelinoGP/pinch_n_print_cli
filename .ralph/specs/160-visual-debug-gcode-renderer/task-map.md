# Task Map: 160-visual-debug-gcode-renderer

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-270` | `Step 1` | `docs/specs/visual-pipeline-debug.md`; `docs/19_visual_debug.md`; packet 157 contract | `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCodeReader.hpp`; delegate locations only | `S` | Establishes the bounded test seam and packet 157 integration inventory. |
| `TASK-270` | `Step 2` | `docs/specs/visual-pipeline-debug.md`; `docs/01_system_architecture.md`; `docs/11_operational_governance_and_acceptance_gate.md` | `crates/pnp-cli/src/main.rs`; `crates/pnp-cli/src/lib.rs`; renderer module under `crates/pnp-cli/src/` | `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp`; `OrcaSlicerDocumented/src/libvgcode/src/GCodeInputData.cpp`; delegate locations only | `M` | Implements the PnP subset, final PNGs, unclassified preservation, warnings, viewport, and deterministic bundle completion. |
| `TASK-270` | `Step 3` | `docs/11_operational_governance_and_acceptance_gate.md`; `docs/07_implementation_status.md` | The three Step 2 implementation/test files only | None beyond delegated Step 2 locations | `S` | Runs targeted tests and workspace compile/lint gates; no broad source inspection. |
