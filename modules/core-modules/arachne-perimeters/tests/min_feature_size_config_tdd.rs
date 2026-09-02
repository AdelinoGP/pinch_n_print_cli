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
