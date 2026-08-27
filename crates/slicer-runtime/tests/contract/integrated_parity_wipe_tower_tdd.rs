#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth,
    PrintEntity, RegionKey, SemVer, StageId, ToolChange,
};
use slicer_runtime::{Blackboard, FinalizationStageRunner};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use wipe_tower::WipeTower;

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

fn layers() -> Vec<LayerCollectionIR> {
    let z = 0.2;
    vec![LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z,
        // exhaustive: parity comparison pins every field explicitly
        ordered_entities: vec![PrintEntity {
            entity_id: 1,
            path: ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 10.0,
                        y: 10.0,
                        z,
                        width: 0.4,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 20.0,
                        y: 10.0,
                        z,
                        width: 0.4,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 10.0,
                        z,
                        width: 0.4,
                        ..Default::default()
                    },
                ],
                ..extrusion_path3d_base(ExtrusionRole::OuterWall)
            },
            role: ExtrusionRole::OuterWall,
            tool_index: 0,
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "parity-object".into(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            topo_order: 0,
        }],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        ..Default::default()
    }]
}

#[test]
fn integrated_parity_wipe_tower() {
    let config = Arc::new(ConfigView::from_map(
        [
            ("wipe_tower_enabled".into(), ConfigValue::Bool(true)),
            ("wipe_tower_x".into(), ConfigValue::Float(10.0)),
            ("wipe_tower_y".into(), ConfigValue::Float(10.0)),
            ("wipe_tower_width".into(), ConfigValue::Float(60.0)),
            ("wipe_tower_purge_volume".into(), ConfigValue::Float(10.0)),
            ("line_width".into(), ConfigValue::Float(0.4)),
            (
                "bed_shape".into(),
                ConfigValue::List(vec![
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(200.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(200.0),
                    ConfigValue::Float(200.0),
                    ConfigValue::Float(0.0),
                    ConfigValue::Float(200.0),
                ]),
            ),
        ]
        .into_iter()
        .collect(),
    ));
    let stage: StageId = "PostPass::LayerFinalization".into();
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 0);
    let (native_layers, wasm_layers) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.wipe-tower".into(),
            wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/wipe-tower/wipe-tower.wasm"),
            stage: "PostPass::LayerFinalization".into(),
            version: semver(),
            min_ir_schema: semver(),
            max_ir_schema: SemVer {
                major: 5,
                minor: 0,
                patch: 0,
            },
            tier: slicer_schema::TIER_FINALIZATION.into(),
            claims: Vec::new(),
            config,
            native_entry: WipeTower::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let mut native_layers = layers();
            let mut wasm_layers = native_layers.clone();
            FinalizationStageRunner::run_stage(
                dispatcher,
                &stage,
                native_live,
                finalization_input(&blackboard),
                &mut native_layers,
            )
            .expect("native finalization dispatch");
            FinalizationStageRunner::run_stage(
                dispatcher,
                &stage,
                wasm_live,
                finalization_input(&blackboard),
                &mut wasm_layers,
            )
            .expect("wasm finalization dispatch");
            (native_layers, wasm_layers)
        },
    );
    assert!(!native_layers.is_empty());
    assert!(!wasm_layers.is_empty());
    assert_finalization_parity_structural(&native_layers, &wasm_layers, ParityTolerance::default())
        .expect("wipe tower native/wasm parity");
}
