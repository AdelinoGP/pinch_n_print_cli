# Task Map: lightning-migration

Single-task crosswalk for `TASK-577` (`docs/07_implementation_status.md`). This file owns the backlog crosswalk only; acceptance criteria live in `packet.spec.md`, scope and the verification matrix in `requirements.md`, code surface in `design.md`, step contracts in `implementation-plan.md`.

| Task | Steps | Primary authorities | Expected surface | Canonical refs | Context | State |
| --- | --- | --- | --- | --- | --- | --- |
| TASK-577 | 1 | docs22 oracle discipline; docs21 fixtures; docs04 prepass ordering | `crates/slicer-runtime/tests/e2e/lightning_baseline.rs` + committed baseline fixtures | `Fill.cpp::Layer::make_fills` (caller identity the baseline records) | M | Planned: recorded oracle before the first production edit |
| TASK-577 | 2 | docs22 (no self-referential oracle); docs21 | `crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs` + registration + dev-deps | none new | M | Planned: nine NET-NEW tests, AC-1 green at exit |
| TASK-577 | 3, 4, 5 | docs02 IR schemas; docs05 module SDK; docs08 units | `modules/core-modules/lightning-infill/src/kernel/{mod,plan,error,tree_node,layer,distance_field,generator,input}.rs` | `Generator.cpp::Generator::generateTrees` (relocated unchanged) | M | Planned: code and plan types move together; planning decisions frozen |
| TASK-577 | 6, 10c | docs22; docs01 | module `tests/kernel_tdd.rs`; delete `crates/slicer-core/src/algos/lightning/`, its `[[test]]` entry and `algos/mod.rs` declaration | none new | S | Planned: host copy retired after the producer (10a) |
| TASK-577 | 7a, 7b, 8 | ADR-0066 as amended; rows 03–06 contracts; docs04/05/08/19 | `src/private_plan.rs`, `src/preparation.rs`, `src/projection.rs`, `src/lib.rs`, `lightning-infill.toml` | none new | M | Planned: AC-2/AC-3/AC-4 gate the deletion phase |
| TASK-577 | 9 | docs22 retire-if-unjustified | module test files | none new | S | Planned |
| TASK-577 | 10a–10c | ADR-0066 as amended; docs01/04 | `prepass.rs` + executor test, producer file, `builtins/mod.rs`, `runtime/src/lib.rs`, `algos/lightning/` + `algos/mod.rs` | none new | S | Blocked until AC-2/AC-3 green; leaf-first |
| TASK-577 | 11a–11d | docs03/05; docs22 compile witnesses | `sdk/traits.rs`, `macros/{lib,build}.rs`, `preparation_bindings.rs`, `binding.rs`, `layer_executor.rs`, `dispatch.rs`, `host.rs`, `marshal/native.rs`, test guest | none new | M | Blocked until AC-2/AC-3 green |
| TASK-577 | 12a, 12b | docs02; docs04 | `blackboard.rs`, `slicer-ir/{lib,slice_ir,stage_io}.rs`, `execution_plan.rs`, `stage_order_tdd.rs`, `stage_list_consistency_tdd.rs` | none new | M | Blocked; ordered after 11a–11d so no reference outlives the type |
| TASK-577 | 13a–13c | docs03; docs14; AGENTS.md guest freshness | `lightning-queries.wit` package + `layer-infill.wit` imports, dragon-curve mirror/glue/`Makefile`, `xtask/src/wit_verify.rs`, schema snapshot test, runtime drift test | none new | S/M | Blocked; package deleted whole, generated output regenerated |
| TASK-577 | 14a–14e | docs22 population discipline | runtime contract/executor/integration test binaries, wasm-host contract binary, standalone raft target, shared test helper | none new | M | Blocked; one binary per step, retarget never narrow |
| TASK-577 | 15a, 15b | docs00 map; ADR-0066/0029 as amended | `docs/01`, `02`, `03`, `04`, `05`, `19` | none new | S | Blocked; placement-only greps |
| TASK-577 | 16 | packet ACs; docs21/22 | verification only; source-plan row 08 note + `docs/07` `TASK-577` line | none new | M | Closure |

## Dependency Change From the Previous Revisions

Two earlier revisions of this crosswalk were unusable, for different reasons.

The first required "independently verified actual executed Gate A/B outcomes recorded in the source plan" before any step could run. That was unsatisfiable (row 07 records `Gate A outcome: fail`; row 09's kernel had empty `planner.rs`/`tree.rs` stubs, so Gate B never had a planner) and tested a property this packet does not claim.

The second replaced that gate with recorded equivalence but left a compile-fatal hole: the planner's return type *is* the public plan IR, which the same plan deleted, while the algorithm directory was declared out of bounds. That is closed by moving the planner and the plan types together into the module's kernel and restating the constraint as "planning logic frozen" rather than "algorithm untouched".

What this packet requires is: rows 01–06 implemented (they are), the baseline recorded deterministically, AC-2/AC-3 green **before deletion begins**, and the ADR amendments present. Gate A/B stay recorded exactly as they are and are neither consumed nor updated here.

`TASK-572`–`TASK-576` remain predecessor-owned and are not absorbed or marked complete. The only ledger edits this packet authorizes are the row-08 note in `docs/specs/layer-module-preparation-plan.md` and the `TASK-577` line in `docs/07_implementation_status.md`, both in Step 16 and both re-derived at edit time. The ADR amendments were recorded during authoring, not as an implementation step.

## Status

`status: draft` — implementation-grade pending independent preflight. Activation is a separate explicit decision; this revision does not flip status.
