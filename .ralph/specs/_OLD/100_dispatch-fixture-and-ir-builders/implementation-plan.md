# Implementation Plan: 100_dispatch-fixture-and-ir-builders

## Execution Rules

- One atomic step at a time.
- This packet is session-derived; no `docs/07` `TASK-###` ids apply. The "Task IDs" field in each step references the packet itself.
- TDD first, then implementation, then the narrowest falsifying validation. Steps 1 and 2 write a failing test first, then make it pass.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Add `ir_builders.rs` (TDD)

- Task IDs:
  - (packet-scope: 100_dispatch-fixture-and-ir-builders)
- Objective: introduce `crates/slicer-runtime/tests/common/ir_builders.rs` with `slice_ir::with_count / with_ids`, `perimeter_ir::with_count / with_ids` (with `at_layer / walls / walls_with / infill` methods), and the sibling `wall_loop()` builder.
- Precondition: `tests/common/mod.rs` does not yet declare `pub mod ir_builders;`; the file does not exist.
- Postcondition: `tests/common/ir_builders.rs` exists, is registered in `tests/common/mod.rs`, and the two new unit tests `ir_builders_slice_ir_with_count_shape` and `ir_builders_slice_ir_with_ids_shape` pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/common/mod.rs` — full file (456 lines, manageable)
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — lines `179-255` (the `make_*` family the builders subsume) and lines `1955-2098` (the `make_slice_ir` / `make_wall_loop` / `make_perimeter_ir` bodies that establish the today-shape)
  - `crates/slicer-ir/src/slice_ir.rs` — delegate SNIPPETS ≤ 30 lines per struct
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/common/ir_builders.rs` (NEW)
  - `crates/slicer-runtime/tests/common/mod.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (the two new unit tests appended at the bottom of the file)
- Files explicitly out-of-bounds for this step:
  - Any path under `crates/slicer-wasm-host/`
  - Any path under `OrcaSlicerDocumented/`
  - `dispatch_tdd.rs` body lines outside 179–255, 1955–2098, and the bottom-of-file append point
- Expected sub-agent dispatches:
  - "Return `SNIPPETS` ≤ 30 lines for each of: `SliceIR`, `SlicedRegion`, `ExPolygon`, `Polygon`, `PerimeterIR`, `PerimeterRegion`, `WallLoop`, `Point3WithWidth`, `WidthProfile`, `WallFeatureFlags`, `ExtrusionPath3D` from the `slicer-ir` crate; one dispatch per struct group; never the whole file."
  - "Run `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_count_shape ir_builders_slice_ir_with_ids_shape --nocapture 2>&1 | tee target/test-output.log`; return `FACT pass/fail`; on fail return `SNIPPETS` ≤ 20 lines around the failing assertion."
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0007-compiled-module-static-live-split.md` — read lines 113+ (the amendment section) to confirm the `with_count` / `with_ids` split.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_count_shape ir_builders_slice_ir_with_ids_shape --nocapture 2>&1 | tee target/test-output.log` — dispatch as `FACT pass/fail`.
- Exit condition: both new unit tests pass; `cargo check --workspace --all-targets` returns exit 0.

### Step 2: Add `dispatch_fixture.rs`

- Task IDs:
  - (packet-scope: 100_dispatch-fixture-and-ir-builders)
- Objective: introduce `crates/slicer-runtime/tests/common/dispatch_fixture.rs` with the `DispatchFixture` value (owning `dispatcher`, `blackboard`, `arena`, `bundle`) and the `DispatchFixtureBuilder` chain (`for_stage` (a free function, not a `DispatchFixture` method) → `with_slice` / `with_perimeter` / `with_config` / `with_wat` / `no_wasm` → `build`). Implement per-runner methods `run_layer(&mut self, ...) -> Result<(), slicer_ir::LayerStageError>` (delegates to `run_layer_and_commit_with_bundle`; `&mut self` is required for arena mutation), `run_prepass(&self) -> Result<slicer_core::PrepassStageOutput, slicer_ir::PrepassRunnerError>`, `run_finalization(&self, layers: &mut Vec<LayerCollectionIR>) -> Result<slicer_ir::FinalizationOutput, slicer_ir::FinalizationError>`, `run_postpass(&self, gcode: &mut GCodeIR) -> Result<slicer_ir::PostpassOutput, slicer_ir::PostpassError>` (uses `PostpassStageRunner::run_gcode_postprocess`) by delegating to the existing `run_layer_and_commit_with_bundle` family and the four `*StageRunner::run_stage` trait calls.
- Precondition: Step 1's exit condition is met.
- Postcondition: `tests/common/dispatch_fixture.rs` exists and compiles; `tests/common/mod.rs` declares `pub mod dispatch_fixture;`; `cargo check --workspace --all-targets` returns exit 0 even though no test uses the fixture yet.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/common/mod.rs` — lines `350-456` (the existing `TestModuleBundle`, `run_layer_and_commit`, `run_layer_and_commit_with_bundle` cluster)
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — lines `179-255` only (the `make_*` family the fixture subsumes; needed to compare default-config and no-wasm shapes)
  - `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — full 142 lines
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/common/dispatch_fixture.rs` (NEW)
  - `crates/slicer-runtime/tests/common/mod.rs`
- Files explicitly out-of-bounds for this step:
  - Any path under `crates/slicer-wasm-host/tests/common/`
  - `dispatch_tdd.rs` body lines outside 179–255
  - `slicer-wasm-host/src/*` (delegate trait-signature lookups; do not browse)
- Expected sub-agent dispatches:
  - "Return `SNIPPETS` ≤ 30 lines for `LayerStageRunner::run_stage`, `PrepassStageRunner::run_stage`, `FinalizationStageRunner::run_stage`, `PostpassStageRunner::run_stage` trait signatures from `slicer-wasm-host`; one dispatch per trait."
  - "Return `SNIPPETS` ≤ 30 lines for `WasmRuntimeDispatcher::new` and the `wasm_cache::shared_engine` / `load_test_guest` helpers used by today's proof tests."
  - "Run `cargo check --workspace --all-targets`; return `FACT pass/fail`."
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — full file (the four trait signatures define the per-runner method shapes).
  - `docs/adr/0007-compiled-module-static-live-split.md` — read the amendment (lines 113+) to confirm the "fixture owns dispatcher + Blackboard + arena" invariant.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets` — dispatch as `FACT pass/fail`.
- Exit condition: file compiles cleanly; `pub mod dispatch_fixture;` line present in `tests/common/mod.rs`; `cargo check --workspace --all-targets` returns exit 0.

### Step 3: Migrate `missing_component_gracefully_skipped` (no-wasm proof)

- Task IDs:
  - (packet-scope: 100_dispatch-fixture-and-ir-builders)
- Objective: replace the body of `missing_component_gracefully_skipped` (currently at `dispatch_tdd.rs:496`) with the fluent-builder form, preserving observable assertions.
- Precondition: Steps 1 and 2 exit conditions met.
- Postcondition: the test body uses `crate::common::dispatch_fixture::for_stage("Layer::Infill").no_wasm().build()` and `fx.run_layer(&layer)?`; the observable assertions are unchanged from the pre-migration form: `result.is_ok()` and `fx.arena.take_infill().is_none()` (the arena is in its default-empty state, no `solid_infill` committed, no error variants raised, no panics); the test passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — lines `496-550` only
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - Every line of `dispatch_tdd.rs` outside 496–550
  - Any file under `crates/slicer-wasm-host/`
- Expected sub-agent dispatches:
  - "Return `SNIPPETS` of the full body of `missing_component_gracefully_skipped` from `dispatch_tdd.rs:496-550`."
  - "Run `cargo test -p slicer-runtime --test contract -- missing_component_gracefully_skipped --nocapture 2>&1 | tee target/test-output.log`; return `FACT pass/fail`; on fail, return `SNIPPETS` ≤ 20 lines around the failing assertion."
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0007-compiled-module-static-live-split.md` — amendment section bullet about the `.no_wasm()` override.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- missing_component_gracefully_skipped --nocapture 2>&1 | tee target/test-output.log` — dispatch as `FACT pass/fail`.
- Exit condition: test passes; the body of the test no longer references `make_compiled_module_no_wasm` (the helper remains in the file for packet 101 to remove, but is no longer called from this test).

### Step 4: Migrate `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` (real-dispatch proof)

- Task IDs:
  - (packet-scope: 100_dispatch-fixture-and-ir-builders)
- Objective: replace the body of `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` (currently at `dispatch_tdd.rs:2102`) with the fluent-builder form using both `DispatchFixture` and `ir_builders`, preserving the round-trip assertion.
- Precondition: Steps 1–3 exit conditions met.
- Postcondition: the test body uses `DispatchFixture::for_stage("Layer::InfillPostProcess").with_slice(ir_builders::slice_ir::with_count(3).at_z(0.4).build()).with_perimeter(ir_builders::perimeter_ir::with_count(3).at_layer(2).walls(2).infill(4).build()).build()` and `fx.run_layer(&layer)?`; the round-trip assertion is unchanged from the pre-migration form: per-region `p.x == 2.0` (the per-region wall count) and `p.y == 4.0` (the per-region infill polygon count); `r.object_id == format!("obj-{i}")` and `r.region_id == i as u64` are preserved; the WAT test guest encodes per-region counts, not aggregate counts, so the assertion is per-region rather than the aggregate `(3, 6, 12)`; the test passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — lines `2055-2220` only (the test body and any closely-coupled helper just above it)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - Every line of `dispatch_tdd.rs` outside 2055–2220
  - Any file under `crates/slicer-wasm-host/`
- Expected sub-agent dispatches:
  - "Return `SNIPPETS` of the full body of `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` from `dispatch_tdd.rs:2102-2220`."
  - "Run `cargo test -p slicer-runtime --test contract -- real_perimeter_region_data_visible_through_infill_postprocess_dispatch --nocapture 2>&1 | tee target/test-output.log`; return `FACT pass/fail`; on fail, return `SNIPPETS` ≤ 20 lines around the failing assertion."
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0007-compiled-module-static-live-split.md` — amendment section bullets about the fluent builder + IR builder distinction.
  - `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — lines 42–77 (the input shape of `LayerStageRunner::run_stage`).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- real_perimeter_region_data_visible_through_infill_postprocess_dispatch --nocapture 2>&1 | tee target/test-output.log` — dispatch as `FACT pass/fail`; on fail, return ≤ 20 lines around the assertion.
- Exit condition: test passes; the body of the test no longer references `make_compiled_module_with`, `make_slice_ir`, `make_perimeter_ir`, or `make_wall_loop` (helpers remain in the file for packet 101 to remove).

### Step 5: Final gate

- Task IDs:
  - (packet-scope: 100_dispatch-fixture-and-ir-builders)
- Objective: run the packet-level gate commands and confirm AC-N2 (wasm-host common/ untouched).
- Precondition: Steps 1–4 exit conditions met.
- Postcondition: all six gate commands return exit 0.
- Files allowed to read: none (verification-only step).
- Files allowed to edit: none.
- Files explicitly out-of-bounds for this step: all source files.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return `FACT pass/fail`."
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return `FACT pass/fail`; on fail return `SNIPPETS` ≤ 20 lines around the first warning."
  - "Run `test -z \"$(git status --porcelain crates/slicer-wasm-host/tests/common/)\"`; return `FACT pass/fail`. Pass = empty status."
- Context cost: `S`
- Authoritative docs: none (verification-only).
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `test -z "$(git status --porcelain crates/slicer-wasm-host/tests/common/)"`
- Exit condition: all three commands return exit 0.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | new file + 2 new unit tests; some `slicer-ir` struct lookup |
| Step 2 | M | new file with 4 runner method bodies; trait-signature dispatches |
| Step 3 | S | single-test body replacement |
| Step 4 | M | single-test body replacement, but the test exercises the full real-dispatch round-trip |
| Step 5 | S | pure verification |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green: each of AC-1 through AC-5 and AC-N1, AC-N2 has its verification command returned `pass` by a sub-agent.
- `docs/07_implementation_status.md` not modified (this packet is session-derived and not registered in the docs/07 backlog).
- No prior packet status to reconcile (no supersession).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm `FACT pass`.
- Confirm the three packet-level gate commands (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and the four-named-test `cargo test` invocation) are green.
- Record any remaining packet-local risk explicitly. None expected.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
