#![allow(missing_docs)]

use std::collections::BTreeMap;

use slicer_core::algos::region_mapping::{
    execute_region_mapping_with_cap, RegionMappingError, RegionMappingPlanProjection,
    DEFAULT_REGION_MAP_CAP,
};
use slicer_ir::{
    ActiveRegion, GlobalLayer, LayerPlanIR, ObjectMesh, PaintSemantic, RegionKey, RegionMapIR,
    ResolvedConfig, SemVer,
};

// ---- helpers ----------------------------------------------------------------

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

fn make_layer_plan() -> LayerPlanIR {
    // 2 layers, 2 objects ("obj_a", "obj_b"), 2 regions each → 4 active_regions per layer
    let mut plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: Vec::new(),
        object_participation: Default::default(),
    };

    for layer_idx in 0u32..2 {
        let mut active_regions = Vec::new();
        for obj_id in &["obj_a", "obj_b"] {
            for region_id in 0u64..2 {
                active_regions.push(ActiveRegion {
                    object_id: obj_id.to_string(),
                    region_id,
                    resolved_config: ResolvedConfig::default(),
                    effective_layer_height: 0.2,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                    ..Default::default()
                });
            }
        }
        plan.global_layers.push(GlobalLayer {
            index: layer_idx,
            z: layer_idx as f32 * 0.2,
            active_regions,
            has_nonplanar: false,
            ..Default::default()
        });
    }
    plan
}


fn no_objects() -> Vec<ObjectMesh> {
    Vec::new()
}

fn no_paint_configs() -> BTreeMap<PaintSemantic, ResolvedConfig> {
    BTreeMap::new()
}

// ---- tests ------------------------------------------------------------------

/// AC-8: basic shape — 2 layers × 2 objects × 2 regions = 8 entries
#[test]
fn region_map_has_expected_entry_count() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let objects = no_objects();

    let result = execute_region_mapping_with_cap(
        &plan,
        &projection,
        None,
        &configs,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    );

    let region_map: RegionMapIR = result.expect("region mapping must succeed");
    // 2 layers × 2 objects × 2 regions = 8 entries
    assert_eq!(
        region_map.entries.len(),
        8,
        "expected 8 entries, got {}",
        region_map.entries.len()
    );
}

/// AC-8: each (layer, object, region) key is present and uniquely addressable
#[test]
fn region_map_keys_are_correct() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let objects = no_objects();

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        None,
        &configs,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .unwrap();

    // Spot-check a few expected keys
    let expected_keys = [
        RegionKey { global_layer_index: 0, object_id: "obj_a".to_string(), region_id: 0 },
        RegionKey { global_layer_index: 0, object_id: "obj_a".to_string(), region_id: 1 },
        RegionKey { global_layer_index: 1, object_id: "obj_b".to_string(), region_id: 0 },
        RegionKey { global_layer_index: 1, object_id: "obj_b".to_string(), region_id: 1 },
    ];
    for key in &expected_keys {
        assert!(
            region_map.entries.contains_key(key),
            "missing key: {:?}",
            key
        );
    }
}

/// AC-8: cap exceeded produces the correct error variant
#[test]
fn region_map_cap_exceeded_returns_error() {
    let plan = make_layer_plan(); // 8 entries
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let objects = no_objects();

    // Cap of 3 is below the 8 entries we have
    let result =
        execute_region_mapping_with_cap(&plan, &projection, None, &configs, &objects, 3);

    match result {
        Err(RegionMappingError::CapExceeded { entry_count, cap, .. }) => {
            assert_eq!(entry_count, 8);
            assert_eq!(cap, 3);
        }
        other => panic!("expected CapExceeded, got {:?}", other),
    }
}

/// AC-8: empty layer plan produces empty entries map
#[test]
fn empty_layer_plan_produces_empty_map() {
    let plan = LayerPlanIR::default();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let objects = no_objects();

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        None,
        &configs,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .unwrap();

    assert!(region_map.entries.is_empty(), "expected no entries for empty plan");
}
