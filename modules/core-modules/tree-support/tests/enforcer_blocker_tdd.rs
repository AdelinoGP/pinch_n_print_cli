//! TDD tests for TASK-096: tree-support enforcer/blocker paint semantics.
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
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use tree_support::TreeSupport;

/// Helper: create an enabled support config.
fn enabled_config() -> ConfigView {
    ConfigViewBuilder::new()
        .bool("support_enabled", true)
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

/// Build a wider ExPolygon (20 mm square) that strictly contains the standard
/// 10 mm test square — used as the `SlicedRegion.polygons` covering the test
/// region so that the centroid containment check passes.
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

/// Compose a `SlicedRegion` covering the test square with the requested
/// segment_annotations populated.  Each annotation entry uses one outer Vec
/// ("one perimeter") and one inner Vec containing one `Some(Flag(true))` so
/// `paint_policy_for` will see a non-empty annotation.
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

/// Build a `PaintRegionLayerView` carrying a one-layer SliceIR with the
/// requested annotation semantics applied to a region whose polygon covers
/// the standard 10 mm test square.
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
///
/// D14 contract: SliceIR carries a SupportBlocker annotation covering the
/// region.  `paint_policy_for` returns `Blocked`; the support module skips.
#[test]
fn fully_blocked_region_generates_zero_support() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
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

/// Test 2: A fully enforced region generates support paths even when the
/// region's `needs_support` flag is false (paint overrides default eligibility).
#[test]
fn fully_enforced_region_generates_support_at_zero_overhang() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
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
/// (blocker takes precedence over enforcer, per docs/10 §"Scenario Trace 2").
#[test]
fn blocked_plus_enforced_resolves_to_zero_support() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
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
    let module = TreeSupport::on_print_start(&config).unwrap();
    let region = square_region(0.3);

    // No paint data at all
    let paint = PaintRegionLayerView::new(0);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    // Existing behaviour: support is generated for all provided ExPolygons.
    assert!(
        !output.support_paths().is_empty(),
        "unpainted region should still generate support (existing behaviour)"
    );
}

// ── SurfaceClassificationIR-driven default eligibility ────────────────────
// docs/02_ir_schemas.md and docs/01_system_architecture.md §"Layer::Support".

#[test]
fn default_ineligible_region_generates_zero_support() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
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

#[test]
fn default_eligible_region_generates_support() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
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
    let module = TreeSupport::on_print_start(&config).unwrap();
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

/// Test 8: Blocker overrides `needs_support=true`.
#[test]
fn blocker_overrides_needs_support_true() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
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
