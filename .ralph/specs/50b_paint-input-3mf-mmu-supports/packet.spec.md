---
status: implemented
packet: 50b_paint-input-3mf-mmu-supports
task_ids:
  - TASK-180b
backlog_source: docs/07_implementation_status.md
predecessor: 50_paint-input-3mf-ingestion
blocker_resolved: benchy_4color.3mf loads successfully; subdivision rejection no longer triggered
---

# Packet 50b — Paint Input: 3MF MMU + Support Co-Presence Tests & MMU Pipeline Fix

## Goal

Verify that the packet-50 parser correctly handles multi-channel co-presence (MMU `paint_color` + `paint_supports`) via integration tests, AND fix the end-to-end MMU tool-index propagation pipeline so that `T{n}` tool-change commands appear in GCode output for multi-color models.

## Scope Boundaries

**In scope:**
- `crates/slicer-host/tests/model_loader_tdd.rs` — 4 new test functions
- `crates/slicer-host/src/layer_executor.rs` — `dominant_tool_index()` helper and `assemble_ordered_entities` fix to propagate paint-derived `tool_index` from `WallFeatureFlags` to `RegionKey.region_id`
- `modules/core-modules/paint-segmentation/wit-guest/src/lib.rs` — fix guest to process `object.paint_layers` instead of ignoring them
- `resources/benchy_4color.3mf` — read-only fixture

**Out of scope:**
- Subdivision TriangleSelector support (hex > 2 nibbles)
- IR schema changes (FacetPaintData, PaintLayer, PaintValue)
- CLI flag additions or output format changes
- STL paint-sidecar or 3MF write/export
- `ActiveRegion.tool_index` propagation (currently hardcoded to 0 in dispatch.rs; the `dominant_tool_index` approach in `assemble_ordered_entities` bypasses this correctly for perimeters)

## Prerequisites

- Packet 50 (TASK-180) status: implemented
- All 8 packet-50 paint tests in `model_loader_tdd.rs` passing

## Acceptance Criteria

**AC-1 — Multi-channel co-presence (positive)**
Given `resources/benchy_4color.3mf` loaded via `load_model`, when `mesh.objects[0].paint_data` is inspected, then `paint_data.is_some()` is true AND `paint_data.unwrap().layers` contains at least one layer with `semantic == PaintSemantic::Material` AND at least one layer with `semantic == PaintSemantic::SupportEnforcer` or `semantic == PaintSemantic::SupportBlocker`.
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers --nocapture`

**AC-2 — Four distinct tool indices (positive)**
Given `resources/benchy_4color.3mf` loaded, when the `PaintSemantic::Material` layer's `facet_values` is scanned, then at least 4 distinct `PaintValue::ToolIndex(n)` values appear.
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_material_spans_four_tool_indices --nocapture`

**AC-3 — Support enforcer facets non-empty (positive)**
Given `resources/benchy_4color.3mf` loaded, when the `PaintSemantic::SupportEnforcer` layer's `facet_values` is inspected, then at least one entry is `Some(PaintValue::Flag(true))`.
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_support_enforcer_has_facets --nocapture`

**AC-4 — Layer count >= 2 (positive)**
Given `resources/benchy_4color.3mf` loaded, when `paint_data.unwrap().layers.len()` is checked, then the result is >= 2.
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_layer_count_at_least_two --nocapture`

**AC-5 — No regression on packet-50 tests (regression)**
Given the existing packet-50 paint tests in `model_loader_tdd.rs`, when the full test file runs, then all previously passing tests still pass.
| `cargo test -p slicer-host --test model_loader_tdd 2>&1` — zero FAILED lines

**AC-6 — MMU tool-change commands in GCode (positive)**
Given `resources/benchy_4color.3mf` sliced end-to-end, when the output GCode is inspected, then at least one `T{n}` tool-change command (matching regex `^T\d`) appears in the GCode.
| `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module modules/core-modules/perimeters-default/target/wasm32-unknown-unknown/release/perimeters_default.wasm --module-dir modules/core-modules --output target/benchy_4color_mmu_test.gcode && Select-String -Path target/benchy_4color_mmu_test.gcode -Pattern "^T\d" | Select-Object -First 1`

**AC-7 — Paint-segmentation produces non-empty regions (positive)**
Given the slicer runs on a model with `paint_data`, when the `PaintRegionIR.per_layer` is inspected after `PrePass::PaintSegmentation`, then `per_layer.len()` > 0 for models with MMU paint data.
| `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd`

**AC-8 — No regression on paint-segmentation roundtrip tests (regression)**
Given the 11 existing paint-segmentation roundtrip tests, when they run after the guest module changes, all pass.
| `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd`

## Verification (Supplemental)

```
# Targeted suite — all model_loader tests including regression
cargo test -p slicer-host --test model_loader_tdd

# Paint-segmentation roundtrip tests
cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd

# Lint gate
cargo clippy -p slicer-host -- -D warnings

# Type-check only (fast smoke test)
cargo check -p slicer-host
```

## Authoritative Docs

- `docs/02_ir_schemas.md` — FacetPaintData, PaintLayer, PaintSemantic, PaintValue field names
- `docs/01_system_architecture.md` — model loader ownership and MeshIR provenance
- `crates/slicer-ir/src/slice_ir.rs:188-199` — PaintValue enum definition
- `crates/slicer-ir/src/slice_ir.rs:734-751` — ActiveRegion (tool_index field)
- `crates/slicer-ir/src/slice_ir.rs:1192-1205` — WallFeatureFlags (tool_index field)
- `crates/slicer-host/src/layer_executor.rs` — assemble_ordered_entities, dominant_tool_index

## OrcaSlicer Reference Obligations

Not required — benchy_4color.3mf is an existing binary fixture, not a re-implementation of OrcaSlicer logic. Parser behavior was established in packet 50 (TASK-180). The paint-segmentation guest logic projects per-triangle facet data onto per-layer polygons using Z-intersection geometry that is slicer-specific.