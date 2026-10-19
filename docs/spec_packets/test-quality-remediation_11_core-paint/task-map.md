# Task Map: core-paint

The approved plan explicitly uses wave/item IDs instead of `TASK-###` mappings; `docs/specs/test-quality-remediation-plan.md` is the backlog source. The same grouped item owns the three code steps and the separate §7 ledger step.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/PAINT` | `Step 1` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/algos/paint_segmentation/mod.rs` | none; local short-circuit contract | `S` | Strengthen the three driver guards with nonempty inputs and unchanged-path equality. |
| `core/PAINT` | `Step 2` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/algos/paint_segmentation/voronoi_prune.rs` | none; local prune rule | `S` | Replace the no-panic ending with the derived deletion plus index pins. |
| `core/PAINT` | `Step 3` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` | none; local constructor plus probe review | `S` | Add the Err arm for twin propagation and close probes with one compile-witness waiver. |
| `core/PAINT` | `Step 4` | `requirements.md`, `design.md`, `implementation-plan.md` | `docs/specs/test-quality-remediation-plan.md` §7 `core` row only | none | `S` | Record FIX/KEEP dispositions, nine names, oracle tokens, and the updated remaining gap; queue stays parent-owned. |
