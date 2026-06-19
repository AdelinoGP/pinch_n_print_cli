// per_vertex_is_bridge_propagation_tdd.rs — AC-1 and AC-N1 TDD tests.
//
// AC-1: When a SliceRegionView has a bridge_areas rectangle covering exactly the
// right half of its outer polygon, run_perimeters must emit outer-wall vertices
// with `is_bridge = true` for those inside the bridge rectangle and `false` for
// those outside.
//
// AC-N1 (no_bridge_areas_case): When bridge_areas is empty, every is_bridge flag
// must be `false` and no panic occurs.
//
// NOTE: AC-1's full per-vertex assertion may remain failing until Step 3 wires
// `point_in_any_polygon` into the perimeter module consumer. The test is written
// as a real TDD test that expresses the target behaviour. AC-N1 is the sub-case
// that MUST pass immediately (empty bridge_areas → no panic, all is_bridge=false).

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a `ConfigView` with `wall_count=1`, `line_width=0.4`.
fn config_1_wall() -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("line_width", 0.4)
        .build()
}

/// Build a 10×10 mm square polygon centered at origin (0..10 mm range).
/// Uses scaled integer units (1 unit = 100 nm).
///
/// The outer polygon spans x: 0..100_000, y: 0..100_000 (100 µm units → 10 mm).
fn outer_square() -> ExPolygon {
    // 10 mm = 100_000 units (1 unit = 100 nm, 10mm = 10*10^4 units)
    let size = 100_000_i64; // 10 mm in units
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: size, y: 0 },
                Point2 { x: size, y: size },
                Point2 { x: 0, y: size },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a bridge rectangle covering the right half of the outer square.
/// x: 50_000..100_000, y: 0..100_000 (right half of the 10 mm square).
fn bridge_right_half() -> ExPolygon {
    let half = 50_000_i64;
    let size = 100_000_i64;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: half, y: 0 },
                Point2 { x: size, y: 0 },
                Point2 { x: size, y: size },
                Point2 { x: half, y: size },
            ],
        },
        holes: Vec::new(),
    }
}

/// AC-N1: Empty bridge_areas must not cause a panic and every is_bridge must be false.
#[test]
fn no_bridge_areas_case() {
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![outer_square()]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    // Explicitly empty bridge_areas (the default — belt-and-suspenders).
    region.set_bridge_areas(vec![]);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic with empty bridge_areas");

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "must emit at least one wall loop");

    for wall in walls {
        for (i, flags) in wall.feature_flags.iter().enumerate() {
            assert!(
                !flags.is_bridge,
                "vertex {i} in wall {} must have is_bridge=false when bridge_areas is empty",
                wall.perimeter_index
            );
        }
    }
}

/// AC-1: Outer-wall vertices inside the bridge rectangle have is_bridge=true;
/// vertices outside have is_bridge=false.
///
/// The assertion is VERTEX-ORDER-INDEPENDENT: we zip emitted path x-coordinates
/// with their feature_flags and classify bridge/non-bridge by comparing each
/// emitted x against the bridge boundary (50_000 units ≈ 5 mm), not by hardcoded
/// index. This is robust against any vertex rotation Clipper2 may apply.
///
/// Fixture: 4-point polygon with two vertices at x≈25_000 (left half, not bridge)
/// and two at x≈75_000 (right half, bridge). After inset the x values shift inward
/// by ~line_width/2 but both groups remain clearly on their respective sides of the
/// bridge midline (50_000 units), so the left/right classification is robust.
#[test]
fn bridge_areas_set_is_bridge_on_inner_vertices() {
    let config = config_1_wall();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    // 4-point polygon:
    //   (25_000, 50_000) — strictly in left half
    //   (75_000, 50_000) — strictly in right half (bridge)
    //   (75_000, 75_000) — strictly in right half (bridge)
    //   (25_000, 75_000) — strictly in left half
    let inner_test_poly = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: 25_000,
                    y: 50_000,
                },
                Point2 {
                    x: 75_000,
                    y: 50_000,
                },
                Point2 {
                    x: 75_000,
                    y: 75_000,
                },
                Point2 {
                    x: 25_000,
                    y: 75_000,
                },
            ],
        },
        holes: Vec::new(),
    };

    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![inner_test_poly]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_bridge_areas(vec![bridge_right_half()]);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "must emit at least one wall loop");

    // Find the outer wall (perimeter_index == 0).
    let outer_walls: Vec<_> = walls.iter().filter(|w| w.perimeter_index == 0).collect();
    assert!(!outer_walls.is_empty(), "must have an outer wall");

    // The bridge boundary is at x = 50_000 units → 5.0 mm in emitted coordinates.
    // expolygon_to_path3d converts units to mm via units_to_mm (1 unit = 100 nm = 1e-4 mm).
    // 50_000 units = 5.0 mm.
    // After inset by ~line_width/2 = ~0.2 mm, left vertices x≈2.5 mm - 0.2 mm ≈ 2.3 mm
    // and right vertices x≈7.5 mm - 0.2 mm ≈ 7.3 mm — both clearly on their respective
    // sides of the 5.0 mm bridge boundary.
    const BRIDGE_BOUNDARY_MM: f32 = 5.0;

    for wall in &outer_walls {
        assert!(
            wall.path.points.len() >= 5,
            "outer wall path must have N+1 points (closing repeat); got {}",
            wall.path.points.len()
        );
        assert_eq!(
            wall.path.points.len(),
            wall.feature_flags.len(),
            "path.points and feature_flags must be parallel"
        );

        // Collect distinct vertices (exclude the closing-repeat last point).
        let n = wall.path.points.len() - 1; // number of distinct vertices
        let vertex_data: Vec<(f32, bool)> = wall.path.points[..n]
            .iter()
            .zip(wall.feature_flags[..n].iter())
            .map(|(pt, flags)| (pt.x, flags.is_bridge))
            .collect();

        assert_eq!(
            vertex_data.len(),
            4,
            "fixture polygon has 4 vertices; got {}",
            vertex_data.len()
        );

        // Classify each vertex by emitted x relative to the bridge boundary.
        let bridge_verts: Vec<f32> = vertex_data
            .iter()
            .filter(|(_, is_bridge)| *is_bridge)
            .map(|(x, _)| *x)
            .collect();
        let non_bridge_verts: Vec<f32> = vertex_data
            .iter()
            .filter(|(_, is_bridge)| !is_bridge)
            .map(|(x, _)| *x)
            .collect();

        assert_eq!(
            bridge_verts.len(),
            2,
            "exactly 2 vertices must be bridge; got bridge={bridge_verts:?} non_bridge={non_bridge_verts:?}"
        );
        assert_eq!(
            non_bridge_verts.len(),
            2,
            "exactly 2 vertices must NOT be bridge; got bridge={bridge_verts:?} non_bridge={non_bridge_verts:?}"
        );

        // All bridge vertices must be on the right side (x > BRIDGE_BOUNDARY_MM).
        for &x in &bridge_verts {
            assert!(
                x > BRIDGE_BOUNDARY_MM,
                "bridge vertex x={x:.4} mm must be > {BRIDGE_BOUNDARY_MM} mm (right half)"
            );
        }
        // All non-bridge vertices must be on the left side (x < BRIDGE_BOUNDARY_MM).
        for &x in &non_bridge_verts {
            assert!(
                x < BRIDGE_BOUNDARY_MM,
                "non-bridge vertex x={x:.4} mm must be < {BRIDGE_BOUNDARY_MM} mm (left half)"
            );
        }
    }
}
