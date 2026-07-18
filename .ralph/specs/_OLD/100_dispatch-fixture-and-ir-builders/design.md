# Design: 100_dispatch-fixture-and-ir-builders

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/tests/common/` — the canonical home of runtime-side test scaffolding (today: `TestModuleBundle`, `commit_hec_for_test`, `run_layer_and_commit`, the four `*_input` projectors, plus mesh/geometry helpers). The new `dispatch_fixture.rs` and `ir_builders.rs` modules sit alongside `mod.rs` and absorb the `make_compiled_module_*` / `make_slice_ir*` / `make_perimeter_ir*` / `make_wall_loop` families that today live inline in `tests/contract/dispatch_tdd.rs`.
- Neighboring tests or fixtures: the existing `run_layer_and_commit` (placeholder pool) and `run_layer_and_commit_with_bundle` (real pool) free functions in `tests/common/mod.rs` become the implementation backbone of `DispatchFixture::run_layer` — the fixture calls them internally rather than replacing them, so existing callers in other test files keep working.
- OrcaSlicer comparison surface: not applicable (no parity behavior).

## Architecture Constraints

- The four runner traits (`LayerStageRunner`, `PrepassStageRunner`, `FinalizationStageRunner`, `PostpassStageRunner`) defined in `slicer-wasm-host` per ADR-0005 are the dispatch contract `DispatchFixture` wraps. Per-runner methods MUST match each trait's input shape: `run_layer(&GlobalLayer)`, `run_prepass()`, `run_finalization(&[LayerCollectionIR])`, `run_postpass(&GCodeIR)`. **No generic `run::<R: StageRunner>` method** and **no type-state `DispatchFixture<LayerStage>`** — both were costed and rejected in the ADR-0007 amendment.
- `DispatchFixture` lives in `crates/slicer-runtime/tests/common/`, NOT in `slicer-sdk::test_support`. The reason is structural: the fixture imports `slicer_runtime::{Blackboard, LayerArena, CompiledModule}` and the `slicer-wasm-host::WasmRuntimeDispatcher`. ADR-0004 keeps `slicer-sdk` dependency-light and ADR-0007's AC-N3 forbids a `slicer-wasm-host[dev] → slicer-runtime` back-edge — so a shared `slicer-sdk`-hosted fixture would either re-import the forbidden types or re-implement them. The runtime-side `tests/common/` location is the only one that satisfies both constraints.
- The parallel `crates/slicer-wasm-host/tests/common/` set (131 LOC of pure mesh/geometry helpers, no runtime-typed scaffolding) MUST remain unchanged. AC-N2 enforces this mechanically.
- `ir_builders.rs` uses **two distinct constructors per IR type** (`with_count` for cardinality, `with_ids` for identity preservation). The ADR-0007 amendment explicitly forbids collapsing them into a single dual-purpose builder; merging would force every test to specify `None`-or-Vec for region IDs.
- The wasm-staleness snippet does NOT apply: this packet edits no path under `wit/`, `slicer-macros/`, `slicer-sdk/`, `slicer-ir/`, `slicer-schema/`, `modules/core-modules/`, or `slicer-runtime/test-guests/` source. The pre-built test-guest `.wasm` artifacts are loaded by tests but not modified.
- The coord-system snippet does NOT apply: `ir_builders` constructs synthetic `Point2` values directly in scaled units (e.g., `Point2 { x: 0, y: 0 }`, `Point2 { x: 10_000, y: 0 }` for a 1mm square in the project's 100nm-per-unit convention); no mm↔unit conversion happens.

## Code Change Surface

- Selected approach: fluent builder with named-method overrides (default = real WAT-compiled test guest + empty `ConfigView`; `.no_wasm()` switches to the MissingComponent lifecycle; `.with_config(ConfigView)` replaces the default; `.with_wat(&str)` replaces the default guest with a custom WAT). The fixture value owns dispatcher + `Blackboard` + `LayerArena` so per-runner methods can mutate arena state in place. The four `run_*` methods delegate to the existing `run_layer_and_commit*` family for the Layer path and to direct trait calls (`PrepassStageRunner::run_stage`, etc.) for the other three.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - NEW: `crates/slicer-runtime/tests/common/dispatch_fixture.rs` — `pub struct DispatchFixture`, `pub struct DispatchFixtureBuilder`, `impl DispatchFixtureBuilder { fn for_stage (free fn in the module, not a method on DispatchFixture), fn with_slice, fn with_perimeter, fn with_config, fn with_wat, fn no_wasm, fn build }`, `impl DispatchFixture { fn run_layer(&mut self, ...), fn run_prepass(&self), fn run_finalization(&self, layers: &mut Vec<LayerCollectionIR>), fn run_postpass(&self, gcode: &mut GCodeIR) }` (per-runner signatures take `&mut` where the underlying `*StageRunner` trait requires it; the associated output types are `slicer_core::PrepassStageOutput`, `slicer_ir::FinalizationOutput`, `slicer_ir::PostpassOutput`).
  - NEW: `crates/slicer-runtime/tests/common/ir_builders.rs` — `pub mod slice_ir { pub fn with_count, pub fn with_ids; pub struct SliceIrBuilder; impl { fn at_z, fn build } }`, parallel `pub mod perimeter_ir` with `pub struct PerimeterIrBuilder` and `impl { fn at_layer, fn walls, fn walls_with, fn infill, fn build }`, sibling `pub fn wall_loop() -> WallLoopBuilder` with `impl { fn outer, fn inner, fn points, fn at_z, fn build }`.
  - EDIT: `crates/slicer-runtime/tests/common/mod.rs` — add `pub mod dispatch_fixture;` and `pub mod ir_builders;` declarations alongside the existing `pub mod model_cache; pub mod seed; pub mod slicer_cache; pub mod wasm_cache;` block at lines 10–13.
  - EDIT: `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — replace the body of `missing_component_gracefully_skipped` (line 496) with the `DispatchFixture::for_stage("Layer::Infill").no_wasm().build()` form; replace the body of `real_perimeter_region_data_visible_through_infill_postprocess_dispatch` (line 2102) with the fluent-builder form; add two new tests `ir_builders_slice_ir_with_count_shape` and `ir_builders_slice_ir_with_ids_shape` that exercise the builders without dispatch.
- Rejected alternatives:
  - **Uniform generic `fx.run::<R: StageRunner>(inputs)`** — would force a per-runner inputs trait everywhere; turbofish at every call site; rejected in ADR-0007 amendment.
  - **Type-state `DispatchFixture<LayerStage>`** — generic parameter spreads to every helper signature in packet 101's eight axis files; rejected in ADR-0007 amendment.
  - **Single dual-purpose `slice_ir(...)` builder** combining count and ids — forces every test to specify `None`-or-Vec for IDs; rejected in ADR-0007 amendment.
  - **Hosting the fixture in `slicer-sdk::test_support`** behind the `test` feature — re-imports forbidden `slicer-runtime` types; rejected per the ADR-0007 amendment / ADR-0004 dep-direction reasoning.
  - **Extracting a shared `slicer-test-fixtures` dev-dep crate** in the same packet — would also subsume the 120 LOC mesh-helper duplication and double the packet's blast radius; deferred to a separate decision.

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/common/dispatch_fixture.rs` — role: new fluent builder + per-runner methods; expected change: full new file, ≈ 220 LOC.
- `crates/slicer-runtime/tests/common/ir_builders.rs` — role: new IR builder family; expected change: full new file, ≈ 180 LOC.
- `crates/slicer-runtime/tests/common/mod.rs` — role: register the two new modules; expected change: two `pub mod` lines added at lines 10–13.
- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — role: migrate two proof tests + add two IR-builder unit tests; expected change: ≈ 4 edits totalling < 100 LOC delta. The surrounding `make_*` helpers stay in place (packet 101 removes them).

Four primary files. The fourth (`dispatch_tdd.rs`) is unavoidable because the AC verifications require the proof tests to live in the existing contract bucket; splitting it out would create a phantom test file that packet 101 would immediately need to fold back.

## Read-Only Context

- `docs/adr/0007-compiled-module-static-live-split.md` — read the full 175 lines, especially the amendment section starting at "## Amendment (packet TBD): dispatch fixture conventions".
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — read lines 42–77 (Decision: the four runner traits, `*StageInput` borrow structs, IR-typed outputs) and lines 136–142 (Test placement).
- `docs/adr/0004-test-support-lives-in-slicer-sdk.md` — read full 72 lines (dep-direction reasoning).
- `crates/slicer-runtime/tests/common/mod.rs` — read full 456 lines; line 350+ is the `TestModuleBundle` / `run_layer_and_commit*` cluster that `DispatchFixture` delegates to.
- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` — read ONLY lines 179–255 (`make_*` helpers), 496–550 (the no-wasm proof test), 2055–2220 (the real-dispatch proof test). Do NOT load the file in full.
- `crates/slicer-wasm-host/tests/common/mod.rs` — read full 131 lines; useful for understanding what is NOT being added here (the parallel set has no dispatch fixture, by design).
- `CLAUDE.md` §Test Discipline — read lines 51–86 only.
- `slicer_ir` struct defs (`SliceIR`, `PerimeterIR`, `WallLoop`, `Point2`, `Point3WithWidth`, `WidthProfile`, `WallFeatureFlags`) — delegate a LOCATIONS or SNIPPETS dispatch; do not read the full crate.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate any parity checks (none expected for this packet).
- `target/`, `Cargo.lock`, generated code — never load.
- Any vendored deps — never load.
- `crates/slicer-wasm-host/tests/common/**` — must not be edited (AC-N2 invariant). Reading is allowed for context but the directory's contents must end the packet byte-identical to its start state.
- `crates/slicer-wasm-host/src/**` — the dispatcher source is read-only context if needed; delegate LOCATIONS or SNIPPETS dispatches for trait signatures.
- `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` outside the surgical ranges (lines 179–255, 496–550, 2055–2220, and the bottom of the file where the new IR-builder unit tests will be appended).

## Expected Sub-Agent Dispatches

- "Run `cargo check --workspace --all-targets`; return FACT pass/fail. On fail, return SNIPPETS ≤ 20 lines around the first `error:` line." — Step 1 verification (AC-1).
- "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail. On fail, return SNIPPETS ≤ 20 lines around the first warning." — final gate (AC-N1).
- "Find all callers of `make_compiled_module_no_wasm` in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`; return LOCATIONS with line numbers." — Step 3 scoping.
- "Find all callers of `make_compiled_module_with` in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs`; return LOCATIONS with line numbers." — Step 4 scoping.
- "Look up the `SliceIR`, `SlicedRegion`, `ExPolygon`, `Polygon` struct definitions in `slicer-ir`; return SNIPPETS ≤ 30 lines for each." — Step 1 (`ir_builders.rs`) construction.
- "Look up the `PerimeterIR`, `PerimeterRegion`, `WallLoop`, `Point3WithWidth`, `WidthProfile`, `WallFeatureFlags`, `ExtrusionPath3D` definitions; return SNIPPETS ≤ 30 lines for each." — Step 1.
- "Look up `WasmRuntimeDispatcher::new`, the `LayerStageRunner::run_stage` trait signature, and the `run_layer_and_commit_with_bundle` function body; return SNIPPETS ≤ 30 lines each." — Step 2.
- "Run `cargo test -p slicer-runtime --test contract -- <test_name> --nocapture 2>&1 | tee target/test-output.log`; return FACT pass/fail. On fail, return SNIPPETS ≤ 20 lines around the failing assertion." — Steps 3, 4, and the two IR-builder unit-test verifications (AC-2, AC-3, AC-4, AC-5).
- "Run `git status --porcelain crates/slicer-wasm-host/tests/common/`; return FACT (empty/non-empty)." — Final gate (AC-N2).

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

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — `dispatch_fixture.rs` construction; the per-runner method bodies require care)
- Highest-risk dispatch: the `slicer-ir` struct-definition SNIPPETS dispatch in Step 1. If the dispatcher returns the whole `slicer-ir/src/slice_ir.rs` file (often > 300 lines) instead of the targeted struct snippets, context budget can blow. Required return format: `SNIPPETS ≤ 30 lines per struct, one struct per dispatch`. Re-dispatch with tighter scope if the first return is oversize.

## Open Questions

`None.`
