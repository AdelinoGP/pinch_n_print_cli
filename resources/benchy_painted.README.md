# benchy_painted.3mf — Authoring Procedure

## Context

This fixture provides a painted 3MF for Packet 50 (`paint-input-3mf-ingestion`). It tests the
`fuzzy_skin_facets` paint channel end-to-end through the host loader → `FacetPaintData` →
`PaintSegmentation` pipeline.

**Important constraint**: Only WHOLE-FACET paint is supported. The painting must be done at
facet granularity. The `paint_fuzzy_skin` per-triangle attribute inherently represents whole-facet
paint; partial-triangle coverage would require a different encoding not supported in this packet.

## Tool

OrcaSlicer (BambuSlicer fork) GUI — any recent version that supports the fuzzy-skin paint tool.

## Geometry Reference

- Source: `resources/benchy.stl`
- Triangle count: ~225,786
- Z range: [0, 48mm]
- Smokestack location: TOP of the model, Z ≈ 40–48mm (not [50,72mm] as originally documented —
  the smokestack is at the top of Benchy, not elevated above it)

## Procedure

1. Open OrcaSlicer (or PrusaSlicer).
2. Load `resources/benchy.stl` via File → Import.
3. Switch to **Face view** (enable face-selection mode) in the plater.
4. Navigate to the **smokestack region** at the top of the model (Z ≈ 40–48mm).
5. **Shift+click** or **box-select** to select the individual facets covering the smokestack
   triangles. Do NOT use freehand brush strokes.
6. With facets selected, use the **Paint → Fuzzy Skin** tool (right-panel painting menu) to
   apply fuzzy skin to the selected whole facets only.
7. Verify the selection covers **entire triangles** — no partial facet coverage.
8. File → Export as 3MF. Ensure **mesh + paint data** are included in the export.
9. Save as `resources/benchy_painted.3mf`.

## Regeneration

To regenerate this fixture:
1. Load `resources/benchy.stl`
2. Repeat steps 4–8
3. The attribute name in the emitted XML is `paint_fuzzy_skin` (unprefixed, no namespace).
   Painted triangles carry `paint_fuzzy_skin="4"`; unpainted triangles omit the attribute.

## What to Paint

Paint the **smokestack region** (the raised cylindrical part at the top of the Benchy model).
A cluster of 50–200 triangles in that region is sufficient for the E2E test to observe a
byte-difference in normalized G-code output.