# Task Map: 279-spiral-vase-modes

Single-ticket packet: wayfinder ticket 52 owns all five steps; no `docs/07_implementation_status.md` row, no reopened or superseded packet.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| — (ticket 52) | `Step 1` | `docs/03_wit_and_manifest.md`, `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-ir/src/resolved_config.rs`, `docs/config/host-keys.toml`, perimeter manifests | `PrintConfig.cpp` | `S` | Fields and strict spelling prove the config surface |
| — (ticket 52) | `Step 2` | `docs/01_system_architecture.md` | `execution_plan.rs`, `execution_plan_live.rs` | `Print.cpp` | `S` | Unified dispatch and validation prove the mode decision |
| — (ticket 52) | `Step 3` | `docs/08_coordinate_system.md` | `crates/slicer-gcode/src/spiral_vase.rs`, `emit.rs` | `SpiralVase.cpp`, `GCode.cpp` | `M` | Stage proves the four SpiralVase keys |
| — (ticket 52) | `Step 4` | `docs/03_wit_and_manifest.md` | `machine-gcode-emit/src/lib.rs` | recorded gate shape | `S` | Gate clause proves the fog obligation |
| — (ticket 52) | `Step 5` | map Notes authoring rules | `04`/`05` annotations, runtime integration | none | `S` | Ledger and boundary prove close-out |
