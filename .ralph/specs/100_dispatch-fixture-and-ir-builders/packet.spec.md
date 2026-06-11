---
status: draft
packet: 100_dispatch-fixture-and-ir-builders
task_ids: []
backlog_source: architecture review (session 2026-06-11, /improve-codebase-architecture Candidate 4 — DispatchFixture & suite split)
context_cost_estimate: M
---

# Packet Contract: 100_dispatch-fixture-and-ir-builders

## Goal

Concentrate the hand-rolled dispatch test scaffolding under `crates/slicer-runtime/tests/common/` into two new modules — a fluent `DispatchFixture` builder that owns the dispatcher + `Blackboard` + `LayerArena` and exposes four per-runner `run_*` methods, and an `ir_builders` module with distinct `with_count` / `with_ids` constructors per IR type — then prove the surface covers both lifecycles by migrating two existing tests in `dispatch_tdd.rs`.

## Scope Boundaries

This packet adds two new test-support modules in `crates/slicer-runtime/tests/common/` and migrates exactly two existing tests in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` as proof of coverage. The axis-aligned split of `dispatch_tdd.rs` into eight files and the migration of the remaining ~99 tests is deferred to packet 101. The parallel fixture set under `crates/slicer-wasm-host/tests/common/` is explicitly out of scope and must remain unchanged (per the ADR-0007 amendment recorded 2026-06-11).

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: packet `101_dispatch-tdd-axis-aligned-split` (requires `DispatchFixture` + `ir_builders` to exist).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the new modules `crates/slicer-runtime/tests/common/dispatch_fixture.rs` and `crates/slicer-runtime/tests/common/ir_builders.rs` exist and are registered in `tests/common/mod.rs`, **when** `cargo check --workspace --all-targets` is run, **then** it returns exit code 0 with no errors and no warnings. | `cargo check --workspace --all-targets`

- **AC-2. Given** the migrated test `missing_component_gracefully_skipped` in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` constructs its fixture via `DispatchFixture::for_stage("Layer::Infill").no_wasm().build()` (replacing the previous `make_compiled_module_no_wasm("com.test-missing", "Layer::Infill")` call) and dispatches via `fx.run_layer(&layer)?`, **when** the test is run, **then** the dispatcher returns `Ok(LayerStageCommitData::default())` and `fx.arena.slice()` / `fx.arena.infill_output_for(0)` remain at their default-empty state (no panics, no error variants). | `cargo test -p slicer-runtime --test contract -- missing_component_gracefully_skipped --nocapture 2>&1 | tee target/test-output.log && rg -q 'test missing_component_gracefully_skipped \.\.\. ok' target/test-output.log`

- **AC-3. Given** the migrated test `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` constructs its fixture via `DispatchFixture::for_stage("Layer::InfillPostProcess").with_slice(ir_builders::slice_ir::with_count(3).at_z(0.4).build()).with_perimeter(ir_builders::perimeter_ir::with_count(3).at_layer(2).walls(2).infill(4).build()).build()` and dispatches via `fx.run_layer(&layer)?`, **when** the test is run, **then** the round-trip assertion holds: the first emitted infill path's `point[0]` carries `x == 3.0` (region_count), `y == 6.0` (total wall_loops = 3×2), `z == 12.0` (total infill polygons = 3×4). | `cargo test -p slicer-runtime --test contract -- real_perimeter_region_data_visible_through_infill_postprocess_dispatch --nocapture 2>&1 | tee target/test-output.log && rg -q 'test real_perimeter_region_data_visible_through_infill_postprocess_dispatch \.\.\. ok' target/test-output.log`

- **AC-4. Given** a new unit test `ir_builders_slice_ir_with_count_shape` in `dispatch_tdd.rs` exercises `ir_builders::slice_ir::with_count(3).at_z(0.2).build()`, **when** the test runs, **then** the returned `SliceIR` carries `global_layer_index == 0`, `z == 0.2`, `regions.len() == 3`, and for each `i in 0..3` the i-th region satisfies `object_id == format!("obj-{i}")`, `region_id == i as u64`, `polygons.len() == 1`, `polygons[0].contour.points.len() == 4`, `polygons[0].holes.is_empty()`, `effective_layer_height == 0.2`. | `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_count_shape --nocapture 2>&1 | tee target/test-output.log && rg -q 'test ir_builders_slice_ir_with_count_shape \.\.\. ok' target/test-output.log`

- **AC-5. Given** the new `ir_builders::slice_ir::with_ids` constructor, **when** the new unit test `ir_builders_slice_ir_with_ids_shape` calls `ir_builders::slice_ir::with_ids(&[("custom-obj", 17), ("other-obj", 99)]).at_z(0.5).build()`, **then** the returned `SliceIR` carries `regions.len() == 2`, `regions[0].object_id == "custom-obj"`, `regions[0].region_id == 17`, `regions[1].object_id == "other-obj"`, `regions[1].region_id == 99`. | `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_ids_shape --nocapture 2>&1 | tee target/test-output.log && rg -q 'test ir_builders_slice_ir_with_ids_shape \.\.\. ok' target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** all four ACs above pass, **when** `cargo clippy --workspace --all-targets -- -D warnings` is run, **then** it returns exit code 0 (no diagnostics elevated to errors, no warnings in the new files). | `cargo clippy --workspace --all-targets -- -D warnings`

- **AC-N2. Given** the parallel fixture set under `crates/slicer-wasm-host/tests/common/` predates this packet, **when** `git status --porcelain crates/slicer-wasm-host/tests/common/` is run after all packet commits land, **then** the output is empty (zero modifications to that directory). | `test -z "$(git status --porcelain crates/slicer-wasm-host/tests/common/)"`

## Verification

Gate commands only — the full matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract -- missing_component_gracefully_skipped real_perimeter_region_data_visible_through_infill_postprocess_dispatch ir_builders_slice_ir_with_count_shape ir_builders_slice_ir_with_ids_shape`

## Authoritative Docs

- `docs/adr/0007-compiled-module-static-live-split.md` — load directly (175 lines incl. the 2026-06-11 amendment that locks the conventions this packet implements).
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — load directly (142 lines; the four runner traits whose `run_stage` signatures the per-runner methods wrap).
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — load directly (72 lines; explains why `DispatchFixture` lives in `tests/common/` rather than `slicer-sdk::test_support`).
- `CLAUDE.md` §Test Discipline — read lines 51–86 only (narrow-test rules + `target/test-output.log` tee requirement).
- `docs/05_module_sdk.md` — delegate a SUMMARY of lines 446–596 if test-support context is needed (full file is 150 lines, but only the test-support section is relevant).

## Doc Impact Statement (Required)

**`none`** — this is an internal test-scaffolding refactor with no public surface change, no IR field touch, no WIT type touch, no scheduler rule change, no manifest schema edit, and no module SDK contract change. The ADR-0007 amendment locking the dispatch fixture conventions was recorded in a prior session under the `/improve-codebase-architecture` skill and is not in scope for this packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
