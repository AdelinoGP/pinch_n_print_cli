# Task Map: core-geometry-dup-review

This packet spans two audit item IDs from one §5.1 row, and the user approved plan wave/item IDs instead of `docs/07` `TASK-###` mappings (standing batch exemption recorded in the plan's Packet Queue preamble). The crosswalk therefore maps the plan item IDs, not `TASK-###` rows; there is no `docs/07_implementation_status.md` row for this slice.

| Plan item ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `core/DUP-CORE` (segment path) | `Step 1` | `docs/specs/test-quality-remediation-plan.md` §5.1 | `crates/slicer-core/src/lib.rs` (inline test removal), `crates/slicer-core/tests/geometry_helpers_tdd.rs` (six-case union incl. preserved `(2.0, 0.75)` tuple) | none | `S` | Fold is case-preserving (AC-2): exact `assert_eq!` endpoint witnesses, non-vacuity, chord cap, nonzero-chord non-degeneracy. |
| `core/DUP-CORE` (point-to-segment distance) | `Step 1` | `docs/specs/test-quality-remediation-plan.md` §5.1 | `crates/slicer-core/src/geometry.rs` (inline test consolidation) | none | `S` | Identical fixture/oracle pair consolidated (AC-3): both APIs called explicitly, both independent `25_000_000.0_f64` oracles retained, plus `assert_eq!` wrapper-delegate witness and ±1 projection witnesses. |
| `core/DUP-CORE` (both, ledger) | `Step 2` | `docs/specs/test-quality-remediation-plan.md` §7 | `docs/specs/test-quality-remediation-plan.md` (`core` row only) | none | `S` | Ledger rows store re-derivable facts; verified via AC-5's scoped named-column parser and synthetic falsified-state controls, never a global slug grep. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.