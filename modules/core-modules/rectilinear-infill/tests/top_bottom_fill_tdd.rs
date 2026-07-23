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

fn empty_paint_view() -> slicer_sdk::traits::PaintRegionLayerView {
    slicer_sdk::traits::PaintRegionLayerView::new(0)
}

/// Create a minimal rectangular ExPolygon: 10mm x 10mm square from (0,0) to (10,10).
/// Using mm_to_units ensures scan-line intersections occur.
fn make_square_expolygon() -> ExPolygon {
    square_polygon(5.0, 5.0, 10.0)
}

// Post-host-partition fixture: each role's canonical polygon is populated
// independently of the others. The host's `sync_perimeter_infill_areas_into_slice`
// already enforces the bridge > bottom > top > sparse precedence and the
// pairwise-disjoint invariant, so this fixture mirrors what the modules see
// AFTER that hook runs. `sparse_infill_area` is only populated when neither
// top/bottom nor bridge applies — matching the post-partition remainder.
fn make_test_region(is_top: bool, is_bottom: bool, is_bridge: bool) -> SliceRegionView {
    let s = square_polygon(5.0, 5.0, 10.0);
    let sparse = if !is_top && !is_bottom && !is_bridge {
        vec![s.clone()]
    } else {
        Vec::new()
    };
    let mut region = SliceRegionViewBuilder::new()
        .object_id("test_object")
        .region_id(0)
        .add_infill_area(s.clone())
        .sparse_infill_area(sparse)
        .effective_layer_height(0.2)
        .z(1.0)
        .has_nonplanar(false)
        .top_shell_index(if is_top { Some(0) } else { None })
        .top_solid_fill(if is_top { vec![s.clone()] } else { vec![] })
        .bottom_shell_index(if is_bottom { Some(0) } else { None })
        .bottom_solid_fill(if is_bottom { vec![s] } else { vec![] })
        .is_bridge(is_bridge)
        .build();
    // Rectilinear manifest declares all four fill claims.
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);
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

// ---------------------------------------------------------------------------
// Test 1: top_surface_region_emits_top_solid_infill
// ---------------------------------------------------------------------------
#[test]
fn top_surface_region_emits_top_solid_infill() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_test_region(true, false, false);
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
    // rev1 contract: is_bridge requires non-empty bridge_areas to emit BridgeInfill.
    let region = with_rectilinear_claims(
        SliceRegionViewBuilder::new()
            .object_id("test_object")
            .region_id(0)
            .add_infill_area(make_square_expolygon())
            .effective_layer_height(0.2)
            .z(1.0)
            .has_nonplanar(false)
            .is_bridge(true)
            .bridge_areas(vec![make_square_expolygon()])
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

    // Must use BridgeInfill, NOT downgraded to SparseInfill
    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::BridgeInfill),
        "expected at least one path with role=BridgeInfill (not SparseInfill), got paths: {:?}",
        all_paths
    );
}

// ---------------------------------------------------------------------------
// Test 3b: bottom_wins_over_top_on_overlap (host-enforced precedence)
//
// Original (pre-partition refactor) intent: when both top_shell_index and
// bottom_shell_index = Some(0), BottomSolidInfill should win on overlap to
// match OrcaSlicer's `detect_surfaces_type`. That precedence is now enforced
// HOST-SIDE in `sync_perimeter_infill_areas_into_slice` (see
// `crates/slicer-runtime/src/region_partition.rs` and
// `crates/slicer-runtime/tests/integration/region_partition_tdd.rs::ac2_*`),
// so by the time a fixture reaches the module, `top_solid_fill` has already
// been subtracted by `bottom_solid_fill`. This test reflects that: when the
// post-host state has bottom populated and top empty, only BottomSolidInfill
// is emitted — exactly the parity behaviour, just enforced one stage earlier.
// ---------------------------------------------------------------------------
#[test]
fn bottom_wins_over_top_on_overlap() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    // Post-host-partition state for a layer-0 region (both shell zones touch
    // it): bottom polygon is populated, top has been subtracted to empty.
    let s = square_polygon(5.0, 5.0, 10.0);
    let region = with_rectilinear_claims(
        SliceRegionViewBuilder::new()
            .object_id("test_object")
            .region_id(0)
            .add_infill_area(s.clone())
            .effective_layer_height(0.2)
            .z(1.0)
            .has_nonplanar(false)
            .top_shell_index(Some(0))
            .bottom_shell_index(Some(0))
            // top_solid_fill empty post-precedence-dedup; bottom carries the area.
            .top_solid_fill(Vec::new())
            .bottom_solid_fill(vec![s])
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

    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::BottomSolidInfill),
        "expected BottomSolidInfill (bottom wins on overlap, host-enforced), got paths: {:?}",
        all_paths
    );
    assert!(
        !has_path_with_role(&all_paths, ExtrusionRole::TopSolidInfill),
        "expected NO TopSolidInfill after host precedence dedup; got paths: {:?}",
        all_paths
    );
}

// ---------------------------------------------------------------------------
// Test 3c/3d (RC4/G4): deeper top/bottom shell layers (depth >= 1) emit
// InternalSolidInfill, NOT the exposed-surface role. Depth 0 stays Top/Bottom
// surface (covered by tests 1/2); depth >= 1 is the solid shell beneath the
// surface and must become ;TYPE:Internal solid infill so the surface is
// supported by solid layers (OrcaSlicer stInternalSolid).
// ---------------------------------------------------------------------------
fn make_shell_region(top_index: Option<u8>, bottom_index: Option<u8>) -> SliceRegionView {
    let s = square_polygon(5.0, 5.0, 10.0);
    let region = SliceRegionViewBuilder::new()
        .object_id("test_object")
        .region_id(0)
        .add_infill_area(s.clone())
        .effective_layer_height(0.2)
        .z(1.0)
        .has_nonplanar(false)
        .top_shell_index(top_index)
        .top_solid_fill(if top_index.is_some() {
            vec![s.clone()]
        } else {
            vec![]
        })
        .bottom_shell_index(bottom_index)
        .bottom_solid_fill(if bottom_index.is_some() {
            vec![s]
        } else {
            vec![]
        })
        .build();
    with_rectilinear_claims(region)
}

#[test]
fn deep_top_shell_emits_internal_solid_infill() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_shell_region(Some(1), None);
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

    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::InternalSolidInfill),
        "depth-1 top shell must emit InternalSolidInfill, got: {:?}",
        all_paths
    );
    assert!(
        !has_path_with_role(&all_paths, ExtrusionRole::TopSolidInfill),
        "depth-1 top shell must NOT emit TopSolidInfill (that is depth 0 only), got: {:?}",
        all_paths
    );
}

#[test]
fn deep_bottom_shell_emits_internal_solid_infill() {
    let module = RectilinearInfill::on_print_start(&ConfigView::new()).unwrap();
    let region = make_shell_region(None, Some(2));
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

    assert!(
        has_path_with_role(&all_paths, ExtrusionRole::InternalSolidInfill),
        "depth-2 bottom shell must emit InternalSolidInfill, got: {:?}",
        all_paths
    );
    assert!(
        !has_path_with_role(&all_paths, ExtrusionRole::BottomSolidInfill),
        "depth-2 bottom shell must NOT emit BottomSolidInfill (depth 0 only), got: {:?}",
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
