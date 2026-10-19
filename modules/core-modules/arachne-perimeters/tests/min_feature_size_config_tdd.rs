//! Regression coverage for the Arachne `min_feature_size` percent plumbing.

use std::collections::HashMap;

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigValue, ConfigView};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn config_with_min_feature_size(percent: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("wall_loops".to_string(), ConfigValue::Int(2));
    fields.insert("inner_wall_line_width".to_string(), ConfigValue::Float(0.4));
    fields.insert("outer_wall_line_width".to_string(), ConfigValue::Float(0.4));
    fields.insert("detect_thin_wall".to_string(), ConfigValue::Bool(true));
    fields.insert("nozzle_diameter".to_string(), ConfigValue::Float(0.4));
    fields.insert(
        "min_feature_size".to_string(),
        ConfigValue::Percent(percent),
    );
    // Required-read baseline (packet 06 5c'): the classified reads in
    // `run_perimeters` / `arachne_params_from_config` are `require_*`.
    fields.insert("layer_height".to_string(), ConfigValue::Float(0.2));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.45));
    fields.insert("bridge_line_width".to_string(), ConfigValue::Float(0.0));
    fields.insert(
        "initial_layer_line_width".to_string(),
        ConfigValue::Float(0.0),
    );
    fields.insert("extra_perimeters".to_string(), ConfigValue::Int(0));
    fields.insert("precise_outer_wall".to_string(), ConfigValue::Bool(false));
    fields.insert(
        "wall_sequence".to_string(),
        ConfigValue::String("InnerOuter".to_string()),
    );
    fields.insert("support_raft_layers".to_string(), ConfigValue::Int(0));
    fields.insert("only_one_wall_top".to_string(), ConfigValue::Bool(false));
    fields.insert(
        "only_one_wall_first_layer".to_string(),
        ConfigValue::Bool(false),
    );
    fields.insert(
        "wall_direction".to_string(),
        ConfigValue::String("counter_clockwise".to_string()),
    );
    fields.insert("alternate_extra_wall".to_string(), ConfigValue::Bool(false));
    fields.insert("spiral_vase".to_string(), ConfigValue::Bool(false));
    fields.insert(
        "sparse_infill_density".to_string(),
        ConfigValue::Float(20.0),
    );
    fields.insert("detect_overhang_wall".to_string(), ConfigValue::Bool(true));
    fields.insert("overhang_reverse".to_string(), ConfigValue::Bool(false));
    fields.insert(
        "overhang_reverse_internal_only".to_string(),
        ConfigValue::Bool(false),
    );
    fields.insert(
        "overhang_reverse_threshold".to_string(),
        ConfigValue::Float(0.0),
    );
    fields.insert("bridge_flow".to_string(), ConfigValue::Float(1.0));
    fields.insert("thick_bridges".to_string(), ConfigValue::Bool(false));
    fields.insert(
        "seam_candidate_angle_threshold_deg".to_string(),
        ConfigValue::Float(30.0),
    );
    ConfigView::from_map(fields)
}

fn thin_strip_region() -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(rect_polygon(0.0, 0.0, 0.15, 5.0))
        .build()
}

fn emitted_walls(config: &ConfigView) -> usize {
    let module = ArachnePerimeters::from_config(config).expect("valid config");
    let regions = vec![thin_strip_region()];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, config)
        .expect("thin strip should be a valid perimeter input");
    output.wall_loops().len()
}

#[test]
fn percent_min_feature_size_reaches_widening_threshold() {
    let canonical_default = config_with_min_feature_size(25.0);
    let larger_threshold = config_with_min_feature_size(50.0);

    assert!(
        emitted_walls(&canonical_default) > 0,
        "a 0.15 mm strip is above the canonical 25% of a 0.4 mm nozzle threshold"
    );
    assert_eq!(
        emitted_walls(&larger_threshold),
        0,
        "a 0.15 mm strip must be rejected when 50% of a 0.4 mm nozzle resolves to 0.2 mm"
    );
}
