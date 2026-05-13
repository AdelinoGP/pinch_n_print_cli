# Task Map — 50b: Paint Input 3MF MMU + Support Co-Presence Tests & Pipeline Fix

## Task ID Mapping

| Packet Step | Task ID | docs/07 Entry | Status |
|-------------|---------|---------------|--------|
| Steps 1–3 | TASK-180b | Deferred sub-task of TASK-180 | Done |
| Step 4 | TASK-180b | MMU pipeline: dominant_tool_index | Done |
| Step 5 | TASK-180b | MMU pipeline: paint-segmentation guest fix | Done |
| Steps 6–7 | TASK-180b | E2E verification + lint | Done |

## Predecessor Relationship

```
TASK-180 → packet 50 (implemented)
               └─ deferred: benchy_4color.3mf multi-channel tests
                       ↓
               TASK-180b → packet 50b (implemented)
                       ├─ AC-1 through AC-5: test verification
                       ├─ AC-6: MMU GCode T commands (pipeline fix)
                       ├─ AC-7/AC-8: paint-segmentation guest fix
                       └─ layer_executor dominant_tool_index propagation
```

## Authoritative Doc Coverage Per Step

| Step | Primary Doc | Secondary Doc |
|------|-------------|---------------|
| 1–3 (Tests) | `crates/slicer-ir/src/slice_ir.rs:188-199` | `docs/02_ir_schemas.md` |
| 4 (dominant_tool_index) | `crates/slicer-host/src/layer_executor.rs:589-685` | `crates/slicer-ir/src/slice_ir.rs:1192-1205` (WallFeatureFlags) |
| 5 (paint-segmentation guest) | `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` | `crates/slicer-host/src/wit_host.rs:2625-2680` |
| 6 (GCode E2E) | — | — |
| 7 (Lint) | — | — |

## Pipeline Gap Root Cause

The paint-segmentation WASM guest (`wit-guest/src/lib.rs`) ignored the `objects` parameter's `paint_layers` field and only read `paint_region:*` config keys. The host never populates those config keys from 3MF data, producing an empty `PaintRegionIR.per_layer`. This caused `boundary_paint` to be empty, `WallFeatureFlags.tool_index` to be `None` for all vertices, and zero `T{n}` tool-change commands in GCode output.

The fix: restructured the guest to iterate `objects`, project 3D triangle facets onto per-layer 2D polygons aggregating per `(layer_index, semantic, paint_value)` tuple. Combined with `dominant_tool_index` in `layer_executor.rs`, this closes the end-to-end paint → region → tool_change → GCode pipeline.

## Related Future Work

- **TASK-136** — progress-event failure codes 501-504 for paint annotation failures.
- **ActiveRegion.tool_index propagation** — `dispatch.rs:1704` hardcodes `tool_index: 0` in `ActiveRegion`. The `dominant_tool_index` approach in `assemble_ordered_entities` bypasses this for perimeters, but `ActiveRegion.tool_index` should eventually be populated from paint data for other subsystems (e.g., infill tool assignment).
- **Subdivision TriangleSelector** — full hex-encoded subdivision (> 2 nibbles) remains deferred.
- **GCode wipe tower / prime tower** — multi-extruder printing requires wipe/prime tower logic, which is not in scope for this packet.