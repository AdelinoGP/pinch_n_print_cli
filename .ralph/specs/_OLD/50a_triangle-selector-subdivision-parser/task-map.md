# Task Map — Packet 50a

## Backlog Mapping

| Packet Step | Task ID | docs/07 Status | Topic |
|-------------|---------|---------------|-------|
| Steps 1–3 | TASK-180b-prereq | gap (not separately listed) | TriangleSelector dominant-state decoder |
| Steps 4–6 | TASK-180b-prereq | gap (not separately listed) | Sub-triangle stroke geometry population |
| Step 7 | — | — | Packet closure gate |

`TASK-180b` (packet 50b's primary task) is the downstream consumer.  
`TASK-180` (closed by packet 50) is the predecessor; its 8 tests must not regress.

## Predecessor Chain

```
TASK-180  →  closed by packet 50 (paint-input-3mf-ingestion)
              ↓
         [gap: TriangleSelector subdivision deferred]
              ↓
TASK-180b-prereq  →  closed by packet 50a (this packet)
              ↓
TASK-180b  →  closed by packet 50b (paint-input-3mf-mmu-supports)
              ↓
TASK-136   →  unblocked: progress-event failure codes for paint annotations
```

## Authoritative Docs by Step

| Step | Primary Doc | OrcaSlicer Ref |
|------|-------------|---------------|
| 1–3  | docs/02_ir_schemas.md | none (tree format from prior delegation) |
| 4    | — (sub-agent dispatch only) | OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp |
| 5    | docs/08_coordinate_system.md | Step 4 FACT result |
| 6    | docs/02_ir_schemas.md | Step 4 FACT result |
| 7    | — | — |

## Phase Boundary

Phase 1 (Steps 1–3) is independently closeable.  
Once AC-1 through AC-6 pass and `cargo test -p slicer-host --test model_loader_tdd` is green,
packet 50b (`50b_paint-input-3mf-mmu-supports`) may be activated in parallel with Phase 2.

If Phase 2 is deferred, create a follow-up draft packet (e.g. `50a-strokes_paint-stroke-geometry`)
and mark this packet `status: partial-implemented` to indicate Phase 1 is complete.
