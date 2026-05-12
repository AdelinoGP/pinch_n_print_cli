# Design — 50b: Paint Input 3MF MMU + Support Co-Presence Tests

## Implementation Shape

This packet adds tests only; no new production code is planned unless a parser defect surfaces during the TDD-GREEN phase. The controlling code path is:

```
load_model("resources/benchy_4color.3mf")
  → parse_3mf_model_xml(...)
    → per-triangle: decode paint_color nibble → PaintValue::ToolIndex(n)
    → per-triangle: decode paint_supports nibble → PaintValue::Flag(true/false)
  → FacetPaintData { layers: vec![PaintLayer{Material,…}, PaintLayer{SupportEnforcer,…}] }
  → MeshIR { paint_data: Some(FacetPaintData { layers }) }
```

## Selected Approach

**Test-only packet.** Write 4 targeted test functions against the existing fixture. If all pass, the implementation is correct and no production change is needed. If any fail, diagnose against `model_loader.rs` lines 512–600 (paint channel assembly loop) before editing.

Rejected alternative: modifying `model_loader.rs` proactively to "improve" multi-channel handling — rejected because the approach is unguided without a failing test to triangulate the defect.

## Exact Code Change Surface

| File | Role | Action |
|------|------|--------|
| `crates/slicer-host/tests/model_loader_tdd.rs` | Test file | Add 4 new test functions |
| `crates/slicer-host/src/model_loader.rs` | Paint parser | Edit only if a test fails and the defect is localized here |
| `resources/benchy_4color.3mf` | Fixture | Read-only |

No other files may be edited in this packet.

## Read-Only Context the Implementer Needs

- `crates/slicer-host/tests/model_loader_tdd.rs:1-50` — imports and helper fn `load_model`
- `crates/slicer-host/tests/model_loader_tdd.rs` lines containing `load_3mf_extracts_mmu_color` and `load_3mf_extracts_support_facets` — to copy the assertion pattern for the new tests
- `docs/02_ir_schemas.md` lines covering `FacetPaintData`, `PaintLayer`, `PaintSemantic`, `PaintValue` (around lines 85–200)
- `crates/slicer-ir/src/slice_ir.rs:188-199` — PaintValue enum variants

## Out-of-Bounds Files

- `target/` — never read
- `OrcaSlicerDocumented/` — not needed (no new OrcaSlicer logic)
- `crates/sdk-prepass-guest/`, `crates/paint-segmentation-guest/` — not touched
- Any file not listed above

## Data and Contract Notes

- `PaintValue::ToolIndex(u32)` — values are 0-based in the IR (OrcaSlicer encodes 1-based nibble; parser in packet 50 adjusts by -1). The existing `load_3mf_extracts_mmu_color` test pins the expected value — read its assertion before writing AC-2's exact `ToolIndex(n)` assertions.
- `PaintValue::Flag(true)` — used for SupportEnforcer facets (enforcer=4 in raw nibble, after packet-50 mapping).
- `FacetPaintData.layers` is a `Vec<PaintLayer>`; layer order is not guaranteed — search by `semantic` field, do not index by position.
- `PaintLayer.facet_values` length equals triangle count. `None` means no paint on that facet.

## Risks and Tradeoffs

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `benchy_4color.3mf` uses subdivision TriangleSelector (hex > 2 nibbles) | Low — fixture authored for 50b | If `load_model` returns `Err`, inspect raw 3MF XML for `p:x` attribute lengths; packet stays draft with new blocker |
| `benchy_4color.3mf` is a multi-body 3MF (separate mesh per color) | Low — OrcaSlicer painted models are single-mesh | If `mesh_ir.paint_data` is None, inspect `model.3mf` XML for `<object>` count; may need model_loader fix |
| ToolIndex values may differ from 0–3 assumption | Medium — verify from existing test | Read `load_3mf_extracts_mmu_color` assertion for exact ToolIndex encoding before writing AC-2 assertion |
| SupportBlocker (not SupportEnforcer) in fixture | Low | Write AC-3 to accept either SupportEnforcer or SupportBlocker as the non-Material layer |

## Open Questions

None blocking activation. ToolIndex exact values and support channel variant (Enforcer vs Blocker) are resolved at TDD-RED time by inspecting the existing test and running the fixture.

## Locked Assumptions

- Packet-50 paint parser (`parse_3mf_model_xml`) is the sole code path for 3MF paint ingestion.
- `load_model` returns `Ok(MeshIR)` for valid whole-facet 3MF files.
- The 4 new tests must not modify any existing test function — regression safety.
- `benchy_4color.3mf` is a single-mesh 3MF with per-triangle paint attributes (not multi-body).
