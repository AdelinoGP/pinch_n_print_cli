#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::support_geometry::{build_emit_schedule, execute_support_geometry};
use slicer_ir::{ActiveRegion, GlobalLayer, LayerPlanIR, ResolvedConfig, SliceIR};

fn make_active_region(object_id: &str, layer_height: f32, support_lh: f32) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        resolved_config: ResolvedConfig {
            support_layer_height_mm: support_lh,
            ..ResolvedConfig::default()
        },
        effective_layer_height: layer_height,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

fn make_2_layer_plan() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.0,
                active_regions: vec![make_active_region("test-object", 0.2, 0.0)],
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.2,
                active_regions: vec![make_active_region("test-object", 0.2, 0.0)],
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ],
        object_participation: HashMap::new(),
        ..Default::default()
    }
}

fn make_two_object_plan() -> LayerPlanIR {
    let mut global_layers = Vec::new();
    for i in 0u32..6 {
        global_layers.push(GlobalLayer {
            index: i,
            z: (i + 1) as f32 * 0.2,
            active_regions: vec![
                make_active_region("obj-A", 0.2, 0.4),
                make_active_region("obj-B", 0.2, 0.0),
            ],
            has_nonplanar: false,
            is_sync_layer: false,
        });
    }
    LayerPlanIR {
        global_layers,
        object_participation: HashMap::new(),
        ..Default::default()
    }
}

#[test]
fn emits_for_2_layer_fixture() {
    let layer_plan = make_2_layer_plan();
    let slice_vec: Vec<SliceIR> = Vec::new();

    let result = execute_support_geometry(&layer_plan, &slice_vec);
    assert!(result.is_ok());

    let ir = result.unwrap();
    assert!(!ir.entries.is_empty());
}

#[test]
fn build_emit_schedule_two_objects_per_object_semantics() {
    use std::collections::BTreeSet;

    let plan = make_two_object_plan();
    let schedule = build_emit_schedule(&plan);

    let a_sched = schedule.get("obj-A").cloned().unwrap_or_default();
    let b_sched = schedule.get("obj-B").cloned().unwrap_or_default();

    assert_eq!(
        a_sched,
        [1u32, 3, 5].iter().cloned().collect::<BTreeSet<u32>>(),
        "obj-A (support_layer_height_mm=0.4, model 0.2mm) must emit at layers {{1,3,5}}"
    );

    assert_eq!(
        b_sched,
        (0u32..6).collect::<BTreeSet<u32>>(),
        "obj-B (support_layer_height_mm=0.0) must emit at every layer {{0..5}}"
    );
}

#[test]
fn empty_plan_produces_empty_support() {
    let layer_plan = LayerPlanIR::default();
    let slice_vec: Vec<SliceIR> = Vec::new();

    let result = execute_support_geometry(&layer_plan, &slice_vec);
    assert!(result.is_ok());
    assert!(result.unwrap().entries.is_empty());
}
