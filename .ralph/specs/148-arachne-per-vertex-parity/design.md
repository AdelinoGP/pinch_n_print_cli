# Design: 148-arachne-per-vertex-parity

## Controlling Code Paths

- Primary code path: `modules/core-modules/arachne-perimeters/src/lib.rs::run_perimeters` (lines 236-352). The `WallLoop` construction loop at 284-304 is the single refactor surface for AC-1 through AC-6.
- `classify_line` (lib.rs:206-214) is the single site for AC-2/AC-3 (loop type + thin-wall flag).
- `arachne_params_from_config` (lib.rs:106-197) gains reads for `precise_outer_wall` and `seam_candidate_angle_threshold_deg`; and gates `ArachneParams.outer_wall_offset` on `precise_outer_wall && wall_sequence==InnerOuter` (AC-8, beading-stack mechanism).
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load). Do not restate the delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see the project instructions §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- `SliceRegionView::bridge_areas() -> &[ExPolygon]` is already exposed to WASM guests today (`crates/slicer-sdk/src/views.rs:388`; WIT at `crates/slicer-schema/wit/deps/ir-types.wit:63`; macro adapter at `crates/slicer-macros/src/lib.rs:2150-2158`; populated host-side by `crates/slicer-wasm-host/src/marshal/in_.rs:312`). No pre-packet to expose it; the arachne module can call it directly.
- `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]` is exposed (`crates/slicer-sdk/src/views.rs:468`; the per-vertex lookup walks the bands in order and assigns the first band whose polygons contain the point — mirrors `expolygon_to_path3d` at `crates/slicer-core/src/perimeter_utils.rs:316-331`).
- The arachne module's `ExtrusionLine::junctions` carries the path in **mm** (set by `extrusion_line_to_extrusion_path3d`); the per-vertex overhang/bridge checks operate in mm against the `region`'s `ExPolygon` (also mm via `SliceRegionView`); no unit conversion is needed in the new code paths.
- `WallLoop.feature_flags` and `boundary_type` are populated in the construction loop at 296-303, BEFORE `output.push_wall_loop(wall)?` is called. The `output` is a `&mut PerimeterOutputBuilder`; the new fields are added to `WallLoop` at construction time, not via a post-hoc mutation pass.
- The `is_bridge` / `overhang_quartile` / `flow_factor` values originate in the host pipeline (`generate_toolpaths.rs:184,470,865` hardcodes `None` / `1.0`). The guest module's `extrusion_line_to_extrusion_path3d` copies these defaults into `path.points`; the guest then overrides per-vertex from `region.bridge_areas()` and `region.overhang_quartile_polygons()`. The rewritten tests assert on the guest's `output.wall_loops()[i].path.points[j]` (the post-override copy), NOT on the host pipeline's `ExtrusionLine.junctions`.
- The test rewrite in `crates/slicer-runtime/tests/arachne_parity.rs` drives `ArachnePerimeters::run_perimeters` natively via `PerimeterOutputBuilder::new()` + `SliceRegionViewBuilder` + `ConfigViewBuilder` + `PaintRegionLayerView::new(0)`, asserting on real `WallLoop` output. The harness is identical to the one used by `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`.

## Code Change Surface

- **Selected approach:** Single-refactor approach. All seven gaps are closed in one pass through `run_perimeters` and `classify_line`, with a Cargo.toml dep addition (`slicer-core` with `default-features = false` so `host-algos` is never enabled on the guest) and a manifest addition (two new sections). The 7 audit red tests are REWRITTEN to drive the guest module natively so the ACs verify real `WallLoop` output, not source-text substrings. The alternative — one packet per gap — was rejected because (a) all seven share the same two code sites (classify_line + WallLoop construction), (b) the manifest entries are two atomic additions, and (c) the deviation row can be refined once at packet close.
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `modules/core-modules/arachne-perimeters/Cargo.toml`:
    - Add `slicer-core = { path = "../../../crates/slicer-core", default-features = false }` to `[dependencies]`. The `default-features = false` pin is the safety net that prevents `host-algos` (which pulls `boostvoronoi` / `rayon` and does not compile to `wasm32`) from accidentally being enabled on the guest. `slicer-core` itself builds to `wasm32-unknown-unknown` clean with default features (verified: `cargo build -p slicer-core --target wasm32-unknown-unknown` finishes in 15s); classic-perimeters already depends on it unconditionally, and that wasm builds. The pin here matches the safety-first pattern.
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`:
    - Add `[config.schema.precise_outer_wall]` (matches classic's at `classic-perimeters.toml:75-79`): `type = "bool"`, `default = false`, `display = "Precise outer wall (gated on wall_sequence=InnerOuter)"`, `group = "Walls"`.
    - Add `[config.schema.seam_candidate_angle_threshold_deg]` (matches classic's at `classic-perimeters.toml:93-99`): `type = "float"`, `default = 30.0`, `min = 0.0`, `max = 180.0`, `display = "Seam candidate sharp-corner angle threshold (degrees)"`, `group = "Seam"`.
  - `modules/core-modules/arachne-perimeters/src/lib.rs`:
    - Add `use slicer_core::perimeter_utils::{generate_sharp_corner_seam_candidates, point_in_any_polygon, WallSequence};` at the top of the imports.
    - `classify_line` (206-214): add a third arm for `LoopType::ThinWall`; the function returns `(ExtrusionRole, LoopType)` so `ExtrusionRole` for thin walls is `ExtrusionRole::ThinWall` (matches classic's `lib.rs:765`).
    - `arachne_params_from_config` (106-197): add `let precise_outer_wall = config.get_bool("precise_outer_wall").unwrap_or(false);` and `let seam_candidate_angle_threshold_deg = config.get_float("seam_candidate_angle_threshold_deg").unwrap_or(30.0);`; thread them into the `ArachneParams` struct via a parallel widening.
    - **AC-8 rewrite (precise_outer_wall)**: compute the offset magnitude `let precise_outer_wall_offset = if precise_outer_wall && matches!(wall_sequence, WallSequence::InnerOuter) { -(outer_wall_line_width / 2.0 - outer_wall_line_spacing / 2.0) } else { 0.0 };`; set `params.outer_wall_offset = precise_outer_wall_offset;` before passing `params` to `generate_arachne_walls(...)`. This is the beading-stack mechanism — the inset is realized inside the beading strategy (via `BeadingStrategyFactory::makeStrategy(..., outer_wall_offset, ...)` mirroring OrcaSlicer's `OuterWallInsetBeadingStrategy::compute` at `Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60`). It is NOT a post-hoc `wall.path` mutation. The classic precedent at `classic-perimeters/src/lib.rs:176-178, 545, 712` uses a different mechanism (it redefines `ext_perimeter_spacing2` for the classic medial-axis path), so the precedent cited in the original spec is misleading — the Arachne precedent is the beading-stack mechanism, which the arachne module already supports via `ArachneParams.outer_wall_offset` (wired at `arachne-perimeters/src/lib.rs:157`). No new IR fields are required.
    - `run_perimeters` (236-352):
      - In the `for region in regions` loop, after `let polygons = region.polygons();`, add `let bridge_areas: &[ExPolygon] = region.bridge_areas();` and `let overhang_bands: &[QuartileBand] = region.overhang_quartile_polygons();` (one allocation per region, reused across all lines).
      - In the `for line in &lines` loop (284-304), replace the `feature_flags: vec![WallFeatureFlags::default(); num_points]` line with a per-vertex construction: for each path point, compute `is_bridge = point_in_any_polygon(pt, bridge_areas)`; for thin-wall loops, `is_thin_wall = true` on every vertex; otherwise `is_thin_wall = false`.
      - In the same `for line in &lines` loop, replace `boundary_type: WallBoundaryType::Interior` with a conditional: `if line.inset_idx == 0 { WallBoundaryType::ExteriorSurface } else { WallBoundaryType::Interior }`.
      - In the same loop, for each path point, look up the overhang quartile from `overhang_bands` (a `&[QuartileBand]` where each `QuartileBand` has `quartile: u8, polygons: &[ExPolygon]`). Walk the bands in order; the first band whose polygons contain the point determines the quartile. If no band contains the point, `overhang_quartile = None`. This mirrors the classic path's `expolygon_to_path3d` logic (`perimeter_utils.rs:316-331`). The per-vertex lookup operates in mm against `overhang_bands` (mm via `SliceRegionView`); the helper `point_in_polygon_winding` lives in `crates/slicer-ir/src/polygon_predicate.rs` and is wasm-compatible.
      - **AC-6 seam-candidate emission rewrite**: After the `for wall in walls { output.push_wall_loop(wall)?; }` loop (314-316), add a new loop: for the outer wall only (the input polygon's outermost contour, identified by `region.polygons()[0]`), call `let candidates = generate_sharp_corner_seam_candidates(&region.polygons()[0].contour, region.z(), seam_candidate_angle_threshold_deg);` and `for c in candidates { output.push_seam_candidate(c.position, c.score)?; }`. The helper takes `&slicer_ir::Polygon` (units-space, the **input region contour**) — NOT `&wall.path` (mm-space `ExtrusionPath3D`); the type bridge is `region.polygons()[0].contour` directly, which is the same call shape as classic's `lib.rs:889-900`.
  - `modules/core-modules/arachne-perimeters/tests/` — new unit-test files (one per rewritten arachne-path red test):
    - `arachne_parity_outer_wall_boundary_type_tdd.rs` — AC-1
    - `arachne_parity_thin_wall_loop_type_tdd.rs` — AC-2
    - `arachne_parity_is_thin_wall_flag_tdd.rs` — AC-3
    - `arachne_parity_is_bridge_flag_tdd.rs` — AC-4
    - `arachne_parity_overhang_quartile_tdd.rs` — AC-5
    - `arachne_parity_seam_candidate_tdd.rs` — AC-6
    - `arachne_parity_precise_outer_wall_manifest_tdd.rs` — AC-7
    - `precise_outer_wall_tdd.rs` — AC-8 (beading-stack offset behavior, plus AC-N2 default-off)
    - Each file builds a `SliceRegionView` via `SliceRegionViewBuilder`, a `ConfigView` via `ConfigViewBuilder`, constructs `PerimeterOutputBuilder::new()`, calls `ArachnePerimeters.run_perimeters(0, &[region], &PaintRegionLayerView::new(0), &mut output, &config)`, and asserts on `output.wall_loops()` / `output.seam_candidates()`. The harness is identical to `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs` (the existing precedent for this pattern).
  - `crates/slicer-runtime/tests/arachne_parity.rs` — REWRITE the 7 arachne-path red tests to delegate to the new `arachne-perimeters/tests/` files (or delete the redundant substring tests and let the `arachne-perimeters/tests/` files be the canonical coverage). The 3 packet-1 stale-doc tests, 4 packet-149 pipeline-config tests, and 1 D-104f test are preserved with their existing predicates (they are manifest-presence / wiring-presence concerns and are correct as-is).
  - `docs/DEVIATION_LOG.md`:
    - Refine `D-104-OVERHANG-QUARTILE-NONE` rationale from "Packet 104 T-024, updated by Packet 107. `Point3WithWidth.overhang_quartile` is left `None` at all construction sites" to "Arachne-path-only per-vertex overhang/flag/seam/boundary parity. The classic path is at parity via T-024, T-077, classic classify_line, and `expolygon_to_path3d` (perimeter_utils.rs:316-331). The arachne path's gaps — `is_bridge` (lib.rs:301), `is_thin_wall` (never set), `LoopType::ThinWall` never emitted (classify_line at 206-214), `boundary_type` hardcoded `Interior` (lib.rs:302), `overhang_quartile` hardcoded `None` (defaults inherited from `generate_toolpaths.rs:184,471,866` and overridden per-vertex by the guest), no seam-candidate producer, no `precise_outer_wall` registration — are closed by packet 148."
  - `docs/14_deviation_audit_history.md`:
    - Append: `| 2026-07-09 | D-104-OVERHANG-QUARTILE-NONE | Packet 148 refined scope from pipeline-wide to arachne-path-only per-vertex overhang/flag/seam/boundary parity. |`
  - `docs/15_config_keys_reference.md`:
    - Append `precise_outer_wall` and `seam_candidate_angle_threshold_deg` to the Walls section table.
- **Rejected alternatives:**
  - One packet per gap (7 packets): rejected — the seven gaps share two code sites; a single refactor pass is cheaper to review and easier to keep consistent with the classic path.
  - Adding the helpers to `slicer-sdk` re-exports instead of adding `slicer-core` to the arachne Cargo.toml: rejected — `slicer-sdk` is the WIT-boundary surface; mixing in geometry helpers from `slicer-core` would blur that boundary. The classic module already imports `slicer_core::perimeter_utils` directly, so the arachne module doing the same is the consistent choice. (Note: the helpers are reachable from the guest because `slicer-core` itself compiles to `wasm32-unknown-unknown` with default features; `host-algos` is opt-in, and the pin `default-features = false` is the safety net.)
  - Implementing the overhang-quartile lookup as a separate `for point in path.points { ... }` pass before the construction loop: rejected — the lookup is per-vertex; doing it inside the existing `for line in &lines` loop is one pass instead of two.
  - **Post-hoc path-offset mutation for precise_outer_wall (the original spec's AC-8)**: rejected — OrcaSlicer's arachne path uses the beading-stack mechanism (`OuterWallInsetBeadingStrategy::compute` at `Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60`), and the PnP arachne module already wires `ArachneParams.outer_wall_offset` into the beading stack (lib.rs:157). Gating `outer_wall_offset` on `precise_outer_wall && wall_sequence==InnerOuter` is the canonical mechanism. Post-hoc path mutation would diverge from OrcaSlicer and from PnP's existing beading-stack wiring. (OrcaSlicer has no unit test for the offset magnitude; the new PnP unit test is the canonical verification.)
  - **Keeping the existing `arachne_parity.rs` substring-matching tests as the AC verification**: rejected — substring matches cannot verify real `WallLoop` output; they would let a façade (a comment with the right strings) pass. The full rewrite to drive `run_perimeters` natively is the only way the ACs verify real behavior.
  - **Editing the host pipeline (`generate_toolpaths.rs`) to set `overhang_quartile` / `flow_factor`**: rejected — the host pipeline's hardcoded `None` / `1.0` are the pre-region baseline; the guest module's per-vertex override is the correct site. The rewritten tests assert on the guest's `WallLoop.path.points`, not on the host's `ExtrusionLine.junctions`.

## Files in Scope (read + edit)

- `modules/core-modules/arachne-perimeters/Cargo.toml` — role: module dependencies; expected change: `slicer-core` with `default-features = false`.
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — role: module manifest; expected change: two new `[config.schema.*]` sections.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: the arachne module's per-region/per-line loop; expected change: classify_line gains ThinWall arm; run_perimeters populates feature_flags + boundary_type + overhang_quartile; emits seam candidates for outer wall; arachne_params_from_config gates outer_wall_offset on precise_outer_wall.
- `modules/core-modules/arachne-perimeters/tests/arachne_parity_*_tdd.rs` and `precise_outer_wall_tdd.rs` — role: native unit tests for the 7 rewritten arachne-path ACs + AC-8 + AC-N2.
- `crates/slicer-runtime/tests/arachne_parity.rs` — role: audit test suite; expected change: rewrite the 7 arachne-path tests to drive `run_perimeters` natively (the 3 stale-doc + 4 pipeline-config + 1 D-104f tests are preserved as-is).
- `docs/DEVIATION_LOG.md` — role: canonical deviation table; expected change: D-104 rationale refined.
- `docs/14_deviation_audit_history.md` — role: deviation audit log; expected change: one row appended.
- `docs/15_config_keys_reference.md` — role: config key reference; expected change: two rows appended to the Walls section.

## Read-Only Context

- `crates/slicer-core/src/perimeter_utils.rs:316-331` — `expolygon_to_path3d` overhang-quartile lookup; read to understand the canonical lookup pattern.
- `crates/slicer-core/src/perimeter_utils.rs:460` — `generate_sharp_corner_seam_candidates` signature.
- `crates/slicer-core/src/perimeter_utils.rs:608` — `point_in_any_polygon` signature.
- `crates/slicer-core/src/perimeter_utils.rs:194-195` — `build_wall_flags` ExteriorSurface logic (the classic-path precedent for AC-1).
- `modules/core-modules/classic-perimeters/src/lib.rs:675-678` — per-vertex `is_bridge` assignment (the classic-path precedent for AC-4).
- `modules/core-modules/classic-perimeters/src/lib.rs:765, 772, 783-790` — ThinWall loop type and `is_thin_wall` flag (the classic-path precedent for AC-2/AC-3).
- `modules/core-modules/classic-perimeters/src/lib.rs:889-900` — seam-candidate emission (the classic-path precedent for AC-6).
- `modules/core-modules/arachne-perimeters/src/lib.rs:157` — the existing `ArachneParams.outer_wall_offset` wiring (the precedent for AC-8's beading-stack mechanism).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60` — OrcaSlicer's beading-stack precise_outer_wall mechanism (delegate; the AC-8 implementation mirrors this).
- `crates/slicer-sdk/src/views.rs:388, 440, 468` — `bridge_areas`, `overhang_areas`, `overhang_quartile_polygons` accessors.
- `crates/slicer-ir/src/slice_ir.rs:1520-1533, 1796-1808` — `WallFeatureFlags` and `WallLoop` field shapes.
- `crates/slicer-ir/src/polygon_predicate.rs:41-47` — `point_in_polygon_winding` and `point_in_contour_winding` (wasm-compatible helpers; the per-vertex overhang-band lookup uses these).
- `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs:1-50` — the harness pattern for driving `run_perimeters` natively (the precedent for the 7 new unit tests).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-runtime/src/run.rs` — out of scope; the arachne module is invoked from `run.rs` but `run.rs` does not need to change.
- `crates/slicer-core/src/arachne/pipeline.rs` — out of scope; the host service bridge already returns `ExtrusionLine`s correctly. The per-vertex `overhang_quartile` / `flow_factor` override happens in the guest module AFTER `extrusion_line_to_extrusion_path3d` copies the host's defaults.
- `crates/slicer-core/src/beading/*` — out of scope; the beading-strategy stack already computes bead counts correctly. The `outer_wall_offset` threading flows through the existing `ArachneParams` plumbing (wired at `arachne-perimeters/src/lib.rs:157`); no beading-stack edits are required.
- `crates/slicer-wasm-host/src/host.rs` — out of scope; the WIT host-service surface is unchanged.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output.log`; return FACT (pass count vs expected ≥ 10) and the failing-test detail block (≤ 20 lines) on any failure." — purpose: AC-9.
- "Run `cargo test -p arachne-perimeters --tests 2>&1 | tee target/test-output.log`; return FACT (all 7+ unit tests green) and the failing-test detail block (≤ 20 lines) on any failure." — purpose: per-AC unit-test verification.
- "Run `cargo build -p arachne-perimeters --target wasm32-unknown-unknown 2>&1 | tee target/wasm-build.log`; return FACT (pass) or SNIPPETS (first 20 lines of error)." — purpose: confirm the `slicer-core` dep with `default-features=false` builds to wasm.
- "Run `cargo xtask build-guests --check 2>&1 | tee target/guest-check.log`; return FACT (Fresh/STALE)." — purpose: confirm the manifest change doesn't leave the arachne guest stale.
- "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60`; return SUMMARY (≤ 200 words) of the precise_outer_wall beading-stack mechanism." — purpose: confirm the AC-8 offset formula mirrors OrcaSlicer.
- "Run `rg -q 'config\.schema\.(detect_overhang_wall|overhang_reverse|overhang_reverse_internal_only|min_width_top_surface|alternate_extra_wall|bridge_flow|thick_bridges)' modules/core-modules/arachne-perimeters/arachne-perimeters.toml; echo $?`; return FACT (exit code 1 = pass for AC-N1)." — purpose: manifest-drift guard.

## Data and Contract Notes

- **IR or manifest contracts touched:**
  - `WallLoop.boundary_type` — already an enum variant in `slicer_ir` (`WallBoundaryType::{Interior, ExteriorSurface, ...}`). No new variant.
  - `WallLoop.feature_flags: Vec<WallFeatureFlags>` — already per-vertex. No shape change.
  - `WallLoop.loop_type` — already an enum; `LoopType::ThinWall` already exists. No new variant.
  - `Point3WithWidth.overhang_quartile: Option<u8>` — already present. No shape change.
  - `Point3WithWidth.flow_factor: f32` — already present. No shape change.
  - `arachne-perimeters.toml [config.schema]` — two new entries: `precise_outer_wall` (bool, default false) and `seam_candidate_angle_threshold_deg` (float, default 30.0, range 0.0..=180.0). Both match classic's manifest entries byte-for-byte.
  - `ArachneParams.outer_wall_offset` — already a field; this packet only GATES its value on `precise_outer_wall && wall_sequence==InnerOuter`. No field additions.
- **WIT boundary considerations:** none. The arachne module's output type `WallLoop` is host-internal; it does not cross a WIT boundary as a guest input. The two new config keys are read from `ConfigView` (the in-memory config representation), not from WIT. The seam-candidate output is emitted via the SDK's `output.push_seam_candidate(pos, score)` (which is the `PerimeterOutputBuilder` method, also host-internal at this stage).
- **Determinism or scheduler constraints:** none beyond what classic-perimeters already enforces. The seam-candidate emission is deterministic for a given input polygon (the helper `generate_sharp_corner_seam_candidates` is pure).

## Locked Assumptions and Invariants

- The two new config keys (`precise_outer_wall`, `seam_candidate_angle_threshold_deg`) MUST have **identical** defaults and ranges to classic's manifest entries. The test for AC-7 grep-asserts the manifest TOML; the implementer should `diff` against classic's manifest before committing.
- The `is_bridge` flag MUST be set per-vertex, NOT per-line. A whole-line `is_bridge = true` is the wrong shape and would fail AC-4 (the rewritten test reads per-vertex).
- The `is_thin_wall` flag MUST only be set on `LoopType::ThinWall` walls, NEVER on `Outer`/`Inner` walls that happen to be narrow. The rewritten AC-3 test asserts the shape lock; the implementer should not over-broaden the flag to all narrow walls.
- The `overhang_quartile` lookup MUST be a per-vertex point-in-polygon against `region.overhang_quartile_polygons()` bands, NOT against `region.overhang_areas()`. The latter is the un-banded overhang footprint; the former is the banded classification. The rewritten AC-5 test asserts the banded shape.
- The seam-candidate emission MUST be limited to the outer wall (the input polygon's outermost contour, accessed via `region.polygons()[0]`). Emitting seam candidates for inner walls would be wrong (the seam-placer reads them only for the outer wall). The seam helper `generate_sharp_corner_seam_candidates` takes a `&slicer_ir::Polygon` (units-space input contour), NOT `&wall.path` (mm-space `ExtrusionPath3D`) — the call shape is `&region.polygons()[0].contour`, mirroring classic's `lib.rs:889-900`.
- The `outer_wall_offset` MUST be applied ONLY when `precise_outer_wall && wall_sequence == InnerOuter`. The `OuterInner` and `InnerOuterInner` sequences do not have the same offset semantics (OrcaSlicer's `OuterWallInsetBeadingStrategy` is gated on `wall_sequence == InnerOuter`).
- None — change is reversible via existing config defaults (precise_outer_wall defaults to false; the seam threshold defaults to 30°; classify_line's ThinWall arm only fires when print_thin_walls is on); no behavior locks introduced beyond the test suite.

## Risks and Tradeoffs

- **Risk:** adding `slicer-core` to `arachne-perimeters/Cargo.toml` increases the module's dependency surface. **Mitigation:** classic-perimeters already has this dep; the increase is zero (same crate path). The `default-features = false` pin ensures `host-algos` (voronoi/rayon) is never enabled on the guest.
- **Risk:** the per-vertex overhang-band lookup in the construction loop adds O(num_points × num_bands × polygon_complexity) to the per-region wall generation. For typical models this is negligible (≤ 1000 points × 4 bands × 100 vertices = 400k operations per region per layer). For pathological cases (large overhang areas with many bands), it could be measurable. **Mitigation:** the classic path does the same lookup at `expolygon_to_path3d:316-331` and no regression has been logged. If a perf issue surfaces, the lookup can be hoisted to a precomputed per-region `Vec<Option<u8>>` keyed by point.
- **Risk:** the seam-candidate emission introduces a new host-service call site (`output.push_seam_candidate`). The host may not yet route the seam candidates correctly if `seam-placer` is wired in a packet that hasn't landed. **Mitigation:** the existing classic path's `push_seam_candidate` is in production; the host's routing is already correct. The new call site is symmetric.
- **Risk:** the test rewrite is a substantial chunk of work — 7 unit-test files plus the `arachne_parity.rs` rewrite. **Mitigation:** the harness pattern is already established by `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`; each new unit test is a small variation on that pattern. The work is parallelizable and can be dispatched to a sub-agent.

## Context Cost Estimate

- Aggregate (sum across all steps): M (7 steps × S/S/S/M/M/S/S).
- Largest single step: M (Step 4: feature_flags + boundary_type + overhang_quartile in the construction loop; touches multiple IR fields and bridges). Step 5 (seam emission) is also M; Step 6 (test rewrite, 7 new unit tests) is M.
- Highest-risk dispatch: the OrcaSlicer parity check (precise_outer_wall beading-stack mechanism) — its return shape is the "is my AC-8 mechanism correct" gate, and a poorly-shaped dispatch (asking for the whole C++ file) blows budget. The dispatch contract must be: "Summarize the canonical beading-stack mechanism at `OuterWallInsetBeadingStrategy.cpp:44-60`; return SUMMARY ≤ 200 words."

## Open Questions

- None. All forward-flagged open questions are resolved by this refined design.
