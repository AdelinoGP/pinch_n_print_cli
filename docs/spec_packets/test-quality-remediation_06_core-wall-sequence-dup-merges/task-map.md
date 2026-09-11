# Task Map: core-wall-sequence-dup-merges

The user-approved plan wave/item IDs are used directly; no `TASK-###` mapping is authorized or required. This file is the docs/07-style crosswalk owner for the approved §5.1 wall-sequence sub-item without editing `docs/07_implementation_status.md`.

| Approved program task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE (wall sequence)` | Steps 1 and 2 | `docs/specs/test-quality-remediation-plan.md` §5.1, §6, and §7; `docs/22_test_quality.md`; ADR-0064; `docs/01_system_architecture.md` wall-sequence range | Step 1: `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` gains the migrated `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` test while `crates/slicer-core/src/perimeter_utils.rs` loses only the four-test `#[cfg(test)] mod wall_sequence_reorder_tests` block; Step 2: §7 `core` ledger row only | `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` delegated `PerimeterGenerator::process` sequencing confirmation | `S` | Accounts for TDD `5 → 6` and inline `4 → 0`, preserves all three modes, the two-wall sandwich input, and both edge cases, and records partial evidence without a Packet Queue change. |

This crosswalk owns only the approved wall-sequence portion of `core/DUP-CORE`. It does not imply closure of the geometry, beading, strengthen, retire, paint, brittle, cross, or parity slices, or the whole `slicer-core` wave, and it exports no symbols to later packets.