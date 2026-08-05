# Task Map: 173-thumbnails-multiformat

Single new task, mapped for the docs/07 crosswalk (TASK-277 is minted by this packet at closure; it has no pre-existing `docs/07_implementation_status.md` row).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-277` | `Step 1` | `docs/specs/fork-gaps-wave2-plan.md` | `crates/slicer-gcode/src/thumbnail.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` | S | `thumbnails` key parser + spec model |
| `TASK-277` | `Step 2` | `docs/02_ir_schemas.md` | `crates/slicer-gcode/src/thumbnail.rs`, `serialize.rs`, `crates/slicer-runtime/src/pipeline.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp` | M | Orca-parseable inner framing @78 cols |
| `TASK-277` | `Step 3` | `docs/ORCASLICER_ATTRIBUTION.md` | `crates/slicer-gcode/src/thumbnail_colpic.rs` (new), `thumbnail.rs`, `Cargo.toml` | `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` | M | image dep + renderer + ColPic port |
| `TASK-277` | `Step 4` | `docs/ORCASLICER_ATTRIBUTION.md` | `crates/slicer-gcode/src/thumbnail_btt.rs` (new) | `OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.cpp` | S | BTT_TFT RGB565 hex port |
| `TASK-277` | `Step 5` | `docs/02_ir_schemas.md` | `crates/slicer-runtime/src/pipeline.rs`, `tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` | none | M | pipeline wiring + roundtrip-test rewrite proves the task end-to-end |
| `TASK-277` | `Step 6` | `docs/02_ir_schemas.md`, `docs/DEVIATION_LOG.md` | docs only | none | S | fork-facing contract note + D-173-THUMBNAIL-SINGLE-PNG |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
