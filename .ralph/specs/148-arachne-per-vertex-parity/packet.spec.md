---
status: draft
packet: 148-arachne-per-vertex-parity
task_ids:
  - none
backlog_source: tmp/arachne_parity_audit_20260709.md
context_cost_estimate: M
---

# Packet Contract: 148-arachne-per-vertex-parity

## Goal

Close the seven arachne-path gaps (G7, G10, G12, G18, G19, G20, G21 in the audit) by populating `WallLoop.boundary_type`, `WallLoop.loop_type`, per-vertex `WallFeatureFlags.is_bridge`/`is_thin_wall` and per-vertex `Point3WithWidth.overhang_quartile` in the arachne guest module's `run_perimeters`, emitting seam candidates for the outer wall, and gating `ArachneParams.outer_wall_offset` on `precise_outer_wall && wall_sequence==InnerOuter` (the beading-stack mechanism — mirrors OrcaSlicer's `OuterWallInsetBeadingStrategy::compute`, NOT a post-hoc path mutation). The audit's `arachne_parity.rs` red tests are REWRITTEN to drive the guest module's `run_perimeters` natively (no WASM, no `run_arachne_pipeline`, no source-text substring matching) so the ACs verify real `WallLoop` output.

## Scope Boundaries

This packet lands the seven arachne-path gaps by editing `modules/core-modules/arachne-perimeters/src/lib.rs`, `arachne-perimeters.toml`, and `arachne-perimeters/Cargo.toml`; rewrites the corresponding 7 arachne-path red tests in `crates/slicer-runtime/tests/arachne_parity.rs` to drive `run_perimeters` natively and asserts on real `output.wall_loops()` / `output.seam_candidates()`; and refines D-104's scope. It does NOT touch the host service bridge (`slicer_core::arachne::pipeline::run_arachne_pipeline`) or the beading-strategy stack — those are upstream of this module and already correct. Pipeline-wide config gaps (G8, G9, G14, G16) live in packet 149.

## Prerequisites and Blockers

- Depends on: Packet 1 (stale doc fixes; landed). P112 wall generation (landed; `arachne-perimeters` already produces real walls). P107 overhang-accessor delivery (landed; `SliceRegionView::overhang_quartile_polygons()` and `SliceRegionView::bridge_areas()` available).
- Unblocks: Packet 149 (per-vertex `is_bridge` set by 148 is the precondition for 149's D4 bridge-flow helper).
- Activation blockers: none. Each of the 7 red tests has an independent runnable acceptance criterion after the test rewrite.

## Acceptance Criteria

- **AC-1. Given** a 10 mm square fixture built via `SliceRegionViewBuilder` (no bridge areas, no overhang bands) and the `ArachnePerimeters` struct, **when** the test calls `module.run_perimeters(0, &[region], &PaintRegionLayerView::new(0), &mut PerimeterOutputBuilder::new(), &config)` and inspects `output.wall_loops()`, **then** the wall loop with `perimeter_index == 0` has `boundary_type == WallBoundaryType::ExteriorSurface`. | `cargo test -p arachne-perimeters --test arachne_parity_outer_wall_boundary_type_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-2. Given** a 0.25 mm × 5 mm thin strip fixture built with `SliceRegionViewBuilder` and a `ConfigView` containing `detect_thin_wall=true`, **when** the test calls `run_perimeters` and inspects `output.wall_loops()`, **then** at least one emitted wall has `loop_type == LoopType::ThinWall`. | `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-3. Given** the same thin-strip fixture as AC-2, **when** the test calls `run_perimeters` and inspects `output.wall_loops()[i].feature_flags`, **then** at least one entry has `is_thin_wall == true`, set on `LoopType::ThinWall` walls only (not on `Outer`/`Inner` even when those walls happen to be narrow). | `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-4. Given** a 10 mm square fixture with `SliceRegionViewBuilder` and a `SliceRegionView` whose `bridge_areas` contains a 4 mm × 4 mm polygon at the center, **when** the test calls `run_perimeters` and inspects `output.wall_loops()[i].feature_flags[j].is_bridge`, **then** the vertices whose path point lies inside the bridge area have `is_bridge == true`, reflecting `point_in_any_polygon(pt, region.bridge_areas())`. | `cargo test -p arachne-perimeters --test arachne_parity_is_bridge_flag_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-5. Given** a 10 mm square fixture with `SliceRegionViewBuilder` and `SliceRegionView` whose `overhang_quartile_polygons` contains a 4 mm × 4 mm `QuartileBand { quartile: 3, polygons: [...] }`, **when** the test calls `run_perimeters` and inspects `output.wall_loops()[i].path.points[j].overhang_quartile`, **then** the vertices inside the band have `overhang_quartile == Some(3)`, populated from `region.overhang_quartile_polygons()` via the band lookup (mirrors `expolygon_to_path3d` at `crates/slicer-core/src/perimeter_utils.rs:316-331`). | `cargo test -p arachne-perimeters --test arachne_parity_overhang_quartile_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-6. Given** a 10 mm square fixture, **when** the test calls `run_perimeters` and inspects `output.seam_candidates()`, **then** the result is non-empty and each candidate's `position` is one of the outer-wall input polygon's corner points. The seam helper `generate_sharp_corner_seam_candidates(&Polygon, z, threshold_deg)` takes a units-space `Polygon` (the input region contour), NOT `&ExtrusionPath3D` (mm-space wall path) — the call site uses `region.polygons()[0].contour` (mirrors classic's `lib.rs:889-900`). | `cargo test -p arachne-perimeters --test arachne_parity_seam_candidate_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-7. Given** the arachne manifest, **when** the test reads the parsed TOML, **then** it contains both `[config.schema.precise_outer_wall]` and `[config.schema.seam_candidate_angle_threshold_deg]` sections with defaults `false` and `30.0` respectively (matches classic's manifest entries byte-for-byte; the seam key's range is `0.0..=180.0`). | `cargo test -p arachne-perimeters --test arachne_parity_precise_outer_wall_manifest_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q '^test result: ok' target/test-output.log`
- **AC-8. Given** a 10 mm square fixture with `ConfigView` containing `precise_outer_wall=true` and `wall_sequence=InnerOuter` resolved from `ArachneParams`, **when** the test calls `run_perimeters` and inspects the outermost wall loop's path, **then** the wall's distance to the input polygon boundary equals `-(ext_perimeter_width/2 - ext_perimeter_spacing/2)`, realized by gating `ArachneParams.outer_wall_offset` on `precise_outer_wall && wall_sequence==InnerOuter` (mirrors OrcaSlicer's `OuterWallInsetBeadingStrategy::compute` mechanism — the beading stack applies the inset at `BeadingStrategyFactory::makeStrategy`, NOT a post-hoc path mutation; OrcaSlicer has no unit test for the offset magnitude, so PnP's unit test is the canonical verification). | `cargo test -p arachne-perimeters --test precise_outer_wall_tdd 2>&1 | tee target/test-output.log | tail -5; grep -q 'test result: ok' target/test-output.log`
- **AC-9. Given** the full `arachne_parity.rs` test file (rewritten to drive `run_perimeters` natively), **when** the suite is run after this packet, **then** the 3 packet-1 stale-doc tests + 7 packet-148 arachne-path tests are green (10 total) and the 4 pipeline-config tests (G8, G9, G14, G16, packet 149 scope) and 1 deferred concentric-infill test (G23 / D-104f) stay red. | `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output.log | tail -5; grep -E '^test result' target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** the arachne manifest does NOT contain `[config.schema.detect_overhang_wall]`, `[config.schema.overhang_reverse]`, `[config.schema.overhang_reverse_internal_only]`, `[config.schema.min_width_top_surface]`, `[config.schema.alternate_extra_wall]`, `[config.schema.bridge_flow]`, or `[config.schema.thick_bridges]` (those are packet 149's scope), **when** the test parses the manifest, **then** those keys remain absent (no manifest drift into packet 149's scope). | `rg -q 'config\.schema\.(detect_overhang_wall|overhang_reverse|overhang_reverse_internal_only|min_width_top_surface|alternate_extra_wall|bridge_flow|thick_bridges)' modules/core-modules/arachne-perimeters/arachne-perimeters.toml; [ $? -ne 0 ] || (echo NEGATIVE FAIL: key from packet 149 scope leaked into arachne manifest && exit 1)`
- **AC-N2. Given** the arachne manifest's `[config.schema.precise_outer_wall].default` is `false` (matches classic's manifest), **when** a fixture is sliced with `precise_outer_wall=false` and `wall_sequence=InnerOuter`, **then** `ArachneParams.outer_wall_offset` is 0 (no beading-stack inset applied). The precise-outer-wall is opt-in, not default-on, and AC-8's offset must not be applied unconditionally. | `cargo test -p arachne-perimeters --test precise_outer_wall_tdd -- default_off 2>&1 | tee target/test-output.log | tail -5; grep -q 'test result: ok' target/test-output.log`

## Verification

- `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output.log` — 10 passed (3 stale-doc from packet 1 + 7 arachne-path rewritten from this packet), 5 still red (4 packet-149 + 1 D-104f).
- `cargo test -p arachne-perimeters --tests 2>&1 | tee target/test-output.log` — all 7 new unit tests in `arachne-perimeters/tests/` green.
- `cargo build -p arachne-perimeters --target wasm32-unknown-unknown 2>&1 | tee target/wasm-build.log` — confirms the `slicer-core` dep with `default-features=false` compiles to wasm (slicer-core's `host-algos` feature gates `boostvoronoi`/`rayon`; `default-features=false` is the safety pin).
- `cargo clippy -p slicer-runtime --test arachne_parity -- -D warnings 2>&1 | tee target/clippy-output.log` — clean.
- `cargo xtask build-guests --check 2>&1 | tee target/guest-check.log` — Fresh (manifest change rebuilds the arachne guest).

## Authoritative Docs

- `docs/02_ir_schemas.md` §1418-1428 (WallBoundaryType enum) and §1520-1533 (WallFeatureFlags) and §1505-1516 (LoopType::ThinWall) and §1542-1558 (Point3WithWidth.overhang_quartile) — direct read by the implementer (delegate the surface summary if you load the whole file, which is > 300 lines).
- `docs/05_module_sdk.md` §"SliceRegionView accessors (packet 107)" (overhang_quartile_polygons; bridge_areas added by packet 36/37) — direct read.
- `docs/15_config_keys_reference.md` — load only the "Arachne beading strategy stack" and "Walls" sections; do NOT load the full file.
- `docs/DEVIATION_LOG.md` D-104-OVERHANG-QUARTILE-NONE — direct read; refine the entry's scope to "arachne-path-only per-vertex overhang/flag/seam/boundary parity" at packet close.

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` §D-104-OVERHANG-QUARTILE-NONE — `rg -q 'arachne-path-only per-vertex overhang' docs/DEVIATION_LOG.md` (verify the scope-refined rationale landed).
- `docs/14_deviation_audit_history.md` — append a one-line record of the D-104 scope refinement.
- `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` §`[config.schema.precise_outer_wall]` and §`[config.schema.seam_candidate_angle_threshold_deg]` — manifest entries are the primary doc surface for new config keys.
- `docs/15_config_keys_reference.md` §"Walls" — append `precise_outer_wall` and `seam_candidate_angle_threshold_deg` to the arachne-keys table (grep anchor: `'seam_candidate_angle_threshold_deg'`) — `rg -q 'seam_candidate_angle_threshold_deg' docs/15_config_keys_reference.md`.
- `crates/slicer-runtime/tests/arachne_parity.rs` — REWRITTEN to drive `ArachnePerimeters::run_perimeters` natively via `PerimeterOutputBuilder::new()` + `SliceRegionViewBuilder` + `ConfigViewBuilder` + `PaintRegionLayerView::new(0)`. The 7 arachne-path tests assert on `output.wall_loops()` / `output.seam_candidates()` instead of source-text substring matches. The 3 stale-doc tests (packet 1 scope) and the 4 pipeline-config tests (packet 149 scope) and the 1 D-104f test are preserved with their existing predicates.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `process_classic()` shape (perimeter_index assignment, wall ordering, is_bridge per-vertex at lines 675-678), `process_arachne()` shape (seam candidate emission at 2093-2535).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.hpp` and `WallToolPaths.cpp` — `WallBoundaryType::ExteriorSurface` provenance; `LoopType::ThinWall` emission at 783-790 equivalent.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/OuterWallInsetBeadingStrategy.cpp:44-60` — the beading-stack-level offset mechanism (the canonical precise_outer_wall mechanism for the Arachne path; PnP's `ArachneParams.outer_wall_offset` mirrors this).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1484-1489` (`precise_outer_wall` coBool; OrcaSlicer defaults to `true` but PnP defaults to `false` for parity with classic).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:7180-7193` (`wall_transition_filter_deviation`; the implementation exists, the manifest description was the gap closed in packet 1).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:
- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%
- treat the test rewrite (rewriting `crates/slicer-runtime/tests/arachne_parity.rs` to drive `run_perimeters` natively) as a first-class implementation step, not a polish item — the original substring-matching tests cannot verify the packet's behavior ACs

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
