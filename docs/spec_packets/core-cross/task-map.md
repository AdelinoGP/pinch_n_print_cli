# Task Map: core-cross

This packet spans a single backlog item (`core/CROSS`) across four test surfaces plus the ledger row, so the crosswalk records each step's surface even though no `TASK-###` mapping exists (plan wave/item IDs replace `TASK-###` under the standing plan exemption).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/CROSS` | `Step 1` | `docs/specs/test-quality-remediation-plan.md` §5.1 CROSS, §6; `docs/22_test_quality.md` §§1-3; `docs/adr/0064` | `crates/slicer-sdk/tests/host_wrappers_tdd.rs` (7 markers: 6 retained + 1 new-test, plus the new miter-limit delegate test) | none | `S` | Proves the SDK offset delegation arms keep thin-delegate cover, incl. the previously uncovered `Some(miter_limit)` arm. |
| `core/CROSS` | `Step 2` | `docs/specs/test-quality-remediation-plan.md` §5.1 CROSS; `docs/22_test_quality.md` §3 | `crates/slicer-sdk/src/host_batch.rs` (2 markers in `#[cfg(test)]` mod) | none | `S` | Proves batch offset alignment and batch/singular consistency; guest-staleness check applies. |
| `core/CROSS` | `Step 3` | `docs/08_coordinate_system.md` (delegated) | `crates/slicer-core/tests/aabb_tree_tdd.rs` (3 markers) | none | `S` | Proves the core AabbTree counterparts are kept distinct from the SDK MeshSource route. |
| `core/CROSS` | `Step 4` | `docs/22_test_quality.md` §§1-3 | `crates/slicer-core/src/polygon_ops.rs` `mod tests` (2 markers) | none | `S` | Proves the core offset/RDP-simplify oracles are kept as independent counterparts. |
| `core/CROSS` | `Step 5` | `docs/specs/test-quality-remediation-plan.md` §7 | `docs/specs/test-quality-remediation-plan.md` §7 `core` row (append only) | none | `S` | Wave-exit ledger disposition; AC-7 binds the accumulated row. |

OrcaSlicer refs: none — no parity work exists in this row (`core/PARITY` is packet #14). Copy costs from `implementation-plan.md`; aggregate `M`, no L step, no split required.
