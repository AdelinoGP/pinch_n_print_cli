# Requirements: 100_dispatch-fixture-and-ir-builders

## Packet Metadata

- Grouped task IDs:
  - (none — session-derived; not a `docs/07` backlog slice)
- Backlog source: architecture review (session 2026-06-11, `/improve-codebase-architecture` Candidate 4)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (4,875 LOC, the single largest file in the workspace) hand-rolls its dispatch fixtures inline. Five near-identical wrappers (`make_compiled_module`, `make_compiled_module_with`, `make_compiled_module_with_config`, `make_compiled_module_no_wasm`, plus the `make_loaded_module` shell) assemble `TestModuleBundle`s; five further IR helpers (`make_slice_ir`, `make_slice_ir_with_ids`, `make_wall_loop`, `make_perimeter_ir`, `make_perimeter_ir_with_ids`) build `SliceIR` / `PerimeterIR` / `WallLoop` values with overlapping signatures — `make_slice_ir` even carries a dead `_polys_per_region` parameter. Every one of the 101 tests in the file re-wires `Blackboard`, `LayerArena`, the `WasmRuntimeDispatcher`, and the runner-input projection by hand.

The friction is Shallow: each helper's interface is nearly as wide as its body, region-identity preservation (the load-bearing Claim the dispatcher's bucket-by-origin logic enforces) lives in test source rather than fixture source, and adding a new test costs ~30 LOC of setup. Without a concentrated fixture, the file split planned for packet 101 would carry the same Shallow surface into eight new files instead of reducing it.

This packet introduces `DispatchFixture` (a fluent builder that owns the dispatcher + `Blackboard` + `LayerArena` and exposes per-runner `run_layer / run_prepass / run_finalization / run_postpass` methods) and `ir_builders.rs` (distinct `with_count` / `with_ids` constructors per IR type, with auto-generated walls and a `wall_loop()` escape hatch), then proves the surface covers both lifecycles by migrating two existing tests.

## In Scope

- Add `crates/slicer-runtime/tests/common/dispatch_fixture.rs` with `pub struct DispatchFixture`; constructor `DispatchFixture::for_stage(stage_id: &str) -> DispatchFixtureBuilder`; builder methods `.with_slice(SliceIR)`, `.with_perimeter(PerimeterIR)`, `.with_config(ConfigView)`, `.with_wat(&str)`, `.no_wasm()`; terminal `.build() -> DispatchFixture`. The default builder path compiles a real WAT test guest and uses an empty `ConfigView::from_map(HashMap::new())`.
- The `DispatchFixture` value owns `dispatcher: WasmRuntimeDispatcher`, `blackboard: Blackboard`, `arena: LayerArena`, and the `TestModuleBundle` produced by the builder. It exposes per-runner methods `run_layer(&self, layer: &GlobalLayer) -> Result<(), LayerStageError>`, `run_prepass(&self) -> Result<PrepassStageOutput, PrepassRunnerError>`, `run_finalization(&self, layers: &[LayerCollectionIR]) -> Result<FinalizationStageOutput, FinalizationError>`, `run_postpass(&self, gcode: &GCodeIR) -> Result<PostpassStageOutput, PostpassError>`. Mutation of arena state happens inside `run_layer` via the existing `commit_layer_outputs_for_test` path.
- Add `crates/slicer-runtime/tests/common/ir_builders.rs` exposing `pub mod slice_ir` with `with_count(n: usize) -> SliceIrBuilder` and `with_ids(ids: &[(&str, u64)]) -> SliceIrBuilder`; both end in `.at_z(z: f32).build() -> SliceIR`. Parallel `pub mod perimeter_ir` with `with_count(n: usize) -> PerimeterIrBuilder` and `with_ids(ids: &[(&str, u64)]) -> PerimeterIrBuilder`; builder accepts `.at_layer(idx: u32)`, `.walls(n: u32)`, `.walls_with(Vec<WallLoop>)`, `.infill(n: usize)`. Sibling `pub fn wall_loop() -> WallLoopBuilder` returning a small builder with `.outer() / .inner()`, `.points(n: usize)`, `.at_z(z: f32)`, `.build() -> WallLoop`.
- Register both new modules in `crates/slicer-runtime/tests/common/mod.rs` via `pub mod dispatch_fixture;` and `pub mod ir_builders;`.
- Migrate `missing_component_gracefully_skipped` (dispatch_tdd.rs line 496) to use `DispatchFixture::for_stage("Layer::Infill").no_wasm().build()`; preserve observable assertions (the runner returns the default `LayerStageCommitData`).
- Migrate `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` (dispatch_tdd.rs line 2102) to use the fluent builder + `ir_builders` modules; preserve the round-trip assertion (encoded counts in `point[0].{x, y, z}` of the emitted infill path).
- Add two new unit tests in `dispatch_tdd.rs` exercising `ir_builders::slice_ir::with_count` and `ir_builders::slice_ir::with_ids` independently of dispatch.

## Out of Scope

- The axis-aligned split of `dispatch_tdd.rs` into eight files (packet 101).
- Migration of any of the other ~99 tests in `dispatch_tdd.rs` (packet 101).
- The currently-`#[ignore]`'d paint-region test cluster (remains ignored across both packets).
- Any change to `crates/slicer-wasm-host/tests/common/` (locked invariant of this packet; see AC-N2).
- Any extraction of a shared `slicer-test-fixtures` dev-dep crate.
- A `SlicePipelineFixture` / `GcodeAnalyzer` surface for the e2e files (deferred indefinitely).
- Any change to the `make_*` helpers that survive in `dispatch_tdd.rs` (packet 101 deletes them along with the original file).
- Any change to `slicer-sdk::test_support` / `slicer-sdk::test_prelude` (governed by ADR-0004; this packet keeps the runtime-side fixture in `tests/common/`).

## Authoritative Docs

- `docs/adr/0007-compiled-module-static-live-split.md` — 175 lines incl. the 2026-06-11 amendment. Load directly. The amendment's "What future architecture reviews must not re-litigate" bullets are the locked conventions this packet implements.
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — 142 lines. Load directly. The four runner traits define the input/output shapes that `run_layer / run_prepass / run_finalization / run_postpass` wrap.
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — 72 lines. Load directly. Explains why `DispatchFixture` does NOT live in `slicer-sdk::test_support` (it imports `slicer_runtime::{Blackboard, LayerArena, CompiledModule}` and AC-N3 forbids the dep edge).
- `CLAUDE.md` §Test Discipline — read lines 51–86 only. Defines the narrow-test rule, the `target/test-output.log` tee requirement, and the prohibition on re-running tests to see truncated output.
- `docs/05_module_sdk.md` — delegate a SUMMARY of §"Test Support (slicer-sdk feature)" lines 446–596. Informational; clarifies the broader test-support landscape but does NOT govern this packet's surface.

## Acceptance Summary

- Positive cases: AC-1 (compile gate), AC-2 (no-wasm proof migration), AC-3 (real-dispatch proof migration), AC-4 (ir_builders `with_count` shape), AC-5 (ir_builders `with_ids` identity preservation) — all defined in `packet.spec.md`.
- Negative cases: AC-N1 (clippy `-D warnings` clean), AC-N2 (`slicer-wasm-host/tests/common/` unchanged) — defined in `packet.spec.md`.
- Cross-packet impact: this packet unblocks packet 101 (`dispatch-tdd-axis-aligned-split`), which requires `DispatchFixture` and `ir_builders` to migrate its ~99 remaining tests.
- Refinement (does not fit Given/When/Then): the `_polys_per_region` parameter that `make_slice_ir` carries today (dead code per the architecture exploration) does not survive into `ir_builders::slice_ir::with_count` — the parameter is absent from the new constructor's signature.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Proves AC-1; both new modules + the migrated tests + the IR builder unit tests compile together with `--all-targets` (catches dead-code in test targets) | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Proves AC-N1 | FACT pass/fail; SNIPPETS ≤ 20 lines on first warning |
| `cargo test -p slicer-runtime --test contract -- missing_component_gracefully_skipped --nocapture 2>&1 \| tee target/test-output.log` | Proves AC-2 | FACT pass/fail; if fail, SNIPPETS of the failing assertion |
| `cargo test -p slicer-runtime --test contract -- real_perimeter_region_data_visible_through_infill_postprocess_dispatch --nocapture 2>&1 \| tee target/test-output.log` | Proves AC-3 | FACT pass/fail; if fail, SNIPPETS of the round-trip assertion |
| `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_count_shape --nocapture 2>&1 \| tee target/test-output.log` | Proves AC-4 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- ir_builders_slice_ir_with_ids_shape --nocapture 2>&1 \| tee target/test-output.log` | Proves AC-5 | FACT pass/fail |
| `test -z "$(git status --porcelain crates/slicer-wasm-host/tests/common/)"` | Proves AC-N2 | FACT pass/fail (exit 0 if directory clean) |

All commands above produce small parseable output and are delegation-friendly. None invokes `cargo test --workspace`; the packet acceptance ceremony does not require a workspace-wide test run.

## Step Completion Expectations

- Steps 1 (ir_builders) and 2 (dispatch_fixture) build the test-support surface. Step 2 depends on `pub mod ir_builders;` being added to `tests/common/mod.rs` in Step 1 — both register their `pub mod` declarations in the same `mod.rs` so the order is enforced by the file-edit sequence rather than by compile dependency.
- Steps 3 and 4 (the two test migrations) MUST NOT regress any other test in `dispatch_tdd.rs`. Each migration is local to its test function body; the surrounding `make_*` helpers stay in place for packet 101 to remove.
- The AC-N2 invariant (`slicer-wasm-host/tests/common/` unchanged) is a packet-level cross-step expectation: no step may edit any file under that directory.

## Context Discipline Notes

- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` is 4,875 lines. Read only the surgical ranges: lines 179–255 (the `make_*` family the fixture replaces), lines 496–550 (the `missing_component_gracefully_skipped` test body and its imports), and lines 2102–2220 (the `real_perimeter_region_...` test body). Do NOT load the file in full.
- The `tests/common/mod.rs` baseline is 456 lines; range-read lines 350–456 to see `TestModuleBundle` and the existing `run_layer_and_commit` family which `DispatchFixture`'s `run_layer` delegates to.
- Sub-agent return-format hints for the heaviest dispatches:
  - "Run AC-2 / AC-3 / AC-4 / AC-5 verification commands": dispatch each one separately as `FACT pass/fail`; on fail, return ≤ 20 lines around the assertion line from `target/test-output.log`.
  - "Confirm the dead `_polys_per_region` parameter is absent from `ir_builders::slice_ir::with_count`": `FACT yes/no` + the constructor's signature line.
