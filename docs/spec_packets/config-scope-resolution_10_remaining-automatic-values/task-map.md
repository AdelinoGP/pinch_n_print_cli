# Task Map: remaining-automatic-values

This single-task crosswalk is emitted because TASK-571 spans a prerequisite/census gate, a `ResolvedConfig` field blast radius, the context-rich emitter seam, and a mandated visual-debug acceptance gate.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-571` | `Step 1` | approved plan RC-8/Expansion/row 10; `docs/22_test_quality.md` | packet-04/05 export reconciliation; derived negative-sentinel census; emitter handoff | `GCode.cpp::GCode::_extrude` | `S` | Blocks implementation on exact dependencies and confirms that packet 04 owns every current negative sentinel. |
| `TASK-571` | `Step 2` | `docs/02_ir_schemas.md`; `docs/21_data_defaults_and_fixtures.md` | `ResolvedConfig::filament_max_volumetric_speed`; registry census test; struct-literal fallout | None | `M` | Carries global/per-tool volumetric inputs without adding a second config representation. |
| `TASK-571` | `Step 3` | approved plan Phase C; `docs/22_test_quality.md` | `DefaultGCodeEmitter::emit_gcode`; `volumetric_auto_speed_tdd` | `GCode.cpp::GCode::_extrude` | `M` | Resolves only numeric zero from live width/height/flow/tool context and preserves explicit speeds. |
| `TASK-571` | `Step 4A` | `docs/19_visual_debug.md` | visual request plus its `parse_cli_config_source` companion JSON | None | `S` | Supplies required geometry-visible evidence from explicitly configured automatic values. |
| `TASK-571` | `Step 4B` | `docs/02_ir_schemas.md`; `docs/11_operational_governance_and_acceptance_gate.md` | docs 02/15; freshness and closure gates | None | `S` | Records Phase-C placement and unchanged public contract versions. |

## Forward Exports

This packet adds `ResolvedConfig::filament_max_volumetric_speed: f32` in `slicer-ir`. It intentionally exports no new public resolver or emitter API for later queue rows; the move-context helper remains private to `slicer-gcode`.

## Ownership Lock

TASK-571 owns only the Phase-C volumetric `0 = auto` emitter path and any live geometry-dependent negative sentinel discovered by the derived census. Packet 04 retains all four overhang-speed percentages over `outer_wall_speed`, `support_interface_bottom_layers = -1`, and `support_bottom_interface_spacing = -1`; packet 05 retains scope resolution and precedence.
