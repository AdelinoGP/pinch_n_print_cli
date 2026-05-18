//! TDD red tests for TASK-096: tree-support enforcer/blocker paint semantics.
//!
//! Tests verify that tree-support reads PaintRegionIR for SupportEnforcer
//! and SupportBlocker semantics, applying blocker > enforcer precedence before
//! tree support generation.
//!
//! Authoritative docs:
//! - docs/01_system_architecture.md §"Layer::Support"
//! - docs/02_ir_schemas.md lines 412-418
//! - docs/10_glossary_and_scenario_traces.md §"Support paint conflict"
//!
//! OrcaSlicer ref:
//! - OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1100-1104 (enforcer/blocker slicing)
//! - OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1180-1186 (blocker diff)
//! - OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:1201-1206 (enforcer overhang)

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, LayerPaintMap, PaintRegionIR, PaintSemantic, PaintValue,
    Point2, Polygon, SemanticRegion,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use tree_support::TreeSupport;

/// Helper: create an enabled support config.
fn enabled_config() -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("support_enabled".to_string(), ConfigValue::Bool(true));
    fields.insert("support_density".to_string(), ConfigValue::Float(0.2));
    fields.insert("support_angle".to_string(), ConfigValue::Float(0.0));
    fields.insert("support_speed".to_string(), ConfigValue::Float(50.0));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.4));
    ConfigView::from_map(fields)
}

/// Helper: create a 10mm square ExPolygon centered at origin.
fn square_expoly() -> ExPolygon {
    let half = 5.0_f32;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-half, -half),
                Point2::from_mm(half, -half),
                Point2::from_mm(half, half),
                Point2::from_mm(-half, half),
            ],
        },
        holes: vec![],
    }
}

/// Helper: create a SliceRegionView with the standard square.
fn square_region(z: f32) -> SliceRegionView {
    let poly = square_expoly();
    {
        let mut tmp = SliceRegionView::default();
        tmp.set_object_id("obj1".to_string());
        tmp.set_region_id(1);
        tmp.set_polygons(vec![poly.clone()]);
        tmp.set_infill_areas(vec![poly]);
        tmp.set_effective_layer_height(0.2);
        tmp.set_z(z);
        tmp.set_has_nonplanar(false);
        tmp
    }
}

/// Helper: build a PaintRegionIR with a single semantic covering the entire
/// square region at layer 0.
fn paint_ir_with_semantic(semantic: PaintSemantic, value: PaintValue) -> Arc<PaintRegionIR> {
    let region = SemanticRegion {
        object_id: "obj1".to_string(),
        polygons: vec![square_expoly()],
        value,
        paint_order: 1,
    };

    let mut semantic_regions = HashMap::new();
    semantic_regions.insert(semantic, vec![region]);

    let layer_paint = LayerPaintMap {
        global_layer_index: 0,
        semantic_regions,
    };

    let mut per_layer = HashMap::new();
    per_layer.insert(0_u32, layer_paint);

    Arc::new(PaintRegionIR {
        per_layer,
        ..Default::default()
    })
}

/// Helper: build a PaintRegionIR with both SupportBlocker and SupportEnforcer
/// covering the same region at layer 0.
fn paint_ir_both_blocker_and_enforcer() -> Arc<PaintRegionIR> {
    let blocker_region = SemanticRegion {
        object_id: "obj1".to_string(),
        polygons: vec![square_expoly()],
        value: PaintValue::Flag(true),
        paint_order: 1,
    };
    let enforcer_region = SemanticRegion {
        object_id: "obj1".to_string(),
        polygons: vec![square_expoly()],
        value: PaintValue::Flag(true),
        paint_order: 1,
    };

    let mut semantic_regions = HashMap::new();
    semantic_regions.insert(PaintSemantic::SupportBlocker, vec![blocker_region]);
    semantic_regions.insert(PaintSemantic::SupportEnforcer, vec![enforcer_region]);

    let layer_paint = LayerPaintMap {
        global_layer_index: 0,
        semantic_regions,
    };

    let mut per_layer = HashMap::new();
    per_layer.insert(0_u32, layer_paint);

    Arc::new(PaintRegionIR {
        per_layer,
        ..Default::default()
    })
}

/// Test 1: A fully blocked region generates zero support paths.
///
/// When all support polygons fall within a SupportBlocker paint region,
/// no support paths should be generated regardless of overhang angle.
#[test]
fn fully_blocked_region_generates_zero_support() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
    let region = square_region(0.3);

    let paint_ir = paint_ir_with_semantic(PaintSemantic::SupportBlocker, PaintValue::Flag(true));
    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "fully blocked region must produce zero support paths, but got {}",
        output.support_paths().len()
    );
}

/// Test 2: A fully enforced region generates support paths at 0-degree overhang.
///
/// When all support polygons fall within a SupportEnforcer paint region,
/// support should be generated even with 0-degree overhang (flat surface).
#[test]
fn fully_enforced_region_generates_support_at_zero_overhang() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
    let region = square_region(0.3);

    let paint_ir = paint_ir_with_semantic(PaintSemantic::SupportEnforcer, PaintValue::Flag(true));
    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.support_paths().is_empty(),
        "enforced region must produce support paths at 0° overhang"
    );
}

/// Test 3: A region that is both blocked and enforced generates zero support
/// (blocker takes precedence over enforcer).
///
/// Per docs/02_ir_schemas.md line 412:
/// "For support logic conflicts at the same point: SupportBlocker takes
/// precedence over SupportEnforcer."
#[test]
fn blocked_plus_enforced_resolves_to_zero_support() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
    let region = square_region(0.3);

    let paint_ir = paint_ir_both_blocker_and_enforcer();
    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "blocker must win over enforcer: expected zero paths, got {}",
        output.support_paths().len()
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

#[test]
fn enforcer_overrides_needs_support_false() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(false);

    let paint_ir = paint_ir_with_semantic(PaintSemantic::SupportEnforcer, PaintValue::Flag(true));
    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.support_paths().is_empty(),
        "enforcer must override needs_support=false",
    );
}

#[test]
fn blocker_overrides_needs_support_true() {
    let config = enabled_config();
    let module = TreeSupport::on_print_start(&config).unwrap();
    let mut region = square_region(0.3);
    region.set_needs_support(true);

    let paint_ir = paint_ir_with_semantic(PaintSemantic::SupportBlocker, PaintValue::Flag(true));
    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);

    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "blocker must win regardless of needs_support",
    );
}
