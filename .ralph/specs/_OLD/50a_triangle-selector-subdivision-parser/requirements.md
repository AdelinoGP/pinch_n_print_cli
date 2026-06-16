# Requirements — Packet 50a

## Problem Statement

Packet 50 (`50_paint-input-3mf-ingestion`) implemented whole-facet paint parsing but
explicitly rejected TriangleSelector subdivision hex strings (any hex string with split bits ≠ 0
or length > 2). This was tracked as a deferred follow-up in that packet's design.

The updated fixture `resources/benchy_4color.3mf` contains both `paint_color` (171,381
occurrences, hex strings up to 7,543 chars) and `paint_supports` (82 occurrences), with 64
triangles carrying both channels. Because the parser errors on long hex strings before reaching
those triangles, the file cannot be loaded at all, blocking packet 50b's co-presence test.

This packet closes the gap in two phases:

1. **Phase 1**: Walk the serialized TriangleSelector tree to extract a per-facet dominant state,
   replacing the unconditional rejection of long strings. Unblocks packet 50b.
2. **Phase 2**: Extend the tree walker to reconstruct sub-triangle 3D geometry and populate
   `PaintLayer.strokes`, giving downstream modules sub-facet paint precision.

## Task ID Mapping

`TASK-180b-prereq` — prerequisite step for TASK-180b (not separately listed in
`docs/07_implementation_status.md`; the gap was identified when packet 50b was drafted).

Packet 50 (TASK-180) is already closed. This packet adds the subdivision capability that
TASK-180 deferred.

## In Scope

- `decode_paint_hex_state` in `crates/slicer-host/src/model_loader.rs`: replace the
  long-string rejection with a tree-walking dominant-state extraction.
- New private helpers in the same file: `parse_nibbles`, `walk_triangle_selector_tree`,
  `dominant_paint_state`, `decode_paint_hex_strokes`.
- Model-loader triangle loop: wire geometry into `decode_paint_hex_strokes`; accumulate
  strokes per semantic into `PaintLayer.strokes`.
- Four new tests and one updated test in `crates/slicer-host/tests/model_loader_tdd.rs`.

## Out of Scope

- Coordinate-unit conversion for stroke vertices (emit raw mm from 3MF; normalization is a
  follow-up).
- IR struct changes (all required fields in `PaintLayer` and `PaintStroke` already exist).
- WIT, scheduler, module manifest changes.
- OrcaSlicer source code — read only via delegated sub-agent calls, never directly.

## Authoritative Docs

| Doc | Relevant Sections |
|-----|------------------|
| `docs/02_ir_schemas.md` | `PaintLayer`, `PaintStroke`, `FacetPaintData` field paths |
| `docs/08_coordinate_system.md` | `mm_to_units()`, unit = 100 nm |
| `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` | Split geometry formulas (delegated) |
| `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:490–580` | TriangleSelector schema and serialization |

## OrcaSlicer Obligations

The TriangleSelector hex format is defined by OrcaSlicer/PrusaSlicer. The implementation
MUST match the deserialization behavior described in:

- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — `deserialize()` / tree walk
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:516–561`

These files are **read only via sub-agent delegation**. The implementer must never load them
into the main context directly.

## Hex Encoding Reference (extracted without reading source)

From delegated analysis, the per-nibble encoding is:
- `bits[1:0]` = split_sides (0=leaf, 1=2 children, 2=3 children, 3=4 children)
- `bits[3:2]` = state for leaf (0=NONE, 1=Enforcer, 2=Blocker, 3=extended→read next nibble)
- Extended state: leaf nibble has `bits[3:2]=11`; next nibble value `N` → state = `N + 3`
  (states 3–16 map to MMU filament indices 0–13)
- Traversal: DFS, parent nibble first, children in reverse order (PrusaSlicer 2.3.1 compat)

### State Mapping per Channel

| Channel | State 0 | State 1 | State 2 | State ≥3 |
|---------|---------|---------|---------|---------|
| `paint_color` | no paint | ToolIndex(0) | ToolIndex(1) | ToolIndex(state-1) |
| `paint_supports` | no paint | SupportEnforcer | SupportBlocker | (invalid, reject) |
| `paint_fuzzy_skin` | no paint | Flag(true) | (invalid, reject) | (invalid, reject) |
| `paint_seam` | no paint | SeamEnforcer | SeamBlocker | (invalid, reject) |

## Acceptance Summary

| AC | Phase | Measurable Outcome |
|----|-------|--------------------|
| AC-1 | 1 | `benchy_4color.3mf` loads without error |
| AC-2 | 1 | Material layer present with ≥1 Some(ToolIndex(_)) entry |
| AC-3 | 1 | SupportEnforcer layer present with ≥1 Some(Flag(true)) entry |
| AC-4 | 1 | Synthetic subdivided tree → dominant ToolIndex in facet_values |
| AC-5 | 1 (neg) | Non-hex chars → ModelLoadError::PaintMetadata "invalid hex digit" |
| AC-6 | 1 (neg) | Truncated tree (split=1, no children) → ModelLoadError "unexpected end" |
| AC-7 | 2 | strokes non-empty in Material layer from benchy_4color.3mf |
| AC-8 | 2 | All PaintStroke triangles are non-degenerate |
| AC-9 | 2 (neg) | Whole-facet paint → strokes is empty, only facet_values populated |

## Cross-Packet Dependencies

- **Unblocks**: `50b_paint-input-3mf-mmu-supports` (after Phase 1 ACs pass)
- **Depends on**: `50_paint-input-3mf-ingestion` (closed; must not regress its 8 tests)

## Verification Commands

```bash
# Phase 1 targeted suite
cargo test -p slicer-host --test model_loader_tdd

# Phase 2 stroke test
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_benchy_4color_strokes_populated

# Regression guard for packet 50 tests
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_extracts_fuzzy_skin_facets
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_extracts_support_facets
cargo test -p slicer-host --test model_loader_tdd -- load_3mf_extracts_mmu_color

# Clippy gate
cargo clippy --workspace -- -D warnings
```
