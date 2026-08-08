# Task Map: 199-literal-gate-enforcement

Single-task packet; emitted anyway because the batch protocol assigned this file the `docs/07` crosswalk for the plan's terminal row (TASK-321 closes `docs/specs/struct-literal-churn-gate-plan.md`), and the one task spans six heterogeneous steps a reviewer needs mapped.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-321` | `Step 1` | `docs/specs/struct-literal-churn-gate-plan.md` | none (baselines under `target/`) | none | S | Tool-derived residue list + pre-edit baselines make every later count re-derived, not quoted |
| `TASK-321` | `Step 2` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-model-io/tests/**`, `crates/slicer-model-io/src/loader.rs` (cfg-test) | none | M | Largest residue area converted to FRU-over-base; proves the conversion rules hold outside sweep packets |
| `TASK-321` | `Step 3` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-helpers/tests/**`, `crates/slicer-macros/tests/slicer_module_tdd.rs` | none | S | Reasoned-waiver path exercised (file-local bases; unrenamable name-colliding mocks) |
| `TASK-321` | `Step 4` | `docs/specs/struct-literal-churn-gate-plan.md` (decision 4) | `xtask/src/test.rs`, `xtask/src/check_literals.rs`, `xtask/src/main.rs` | none | M | The actual enforcement wiring; negative ACs prove the block |
| `TASK-321` | `Step 4b` | `docs/specs/struct-literal-churn-gate-plan.md` (decision 4, user-extended to CI 2026-08-07) | `.github/workflows/ci.yml` (`docs-guard` job, one step) | none | S | CI enforcement; without it the gate is local-only because CI's `test` job bypasses `cargo xtask test` |
| `TASK-321` | `Step 5` | `docs/21_data_defaults_and_fixtures.md`, `CLAUDE.md` | `CLAUDE.md`, `docs/21_data_defaults_and_fixtures.md` | none | S | Gate-off → enforced flip; stale-fact repair; sdk hazard addendum |
| `TASK-321` | `Step 6` | `docs/07_implementation_status.md` (dispatch) | none | none | S | Waiver audit, workspace gates, crosswalk; plan closes |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
