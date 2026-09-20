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

/// Baseline builder for `from_config` fixtures (packet 06 5c', item 11):
/// holds every key the module's classified reads touch on the tested path —
/// the ten `require_*` reads plus the remaining `from_config` reads — at the
/// guest's manifest-default values (rectilinear-infill.toml
/// [config.schema]). `line_width` holds its post-expansion default
/// (1.125 × nozzle_diameter): the raw manifest default 0 is the auto
/// sentinel the host expands at Phase B (slicer-config
/// `expand_automatic_values`), and `resolve_role_width` no longer expands —
/// a raw 0 cancels every emission via the spacing gate. Tests that exercise
/// a specific key add it after this baseline so their explicit value wins.
fn baseline_config() -> ConfigViewBuilder {
    ConfigViewBuilder::new()
        .float("infill_density", 0.2)
        .float("infill_angle", 45.0)
        .float("infill_speed", 60.0)
        .float("line_width", 0.45)
        .float("bridge_line_width", 0.0)
        .float("initial_layer_line_width", 0.0)
        .float("top_surface_line_width", 0.0)
        .float("internal_solid_infill_line_width", 0.0)
        .float("sparse_infill_line_width", 0.0)
        .float("bridge_density", 1.0)
        .float("bridge_speed", 25.0)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", false)
        .float("internal_bridge_density", 1.0)
        .float("internal_bridge_speed", 37.5)
        .float("internal_bridge_flow", 1.0)
        .bool("thick_internal_bridges", true)
        .float("top_surface_speed", 60.0)
        .float("internal_solid_infill_speed", 60.0)
        .float("sparse_infill_speed", 60.0)
        .bool("dont_filter_internal_bridges", false)
        .bool("enable_extra_bridge_layer", false)
        .float("internal_bridge_angle", 0.0)
        .float("infill_shift_step", 0.0)
}

fn config(density: f64) -> ConfigView {
    baseline_config()
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
    let module = RectilinearInfill::from_config(&cfg).unwrap();
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
fn very_small_polygon_emits_one_scan_row_without_panic() {
    // 0.1mm square is smaller than the 0.8mm line spacing (line_width/density),
    // but the canonical half-open grid always emits its first scan row.
    let cfg = config(0.5);
    let module = RectilinearInfill::from_config(&cfg).unwrap();
    let tiny = square_polygon(0.0, 0.0, 0.1);
    let region = region_with_sparse(tiny, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &cfg)
        .expect("run_infill must not panic on a sub-spacing polygon");

    assert_eq!(
        output.sparse_paths().len(),
        1,
        "a sub-spacing polygon must yield exactly one scan row"
    );
}
