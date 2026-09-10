# Task Map: core-flow-consolidation

The user-approved program IDs are used directly; no `TASK-###` mapping is authorized or required.

| Approved program task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE (flow)` | Steps 1 and 2 | `docs/specs/test-quality-remediation-plan.md` §5.1 and §7; `docs/22_test_quality.md` | Step 1: `crates/slicer-core/src/flow.rs` test block and `crates/slicer-core/tests/flow_tdd.rs`; Step 2: `docs/specs/test-quality-remediation-plan.md` §7 `core` row | None; pre-existing API behavior | `S` | Merge the inline/integration flow union without dropping distinct inputs, then record partial survivor/validation evidence without closing other core work. |
| `core/RETIRE (flow)` | Steps 1 and 2 | `docs/specs/test-quality-remediation-plan.md` §5.1 and §7; ADR-0064 | Step 1: same two test homes; Step 2: §7 `core` row | None | `S` | Retire the inline duplicate home only after every case has a surviving integration assertion, then record the scoped ledger disposition. |
| `core/DUP-CORE-strengthen (wider-bead)` | Steps 1 and 2 | `docs/specs/test-quality-remediation-plan.md` §5.1 and §7; `docs/22_test_quality.md` | Step 1: `wider_bead_spacing_is_larger_than_canonical` in `flow_tdd.rs`; Step 2: §7 `core` row | None; no porting | `S` | Keep the independent formula oracle and strict wider-than-canonical assertion, then record the partial evidence and remaining `core-bridge-dup-merge` gap. |

This exact crosswalk owns both the flow-only sub-items and their mandatory §7 evidence bookkeeping. It does not imply closure of the remaining `core/DUP-CORE`, `core/RETIRE`, or `core/DUP-CORE-strengthen` items or the whole core wave, and it exports no symbols to later packets.
