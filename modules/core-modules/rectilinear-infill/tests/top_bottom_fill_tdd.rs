//! Top/bottom/bridge surface fill role tests for rectilinear-infill.
//!
//! These tests verify that the rectilinear-infill module correctly emits
//! ExtrusionRole variants based on SliceRegionView surface classification signals.
//!
//! All four tests are intentionally FAILING (TDD approach) because the role
//! logic is not yet implemented in the module.

use slicer_ir::{ConfigView, ExPolygon, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::prelude::LayerModule;
use slicer_sdk::views::SliceRegionView;

use rectilinear_infill::RectilinearInfill;

/// Create a minimal rectangular ExPolygon: 10mm x 10mm square from (0,0) to (10,10).
/// Using mm_to_units ensures scan-line intersections occur.
fn make_square_expolygon() -> ExPolygon {
    let u = slicer_ir::mm_to_units;
    let contour = Polygon {
        points: vec![
            Point2 {
                x: u(0.0),
                y: u(0.0),
            },
            Point2 {
                x: u(10.0),
                y: u(0.0),
            },
            Point2 {
                x: u(10.0),
                y: u(10.0),
            },
            Point2 {
                x: u(0.0),
                y: u(10.0),
            },
        ],
    };
    ExPolygon {
        contour,
        holes: vec![],
    }
}

fn make_test_region(is_top: bool, is_bottom: bool, is_bridge: bool) -> SliceRegionView {
    let square = make_square_expolygon();
    let mut region = SliceRegionView::new(
        "test_object".to_string(),
        0,
        vec![],
        vec![square],
        0.2,
        1.0,
        false,
    );
    region.set_is_top_surface(is_top);
    region.set_is_bottom_surface(is_bottom);
    region.set_is_bridge(is_bridge);
    region
}

/// Helper: returns true if any path in `paths` has the given role AND > 1 point.
fn has_path_with_role(paths: &[slicer_ir::ExtrusionPath3D], role: ExtrusionRole) -> bool {
    paths.iter().any(|p| p.role == role && p.points.len() > 1)
}

/// Helper: returns true if NO path in `paths` uses any of the three surface roles.
fn no_surface_fill_roles(paths: &[slicer_ir::ExtrusionPath3D]) -> bool {
    !paths.iter().any(|p| {
        matches!(
            p.role,
            ExtrusionRole::TopSolidInfill
                | ExtrusionRole::BottomSolidInfill
                | ExtrusionRole::BridgeInfill
        )
    })
}

// ---------------------------------------------------------------------------
// Test 1: top_surface_region_emits_top_solid_infill
// ---------------------------------------------------------------------------
#[test]
fn top_surface_region_emits_top_solid_infill() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_test_region(true, false, false);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &ConfigView::new())
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::TopSolidInfill),
        "expected at least one path with role=TopSolidInfill, got paths: {:?}",
        all_paths
    );
}

// ---------------------------------------------------------------------------
// Test 2: bottom_surface_region_emits_bottom_solid_infill
// ---------------------------------------------------------------------------
#[test]
fn bottom_surface_region_emits_bottom_solid_infill() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_test_region(false, true, false);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &ConfigView::new())
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::BottomSolidInfill),
        "expected at least one path with role=BottomSolidInfill, got paths: {:?}",
        all_paths
    );
}

// ---------------------------------------------------------------------------
// Test 3: bridge_surface_region_emits_bridge_infill_role
// ---------------------------------------------------------------------------
#[test]
fn bridge_surface_region_emits_bridge_infill_role() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_test_region(false, false, true);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &ConfigView::new())
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    // Must use BridgeInfill, NOT downgraded to SparseInfill
    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::BridgeInfill),
        "expected at least one path with role=BridgeInfill (not SparseInfill), got paths: {:?}",
        all_paths
    );
}

// ---------------------------------------------------------------------------
// Test 4: sparse_only_region_does_not_fabricate_surface_fill_roles
// ---------------------------------------------------------------------------
#[test]
fn sparse_only_region_does_not_fabricate_surface_fill_roles() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_test_region(false, false, false);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &ConfigView::new())
        .unwrap();

    let all_paths: Vec<_> = output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect();

    // Solid paths must be empty
    assert!(
        output.solid_paths().is_empty(),
        "expected solid_paths to be empty for sparse-only region, got {:?}",
        output.solid_paths()
    );

    // No path may use TopSolidInfill, BottomSolidInfill, or BridgeInfill
    assert!(
        no_surface_fill_roles(&all_paths),
        "expected NO surface fill roles (TopSolidInfill/BottomSolidInfill/BridgeInfill) \
         for sparse-only region, got paths: {:?}",
        all_paths
    );
}
