---
status: implemented
packet: 219-anchored-entity-execution
task_ids:
  - TASK-330
---

# 219-anchored-entity-execution

## Goal

Add a generic anchored-work IR and capability-derived execution path so each global-layer worker returns ordered event collections for planar and atomic Z-spanning entities without replacing global layers as the scheduler barrier.

## Problem Statement

The live tree models `GlobalLayer` as an unsigned-index worker and `LayerCollectionIR` as one flat per-layer output (`crates/slicer-ir/src/slice_ir.rs:1015-1026`, `2323-2349`). The approved plan requires work below, at, and above a model event while retaining global-layer barriers. Existing `Layer::PathOptimization` and staged commit seams must be generalized rather than introducing a second scheduler.

## Architecture Constraints

- Preserve `GlobalLayer.index: u32` and existing signed raft-prefix representation in support-plan entries; do not make `GlobalLayer` negative.
- Capability closure must be computed from declared input/output capabilities, not an event-kind table.
- `layer-parallel-safe` remains the manifest hint and applies to anchored invocations as well as ordinary model layers.
- Planar event validation uses the repository coordinate tolerance; Z-spanning validation replaces the old model-layer envelope assumption only for declared spans.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

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
