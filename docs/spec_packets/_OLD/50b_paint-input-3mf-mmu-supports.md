---
status: implemented
packet: 50b_paint-input-3mf-mmu-supports
task_ids:
  - TASK-180b
---

# 50b_paint-input-3mf-mmu-supports

## Goal

Verify that the packet-50 parser correctly handles multi-channel co-presence (MMU `paint_color` + `paint_supports`) via integration tests, AND fix the end-to-end MMU tool-index propagation pipeline so that `T{n}` tool-change commands appear in GCode output for multi-color models.

## Problem Statement

Packet 50 (TASK-180) implemented parsing for all four OrcaSlicer paint channels but explicitly deferred multi-channel co-presence tests to "Packet 50b". Additionally, during implementation of those tests, a gap was discovered in the MMU tool-index propagation pipeline: `ObjectMesh.paint_data` was parsed correctly (verified by tests), but `PaintRegionIR.per_layer` was always empty because the `paint-segmentation` WASM guest ignored the `paint_layers` input parameter and only read `paint_region:*` config keys. Furthermore, `assemble_ordered_entities` in `layer_executor.rs` used `region.region_id` directly as the entity's `region_key.region_id` without considering paint-derived `WallFeatureFlags.tool_index`, so even if paint data were segmented, it would never produce tool-change commands in GCode.

The pipeline gap chain was:
1. `paint-segmentation` guest: ignored `object.paint_layers` → produced empty `PaintRegionIR`
2. Empty `PaintRegionIR` → `boundary_paint` never populated → `WallFeatureFlags.tool_index` always `None`
3. `assemble_ordered_entities`: used `region.region_id` directly → paint-derived tool index never reached `RegionKey.region_id`
4. `path-optimization-default`: grouped by `region_key.region_id as u32` → all entities same region → zero tool changes
5. `gcode_emit`: emitted no `T{n}` commands

## Data and Contract Notes

- `PaintValue::ToolIndex(u32)` — values are 0-based in the IR (OrcaSlicer encodes 1-based nibble; parser adjusts by -1).
- `WallFeatureFlags.tool_index: Option<u32>` — per-vertex paint annotation populated by perimeters from `boundary_paint`; `None` means unpainted.
- `RegionKey.region_id: u64` — `path-optimization-default` casts this to `u32` via `region_id as u32` for tool grouping. The `dominant_tool_index` helper returns `Option<u64>` to fit this field.
- `PaintSegmentationObjectView.paint_layers` — WIT type carrying per-object `PaintLayerView` data to the guest; was previously ignored.
- Benchy_4color fixture uses whole-facet paint (hex length = 2 nibbles per triangle), not subdivision.

## Risks and Tradeoffs

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `dominant_tool_index` returns `None` for unpainted walls → falls back to `region.region_id` | Expected — unpainted walls keep their default grouping | The `unwrap_or(region.region_id)` fallback preserves existing behavior |
| Paint-segmentation guest projects too many regions (per triangle) → OOM or 5+ min runtime | Medium — initial implementation had this bug | Aggregated per `(layer, semantic, value)` to keep entry count reasonable |
| `ActiveRegion.tool_index` stays hardcoded 0 | Low impact — `dominant_tool_index` bypasses this field | Future packet can propagate `ActiveRegion.tool_index` from paint data if needed |
| WIT type mismatch (`layer-index` as `s32` vs `u32`) | Fixed — aligned to `s32` in guest inline WIT | Verified by roundtrip tests |
