#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use overhang_classifier_default::OverhangClassifierDefault;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, StageId,
};
use slicer_runtime::{Blackboard, FinalizationStageRunner};

use crate::common::{
    finalization_input,
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_finalization_parity_structural, ParityTolerance},
};

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn entity(layer: u32, annotated: bool) -> PrintEntity {
    // exhaustive: parity comparison pins every field explicitly
    PrintEntity {
        entity_id: layer as u64 + 1,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: layer as f32 * 0.2 + 0.2,
                    width: 0.4,
                    overhang_quartile: annotated.then_some(1),
                    overhang_distance_mm: annotated.then_some(1.0),
                    ..Default::default()
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z: layer as f32 * 0.2 + 0.2,
                    width: 0.4,
                    overhang_quartile: annotated.then_some(1),
                    overhang_distance_mm: annotated.then_some(1.0),
                    ..Default::default()
                },
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: layer as f32 * 0.2 + 0.2,
                    width: 0.4,
                    overhang_quartile: annotated.then_some(1),
                    overhang_distance_mm: annotated.then_some(1.0),
                    ..Default::default()
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: 0,
        region_key: RegionKey {
            global_layer_index: layer,
            object_id: "parity-object".into(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        topo_order: 0,
    }
}

fn layers() -> Vec<LayerCollectionIR> {
    vec![
        LayerCollectionIR {
            schema_version: semver(),
            global_layer_index: 0,
            z: 0.2,
            ordered_entities: vec![entity(0, false)],
            ..Default::default()
        },
        LayerCollectionIR {
            schema_version: semver(),
            global_layer_index: 1,
            z: 0.4,
            ordered_entities: vec![entity(1, true)],
            ..Default::default()
        },
    ]
}

#[test]
fn integrated_parity_overhang_classifier() {
    let config = Arc::new(ConfigView::from_map(
        [
            ("enable_overhang_speed".into(), ConfigValue::Bool(true)),
            ("line_width".into(), ConfigValue::Float(0.4)),
            ("outer_wall_speed".into(), ConfigValue::Float(60.0)),
            ("overhang_1_4_speed".into(), ConfigValue::Float(20.0)),
        ]
        .into_iter()
        .collect(),
    ));
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 0);
    let stage: StageId = "PostPass::LayerFinalization".into();
    let mut native_layers = layers();
    let mut wasm_layers = native_layers.clone();
    run_integrated_parity(IntegratedParitySpec { module_id: "com.core.overhang-classifier-default".into(), wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules/overhang-classifier-default/overhang-classifier-default.wasm"), stage: stage.clone(), version: semver(), min_ir_schema: semver(), max_ir_schema: SemVer { major: 5, minor: 0, patch: 0 }, tier: slicer_schema::TIER_FINALIZATION.into(), claims: Vec::new(), config, native_entry: OverhangClassifierDefault::__slicer_native_entry() }, |dispatcher, native_live, wasm_live| {
        FinalizationStageRunner::run_stage(dispatcher, &stage, native_live, finalization_input(&blackboard), &mut native_layers).expect("native finalization dispatch");
        FinalizationStageRunner::run_stage(dispatcher, &stage, wasm_live, finalization_input(&blackboard), &mut wasm_layers).expect("wasm finalization dispatch");
    });
    assert!(!native_layers.is_empty());
    assert!(!wasm_layers.is_empty());
    assert_finalization_parity_structural(&native_layers, &wasm_layers, ParityTolerance::default())
        .expect("overhang classifier native/wasm parity");
}
