# Task Map: 280-bed-mesh-adaptive-placeholders

Single-ticket packet: wayfinder ticket 53 owns all steps; no `docs/07_implementation_status.md` row, no reopened or superseded packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| — (ticket 53) | `Step 1` | `docs/03_wit_and_manifest.md`, `docs/21_data_defaults_and_fixtures.md` | `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` | `PrintConfig.cpp` | `S` | Manifest declares the four inputs |
| — (ticket 53) | `Step 2` | `docs/01_system_architecture.md` | `modules/core-modules/machine-gcode-emit/src/lib.rs` | `GCode.cpp` | `M` | Bbox + probe math + site variables |
| — (ticket 53) | `Step 3` | `docs/08_coordinate_system.md` | `modules/core-modules/machine-gcode-emit/tests/bed_mesh_adaptive_tdd.rs` | `GCode.cpp` | `M` | Math, flavor, substitution proofs |
| — (ticket 53) | `Step 4` | map Notes authoring rules | `04`/`05` annotations, `docs/15_config_keys_reference.md` regen | none | `S` | Ledger and boundary close-out |
| — (ticket 53) | `Step 5` | `docs/adr/0050-custom-gcode-architecture.md` | `docs/adr/0050-*.md`, `docs/DEVIATION_LOG.md` | none | `S` | D-280-ADR-0050-AMENDED proves AC-8 |
