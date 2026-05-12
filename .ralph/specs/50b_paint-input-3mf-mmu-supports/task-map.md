# Task Map — 50b: Paint Input 3MF MMU + Support Co-Presence Tests

## Task ID Mapping

| Packet Step | Task ID | docs/07 Entry | Status |
|-------------|---------|---------------|--------|
| Steps 1–5 | TASK-180b | Deferred sub-task of TASK-180 (`50_paint-input-3mf-ingestion`) | New / not yet in docs/07 |

TASK-180b is not a formal `docs/07_implementation_status.md` entry. It was described as future work in packet 50's out-of-scope section ("Multi-channel binary fixture `benchy_4color.3mf` deferred to Packet 50b"). Add a `TASK-180b` row to docs/07 at packet closure if the backlog requires formal tracking.

## Predecessor Relationship

```
TASK-180 → packet 50 (implemented)
               └─ deferred: benchy_4color.3mf multi-channel tests
                       ↓
               TASK-180b → packet 50b (this packet, active)
```

## Authoritative Doc Coverage Per Step

| Step | Primary Doc | Secondary Doc |
|------|-------------|---------------|
| 1 (TDD-RED) | `crates/slicer-ir/src/slice_ir.rs:188-199` | `docs/02_ir_schemas.md` |
| 2 (TDD-GREEN) | `crates/slicer-host/src/model_loader.rs:490-600` | `docs/02_ir_schemas.md` |
| 3 (Regression) | — | — |
| 4 (GCode) | `docs/00_project_overview.md` (CLI flags) | — |
| 5 (Lint) | — | — |

## OrcaSlicer References Per Step

None required. Parser parity for the paint channels was established in packet 50. No new OrcaSlicer-derived logic is introduced.

## Related Future Work

- **TASK-136** — progress-event failure codes 501-504 for paint annotation failures. This remains out of scope for 50b but is a natural successor once multi-channel parsing is verified.
- **Subdivision TriangleSelector** — full hex-encoded subdivision (> 2 nibbles) is deferred and not addressed in this packet.
