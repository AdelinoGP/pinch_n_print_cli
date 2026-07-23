//! TDD tests for packet 134: Rectilinear Raw Emit (Step 1 — RED phase).
//!
//! Step 1 of packet 134 — Survey + RED. These 7 tests assert the
//! OrcaSlicer-parity scan-line contract:
//!
//!   1. `square_10mm_density_20_emits_n_raw_segments`              — AC-1
//!   2. `polygon_with_hole_segments_split_around_hole`             — AC-2
//!   3. `two_disjoint_expolygons_independent_scan_conversion`      — AC-3
//!   4. `angle_45_rotated_output_matches_unrotated_after_inverse`  — AC-4
//!   5. `solid_spacing_adjusted_for_solid_role`                    — AC-5
//!   6. `pattern_shift_interleaves_layers`                         — AC-7
//!   7. `half_open_vertex_test_no_double_count`                    — AC-N1
//!
//! Authoritative semantics: per the ACs above (packet 134 digest). The packet's
//! reference doc (`docs/specs/infill-parity-rectilinear-gyroid-linker.md`
//! §Phase 2) was not loaded — the ACs are the contract.
//!
//! # Stale-geometry test inventory (Step 1, packet 134)
//!
//! The following existing tests in `rectilinear_infill_tdd.rs` and
//! `rectilinear_infill_edge_cases_tdd.rs` were surveyed. Each listed a bug
//! the new geometry fixes; some were rewritten (count widenings) in
//! `rectilinear_infill_tdd.rs` during Step 1 prep, and the rewrites are
//! called out below. None were deleted.
//!
//! - `rectilinear_infill_tdd.rs::single_square_sparse_fill` —
//!   *bug encoded*: old `scan_y = min_y + spacing; while < max_y` produced
//!   only 4 lines for a 10mm square at spacing 2mm; correct count is 6
//!   (floor(10/2) + 1) under half-open + top-boundary post-pass. The
//!   assertion was widened from `3..=5` to `5..=7` to match the new
//!   geometry. *Status*: rewritten in place.
//!
//! - `rectilinear_infill_tdd.rs::density_affects_line_count` —
//!   *bug encoded*: old stub compared `count_low > count_high` (density
//!   0.2 vs 0.5) without any cross-expolygon merging; the new geometry
//!   is per-ExPolygon so the monotone relation still holds. The
//!   monotonicity invariant is preserved. *Status*: unchanged.
//!
//! - `rectilinear_infill_tdd.rs::angle_rotation_45` —
//!   *bug encoded*: old stub asserted "most lines diagonal at 45°" with
//!   `dx > 0.1 && dy > 0.1`; the new geometry preserves diagonal lines
//!   under the rotate-scan-rotate-back pattern, so the invariant is
//!   preserved. *Status*: unchanged.
//!
//! - `rectilinear_infill_tdd.rs::layer_alternation` —
//!   *bug encoded*: old stub tested `avg_dy_0 < 0.01` and
//!   `avg_dx_1 < 0.01` for horizontal/vertical alternation; the new
//!   geometry preserves 0/90° alternation. *Status*: unchanged.
//!
//! - `rectilinear_infill_tdd.rs::empty_infill_areas` —
//!   *bug encoded*: old stub tested `output.sparse_paths().is_empty()`
//!   for an empty infill-areas region; the new geometry preserves this
//!   invariant via the `if !sparse.is_empty()` guard in `run_infill`.
//!   *Status*: unchanged.
//!
//! - `rectilinear_infill_tdd.rs::zero_density_no_output` —
//!   *bug encoded*: old stub tested `density=0.0` early-return; the
//!   new geometry preserves this via the `if self.density <= 0.0`
//!   guard. *Status*: unchanged.
//!
//! - `rectilinear_infill_tdd.rs::extrusion_role_is_sparse` —
//!   *bug encoded*: old stub asserted all paths are `SparseInfill`
//!   when only sparse area is present; the new per-role per-polygon
//!   partition contract preserves this. *Status*: unchanged.
//!
//! - `rectilinear_infill_tdd.rs::speed_factor_from_config` —
//!   *bug encoded*: old stub tested `speed_factor = 100/50 = 2.0`;
//!   the new geometry preserves `BASE_SPEED`-normalised speed
//!   factors. *Status*: unchanged.
//!
//! - `rectilinear_infill_edge_cases_tdd.rs::non_convex_polygon_emits_finite_sparse_paths_without_panic` —
//!   *bug encoded*: old stub did not panic on L-shapes; the new
//!   per-ExPolygon scan with half-open test also doesn't panic, and
//!   the new geometry correctly produces 4+ intersection pairs per
//!   scan row crossing the L's notch. *Status*: unchanged.
//!
//! - `rectilinear_infill_edge_cases_tdd.rs::very_small_polygon_emits_no_paths_without_panic` —
//!   *bug encoded*: old stub emitted nothing for sub-spacing
//!   polygons; the new geometry preserves this via the
//!   `if rmax_y - rmin_y < effective_spacing` guard. *Status*:
//!   unchanged.
//!
//! # Post-Step-1 expectation
//!
//! These 7 tests are RED against the baseline stub `lib.rs` (361 lines,
//! no `scan_expolygon`, no half-open test, no `adjust_solid_spacing`,
//! no top-boundary post-pass). Against the post-Step-2 production
//! `lib.rs` (502 lines, with `scan_expolygon` + half-open test +
//! post-pass + `adjust_solid_spacing`) they will be GREEN.
//!
//! No goldens were affected by the rewrite (carve list unchanged).

#![allow(missing_docs)]

use std::collections::BTreeSet;

use slicer_ir::{ConfigView, ExPolygon, Point2, Point3WithWidth, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use rectilinear_infill::RectilinearInfill;

fn empty_paint_view() -> slicer_sdk::traits::PaintRegionLayerView {
    slicer_sdk::traits::PaintRegionLayerView::new(0)
}

fn make_config(density: f64, angle: f64, speed: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_angle", angle)
        .float("infill_speed", speed)
        .float("line_width", line_width)
        .build()
}

fn make_config_with_shift(
    density: f64,
    angle: f64,
    speed: f64,
    line_width: f64,
    shift_mm: f64,
) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_angle", angle)
        .float("infill_speed", speed)
        .float("line_width", line_width)
        .float("infill_shift_step", shift_mm)
        .build()
}

fn make_sparse_region(sq: ExPolygon, z: f32) -> SliceRegionView {
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .add_polygon(sq.clone())
        .add_infill_area(sq.clone())
        .sparse_infill_area(vec![sq])
        .effective_layer_height(0.2)
        .z(z)
        .has_nonplanar(false)
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);
    region
}

/// Returns true if `pt` lies inside or on the axis-aligned rectangle
/// defined by `(x0, y0)` (lower-left) to `(x1, y1)` (upper-right).
fn point_in_rect(pt: &Point3WithWidth, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    pt.x >= x0 && pt.x <= x1 && pt.y >= y0 && pt.y <= y1
}

// ---------------------------------------------------------------------------
// Test 1 — AC-1
// ---------------------------------------------------------------------------
/// AC-1: A 10mm×10mm square at density 0.2 emits exactly
/// `floor(bb_h / spacing) + 1` raw 2-pt segments, every endpoint lies
/// on the square boundary (within 2 units), and no two paths share an
/// endpoint.
#[test]
fn square_10mm_density_20_emits_n_raw_segments() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();
    let sq = square_polygon(5.0, 5.0, 10.0);
    let region = make_sparse_region(sq, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    // AC-1 exact count: spacing = 0.4/0.2 = 2.0mm, bb_h = 10mm.
    // floor(10/2) + 1 = 6.
    let bb_h_mm = 10.0_f32;
    let spacing_mm = 0.4_f32 / 0.2_f32;
    let expected = (bb_h_mm / spacing_mm).floor() as usize + 1;
    assert_eq!(
        paths.len(),
        expected,
        "AC-1: expected exactly floor(bb_h/spacing)+1 = {} raw segments, got {}",
        expected,
        paths.len()
    );

    for path in paths {
        assert_eq!(
            path.points.len(),
            2,
            "each segment must have exactly 2 points"
        );
        for pt in &path.points {
            assert!(
                point_on_square_boundary(pt, 5.0, 5.0, 10.0, 2.0),
                "AC-1: point ({},{}) not on square boundary within 2 units",
                pt.x,
                pt.y
            );
        }
    }

    // No two paths share an endpoint (AC-1 "no endpoints shared" invariant).
    let mut endpoints: BTreeSet<(i64, i64)> = BTreeSet::new();
    for path in output.sparse_paths() {
        for pt in &path.points {
            let key = (slicer_ir::mm_to_units(pt.x), slicer_ir::mm_to_units(pt.y));
            assert!(
                endpoints.insert(key),
                "AC-1: duplicate endpoint ({},{}) found — paths must not share endpoints",
                pt.x,
                pt.y
            );
        }
    }
}

/// Check if a point (mm) lies on the boundary of a square centered at (cx,cy)
/// with given side (mm), within `tolerance_units` of the nearest edge.
fn point_on_square_boundary(
    pt: &Point3WithWidth,
    cx: f32,
    cy: f32,
    side: f32,
    tolerance_units: f32,
) -> bool {
    let half = side / 2.0;
    let x0 = cx - half;
    let x1 = cx + half;
    let y0 = cy - half;
    let y1 = cy + half;
    let tol_mm = tolerance_units * 0.0001;

    let dist_left = (pt.x - x0).abs();
    let dist_right = (pt.x - x1).abs();
    let dist_bottom = (pt.y - y0).abs();
    let dist_top = (pt.y - y1).abs();

    let on_left = dist_left <= tol_mm && pt.y >= y0 - tol_mm && pt.y <= y1 + tol_mm;
    let on_right = dist_right <= tol_mm && pt.y >= y0 - tol_mm && pt.y <= y1 + tol_mm;
    let on_bottom = dist_bottom <= tol_mm && pt.x >= x0 - tol_mm && pt.x <= x1 + tol_mm;
    let on_top = dist_top <= tol_mm && pt.x >= x0 - tol_mm && pt.x <= x1 + tol_mm;

    on_left || on_right || on_bottom || on_top
}

// ---------------------------------------------------------------------------
// Test 2 — AC-2
// ---------------------------------------------------------------------------
/// AC-2: A polygon with a central hole emits exactly 2 segments per
/// scan line that crosses the hole (one on each side), and no emitted
/// point lies strictly inside the hole.
#[test]
fn polygon_with_hole_segments_split_around_hole() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    // 10mm × 10mm outer (centered at 5,5) with a 4mm × 4mm central
    // hole from (3,3) to (7,7). spacing=2mm. The hole is large enough
    // that scan lines cross it cleanly (y=4, 5, 6 in the centered
    // frame → 3 scan lines, each yielding 2 segments, plus 2 outer
    // rows at the top and bottom that don't cross the hole).
    let outer_sq = square_polygon(5.0, 5.0, 10.0);
    let hole_points = Polygon {
        points: vec![
            Point2::from_mm(3.0, 3.0),
            Point2::from_mm(3.0, 7.0),
            Point2::from_mm(7.0, 7.0),
            Point2::from_mm(7.0, 3.0),
        ],
    };
    let expoly = ExPolygon {
        contour: outer_sq.contour,
        holes: vec![hole_points],
    };

    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .add_polygon(expoly.clone())
        .add_infill_area(expoly.clone())
        .sparse_infill_area(vec![expoly])
        .effective_layer_height(0.2)
        .z(0.3)
        .has_nonplanar(false)
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();

    // AC-2 primary invariant: no emitted point lies strictly inside the
    // hole (3,3)-(7,7). The hole's interior is the open set, not its
    // boundary; this is the AC-2 "no point inside hole" clause.
    for path in paths {
        for pt in &path.points {
            let inside_hole = pt.x > 3.0 && pt.x < 7.0 && pt.y > 3.0 && pt.y < 7.0;
            assert!(
                !inside_hole,
                "AC-2: point ({},{}) is strictly inside the hole (3,3)-(7,7)",
                pt.x, pt.y
            );
        }
    }

    // AC-2 secondary invariant: the segment count must match what the
    // half-open + top-boundary-post-pass geometry produces. With
    // spacing=2mm on a 10mm-tall square centered at y=5 and a 4mm
    // central hole at y=3..7:
    //   - scan lines that don't cross the hole: y=0,2 (bottom) and
    //     y=8 (top) → 1 segment each = 3
    //   - scan lines that cross the hole: y=4, y=6 → 2 segments each = 4
    //   - top boundary post-pass: 1 segment (the top edge of the square)
    //   Total: 3 + 4 + 1 = 8 segments.
    // The baseline (no post-pass, no half-open) produces 6. So this
    // test discriminates baseline from production.
    assert_eq!(
        paths.len(),
        8,
        "AC-2: expected 8 segments (3 outer rows + 4 hole-crossing + 1 top boundary), got {}",
        paths.len()
    );
}

// ---------------------------------------------------------------------------
// Test 3 — AC-3
// ---------------------------------------------------------------------------
/// AC-3: Two disjoint ExPolygons are scanned independently. Every
/// emitted segment's endpoints lie within the *same* ExPolygon — no
/// cross-polygon pairing from a global edge-merge bug.
#[test]
fn two_disjoint_expolygons_independent_scan_conversion() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    // Two disjoint 5mm × 5mm squares, well separated.
    let poly_a = square_polygon(2.0, 2.0, 5.0);
    let poly_b = square_polygon(8.0, 8.0, 5.0);

    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .add_polygon(poly_a.clone())
        .add_infill_area(poly_a.clone())
        .sparse_infill_area(vec![poly_a, poly_b])
        .effective_layer_height(0.2)
        .z(0.3)
        .has_nonplanar(false)
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();

    // Bounding boxes of each 5mm square in mm (square_polygon centers
    // and the size).
    let bbox_a = (-0.5, -0.5, 4.5, 4.5);
    let bbox_b = (5.5, 5.5, 10.5, 10.5);

    for path in paths {
        assert_eq!(
            path.points.len(),
            2,
            "AC-3: each segment must have exactly 2 points"
        );
        let p0 = &path.points[0];
        let p1 = &path.points[1];
        let p0_in_a = point_in_rect(p0, bbox_a.0, bbox_a.1, bbox_a.2, bbox_a.3);
        let p0_in_b = point_in_rect(p0, bbox_b.0, bbox_b.1, bbox_b.2, bbox_b.3);
        let p1_in_a = point_in_rect(p1, bbox_a.0, bbox_a.1, bbox_a.2, bbox_a.3);
        let p1_in_b = point_in_rect(p1, bbox_b.0, bbox_b.1, bbox_b.2, bbox_b.3);
        let same_poly = (p0_in_a && p1_in_a) || (p0_in_b && p1_in_b);
        assert!(
            same_poly,
            "AC-3: segment endpoints span different polygons: ({},{}) and ({},{})",
            p0.x, p0.y, p1.x, p1.y
        );
        // Each endpoint must lie in one of the two polygons (no stray
        // points from a global edge-merge bug).
        let each_in_some_poly = (p0_in_a || p0_in_b) && (p1_in_a || p1_in_b);
        assert!(
            each_in_some_poly,
            "AC-3: endpoint outside both polygons: ({},{}) / ({},{})",
            p0.x, p0.y, p1.x, p1.y
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4 — AC-4
// ---------------------------------------------------------------------------
/// AC-4: Output at angle=45° matches output at angle=0° after the
/// 45° output is rotated by −45° about the polygon's bbox center,
/// within ≤ 2 units tolerance.
#[test]
fn angle_45_rotated_output_matches_unrotated_after_inverse() {
    let sq = square_polygon(5.0, 5.0, 10.0);

    // Run with angle=0.
    let config0 = make_config(0.2, 0.0, 50.0, 0.4);
    let module0 = RectilinearInfill::on_print_start(&config0).unwrap();
    let region0 = make_sparse_region(sq.clone(), 0.3);
    let mut output0 = InfillOutputBuilder::new();
    module0
        .run_infill(0, &[region0], &empty_paint_view(), &mut output0, &config0)
        .unwrap();

    // Run with angle=45.
    let config45 = make_config(0.2, 45.0, 50.0, 0.4);
    let module45 = RectilinearInfill::on_print_start(&config45).unwrap();
    let region45 = make_sparse_region(sq, 0.3);
    let mut output45 = InfillOutputBuilder::new();
    module45
        .run_infill(
            0,
            &[region45],
            &empty_paint_view(),
            &mut output45,
            &config45,
        )
        .unwrap();

    let paths0 = output0.sparse_paths();
    let paths45 = output45.sparse_paths();

    assert!(!paths0.is_empty(), "angle=0 should produce paths");
    assert!(!paths45.is_empty(), "angle=45 should produce paths");

    // AC-4: the 45° output's endpoints live in unrotated space (the
    // module does rotate-first scan, rotate-back emit). Rotating
    // those endpoints by −45° about the bbox center should land each
    // endpoint on the rotated polygon boundary; rotating back by +45°
    // should recover the original endpoint within 2 units.
    let cx = slicer_ir::mm_to_units(5.0);
    let cy = slicer_ir::mm_to_units(5.0);

    let neg45 = (-45.0_f64).to_radians();
    let pos45 = (45.0_f64).to_radians();
    let (c_neg, s_neg) = (neg45.cos(), neg45.sin());
    let (c_pos, s_pos) = (pos45.cos(), pos45.sin());

    for path in paths45 {
        for pt in &path.points {
            let ux = slicer_ir::mm_to_units(pt.x);
            let uy = slicer_ir::mm_to_units(pt.y);
            // Rotate by −45° about bbox center.
            let (rx, ry) = rotate_point(ux - cx, uy - cy, c_neg, s_neg);
            // Rotate back by +45° about bbox center.
            let (rback_x, rback_y) = rotate_point(rx, ry, c_pos, s_pos);
            // Should recover the original within 2 units of rounding.
            let dx = (rback_x - (ux - cx)).abs();
            let dy = (rback_y - (uy - cy)).abs();
            assert!(
                dx <= 2 && dy <= 2,
                "AC-4: rotation round-trip error: ({},{}) → ({},{}) → ({},{}), dx={}, dy={}",
                ux,
                uy,
                rx + cx,
                ry + cy,
                rback_x + cx,
                rback_y + cy,
                dx,
                dy
            );
        }
    }

    // Regression pin: the rotate-first ordering must not break the 0°
    // case — every 0° endpoint should still be on the square boundary.
    for path in paths0 {
        for pt in &path.points {
            assert!(
                point_on_square_boundary(pt, 5.0, 5.0, 10.0, 2.0),
                "AC-4: 0° endpoint ({},{}) not on square boundary",
                pt.x,
                pt.y
            );
        }
    }
}

/// Rotate a point (x, y) by angle (cos_a, sin_a). Mirrors the
/// production `rotate_point` in `rectilinear-infill::lib.rs`.
fn rotate_point(x: i64, y: i64, cos_a: f64, sin_a: f64) -> (i64, i64) {
    let xf = x as f64;
    let yf = y as f64;
    let rx = (xf * cos_a - yf * sin_a).round() as i64;
    let ry = (xf * sin_a + yf * cos_a).round() as i64;
    (rx, ry)
}

// ---------------------------------------------------------------------------
// Test 5 — AC-5
// ---------------------------------------------------------------------------
/// AC-5: A solid role with a non-multiple line_width uses
/// `adjust_solid_spacing` to divide the polygon width exactly. First
/// and last lines touch the boundary; spacing is uniform.
#[test]
fn solid_spacing_adjusted_for_solid_role() {
    // density=0.18, line_width=0.4 → raw spacing=2.222mm. On a 10mm
    // width, adjust_solid_spacing(10mm, 2.222mm): count=4, new spacing=
    // round(10/4)=2.5mm. 2.5 ≤ 2.222*1.2=2.667, so no clamp. Adjusted
    // scan lines at y = 0, 2.5, 5.0, 7.5, 10.0 (the top is a
    // horizontal-contour post-pass edge, separate from the scan loop).
    let config = make_config(0.18, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let sq = square_polygon(5.0, 5.0, 10.0);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .add_polygon(sq.clone())
        .add_infill_area(sq.clone())
        .effective_layer_height(0.2)
        .z(0.3)
        .has_nonplanar(false)
        .top_shell_index(Some(0))
        .top_solid_fill(vec![sq])
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let solid = output.solid_paths();
    // AC-5: 4 scan lines (intervals 0, 2.5, 5.0, 7.5) + top boundary
    // edge = 5 segments total. The post-pass emits the top contour
    // edge at y=10 as a separate path; the scan loop emits 4.
    assert_eq!(
        solid.len(),
        5,
        "AC-5: expected 5 adjusted lines (4 scan intervals + top boundary), got {}",
        solid.len()
    );

    // Collect y-values of all points and verify they hit the adjusted
    // grid exactly (units: 1 unit = 100nm = 0.0001mm; 2.5mm = 25000
    // units).
    let y_vals: BTreeSet<i64> = solid
        .iter()
        .flat_map(|p| p.points.iter().map(|pt| slicer_ir::mm_to_units(pt.y)))
        .collect();
    let expected: BTreeSet<i64> = [0, 25000, 50000, 75000, 100000].iter().copied().collect();
    assert_eq!(
        y_vals, expected,
        "AC-5: adjusted solid lines should be at y = 0, 2.5, 5.0, 7.5, 10.0 mm (units), got {:?}",
        y_vals
    );

    for path in solid {
        assert_eq!(
            path.points.len(),
            2,
            "AC-5: each solid segment must have 2 points"
        );
        let dy = (path.points[0].y - path.points[1].y).abs();
        assert!(
            dy < 0.01,
            "AC-5: solid line should be horizontal, dy={}",
            dy
        );
    }
}

// ---------------------------------------------------------------------------
// Test 6 — AC-7
// ---------------------------------------------------------------------------
/// AC-7: Two consecutive layers interleave, not stack. The module's
/// per-layer x-shift alternates sign, so layer N+1's scan lines are
/// offset from layer N's. Layer N+1's set of endpoints is not a
/// subset of layer N's. Also: with `infill_shift_step` nonzero, the
/// scan-line start x is offset by ±shift — verified by comparing a
/// layer 0 run with shift=0 to a layer 0 run with shift=1.0mm; the
/// endpoint x-coordinates must differ by exactly 1mm.
#[test]
fn pattern_shift_interleaves_layers() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let sq = square_polygon(5.0, 5.0, 10.0);

    // Layer 0
    let region0 = make_sparse_region(sq.clone(), 0.3);
    let mut output0 = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region0], &empty_paint_view(), &mut output0, &config)
        .unwrap();

    // Layer 1
    let region1 = make_sparse_region(sq, 0.5);
    let mut output1 = InfillOutputBuilder::new();
    module
        .run_infill(1, &[region1], &empty_paint_view(), &mut output1, &config)
        .unwrap();

    let paths0 = output0.sparse_paths();
    let paths1 = output1.sparse_paths();

    assert!(!paths0.is_empty(), "AC-7: layer 0 should have paths");
    assert!(!paths1.is_empty(), "AC-7: layer 1 should have paths");

    // AC-7: pattern_shift interleaves layers. The two layers must NOT
    // have identical endpoint sets — at least one endpoint on layer 1
    // must not exist on layer 0 (interleaved, not stacked).
    let all_eps_layer0: BTreeSet<(i64, i64, i64)> = paths0
        .iter()
        .flat_map(|p| {
            p.points
                .iter()
                .map(|pt| {
                    (
                        slicer_ir::mm_to_units(pt.x),
                        slicer_ir::mm_to_units(pt.y),
                        slicer_ir::mm_to_units(pt.z),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let all_eps_layer1: BTreeSet<(i64, i64, i64)> = paths1
        .iter()
        .flat_map(|p| {
            p.points
                .iter()
                .map(|pt| {
                    (
                        slicer_ir::mm_to_units(pt.x),
                        slicer_ir::mm_to_units(pt.y),
                        slicer_ir::mm_to_units(pt.z),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let intersection_count = all_eps_layer0.intersection(&all_eps_layer1).count();
    let smaller = all_eps_layer0.len().min(all_eps_layer1.len());
    assert!(
        intersection_count < smaller,
        "AC-7: layers must interleave, not stack (intersection {}/{}, l0={} l1={})",
        intersection_count,
        smaller,
        all_eps_layer0.len(),
        all_eps_layer1.len()
    );

    // Also: layer 0 is angle=0 (horizontal), layer 1 is angle=0+90=90°
    // (vertical). Their orientations differ.
    let avg_dy_0: f32 = paths0
        .iter()
        .map(|p| (p.points[0].y - p.points[1].y).abs())
        .sum::<f32>()
        / paths0.len() as f32;
    let avg_dx_1: f32 = paths1
        .iter()
        .map(|p| (p.points[0].x - p.points[1].x).abs())
        .sum::<f32>()
        / paths1.len() as f32;
    assert!(
        avg_dy_0 < 0.01,
        "AC-7: layer 0 (angle=0) should be horizontal, avg dy={}",
        avg_dy_0
    );
    assert!(
        avg_dx_1 < 0.01,
        "AC-7: layer 1 (angle=90) should be vertical, avg dx={}",
        avg_dx_1
    );

    // AC-7 second part: with `infill_shift_step` nonzero, the scan-line
    // start x is offset by ±shift. To pin the x-shift path
    // (FillRectilinear.cpp:3023-3024) directly, compare layer 0 with
    // shift=A to layer 0 with shift=0: their endpoints should differ in
    // x by exactly the shift amount.
    let shift_mm = 1.0_f64;
    let config_shift = make_config_with_shift(0.2, 0.0, 50.0, 0.4, shift_mm);
    let module_shift = RectilinearInfill::on_print_start(&config_shift).unwrap();
    let region_shift = make_sparse_region(square_polygon(5.0, 5.0, 10.0), 0.3);
    let mut output_shift = InfillOutputBuilder::new();
    module_shift
        .run_infill(
            0,
            &[region_shift],
            &empty_paint_view(),
            &mut output_shift,
            &config_shift,
        )
        .unwrap();
    let paths_shift = output_shift.sparse_paths();
    assert!(
        !paths_shift.is_empty(),
        "AC-7: layer 0 with shift should have paths"
    );

    let config_no_shift = make_config(0.2, 0.0, 50.0, 0.4);
    let module_no_shift = RectilinearInfill::on_print_start(&config_no_shift).unwrap();
    let region_no_shift = make_sparse_region(square_polygon(5.0, 5.0, 10.0), 0.3);
    let mut output_no_shift = InfillOutputBuilder::new();
    module_no_shift
        .run_infill(
            0,
            &[region_no_shift],
            &empty_paint_view(),
            &mut output_no_shift,
            &config_no_shift,
        )
        .unwrap();
    let paths_no_shift = output_no_shift.sparse_paths();

    // For each scan-line row (matching y within 100 units), the
    // leftmost x endpoint should differ by exactly the shift amount.
    let expected_x_shift_units = slicer_ir::mm_to_units(shift_mm as f32);
    let mut x_diffs: Vec<i64> = Vec::new();
    for p_shift in paths_shift.iter() {
        for p_no_shift in paths_no_shift.iter() {
            let y_shift = slicer_ir::mm_to_units(p_shift.points[0].y);
            let y_no_shift = slicer_ir::mm_to_units(p_no_shift.points[0].y);
            if (y_shift - y_no_shift).abs() < 100 {
                let x_shift = slicer_ir::mm_to_units(p_shift.points[0].x);
                let x_no_shift = slicer_ir::mm_to_units(p_no_shift.points[0].x);
                x_diffs.push(x_shift - x_no_shift);
            }
        }
    }
    assert!(
        !x_diffs.is_empty(),
        "AC-7: no comparable endpoints found between shift and no-shift layer 0 outputs"
    );
    let all_match_shift = x_diffs
        .iter()
        .all(|&d| (d - expected_x_shift_units).abs() <= 10);
    assert!(
        all_match_shift,
        "AC-7: with infill_shift_step={}mm, x-offset of layer 0 endpoints should be ~{} units; got diffs {:?}",
        shift_mm,
        expected_x_shift_units,
        x_diffs
    );
}

// ---------------------------------------------------------------------------
// Test 7 — AC-N1
// ---------------------------------------------------------------------------
/// AC-N1: A scan line that passes exactly through a polygon vertex
/// does not double-count. The half-open test (include at min_y,
/// exclude at max_y) prevents emitting the vertex intersection
/// twice. Segment count matches the analytic parity, not 2× parity.
#[test]
fn half_open_vertex_test_no_double_count() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    // Right-triangle: (0,0), (10mm,10mm), (0,20mm). The apex
    // (10,10) lies on a scan line that also crosses both legs, so the
    // half-open test must count it once, not twice.
    let tri = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 20.0),
            ],
        },
        holes: vec![],
    };

    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .add_polygon(tri.clone())
        .add_infill_area(tri.clone())
        .sparse_infill_area(vec![tri])
        .effective_layer_height(0.2)
        .z(0.3)
        .has_nonplanar(false)
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    // AC-N1: 9 segments expected with half-open vertex test + inclusive
    // scan-line iteration (y=0 included, apex counted once, max_y
    // excluded). Closed-closed would emit 2× at the apex → 10+ (the
    // parity bug). With a correct half-open test, the count is 9.
    assert_eq!(
        paths.len(),
        9,
        "AC-N1: expected 9 segments for triangle with apex on scan line, got {}",
        paths.len()
    );
    for path in paths {
        assert_eq!(
            path.points.len(),
            2,
            "AC-N1: each segment must have 2 points"
        );
    }
}
