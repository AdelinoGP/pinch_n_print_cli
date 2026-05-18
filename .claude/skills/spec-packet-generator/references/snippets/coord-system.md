# Snippet: coord-system

**When to include**: packets that touch geometry, slicing, polygon ops, mesh ops, or any pipeline stage that converts between millimetres and internal integer units. Skip for packets that handle only G-code text emission, manifest/config parsing, scheduler wiring, or other non-geometric concerns.

**Where to include**: as a bullet in `design.md` §`Architecture Constraints`. Add `<!-- snippet: coord-system -->` on the line above the bullet.

**Verbatim bullet**:

```
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
```

**Do not paraphrase.** The `1 unit = 100 nm` factor is the single most common source of OrcaSlicer-port bugs; the wording must match the canonical reference in `CLAUDE.md` and `docs/08_coordinate_system.md` so a reader can grep across them.
