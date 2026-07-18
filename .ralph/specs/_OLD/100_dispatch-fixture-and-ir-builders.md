---
status: implemented
packet: 100_dispatch-fixture-and-ir-builders
task_ids: []
---

# 100_dispatch-fixture-and-ir-builders

## Goal

Concentrate the hand-rolled dispatch test scaffolding under `crates/slicer-runtime/tests/common/` into two new modules — a fluent `DispatchFixture` builder that owns the dispatcher + `Blackboard` + `LayerArena` and exposes four per-runner `run_*` methods, and an `ir_builders` module with distinct `with_count` / `with_ids` constructors per IR type — then prove the surface covers both lifecycles by migrating two existing tests in `dispatch_tdd.rs`.

## Problem Statement

`crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (4,875 LOC, the single largest file in the workspace) hand-rolls its dispatch fixtures inline. Five near-identical wrappers (`make_compiled_module`, `make_compiled_module_with`, `make_compiled_module_with_config`, `make_compiled_module_no_wasm`, plus the `make_loaded_module` shell) assemble `TestModuleBundle`s; five further IR helpers (`make_slice_ir`, `make_slice_ir_with_ids`, `make_wall_loop`, `make_perimeter_ir`, `make_perimeter_ir_with_ids`) build `SliceIR` / `PerimeterIR` / `WallLoop` values with overlapping signatures — `make_slice_ir` even carries a dead `_polys_per_region` parameter. Every one of the 101 tests in the file re-wires `Blackboard`, `LayerArena`, the `WasmRuntimeDispatcher`, and the runner-input projection by hand.

The friction is Shallow: each helper's interface is nearly as wide as its body, region-identity preservation (the load-bearing Claim the dispatcher's bucket-by-origin logic enforces) lives in test source rather than fixture source, and adding a new test costs ~30 LOC of setup. Without a concentrated fixture, the file split planned for packet 101 would carry the same Shallow surface into eight new files instead of reducing it.

This packet introduces `DispatchFixture` (a fluent builder that owns the dispatcher + `Blackboard` + `LayerArena` and exposes per-runner `run_layer / run_prepass / run_finalization / run_postpass` methods) and `ir_builders.rs` (distinct `with_count` / `with_ids` constructors per IR type, with auto-generated walls and a `wall_loop()` escape hatch), then proves the surface covers both lifecycles by migrating two existing tests.

## Architecture Constraints

- The four runner traits (`LayerStageRunner`, `PrepassStageRunner`, `FinalizationStageRunner`, `PostpassStageRunner`) defined in `slicer-wasm-host` per ADR-0005 are the dispatch contract `DispatchFixture` wraps. Per-runner methods MUST match each trait's input shape: `run_layer(&GlobalLayer)`, `run_prepass()`, `run_finalization(&[LayerCollectionIR])`, `run_postpass(&GCodeIR)`. **No generic `run::<R: StageRunner>` method** and **no type-state `DispatchFixture<LayerStage>`** — both were costed and rejected in the ADR-0007 amendment.
- `DispatchFixture` lives in `crates/slicer-runtime/tests/common/`, NOT in `slicer-sdk::test_support`. The reason is structural: the fixture imports `slicer_runtime::{Blackboard, LayerArena, CompiledModule}` and the `slicer-wasm-host::WasmRuntimeDispatcher`. ADR-0004 keeps `slicer-sdk` dependency-light and ADR-0007's AC-N3 forbids a `slicer-wasm-host[dev] → slicer-runtime` back-edge — so a shared `slicer-sdk`-hosted fixture would either re-import the forbidden types or re-implement them. The runtime-side `tests/common/` location is the only one that satisfies both constraints.
- The parallel `crates/slicer-wasm-host/tests/common/` set (131 LOC of pure mesh/geometry helpers, no runtime-typed scaffolding) MUST remain unchanged. AC-N2 enforces this mechanically.
- `ir_builders.rs` uses **two distinct constructors per IR type** (`with_count` for cardinality, `with_ids` for identity preservation). The ADR-0007 amendment explicitly forbids collapsing them into a single dual-purpose builder; merging would force every test to specify `None`-or-Vec for region IDs.
- The wasm-staleness snippet does NOT apply: this packet edits no path under `wit/`, `slicer-macros/`, `slicer-sdk/`, `slicer-ir/`, `slicer-schema/`, `modules/core-modules/`, or `slicer-runtime/test-guests/` source. The pre-built test-guest `.wasm` artifacts are loaded by tests but not modified.
- The coord-system snippet does NOT apply: `ir_builders` constructs synthetic `Point2` values directly in scaled units (e.g., `Point2 { x: 0, y: 0 }`, `Point2 { x: 10_000, y: 0 }` for a 1mm square in the project's 100nm-per-unit convention); no mm↔unit conversion happens.

## Data and Contract Notes

- IR or manifest contracts touched: none. The `ir_builders` module produces values that satisfy the existing IR struct contracts; no IR field is renamed, added, or removed.
- WIT boundary considerations: none. The runner traits' IR-typed boundary (ADR-0005) is unchanged; `DispatchFixture` consumes the boundary, it does not move it.
- Determinism or scheduler constraints: `ir_builders::slice_ir::with_count(n)` produces deterministically-named synthetic IDs (`format!("obj-{i}")`, `region_id = i as u64`) matching today's `make_slice_ir` exactly. Tests that previously relied on these synthetic IDs continue to work bit-identically.

## Locked Assumptions and Invariants

- `crates/slicer-wasm-host/tests/common/` is byte-identical at packet end vs packet start (AC-N2).
- `DispatchFixture` owns dispatcher + `Blackboard` + `LayerArena` internally; no caller-facing handle to those pieces survives outside `fx.dispatcher`, `fx.blackboard`, `fx.arena` accessors.
- Per-runner methods only; no generic `run::<R: StageRunner>` method; no type-state `DispatchFixture<LayerStage>`.
- `ir_builders` uses two distinct constructors per IR type (`with_count`, `with_ids`); no single dual-purpose builder.
- The dead `_polys_per_region` parameter on the legacy `make_slice_ir` does NOT survive into the new builders.
- Default builder path = real WAT-compiled test guest + empty `ConfigView`; `.no_wasm()` is the explicit MissingComponent opt-out.

## Risks and Tradeoffs

- The four primary files exceed the "≤ 3" target. Justified: the fourth (`dispatch_tdd.rs`) is unavoidable because AC verifications require the proof tests to live in the existing contract bucket; a separate location would create a phantom file that packet 101 has to immediately fold back.
- Two test files temporarily contain BOTH the new fixture-based form (the two migrated tests + the two new unit tests) AND the legacy `make_*` helpers (untouched, used by the other ~99 tests). Packet 101 cleans this up by deleting the legacy helpers along with the original file.
- The fixture default of "real WAT-compiled test guest" implies that `cargo xtask build-guests` has been run before the proof tests are exercised. This is an existing precondition of the contract test bucket (the legacy `real_perimeter_...` test already relies on it via `load_test_guest`); the fixture does not weaken the guarantee.
