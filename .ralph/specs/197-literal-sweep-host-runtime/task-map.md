# Task Map: 197-literal-sweep-host-runtime

Single-task packet; map emitted because the batch plan (`docs/specs/struct-literal-churn-gate-plan.md`) tracks TASK-316–321 across six packets and reviewers need the per-area crosswalk.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-319` | `Step 1` | `docs/21_data_defaults_and_fixtures.md` | none (baseline scratch under `target/sweep-197-*`) | none | M | Baseline is the invariance contract |
| `TASK-319` | `Step 2` | `docs/21_data_defaults_and_fixtures.md` | `crates/pnp-cli/tests/**`, `crates/pnp-cli/Cargo.toml` | none | M | dev-dep + twin activation + fixtures |
| `TASK-319` | `Step 3` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-scheduler/tests/**` | none | S | `ExecutionPlan` FRU |
| `TASK-319` | `Step 4` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-wasm-host/tests/**`, cfg-test src mods | none | S | carrier waivers; `test-guests/**` untouched |
| `TASK-319` | `Step 5` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-runtime/tests/{unit,contract,integration}/**`, `tests/common/mod.rs` | none | M | `pipeline_tdd.rs` → `common::pipeline_config_base` |
| `TASK-319` | `Step 6` | `CLAUDE.md` §Guest WASM Staleness | `crates/slicer-runtime/tests/{executor,e2e}/**`, top-level tests, `benches/**`, `layer_executor.rs` cfg-test | none | M | stale-guest triage before sweep-blame |
| `TASK-319` | `Step 7` | `CLAUDE.md` §Test Discipline | none (verification only) | none | M | area gate + invariance + workspace gates |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
