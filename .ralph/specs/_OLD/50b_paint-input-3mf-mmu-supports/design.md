# Design — 50b: Paint Input 3MF MMU + Support Co-Presence Tests & Pipeline Fix

## Implementation Shape

This packet has two parts:
1. **Test functions** (original scope): 4 integration tests verifying multi-channel paint co-presence in `model_loader_tdd.rs`.
2. **MMU pipeline fix** (discovered during implementation): two production changes that close the gap between parsed paint data and GCode tool-change commands.

The controlling code paths for the pipeline fix:

```
load_model("resources/benchy_4color.3mf")
  → PerPixel::ToolIndex(n) stored in ObjectMesh.paint_data
  → PaintSegmentationObjectView.paint_layers passed to WASM guest
  → guest projects 3D facets to 2D per-layer polygons and pushes PaintRegion entries
  → boundary_paint populated on SlicedRegion via host paint annotation
  → arachne-perimeters reads boundary_paint → sets WallFeatureFlags.tool_index = Some(n)
  → dominant_tool_index(&wl.feature_flags) extracts majority tool per WallLoop
  → assemble_ordered_entities uses paint-derived region_id for RegionKey
  → path-optimization-default groups entities by tool_index_of(entity.region_key.region_id)
  → gcode_emit emits T{n} commands at tool-index transitions
```

## Selected Approach

**Part 1: Test-only.** Write 4 targeted test functions against the existing fixture. Tests access `mesh.objects[0].paint_data` (not `mesh.paint_data` — MeshIR has no `paint_data` field; it lives on ObjectMesh).

**Part 2: Pipeline fix — two changes:**

1. **`layer_executor.rs` — `dominant_tool_index()` + `assemble_ordered_entities` modification.** Added a helper that extracts the most common `WallFeatureFlags.tool_index` from a wall loop's per-vertex flags. When a wall loop has paint-derived tool data, the entity's `RegionKey.region_id` is set to the dominant tool index instead of the default `region.region_id`. This connects paint data to the path-optimization grouping mechanism without changing `ActiveRegion.tool_index` (which remains 0 for other subsystems).

2. **`paint-segmentation/wit-guest/src/lib.rs` — process `object.paint_layers`.** The WASM guest was ignoring the `paint_layers` field of `PaintSegmentationObjectView` and only reading `paint_region:*` config keys (which the host never populates from 3MF data). Changed the guest to iterate `objects`, check `paint_layers` for non-empty data, and project 3D triangle facets onto per-layer 2D polygons with the appropriate `paint-value`. Aggregated per `(layer_index, semantic, value)` tuple into single regions (not per-triangle, which would produce millions of entries).

Rejected alternative: Propagating `ActiveRegion.tool_index` from paint data via the dispatch layer — rejected because `ActiveRegion.tool_index` is currently unused downstream and the `dominant_tool_index` approach correctly handles per-wall-loop tool assignment (walls can have mixed tool indices per vertex).

## Exact Code Change Surface

| File | Role | Action |
|------|------|--------|
| `crates/slicer-host/tests/model_loader_tdd.rs` | Test file | Add 4 new test functions; fix `mesh.paint_data` → `mesh.objects[0].paint_data` and `*n` → `n` dereference |
| `crates/slicer-host/src/layer_executor.rs` | Entity assembly | Add `dominant_tool_index()` helper; modify perimeter loop in `assemble_ordered_entities` to prefer paint-derived `region_id` |
| `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` | WASM guest | Replace config-key-only logic with `object.paint_layers` projection; aggregate per `(layer, semantic, value)` |
| `modules/core-modules/paint-segmentation/paint-segmentation.wasm` | Built WASM | Rebuild after guest changes |

## Read-Only Context the Implementer Needs

- `crates/slicer-host/tests/model_loader_tdd.rs:1-50` — imports and helper fn `load_model`
- `crates/slicer-host/tests/model_loader_tdd.rs` — `load_3mf_extracts_mmu_color` and `load_3mf_extracts_support_facets` for assertion patterns
- `crates/slicer-ir/src/slice_ir.rs:188-199` — PaintValue enum variants
- `crates/slicer-ir/src/slice_ir.rs:734-751` — ActiveRegion struct (`tool_index: u32`)
- `crates/slicer-ir/src/slice_ir.rs:1192-1205` — WallFeatureFlags (`tool_index: Option<u32>`)
- `crates/slicer-ir/src/slice_ir.rs:1300-1313` — WallLoop struct (`feature_flags: Vec<WallFeatureFlags>`)
- `crates/slicer-host/src/layer_executor.rs:589-685` — `assemble_ordered_entities` and `dominant_tool_index`
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — guest implementation
- `modules/core-modules/arachne-perimeters/src/lib.rs:472-478` — where `WallFeatureFlags.tool_index` is set from paint data

## Out-of-Bounds Files

- `target/` — never read
- `OrcaSlicerDocumented/` — not needed
- `crates/slicer-host/src/dispatch.rs` — `ActiveRegion.tool_index: 0` is NOT changed; the `dominant_tool_index` bypass handles this correctly
- Any file not listed in Code Change Surface or Read-Only Context

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

## Open Questions

None. All open questions from the original draft (subdivision rejection, ToolIndex values, support variant) were resolved during implementation.

## Locked Assumptions

- `benchy_4color.3mf` is a single-mesh 3MF with per-triangle whole-facet paint attributes (not subdivision, not multi-body).
- The `dominant_tool_index` approach is correct for perimeter walls; infill and support entities retain their default `region_id`.
- `path-optimization-default` correctly groups by `entity.region_key.region_id as u32` and emits `ToolChangeRecord` at transitions.
- `gcode_emit` correctly converts `ToolChangeRecord` to `T{n}` commands in the output.