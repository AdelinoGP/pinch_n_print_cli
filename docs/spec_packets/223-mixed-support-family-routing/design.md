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
- Selected approach: host-owned `SupportRoutingCell`, `SupportRoutingDiagnostics`, deterministic multi-writer aggregation, complete-body validation, and anchored rendered swept-path conflict handling.
- Exact functions, traits, manifests, tests, and fixtures: existing support-analysis/blackboard/anchored commit seams from TASK-331/TASK-330; planned integration target and module registration; no family algorithm internals.
- Rejected alternative: planner-to-planner negotiation, because the approved first implementation uses independent family writers and host validation.

## Files in Scope (read + edit)
- `crates/slicer-runtime/src/` support dispatch/aggregation seam - route and validate family outputs.
- `crates/slicer-runtime/tests/integration/support_family_routing.rs` - mixed-family contract tests.
- `crates/slicer-runtime/tests/integration/main.rs` and `crates/slicer-runtime/Cargo.toml` - planned target registration (split across atomic steps).

## Read-Only Context
- `crates/slicer-ir/src/slice_ir.rs` lines 1187-1217, 2172-2199 - current SupportPlanIR/SupportIR shapes to migrate through TASK-331.
- `crates/slicer-runtime/src/visual_debug_render.rs` lines 1082-1142 - current support visual tap field names.
- `crates/slicer-runtime/src/blackboard.rs` lines 190-225, 540-565 - current plan and SupportIR slots.

## Out-of-Bounds Files
- `docs/07_implementation_status.md`, packet 213, target artifacts, generated bindings, and Orca source.
- Tree/traditional planner algorithms and TASK-331 unresolved exact-Z/WIT decisions.

## Expected Sub-Agent Dispatches
- Question: locate host support candidate assignment, plan aggregation, body-drop, unmet-demand, and diagnostic seams; scope: `crates/slicer-runtime/src/**`, `crates/slicer-core/src/**`; return: `LOCATIONS`.
- Question: verify integration target wiring and current module declarations; scope: `crates/slicer-runtime/Cargo.toml`, `crates/slicer-runtime/tests/integration/main.rs`; return: `LOCATIONS`.
- Question: summarize routing/overlap behavior in documented Orca files; scope: `OrcaSlicerDocumented/src/libslic3r/Support/**`; return: `LOCATIONS`.

## Data and Contract Notes
- IR/manifest contracts: consume structural `SupportPlanIR`/`SupportIR` from TASK-331; define `SupportRoutingCell` and `SupportRoutingDiagnostics` as host-side names unless TASK-331 exports equivalent fields.
- WIT boundary: do not hard-code the unresolved exact-Z service or migration mode.
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
- [BLOCK] TASK-331 exact-Z seam owner and exact result shape remain unresolved.
- [BLOCK] TASK-331 breaking-versus-additive WIT migration remains unresolved.
- [FWD] TASK-331 must expose structural identity fields consumed by routing and diagnostics.
