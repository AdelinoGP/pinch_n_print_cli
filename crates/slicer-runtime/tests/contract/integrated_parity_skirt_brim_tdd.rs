#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use skirt_brim::SkirtBrim;
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
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
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
        ..Default::default()
    }]
}

#[test]
fn integrated_parity_skirt_brim() {
    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 0);
    let stage: StageId = "PostPass::LayerFinalization".into();
    let config = Arc::new(ConfigView::from_map(
        [
            ("skirt_brim_enabled".into(), ConfigValue::Bool(true)),
            ("skirt_loops".into(), ConfigValue::Int(1)),
            ("skirt_height".into(), ConfigValue::Int(1)),
            ("brim_width".into(), ConfigValue::Float(0.0)),
            ("skirt_distance".into(), ConfigValue::Float(3.0)),
            ("line_width".into(), ConfigValue::Float(0.4)),
        ]
        .into_iter()
        .collect(),
    ));
    let (native_layers, wasm_layers) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.skirt-brim".into(),
            wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/skirt-brim/skirt-brim.wasm"),
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
            native_entry: SkirtBrim::__slicer_native_entry(),
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
        .expect("skirt brim native/wasm parity");
}
