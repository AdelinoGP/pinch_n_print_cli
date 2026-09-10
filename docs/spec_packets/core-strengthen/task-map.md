# Task Map: core-strengthen

The approved plan explicitly uses wave/item IDs instead of `TASK-###` mappings; `docs/specs/test-quality-remediation-plan.md` is the backlog source. The same grouped item owns the two code steps and the separate §7 ledger step.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE-strengthen (excluding wider-bead)` | `Step 1` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/tests/bridge_over_infill_tdd.rs`; `crates/slicer-core/src/lib.rs` | none; analytic local expectations | `S` | Retain bridge determinism and strengthen the degree oracle plus exact vertical-flow fallback. |
| `core/DUP-CORE-strengthen (excluding wider-bead)` | `Step 2` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs`; `crates/slicer-core/tests/polygon_ops_tdd.rs` | none; analytic local expectations | `S` | Strengthen endpoint geometry and boolean area/bounds/component invariants without vertex-order pins. |
| `core/DUP-CORE-strengthen (excluding wider-bead)` | `Step 3` | `requirements.md`, `design.md`, `implementation-plan.md` | `docs/specs/test-quality-remediation-plan.md` §7 `core` row only | none | `S` | Record accumulated partial progress, own validation, and remaining gap; queue remains parent-owned. |
