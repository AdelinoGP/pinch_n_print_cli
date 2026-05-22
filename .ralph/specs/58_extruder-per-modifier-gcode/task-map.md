# Task Map: 58_extruder-per-modifier-gcode

## Purpose

This packet introduces TASK-208 and TASK-209, not present in `docs/07_implementation_status.md` at packet-author time. Step 6 of `implementation-plan.md` appends them after TASK-207. This file maps each task to implementation steps, the packets it depends on, and the tests it turns GREEN.

This packet is the second of a two-packet chain: Packet 57 added the fixture test harness and RED extruder tests; Packet 58 implements full Region Config Modifiers (config_delta stamping) and turns the RED tests GREEN.

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-208 | Config delta stamping: add `stamp_modifier_config_deltas()` in `region_mapping.rs` that stamps all `config_delta.fields` (except `"subtype"`) from overlapping `ModifierVolume` entries into `RegionPlan.config.extensions` via `overlay_resolved()`. Add config-extensions-driven `required_tool` fallback in `layer_executor.rs` so the `"extruder"` key in config.extensions selects the tool when no paint-derived tool exists. | Step 1 (discovery), Step 2 (implement stamping), Step 3 (implement tool fallback) | None. | `docs/02_ir_schemas.md` (ConfigDelta, ConfigValue, RegionPlan.config.extensions); `docs/04_host_scheduler.md` (region-mapping prepass ordering) | `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — `generate_print_object_regions()` at line 1049: config merging from modifier volumes into PrintRegion (LOCATIONS dispatch at Step 1) |
| TASK-209 | GCode and config integration tests: extend `threemf_fixture_e2e_tdd.rs` with 7 tests (turn 2 RED GREEN + 5 new). Assert `T0`/`T1` tool-change commands from config-stamped extruder, assert non-extruder keys survive in extensions, assert backward compatibility, and assert modifier priority ordering. | Step 4 (author tests), Step 5 (regression) | None. | This packet's `packet.spec.md` Acceptance Criteria section | None new |

## Fixture Coverage Map

| Fixture | Config delta keys tested | Tests |
|---------|-------------------------|-------|
| `resources/bridge_support_enforcers.3mf` | support_enforcer parts: `extruder=0`, `fuzzy_skin="all"`, `subtype="support_enforcer"`. Parent parts: extruder=1. | AC-1 (extruder stamped into extensions), AC-2 (T0/T1 in GCode), AC-3 (non-extruder key survives), AC-N1 (subtype-only stamps none), AC-N2 (conflicting priority) |
| `resources/cube_positive_n_negative.3mf` | negative_part: `extruder=0`, `subtype="negative_part"` | AC-4 (extruder on negative_part is benign) |

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 56 (`56_threemf-sidecar-parser`) | Depends on | Parses `extruder`, `fuzzy_skin`, `matrix` metadata into `config_delta.fields`. This packet reads those values and stamps them. |
| Packet 56c (`56c_threemf-negative-and-support-subtype-routing`) | Depends on | Populates `modifier_volumes` in `MeshIR`. Provides the support paint-segmentation piggyback. This packet reads `modifier_volumes` and stamps config from them. |
| Packet 64 (`64_paint-native-migration`) | Depends on | Provides host-native paint pipeline. The config stamping runs in the host-native `PrePass::RegionMapping` path. |
| Packet 51 (`51_regionmap-paint-semantic-aware`) | Depends on | TASK-181 provides `overlay_resolved()` (used for merging) and `paint_overrides` infrastructure (config-delta stamps before paint_overrides apply). |
| Packet 57 (`57_3mf-fixture-e2e-hardening`) | Depends on | Provides fixture test harness and the two RED tests this packet turns GREEN. |
| (none) | Unblocks | Terminal packet in this chain. No further packets planned. Future work: per-key consumers (fuzzy_skin → fuzzy-skin module), polygon-level overlap, per-layer Z-range intervals. |

## Notes for Implementer

- The production change is ~40 lines in `region_mapping.rs` + ~8 lines in `layer_executor.rs`. The most complex part is the test authoring (Step 4).
- Step 1 is critical — it confirms `mesh_ir` is available in the region-mapping scope and `overlay_resolved` handles extensions merging. If either assumption is wrong, the design adjusts before code is written.
- The `ConfigDelta.fields` type (`HashMap<String, ConfigValue>`) is identical to `ResolvedConfig.extensions` type — direct copy with subtype filtering.
- `ModifierVolume.mesh: IndexedTriangleSet` — compute the 2D bbox from `(x, y)` components of all vertices.
- GCode tests search for `T0` and `T1` as string patterns. Position-independent. Case-sensitive.
- Packet 57 must be `status: implemented` before this packet activates — Step 0 verifies.
- No `cargo test --workspace` needed — targeted regression commands sufficient.
- The `paint_segmentation.rs` approach from the previous packet version is replaced by the config-stamping approach. The existing paint pipeline (`PaintValue::ToolIndex`) is untouched; painted tools have higher priority than config-stamped tools.
