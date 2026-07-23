# Task Map: 170-seam-livepath-audit

Single-task packet, but the crosswalk is emitted because it reconciles prior work: TASK-120c has a standalone reopened row `- [~] TASK-120c Restore seam placement on real wall-loop seam candidates ...` in `docs/07_implementation_status.md` (reopened 2026-04-21 by packet `22_live-seam-contract-repair`), and is also referenced by the TASK-120, TASK-151, and TASK-159 rows. Step 3 updates that reopened row in place (flip `[~]` to `[x]` closed or `[ ]` re-scoped per the audit outcome, replacing the stale gap text). No line-number pins — the file has been edited since this packet was drafted. See `implementation-plan.md` Step 3 for anchor and format.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-120c` | `Step 1` | none (builder FACTs go to `crates/slicer-sdk/src/builders.rs`) | `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` | none | S | Fixtures are the audit instrument proving the wall-preservation half of TASK-120c. AC-3 exercises `select_seam_candidate` (not `project_onto_wall_segment`); AC-N1 restricted to `nearest` mode. Mirror helper shape from packet 180's `seam_continuous_projection_tdd.rs` / `seam_degraded_fallback_tdd.rs`. |
| `TASK-120c` | `Step 2` (conditional) | none | `modules/core-modules/seam-placer/src/lib.rs` | none | S | Fix only on falsification; skip-record otherwise |
| `TASK-120c` | `Step 3` | `docs/07_implementation_status.md` | docs only | none | S | Reconciles the reopened TASK-120c row: closes or re-scopes the task. Anchor is the literal `[~] TASK-120c` row text; no line-number pin. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
