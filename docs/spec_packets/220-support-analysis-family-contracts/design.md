# Design: support-analysis-family-contracts

## Controlling Code Paths

- Primary code path: `crates/slicer-ir/src/slice_ir.rs:253-319, 1141-1207, 2170-2200`; `crates/slicer-ir/src/stage_io.rs:257-298`; `crates/slicer-scheduler/src/execution_plan.rs:15-46, 219-459`; `crates/slicer-wasm-host/src/marshal/out.rs:155-276`; `crates/slicer-macros/src/lib.rs:2300-2343`.
- Neighboring tests/fixtures: `crates/slicer-ir/tests/ir_tests.rs:726-758`, `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs:275-320, 744-779`, `crates/slicer-wasm-host/tests/contract/prepass_output_builder_validation_tdd.rs:1-40`, and `crates/slicer-wasm-host/tests/contract/wit_boundary_tdd.rs`.
- OrcaSlicer comparison: delegated only; see `requirements.md` obligations.

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

## Code Change Surface

- Selected approach: add strategy-neutral host analysis before family planning, migrate legacy support IR at one explicit schema boundary, retain all family candidates through load/dispatch, aggregate and validate on the host, then pass attributed entries to anchored render events from TASK-330.
- Exact functions, traits, manifests, tests, and fixtures: `STAGE_ORDER` and prepass runner; `BlackboardPrepassSlot`; support IR defaults/constants; WIT support-geometry and layer-support records; macro support plan adapter; host marshal conversion; claim dedup/validation; `support-planner.toml`, `tree-support.toml`, `traditional-support.toml`; contract/integration aggregators.
- Rejected alternatives: preserving branch paths under the old schema version (explicitly prohibited by the plan); letting renderers infer eligibility from `region.polygons()` or `region.overhang_areas()`; planner negotiation; opaque family bytes/private role IDs.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/stage_io.rs` - new analysis/plan/output shapes and blackboard slots.
- `crates/slicer-scheduler/src/execution_plan.rs` and manifest validation files - stage, family selection, pairing, and retention.
- `crates/slicer-wasm-host/src/*` selected analysis/query/marshal files - host services and aggregation.
- `crates/slicer-schema/wit/deps/*`, `crates/slicer-sdk/src/*`, `crates/slicer-macros/src/lib.rs` - boundary migration.
- `modules/core-modules/*/*.toml` - claims/IR access/family selectors.
- Targeted IR/scheduler/host/runtime tests and their aggregators - contract evidence.

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` lines 240-319, 1141-1207, 2170-2200 - live schema constants and legacy shapes.
- `crates/slicer-ir/src/stage_io.rs` lines 257-344 - blackboard slot/error model.
- `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit` lines 1-90 - current WIT producer.
- `crates/slicer-schema/wit/deps/layer-support/layer-support.wit` lines 1-20 - current renderer WIT.
- `modules/core-modules/support-planner/support-planner.toml` lines 1-18, `tree-support/tree-support.toml` lines 1-18, `traditional-support/traditional-support.toml` lines 1-18 - current stage/claims.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, generated bindings, `Cargo.lock`, vendored dependencies - never load.
- `docs/07_implementation_status.md` - delegate; do not edit.
- `docs/spec_packets/213-support-planner-defect-fix/` - never touch.
- TASK-332/TASK-333 implementation files except manifest claim changes explicitly required for pairing.

## Expected Sub-Agent Dispatches

- Question: enumerate all legacy `SupportPlanIR`, `SupportIR`, `SupportPlanEntry`, and WIT `support-plan-entry` consumers/literals; scope: `crates/**/*.rs` and `crates/slicer-schema/wit/**`; return: `LOCATIONS`.
- Question: locate all `support_type` reads and support-generator dedup tests; scope: `crates/slicer-scheduler`, `crates/slicer-model-io`, `crates/slicer-runtime`; return: `LOCATIONS`.
- Question: locate exact-Z mesh cross-section, occupancy, support geometry, and caching seams; scope: `crates/slicer-wasm-host/src`, `crates/slicer-core/src`; return: `LOCATIONS`.
- Question: summarize ADR-0059 and current support schema documentation; scope: `docs/adr/0059-support-families-and-anchored-entities.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`; return: `SUMMARY`.
- Question: inspect documented Orca support planning locations; scope: `OrcaSlicerDocumented/`; return: `LOCATIONS`.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: legacy support IR/WIT consumer inventory; `LOCATIONS` batches of at most 20 entries.

## Open Questions

- [RESOLVED] Q1 (exact-Z seam): a new host query service `crates/slicer-wasm-host/src/exact_z_query.rs` was created beside the `HostExecutionContext` mesh-query block. It normalizes exact-Z results to repository units, and caches immutably per `(object, region, Z)`.
- [RESOLVED] Q2 (WIT migration): breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0` (no external consumers; all consumers regenerated). `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` moved 1.3.0 → 2.0.0 and `CURRENT_SUPPORT_IR_SCHEMA_VERSION` moved 1.0.0 → 2.0.0.
- [FWD] TASK-330's anchored exports are accepted as draft forward dependencies only if its final names and shapes remain exactly those listed in `requirements.md`.
