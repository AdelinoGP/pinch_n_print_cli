# Task Map: wit-consolidation-and-custom-type-widening

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-144` | Steps 1–4 | `docs/03_wit_and_manifest.md` | `wit/`, `crates/slicer-host/`, `modules/core-modules/*/wit-guest/` | None | Consolidate all WIT to canonical source |
| `TASK-145` | Steps 5–6 | `docs/03_wit_and_manifest.md` | `wit/**/*.wit`, generated bindings | None | Normalize identifiers, add drift-detection |
| `TASK-146` | Step 7 | `docs/04_host_scheduler.md` | `crates/slicer-host/src/scheduler/` | None | wit_world allowlist validation |
| `TASK-149` | Steps 8, 10, 12 | `docs/02_ir_schemas.md` | `wit/deps/types.wit`, `wit/deps/ir-types.wit` | None | Widen custom types in WIT |
| `TASK-150` | Steps 9, 11, 13–14 | `docs/02_ir_schemas.md` | `crates/slicer-host/src/wit/converter.rs` | None | Update converters, add round-trip tests |