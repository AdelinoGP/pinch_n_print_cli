#![allow(missing_docs)]

use std::sync::Arc;

use seam_placer::SeamPlacer;
use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LoopType, MeshIR,
    PerimeterIR, PerimeterRegion, Point3WithWidth, RegionKey, SeamPlanEntry, SeamPlanIR,
    SeamPosition, SemVer, StageId, WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageRunner,
    LoadedModuleBuilder, WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{
    parity_invariants::{assert_parity_structural, ParityTolerance},
    wasm_cache,
};

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
                    role: ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                width_profile: WidthProfile {
                    widths: points.iter().map(|point| point.width).collect(),
                },
                feature_flags: vec![Default::default(); points.len()],
                boundary_type: WallBoundaryType::ExteriorSurface,
            }],
            ..Default::default()
        }],
    }
}

fn wasm_live<'a>(module: &'a slicer_runtime::CompiledModule) -> CompiledModuleLive<'a> {
    let loaded = LoadedModuleBuilder::new(
        module.module_id().as_str(),
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::PerimetersPostProcess",
        String::new(),
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    let component = wasm_cache::compiled_component_at(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/core-modules/seam-placer/seam-placer.wasm"),
    );
    CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(component),
        module.claims(),
        Arc::clone(module.config_view()),
    )
}

#[test]
fn integrated_parity_seam_placer() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([(
        "seam_mode".into(),
        ConfigValue::String("nearest".into()),
    )])));
    let wasm_module = CompiledModuleBuilder::new("com.core.seam-placer")
        .claims(vec!["seam-placer".into()])
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new("com.core.seam-placer")
        .claims(vec!["seam-placer".into()])
        .config_view(config)
        .build();
    let wasm_live = wasm_live(&wasm_module);
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SeamPlacer::__slicer_native_entry());

    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_seam_plan(Arc::new(SeamPlanIR {
            entries: vec![SeamPlanEntry {
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "parity-object".into(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                chosen_candidate: SeamPosition {
                    point: Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        ..Default::default()
                    },
                    wall_index: 0,
                },
                ..Default::default()
            }],
            ..Default::default()
        }))
        .expect("commit seam plan");

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
    let wasm = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &wasm_live,
        crate::common::layer_input(&blackboard, &wasm_arena),
    )
    .expect("wasm dispatch")
    .expect("wasm commit");
    let native = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &native_live,
        crate::common::layer_input(&blackboard, &native_arena),
    )
    .expect("native dispatch")
    .expect("native commit");
    assert_parity_structural(&native, &wasm, ParityTolerance::default(), 0.4)
        .expect("seam placer native/wasm parity");
}

/// Regression: the native `Layer::PerimetersPostProcess` leg must commit a
/// module-emitted `resolved_seam` with its source region origin, matching the
/// wasm leg.
///
/// Pre-fix, `collect_perimeter` (marshal/native.rs) hardcoded
/// `resolved_seam_origin: None` for every native perimeter commit. A
/// post-process module that calls `set_resolved_seam` (seam-placer in
/// `aligned` mode) therefore hit the host commit guard "resolved_seam was
/// emitted without an active perimeter source region" and aborted the layer —
/// a native/wasm leg divergence (wasm captures the origin at
/// `push_resolved_seam`).
#[test]
fn native_seam_placer_aligned_commits_resolved_seam_with_origin() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([(
        "seam_mode".into(),
        ConfigValue::String("aligned".into()),
    )])));
    let native_module = CompiledModuleBuilder::new("com.core.seam-placer")
        .claims(vec!["seam-placer".into()])
        .config_view(config)
        .build();
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SeamPlacer::__slicer_native_entry());

    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_seam_plan(Arc::new(SeamPlanIR {
            entries: vec![SeamPlanEntry {
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "parity-object".into(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                chosen_candidate: SeamPosition {
                    point: Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        ..Default::default()
                    },
                    wall_index: 0,
                },
                ..Default::default()
            }],
            ..Default::default()
        }))
        .expect("commit seam plan");

    // The native leg reads `resolved_seam` from the committed PerimeterIR
    // region directly (it does not resolve from the seam plan like the wasm
    // leg does), so seed it here.
    let mut perim = perimeter();
    perim.regions[0].object_id = "source-object".into();
    perim.regions[0].region_id = 7;
    perim.regions[0].resolved_seam = Some(SeamPosition {
        point: Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            ..Default::default()
        },
        wall_index: 0,
    });
    let mut native_arena = LayerArena::new();
    native_arena
        .set_perimeter(perim)
        .expect("set native perimeter");

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
        crate::common::layer_input(&blackboard, &native_arena),
    )
    .expect("native dispatch must not fatal on an aligned resolved_seam");
    let perim = match commit {
        Some(slicer_ir::LayerStageCommit::PerimetersPostProcess(Some(ir))) => ir,
        other => panic!("seam-placer must commit a perimeter; got {other:?}"),
    };
    assert!(
        perim.regions[0].resolved_seam.is_some(),
        "seam-placer must emit the resolved_seam into the committed perimeter"
    );
    assert_eq!(perim.regions[0].object_id, "source-object");
    assert_eq!(perim.regions[0].region_id, 7);
}
