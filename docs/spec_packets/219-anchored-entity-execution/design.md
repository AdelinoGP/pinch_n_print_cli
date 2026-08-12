# Design: anchored-entity-execution

## Controlling Code Paths

- Primary code path: `crates/slicer-ir/src/slice_ir.rs:1013-1048, 2323-2366`, `crates/slicer-scheduler/src/execution_plan.rs:15-46`, and `crates/slicer-runtime/src/layer_executor.rs:1180-1258, 1959-1989`.
- Neighboring tests/fixtures: `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs:275-320`, `crates/slicer-scheduler/tests/unit/execution_plan_tdd.rs:1-80`, `crates/slicer-runtime/tests/integration/runtime_wiring_tdd.rs:209-220`, and `crates/slicer-runtime/tests/executor/finalization_builder_permute.rs:98-247`.
- OrcaSlicer comparison: no direct source read; implementation workers must use delegated `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` and `OrcaSlicerDocumented/src/libslic3r/Print.cpp` location searches only if parity details are needed.

## Architecture Constraints

- Preserve `GlobalLayer.index: u32` and existing signed raft-prefix representation in support-plan entries; do not make `GlobalLayer` negative.
- Capability closure must be computed from declared input/output capabilities, not an event-kind table.
- `layer-parallel-safe` remains the manifest hint and applies to anchored invocations as well as ordinary model layers.
- Planar event validation uses the repository coordinate tolerance; Z-spanning validation replaces the old model-layer envelope assumption only for declared spans.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface

- Selected approach: introduce generic anchored contracts beside existing layer IR, adapt the runtime worker and stage commit to return ordered event collections, then thread the same event abstraction through SDK/WIT only where a guest must produce or optimize anchored work.
- Exact functions, traits, manifests, tests, and fixtures: `GlobalLayer`/`LayerCollectionIR` construction and validation; `STAGE_ORDER`; per-layer execution/commit; `Layer::PathOptimization` projection and `set_entity_order`; cooling/time accounting hook; scheduler manifest parsing; WIT stage records; scheduler/runtime integration aggregators.
- Rejected alternatives: a cross-layer scheduler (violates the approved global-layer barrier); encoding Z-spanning paths as synthetic model slices (breaks atomicity and slice semantics); treating raft as anchored work (violates ADR-0009).

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - add anchored IR and ordered event collection contract; update affected constructors.
- `crates/slicer-scheduler/src/execution_plan.rs` - capability closure and scheduler-facing event planning.
- `crates/slicer-runtime/src/layer_executor.rs` - worker event assembly, validation, optimization/accounting hooks.
- `crates/slicer-sdk/src/*`, `crates/slicer-schema/wit/*`, `crates/slicer-macros/src/lib.rs` - justified boundary migration after the IR seam is fixed.
- `crates/slicer-scheduler/tests/integration/*`, `crates/slicer-runtime/tests/integration/*`, aggregating `main.rs` files - regression drivers.

## Read-Only Context

- `crates/slicer-ir/src/stage_io.rs` lines 257-344 and 474-498 - blackboard and staged commit error shapes.
- `crates/slicer-schema/wit/deps/ir-types.wit` lines 1-87 - current region and path boundary identifiers.
- `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit` lines 1-80 - current optimization boundary.
- `docs/adr/0059-support-families-and-anchored-entities.md`, `docs/adr/0009-raft-as-layer-infill-role.md`, and `docs/adr/0020-layer-stage-commit-as-per-stage-enum.md` - delegated summaries only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `docs/07_implementation_status.md` - delegate a bounded location check; do not edit in this packet.
- `docs/spec_packets/213-support-planner-defect-fix/` - never touch.

## Expected Sub-Agent Dispatches

- Question: enumerate all `LayerCollectionIR` and new anchored struct literals; scope: `crates/**/*.rs`; return: `LOCATIONS`; purpose: blast-radius inventory before field edits.
- Question: enumerate scheduler/runtime cooling and time-accounting functions; scope: `crates/slicer-runtime/src`, `crates/slicer-scheduler/src`; return: `LOCATIONS`; purpose: select the per-event hook without guessing.
- Question: summarize ADR-0059, ADR-0009, and ADR-0020 clauses governing anchored entities, raft, and staged commits; scope: those ADR files; return: `SUMMARY`; purpose: conformance check.
- Question: locate Orca documented scheduling/event references; scope: `OrcaSlicerDocumented/`; return: `LOCATIONS`; purpose: parity grounding without source loading.

## Data and Contract Notes

- IR/manifest contracts: current `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` is live at `crates/slicer-ir/src/slice_ir.rs:315-319`; compute any required bump from that constant at activation rather than hard-coding a future version. Existing `SupportPlanIR` is legacy and is consumed by TASK-331.
- WIT boundary: current `layer-support` is `slicer:layer-support@1.0.0` and current support output is path-oriented (`crates/slicer-schema/wit/deps/layer-support/layer-support.wit:1-19`); anchored WIT additions must be additive or explicitly migrated with generated guest checks.
- Determinism/scheduler constraints: preserve `STAGE_ORDER`, global-layer worker parallelism, ordered `topo_order`, and no reordering across physical event boundaries.

## Locked Assumptions and Invariants

- A Z-spanning entity is atomic and executes at its anchor's normal scheduler position.
- Planar events are ordered before the anchor model event by physical Z.
- Same-Z support uses ordinary model-event ordering.
- Forced serial and parallel output is identical.

## Risks and Tradeoffs

- Changing `LayerCollectionIR` has a broad struct-literal blast radius across gcode, runtime, SDK, and tests; decomposition is mandatory if inventory exceeds one step's three-edit cap.
- Existing WIT generated bindings may require a guest rebuild even when Rust compilation succeeds.
- Cooling is owned by `PartCooling::run_finalization` in `modules/core-modules/part-cooling/src/lib.rs:79-89`, which consumes the full `LayerCollectionView` set and emits per-layer fan commands; runtime invokes the finalization tier from `crates/slicer-runtime/src/pipeline.rs:412-432`. Print time accounting is owned by `slicer-gcode::estimator::estimate_print` (or `estimate_print_with_elapsed` when elapsed samples are required), whose `PrintEstimate::total_time_s` is defined at `crates/slicer-gcode/src/estimator.rs:89-99` and is committed to `GCodeIR.metadata.estimated_print_time_s` by `DefaultGCodeEmitter::emit_gcode` at `crates/slicer-gcode/src/emit.rs:810-828`; runtime exposes the completed result through `crates/slicer-runtime/src/postpass.rs:38-43`. The anchored implementation must project optimization, cooling, and estimator inputs per physical event without crossing event boundaries.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: all `LayerCollectionIR` literals; `LOCATIONS` at most 20 entries per dispatch.

## Open Questions

- [FWD] The implementation must adapt the existing `PartCooling` finalization owner and `slicer-gcode::estimator::estimate_print` / `estimate_print_with_elapsed` accounting owners to per-event inputs while preserving finalization-stage behavior for ordinary layers.
- [FWD] The exact additive WIT package/version names can be selected by the implementer after generated guest inventory, provided the exported Rust shapes remain unchanged.
