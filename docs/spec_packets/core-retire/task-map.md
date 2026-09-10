# Task Map: core-retire

The approved plan explicitly uses wave/item IDs instead of `TASK-###` mappings; `docs/specs/test-quality-remediation-plan.md` is the backlog source. The same grouped item owns the two code steps and the separate §7 ledger step.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/RETIRE (excluding flow)` | `Step 1` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/src/polygon_ops.rs`; `crates/slicer-core/tests/triangle_mesh_slicer_tdd.rs` | none; local contract only | `S` | Delete the two tests no production defect can falsify, keeping every survivor, helper, import, and banner. |
| `core/RETIRE (excluding flow)` | `Step 2` | `requirements.md`, `design.md`, `implementation-plan.md` | `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` | none; analytic local expectations | `M` | Re-home the NEG-3 intent as a contrast pair driving the real closing-radius gate in the `host-algos`-gated target. |
| `core/RETIRE (excluding flow)` | `Step 3` | `requirements.md`, `design.md`, `implementation-plan.md` | `docs/specs/test-quality-remediation-plan.md` §7 `core` row only | none | `S` | Record retirements, replacement, the three function-count deltas, and the updated remaining gap; queue stays parent-owned. |

The `flow.rs` rows of the audit's RETIRE item are excluded here and remain owned by `core-flow-consolidation`.
