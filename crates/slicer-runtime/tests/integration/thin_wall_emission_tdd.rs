//! AC-3: thin-wall emission contract (T-060/T-061/T-062, packet 105).
//!
//! Fixture geometry: a 10 mm × 10 mm body with a thin protrusion attached at
//! one edge.  The protrusion is 0.22 mm wide — strictly between the thin-wall
//! cascade's `min_width` (nozzle_diameter/3 = 0.4/3 ≈ 0.133 mm per R4/P105) and the
//! nozzle width (0.4 mm = inner_wall_line_width) so `opening_ex` erodes it away from
//! the "thick core" and `difference_ex` returns it as a thin protrusion.
//!
//! R4 (P105) changed min_width from `inner_wall_line_width * 0.5` to `nozzle_diameter / 3.0`
//! (OrcaSlicer PerimeterGenerator.cpp:1603). With nozzle_diameter=0.4 this gives 0.133 mm.
//! The fixture protrusion (0.22 mm) is < 2 * min_width = 0.267 mm, so it is eroded by
//! `opening_ex` and classified as a thin wall.
//!
//! Two acceptance tests:
//!  - `thin_wall_emitted_for_thin_protrusion` (AC-3 positive case)
//!  - `detect_disabled_case` (AC-N1 negative case — detect_thin_wall = false)
//!
//! Fixture verified against live medial_axis output: a thin protrusion
//! produces a ThinWall loop with all vertices at x≈0.0 mm (max deviation < 0.001 mm),
//! confirming the ±0.05 mm AC-3 centerline bound is met.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ExPolygon, ExtrusionRole, LoopType, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a region that is a 10 mm × 10 mm body with a thin protrusion
/// of width `protrusion_width_mm` extending upward by 3 mm from the top edge.
///
/// The overall ExPolygon is an L-shaped polygon: the main body occupies
/// [-5, 5] × [-5, 5] mm, and the protrusion extends from
/// [-protrusion_width_mm/2, protrusion_width_mm/2] × [5, 8] mm.
///
/// The polygon is CCW with no holes.
fn make_thin_protrusion_region(protrusion_width_mm: f32, z: f32) -> SliceRegionView {
    let half_w = protrusion_width_mm / 2.0;

    // Main body corners + protrusion, traced CCW:
    //
    //                  [-hw, 8]   [+hw, 8]
    //                     |          |
    //  [-5, 5] --------[-hw, 5]  [+hw, 5]--------- [5, 5]
    //  |                                                  |
    //  [-5, -5] ---------------------------------------- [5, -5]
    //
    // CCW order (viewed from +Z):
    // BL, BR, (step up to protrusion base right), protrusion top-right,
    // protrusion top-left, (step down to protrusion base left), TL.

    let pts = vec![
        Point2::from_mm(-5.0, -5.0),   // 0 BL
        Point2::from_mm(5.0, -5.0),    // 1 BR
        Point2::from_mm(5.0, 5.0),     // 2 body top-right
        Point2::from_mm(half_w, 5.0),  // 3 protrusion base right
        Point2::from_mm(half_w, 8.0),  // 4 protrusion top-right
        Point2::from_mm(-half_w, 8.0), // 5 protrusion top-left
        Point2::from_mm(-half_w, 5.0), // 6 protrusion base left
        Point2::from_mm(-5.0, 5.0),    // 7 body top-left
    ];

    let poly = ExPolygon {
        contour: Polygon { points: pts },
        holes: Vec::new(),
    };

    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(poly)
        .build()
}

/// AC-3: a thin protrusion (0.22 mm wide) attached to a 10 mm body must
/// yield at least one WallLoop with `loop_type == ThinWall`.
///
/// Config: `inner_wall_line_width = 0.4 mm`, `nozzle_diameter = 0.4 mm`,
/// `wall_count = 2`, `detect_thin_wall = true`.
///
/// R4 (P105) thin-wall thresholds for this config:
///   min_width = nozzle_diameter / 3.0 = 0.4/3 ≈ 0.133 mm
///   (OrcaSlicer PerimeterGenerator.cpp:1603)
///   opening radius = min_width = 0.133 mm
///   feature eroded (thin) if width < 2 * min_width ≈ 0.267 mm
///   max_width = inner_wall_line_width * 2.0 = 0.8 mm
///
/// A 0.22 mm protrusion is < 0.267 mm so it gets eroded by `opening_ex`,
/// causing `difference_ex` to classify it as a thin protrusion.
#[test]
fn thin_wall_emitted_for_thin_protrusion() {
    let inner_w = 0.4_f32;
    let nozzle_d = 0.4_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", inner_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .float("nozzle_diameter", nozzle_d as f64)
        .bool("detect_thin_wall", true)
        .build();

    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    // Use 0.22 mm protrusion — narrower than 2*(nozzle_diameter/3) = 0.267 mm.
    let regions = vec![make_thin_protrusion_region(0.22, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();

    // There must be at least one ThinWall loop.
    let thin_loops: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::ThinWall)
        .collect();

    assert!(
        !thin_loops.is_empty(),
        "Expected ≥1 WallLoop with LoopType::ThinWall for 0.35 mm protrusion, got walls: {:?}",
        walls.iter().map(|w| w.loop_type).collect::<Vec<_>>()
    );

    for tl in &thin_loops {
        // Every ThinWall loop must carry ExtrusionRole::ThinWall.
        assert_eq!(
            tl.path.role,
            ExtrusionRole::ThinWall,
            "ThinWall loop has wrong ExtrusionRole: {:?}",
            tl.path.role
        );

        // Every vertex feature flag must have is_thin_wall = true.
        for (i, flag) in tl.feature_flags.iter().enumerate() {
            assert!(
                flag.is_thin_wall,
                "ThinWall loop feature_flags[{}].is_thin_wall is false",
                i
            );
        }

        // The thin loop path must have at least 2 points (a degenerate axis
        // with < 2 points is filtered by the impl, so any surviving loop is
        // non-trivial).
        assert!(
            tl.path.points.len() >= 2,
            "ThinWall loop has fewer than 2 points: {}",
            tl.path.points.len()
        );

        // AC-3 centerline assertion (±0.05 mm, per spec).
        //
        // The protrusion is a thin vertical rectangle of width 0.22 mm centered
        // at x=0, extending from y=5.0 to y=8.0 mm.  Its medial axis is the
        // straight vertical segment at x=0.  Every vertex of the ThinWall loop
        // must lie within ±0.05 mm of x=0 (the centerline).
        //
        // The main-span vertices observed from the live medial_axis output are at
        // x=-0.0000 mm (max deviation ≈ 0 mm).  The end-cap vertices where the
        // axis rounds to meet the protrusion's top/bottom edge may deviate in Y
        // but not in X (for this purely vertical protrusion); no vertices are
        // excluded here because the geometry is symmetric and the axis is exact.
        let centerline_x = 0.0_f32;
        let centerline_tolerance_mm = 0.05_f32;
        for pt in &tl.path.points {
            let dist_from_centerline = (pt.x - centerline_x).abs();
            assert!(
                dist_from_centerline <= centerline_tolerance_mm,
                "AC-3 violation: ThinWall vertex x={:.5} is {:.5} mm from centerline x={}; \
                 tolerance is ±{} mm.  Protrusion medial axis must lie within ±0.05 mm.",
                pt.x,
                dist_from_centerline,
                centerline_x,
                centerline_tolerance_mm
            );
        }
        // Y-bounds sanity: vertices must be within the protrusion region
        // (base y=5.0 mm, top y=8.0 mm, generous margin for inset).
        let protrusion_y_min = 4.5_f32;
        let protrusion_y_max = 8.5_f32;
        for pt in &tl.path.points {
            assert!(
                pt.y >= protrusion_y_min && pt.y <= protrusion_y_max,
                "ThinWall vertex y={} outside protrusion y-bounds [{}, {}]",
                pt.y,
                protrusion_y_min,
                protrusion_y_max
            );
        }
    }
}

/// AC-N1: same fixture geometry but `detect_thin_wall = false`.
///
/// Zero WallLoops with `loop_type == ThinWall` must be emitted.
#[test]
fn detect_disabled_case() {
    let inner_w = 0.4_f32;
    let nozzle_d = 0.4_f32;

    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("outer_wall_line_width", inner_w as f64)
        .float("inner_wall_line_width", inner_w as f64)
        .float("nozzle_diameter", nozzle_d as f64)
        .bool("detect_thin_wall", false)
        .build();

    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_thin_protrusion_region(0.22, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let thin_count = output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::ThinWall)
        .count();

    assert_eq!(
        thin_count, 0,
        "Expected 0 ThinWall loops when detect_thin_wall = false, got {}",
        thin_count
    );
}
