---
status: implemented
packet: 68_extruder-per-modifier-gcode
task_ids:
  - TASK-208
  - TASK-209
---

# 68_extruder-per-modifier-gcode

## Goal

Stamp all `config_delta.fields` from overlapping `ModifierVolume` entries into `RegionPlan.config.extensions` during region mapping, so every 3MF modifier part's config keys (`extruder`, `fuzzy_skin`, `matrix`, and extensible) act as per-region config overrides — achieving OrcaSlicer parity for Region Config Modifiers without new IR fields, WIT changes, or pipeline stages.

## Problem Statement

Packet 56's sidecar parser (`model_loader.rs`) parses `<metadata key="...">` from 3MF `<part>` elements and stores values in `ModifierVolume.config_delta.fields`. Currently, only the `"subtype"` key is consumed downstream (by `paint_segmentation.rs`, `negative_part_subtract.rs`, and `layer_executor.rs` modifier projections). All other keys — `"extruder"`, `"fuzzy_skin"`, `"matrix"`, and any future extensible keys — are parsed but silently ignored: they sit inert in `config_delta.fields` and never reach `RegionPlan.config`.

In OrcaSlicer, this is handled by `PrintApply::generate_print_object_regions()` which iterates every model volume, merges `config_delta` overrides into `PrintRegion.config`, and creates distinct regions when the resulting config differs. The modifier's config then drives every downstream stage — wall generation, infill density, speed, extruder selection — through the `PrintRegion.config` → `LayerRegion` → toolpath pipeline.

The infrastructure for config overrides already exists in our codebase:
- `overlay_resolved()` (`region_mapping.rs:100-193`) — merges one `ResolvedConfig` over another
- `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` (`RegionPlan`, slice_ir.rs:1126) — per-paint-semantic config overlays
- `RegionPlan.config.extensions: HashMap<String, ConfigValue>` — overflow bucket for unknown config keys
- `build_paint_semantic_configs()` (`prepass.rs:432-470`) — resolves `paint_config:<semantic>:<key>` from CLI config
- `overlapping_semantics_for_region()` (`region_mapping.rs:200-237`) — finds which paint semantics overlap a region's polygons

What's missing is the first step in the chain: reading `config_delta.fields` from modifier volumes, finding which volumes overlap a region, and stamping their keys into `RegionPlan.config.extensions`. Once stamped, every downstream stage automatically consumes the modifier's config — because they already read `RegionPlan.config`.

The previous version of this packet proposed a narrow workaround: route only `extruder` through `PaintValue::ToolIndex` in `paint_segmentation.rs`. That approach limits the mechanism to exactly one key and couples config routing to the paint pipeline. This revision implements the general mechanism: stamp ALL keys from `config_delta` into `config.extensions`, achieving OrcaSlicer parity for Region Config Modifiers.

## Architecture Constraints

- **No new IR fields**: All modifier config keys go into `RegionPlan.config.extensions: HashMap<String, ConfigValue>`. No changes to `RegionPlan`, `SlicedRegion`, `ResolvedConfig`, `MeshIR`, or any other IR struct. The `extensions` bucket already exists on `ResolvedConfig` and carries unknown config keys.
- **No new pipeline stages**: The config stamping is a function call inside existing `execute_region_mapping_with_cap()` (PrePass::RegionMapping). No stage reordering, no new PrePass/Layer/PostPass stages.
- **No WIT changes**: The change is host-side only. Guest WASM modules read `ConfigView` which already includes the `extensions` bucket via the `paint_overrides` and `config` paths.
- **Backward compatibility**: Modifier volumes with empty `config_delta.fields` (or only the `"subtype"` key) stamp no extensions. Existing behavior unchanged for all current fixtures.
- **Single merge point**: `overlay_resolved()` (region_mapping.rs:100-193) is the sole function that merges config. Config-delta stamping calls it once per overlapping modifier volume. The implementer must not introduce a second merge mechanism.
- **Determinism**: Config delta values are parsed from 3MF sidecar metadata — static per-input. No runtime non-determinism introduced.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- `ConfigDelta.fields` is `HashMap<String, ConfigValue>`. Key `"subtype"` is excluded from stamping. All other keys are stamped.
- Subtype values `support_enforcer` and `support_blocker` cause the entire modifier_volume to be skipped during stamping (OrcaSlicer parity; see Subtype filter row and `PrintApply.cpp:590-594`).
- `ConfigValue::Int(i64)` — for `"extruder"`, cast to `u32` via `try_into().unwrap_or(0)`. Valid extruder values are 0-15.
- `ConfigValue::String(s)` — for `"fuzzy_skin"`, `"matrix"`, and future keys. Copied verbatim into extensions.
- `ConfigValue::Bool(b)`, `ConfigValue::Float(f)`, `ConfigValue::List(v)` — supported but not used by current fixtures. Future-proof.
- `ResolvedConfig.extensions` is `HashMap<String, ConfigValue>` — same type as `ConfigDelta.fields`. Direct copy after filtering.
- `overlay_resolved(&mut base, &overlay)` at region_mapping.rs:100-193 merges `extensions` at line 193: for each key in overlay.extensions, inserts into base.extensions (overwriting if present).
- `ModifierVolume.priority: u32` — higher value = higher priority. The stamping loop sorts by priority ascending before stamping, so higher-priority modifiers stamp last and win.
- `ModifierVolume.mesh: IndexedTriangleSet` — provides vertex positions in world space. Reserved for future bbox / polygon-level overlap refinement; not consumed by the current global-per-object stamping path.

## Locked Assumptions and Invariants

1. `config_delta.fields` uses `ConfigKey = String` and `ConfigValue` enum — same types as `ResolvedConfig.extensions`. No type conversion needed between stamp source and destination.
2. `overlay_resolved()` merges `extensions` keys by simple insertion (line 193: `base.extensions.insert(k, v)`). This is the last-writer-wins behavior needed for priority ordering.
3. The per-region loop in `execute_region_mapping_with_cap()` does **not** have access to region polygon data — `ActiveRegion` carries only `object_id`, `region_id`, and `resolved_config`. Modifier-volume stamping therefore applies globally per object (matching `ModifierScope::AllFeatures`, the only variant in use) rather than via a bbox/polygon overlap test.
4. `mesh_ir: &MeshIR` is available in the region-mapping prepass scope (either from function parameters or from the blackboard). If not directly available, it must be threaded through.
5. `dominant_tool_index()` returns `None` when no paint-derived tool exists — the config-extensions fallback is only used in that case. Paint tools take precedence.
6. The `"subtype"` exclusion filter is correct for all current and future modifier volumes. If a future modifier type uses `"subtype"` as a config key (unlikely), the exclusion can be narrowed.
7. Modifier-volume stamping is global per object: every region whose `object_id` matches an `ObjectMesh.id` receives all of that object's modifier_volume config_delta keys (subject to the subtype filter and priority ordering). This is intentionally coarse — refinement to bbox / polygon overlap or per-layer Z-range intervals is future work, and only becomes meaningful once `ModifierScope` variants other than `AllFeatures` are introduced.
8. Packet 67's fixture tests exist and are compilable — this packet extends them.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| Global per-object stamping is too coarse — a small modifier affects every region of the same object even where it doesn't geometrically overlap | Acceptable for the only `ModifierScope` variant in use (`AllFeatures`) and the only 3MF modifier subtypes in flight (`modifier_part`, `negative_part`). For complex meshes with partial-volume modifiers, the refinement path is bbox overlap (then polygon overlap), which can be added without changing the stamping API — `stamp_modifier_config_deltas` would gain a polygon/extent parameter and per-modifier filtering. |
| `mesh_ir` not available in the region-mapping prepass scope | Step 1 discovery checks this. If unavailable, thread it through the function signature or store on the blackboard via a new prepass parameter. |
| Existing `config.extensions` keys from CLI config could collide with modifier-stamped keys | The `overlay_resolved` merge is last-writer-wins. Modifier config_delta stamps after per-object but before paint. If CLI sets `extensions["extruder"]`, the modifier overrides it. This is correct behavior. |
| `ConfigValue::Int(0)` for extruder is indistinguishable from an absent value | Special-case the default-skip: only skip empty strings and empty lists. `Int(0)` for `"extruder"` is meaningful. Use a key-aware check. |
| Non-extruder keys stamped into extensions but never consumed | Intentionally deferred. The stamping mechanism is the primary deliverable. Individual key consumers (e.g., `"fuzzy_skin"` → fuzzy-skin module) are follow-up packets. The AC-3 test proves the keys survive. |
| `dominant_tool_index()` already returns a tool from paint — config-extensions fallback might never be reached | In the existing `bridge_support_enforcers.3mf` fixture, modifier volumes have NO paint (`PaintValue::Flag(true)` after Packet 56c). After this packet, `PaintValue::Flag(true)` → `dominant_tool_index()` returns `None` → config-extensions fallback kicks in. The fallback path is exercised. |
