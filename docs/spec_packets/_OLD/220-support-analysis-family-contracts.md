---
status: implemented
packet: 220-support-analysis-family-contracts
task_ids:
  - TASK-331
---

# 220-support-analysis-family-contracts

## Goal

Split host support analysis from family strategy planning by adding exact-Z host queries, universal structural support plan/output contracts, and atomic per-region planner-renderer family selection with fatal pairing validation and no fallback filler.

## Problem Statement

The live tree has only `PrePass::SupportGeometry` in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs:15-46`), a tree-specific `SupportPlanIR` carrying `ExtrusionPath3D` branch segments (`crates/slicer-ir/src/slice_ir.rs:1144-1207`), and flat path-only `SupportIR` (`crates/slicer-ir/src/slice_ir.rs:2172-2187`). Both support modules claim the same global `support-generator` and are selected by raw `support_type` (`crates/slicer-scheduler/src/execution_plan.rs:219-251, 393-412`). This packet establishes the strategy-neutral and family-atomic contracts needed by downstream planners.

## Architecture Constraints

- `SupportPlanIR` becomes structural and universal; no planner emits `ExtrusionPath3D` nozzle-width paths into it.
- `SupportIR` retains printable paths only as renderer output and carries body/family/demand/role attribution.
- Family planner and renderer selection is atomic per region; no global “first winner” support-generator dedup.
- Exact-Z host results are normalized to repository units and immutable/cached; families tighten the baseline envelope for their geometry.
- Invalid complete bodies are dropped, not clipped or replaced by fallback filler; positive-area cross-family conflict handling is completed by TASK-334.
- `support_type` remains a compatibility alias to `support_family`; `support_family` is the canonical selector.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- IR/manifest contracts: live `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` is `1.3.0` and `CURRENT_SUPPORT_IR_SCHEMA_VERSION` is `1.0.0` at `crates/slicer-ir/src/slice_ir.rs:253-308`; do not hard-code a future target in ACs. The migration step must compute and document the actual bump.
- WIT boundary: `prepass-support-geometry` is `slicer:prepass-support-geometry@1.0.0` with branch-segment records at lines 1-20; `layer-support` is `slicer:layer-support@1.0.0`. Existing package versions are pre-existing facts; new version choice is a blocker until generated guests are inventoried.
- Determinism/scheduler constraints: host invokes each selected family once per object, retains demand identity, aggregates immutably, and uses TASK-330 `AnchoredEntity.anchor_global_layer_index` for support event placement.

## Locked Assumptions and Invariants

- Support family selection resolves planner and renderer together.
- Enforcers guarantee candidate creation, not printed geometry.
- Declined/unroutable candidates are degraded structured outcomes.
- Complete-body validation uses exact-Z occupancy and routing cells.
- Support disabled means no candidates, plans, anchored events, or paths.

## Risks and Tradeoffs

- This is a breaking semantic migration of both existing support IRs, with macro/WIT/SDK/host/test fallout; preserving old branch-path meaning under the same schema is forbidden.
- Current `dedup_same_claim_modules` intentionally removes losing support modules globally; changing it to retain per-region candidates may affect unrelated claim tests and requires a focused compatibility audit.
- Exact-Z caching may need a new host service seam because the grounded tree exposes coarse `SupportGeometryIR`, not a generic query API.
