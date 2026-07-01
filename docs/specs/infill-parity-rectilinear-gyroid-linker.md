# Infill Parity — Rectilinear, Gyroid, and the Infill-Linker Module

## Context

The PnP `rectilinear-infill` and `gyroid-infill` modules are LLM-generated Rust
stubs that implement a fraction of the canonical OrcaSlicer algorithms. A
gap analysis (2026-07-01) against `OrcaSlicerDocumented/src/libslic3r/Fill/`
found both modules missing the bulk of the scan-line / wave pipeline, with
several correctness bugs in what they do implement. This spec covers bringing
both to OrcaSlicer parity **under a new PnP-native architecture** (ADR-0025):
modules emit raw unlinked segments; a new `Layer::InfillPostProcess`
`infill-linker` module is the single place infill path connection happens.

This is **not** a 1:1 port. The PnP pipeline differs from OrcaSlicer in three
load-bearing ways (see "Pipeline differences" below), and ADR-0025 introduces a
fourth deliberate divergence (centralized linking). The goal is *output
parity* — the geometry a user gets — not *code-structure parity*.

## Authoritative References

### ADRs (the design decisions — read these first)
- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` — Architecture A: modules emit raw; linker connects.
- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — linking algorithms live in the linker, not `slicer-core`.
- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — gyroid can fill solid shells (opt-in).
- `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` — WIT contract change.

### OrcaSlicer canonical sources
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp` (4151 lines) — scan-line engine, link graph, traversal.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.hpp` — class hierarchy.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.cpp` (378 lines) — gyroid wave generation.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillGyroid.hpp` — gyroid class.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp` (3310 lines) — `connect_infill`, `chain_or_connect_infill`, `infill_direction`, `adjust_solid_spacing`, `multiline_fill`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.hpp` — `FillParams`, `Fill` state.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` (1756 lines) — `_fill_surface_single` dispatch, `FillParams` population, pattern selection.

### PnP codebase
- `modules/core-modules/rectilinear-infill/src/lib.rs` (361 lines) — current stub.
- `modules/core-modules/gyroid-infill/src/lib.rs` (695 lines) — current stub.
- `modules/core-modules/rectilinear-infill/rectilinear-infill.toml` — manifest (claims all four fill roles).
- `modules/core-modules/gyroid-infill/gyroid-infill.toml` — manifest (claims only `claim:sparse-fill`).
- `crates/slicer-sdk/src/traits.rs:320-393` — `run_infill` / `run_infill_postprocess` hooks.
- `crates/slicer-sdk/src/builders.rs:21-141` — `InfillOutputBuilder` (supports multi-point polylines).
- `crates/slicer-sdk/src/views.rs:19-483` — `SliceRegionView` (partitioned fill polygons).
- `crates/slicer-sdk/src/views.rs:490+` — `PerimeterRegionView` (lacks fill polygons — ADR-0028 adds them).
- `crates/slicer-core/src/polygon_ops.rs` — Clipper2 ops (available on wasm32; not `host-algos`-gated).
- `crates/slicer-runtime/src/region_partition.rs` — host-side wall-inset partition (no overlap applied).
- `crates/slicer-runtime/src/layer_executor.rs:1139-1156` — `Infill` merge vs `InfillPostProcess` replace.
- `crates/slicer-wasm-host/src/dispatch.rs:435-454` — `run_infill_postprocess` dispatch (empty builder).
- `crates/slicer-scheduler/src/execution_plan.rs:19-41` — `STAGE_ORDER` (includes `Layer::InfillPostProcess`).
- `crates/slicer-ir/src/resolved_config.rs:577-649` — infill config keys.
- `docs/08_coordinate_system.md` — 1 unit = 100 nm; porting checklist.

## Pipeline differences (PnP vs OrcaSlicer)

These are structural differences that make a 1:1 port impossible or undesirable.
They are not bugs; they are the PnP architecture.

1. **Host pre-partitions fill polygons.** `crates/slicer-runtime/src/region_partition.rs`
   partitions each region's wall-inset polygon into four pairwise-disjoint
   subsets (`sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`,
   `bridge_areas`) with precedence `bridge > bottom > top > sparse` at
   `Layer::Perimeters` commit. OrcaSlicer does this partitioning inside
   `PrintObject::prepare_infill`. PnP modules receive pre-partitioned polygons
   and emit over them directly — no per-region role-pick, no polygon math. The
   module does NOT re-derive top/bottom/sparse.

2. **Per-role fill-holder dispatch.** `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`
   (default `"rectilinear-infill"`) routes each role to a module. A single
   module may hold multiple fill-role claims. OrcaSlicer selects the pattern
   per-role via `top_surface_pattern` / `bottom_surface_pattern` etc. inside
   `Fill.cpp:941-959`; PnP does it via config-key-driven dispatch.

3. **Coordinate system.** PnP uses 1 unit = 100 nm (i64); OrcaSlicer uses 1 nm.
   Divide OrcaSlicer constants by 100. `ExtrusionPath3D` uses mm (f32);
   `ExPolygon` uses integer units. See `docs/08_coordinate_system.md` for the
   full porting checklist.

4. **Centralized linking (ADR-0025).** PnP modules emit raw unlinked segments;
   a dedicated `Layer::InfillPostProcess` linker module connects them. OrcaSlicer
   links inside each fill class. This is the deliberate divergence this effort
   introduces.

## Scope

### In scope
- **Rectilinear base pattern only.** The core scan-line engine
  (`fill_surface_by_lines`, `slice_region_by_vertical_lines`) producing raw
  2-point segments. NOT the Grid / Triangles / Stars / Cubic / QuarterCubic /
  ZigZag / CrossZag / LockedZag / Monotonic / MonotonicLines / Aligned /
  LateralHoneycomb / LateralLattice / SupportBase subclasses — those are
  separate selectable patterns in OrcaSlicer, out of scope. See "Out of
  scope" below.
- **Gyroid pattern.** The wave-generation core (`gyroid_f`, `make_one_period`,
  `make_wave`, orientation choice, z-phase) producing raw wave polylines.
- **The infill-linker module.** New `Layer::InfillPostProcess` module that
  links all infill modules' raw output.
- **The WIT contract change (ADR-0028).** `run_infill_postprocess` takes prior
  `InfillIR`; `PerimeterRegionView` gains the four partitioned fill polygons.
- **`clip_polylines` in `slicer-core::polygon_ops`.** A proper Clipper2
  polyline-vs-`ExPolygon` intersection operation (replaces gyroid's broken
  per-vertex ray-casting).
- **Gyroid multi-role claims (ADR-0027).** Add the three solid claims to
  `gyroid-infill.toml` so the existing emission code can fire when configured.

### Out of scope
- **Rectilinear subclasses** (Grid, Triangles, Stars, Cubic, QuarterCubic,
  ZigZag, CrossZag, LockedZag, Monotonic, MonotonicLines, Aligned,
  LateralHoneycomb, LateralLattice, SupportBase). These are separate selectable
  infill patterns that happen to subclass `FillRectilinear` in OrcaSlicer.
  The user's task is "rectilinear-infill" and "gyroid-infill" only.
- **Lightning-infill rewrite.** Lightning-infill (out of parity scope) currently
  self-links. Under Architecture A it should switch to raw emit, but that is a
  separate follow-up packet. Transitional state tracked in DEV-081.
- **`multiline_fill`.** Multi-extrusion / density > 100% offset. Low priority;
  only affects `fill_multiline > 1`. Deferred.
- **`fill_surface_trapezoidal`.** Experimental Orca-specific trapezoidal infill.
  Deferred.
- **Monotonic path.** Top-surface quality variant. Deferred (the rectilinear
  base pattern does not include monotonic ordering; a separate `Monotonic`
  module would declare it).

---

## Phase 0 — `clip_polylines` in `slicer-core::polygon_ops`

### What
Add a generic Clipper2 polyline-vs-`ExPolygon` intersection function to
`crates/slicer-core/src/polygon_ops.rs`. This replaces gyroid's broken
per-vertex ray-casting `clip_polyline_to_expolygon` and is the proper
equivalent of OrcaSlicer's `intersection_pl`.

### Signature (proposed)
```rust
/// Clip a set of polylines (each a `Vec<Point2>` in integer units) against an
/// `ExPolygon` set, returning the inside-sub-polylines. Uses Clipper2 line
/// intersection (not per-vertex point-in-polygon), so it correctly handles
/// boundary crossings between sample points.
pub fn clip_polylines(
    polylines: &[Vec<Point2>],
    clip: &[ExPolygon],
) -> Vec<Vec<Point2>>
```

### Implementation note
Verify `clipper2_rust` exposes a line-clipping API. If it does not, two
fallbacks:
1. Thicken each polyline into a thin polygon (offset by ±1 unit), intersect
   with the clip polygon, extract the centerline. Works but adds offset cost.
2. Implement a direct Sutherland-Hodgman variant for polyline clipping
   (segment-by-segment intersection with each polygon edge). Simpler, no
   Clipper2 dependency, but does not handle the ExPolygon hole structure as
   cleanly — would need separate contour/hole passes.

Pick at implementation time. The function must produce correct results for:
- Line fully inside the polygon → returned whole.
- Line fully outside → dropped.
- Line crossing the boundary once → split into inside + outside; inside returned.
- Line crossing the boundary twice (enter-exit-enter) → two inside segments returned.
- Line passing through a hole → split around the hole.
- Line along a polygon edge → inside (Clipper2 boundary rule).

### Tests (TDD)
- `line_fully_inside_returned_whole`
- `line_fully_outside_dropped`
- `line_crossing_once_split`
- `line_crossing_twice_two_segments`
- `line_through_hole_split_around_hole`
- `line_along_edge_inside`
- `multi_polyline_clip`
- `empty_input_returns_empty`

### Validation
```bash
cargo clippy -p slicer-core --all-targets -- -D warnings
cargo test -p slicer-core
```

---

## Phase 1 — WIT contract redesign (ADR-0028)

### What
Close the two contract gaps that make `Layer::InfillPostProcess` unusable as
the linker's home.

### Files touched
1. `crates/slicer-schema/wit/deps/ir-types.wit` — `perimeter-region-view` resource gains four fields: `sparse-infill-area`, `top-solid-fill`, `bottom-solid-fill`, `bridge-areas` (all `list<ex-polygon>`).
2. `crates/slicer-schema/wit/deps/world-layer/world-layer.wit` — `run-infill-postprocess` signature: either pre-populated builder (Option 1a) or new `prior-infill` input (Option 1b). Pick at implementation. Bump `world-layer@1.0.0` → `1.1.0`.
3. `crates/slicer-sdk/src/views.rs` — `PerimeterRegionView` struct gains the four fields + setters + accessors.
4. `crates/slicer-sdk/src/test_support/fixtures.rs` — `PerimeterRegionViewBuilder` gains setters for the four fields.
5. `crates/slicer-sdk/src/traits.rs:374-393` — `run_infill_postprocess` signature update (if Option 1b).
6. `crates/slicer-wasm-host/src/dispatch.rs:435-454` — populate `PerimeterRegionView` with partitioned polygons from `SliceIR`; pre-populate builder with prior `InfillIR` (Option 1a) or pass prior-IR as new input (Option 1b).
7. `crates/slicer-wasm-host/src/marshal/out.rs` — marshal new fields across the WIT boundary.
8. `crates/slicer-runtime/src/layer_executor.rs:1151-1156` — `LayerStageCommit::InfillPostProcess` changes from replace to merge (or stays replace if Option 1a's pre-populated builder makes the linker's output the full set).
9. `crates/slicer-ir/src/slice_ir.rs` — `InfillIR` schema version bump (minor, additive) if Option 1b.
10. `crates/slicer-macros/src/lib.rs` — bindgen glue for the new fields.
11. Every exhaustive match on `PerimeterRegionView` across the workspace (~30 files per the grep survey) — add the new fields.

### Per CLAUDE.md WIT/Type Changes Checklist
- Search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for `PerimeterRegionView` / `perimeter-region-view`.
- Verify type identity matches across component boundaries.
- Run `cargo build --tests` after WIT changes.
- Edit the canonical source at `crates/slicer-schema/wit/` (both host bindgen and guest macro read these).
- Run `cargo xtask build-guests --check` — MUST be clean before any test run touches guest WASM.
- Run `cargo xtask build-guests` (rebuild all guests).

### Tests
- `crates/slicer-runtime/tests/contract/` — add contract tests asserting the new `PerimeterRegionView` fields are populated at dispatch and readable by `run_infill_postprocess`.
- `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` — assert the new WIT types are present.
- `crates/slicer-sdk/tests/test_support_perimeter_region_view_builder_tdd.rs` — assert the builder setters work.

### Validation
```bash
cargo build --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask build-guests --check    # MUST be clean
cargo xtask build-guests             # rebuild all guests
cargo test -p slicer-sdk
cargo test -p slicer-runtime --test contract
```

---

## Phase 2 — Rectilinear infill rewrite (raw emit)

### What
Rewrite `modules/core-modules/rectilinear-infill/src/lib.rs` to emit raw
2-point scan-line segments over the wall-inset polygon (no linking, no
overlap offset, no `ExPolygonWithOffset` — all of that is the linker's job per
ADR-0025).

### Algorithm (per region, per role polygon)
Port from OrcaSlicer `FillRectilinear.cpp:2979-3143` (`fill_surface_by_lines`),
but stop before the link-graph and traversal stages (those are the linker's).

1. **`infill_direction`** (port FillBase.cpp:352-391): resolve angle + reference
   point. Priority: bridge_angle > per-layer rotation > static `base_angle`.
   Add π/2 (infill lines are perpendicular to the angle). Use
   `bounding_box.center()` as the reference point. Returns `(angle_rad, ref_pt)`.
   - Note: this helper lives in the rectilinear module (not `slicer-core` per
     ADR-0026; it is infill-specific). The linker does not need it.

2. **Float-rotate the role's `ExPolygon` by `-angle`** (f64 cos/sin → round to
   i64). The rotation rounding is ≤ 50 nm, below the 100 nm resolution floor
   and the 0.001 mm/layer contract (`docs/08_coordinate_system.md`). Use the
   existing `rotate_point` pattern but verify the rounding.

3. **Scan-line intersection** (port `slice_region_by_vertical_lines`,
   FillRectilinear.cpp:842-1154, but single-level — no `ExPolygonWithOffset`
   since overlap is the linker's job):
   - Cast N vertical scan lines through the rotated polygon at `x = min_x + i * line_spacing`.
   - `line_spacing = spacing / density` in integer units. `spacing` comes from
     `line_width` (config) via `mm_to_units`. For solid roles, call
     `adjust_solid_spacing` (port FillBase.cpp:326-340) so an integer number of
     lines fills the width exactly.
   - For each polygon edge, compute the y-intersection with each scan line using
     **integer arithmetic with a half-open edge test** (edge included at
     `min_y`, excluded at `max_y`). This handles vertices correctly without
     rational arithmetic: a scan line through a vertex intersects the edge
     whose `min_y == scan_y` but not the edge whose `max_y == scan_y`, avoiding
     double-count.
   - Classify intersections as the scan line enters/exits the polygon. With a
     single-level (no offset), classification is simple: sort intersections by
     y, pair them as (enter, exit), emit a 2-point segment per pair.

4. **Emit raw 2-point segments** as `ExtrusionPath3D` with `points: [start, end]`
   (rotate back by `+angle` first), `role` (SparseInfill / TopSolidInfill /
   BottomSolidInfill / InternalSolidInfill / BridgeInfill per the role-polygon),
   `speed_factor` (from config). Push via `push_sparse_path` or `push_solid_path`
   per the role.

5. **Per-`ExPolygon` scan conversion.** The current code collects all edges
   from all expolygons into a flat list and does one global intersection sort
   (lib.rs:231-237) — this produces incorrect results when expolygons overlap
   or are nested. The rewrite processes each `ExPolygon` independently.

### What stays from the current stub
- The four-role emission structure (sparse / top / bottom / bridge over the
  pre-partitioned polygons) — the host partition contract is correct.
- `solid_fill_role` mapping (depth-0 = exposed, deeper = InternalSolidInfill).
- `should_emit` gating.
- Config reading (`infill_density`, `infill_angle`, `infill_speed`,
  `line_width`).
- The `#[slicer_module]` macro and `LayerModule` impl.

### What is deleted
- `fill_expolygon_multi` (the global-edge-merge scan-line) — replaced.
- `collect_edges` — replaced by per-ExPolygon scan.
- The 2-point segment emission stays but is now correct (half-open test,
  per-ExPolygon, correct angle).

### What is NOT added (deferred to linker)
- `ExPolygonWithOffset` (two-level offset) — linker applies overlap.
- `connect_segment_intersections_by_contours` — linker builds the link graph.
- `traverse_graph_generate_polylines` — linker traverses.
- `connect_infill` / `chain_or_connect_infill` — linker connects.
- `INFILL_OVERLAP_OVER_SPACING` — linker applies.
- `pattern_shift` — linker handles layer continuity (or the module passes
  enough info for the linker to derive it; TBD at implementation).

### `pattern_shift` open question
OrcaSlicer applies `pattern_shift` (FillRectilinear.cpp:3023-3024) to
`refpt.x()` for layer-to-layer continuity. Under Architecture A, the module
emits raw segments; the linker connects them. Whether `pattern_shift` lives
in the module (scan lines are shifted per layer) or the linker (connection
pattern shifts per layer) is an implementation detail. Recommended: the
module applies `pattern_shift` to the scan-line starting x so adjacent layers'
segments interleave; the linker connects whatever segments it receives.

### Tests (TDD)
- `square_10mm_density_20_emits_n_raw_segments` — assert segment count matches
  `bb_height / line_spacing`, each segment is 2 points, endpoints on the
  wall-inset boundary, no linking.
- `polygon_with_hole_segments_split_around_hole` — assert scan lines that pass
  through the hole produce two segments (one per side).
- `two_disjoint_expolygons_independent_scan_conversion` — assert no
  cross-polygon pairing.
- `angle_45_rotated_output_matches_unrotated_after_inverse` — assert rotating
  the 45° output by -45° matches the 0° output geometry.
- `solid_spacing_adjusted_for_solid_role` — assert solid fill produces an
  integer number of lines.
- `half_open_vertex_test_no_double_count` — assert a scan line through a
  polygon vertex produces exactly the right segment count (no duplicate).
- `bridge_angle_overrides_layer_rotation` — assert bridge regions use the
  bridge angle, not the per-layer alternation.

### Validation
```bash
cargo build -p rectilinear-infill
cargo clippy -p rectilinear-infill --all-targets -- -D warnings
cargo test -p rectilinear-infill
cargo xtask build-guests --check
cargo xtask build-guests    # if stale
```

---

## Phase 3 — Gyroid infill fixes (raw emit)

### What
Modify `modules/core-modules/gyroid-infill/src/lib.rs`. The wave-generation
core (`gyroid_f`, `make_one_period`, `make_wave`, orientation choice) is
already correct per the gap analysis — no rewrite, just targeted fixes +
deletion of the clipping/linking code that moves to the linker.

### Changes (priority order)

**P0 — Add multi-role claims (ADR-0027).**
`gyroid-infill.toml`: add `claim:top-fill`, `claim:bottom-fill`,
`claim:bridge-fill` to `claims.holds`. This makes the existing top/bottom/bridge
emission code (lib.rs:180-210) actually fire when the user configures the
module as the holder for those roles.

**P0 — Rotation order fix.**
The current code (lib.rs:332-352) generates waves in the *unrotated* polygon's
bbox, then rotates the wave points around the bbox center. This is
geometrically wrong (see gap analysis §2.4). Match OrcaSlicer
`_fill_surface_single` (FillGyroid.cpp:300-376):
1. Rotate the ExPolygon by `-(base_angle + CorrectionAngle)` first (integer
   rotate of `Point2`).
2. Compute the rotated polygon's bbox.
3. Generate axis-aligned waves in the rotated bbox (gyroid units → mm).
4. Rotate the wave points back by `+(base_angle + CorrectionAngle)` to world
   space.
5. Emit raw wave polylines in world space. The linker clips against the
   world-space partitioned polygon.

**P0 — Delete the broken clipping.**
Delete `clip_polyline_to_expolygon` (lib.rs:611-636),
`point_in_expolygon` (lib.rs:569-582), `point_in_polygon` (lib.rs:585-606).
The linker does clipping via `slicer_core::polygon_ops::clip_polylines`
(Phase 0). The module emits raw waves; it does not clip.

**P1 — `align_to_grid`.** (FillGyroid.cpp:322)
Snap `bb.min` to a grid multiple of `2 * PI * scale_factor` for layer-to-layer
phase coherence. Stays in the module (pre-wave-generation).

**P1 — Expand factor.** (FillGyroid.cpp:326)
Change `expand = 4.0 * spacing_mm` (lib.rs:259) → `expand = 10.0 * spacing_mm`
to match OrcaSlicer. Prevents edge-clipped waves at low density.

**P1 — Delete short-segment filter from module.**
The `remove_short_polylines` (< 0.8 × spacing) filter (FillGyroid.cpp:356-359)
moves to the linker (post-clip). The module emits raw waves regardless of
length.

**P1 — Delete `chain_or_connect_infill` from module.**
Path ordering/connection moves to the linker. The module emits waves in
generation order.

**P2 — `align_to_grid` and expand factor** (above).

**P3 — `multiline` (deferred).**
Add `multiline` config key; divide `density_adjusted` by `multiline`
(FillGyroid.cpp:315). Implement `multiline_fill` via
`slicer_core::polygon_ops::offset` on the wave polylines. Low priority; only
affects `fill_multiline > 1`. Deferred.

### What stays
- `gyroid_f` (lib.rs:394-422) — math is identical to OrcaSlicer; the NaN guards
  are a safety improvement. No change.
- `make_one_period` (lib.rs:430-484) — functionally equivalent to OrcaSlicer.
  No change.
- `make_wave` (lib.rs:491-548) — tiling loop produces the same points. No change.
- The orientation choice (`vertical = |z_sin| <= |z_cos|`). No change.
- `DENSITY_ADJUST = 2.44`, `CORRECTION_ANGLE_DEG = -45.0`,
  `PATTERN_TOLERANCE = 0.2`. No change.
- The four-role emission structure (sparse / top / bottom / bridge). Stays
  (ADR-0027 makes it live).
- `solid_fill_role` mapping. Stays.

### What is deleted
- `clip_polyline_to_expolygon`, `point_in_expolygon`, `point_in_polygon` —
  linker clips.
- `polygon_bbox_mm` — replaced by the rotated-polygon bbox.
- The short-segment filter (if any exists in the module) — linker filters.
- The rotation-around-bbox-center code (lib.rs:344-352) — replaced by
  rotate-polygon-first.

### Tests (TDD)
- `square_10mm_z_0p2_emits_raw_waves` — assert waves are emitted raw (no
  clipping), in world space, with the correct rotation applied.
- `rotated_square_45_matches_unrotated_after_inverse` — assert the 45° output
  matches the 0° output after inverse rotation (regression for the rotation
  fix).
- `no_emitted_point_outside_partitioned_polygon` — assert every emitted point
  is inside the source ExPolygon (the linker clips, but the module should emit
  waves that are at least bounded by the expanded bbox).
- `align_to_grid_snaps_bbox_min` — assert the bbox min is a multiple of
  `2 * PI * scale_factor`.
- `expand_factor_is_10x_spacing` — assert the expand is 10× spacing.
- Keep `gyroid_f_no_nan`, `make_one_period_produces_points` (the point-in-polygon
  tests are deleted since their functions are removed).

### Validation
```bash
cargo build -p gyroid-infill
cargo clippy -p gyroid-infill --all-targets -- -D warnings
cargo test -p gyroid-infill
cargo xtask build-guests --check
cargo xtask build-guests    # if stale
```

---

## Phase 4 — New `infill-linker` module

### What
New module `modules/core-modules/infill-linker/`. The single
`Layer::InfillPostProcess` module that connects raw infill segments into
continuous multi-point polylines, globally across all regions and modules.

### Manifest
```toml
id = "com.core.infill-linker"
name = "Infill Linker"
description = "Connects raw infill segments into linked polylines (Layer::InfillPostProcess)"
version = "0.1.0"

[stage]
layer = "InfillPostProcess"

[claims]
# The linker does not hold fill-role claims; it operates on the prior stage's
# output. A new claim or the stage-dispatch default routes it. Design detail
# at implementation.
holds = []  # or ["claim:infill-link"] — TBD

[config.schema.infill_overlap]
type = "float"
default = 0.45
description = "Lateral infill overlap with perimeters, as a fraction of spacing. OrcaSlicer INFILL_OVERLAP_OVER_SPACING."
```

### Algorithm (`run_infill_postprocess`)
Port from OrcaSlicer `FillBase.cpp:1497-2300` (`connect_infill` +
`chain_or_connect_infill`), adapted for the PnP pipeline:

1. **Read prior `InfillIR`** (the raw segments emitted by all `Layer::Infill`
   modules) via the ADR-0028 contract change (pre-populated builder or new input
   param).
2. **Read partitioned fill polygons** from `PerimeterRegionView` (ADR-0028
   added them): `sparse_infill_area`, `top_solid_fill`, `bottom_solid_fill`,
   `bridge_areas`.
3. **Per region, per role:**
   a. Apply the infill overlap offset: `offset(polygons, -INFILL_OVERLAP_OVER_SPACING * spacing)`
      via `slicer_core::polygon_ops::offset`. The offset polygon is the
      infill boundary (wall-inset minus overlap).
   b. Re-clip raw segments to the offset boundary via
      `slicer_core::polygon_ops::clip_polylines` (Phase 0). Segments that were
      emitted over the wall-inset polygon are clipped back to the
      overlap-inset boundary.
   c. `remove_short_polylines`: drop clipped segments shorter than
      `0.8 * spacing` (port FillGyroid.cpp:356-359).
4. **`connect_infill`** (port FillBase.cpp:1497-2201): build a
   `BoundaryInfillGraph` (arc-length parametrization of the offset boundary),
   greedily connect adjacent scan-line segment endpoints via perimeter walks.
   This is the core linking algorithm.
5. **`chain_or_connect_infill`** (port FillBase.cpp:2201-2300): nearest-neighbor
   ordering + connect_infill wrapper. Orders the linked polylines for minimal
   travel.
6. **Cross-region/cross-module connection** (PnP-native, no OrcaSlicer
   precedent): connect endpoints between paths emitted by *different* regions
   or *different* infill modules, via perimeter walks on shared boundaries.
   This is the globally-optimal step that no single `run_infill` module could do.
7. **Emit linked multi-point polylines** via `InfillOutputBuilder`
   (`push_sparse_path` / `push_solid_path`), tagged with the original role +
   speed factor from the raw segments.

### `ExPolygonWithOffset`
The two-level offset structure (FillRectilinear.cpp:472-571) lives in the
linker. The linker constructs it from the partitioned fill polygon (outer =
wall-inset boundary; inner = overlap-inset boundary) for `connect_infill` to
walk. The `BoundaryInfillGraph` is built on the inner (offset) boundary.

### Default config
`ResolvedConfig` must add `infill-linker` to the default dispatch graph so
every print has linking. Without it, infill is raw disjoint segments (ADR-0025
trade-off). Add the `infill_overlap` config key (default 0.45) to
`crates/slicer-ir/src/resolved_config.rs` + the CLI in `pnp_cli`.

### Claim question (open)
The linker needs the dispatcher to route `Layer::InfillPostProcess` to it.
Options:
- A new `claim:infill-link` claim that the linker holds.
- No claim; the stage dispatches to the module by default (like the host
  built-ins).
Decide at implementation. The `docs/04_host_scheduler.md:378` rule permits a
module to hold multiple fill-role claims, but the linker is not a fill-role
module — it is a post-process module. A new claim or a stage-level default is
cleaner.

### Tests (TDD)
- `raw_segments_in_linked_polylines_out` — feed raw 2-point segments, assert
  the output is connected multi-point polylines.
- `cross_region_endpoint_connection` — two regions' raw segments, assert the
  linker connects endpoints across the region boundary.
- `re_clip_to_offset_boundary` — assert segments are clipped to the
  overlap-inset boundary (not the wall-inset boundary).
- `short_segment_filter` — assert segments < 0.8 × spacing are dropped.
- `overlap_offset_applied` — assert the offset boundary is
  `wall_inset - 0.45 * spacing`.
- `role_preserved` — assert the linked polyline's role matches the raw
  segment's role.
- `no_linker_module_degraded_raw_output` — assert that without the linker in
  the dispatch graph, infill is raw (integration test in slicer-runtime).

### Validation
```bash
cargo build -p infill-linker
cargo clippy -p infill-linker --all-targets -- -D warnings
cargo test -p infill-linker
cargo xtask build-guests --check
cargo xtask build-guests    # if stale
```

---

## Phase 5 — Integration

### What
Wire everything together, run the full validation ceremony.

### Steps
1. Add `infill-linker` to the default module set in `ResolvedConfig` /
   `pnp_cli` default dispatch.
2. Add `infill_overlap` config key to `ResolvedConfig` + CLI flag.
3. Run the full validation:
   ```bash
   cargo build --workspace --all-targets
   cargo clippy --workspace --all-targets -- -D warnings
   cargo xtask build-guests --check    # MUST be clean
   cargo xtask build-guests             # rebuild all guests
   cargo xtask test --workspace --summary   # full suite (packet-close ceremony)
   ```
4. End-to-end slice on `resources/benchy.stl` with both infill patterns and
   `--report`:
   ```bash
   cargo run --bin pnp_cli --release -- slice \
       --model resources/benchy.stl \
       --module-dir modules/core-modules \
       --output /tmp/out.gcode \
       --report /tmp/slicer-report.html
   ```
   Visually confirm linked infill paths in the HTML report (no disjoint
   segments with travels between them).

### Existing-test survey
Under Architecture A, any test that asserts on *linked* polylines from a
`run_infill` module will now see raw segments. Survey:
- `modules/core-modules/rectilinear-infill/tests/` — any test asserting path
  connection?
- `modules/core-modules/gyroid-infill/tests/gyroid_infill_tdd.rs` — any test
  asserting path connection?
- `crates/slicer-runtime/tests/contract/` — infill contract tests.
- `crates/slicer-runtime/tests/executor/` — executor tests asserting on
  infill path shape.
Tests asserting on linked output move to assert on the linker's output (or
the integration test asserts on the post-`Layer::InfillPostProcess` `InfillIR`).
Tests asserting on raw segment count/length still pass (modules still emit,
just raw).

---

## Subagent delegation

| Worker | Phase | Depends on | Disjoint? |
|---|---|---|---|
| A | Phase 0 — `clip_polylines` in slicer-core | None | Yes (slicer-core only) |
| B | Phase 1 — WIT contract redesign | None | Yes (WIT/SDK/views; disjoint from A) |
| C | Phase 2 — Rectilinear rewrite | A (uses `clip_polylines`? No — module emits raw, no clip. Actually independent of A.) B (must land after the contract is stable, OR land first emitting raw under the old contract). | Yes (rectilinear-infill only) |
| D | Phase 3 — Gyroid fixes | A (no — module emits raw). B (same as C). | Yes (gyroid-infill only) |
| E | Phase 4 — infill-linker module | A (uses `clip_polylines`) + B (uses new contract). | Yes (new module) |
| F | Phase 5 — Integration | A + B + C + D + E | — |

**Recommended execution:**
1. **A + B in parallel** (disjoint: `slicer-core::polygon_ops` vs WIT/SDK/views).
2. **C + D in parallel** after A + B (disjoint: `rectilinear-infill/src/lib.rs`
   vs `gyroid-infill/src/lib.rs`). C and D can land *before* B if they emit raw
   under the old contract (infill is raw, max travel, until the linker ships —
   acceptable for in-development). But landing after B is cleaner.
3. **E after A + B** (linker uses `clip_polylines` + the new contract).
4. **F after all** (integration + full validation).

**Note:** C and D are decoupled from the contract change (B) — they emit raw
segments under either contract. This decouples the parity rewrites from the
WIT schema bump. But the user experience is degraded (raw infill, max travel)
until E lands. Flag this in the packet plan.

---

## Open questions for implementation

1. **Option 1a vs 1b for prior-IR input (ADR-0028).** Pre-populated builder
   (muddies write-only semantics, smaller WIT change) vs new input parameter
   (cleaner semantics, larger WIT change). Pick at Phase 1 implementation.

2. **`clipper2_rust` line-clipping API.** Phase 0 must verify whether
   `clipper2_rust` exposes line clipping or whether we need the
   thicken-and-intersect workaround or a Sutherland-Hodgman variant. Research
   task before Phase 0 can commit to an approach.

3. **Linker claim.** New `claim:infill-link` vs stage-level default dispatch.
   Phase 4 design detail.

4. **`pattern_shift` placement.** Module (scan lines shifted per layer) or
   linker (connection pattern shifts). Recommended: module, so adjacent layers'
   segments interleave before the linker sees them.

5. **Lightning-infill transitional state (DEV-081).** Lightning currently
   self-links. Under Architecture A it should switch to raw emit, but that is a
   separate follow-up packet. Until then, lightning's self-linked output passes
   through the linker unchanged (the linker links raw segments; already-linked
   paths are not re-broken unless re-clipping changes them). Flag in
   DEVIATION_LOG.

6. **Existing infill tests.** Survey (Phase 5) which tests assert on linked
   polylines from `run_infill` modules — those need updating to assert on the
   linker's output or the post-`InfillPostProcess` `InfillIR`.

---

## Risks

- **WIT schema bump blast radius.** ~30 files touch `PerimeterRegionView`. The
  standard schema-bump pattern (ADR-0002, ADR-0009, ADR-0010) handles this, but
  it is real work. If the schema bump lands first (Phase 1), the parity rewrites
  (Phases 2-3) are not blocked.
- **`clipper2_rust` API uncertainty.** If `clipper2_rust` does not expose line
  clipping, Phase 0 needs a workaround. This is the single biggest unknown;
  research it first.
- **Linker is required infrastructure.** Every print degrades to raw infill
  without it. The default config MUST include it. Forgetting this is a
  user-visible regression.
- **Lightning-infill inconsistency.** Transitional; tracked in DEV-081. Not a
  blocker for this effort.
- **Test churn.** Existing tests asserting on linked output from `run_infill`
  modules break. The survey (Phase 5) must be thorough.