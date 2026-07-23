//! Edge-case coverage for rectilinear-infill: non-convex and degenerate-small
//! fill polygons. Complements `rectilinear_infill_tdd.rs` (convex squares).

#![allow(missing_docs)]

use slicer_ir::{ConfigView, ExPolygon, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use rectilinear_infill::RectilinearInfill;

fn empty_paint_view() -> slicer_sdk::traits::PaintRegionLayerView {
    slicer_sdk::traits::PaintRegionLayerView::new(0)
}

fn config(density: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_angle", 0.0)
        .float("infill_speed", 50.0)
        .float("line_width", 0.4)
        .build()
}

/// L-shape: a 10×10 square (centred at origin) with the upper-right 5×5
/// quadrant removed — a non-convex polygon whose scan rows cross the contour in
/// more than two points on the lower band.
fn l_shape() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-5.0, -5.0),
                Point2::from_mm(5.0, -5.0),
                Point2::from_mm(5.0, 0.0),
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(0.0, 5.0),
                Point2::from_mm(-5.0, 5.0),
            ],
        },
        holes: vec![],
    }
}

fn region_with_sparse(area: ExPolygon, z: f32) -> SliceRegionView {
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .add_polygon(area.clone())
        .add_infill_area(area.clone())
        .sparse_infill_area(vec![area])
        .effective_layer_height(0.2)
        .z(z)
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);
    region
}

#[test]
fn non_convex_polygon_emits_finite_sparse_paths_without_panic() {
    let cfg = config(0.5);
    let module = RectilinearInfill::on_print_start(&cfg).unwrap();
    let region = region_with_sparse(l_shape(), 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &cfg)
        .expect("run_infill must not error on a non-convex polygon");

    let paths = output.sparse_paths();
    assert!(
        !paths.is_empty(),
        "an L-shape at 0.5 density should still produce sparse fill"
    );
    for p in paths {
        assert_eq!(p.role, ExtrusionRole::SparseInfill);
        for pt in &p.points {
            assert!(
                pt.x.is_finite() && pt.y.is_finite() && pt.z.is_finite(),
                "all emitted points must be finite"
            );
        }
    }
}

#[test]
fn very_small_polygon_emits_no_paths_without_panic() {
    // 0.1mm square is smaller than the 0.8mm line spacing (line_width/density),
    // so the scan-line loop never produces a row → empty output, no panic.
    let cfg = config(0.5);
    let module = RectilinearInfill::on_print_start(&cfg).unwrap();
    let tiny = square_polygon(0.0, 0.0, 0.1);
    let region = region_with_sparse(tiny, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &cfg)
        .expect("run_infill must not panic on a sub-spacing polygon");

    assert!(
        output.sparse_paths().is_empty(),
        "a polygon smaller than the line spacing must yield no paths"
    );
}
