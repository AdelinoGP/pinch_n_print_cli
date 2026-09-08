# Design: traditional-support-family

## Selected Approach
Introduce a strategy-neutral-input consumer that plans traditional support as connected cross-layer bodies. The renderer becomes a narrow polygon scan-fill adapter and cannot inspect model region polygons or infer support eligibility during parallel layer execution.

## Code Change Surface
- `modules/core-modules/traditional-support-planner/` -> new planner package, manifest, guest/native implementation, and tests.
- `modules/core-modules/traditional-support/` -> family manifest, plan filtering, semantic polygon renderer, and tests.
- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-sdk/src/*`, `crates/slicer-wasm-host/src/*` -> adapters to TASK-331 structural contracts where required.
- `crates/slicer-runtime/tests/integration/` -> planned real-slice traditional family fixture and aggregator registration.

## Architecture Constraints
- Traditional planner owns contact detection, propagation, interfaces, obstacles, and termination; renderer owns no eligibility algorithm.
- `SupportPlanIR` contains structural polygons and roles, never nozzle-width paths.
- Selection is atomic through `support-family:traditional`; `normal*` and `classic*` aliases map to it.
- Invalid complete bodies are dropped, never clipped or replaced by fallback filler.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## WASM Boundary Disclosure (Design Note / Risk)
Packet 220's live WASM dispatch hands guests an EMPTY structural plan — the layer paint-view boundary does not carry plan entries (macro `__slicer_support_plan_from_view` returns an empty plan); plan-consuming tests drive the renderer natively. This packet's renderer design ("scan-fill only planned body/interface polygons into attributed `SupportIR`, never read `region.polygons()`") must either extend the paint-view boundary to carry plan entries or consume planned geometry via the host aggregation → TASK-330 anchored events path. This is an open design decision for this packet and must be resolved before the renderer can be exercised through live WASM dispatch.

## Verified Live Anchors
- `modules/core-modules/traditional-support/traditional-support.toml:2,9-18` is `com.core.traditional-support`, `Layer::Support`, and `support-generator`; it now holds the `support-family:traditional` claim and per-region selection. It still does not read `SupportPlanIR` — the new `traditional-support-planner` will produce it.
- `modules/core-modules/tree-support/tree-support.toml:9-18` verifies the existing paired renderer stage/IR pattern.
- `docs/04_host_scheduler.md:208,216-217` documents `PrePass::SupportGeometry`, `Layer::Support`, and `Layer::SupportPostProcess`.
- `docs/05_module_sdk.md:1068` documents `overhang_areas()`; renderer use is explicitly prohibited by this packet.
- `docs/04_host_scheduler.md:216-217` documents `Layer::Support` and `Layer::SupportPostProcess`, including the downstream ironing stage.

## Out of Bounds
`docs/07_implementation_status.md`, packet 213, Orca source, generated bindings, `target/`, tree implementation, mixed-family routing, and TASK-331 unresolved schema/WIT decisions.

## Open Questions
- [RESOLVED] TASK-331 exact-Z query ownership and shape: the host exact-Z support query service is `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs`, injected into `HostExecutionContext`, normalized to repo units, immutable per-(object,region,Z) caching, returning occupancy, blockers, eligible termination geometry, and baseline envelope. This packet pins that symbol.
- [RESOLVED] TASK-331 WIT migration: breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0.
- [RESOLVED] TASK-331 exports exact structural role and attribution fields consumed by this planner/renderer: structural `SupportPlanIR` v2.0.0 (family_id, demand IDs, body IDs, anchor layer index + Z, semantic ExPolygon roles, optional skeleton metadata, capabilities/provenance, decline reasons {declined-policy, no-route, blocked, unsupported-mode}); attributed `SupportIR` v2.0.0 (per body/role: family_id, body_id, demand_ids, object/region, role incl. raft+ironing, printable paths); `support_family` canonical + `support_type` aliases (normal*/classic* → traditional, tree*/hybrid* → tree); `support-family:<id>` claims; startup pairing validation; host aggregation in `crates/slicer-wasm-host/src/support_aggregation.rs` as sole multi-writer merge point with internal deterministic routing cells; no fallback filler.

## Context Cost
Aggregate `M`; four `M` steps, no `L` step.
