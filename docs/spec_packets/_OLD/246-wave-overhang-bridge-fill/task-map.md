# Task Map: 246-wave-overhang-bridge-fill

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-356` | `Step 1` | `docs/03_wit_and_manifest.md` | `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-sdk/src/views.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-wasm-host/src/marshal/in_.rs` | none | `M` | `internal-bridge-areas` view accessor |
| `TASK-356` | `Step 2` | `docs/03_wit_and_manifest.md` | `modules/core-modules/wave-overhangs/**`, `Cargo.toml`, `crates/slicer-integrated-modules/Cargo.toml`, `crates/pnp-cli/Cargo.toml`, `crates/slicer-scheduler/tests/contract/holder_matching_tdd.rs` | none | `M` | scaffold + manifest + registration + holder selection |
| `TASK-356` | `Step 3` | `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `modules/core-modules/wave-overhangs/src/{lib,generator}.rs` | `OrcaSlicerDocumented/src/libslic3r/WaveOverhangs/WaveOverhangs.cpp` | `M` | generator port + region pipeline |
| `TASK-356` | `Step 4` | `docs/specs/wave-overhangs-bridge-fill-plan.md` | `modules/core-modules/wave-overhangs/tests/wave_overhangs_tdd.rs` | none | `S` | negative cases |
| `TASK-356` | `Step 5` | `docs/19_visual_debug.md` | `crates/slicer-runtime/tests/e2e/wave_overhang_bridge_fill_e2e_tdd.rs` | none | `M` | end-to-end discriminator |
