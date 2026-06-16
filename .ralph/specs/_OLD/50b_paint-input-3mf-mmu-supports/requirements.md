# Requirements — 50b: Paint Input 3MF MMU + Support Co-Presence Tests & Pipeline Fix

## Problem Statement

Packet 50 (TASK-180) implemented parsing for all four OrcaSlicer paint channels but explicitly deferred multi-channel co-presence tests to "Packet 50b". Additionally, during implementation of those tests, a gap was discovered in the MMU tool-index propagation pipeline: `ObjectMesh.paint_data` was parsed correctly (verified by tests), but `PaintRegionIR.per_layer` was always empty because the `paint-segmentation` WASM guest ignored the `paint_layers` input parameter and only read `paint_region:*` config keys. Furthermore, `assemble_ordered_entities` in `layer_executor.rs` used `region.region_id` directly as the entity's `region_key.region_id` without considering paint-derived `WallFeatureFlags.tool_index`, so even if paint data were segmented, it would never produce tool-change commands in GCode.

The pipeline gap chain was:
1. `paint-segmentation` guest: ignored `object.paint_layers` → produced empty `PaintRegionIR`
2. Empty `PaintRegionIR` → `boundary_paint` never populated → `WallFeatureFlags.tool_index` always `None`
3. `assemble_ordered_entities`: used `region.region_id` directly → paint-derived tool index never reached `RegionKey.region_id`
4. `path-optimization-default`: grouped by `region_key.region_id as u32` → all entities same region → zero tool changes
5. `gcode_emit`: emitted no `T{n}` commands

## Task IDs

- **TASK-180b** — deferred sub-task of TASK-180 (packet 50 / `50_paint-input-3mf-ingestion`)

## In Scope

- 4 new test functions in `crates/slicer-host/tests/model_loader_tdd.rs`
- `crates/slicer-host/src/layer_executor.rs` — `dominant_tool_index()` helper and `assemble_ordered_entities` modification
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — process `object.paint_layers` for 3D-to-2D projection
- `resources/benchy_4color.3mf` as read-only test fixture

## Out of Scope

- Subdivision TriangleSelector (hex > 2 nibbles)
- `ActiveRegion.tool_index` propagation in `dispatch.rs` (hardcoded to 0; `dominant_tool_index` bypasses this)
- Adding new CLI flags or output formats
- STL paint-sidecar or 3MF write/export

## Authoritative Docs

| Doc | Relevance |
|-----|-----------|
| `docs/02_ir_schemas.md` | FacetPaintData, PaintLayer, PaintSemantic, PaintValue exact field names |
| `docs/01_system_architecture.md` | MeshIR ownership, model loader boundary, per-layer execution |
| `crates/slicer-ir/src/slice_ir.rs:188-199` | PaintValue enum variants |
| `crates/slicer-ir/src/slice_ir.rs:734-751` | ActiveRegion struct with `tool_index: u32` |
| `crates/slicer-ir/src/slice_ir.rs:1192-1205` | WallFeatureFlags struct with `tool_index: Option<u32>` |
| `crates/slicer-host/src/layer_executor.rs` | `assemble_ordered_entities`, `dominant_tool_index` |

## OrcaSlicer Obligations

None. The paint-segmentation guest's Z-intersection projection is slicer-specific; OrcaSlicer uses a different projection strategy via its own TriangleSelector module.

## Acceptance Summary

| AC | Type | Measurable Outcome |
|----|------|--------------------|
| AC-1 | Positive | `paint_data.layers` contains both `Material` and `SupportEnforcer`/`SupportBlocker` semantics |
| AC-2 | Positive | Material layer `facet_values` contains ≥4 distinct `ToolIndex(n)` values |
| AC-3 | Positive | SupportEnforcer layer has ≥1 `Some(PaintValue::Flag(true))` facet |
| AC-4 | Positive | `paint_data.layers.len()` ≥ 2 |
| AC-5 | Regression | All existing packet-50 paint tests still pass |
| AC-6 | Positive | GCode output contains ≥1 `T{n}` tool-change command |
| AC-7 | Positive | `PaintRegionIR.per_layer.len()` > 0 for MMU-painted models |
| AC-8 | Regression | All 11 paint-segmentation roundtrip tests pass |

## Cross-Packet Dependencies

- **Depends on:** Packet 50 (TASK-180) — implemented; its parser is the code under test here.

## Verification Commands

```
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers --nocapture
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_material_spans_four_tool_indices --nocapture
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_support_enforcer_has_facets --nocapture
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_layer_count_at_least_two --nocapture
cargo test -p slicer-host --test model_loader_tdd
cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd
cargo clippy -p slicer-host -- -D warnings
```