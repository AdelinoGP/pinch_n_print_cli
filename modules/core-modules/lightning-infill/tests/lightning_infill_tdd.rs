//! TDD tests for the lightning-infill module.
//!
// Step 3 (packet 140) - classification:
// KEEP:   on_print_start_defaults - default config values are stable.
// KEEP:   on_print_start_custom - configured module values are read.
// KEEP:   paths_have_sparse_infill_role - emitted paths retain SparseInfill role tagging.
// KEEP:   empty_regions_no_output - an empty region emits no paths.
// KEEP:   paths_at_correct_z - emitted points retain the region's layer height.
// KEEP:   width_matches_config - emitted points retain the configured line width.
// ADAPT:  square_region_produces_paths - assert committed tree-segment emission.
// DELETE: zero_density_no_paths - encoded the deleted stub's density gate.
// DELETE: branching_pattern_present - encoded deleted branch construction.
// DELETE: density_affects_coverage - encoded deleted grid sampling.
// DELETE: interior_first_growth - encoded deleted interior sampling.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigView, ExtrusionRole, LightningTreeEntry, LightningTreeIR, Point2};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use lightning_infill::LightningInfill;

fn empty_paint_view() -> PaintRegionLayerView {
    PaintRegionLayerView::new(0)
}

fn paint_view_with_segments(segments: Vec<[Point2; 2]>) -> PaintRegionLayerView {
    let ir = LightningTreeIR {
        entries: vec![LightningTreeEntry {
            object_id: "obj1".to_string(),
            global_layer_index: 0,
            region_id: 1,
            tree_edge_segments: segments,
        }],
        ..LightningTreeIR::default()
    };
    PaintRegionLayerView::new(0).with_lightning_tree_ir(Arc::new(ir))
}

fn make_config(density: f64, speed: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_speed", speed)
        .float("line_width", line_width)
        .build()
}

fn make_square_region(size_mm: f32, z: f32) -> SliceRegionView {
    // Post-host-partition fixture: populate `sparse_infill_area` so lightning's
    // sparse-fill emission has its canonical polygon (see
    // `crates/slicer-runtime/src/region_partition.rs`).
    let sq = square_polygon(0.0, 0.0, size_mm);
    let mut region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(z)
        .add_polygon(sq.clone())
        .sparse_infill_area(vec![sq])
        .build();
    // Lightning manifest declares only `claim:sparse-fill`; set held_claims
    // so should_emit gates correctly (empty held_claims = emit nothing).
    region.set_held_claims(vec!["claim:sparse-fill".into()]);
    region
}

fn sample_segments() -> Vec<[Point2; 2]> {
    vec![[Point2::from_mm(1.0, 2.0), Point2::from_mm(3.0, 4.0)]]
}

/// Test 1: Default config values when no fields provided.
#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = LightningInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.2).abs() < 0.001);
    assert!((module.line_width() - 0.4).abs() < 0.001);
}

/// Test 2: Custom config values are read correctly.
#[test]
fn on_print_start_custom() {
    let config = make_config(0.3, 80.0, 0.5);
    let module = LightningInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.3).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

/// Test 3: committed tree segments become sparse paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(
            0,
            &[region],
            &paint_view_with_segments(sample_segments()),
            &mut output,
            &config,
        )
        .unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
}

/// Test 4: All paths have SparseInfill extrusion role.
#[test]
fn paths_have_sparse_infill_role() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(
            0,
            &[region],
            &paint_view_with_segments(sample_segments()),
            &mut output,
            &config,
        )
        .unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
    assert_eq!(output.sparse_paths()[0].role, ExtrusionRole::SparseInfill);
}

/// Test 5: Empty regions produce no output.
#[test]
fn empty_regions_no_output() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj1".to_string());
    region.set_region_id(1);
    region.set_polygons(vec![]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.3);
    region.set_has_nonplanar(false);

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    assert_eq!(output.sparse_paths().len(), 0);
}

/// Test 6: All output points have the correct z value.
#[test]
fn paths_at_correct_z() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();
    let z = 1.5;
    let region = make_square_region(10.0, z);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(
            0,
            &[region],
            &paint_view_with_segments(sample_segments()),
            &mut output,
            &config,
        )
        .unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
    for point in &output.sparse_paths()[0].points {
        assert!((point.z - z).abs() < 0.001);
    }
}

/// Test 7: All point widths match configured line_width.
#[test]
fn width_matches_config() {
    let line_width = 0.6;
    let config = make_config(0.2, 50.0, line_width);
    let module = LightningInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(
            0,
            &[region],
            &paint_view_with_segments(sample_segments()),
            &mut output,
            &config,
        )
        .unwrap();

    assert_eq!(output.sparse_paths().len(), 1);
    for point in &output.sparse_paths()[0].points {
        assert!((point.width - line_width as f32).abs() < 0.001);
    }
}

/// AC-1: emit each committed lightning tree segment as one raw SparseInfill path.
#[test]
fn samples_tree_ir_raw_emit() {
    let config = make_config(0.2, 80.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let expected = vec![
        [Point2::from_mm(1.0, 2.0), Point2::from_mm(3.0, 4.0)],
        [Point2::from_mm(-2.0, 1.5), Point2::from_mm(0.0, 5.0)],
    ];
    let paint = paint_view_with_segments(expected.clone());
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    assert_eq!(paths.len(), expected.len());
    for (path, segment) in paths.iter().zip(expected.iter()) {
        assert_eq!(path.points.len(), 2);
        let start = &path.points[0];
        let end = &path.points[1];
        assert!((start.x - slicer_ir::units_to_mm(segment[0].x)).abs() < 0.001);
        assert!((start.y - slicer_ir::units_to_mm(segment[0].y)).abs() < 0.001);
        assert!((end.x - slicer_ir::units_to_mm(segment[1].x)).abs() < 0.001);
        assert!((end.y - slicer_ir::units_to_mm(segment[1].y)).abs() < 0.001);
        assert_eq!(path.role, ExtrusionRole::SparseInfill);
        assert!((path.speed_factor - 1.6).abs() < 0.001);
    }
}

/// AC-N2: an empty tree entry is a successful no-op.
#[test]
fn empty_trees_emit_nothing() {
    let config = make_config(0.2, 80.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let paint = paint_view_with_segments(Vec::new());
    let mut output = InfillOutputBuilder::new();

    let result = module.run_infill(0, &[region], &paint, &mut output, &config);

    assert!(result.is_ok());
    assert_eq!(output.sparse_paths().len(), 0);
}
