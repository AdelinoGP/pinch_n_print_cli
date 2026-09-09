---
status: implemented
packet: 223-mixed-support-family-routing
task_ids:
  - TASK-334
---

# 223-mixed-support-family-routing

## Goal
Implement deterministic host-owned routing cells and validation so mixed tree/traditional support plans merge only within family, reject cross-family collisions, and report degraded unmet demands without fallback geometry.

## Problem Statement
The host currently has one generic support-plan ownership path while the approved family packets produce independent tree and traditional entries. Without deterministic ownership and conflict rejection, mixed regions can silently cross-write, collide, or fall back to an unrelated filler. This packet supplies the host boundary needed before closure evidence.

## Architecture Constraints
- The host is the sole writer of aggregated `SupportPlanIR`; family planners emit only assigned-family entries.
- `support_family` resolves planner and renderer atomically; `normal*`/`classic*` map to traditional and `tree*`/`hybrid*` map to tree.
- Invalid complete bodies are dropped, never clipped or replaced by fallback filler.
- Routing-cell tie breaks use object, region, candidate, and family stable IDs; forced serial/parallel execution must produce identical output.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

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
