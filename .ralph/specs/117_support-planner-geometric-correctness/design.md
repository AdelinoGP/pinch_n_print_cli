# Design: support-planner-geometric-correctness

## Controlling Code Paths

- Primary code paths:
  - `modules/core-modules/support-planner/src/lib.rs::tapered_radius` - replace the branch-radius floor with the two-piece mm formula.
  - `modules/core-modules/support-planner/src/lib.rs::run_support_geometry` - replace the sole `inflate_polygon` call with the SDK host-geometry wrapper and preserve complete input/output `ExPolygon` values.
  - `modules/core-modules/support-planner/src/lib.rs::LayerCollisionCache` - change internal avoidance/collision storage only as required to retain holes.
  - `modules/core-modules/support-planner/src/lib.rs::point_in_any_polygon`, `clamp_to_avoidance`, and `push_interface_scan_lines` - consume the corrected cache shape and convert planner mm coordinates at the polygon boundary.
- Neighboring tests/fixtures:
  - Existing `modules/core-modules/support-planner/src/lib.rs` `#[cfg(test)] mod tests` - add named radius and offset oracles.
  - Existing `modules/core-modules/support-planner/tests/orca_parity_tdd.rs::radius_tapers_with_distance_to_top` - migrate the obsolete top-radius assertion and any raw-coordinate fixture affected by the cache representation.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The existing `slicer_sdk::host::offset_polygons` has the guest-compatible shape `offset_polygons(polygons: &[ExPolygon], delta_mm: f32, join: OffsetJoinType) -> Vec<ExPolygon>`. Its host-backed implementation converts the mm delta to scaled units and reconstructs hole nesting; call it with `OffsetJoinType::Miter`.
- `tapered_radius` is entirely mm-valued `f32` arithmetic; no coordinate conversion belongs in B5. `branch_radius`, `effective_layer_height`, and `MAX_BRANCH_RADIUS_MM` remain mm values.
- The planner's node positions are mm floats while `SupportGeometryViewEntry.outlines` are scaled `Point2` polygons. The B6 cache must make this boundary explicit rather than comparing raw `Point2.x/y` values to mm nodes.
- The existing SDK offset wrapper fixes the underlying arc tolerance and miter behavior; do not invent a `JoinType` or miter-limit parameter that the guest-facing signature does not expose.

## Code Change Surface

- Selected approach: keep the public `tapered_radius` signature, replace only its formula, use the existing SDK host-geometry wrapper without changing the guest dependency graph, and make the avoidance cache ExPolygon-aware so the existing call path uses the sanctioned offset operation without discarding holes.
- Exact functions, structs, tests, and fixtures:
  - `tapered_radius` - two-piece formula and function docs.
  - `run_support_geometry` - one `slicer_sdk::host::offset_polygons` call over each input outline and cache insertion.
  - `LayerCollisionCache` plus its containment/clamping consumers - internal ExPolygon representation and canonical coordinate conversion.
  - Existing source unit-test module - radius, concave, hole, and coordinate-boundary tests.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs::radius_tapers_with_distance_to_top` - old expectation migration only.
- Rejected alternatives and reasons:
  - Keep `inflate_polygon` as a fallback - rejected; it is the defect being removed and cannot represent holes.
  - Flatten offset results back to `Vec<Vec<[f32; 2]>>` - rejected; it discards hole nesting and preserves the current unit confusion.
  - Add a direct `slicer-core` dependency - rejected by the guest dependency boundary; the existing SDK host-geometry wrapper already supplies the required ExPolygon-preserving operation.
  - Add Orca's interface-aware radius widening - rejected; B5 is only the tip-cone correction.

## Files in Scope (read + edit)

The two primary files are sufficient: production logic/tests and the one existing integration oracle that asserts the old behavior. No manifest edit is needed because the SDK host-geometry API already exists.

- `modules/core-modules/support-planner/src/lib.rs` - role: B5/B6 implementation and focused unit tests; expected change: formula, cache/API boundary, helper consumers, tests.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - role: existing radius oracle; expected change: migrate `radius_tapers_with_distance_to_top` and only coordinate fixture literals affected by B6.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` - §B5, §B6, §D2 only - formula and scope.
- `docs/08_coordinate_system.md` - the rule/conversion and Clipper2 integration sections only - unit boundary.
- `docs/05_module_sdk.md` and `docs/adr/0023-arachne-port-strategy.md` - guest dependency boundary and host-side crate strategy only.
- `crates/slicer-sdk/src/host.rs` - `OffsetJoinType` and `offset_polygons` definitions only - exact guest-facing API and hole behavior.
- `crates/slicer-schema/wit/deps/common.wit` - existing `offset-polygons` host-service contract.
- `modules/core-modules/support-planner/src/lib.rs` - `tapered_radius`, `run_support_geometry`, `LayerCollisionCache`, containment/clamping helpers, and existing test module only.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - `radius_tapers_with_distance_to_top` and the `node_dropped_when_avoidance_rejects_all_moves` fixture only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - delegate the one named formula lookup; never load directly.
- `docs/07_implementation_status.md` - mutable backlog ownership; use only a bounded mapping survey and never edit it here.
- `crates/slicer-core/**` - no direct guest dependency or packet edit; consume geometry through the existing SDK seam.
- `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`, `crates/slicer-runtime/**`, `crates/slicer-scheduler/**` - no public contract or host pipeline change.
- Other support-planner tests and all other module sources - not needed for this local oracle.
- `target/`, `Cargo.lock`, generated code, and every other packet directory - never load or edit.

## Expected Sub-Agent Dispatches

- Question: Summarize `TreeSupport::calc_branch_radius`'s second overload and confirm the two-piece formula plus upper clamp; scope: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp`; return: `SUMMARY` at most 200 words; purpose: parity check for B5.
- Question: Return the current SDK `OffsetJoinType` and `offset_polygons` signatures plus immediate behavior; scope: `crates/slicer-sdk/src/host.rs` named symbols; return: `SNIPPETS` at most 3, 30 lines each; purpose: prevent a direct host-only dependency or stale call-shape assumption.
- Question: Find the existing direct `tapered_radius` oracle and raw-coordinate avoidance fixture. Scope: the two named symbols in `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`; return: `SNIPPETS` at most 2, 20 lines each; purpose: bound migration fallout.
- Question: Run the targeted radius, offset, full planner, clippy, and guest freshness commands from `requirements.md`; scope: commands only. Return: `FACT` PASS/FAIL, with bounded failure snippets; purpose: gate implementation.

## Data and Contract Notes

- IR/manifest contracts: no schema change; internal cache storage changes from flattened contours to an ExPolygon-aware representation. `SupportPlanIR` field shapes and `tapered_radius` signature remain unchanged.
- WIT boundary: none. The planner still receives `SupportGeometryViewEntry.outlines` as `Vec<ExPolygon>` and calls the existing SDK geometry seam.
- Determinism/scheduler constraints: `tapered_radius` is pure; Clipper offset is deterministic for fixed ExPolygon input and delta; no stage order or scheduling changes.

## Locked Assumptions and Invariants

- `MAX_BRANCH_RADIUS_MM = 6.0` remains unchanged.
- `tapered_radius(..., dist_to_top = 0, ...) == 0.0`; interface-aware widening is not added.
- `offset_polygons` is called once per support outline at the existing avoidance-cache site with positive `avoid_inflate` in mm and `OffsetJoinType::Miter`.
- Holes survive from `SupportGeometryViewEntry.outlines` through the offset result and are not treated as independent filled collision polygons.
- Every comparison between mm node positions and scaled polygon coordinates uses the canonical 10,000-units/mm conversion.
- Public planner function signatures and IR/WIT contracts remain unchanged.

## Risks and Tradeoffs

- The tip width changes from the old floor to zero at contact, so self-capture output and downstream validation baselines must be regenerated by packet 119 rather than silently accepted.
- Converting the cache to ExPolygon-aware containment touches clamping and interface scan-line helpers; this is necessary to avoid a nominal offset replacement that still drops holes.
- The SDK host-geometry seam keeps the guest dependency graph unchanged; any source edit still requires the guest freshness gate, and stale WASM must not be misattributed to the geometry tests.
- The existing `node_dropped_when_avoidance_rejects_all_moves` fixture documents the old raw-coordinate shortcut; migrate it to `Point2::from_mm` in the same implementation step if the new cache requires scaled values.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: bounded SDK geometry API read plus Orca formula summary; `SNIPPETS` at most 3/30 lines and `SUMMARY` at most 200 words.

## Open Questions

- **Closed 2026-07-19:** TASK-281 (B5) and TASK-282 (B6) were added to `docs/07_implementation_status.md` and closed the same day. The prior `[BLOCK]` (no canonical B5/B6 rows; collision with `TASK-254`/`TASK-255` infill rows and closed `TASK-163 (algorithmic)`) is resolved: new rows were created rather than repurposing colliding ones. The implementation was independently verified against OrcaSlicer `TreeSupport::calc_branch_radius` second overload at `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1801` and matches the formula byte-for-byte (modulo the intentionally-excluded interface-aware widening branch).
