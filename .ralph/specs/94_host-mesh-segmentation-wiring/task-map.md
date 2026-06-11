# Task Map: 94_host-mesh-segmentation-wiring

This is a retirement packet (TASK-244 → TASK-250 supersession). The bridge to the backlog and the archival record both live in `closure-log.md`; this file is a thin pointer to keep the standard packet template intact.

| Task ID | Backlog Source | Status | Bridge |
| --- | --- | --- | --- |
| `TASK-244` | `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2" (historical) | **CLOSED** by retirement (per `docs/07_implementation_status.md:217`) | `closure-log.md` §"TASK-250 Supersession Rationale" + `docs/specs/paint-pipeline-orca-parity-roadmap.md:579-589` (§P2 addendum) |

The original TASK-244 framing ("wire `execute_mesh_segmentation` into a `PrePass::MeshSegmentation` stage") was retired via the TASK-250 architectural finding that the host kernel was orphaned and structurally incompatible with OrcaSlicer-pattern leaves; the loader's `split_triangle_strokes` is the canonical TriangleSelector normalization site post-P94. Full rationale: `closure-log.md`.
