---
when: Include in `design.md` for geometry or millimetre/internal-unit conversion work.
keywords: coordinates, geometry, millimetres, internal units
---

# Coordinate System Snippet

Skip pure G-code text, manifest/config parsing, scheduler wiring, and other non-geometric work. Copy exactly as one `design.md` Architecture Constraints bullet:

```markdown
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
```

Do not paraphrase the conversion factor or canonical helpers.
