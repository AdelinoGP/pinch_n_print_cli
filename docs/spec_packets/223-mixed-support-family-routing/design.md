# Design: mixed-support-family-routing

## Controlling Code Paths
- Primary code path: host support analysis dispatch, plan aggregation/validation, anchored `Layer::Support` commit hook, and diagnostic serialization.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/main.rs` aggregator; new `crates/slicer-runtime/tests/integration/support_family_routing.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints
- The host is the sole writer of aggregated `SupportPlanIR`; family planners emit only assigned-family entries.
- `support_family` resolves planner and renderer atomically; `normal*`/`classic*` map to traditional and `tree*`/`hybrid*` map to tree.
- Invalid complete bodies are dropped, never clipped or replaced by fallback filler.
- Routing-cell tie breaks use object, region, candidate, and family stable IDs; forced serial/parallel execution must produce identical output.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface
- Selected approach: extend packet 220's host-owned internal `RoutingCell` and `in_routing_cell` validation in `crates/slicer-wasm-host/src/support_aggregation.rs` with cross-family conflict policy; add structured `SupportRoutingDiagnostics` (host-side), deterministic multi-writer aggregation, complete-body validation, and anchored rendered swept-path conflict handling.
- Exact functions, traits, manifests, tests, and fixtures: existing support-analysis/blackboard/anchored commit seams from TASK-331/TASK-330; planned integration target and module registration; no family algorithm internals.
- Rejected alternative: planner-to-planner negotiation, because the approved first implementation uses independent family writers and host validation.

## Files in Scope (read + edit)
- `crates/slicer-runtime/src/` support dispatch/aggregation seam - route and validate family outputs.
- `crates/slicer-runtime/tests/integration/support_family_routing.rs` - mixed-family contract tests.
- `crates/slicer-runtime/tests/integration/main.rs` and `crates/slicer-runtime/Cargo.toml` - planned target registration (split across atomic steps).

## Read-Only Context
- `crates/slicer-ir/src/slice_ir.rs` - structural `SupportPlanIR` v2.0.0 and attributed `SupportIR` v2.0.0, consumed from TASK-331's implemented shapes (packet 220 migrated them; no further migration needed here).
- `crates/slicer-wasm-host/src/support_aggregation.rs` - packet 220's host aggregation: `RoutingCell` (internal deterministic routing cell, fixed grid, cell derived from body-centroid; `ROUTING_CELL_SIZE` = 1<<20) and `in_routing_cell` complete-body validation against exact-Z occupancy.
- `crates/slicer-runtime/src/visual_debug_render.rs` lines 1082-1142 - current support visual tap field names.
- `crates/slicer-runtime/src/blackboard.rs` lines 190-225, 540-565 - current plan and SupportIR slots.

## Out-of-Bounds Files
- `docs/07_implementation_status.md`, packet 213, target artifacts, generated bindings, and Orca source.
- Tree/traditional planner algorithms; packet 220's resolved exact-Z/WIT decisions are inherited, not reopened.

## Expected Sub-Agent Dispatches
- Question: locate host support candidate assignment, plan aggregation, body-drop, unmet-demand, and diagnostic seams; scope: `crates/slicer-runtime/src/**`, `crates/slicer-core/src/**`; return: `LOCATIONS`.
- Question: verify integration target wiring and current module declarations; scope: `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/tests/integration/main.rs`; return: `LOCATIONS`.
- Question: summarize routing/overlap behavior in documented Orca files; scope: `OrcaSlicerDocumented/src/libslic3r/Support/**`; return: `LOCATIONS`.

## Data and Contract Notes
- IR/manifest contracts: consume structural `SupportPlanIR` v2.0.0 (family_id, demand IDs, body IDs, anchor layer index + Z, semantic ExPolygon roles, optional skeleton metadata, capabilities/provenance, decline reasons) and attributed `SupportIR` v2.0.0 (per body/role: family_id, body_id, demand_ids, object/region, role incl. raft+ironing, printable paths) from TASK-331's implemented shapes.
- Host aggregation: packet 220's `crates/slicer-wasm-host/src/support_aggregation.rs` is the sole multi-writer merge point; it already owns internal deterministic `RoutingCell` territory (fixed grid, per-body cell from geometry centroid) plus complete-body validation against exact-Z occupancy and routing cells, structured unmet diagnostics, and degraded continuation — no fallback filler. This packet extends those cells with cross-family positive-area overlap rejection and the full mixed-family conflict policy.
- `support_family` canonical + `support_type` aliases (normal*/classic* → traditional, tree*/hybrid* → tree); `support-family:<id>` claims and startup pairing validation (fatal on mismatch) come from TASK-331's contracts.
- WIT boundary: do not hard-code the unresolved exact-Z service or migration mode. Use the resolved `ExactZQueryService` seam from packet 220.
- Determinism/scheduler constraints: preserve global-layer barriers and anchored event order; no cross-layer scheduler.

## Locked Assumptions and Invariants
- Accepted bodies remain connected to demands and eligible termination; declined/unroutable demands are degraded, not fatal.
- Positive-area cross-family overlap drops both complete bodies; tolerance-only boundary contact is allowed.

## Risks and Tradeoffs
- Shared host validation may expose field-shape changes from TASK-331; keep adapters at the host seam and do not duplicate family semantics.
- Rendered swept-path conflict checks can reduce coverage; diagnostics must make every unmet demand explainable.

## Context Cost Estimate
- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: host aggregation symbol inventory, `LOCATIONS`.

## Open Questions
- [RESOLVED] TASK-331 exact-Z seam owner and result shape: `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs`, injected into `HostExecutionContext`, normalized to repo units, immutable per-(object,region,Z) caching.
- [RESOLVED] TASK-331 breaking-versus-additive WIT migration: breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0; new `PrePass::SupportAnalysis` + `SupportAnalysisIR` contracts live.
- [RESOLVED] TASK-331 exposed the structural identity fields consumed by routing and diagnostics (implemented in packet 220; consumed here).

## Scope Boundary vs Packet 220
Packet 220 (TASK-331) implemented the BASE routing-cell attribution contract: internal deterministic `RoutingCell` territory in `crates/slicer-wasm-host/src/support_aggregation.rs` (fixed grid, per-body cell derived from geometry centroid) and complete-body validation against exact-Z occupancy + routing cells, with degraded continuation. Packet 220 deliberately did NOT implement cross-family positive-area overlap rejection. THIS packet (TASK-334) owns cross-family positive-area overlap rejection and the full mixed-family routing conflict policy.

## WASM Boundary Note
Packet 220's live WASM dispatch hands guests an EMPTY structural plan — the paint-view boundary does not carry plan entries, so plan-consuming tests drive the renderer natively. This packet's anchored `Layer::Support` commit hook design should consume planned geometry via the host aggregation → TASK-330 anchored events path.
