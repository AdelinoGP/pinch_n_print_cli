# Task Map: 167-config-block-viewer-keys

This packet mints a new backlog task; the crosswalk below is the authoritative mapping to add to `docs/07_implementation_status.md` at closure (append as a `- [x] TASK-273 — …` row in the same style as TASK-270/271).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-273` (new — mint at closure: "CONFIG_BLOCK viewer-key correctness: purge speed/accel/jerk-valued padding from `ORCA_CONFIG_PADDING` (machine_max_* never emitted as padding), synthesize non-BBL `printer_model = Generic PNP Printer` when absent, and document the fork-facing required-key contract in docs/02. Spec: packet 167-config-block-viewer-keys.") | Steps 1-5 | `docs/02_ir_schemas.md` | `crates/slicer-gcode/src/serialize.rs`, `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs` | upstream behavior cited by function (`ConfigBase::load_from_gcode_file`, `GCodeProcessor::apply_config`, `s_IsBBLPrinter`); no port | S | AC-1 name-class grep + AC-2 gate count + AC-N1 no-shadowing jointly prove the task. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
