# Task Map: 212-extra-perimeters-parity

Single task ID, but emitted because the preflight gate (S0) requires all five contract files and because `TASK-328` is newly allocated by the approved plan rather than pre-existing in the backlog — that allocation needs an explicit record.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-328` | `Step 1` | `docs/15_config_keys_reference.md` (rg the `extra_perimeters` owner rows only) | `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic` / `process_arachne` `loop_number` fold; delegate, never load | `S` | Red proof: the arachne and cross-generator cases must fail on a numeric wall-count mismatch, proving DEV-132 half (a) is live in the tree and not already fixed. |
| `TASK-328` | `Step 2` | `docs/03_wit_and_manifest.md` (delegated SUMMARY, only if the `int` key field set is in doubt) | `modules/core-modules/arachne-perimeters/src/lib.rs` (`arachne_params_from_config`), `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` (`ARACHNE_FALLBACKS`) | `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` — `WallToolPaths::generate`'s `max_bead_count = 2 * inset_count`; delegate | `S` | The fix itself. Proves the task by turning all seven `extra_perimeters_config_tdd` tests green while `alternate_extra_wall_tdd` stays at `2 passed`. |
| `TASK-328` | `Step 3` | `docs/15_config_keys_reference.md` (generated; regenerate only) | none (generated doc tables only) | none | `S` | Proves the new manifest key is visible in the canonical generated config catalog, i.e. the key is a real user-facing surface under arachne and not a silent internal read. |
| `TASK-328` | `Step 4` | `docs/DEVIATION_LOG.md` (DEV-132 row, ranged), `docs/07_implementation_status.md` (delegated) | none (ledger only) | `OrcaSlicerDocumented/src/libslic3r/Surface.hpp`, `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `Surface::extra_perimeters` and its dead writer `PrintObject::make_perimeters`; delegate | `S` | Closes DEV-132 half (a), re-files half (b) under a freshly re-derived `DEV-###`, and creates the `TASK-328` backlog line (verified absent this session; highest present was `TASK-315`). |

Costs copied from `implementation-plan.md`. No row is `L`; aggregate is `S`.

## Backlog-allocation note

`TASK-328` is assigned by row #7 of `docs/specs/deviation-remediation-206-212-plan.md` (approved queue) and **does not yet exist** in `docs/07_implementation_status.md`. Step 4 creates it. This is deliberate scope, not a stale reference. The surrounding IDs `TASK-322`–`TASK-327` belong to packets 206–211 of the same queue and are likewise plan-allocated; `TASK-320`/`TASK-321` are already claimed by `docs/specs/struct-literal-churn-gate-plan.md`. Re-derive the highest live `TASK-###` at the moment of writing rather than trusting this note.
