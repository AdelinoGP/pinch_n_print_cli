---
status: draft
packet: 50b_paint-input-3mf-mmu-supports
task_ids:
  - TASK-180b
backlog_source: docs/07_implementation_status.md
predecessor: 50_paint-input-3mf-ingestion
blocker: benchy_4color.3mf contains TriangleSelector subdivision data (hex length 391) rejected by parser; fixture incompatible with whole-facet paint path
---

# Packet 50b — Paint Input: 3MF MMU + Support Co-Presence Tests

## Blocker

`benchy_4color.3mf` triggers the subdivision rejection guard in `model_loader.rs`: `load_model` returns `Err(PaintMetadata { reason: "TriangleSelector hex string length 391 indicates subdivision, which is not supported" })`. The fixture uses per-triangle subdivision encoding (hex strings > 2 nibbles), which is explicitly out of scope for this packet. The packet cannot proceed until either:

1. A whole-facet (non-subdivision) 3MF fixture with both MMU color and support paint channels is provided, OR
2. The packet scope is expanded to include subdivision TriangleSelector support (would require a new design).

## Goal

Add integration tests using `resources/benchy_4color.3mf` to verify that the packet-50 parser correctly handles multi-channel co-presence: MMU `paint_color` (all 4 tool indices) and `paint_supports` (SupportEnforcer/SupportBlocker) present simultaneously in the same mesh load. Includes a manual GCode output step for end-to-end slicer verification by the user.

## Scope Boundaries

**In scope:**
- `crates/slicer-host/tests/model_loader_tdd.rs` — new test functions only (no modification to existing tests)
- `resources/benchy_4color.3mf` — read-only fixture, no authoring
- `target/benchy_4color_manual_test.gcode` — generated artifact for manual inspection

**Out of scope:**
- `crates/slicer-host/src/model_loader.rs` — unless tests reveal a parser defect requiring a fix
- WIT definitions, PaintSegmentation pipeline, macro path
- IR shape changes (FacetPaintData, PaintLayer, PaintValue)
- Subdivision TriangleSelector support (hex > 2 nibbles) — remains out of scope
- CLI flag additions or output format changes
- STL paint-sidecar or 3MF write/export

## Prerequisites

- Packet 50 (TASK-180) status: implemented
- All 8 packet-50 paint tests in `model_loader_tdd.rs` passing:
  `load_3mf_extracts_fuzzy_skin_facets`, `load_3mf_malformed_fuzzy_skin_rejects`,
  `load_3mf_without_paint_returns_none`, `load_3mf_extracts_support_facets`,
  `load_3mf_extracts_seam_facets`, `load_3mf_extracts_mmu_color`,
  `load_3mf_malformed_support_value_rejects`, `load_3mf_subdivision_paint_rejects`

## Acceptance Criteria

**AC-1 — Multi-channel co-presence (positive)**
Given `resources/benchy_4color.3mf` loaded via `load_model`, when `mesh_ir.paint_data` is inspected, then `paint_data.is_some()` is true AND `paint_data.unwrap().layers` contains at least one layer with `semantic == PaintSemantic::Material` AND at least one layer with `semantic == PaintSemantic::SupportEnforcer` or `semantic == PaintSemantic::SupportBlocker`.
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers --nocapture`

**AC-2 — Four distinct tool indices (positive)**
Given `resources/benchy_4color.3mf` loaded, when the `PaintSemantic::Material` layer's `facet_values` is scanned, then at least 4 distinct `PaintValue::ToolIndex(n)` values appear (covering all 4 painted regions).
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_material_spans_four_tool_indices --nocapture`

**AC-3 — Support enforcer facets non-empty (positive)**
Given `resources/benchy_4color.3mf` loaded, when the `PaintSemantic::SupportEnforcer` layer's `facet_values` is inspected, then at least one entry is `Some(PaintValue::Flag(true))`.
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_support_enforcer_has_facets --nocapture`

**AC-4 — Layer count >= 2 (positive)**
Given `resources/benchy_4color.3mf` loaded, when `paint_data.unwrap().layers.len()` is checked, then the result is >= 2 (at least `PaintSemantic::Material` and one support channel).
| `cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_layer_count_at_least_two --nocapture`

**AC-5 — No regression on packet-50 tests (negative / regression)**
Given the 8 existing packet-50 paint tests in `model_loader_tdd.rs`, when the full test file runs after adding the 4 new 4color tests, then all 8 existing tests still pass (zero FAILED lines in output).
| `cargo test -p slicer-host --test model_loader_tdd 2>&1` — all lines must be `ok`, none `FAILED`

**AC-6 — GCode produced and output for manual inspection (manual)**
Given `resources/benchy_4color.3mf`, when the slicer CLI is run with `--slice --input resources/benchy_4color.3mf --output target/benchy_4color_manual_test.gcode`, then the command exits 0, the output file is non-empty, and the implementing agent prints the first 100 lines of the GCode file into the conversation for the user to copy and load into their slicer.
| `cargo run --bin slicer-cli --release --slice --input resources/benchy_4color.3mf --output target/benchy_4color_manual_test.gcode` then `Get-Content target/benchy_4color_manual_test.gcode -TotalCount 100`

## Verification (Supplemental)

```
# Targeted suite — all model_loader tests including regression
cargo test -p slicer-host --test model_loader_tdd

# Lint gate
cargo clippy -p slicer-host -- -D warnings

# Type-check only (fast smoke test)
cargo check -p slicer-host
```

## Authoritative Docs

- `docs/02_ir_schemas.md` — FacetPaintData, PaintLayer, PaintSemantic, PaintValue field names
- `docs/01_system_architecture.md` — model loader ownership and MeshIR provenance
- `crates/slicer-ir/src/slice_ir.rs:188-199` — PaintValue enum definition

## OrcaSlicer Reference Obligations

Not required — benchy_4color.3mf is an existing binary fixture, not a re-implementation of OrcaSlicer logic. Parser behavior was established in packet 50 (TASK-180).
