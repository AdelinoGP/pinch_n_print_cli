# Task Map: 01-rev1_manifest-ir-access-and-config-schema

Use this file when the packet needs an explicit bridge back to `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-122` | Steps 1–6 | `docs/03_wit_and_manifest.md` (Config Field Types) | `modules/core-modules/**/*.toml` | None | Complete config.schema population for all 17 modules |
| CLI wiring (implicit) | Step 7 | `docs/01_system_architecture.md` (JSON protocol, lines 465-480) | `crates/slicer-host/src/main.rs` | None | Wire ConfigSchema subcommand to build_config_schema_json |
