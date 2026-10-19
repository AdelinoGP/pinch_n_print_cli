# Task Map: core-bridge-dup-merge

The user-approved program IDs are used directly; no `TASK-###` mapping is authorized or required.

| Approved program task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE (bridge)` | Steps 1 and 2 | `docs/specs/test-quality-remediation-plan.md` §5.1 and §7; `docs/22_test_quality.md`; ADR-0064 | Step 1: `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` duplicate deletion; Step 2: `docs/specs/test-quality-remediation-plan.md` §7 `core` row | None; pre-existing API behavior | `S` | Merge the textually identical supported-bridge duplicate into its plan-named survivor without touching the five distinct guards, then record partial survivor/validation evidence without closing other core work. |

This exact crosswalk owns the bridge-only sub-item and its mandatory §7 evidence bookkeeping. It does not imply closure of the remaining `core/DUP-CORE` items (region mapping, support, wall sequence, geometry, beading), `core/RETIRE`, `core/DUP-CORE-strengthen`, `core/PAINT`, `core/BRITTLE`, `core/CROSS`, or `core/PARITY`, or the whole core wave, and it exports no symbols to later packets.