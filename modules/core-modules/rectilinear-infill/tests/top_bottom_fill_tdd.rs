//! Top/bottom/bridge surface fill role tests for rectilinear-infill.
//!
//! These tests verify that the rectilinear-infill module correctly emits
//! ExtrusionRole variants based on SliceRegionView surface classification signals.
//!
//! All four tests are intentionally FAILING (TDD approach) because the role
//! logic is not yet implemented in the module.

use slicer_ir::{ConfigView, ExPolygon, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::prelude::LayerModule;
use slicer_sdk::test_prelude::*;
use slicer_sdk::views::SliceRegionView;

use rectilinear_infill::RectilinearInfill;

/// Create a minimal rectangular ExPolygon: 10mm x 10mm square from (0,0) to (10,10).
/// Using mm_to_units ensures scan-line intersections occur.
fn make_square_expolygon() -> ExPolygon {
    square_polygon(5.0, 5.0, 10.0)
}

#[rustfmt::skip]
#[allow(clippy::suspicious_else_formatting)]
fn make_test_region(is_top: bool, is_bottom: bool, is_bridge: bool) -> SliceRegionView {
    let s = square_polygon(5.0, 5.0, 10.0); let mut r = SliceRegionViewBuilder::new().object_id("test_object").region_id(0).add_infill_area(s.clone()).effective_layer_height(0.2).z(1.0).has_nonplanar(false).build();
    if is_top { r.set_top_shell_index(Some(0)); r.set_top_solid_fill(vec![s.clone()]); }; if is_bottom { r.set_bottom_shell_index(Some(0)); r.set_bottom_solid_fill(vec![s]); }; r.set_is_bridge(is_bridge); r
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
    let mut region = make_test_region(false, false, true);
    // rev1 contract: is_bridge requires non-empty bridge_areas to emit BridgeInfill.
    region.set_bridge_areas(vec![make_square_expolygon()]);
    region.set_bridge_orientation_deg(0.0);
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
// Test 3b: bottom_wins_over_top_on_overlap (OrcaSlicer parity)
// ---------------------------------------------------------------------------
#[test]
fn bottom_wins_over_top_on_overlap() {
    // Single-layer region: both top_shell_index and bottom_shell_index = Some(0).
    // Per OrcaSlicer's detect_surfaces_type convention, BottomSolidInfill wins on
    // overlap (see DEVIATION_LOG.md). The pinch_n_print pre-refactor convention
    // was top-wins; this test pins the new behavior.
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_test_region(true, true, false);
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
        "expected BottomSolidInfill (bottom wins on overlap), got paths: {:?}",
        all_paths
    );
    assert!(
        !has_path_with_role(&all_paths, ExtrusionRole::TopSolidInfill),
        "expected NO TopSolidInfill when both shell indices = Some(0); got paths: {:?}",
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
