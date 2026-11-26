#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::support_geometry::execute_support_geometry;
use slicer_ir::{ActiveRegion, GlobalLayer, LayerPlanIR, ResolvedConfig, SliceIR};

fn make_active_region() -> ActiveRegion {
    ActiveRegion {
        object_id: "test-object".to_string(),
        region_id: 0,
        resolved_config: ResolvedConfig {
            support_layer_height_mm: 0.4,
            support_top_z_distance: 0.4,
            ..ResolvedConfig::default()
        },
        effective_layer_height: 0.2,
        ..Default::default()
    }
}

fn make_plan() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.0,
            active_regions: vec![make_active_region()],
            ..Default::default()
        }],
        object_participation: HashMap::new(),
        ..Default::default()
    }
}

#[test]
fn support_geometry_ir_carries_resolved_distances() {
    let ir = execute_support_geometry(&make_plan(), &Vec::<SliceIR>::new())
        .expect("support geometry should execute");

    assert_eq!(ir.support_top_z_distance, 0.4);
    assert_eq!(ir.support_layer_height_mm, 0.4);
}
