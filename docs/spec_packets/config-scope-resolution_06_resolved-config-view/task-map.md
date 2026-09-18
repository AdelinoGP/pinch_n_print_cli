# Task Map: resolved-config-view

This explicit queue crosswalk is retained because TASK-567 consumes forward exports from packet 05 and owns a staged warn-to-drop gate.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-567` | `Steps 1–7` | `docs/specs/config-scope-resolution-plan.md`, ADR-0067, ADR-0068 | registry metadata, host-key registration, seeding (`slicer-ir`, `slicer-config`), manifest parse, binding (`slicer-scheduler`, `slicer-wasm-host`, `slicer-runtime`), CONFIG_BLOCK emission (`slicer-runtime`, `slicer-gcode`), `ConfigView` required accessors, 13 guests + 6 manifests, ingestion, focused tests/docs | `GCode::append_full_config` (`GCode.cpp`), `ConfigOptionString::serialize` / `escape_string_cstyle` (`Config.hpp`/`Config.cpp`) | `M` | Proves always-resolved registry-complete views, exact 89-site cleanup, full-registry escaped emission, host-key survival, no-drop gate, and warn-to-drop. |
