# Task Map: 198-literal-sweep-sdk-modules

Single-task packet; map emitted because the batch plan (`docs/specs/struct-literal-churn-gate-plan.md`) tracks TASK-316–321 across six packets and reviewers need the per-area crosswalk.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-320` | `Step 1` | `docs/21_data_defaults_and_fixtures.md` | none (baseline scratch under `target/sweep-198-*`) | none | M | Baseline uses `--features test` for the sdk |
| `TASK-320` | `Step 2` | `docs/adr/0004-test-support-lives-in-slicer-sdk.md` | `crates/slicer-sdk/tests/**`, `crates/slicer-sdk/Cargo.toml`, waivers in `src/test_support/**` | none | M | Gating entries + fixture/FRU conversions |
| `TASK-320` | `Step 3` | `docs/21_data_defaults_and_fixtures.md` | batch-A `modules/core-modules/*/tests/**` | none | M | Largest 4 modules by violating files |
| `TASK-320` | `Step 4` | `docs/21_data_defaults_and_fixtures.md` | batch-B `modules/core-modules/*/tests/**` | none | S | Remaining listed modules |
| `TASK-320` | `Step 5` | `CLAUDE.md` §Guest WASM Staleness | none (rebuild + verification only) | none | M | Expected `STALE:` after manifest edit → rebuild → clean gate |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
