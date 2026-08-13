# Design: support-family-orca-closure

## Controlling Code Paths
- Primary code path: `slicer-runtime` real-slice closure test, `pnp_cli visual-debug` model/G-code requests, manifest evidence index, and final G-code role parser.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/integration/main.rs`; existing visual-debug tests under `crates/pnp-cli/tests/`; existing SupportTest model and Orca references; comparison bundles `target/vd-orca-tree-compare` and `target/vd-orca-normal-compare`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints
- Closure proves behavioral parity only: coverage, termination, collision freedom, interfaces, independent heights, and printable construction; exact path identity is out of scope.
- `Layer::Support`, `PrePass::SupportAnalysis`, and `PrePass::SupportGeometry` are separate evidence boundaries and must all be captured. Both support stages exist: `PrePass::SupportAnalysis` (host analysis stage carrying candidates, occupancy/termination surfaces, baseline envelope, and deterministic family assignments) and `PrePass::SupportGeometry` (legacy geometry stage, still in STAGE_ORDER).
- The existing decisive fixtures are the primary closure path. A deliberately missing copied path is reserved for the negative gate.
- Final evidence must not treat PNG existence, byte size, manifest greps, or self-captured goldens as proof.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Code Change Surface
- Selected approach: fixture-driven runtime assertions plus visual-debug request/evidence generation and final-G-code role inspection.
- Exact functions, tests, and fixtures: existing typed capture and G-code visual-debug paths; new closure integration test and manifest/evidence fixture requests; fixture production script/process if approved.
- Rejected alternative: accepting stale self-captured goldens, because the plan explicitly requires regenerated inspected differential evidence.

## Files in Scope (read + edit)
- `crates/slicer-runtime/tests/integration/support_family_closure.rs` - real fixture invariants and role tests.
- `crates/slicer-runtime/tests/integration/main.rs` and `crates/slicer-runtime/Cargo.toml` - planned closure module and single `integration` Cargo test target registration.
- `tmp/SupportTest.stl`, `tmp/SupportTest_Tree_Orca.gcode`, `tmp/SupportTest_Normal_Orca.gcode`, and `tmp/visual-debug-support-family.json` - decisive fixture inputs and dual-family request fixture.

## Read-Only Context
- `crates/pnp-cli/src/visual_debug.rs` lines 743-761, 1500-1680 - manifest and typed capture fields.
- `crates/slicer-runtime/src/visual_debug_render.rs` lines 1082-1142 - current support geometry tap renderer.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - standalone G-code role parsing pattern.

## Out-of-Bounds Files
- Orca source, target bundles, generated bindings, packet 213 files, and unrelated planner implementation.
- `docs/07_implementation_status.md` is updated only through delegated status work.

## Expected Sub-Agent Dispatches
- Question: verify fixture existence and gitignore/production path; scope: `tmp/**`, `docs/specs/**`; return: `LOCATIONS`.
- Question: locate visual-debug tap/request and manifest differential seams; scope: `crates/pnp-cli/src/**`, `crates/slicer-runtime/src/**`, existing tests; return: `LOCATIONS`.
- Question: inspect Orca documented behavior at the listed locations; scope: `OrcaSlicerDocumented/**`; return: `LOCATIONS`.
- Question: delegate docs/07 closure and TASK-163b status; scope: `docs/07_implementation_status.md`; return: `SUMMARY`.

## Data and Contract Notes
- IR/manifest contracts: assert typed captures at `PrePass::SupportAnalysis`, `PrePass::SupportGeometry`, and `Layer::Support`; preserve structured family/body/demand roles from TASK-334.
- WIT boundary: no new WIT contract; inherited TASK-331 migration blocker is resolved. Packet 220 performed a breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0` (package stays 1.0.0). Schema versions: `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0.
- WASM boundary: packet 220's live WASM dispatch hands guests an EMPTY structural plan (the paint-view boundary does not carry plan entries); plan-consuming tests drive the renderer natively. For closure evidence, visual-debug taps and G-code role parsing must capture the host-side aggregated plan/`SupportIR` (which carry the identity), not guest-side plan reads.
- Determinism/scheduler constraints: compare forced serial/parallel fixture results and preserve anchored event order.

## Locked Assumptions and Invariants
- Exact-Z body and rendered sweep collision checks are authoritative over skeleton-only checks.
- Missing Orca references cannot be silently replaced by PNP output.

## Risks and Tradeoffs
- `TASK-163b-orca-ref` may remain externally blocked only if provenance/authority cannot be established; the existing references must still be used for primary differential review.
- Visual evidence is human-inspected and therefore cannot be reduced to a grep-only AC.

## Context Cost Estimate
- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: fixture production/availability, `FACT` or `LOCATIONS`.

## Open Questions
- [RESOLVED] TASK-331 exact-Z seam and WIT migration decisions. Exact-Z seam = `ExactZQueryService` in `crates/slicer-wasm-host/src/exact_z_query.rs` (injected into `HostExecutionContext`; normalized to repo units, immutable per-(object,region,Z) caching). WIT migration = breaking in-place replacement of the `support-plan-entry` record within `slicer:prepass-support-geometry@1.0.0`; `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` 1.3.0→2.0.0, `CURRENT_SUPPORT_IR_SCHEMA_VERSION` 1.0.0→2.0.0, `CURRENT_SUPPORT_ANALYSIS_IR_SCHEMA_VERSION` 1.0.0. New contracts live: `PrePass::SupportAnalysis` + `SupportAnalysisIR` (candidates, occupancy/termination surfaces, baseline envelope, deterministic family assignments); structural `SupportPlanIR` v2.0.0 (family_id, demand IDs, body IDs, anchor layer index + Z, semantic ExPolygon roles, optional skeleton metadata, capabilities/provenance, decline reasons); attributed `SupportIR` v2.0.0 (per body/role: family_id, body_id, demand_ids, object/region, role incl. raft+ironing, printable paths); `support_family` canonical + `support_type` aliases; `support-family:<id>` claims; startup pairing validation; host aggregation in `crates/slicer-wasm-host/src/support_aggregation.rs` with internal deterministic routing cells; no fallback filler.
- [BLOCK] Who will provide authoritative Orca tree/normal G-code references if the documented checkout cannot regenerate them?
- [FWD] TASK-334 must export final diagnostic fields and unmet-demand disposition consumed by closure tests.
