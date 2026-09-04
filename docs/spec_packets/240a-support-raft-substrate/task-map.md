# Task Map: 240a-support-raft-substrate

Crosswalk for this packet's share of queue row #7 of
`docs/specs/support-families-anchored-entities-plan.md`. Row #7 originally
allocated `TASK-409`..`TASK-418` to a single `240-support-raft` packet; that
packet was split at preflight into **240a-support-raft-substrate** (this one,
`TASK-409`..`TASK-413`) and **240b-support-raft-module**
(`TASK-414`..`TASK-418`). The split exposed scope the original allocation did
not cover, so this packet also carries `TASK-533`..`TASK-536`, taken from the
free range above the highest ID then in use. **Re-derive the free range before
allocating any further ID** — `rg -o 'TASK-[0-9]{3}' docs/ -N --no-filename | sort -u | tail -1`
— rather than trusting the boundary implied here.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-409` | `Step 1` | plan §12 | `crates/slicer-ir/tests/{signed_layer_indices_tdd,sliced_region_raft_fill_tdd}.rs` | none | S | Red-first IR contract |
| `TASK-410` | `Step 2a` | `docs/02_ir_schemas.md` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs` + crates-side sweep fallout | none | M | u32→i32, crates half; split trigger if >~20 files |
| `TASK-411` | `Step 2b` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-sdk/src/test_support/fixtures.rs`, macro test files, modules + test fallout | none | M | u32→i32, modules/tests half; literal gate |
| `TASK-412` | `Step 3` | `docs/03_wit_and_manifest.md` | `crates/slicer-macros/src/lib.rs`, `crates/slicer-macros/tests/binding_surface_tdd.rs` | none | S | kills the `as u32` sign truncation |
| `TASK-413` | `Step 4` | `design.md` §Positional Consumer Ruling | `crates/slicer-runtime/src/layer_executor.rs`, `builtins/{prepass_slice_producer,support_analysis_producer}.rs`, `tests/executor/*` | none | M | `index != Vec position` repair |
| `TASK-533` | `Step 5` | `docs/03_wit_and_manifest.md` | `prepass-layer-planning.wit`, `crates/slicer-wasm-host/src/marshal/{in_,native}.rs`, `tests/marshal_layer_plan_prefix_tdd.rs` | `SupportCommon.cpp::generate_support_layers` (delegated) | M | `is-raft-prefix` + negative index assignment, both legs |
| `TASK-534` | `Step 6` | `docs/08_coordinate_system.md` | `modules/core-modules/layer-planner-default/*`, `crates/slicer-runtime/tests/integration/*` | `Slicing.cpp::generate_object_layers` (delegated) | M | planner emits the `-N..-1` band; guest rebuild |
| `TASK-535` | `Step 7` | `docs/02_ir_schemas.md` | `slice_ir.rs`, `ir-types.wit`, `region_partition.rs` + carrier footprint | none | M | `raft_fill` carrier + both accessors + schema minor bump (next minor above the live `CURRENT_SLICE_IR_SCHEMA_VERSION`, re-derived from `crates/slicer-ir/src/slice_ir.rs` at the moment of the edit) |
| `TASK-536` | `Steps 8+9` | `docs/03_wit_and_manifest.md`, `docs/02_ir_schemas.md` | `ir-types.wit`, `host.rs`, `dispatch.rs`, `traits.rs`; then docs + `DEVIATION_LOG.md` | none | M+S | `raft_plan` read path; DEV-124 reopen row |

Copy costs from `implementation-plan.md`. Aggregate is `L`; no row is `L`.
