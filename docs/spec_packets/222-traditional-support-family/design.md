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

## Verified Live Anchors
- `modules/core-modules/traditional-support/traditional-support.toml:2,9-18` is `com.core.traditional-support`, `Layer::Support`, and global `support-generator`; it currently lacks `SupportPlanIR`.
- `modules/core-modules/tree-support/tree-support.toml:9-18` verifies the existing paired renderer stage/IR pattern.
- `docs/04_host_scheduler.md:208,216-217` documents `PrePass::SupportGeometry`, `Layer::Support`, and `Layer::SupportPostProcess`.
- `docs/05_module_sdk.md:1068` documents `overhang_areas()`; renderer use is explicitly prohibited by this packet.
- `docs/04_host_scheduler.md:216-217` documents `Layer::Support` and `Layer::SupportPostProcess`, including the downstream ironing stage.

## Out of Bounds
`docs/07_implementation_status.md`, packet 213, Orca source, generated bindings, `target/`, tree implementation, mixed-family routing, and TASK-331 unresolved schema/WIT decisions.

## Open Questions
- [BLOCK] TASK-331 must finalize exact-Z query ownership and shape.
- [BLOCK] The final name and shape of the host exact-Z support query service are inherited from TASK-331's unresolved exact-Z seam; this packet must not hard-code a service symbol before TASK-331 resolves it.
- [BLOCK] TASK-331 must decide breaking versus additive WIT migration.
- [FWD] TASK-331 must export exact structural role and attribution fields consumed by this planner/renderer.

## Context Cost
Aggregate `M`; four `M` steps, no `L` step.
