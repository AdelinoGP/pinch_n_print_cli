---
status: implemented
packet: 50a
task_ids: [TASK-180b-prereq]
---

# 50a_triangle-selector-subdivision-parser

## Goal

Implement a TriangleSelector hex-tree decoder in the 3MF paint parser so that
real-world painted 3MF files (including `benchy_4color.3mf`) load without error.

**Phase 1** extracts a per-facet dominant state from the tree and unblocks packet 50b.  
**Phase 2** reconstructs sub-triangle 3D geometry and populates `PaintLayer.strokes`.

Resolves the blocker documented in `.ralph/specs/50b_paint-input-3mf-mmu-supports/packet.spec.md`:
`benchy_4color.3mf` could not be loaded because long paint hex strings were rejected.

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
