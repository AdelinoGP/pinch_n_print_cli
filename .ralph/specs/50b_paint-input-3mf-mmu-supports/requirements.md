# Requirements — 50b: Paint Input 3MF MMU + Support Co-Presence Tests

## Problem Statement

Packet 50 (TASK-180) implemented parsing for all four OrcaSlicer paint channels (`paint_fuzzy_skin`, `paint_supports`, `paint_seam`, `paint_color`) but explicitly deferred multi-channel co-presence tests using `benchy_4color.3mf` to "Packet 50b". The existing 8 paint tests each isolate a single channel using `benchy_painted.3mf`. No test verifies that both `PaintSemantic::Material` (MMU color, 4 tool indices) and `PaintSemantic::SupportEnforcer`/`SupportBlocker` co-exist correctly in a single mesh load from a real-world fixture. Additionally, the slicer has not been run end-to-end on a multi-color painted model; the user requires a GCode artifact to manually verify feature support in their slicer.

## Task IDs

- **TASK-180b** — deferred sub-task of TASK-180 (packet 50 / `50_paint-input-3mf-ingestion`)
- No separate `docs/07_implementation_status.md` entry; tracked as a packet-50 deferred item.

## In Scope

- 4 new test functions in `crates/slicer-host/tests/model_loader_tdd.rs`
- `resources/benchy_4color.3mf` as read-only test fixture
- Manual GCode output via slicer CLI for user inspection
- Bug fixes in `model_loader.rs` only if tests reveal a parser defect

## Out of Scope

- Authoring or modifying `resources/benchy_4color.3mf`
- WIT, PaintSegmentation, macro path, IR schema changes
- Subdivision TriangleSelector (hex > 2 nibbles) — remains an explicit non-goal
- Adding new CLI flags or output formats
- Downstream tool_index → extruder resolution

## Authoritative Docs

| Doc | Relevance |
|-----|-----------|
| `docs/02_ir_schemas.md` | FacetPaintData, PaintLayer, PaintSemantic, PaintValue exact field names |
| `docs/01_system_architecture.md` | MeshIR ownership, model loader boundary |
| `crates/slicer-ir/src/slice_ir.rs:188-199` | PaintValue enum variants (Flag, Scalar, ToolIndex, Custom) |

## OrcaSlicer Obligations

None. Parser parity was established in packet 50. This packet adds test coverage for an existing fixture.

## Acceptance Summary

| AC | Type | Measurable Outcome |
|----|------|--------------------|
| AC-1 | Positive | `paint_data.layers` contains both `Material` and `SupportEnforcer`/`SupportBlocker` semantics from `benchy_4color.3mf` |
| AC-2 | Positive | Material layer `facet_values` contains ≥4 distinct `ToolIndex(n)` values |
| AC-3 | Positive | SupportEnforcer layer has ≥1 `Some(PaintValue::Flag(true))` facet |
| AC-4 | Positive | `paint_data.layers.len()` ≥ 2 |
| AC-5 | Regression/Negative | All 8 packet-50 paint tests still pass after adding new tests |
| AC-6 | Manual | CLI exits 0 on `benchy_4color.3mf`; first 100 GCode lines printed in conversation |

## Cross-Packet Dependencies

- **Depends on:** Packet 50 (TASK-180) — implemented; its parser is the code under test here.
- **Unblocks:** TASK-136 (progress-event failure codes for paint annotations 501-504) — that task can be addressed in packet 51 or later once multi-channel parsing is verified.

## Verification Commands

```
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_has_mmu_and_support_layers --nocapture
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_material_spans_four_tool_indices --nocapture
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_support_enforcer_has_facets --nocapture
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_4color_layer_count_at_least_two --nocapture
cargo test -p slicer-host --test model_loader_tdd
cargo clippy -p slicer-host -- -D warnings
```
