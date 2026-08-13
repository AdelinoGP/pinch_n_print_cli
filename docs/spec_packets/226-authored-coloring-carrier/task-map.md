# Task Map: 226-authored-coloring-carrier

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-337` | `Step 1` | `docs/adr/0058-*`, `docs/adr/0044-*`, `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-schema/wit/deps/types.wit`, `crates/slicer-ir/src/slice_ir.rs`, production `ExtrusionPath3D` literals | none | `M` | WIT carrier + IR mirror + schema bump |
| `TASK-337` | `Step 2` | `docs/21_data_defaults_and_fixtures.md` | test/fixture/guest `ExtrusionPath3D` literals | none | `M` | Test blast-radius closure |
| `TASK-337` | `Step 3` | — | `crates/slicer-wasm-host/src/marshal/leaf.rs`, `crates/slicer-macros/src/lib.rs` | none | `S` | Field round-trip in converters |
| `TASK-337` | `Step 4` | `docs/specs/community-modules-dragon-curve-plan.md` | `crates/slicer-schema/wit/deps/common.wit`, `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-sdk/src/host.rs`, `crates/slicer-ir/src/resolved_config.rs` | none | `M` | tool-count service + SDK wrapper + config key |
| `TASK-337` | `Step 5` | `docs/adr/0058-*`, `docs/specs/community-modules-dragon-curve-infill.md` §2 | `crates/slicer-wasm-host/src/marshal/out.rs`, `dispatch.rs`, `marshal/native.rs`, `crates/slicer-runtime/src/layer_executor.rs`, new contract test + aggregator | none | `M` | Grant predicate + strip/clamp enforcement |
| `TASK-337` | `Step 6` | `docs/adr/0058-*` Consequences | `modules/core-modules/infill-linker/src/{orchestrate.rs,connect.rs}`, `tests/connect_tdd.rs` | none | `S` | Linker tool-equality guard |
| `TASK-337` | `Step 7` | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/DEVIATION_LOG.md` | docs + guest rebuild | none | `M` | Guest staleness + docs + DEV-135 |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M. Aggregate = M.
