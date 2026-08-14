#![allow(missing_docs)]

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_parity_structural, ParityTolerance},
    wasm_cache,
};
use fuzzy_skin::FuzzySkinModule;
use slicer_ir::{
    ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LoopType, MeshIR, PerimeterIR,
    PerimeterRegion, Point3WithWidth, SemVer, StageId, WallBoundaryType, WallFeatureFlags,
    WallLoop, WidthProfile,
};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    WasmInstancePool, WasmRuntimeDispatcher,
};
use std::sync::Arc;
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;

fn perimeter() -> PerimeterIR {
    let points = vec![
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ]
    .into_iter()
    .map(|(x, y)| Point3WithWidth {
        x,
        y,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        ..Default::default()
    })
    .collect::<Vec<_>>();
    let flags = (0..points.len())
        .map(|_| WallFeatureFlags {
            fuzzy_skin: true,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    PerimeterIR {
        schema_version: SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: "parity-object".into(),
            region_id: 0,
            // exhaustive: parity comparison pins every field explicitly
            walls: vec![WallLoop {
                perimeter_index: 0,
                loop_type: LoopType::Outer,
                path: ExtrusionPath3D {
                    points: points.clone(),
                    ..extrusion_path3d_base(ExtrusionRole::OuterWall)
                },
                width_profile: WidthProfile {
                    widths: points.iter().map(|p| p.width).collect(),
                },
                feature_flags: flags,
                boundary_type: WallBoundaryType::ExteriorSurface,
            }],
            ..Default::default()
        }],
    }
}

#[test]
fn integrated_parity_fuzzy_skin() {
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        ("thickness".into(), slicer_ir::ConfigValue::Float(0.3)),
        ("point_distance".into(), slicer_ir::ConfigValue::Float(0.5)),
        ("apply_to_all".into(), slicer_ir::ConfigValue::Bool(true)),
    ])));
    let bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_perimeter(perimeter())
        .expect("set wasm perimeter");
    native_arena
        .set_perimeter(perimeter())
        .expect("set native perimeter");
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::PerimetersPostProcess".into();
    let (native, wasm) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: "com.core.fuzzy-skin".into(),
            wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/fuzzy-skin/fuzzy-skin.wasm"),
            stage: stage.clone(),
            version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 5,
                minor: 0,
                patch: 0,
            },
            tier: String::new(),
            claims: Vec::new(),
            config: Arc::clone(&config),
            native_entry: FuzzySkinModule::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let wasm = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                wasm_live,
                crate::common::layer_input(&bb, &wasm_arena),
            )
            .expect("wasm dispatch")
            .expect("wasm commit");
            let native = LayerStageRunner::run_stage(
                dispatcher,
                &stage,
                &layer,
                native_live,
                crate::common::layer_input(&bb, &native_arena),
            )
            .expect("native dispatch")
            .expect("native commit");
            (native, wasm)
        },
    );
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("fuzzy skin native/wasm parity");
}

/// Regression: the native `Layer::PerimetersPostProcess` leg must tolerate a
/// layer with no committed `PerimeterIR` (arena.perimeter() == None) exactly
/// like the wasm leg does.
///
/// Pre-fix, `build_native_layer_request` set `perimeter_regions = None` when
/// `input.perimeter` was None, and the macro-emitted `run_wall_postprocess`
/// native entry aborted with a fatal "missing perimeter regions" error. The
/// wasm leg (`push_perimeter_regions`) instead pushes an empty region list and
/// the module succeeds with no output. The two legs diverged; this test pins
/// the native leg to the wasm leg's behaviour.
#[test]
fn native_fuzzy_skin_without_committed_perimeter_does_not_fatal() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([
        ("thickness".into(), slicer_ir::ConfigValue::Float(0.3)),
        ("point_distance".into(), slicer_ir::ConfigValue::Float(0.5)),
        ("apply_to_all".into(), slicer_ir::ConfigValue::Bool(true)),
    ])));
    let native_module = CompiledModuleBuilder::new("com.core.fuzzy-skin")
        .config_view(config)
        .build();
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(FuzzySkinModule::__slicer_native_entry());
    let bb = Blackboard::new(Arc::new(MeshIR::default()), 1);
    // Deliberately leave the arena perimeter unset: input.perimeter == None.
    let native_arena = LayerArena::new();
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        ..Default::default()
    };
    let stage: StageId = "Layer::PerimetersPostProcess".into();
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &native_live,
        crate::common::layer_input(&bb, &native_arena),
    )
    .expect("native dispatch must not fatal on a missing committed perimeter");
    match commit {
        Some(slicer_ir::LayerStageCommit::PerimetersPostProcess(None)) => {}
        other => {
            panic!("with no committed perimeter the module must emit no output; got {other:?}")
        }
    }
}
