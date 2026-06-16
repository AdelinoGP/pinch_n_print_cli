# Design — Packet 50a

## Problem Shape

The function `decode_paint_hex_state` in `model_loader.rs` handles three cases:
1. 1-char hex → single-nibble leaf (whole-facet, state 0–2)
2. 2-char hex → extended-state leaf with `0xC` second nibble (state ≥3, filaments)
3. `>2` chars → **unconditional error** — this is what blocks `benchy_4color.3mf`

The fix: replace case 3 with a DFS tree walker that collects leaf states and reduces them to a
dominant state for `facet_values`. Phase 2 adds a geometry-aware variant that also emits
`PaintStroke` entries with the actual sub-triangle vertex positions.

## Selected Approach

**Phase 1: Dominant-state tree decoder**

Add three private functions:
```
parse_nibbles(hex: &str, byte_offset: usize) -> Result<Vec<u8>, ModelLoadError>
walk_triangle_selector_tree(nibbles: &[u8], pos: &mut usize, states: &mut Vec<u32>, byte_offset: usize) -> Result<(), ModelLoadError>
dominant_paint_state(states: &[u32]) -> u32
```

`decode_paint_hex_state` gains a third branch for `bytes.len() > 2`: call `parse_nibbles` →
`walk_triangle_selector_tree` → `dominant_paint_state`. Returns the dominant state as `u32`,
matching the existing return type. No callers change.

Tree walking algorithm:
```
read nibble → split_type = nibble & 0x3, state_bits = nibble >> 2
if split_type == 0:            // leaf
    if state_bits == 3:        // extended state: next nibble is (state - 3)
        read next nibble → state = next + 3
    else:
        state = state_bits
    push state
else:                          // non-leaf
    num_children = split_type + 1  // 1→2, 2→3, 3→4
    recurse num_children times
```

Dominant state: most-frequent non-zero state in the collected vec; if all zero → 0.

**Phase 2: Stroke geometry decoder**

Add:
```
decode_paint_hex_strokes(
    hex: &str,
    verts: [Point3; 3],
    byte_offset: usize,
) -> Result<Vec<([Point3; 3], u32)>, ModelLoadError>
```

This mirrors `walk_triangle_selector_tree` but threads triangle geometry through the recursion.
At each non-leaf, compute child sub-triangles according to the OrcaSlicer split geometry rules
(exact midpoint formula delegated to sub-agent read of `TriangleSelector.cpp`).

At each leaf: push `(current_triangle_verts, state)` if state ≠ 0.

In the model-loader triangle loop, after computing per-facet states, call
`decode_paint_hex_strokes` for any channel where the hex string has length > 2 and the
dominant state is non-zero. Accumulate returned pairs into a `per_channel_strokes: HashMap<u8_channel, Vec<(Triangle, u32)>>` and, after the triangle loop, convert to `PaintStroke` entries appended to the appropriate `PaintLayer.strokes`.

## Rejected Alternatives

- **Fixture-only approach**: Create a synthetic fixture with both channels and short hex values.
  Rejected: doesn't fix the parser; real-world files stay unloadable.
- **Skip subdivided triangles silently**: Return state 0 for long strings instead of error.
  Rejected: silently drops color data, produces incorrect IR.
- **Full sub-triangle IR representation without dominant state**: Populate only strokes, not
  facet_values, for subdivided triangles. Rejected: breaks downstream code that expects
  facet_values to be populated for every painted triangle.

## Exact Code Change Surface

### Primary file (read + edit)

`crates/slicer-host/src/model_loader.rs`
- Read lines 640–720 (current `decode_paint_hex_state` + `hex_nibble` helper)
- Edit: add `parse_nibbles`, `walk_triangle_selector_tree`, `dominant_paint_state` after `hex_nibble`
- Edit: `decode_paint_hex_state` — add `else` branch for `bytes.len() > 2`
- Phase 2 edit: add `decode_paint_hex_strokes`
- Phase 2 edit: model-loader triangle loop (lines ~350–520) to call strokes decoder and accumulate

### Secondary file (read + edit)

`crates/slicer-host/tests/model_loader_tdd.rs`
- Read lines 560–600 (`load_3mf_subdivision_paint_rejects`)
- Edit: rename to `load_3mf_truncated_paint_tree_rejects`, update expected error message
- Add new test: `load_3mf_invalid_paint_hex_rejects` (uses `paint_fuzzy_skin="GG"`)
- Add new test: `load_3mf_subdivision_dominant_state` (uses synthetic multi-nibble paint_color)
- Add new test: `load_3mf_benchy_4color_loads` (loads `resources/benchy_4color.3mf`)
- Phase 2 add: `load_3mf_benchy_4color_strokes_populated`
- Phase 2 add: `load_3mf_wholefacet_has_no_strokes`

### Read-only context

- `docs/02_ir_schemas.md` lines covering `PaintLayer`, `PaintStroke`, `FacetPaintData`
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md:516–561` (sub-agent)
- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` (sub-agent, split geometry only)

### Out-of-bounds files

- `target/` — never read
- `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` — never read directly; sub-agent only
- Any other `OrcaSlicerDocumented/` source files
- `crates/slicer-ir/` — no changes; existing types are sufficient
- Other crates (slicer-cli, scheduler, modules) — unaffected

## Data and Contract Notes

- `PaintLayer.facet_values` is parallel to `mesh.triangles`; must have exactly `facet_count` entries.
  Subdivided triangles use dominant state; there is no `None` for parsed subdivisions.
- `PaintLayer.strokes` is additive; each `PaintStroke` describes a sub-triangle region.
  Existing whole-facet entries remain in `facet_values`; strokes are additional precision.
- `PaintStroke.triangles: Vec<[Point3; 3]>` uses the codebase unit system (1 unit = 100 nm).
  The 3MF model gives vertices in mm. Apply `mm_to_units()` from `slicer-helpers` when
  populating stroke triangles. (See `docs/08_coordinate_system.md`.)
- `PaintStroke.semantic` and `.value` must match the channel being decoded (same as the
  parent `PaintLayer.semantic`/dominant value).

## Phase 2 Sub-Agent Obligation

Before implementing `decode_paint_hex_strokes`, the implementer MUST dispatch:

```
Question: In TriangleSelector.cpp, what are the exact vertex-index formulas for each of the
3 split types (split_sides=1, 2, 3)? Specifically: given parent triangle with vertices
v[0], v[1], v[2], which edges are split for each split_sides value, and in what order are
children emitted during DFS serialization?
Scope: OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp
Return: FACT ≤ 5 lines (one formula per split type)
```

Do NOT read `TriangleSelector.cpp` directly. Return format must be FACT.

## Risks and Tradeoffs

- **Dominant state is lossy**: triangles with mixed sub-triangle colors get a single representative
  state. This is acceptable for Phase 1 (unblocking co-presence test) and is corrected by Phase 2
  strokes (which provide full sub-triangle detail).
- **DFS traversal assumes PrusaSlicer 2.3.1 child ordering**: if OrcaSlicer changed the order,
  the parser would produce wrong geometry. Verified by AC-7/AC-8 against the actual fixture.
- **Infinite recursion hazard**: malformed trees could recurse indefinitely. Mitigation: the
  nibble array is bounded by the input string length; `pos` advances monotonically; recursion
  depth ≤ `nibbles.len()`. Add a depth guard (max 64) if tests show stack overflow.
- **Coordinate units**: Phase 2 vertices are in mm from 3MF; must apply `mm_to_units()`.
  Forgetting this would silently produce coordinates 10,000× too large.

## Locked Assumptions

- `bytes.len() == 0` → `Ok(0)` — no paint. Existing behavior; must not change.
- `bytes.len() == 1` → single-nibble whole-facet. Existing behavior; must not change.
- `bytes.len() == 2`, second nibble == `0xC` → extended state leaf. Existing behavior; must not change.
- `bytes.len() == 2`, second nibble ≠ `0xC` → previously: check split bits of second nibble.
  After this packet: this case is subsumed by the tree walker (a 2-nibble sequence where the
  first nibble has split_type=0 and the second is the extended state). Keep existing behavior for
  this sub-case; the tree walker handles it naturally since the second nibble follows the first.

## Open Questions

None blocking Phase 1. Phase 2 is blocked on the sub-agent dispatch for split geometry formulas
(the implementer dispatches this as Step 4 of the plan).
