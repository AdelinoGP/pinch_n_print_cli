//! TDD tests for packet 36: Bridge Infill Emission (Orca Parity).
//!
//! AC-5: When `SliceRegionView.bridge_areas` is non-empty and
//! `bridge_orientation_deg` is set, the rectilinear-infill module must emit
//! `InfillIR` paths with `role == ExtrusionRole::BridgeInfill` and direction
//! within ±1° of `bridge_orientation_deg`.
//!
//! Coordinate system: 1 unit = 100 nm (10⁻⁴ mm) per docs/08_coordinate_system.md.

use slicer_ir::{ConfigView, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::prelude::LayerModule;
use slicer_sdk::test_prelude::*;
use slicer_sdk::views::SliceRegionView;

use rectilinear_infill::RectilinearInfill;

fn empty_paint_view() -> slicer_sdk::traits::PaintRegionLayerView {
    slicer_sdk::traits::PaintRegionLayerView::new(0)
}

/// Create a region with bridge areas and orientation set.
fn make_bridge_region(bridge_orientation_deg: f32) -> SliceRegionView {
    let s = square_polygon(5.0, 5.0, 10.0);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("test_object")
        .region_id(0)
        .add_infill_area(s.clone())
        .effective_layer_height(0.2)
        .z(1.0)
        .has_nonplanar(false)
        .is_bridge(true)
        .bridge_areas(vec![s])
        .bridge_orientation_deg(bridge_orientation_deg)
        .build();
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);
    region
}

/// Add the four rectilinear held claims to a region built inline.
fn with_rectilinear_claims(mut region: SliceRegionView) -> SliceRegionView {
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);
    region
}

/// Compute the direction angle (degrees, 0-360) of a path from its first two points.
fn path_direction_deg(path: &slicer_ir::ExtrusionPath3D) -> f32 {
    assert!(
        path.points.len() >= 2,
        "path must have at least 2 points to compute direction"
    );
    let p1 = &path.points[0];
    let p2 = &path.points[1];
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let angle = dy.atan2(dx).to_degrees();
    // Normalize to [0, 360)
    if angle < 0.0 {
        angle + 360.0
    } else {
        angle
    }
}

/// Returns the smallest angular difference between `actual` and `expected` in degrees.
/// Accounts for wrap-around at 0/360.
fn angle_diff_deg(actual: f32, expected: f32) -> f32 {
    let mut diff = (actual - expected).abs();
    if diff > 180.0 {
        diff = 360.0 - diff;
    }
    diff
}

/// Helper: create a rectangular ExPolygon in mm-unit coordinates.
fn rect_expoly_mm(x0: i32, y0: i32, x1: i32, y1: i32) -> slicer_ir::ExPolygon {
    let u = slicer_ir::mm_to_units;
    let contour = Polygon {
        points: vec![
            Point2 {
                x: u(x0 as f32),
                y: u(y0 as f32),
            },
            Point2 {
                x: u(x1 as f32),
                y: u(y0 as f32),
            },
            Point2 {
                x: u(x1 as f32),
                y: u(y1 as f32),
            },
            Point2 {
                x: u(x0 as f32),
                y: u(y1 as f32),
            },
        ],
    };
    slicer_ir::ExPolygon {
        contour,
        holes: vec![],
    }
}

/// AC-5: bridge_areas_emit_bridge_infill_at_oriented_angle
///
/// A `SliceRegionView` with non-empty `bridge_areas` and a known
/// `bridge_orientation_deg`. When rectilinear-infill runs, the emitted
/// `InfillIR` must contain at least one path with:
///   - `role == ExtrusionRole::BridgeInfill`
///   - direction within ±1° of `bridge_orientation_deg`
#[test]
fn bridge_areas_emit_bridge_infill_at_oriented_angle() {
    let bridge_angle = 45.0_f32;
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_bridge_region(bridge_angle);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    // Must have at least one BridgeInfill path
    let bridge_paths: Vec<_> = all_paths
        .iter()
        .filter(|p| p.role == ExtrusionRole::BridgeInfill && p.points.len() >= 2)
        .collect();

    assert!(
        !bridge_paths.is_empty(),
        "expected at least one BridgeInfill path, got none. All paths: {:?}",
        all_paths
    );

    // At least one BridgeInfill path must be within ±1° of bridge_angle
    let any_within_tolerance = bridge_paths.iter().any(|p| {
        let dir = path_direction_deg(p);
        angle_diff_deg(dir, bridge_angle) <= 1.0
    });

    assert!(
        any_within_tolerance,
        "expected at least one BridgeInfill path within ±1° of {}°, \
         but none matched. Bridge paths: {:?}",
        bridge_angle, bridge_paths
    );
}

/// AC-8: straddling_expoly_partitioned_via_set_difference
///
/// infill_areas = [0,0]–[20,20], bridge_areas = [5,5]–[15,15].
/// BridgeInfill paths must lie inside [5,5]–[15,15]; SparseInfill paths
/// must lie inside [0,0]–[20,20] \ [5,5]–[15,15]; no overlap between roles.
#[test]
fn straddling_expoly_partitioned_via_set_difference() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();

    let outer = rect_expoly_mm(0, 0, 20, 20);
    let bridge = rect_expoly_mm(5, 5, 15, 15);

    let region = with_rectilinear_claims(
        SliceRegionViewBuilder::new()
            .object_id("test_object")
            .region_id(0)
            .add_infill_area(outer)
            .effective_layer_height(0.2)
            .z(1.0)
            .has_nonplanar(false)
            .is_bridge(true)
            .bridge_areas(vec![bridge])
            .bridge_orientation_deg(0.0)
            .build(),
    );

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    let bridge_paths: Vec<_> = all_paths
        .iter()
        .filter(|p| p.role == ExtrusionRole::BridgeInfill)
        .collect();
    let sparse_paths: Vec<_> = all_paths
        .iter()
        .filter(|p| p.role == ExtrusionRole::SparseInfill)
        .collect();

    // Tolerance: 0.5 mm
    let tol = 0.5_f32;

    // All BridgeInfill midpoints must lie inside [5,5]–[15,15] ± tol
    for p in &bridge_paths {
        for seg in p.points.windows(2) {
            let mx = (seg[0].x + seg[1].x) / 2.0;
            let my = (seg[0].y + seg[1].y) / 2.0;
            assert!(
                mx >= 5.0 - tol && mx <= 15.0 + tol && my >= 5.0 - tol && my <= 15.0 + tol,
                "AC-8: BridgeInfill midpoint ({mx:.2},{my:.2}) outside [5,5]–[15,15] ± {tol}"
            );
        }
    }

    // All SparseInfill midpoints must lie inside [0,0]–[20,20] but outside [5,5]–[15,15] ± tol
    for p in &sparse_paths {
        for seg in p.points.windows(2) {
            let mx = (seg[0].x + seg[1].x) / 2.0;
            let my = (seg[0].y + seg[1].y) / 2.0;
            assert!(
                mx >= 0.0 - tol && mx <= 20.0 + tol && my >= 0.0 - tol && my <= 20.0 + tol,
                "AC-8: SparseInfill midpoint ({mx:.2},{my:.2}) outside [0,0]–[20,20] ± {tol}"
            );
            let in_bridge_zone =
                mx >= 5.0 - tol && mx <= 15.0 + tol && my >= 5.0 - tol && my <= 15.0 + tol;
            assert!(
                !in_bridge_zone,
                "AC-8: SparseInfill midpoint ({mx:.2},{my:.2}) overlaps bridge zone [5,5]–[15,15]"
            );
        }
    }
}

/// AC-9: bridge_paths_use_bridge_orientation_not_sparse_alternation
///
/// layer_index=1 would alternate sparse to 90°. bridge_orientation_deg=37°.
/// Every BridgeInfill path must be within ±1° of 37°, not 0° or 90°.
#[test]
fn bridge_paths_use_bridge_orientation_not_sparse_alternation() {
    let bridge_angle = 37.0_f32;
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();

    let outer = rect_expoly_mm(0, 0, 20, 20);
    let bridge_rect = rect_expoly_mm(2, 2, 18, 18);

    let region = with_rectilinear_claims(
        SliceRegionViewBuilder::new()
            .object_id("test_object")
            .region_id(0)
            .add_infill_area(outer)
            .effective_layer_height(0.2)
            .z(1.0)
            .has_nonplanar(false)
            .is_bridge(true)
            .bridge_areas(vec![bridge_rect])
            .bridge_orientation_deg(bridge_angle)
            .build(),
    );

    let mut output = InfillOutputBuilder::new();
    // layer_index=1 → sparse alternation would be 90°
    module
        .run_infill(
            1,
            &[region],
            &empty_paint_view(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    let bridge_paths: Vec<_> = all_paths
        .iter()
        .filter(|p| p.role == ExtrusionRole::BridgeInfill && p.points.len() >= 2)
        .collect();

    assert!(
        !bridge_paths.is_empty(),
        "AC-9: expected at least one BridgeInfill path, got none"
    );

    for p in &bridge_paths {
        let dir = path_direction_deg(p);
        assert!(
            angle_diff_deg(dir, bridge_angle) <= 1.0,
            "AC-9: BridgeInfill path direction {dir:.1}° is not within ±1° of {bridge_angle}°"
        );
        assert!(
            angle_diff_deg(dir, 0.0) > 1.0 || angle_diff_deg(dir, bridge_angle) <= 1.0,
            "AC-9: BridgeInfill path at sparse-alternation angle 0°"
        );
        assert!(
            angle_diff_deg(dir, 90.0) > 1.0,
            "AC-9: BridgeInfill path at sparse-alternation angle 90°"
        );
    }
}

/// NEG-2: empty_bridge_areas_emits_no_bridge_infill_even_when_is_bridge_true
///
/// is_bridge=true but bridge_areas is empty. Module must emit zero BridgeInfill paths.
#[test]
fn empty_bridge_areas_emits_no_bridge_infill_even_when_is_bridge_true() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();

    let region = with_rectilinear_claims(
        SliceRegionViewBuilder::new()
            .object_id("test_object")
            .region_id(0)
            .add_infill_area(rect_expoly_mm(0, 0, 20, 20))
            .effective_layer_height(0.2)
            .z(1.0)
            .has_nonplanar(false)
            .is_bridge(true)
            // bridge_areas intentionally left empty
            .build(),
    );

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(
            0,
            &[region],
            &empty_paint_view(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    let bridge_count = all_paths
        .iter()
        .filter(|p| p.role == ExtrusionRole::BridgeInfill)
        .count();

    assert_eq!(
        bridge_count, 0,
        "NEG-2: expected zero BridgeInfill paths when bridge_areas is empty, got {bridge_count}"
    );
}
