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
    let module = RectilinearInfill::from_config(&ConfigView::new()).unwrap();
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

/// Canonical `Fill::make_fills` uses bridge flow and full density for a bridge
/// surface; it must not reuse sparse infill's density or line width.
#[test]
fn bridge_fill_uses_bridge_width_and_full_density() {
    let config = ConfigViewBuilder::new()
        .float("sparse_infill_density", 20.0)
        .float("line_width", 0.4)
        .float("bridge_line_width", 0.8)
        .float("bridge_flow", 0.7)
        .float("bridge_speed", 25.0)
        .build();
    let bridge = rect_expoly_mm(0, 0, 10, 10);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("test_object")
        .region_id(0)
        .add_infill_area(bridge.clone())
        .effective_layer_height(0.2)
        .z(1.0)
        .has_nonplanar(false)
        .is_bridge(true)
        .bridge_areas(vec![bridge])
        .bridge_orientation_deg(0.0)
        .build();
    region.set_held_claims(vec!["claim:bridge-fill".into()]);

    let module = RectilinearInfill::from_config(&config).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let bridge_paths: Vec<_> = output
        .solid_paths()
        .iter()
        .filter(|path| path.role == ExtrusionRole::BridgeInfill)
        .collect();
    assert!(
        bridge_paths.len() > 6,
        "bridge fill must use full density, got only {} paths",
        bridge_paths.len()
    );
    assert!(
        bridge_paths
            .iter()
            .flat_map(|path| path.points.iter())
            .all(|point| (point.width - 0.8).abs() < 1e-5),
        "bridge fill must use bridge_line_width rather than sparse line_width"
    );
    assert!(
        bridge_paths
            .iter()
            .all(|path| (path.speed_factor - 1.0).abs() < 1e-5),
        "bridge fill must use bridge_speed rather than sparse_infill_speed"
    );
    assert!(
        bridge_paths
            .iter()
            .flat_map(|path| path.points.iter())
            .all(|point| (point.flow_factor - 0.7).abs() < 1e-5),
        "thin bridge fill must use bridge_flow"
    );
}

/// Internal bridge surfaces use the distinct Orca role and their own
/// density, flow, and speed settings while retaining the bridge claim.
#[test]
fn internal_bridge_uses_internal_role_settings() {
    let config = ConfigViewBuilder::new()
        .float("line_width", 0.4)
        .float("bridge_line_width", 0.4)
        .float("bridge_speed", 25.0)
        .float("internal_bridge_speed", 37.5)
        .float("internal_bridge_density", 0.5)
        .float("internal_bridge_flow", 0.8)
        .bool("thick_internal_bridges", false)
        .build();
    let bridge = rect_expoly_mm(0, 0, 10, 10);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("test_object")
        .region_id(0)
        .add_infill_area(bridge.clone())
        .effective_layer_height(0.2)
        .z(1.0)
        .is_bridge(true)
        .is_internal_bridge(true)
        .bridge_areas(vec![bridge])
        .bridge_orientation_deg(0.0)
        .build();
    region.set_held_claims(vec!["claim:bridge-fill".into()]);

    let module = RectilinearInfill::from_config(&config).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.solid_paths();
    assert!(!paths.is_empty());
    assert!(paths.iter().all(|path| {
        path.role == ExtrusionRole::InternalBridgeInfill
            && (path.speed_factor - 1.0).abs() < 1e-5
            && path
                .points
                .iter()
                .all(|point| (point.flow_factor - 0.8).abs() < 1e-5)
    }));
    assert!(
        paths.len() < 30,
        "50% internal density should space bridge lines, got {}",
        paths.len()
    );
}

/// AC-8: straddling_expoly_partitioned_via_set_difference
///
/// infill_areas = [0,0]–[20,20], bridge_areas = [5,5]–[15,15].
/// BridgeInfill paths must lie inside [5,5]–[15,15]; SparseInfill paths
/// must lie inside [0,0]–[20,20] \ [5,5]–[15,15]; no overlap between roles.
#[test]
fn straddling_expoly_partitioned_via_set_difference() {
    let module = RectilinearInfill::from_config(&ConfigView::new()).unwrap();

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
    let module = RectilinearInfill::from_config(&ConfigView::new()).unwrap();

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
    let module = RectilinearInfill::from_config(&ConfigView::new()).unwrap();

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

/// Run the module over a single 10×10 mm bridge region and return the bridge
/// paths it emitted, using the caller's config.
///
/// `internal` selects the internal-bridge flag, which is what steers
/// `internal_bridge_density` / `thick_internal_bridges` against their external
/// twins inside `run_infill`.
fn bridge_paths_for(config: &ConfigView, internal: bool) -> Vec<slicer_ir::ExtrusionPath3D> {
    let bridge = rect_expoly_mm(0, 0, 10, 10);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("test_object")
        .region_id(0)
        .add_infill_area(bridge.clone())
        .effective_layer_height(0.2)
        .z(1.0)
        .has_nonplanar(false)
        .is_bridge(true)
        .is_internal_bridge(internal)
        .bridge_areas(vec![bridge])
        .bridge_orientation_deg(0.0)
        .build();
    region.set_held_claims(vec!["claim:bridge-fill".into()]);

    let module = RectilinearInfill::from_config(config).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, config)
        .unwrap();

    let role = if internal {
        ExtrusionRole::InternalBridgeInfill
    } else {
        ExtrusionRole::BridgeInfill
    };
    output
        .solid_paths()
        .iter()
        .filter(|path| path.role == role)
        .cloned()
        .collect()
}

/// Canonical `Fill::make_fills` overrides an external bridge's fill density
/// with `bridge_density` (`params.density = bridge_density.get_abs_value(1.0)`),
/// and the filler divides line spacing by that density. Halving the key must
/// therefore roughly halve the number of external bridge lines.
#[test]
fn bridge_density_spaces_external_bridge_lines() {
    let full = ConfigViewBuilder::new()
        .float("line_width", 0.4)
        .float("bridge_line_width", 0.4)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", false)
        .float("bridge_density", 1.0)
        .build();
    let half = ConfigViewBuilder::new()
        .float("line_width", 0.4)
        .float("bridge_line_width", 0.4)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", false)
        .float("bridge_density", 0.5)
        .build();

    let full_paths = bridge_paths_for(&full, false).len();
    let half_paths = bridge_paths_for(&half, false).len();

    assert!(
        full_paths > 4,
        "100% bridge density must emit a solid bridge, got {full_paths} paths"
    );
    let ratio = half_paths as f32 / full_paths as f32;
    assert!(
        (0.4..=0.65).contains(&ratio),
        "50% bridge_density must roughly halve the bridge line count: \
         {half_paths} of {full_paths} (ratio {ratio})"
    );
}

/// Canonical `Fill::make_fills` picks the bridging flow for an internal bridge
/// from `thick_internal_bridges` (`is_thick_bridge` → `layerm.bridging_flow`),
/// so the key changes bridge line *spacing*: the thick path uses the round
/// thread's diameter plus `BRIDGE_EXTRA_SPACING`, the thin path uses the
/// flattened extrusion's spacing, which is narrower. Fewer lines must therefore
/// be emitted with the key on than with it off, at identical density.
#[test]
fn thick_internal_bridges_widens_internal_bridge_spacing() {
    let make = |thick: bool| {
        ConfigViewBuilder::new()
            .float("line_width", 0.4)
            .float("bridge_line_width", 0.4)
            .float("internal_bridge_flow", 1.0)
            .float("internal_bridge_density", 1.0)
            .bool("thick_internal_bridges", thick)
            .build()
    };

    let thick_paths = bridge_paths_for(&make(true), true).len();
    let thin_paths = bridge_paths_for(&make(false), true).len();

    assert!(
        thin_paths > 0 && thick_paths > 0,
        "both settings must still fill the bridge: thick {thick_paths}, thin {thin_paths}"
    );
    assert!(
        thick_paths < thin_paths,
        "thick_internal_bridges must widen bridge spacing (fewer lines): \
         thick {thick_paths} vs thin {thin_paths}"
    );
}
