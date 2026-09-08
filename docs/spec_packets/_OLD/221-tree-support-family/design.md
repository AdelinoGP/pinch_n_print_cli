# Design: tree-support-family

## Selected Approach
Retain the existing tree propagation math only where it can consume strategy-neutral demands and exact-Z host results. Replace branch extrusion-path output with structural body/interface polygons and have the renderer scan-fill those polygons into attributed anchored support events.

## Code Change Surface
- `modules/core-modules/support-planner/` -> rename module identity and planner boundary; preserve/rework tree algorithm internals.
- `modules/core-modules/tree-support/` -> family manifest, plan filtering, polygon-to-path renderer.
- `modules/core-modules/tree-support-planner/` -> new package path, manifest, guest/native tests, and guest build inputs.
- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/*`, `crates/slicer-wasm-host/src/*` -> consume TASK-331 structural contracts only where tree-specific adapters are required.
- `crates/slicer-runtime/tests/integration/` -> planned end-to-end tree fixture target registration if existing package target cannot drive anchored dispatch.

## Architecture Constraints
- `SupportPlanIR` is structural; no `ExtrusionPath3D` branch path is emitted.
- ADR-0009's single-writer rule, that `support-planner` keeps sole ownership of `SupportPlanIR`, is preserved by hosting that ownership in the host aggregator: it is the sole writer of the aggregated `SupportPlanIR`, while each family planner emits only family-scoped plan entries for host aggregation. The named-owner change is recorded as an explicit amendment in ADR-0059 (which supersedes ADR-0009's single-writer clause); this packet conforms to ADR-0059 and introduces no second writer.
- Tree planner and renderer selection is atomic and uses `support-family:tree`; `tree*` and `hybrid*` aliases resolve to it.
- Invalid complete bodies are dropped, never clipped or replaced by filler.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Design Note: WASM Boundary Disclosure
Packet 220's live WASM dispatch hands guests an EMPTY structural plan — the layer paint-view boundary does not carry plan entries (macro `__slicer_support_plan_from_view` returns an empty plan), so plan-consuming tests drive the renderer natively. This packet's renderer design ("scan-fill planned polygons into attributed `SupportIR`") must either extend the paint-view boundary to carry plan entries or consume planned geometry via the host aggregation → TASK-330 anchored events path. Treat this as an open risk when wiring the renderer end-to-end tests.

## Verified Live Anchors
- `modules/core-modules/support-planner/support-planner.toml:2,10,13-17` is `com.core.support-planner`, the `PrePass::SupportAnalysis` stage, and `support-planner` plus per-region `support-family:tree` / `support-family:traditional` claims. Family selection is per-region and deterministic, not a single global winner.
- `modules/core-modules/tree-support/tree-support.toml:2,10,13-18` is `com.core.tree-support`, `Layer::Support`, and global `support-generator` claim.
- `modules/core-modules/tree-support/tests/tree_support_tdd.rs` and `slicer_module_binding_tdd.rs` are existing tree test targets.
- Existing `SupportPlanIR`/`SupportIR` shapes are consumed through TASK-331's implemented contracts (packet 220); their migration is not duplicated here.

## Out of Bounds
`docs/07_implementation_status.md`, packet 213, Orca source, generated bindings, `target/`, traditional algorithm code, mixed-family routing, and unresolved TASK-331 schema/WIT decisions.

## Open Questions
- [RESOLVED] The host exact-Z support query service is `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs` (exported name `ExactZQueryService`, with `ExactZSupportQuery` as the query result type); the structural role names and renderer attribution fields are exported by TASK-331 as `SupportAnalysisIR` / `SupportPlanIR` / `SupportIR` (see packet 220).
- [RESOLVED] Breaking-versus-additive WIT migration: breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` stays 1.0.0.
- [RESOLVED] Exact-Z service and host aggregation live in `ExactZQueryService` (exact_z_query.rs) and the sole multi-writer merge point `aggregate_support_plans` / `aggregate_support_plan_ir` in `crates/slicer-wasm-host/src/support_aggregation.rs`. This packet consumes them by their pinned names.

## Context Cost
Aggregate `M`; four `M` steps, no `L` step.
