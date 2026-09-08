# Task Map: 196-literal-sweep-core-ir-gcode

Single-task packet; map emitted because the batch plan (`docs/specs/struct-literal-churn-gate-plan.md`) tracks TASK-316–321 across six packets and reviewers need the per-area crosswalk.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-318` | `Step 1` | `docs/21_data_defaults_and_fixtures.md` | none (baseline scratch under `target/sweep-196-*`) | none | M | Baseline is the invariance contract for every later step |
| `TASK-318` | `Step 2` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-ir/tests/**`, cfg-test mod in `crates/slicer-ir/src/slice_ir.rs` | none | S | FRU + carrier waivers |
| `TASK-318` | `Step 3` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-gcode/tests/**`, `crates/slicer-gcode/Cargo.toml` | none | S | sdk dev-dep + `print_entity_base` |
| `TASK-318` | `Step 4` | `CLAUDE.md` §Feature-gated test files | `crates/slicer-core/tests/**`, `benches/**`, cfg-test src mods | none | M | `--features host-algos` mandatory |
| `TASK-318` | `Step 5` | `CLAUDE.md` §Guest WASM Staleness | none (verification only) | none | M | Area gate + freshness + workspace gates |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.
