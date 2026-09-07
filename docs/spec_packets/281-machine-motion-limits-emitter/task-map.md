# Task Map: 281-machine-motion-limits-emitter

Single-ticket packet: wayfinder ticket 54 owns all steps; no `docs/07_implementation_status.md` row, no reopened or superseded packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| — (ticket 54) | `Step 1` | map Notes authoring rules 1–2, ticket-140 precedent | `crates/slicer-ir/src/resolved_config.rs`, scheduler bounds | `PrintConfig.cpp` | `M` | Eight fields + map arms + bounds prove the config surface |
| — (ticket 54) | `Step 2` | packet 267 spec (envelope seam) | `crates/slicer-gcode/src/emit.rs`, `gcode_emit_tdd.rs` | `GCode.cpp`, `GCodeWriter.cpp` | `M` | M201 + M204 R + M205 J prove the envelope keys |
| — (ticket 54) | `Step 3` | `docs/01_system_architecture.md` | `crates/slicer-gcode/src/estimator.rs`, `tests/estimator.rs` | `GCodeProcessor.cpp` | `S` | Min-rate clamps prove the estimator keys |
| — (ticket 54) | `Step 4` | map Notes ledger-facts rule | `DEVIATION_LOG.md`, 04/05 rows, config reference | none | `S` | DEV-173 + regen + ledger prove close-out |
