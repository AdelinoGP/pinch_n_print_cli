# Task Map: 278-gcode-output-emitter-modes

This queue packet has `task_ids: []`; implementation is recorded against [51 - Author packet P44 - Others / G-code output - emitter](../../specs/orca-feature-gap/issues/51-author-packet-p44-others-g-code-output-emitter.md), and the `docs/07_implementation_status.md` crosswalk is N/A unless a task is added before activation. Re-derive that mapping at activation and closure.

| docs/07 task ID | Packet steps | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| - (queue packet, `task_ids: []`) | Steps 1-6 | `docs/config/host-keys.toml`; generated `docs/15_config_keys_reference.md`; feature-gap 04/05 annotations | `crates/slicer-gcode/src/emit.rs`; `crates/slicer-ir/src/resolved_config.rs`; `modules/core-modules/{machine-gcode-emit,path-optimization-default}` manifests/source/tests; runtime/scheduler integration tests | `GCode.cpp` (`apply_print_config`, `process_layer`, `_extrude`, `needs_retraction`, `_print_first_layer_extruder_temperatures`); `GCodeWriter.cpp` (`GCodeWriter::apply_print_config`); `Print.cpp::output_filename`; `PrintBase.cpp::output_filename`; `PrintConfig.cpp` declarations | M | Five P44 keys retained. `filename_format` returns to host-export. `support_object_skip_flush` sequences after Bambu/M624 flush support. |
