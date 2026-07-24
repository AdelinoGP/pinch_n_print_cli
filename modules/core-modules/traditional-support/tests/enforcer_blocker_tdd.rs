//! TDD tests for TASK-095: traditional-support enforcer/blocker paint semantics.
//!
//! After packet 95 closure (D14): paint annotations travel on
//! `SliceIR.regions[*].segment_annotations`.  Hosts attach the SliceIR to
//! `PaintRegionLayerView::with_slice_ir`, and modules read enforcer / blocker
//! policy via `PaintRegionLayerView::paint_policy_for`.
//!
//! Authoritative docs:
//! - docs/01_system_architecture.md §"Layer::Support"
//! - docs/02_ir_schemas.md (SlicedRegion.segment_annotations)
//! - docs/10_scenario_traces.md §"Scenario Trace 2" (blocker > enforcer precedence)

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, PaintSemantic, PaintValue, Point2, Polygon, SliceIR, SlicedRegion,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView, SupportPaintPolicy};
use slicer_sdk::views::SliceRegionView;

use traditional_support::TraditionalSupport;

/// Helper: create an enabled support config.
fn enabled_config() -> ConfigView {
    ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_density", 0.2)
        .float("support_angle", 0.0)
        .float("support_speed", 50.0)
        .float("line_width", 0.4)
        .build()
}

/// Helper: create a 10mm square ExPolygon centered at origin.
fn square_expoly() -> ExPolygon {
    square_polygon(0.0, 0.0, 10.0)
}

/// Helper: create a SliceRegionView with the standard square.
fn square_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(z)
        .add_polygon(square_expoly())
        .build()
}

fn enclosing_square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, -10.0),
                Point2::from_mm(10.0, -10.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(-10.0, 10.0),
            ],
        },
        holes: vec![],
    }
}

fn region_with_annotations(polygons: Vec<ExPolygon>, semantics: &[PaintSemantic]) -> SlicedRegion {
    let mut segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
        HashMap::new();
    for sem in semantics {
        segment_annotations.insert(sem.clone(), vec![vec![Some(PaintValue::Flag(true))]]);
    }
    SlicedRegion {
        object_id: "obj1".to_string(),
        region_id: 1u64,
        polygons,
        segment_annotations,
        ..Default::default()
    }
}

fn paint_view_with_annotations(z: f32, semantics: &[PaintSemantic]) -> PaintRegionLayerView {
    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z,
        regions: vec![region_with_annotations(vec![enclosing_square()], semantics)],
    };
    PaintRegionLayerView::new(0).with_slice_ir(Arc::new(slice))
}

/// Test 1: A fully blocked region generates zero support paths.
#[test]
fn fully_blocked_region_generates_zero_support() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(true);

    let paint = paint_view_with_annotations(0.3, &[PaintSemantic::SupportBlocker]);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "SupportBlocker annotation must suppress support generation (D14 + blocker precedence)"
    );
}

/// Test 2: A fully enforced region generates support paths at 0-degree overhang.
#[test]
fn fully_enforced_region_generates_support_at_zero_overhang() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(false);

    let paint = paint_view_with_annotations(0.3, &[PaintSemantic::SupportEnforcer]);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.support_paths().is_empty(),
        "SupportEnforcer annotation must force support generation (D14)"
    );
}

/// Test 3: A region that is both blocked and enforced generates zero support
/// (blocker takes precedence over enforcer).
#[test]
fn blocked_plus_enforced_resolves_to_zero_support() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(true);

    let paint = paint_view_with_annotations(
        0.3,
        &[
            PaintSemantic::SupportBlocker,
            PaintSemantic::SupportEnforcer,
        ],
    );

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "blocker > enforcer precedence: zero support when both annotations apply"
    );
}

/// Test 4: Unpainted regions keep existing behaviour — support is generated
/// normally when no SupportBlocker or SupportEnforcer paint is present.
#[test]
fn unpainted_region_keeps_existing_behaviour() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let region = square_region(0.3);

    // No paint data at all
    let paint = PaintRegionLayerView::new(0);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.support_paths().is_empty(),
        "unpainted region should still generate support (existing behaviour)"
    );
}

// ── SurfaceClassificationIR-driven default eligibility ────────────────────
// docs/02_ir_schemas.md and docs/01_system_architecture.md §"Layer::Support".

/// Default eligibility: `needs_support=false` and no paint → zero support.
#[test]
fn default_ineligible_region_generates_zero_support() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(false);

    let paint = PaintRegionLayerView::new(0);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "needs_support=false with no paint must yield zero support paths",
    );
}

/// Default eligibility: `needs_support=true` and no paint → support generated.
#[test]
fn default_eligible_region_generates_support() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(true);

    let paint = PaintRegionLayerView::new(0);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.support_paths().is_empty(),
        "needs_support=true with no paint must yield support paths",
    );
}

/// Test 7: Enforcer overrides `needs_support=false`.
#[test]
fn enforcer_overrides_needs_support_false() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(false);

    let paint = paint_view_with_annotations(0.3, &[PaintSemantic::SupportEnforcer]);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.support_paths().is_empty(),
        "SupportEnforcer must override needs_support=false (D14 precedence)"
    );
}

/// Test 8: Blocker still wins even when `needs_support=true`.
#[test]
fn blocker_overrides_needs_support_true() {
    let config = enabled_config();
    let module = TraditionalSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(true);

    let paint = paint_view_with_annotations(0.3, &[PaintSemantic::SupportBlocker]);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "SupportBlocker must override needs_support=true (D14 precedence)"
    );
}

// ── L-shape regression for the polygon-intersection helper ──────────────────
//
// Regression test for the centroid-probe → polygon-intersection migration
// (packet 120 Step 3, wired into `PaintRegionLayerView::paint_policy_for`).
// The L-shape's vertex-mean centroid lies in the notch (outside the polygon
// AND outside the painted enforcer arm). The OLD centroid-based helper
// would have returned `DefaultEligible`; the NEW polygon-intersection
// helper must return `Enforced` because the enforcer annotation covers
// the L's vertical arm with non-trivial area.

/// L-shaped 10×10 mm ExPolygon with a 6×6 mm notch in the top-right corner.
///
/// Vertices CCW (mm):
///   (0,0) → (10,0) → (10,4) → (4,4) → (4,10) → (0,10) → close
///
/// Centroid of L-shape contour: (28/6, 28/6) ≈ (4.667, 4.667) mm; lies in
/// the notch (x >= 4, y >= 4), outside the L's polygon AND outside the
/// painted enforcer arm. The new polygon-intersection helper must return
/// `Enforced`; the OLD centroid-based helper would have returned
/// `DefaultEligible` because the centroid lies outside the painted area.
fn l_shape_expoly() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 4.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(4.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    }
}

/// Build a `SlicedRegion` whose polygon is the L-shape (so the
/// polygon-intersection helper sees the L-shape as the painted area).
fn l_shape_region_with_annotations(semantics: &[PaintSemantic]) -> SlicedRegion {
    region_with_annotations(vec![l_shape_expoly()], semantics)
}

fn l_shape_paint_view(z: f32, semantics: &[PaintSemantic]) -> PaintRegionLayerView {
    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z,
        regions: vec![l_shape_region_with_annotations(semantics)],
    };
    PaintRegionLayerView::new(0).with_slice_ir(Arc::new(slice))
}

#[test]
fn enforcer_works_when_centroid_outside_paint_region() {
    // Centroid of L-shape contour: (4.667, 4.667) mm; lies in the notch
    // (x >= 4, y >= 4), outside the L's polygon AND outside the painted
    // enforcer arm. The new polygon-intersection helper must return
    // Enforced; the OLD centroid-based helper would have returned
    // DefaultEligible because the centroid lies outside the painted area.
    let l_expoly = l_shape_expoly();
    let paint = l_shape_paint_view(0.3, &[PaintSemantic::SupportEnforcer]);

    // Direct call: the L-shape's centroid is in the notch (outside the
    // painted enforcer arm) so a centroid-based probe would return
    // DefaultEligible. The polygon-intersection helper must return Enforced.
    assert_eq!(
        paint.paint_policy_for(&l_expoly),
        SupportPaintPolicy::Enforced,
        "polygon-intersection helper must classify L-shape with enforcer \
         annotation on the vertical arm as Enforced, even though the \
         vertex-mean centroid lies in the notch (outside the polygon)"
    );
}
