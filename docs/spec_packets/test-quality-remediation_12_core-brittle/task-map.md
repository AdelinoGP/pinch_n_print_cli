# Task Map: core-brittle

The approved plan explicitly uses wave/item IDs instead of `TASK-###` mappings; `docs/specs/test-quality-remediation-plan.md` is the backlog source. The same grouped item owns the three guard steps and the separate §7 ledger step.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/BRITTLE` | `Step 1` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/algos/mesh_analysis.rs` | none; local regression guard | `S` | Keep the 1200-facet 3s guard with its incremental-union regression rationale. |
| `core/BRITTLE` | `Step 2` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/algos/overhang_annotation.rs` | none; local sweep guard | `S` | Keep the 1200-layer 1s guard with its setup-vs-timed-region rationale. |
| `core/BRITTLE` | `Step 3` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` | none; local cache guard | `S` | Keep the 1.5x ratio guard with its self-normalizing rationale; preserve the 6-name roster plus the optional retire 7th name (FORWARD-DEP). |
| `core/BRITTLE` | `Step 4` | `requirements.md`, `design.md`, `implementation-plan.md` | `docs/specs/test-quality-remediation-plan.md` §7 `core` row only | none | `S` | Record KEEP-review dispositions, three names, oracle tokens, and the updated remaining gap; queue stays parent-owned. |
