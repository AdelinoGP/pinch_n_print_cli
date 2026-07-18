---
status: implemented
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

- **AC-2. Given** the migrated test `missing_component_gracefully_skipped` in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` constructs its fixture via `DispatchFixture::for_stage("Layer::Infill").no_wasm().build()` (replacing the previous `make_compiled_module_no_wasm("com.test-missing", "Layer::Infill")` call) and dispatches via `fx.run_layer(&layer)?`, **when** the test is run, **then** the dispatcher returns `Ok(_)` and `fx.arena.take_infill().is_none()` (the arena is in its default-empty state — no `solid_infill` was committed, no error variants raised, no panics). The migration preserves the pre-packet observable behavior bit-identically. | `cargo test -p slicer-runtime --test contract -- missing_component_gracefully_skipped --nocapture 2>&1 | tee target/test-output.log && rg -q 'test (dispatch_tdd::)?missing_component_gracefully_skipped \.\.\. ok' target/test-output.log`

- **AC-3. Given** the migrated test `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` constructs its fixture via `DispatchFixture::for_stage("Layer::InfillPostProcess").with_slice(ir_builders::slice_ir::with_count(3).at_z(0.4).build()).with_perimeter(ir_builders::perimeter_ir::with_count(3).at_layer(2).walls(2).infill(4).build()).build()` and dispatches via `fx.run_layer(&layer)?`, **when** the test is run, **then** the round-trip assertion holds: for each of the 3 output `InfillRegion`s, the first emitted infill path's `point[0]` carries `x == 2.0` (the per-region wall count) and `y == 4.0` (the per-region infill polygon count), and `r.object_id == "obj-{i}"` / `r.region_id == i as u64` for `i in 0..3`. The WAT test guest encodes per-region counts, not aggregate counts, so the assertion is per-region rather than the aggregate `(3, 6, 12)`; the migration preserves the pre-packet observable behavior bit-identically. | `cargo test -p slicer-runtime --test contract -- real_perimeter_region_data_visible_through_infill_postprocess_dispatch --nocapture 2>&1 | tee target/test-output.log && rg -q 'test (dispatch_tdd::)?real_perimeter_region_data_visible_through_infill_postprocess_dispatch \.\.\. ok' target/test-output.log`

- **AC-4. Given** a new unit test `ir_builders_slice_ir_with_count_shape` in `dispatch_tdd.rs` exercises `ir_builders::slice_ir::with_count(3).at_z(0.2).build()`, **when** the test runs, **then** the returned `SliceIR` carries `global_layer_index == 0`, `z == 0.2`, `regions.len() == 3`, and for each `i in 0..3` the i-th region satisfies `object_id == format!("obj-{i}")`, `region_id == i as u64`, `polygons.len() == 1`, `polygons[0].contour.points.len() == 4`, `polygons[0].holes.is_empty()`, `effective_layer_height == 0.2`. | `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_count_shape --nocapture 2>&1 | tee target/test-output.log && rg -q 'test (dispatch_tdd::)?ir_builders_slice_ir_with_count_shape \.\.\. ok' target/test-output.log`

- **AC-5. Given** the new `ir_builders::slice_ir::with_ids` constructor, **when** the new unit test `ir_builders_slice_ir_with_ids_shape` calls `ir_builders::slice_ir::with_ids(&[("custom-obj", 17), ("other-obj", 99)]).at_z(0.5).build()`, **then** the returned `SliceIR` carries `regions.len() == 2`, `regions[0].object_id == "custom-obj"`, `regions[0].region_id == 17`, `regions[1].object_id == "other-obj"`, `regions[1].region_id == 99`. | `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_ids_shape --nocapture 2>&1 | tee target/test-output.log && rg -q 'test (dispatch_tdd::)?ir_builders_slice_ir_with_ids_shape \.\.\. ok' target/test-output.log`

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

## Deviations

The spec text below was authored against an idealized description of the dispatcher and the WAT test guests; the implementation, while functionally complete, surfaces a small number of spec/implementation divergences and pre-existing implementation details that the spec did not capture. These are recorded here so future readers of the spec know what is and is not load-bearing.

- [packet.spec.md §AC-2 "When/Then" + requirements.md §In Scope] — Specified: "the dispatcher returns `Ok(LayerStageCommitData::default())` and `fx.arena.slice()` / `fx.arena.infill_output_for(0)` remain at their default-empty state" | Implemented: the migrated test asserts `result.is_ok()` and `fx.arena.take_infill().is_none()` | Reason: `take_infill()` is the API the original (pre-migration) test used. `slice()` / `infill_output_for(0)` are not the canonical "empty arena" probe on `LayerArena`; the migration preserved the original observable behavior. The new "Then" text in AC-2 is rewritten to reflect this.
- [packet.spec.md §AC-3 "When/Then" + implementation-plan.md §Step 4 Postcondition] — Specified: round-trip is `point[0].{x == 3.0, y == 6.0, z == 12.0}` (aggregate: region_count, total wall_loops, total infill polygons) | Implemented: per-region `p.x == 2.0` and `p.y == 4.0` (the per-region wall count and per-region infill polygon count, respectively); `p.z` is not asserted | Reason: the WAT test guest encodes per-region counts, not aggregate counts. The original (pre-migration) test asserted the same `(2.0, 4.0, _)` values (verified via `git show HEAD:crates/slicer-runtime/tests/contract/dispatch_tdd.rs` at the matching range). The migration preserved the original observable behavior; the spec's described aggregate values did not match the guest's actual behavior.
- [requirements.md §In Scope + design.md §Code Change Surface] — Specified: `DispatchFixture::for_stage(stage_id: &str) -> DispatchFixtureBuilder` as a method on the `DispatchFixture` type | Implemented: `pub fn for_stage(stage_id: &str) -> DispatchFixtureBuilder` as a free function in the `dispatch_fixture` module | Reason: both call sites in `dispatch_tdd.rs:502` and `:2191` use the free-function form `crate::common::dispatch_fixture::for_stage(...)`. Semantically equivalent — the free function is the idiomatic Rust builder-factory shape for a free-standing builder type. No behavioral difference.
- [requirements.md §In Scope + design.md §Code Change Surface + implementation-plan.md §Step 2] — Specified: per-runner signatures `run_layer(&self, layer: &GlobalLayer)`, `run_finalization(&self, layers: &[LayerCollectionIR])`, `run_postpass(&self, gcode: &GCodeIR)`, and return types `Result<PrepassStageOutput, PrepassRunnerError>` / `Result<FinalizationStageOutput, FinalizationError>` / `Result<PostpassStageOutput, PostpassError>` | Implemented: `run_layer(&mut self, layer: &GlobalLayer) -> Result<(), slicer_ir::LayerStageError>`, `run_finalization(&self, layers: &mut Vec<LayerCollectionIR>) -> Result<slicer_ir::FinalizationOutput, slicer_ir::FinalizationError>`, `run_postpass(&self, gcode: &mut GCodeIR) -> Result<slicer_ir::PostpassOutput, slicer_ir::PostpassError>`, and `run_prepass(&self) -> Result<slicer_core::PrepassStageOutput, slicer_ir::PrepassRunnerError>` | Reason: the underlying `*StageRunner::run_stage` and `run_gcode_postprocess` traits take mutable references to the data they touch (`&mut LayerArena` is reachable through `&mut self.arena`; the trait inputs take `&mut Vec<LayerCollectionIR>` and `&mut GCodeIR.commands`). The associated output types are defined in `slicer_core` / `slicer_ir`, not as bare `*StageOutput` names. `&mut self` on `run_layer` is required because the test mutates the arena through it (this is the spec's stated intent: "Mutation of arena state happens inside `run_layer`").
- [packet.spec.md §AC-2/AC-3/AC-4/AC-5 verification commands] — Specified: `rg -q 'test <name> \.\.\. ok' target/test-output.log` (bare function name) | Implemented: `rg -q 'test (dispatch_tdd::)?<name> \.\.\. ok' target/test-output.log` (accepts the optional `dispatch_tdd::` module prefix that `cargo test` emits) | Reason: cargo test's per-binary test path is `dispatch_tdd::<fn_name>`, not the bare function name. The original spec's bare-name regex would not match the actual output line and the chained `&&` would return non-zero. The widened regex preserves the intent (prove the test ran and passed) while tolerating the module-prefix emission. AC-2's and AC-3's "When/Then" descriptions are also rewritten to match the actual assertions the migrated tests make.
- [implementation-plan.md §Step 4] — The WAT test guest's encoding is per-region rather than aggregate, so the original (pre-packet) test's round-trip assertion `p.x == 2.0`, `p.y == 4.0`, with no `p.z`, is what the migration preserves. The Step 4 Postcondition is rewritten to describe this per-region assertion. The values are derived from the input fixture (3 regions × 2 walls = per-region wall count of 2; 3 regions × 4 infill polys = per-region infill poly count of 4) but live on a per-region basis on the `point[0]`, not as aggregate counts across all regions.

The above deviations are author-recorded. None affect the packet's functional contract: all five ACs and both negative cases are green, the full 164-test contract bucket passes, the `slicer-wasm-host/tests/common/` directory is byte-identical to its pre-packet state, and `cargo clippy --workspace --all-targets -- -D warnings` is clean. The `## Deviations` section is the only place these are recorded; the ACs as rewritten above are the load-bearing acceptance contract.
