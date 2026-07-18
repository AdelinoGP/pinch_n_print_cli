---
status: implemented
packet: 148-arachne-per-vertex-parity
task_ids:
  - none
---

# 148-arachne-per-vertex-parity

## Goal

Close the seven arachne-path gaps (G7, G10, G12, G18, G19, G20, G21 in the audit) by populating `WallLoop.boundary_type`, `WallLoop.loop_type`, per-vertex `WallFeatureFlags.is_bridge`/`is_thin_wall` and per-vertex `Point3WithWidth.overhang_quartile` in the arachne guest module's `run_perimeters`, emitting seam candidates for the outer wall, and gating `ArachneParams.outer_wall_offset` on `precise_outer_wall && wall_sequence==InnerOuter` (the beading-stack mechanism — mirrors OrcaSlicer's `OuterWallInsetBeadingStrategy::compute`, NOT a post-hoc path mutation). The audit's `arachne_parity.rs` red tests are REWRITTEN to drive the guest module's `run_perimeters` natively (no WASM, no `run_arachne_pipeline`, no source-text substring matching) so the ACs verify real `WallLoop` output.

## Problem Statement

The audit (`tmp/arachne_parity_audit_20260709.md`) found that `arachne-perimeters` produces real walls (P112 + P141–P147) but emits them as a degenerate `WallLoop`: `boundary_type` is hardcoded `Interior` for every wall, `LoopType::ThinWall` is never returned by `classify_line`, `is_thin_wall`/`is_bridge` per-vertex flags are never set, `overhang_quartile` is never populated, no seam candidates are emitted, and `precise_outer_wall` is not even registered in the manifest. The pipeline reaches OrcaSlicer parity via the classic path (which already does all of these), but the arachne path diverges. Seven red tests in `crates/slicer-runtime/tests/arachne_parity.rs` lock this gap; closing them is the runnable acceptance criterion for this packet.

The classic path's `crates/slicer-core/src/perimeter_utils.rs` already exports the helpers this packet needs (`point_in_any_polygon`, `generate_sharp_corner_seam_candidates`, `WallSequence`); the work is wiring them into the arachne module's `run_perimeters` loop and `classify_line` function, plus the manifest entries for two new config keys.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- `SliceRegionView::bridge_areas() -> &[ExPolygon]` is already exposed to WASM guests today (`crates/slicer-sdk/src/views.rs:388`; WIT at `crates/slicer-schema/wit/deps/ir-types.wit:63`; macro adapter at `crates/slicer-macros/src/lib.rs:2150-2158`; populated host-side by `crates/slicer-wasm-host/src/marshal/in_.rs:312`). No pre-packet to expose it; the arachne module can call it directly.
- `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]` is exposed (`crates/slicer-sdk/src/views.rs:468`; the per-vertex lookup filters the bands whose polygons contain the point and takes the max quartile among matches — mirrors `expolygon_to_path3d` at `crates/slicer-core/src/perimeter_utils.rs:316-331`).
- The arachne module's `ExtrusionLine::junctions` carries the path in **mm** (set by `extrusion_line_to_extrusion_path3d`); the per-vertex overhang/bridge checks operate in mm against the `region`'s `ExPolygon` (also mm via `SliceRegionView`); no unit conversion is needed in the new code paths.
- `WallLoop.feature_flags` and `boundary_type` are populated in the construction loop at 296-303, BEFORE `output.push_wall_loop(wall)?` is called. The `output` is a `&mut PerimeterOutputBuilder`; the new fields are added to `WallLoop` at construction time, not via a post-hoc mutation pass.
- The `is_bridge` / `overhang_quartile` / `flow_factor` values originate in the host pipeline (`generate_toolpaths.rs:184,470,865` hardcodes `None` / `1.0`). The guest module's `extrusion_line_to_extrusion_path3d` copies these defaults into `path.points`; the guest then overrides per-vertex from `region.bridge_areas()` and `region.overhang_quartile_polygons()`. The rewritten tests assert on the guest's `output.wall_loops()[i].path.points[j]` (the post-override copy), NOT on the host pipeline's `ExtrusionLine.junctions`.
- The test rewrite in `crates/slicer-runtime/tests/arachne_parity.rs` drives `ArachnePerimeters::run_perimeters` natively via `PerimeterOutputBuilder::new()` + `SliceRegionViewBuilder` + `ConfigViewBuilder` + `PaintRegionLayerView::new(0)`, asserting on real `WallLoop` output. The harness is identical to the one used by `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`.

## Data and Contract Notes

- **IR or manifest contracts touched:**
  - `WallLoop.boundary_type` — already an enum variant in `slicer_ir` (`WallBoundaryType::{Interior, ExteriorSurface, ...}`). No new variant.
  - `WallLoop.feature_flags: Vec<WallFeatureFlags>` — already per-vertex. No shape change.
  - `WallLoop.loop_type` — already an enum; `LoopType::ThinWall` already exists. No new variant.
  - `Point3WithWidth.overhang_quartile: Option<u8>` — already present. No shape change.
  - `Point3WithWidth.flow_factor: f32` — already present. No shape change.
  - `arachne-perimeters.toml [config.schema]` — three new entries: `precise_outer_wall` (bool, default false), `seam_candidate_angle_threshold_deg` (float, default 30.0, range 0.0..=180.0), and `wall_sequence` (copied from classic:81). All match classic's manifest entries byte-for-byte.
  - `ArachneParams.outer_wall_offset` — already a field; this packet only GATES its value on `precise_outer_wall && wall_sequence==InnerOuter`. No field additions.
- **WIT boundary considerations:** none. The arachne module's output type `WallLoop` is host-internal; it does not cross a WIT boundary as a guest input. The two new config keys are read from `ConfigView` (the in-memory config representation), not from WIT. The seam-candidate output is emitted via the SDK's `output.push_seam_candidate(pos, score)` (which is the `PerimeterOutputBuilder` method, also host-internal at this stage).
- **Determinism or scheduler constraints:** none beyond what classic-perimeters already enforces. The seam-candidate emission is deterministic for a given input polygon (the helper `generate_sharp_corner_seam_candidates` is pure).

## Locked Assumptions and Invariants

- The two new config keys (`precise_outer_wall`, `seam_candidate_angle_threshold_deg`) MUST have **identical** defaults and ranges to classic's manifest entries. The test for AC-7 grep-asserts the manifest TOML; the implementer should `diff` against classic's manifest before committing.
- The `is_bridge` flag MUST be set per-vertex, NOT per-line. A whole-line `is_bridge = true` is the wrong shape and would fail AC-4 (the rewritten test reads per-vertex).
- The `is_thin_wall` flag MUST only be set on `LoopType::ThinWall` walls, NEVER on `Outer`/`Inner` walls that happen to be narrow. The rewritten AC-3 test asserts the shape lock; the implementer should not over-broaden the flag to all narrow walls.
- The `overhang_quartile` lookup MUST be a per-vertex point-in-polygon against `region.overhang_quartile_polygons()` bands, NOT against `region.overhang_areas()`. The latter is the un-banded overhang footprint; the former is the banded classification. The rewritten AC-5 test asserts the banded shape.
- The seam-candidate emission MUST be limited to the outer walls (each input region polygon's outermost contour — one helper call per island, mirroring classic's per-polygon loop at `lib.rs:887-893`; holes are excluded). Emitting seam candidates for inner walls would be wrong (the seam-placer reads them only for the outer wall). The seam helper `generate_sharp_corner_seam_candidates` takes a `&slicer_ir::Polygon` (units-space input contour), NOT `&wall.path` (mm-space `ExtrusionPath3D`) — the call shape is `&region.polygons()[0].contour`, mirroring classic's `lib.rs:889-900`.
- The `outer_wall_offset` MUST be applied ONLY when `precise_outer_wall && wall_sequence == InnerOuter`. The `OuterInner` and `InnerOuterInner` sequences do not have the same offset semantics (OrcaSlicer's `OuterWallInsetBeadingStrategy` is gated on `wall_sequence == InnerOuter`).
- Reversibility: the change is reversible via existing config defaults (precise_outer_wall defaults to false; the seam threshold defaults to 30°; classify_line's ThinWall arm only fires when `detect_thin_wall` is on); no behavior locks beyond the invariants above and the test suite.

## Risks and Tradeoffs

- **Risk:** adding `slicer-core` to `arachne-perimeters/Cargo.toml` increases the module's dependency surface. **Mitigation:** classic-perimeters already has this dep; the increase is zero (same crate path). The `default-features = false` pin ensures `host-algos` (voronoi/rayon) is never enabled on the guest.
- **Risk:** the per-vertex overhang-band lookup in the construction loop adds O(num_points × num_bands × polygon_complexity) to the per-region wall generation. For typical models this is negligible (≤ 1000 points × 4 bands × 100 vertices = 400k operations per region per layer). For pathological cases (large overhang areas with many bands), it could be measurable. **Mitigation:** the classic path does the same lookup at `expolygon_to_path3d:316-331` and no regression has been logged. If a perf issue surfaces, the lookup can be hoisted to a precomputed per-region `Vec<Option<u8>>` keyed by point.
- **Risk:** the seam-candidate emission introduces a new host-service call site (`output.push_seam_candidate`). The host may not yet route the seam candidates correctly if `seam-placer` is wired in a packet that hasn't landed. **Mitigation:** the existing classic path's `push_seam_candidate` is in production; the host's routing is already correct. The new call site is symmetric.
- **Risk:** the test rewrite is a substantial chunk of work — 7 unit-test files plus the `arachne_parity.rs` rewrite. **Mitigation:** the harness pattern is already established by `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`; each new unit test is a small variation on that pattern. The work is parallelizable and can be dispatched to a sub-agent.
