//! End-to-end coverage for the live Arachne sandwich order at path optimization.

#[path = "common/mod.rs"]
mod common;

#[path = "unit/path_ordering_tdd.rs"]
mod path_ordering;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_ir::{
    ActiveRegion, ConfigValue, ConfigView, ExPolygon, LayerStageCommit, LayerStageError, Point2,
    Polygon, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::{
    execute_per_layer, Blackboard, CompiledModule, CompiledModuleBuilder, CompiledModuleLive,
    CompiledStage, ExecutionPlan, LayerStageInput, WasmEngine,
};
use slicer_wasm_host::{
    build_wasm_instance_pool, LayerStageRunner, WasmArtifactMetadata, WasmComponent,
    WasmInstancePool, WasmRuntimeDispatcher,
};

use crate::common::{semver, wasm_cache};

const ARACHNE_ID: &str = "com.core.arachne-perimeters";
const PATH_OPT_ID: &str = "com.core.path-optimization-default";

/// The optimizer must consume the wall order committed by the live Arachne
/// module, rather than applying its own role-priority order.
#[test]
fn live_arachne_layer_one_sandwich_reaches_path_optimizer_unchanged() {
    let _ordered_entities_guard = crate::common::ordered_entities_counter_lock();
    for (wall_sequence, expected_identities, expected_roles) in [
        (
            "InnerOuter",
            vec![0, 1, 2],
            vec![
                slicer_ir::ExtrusionRole::OuterWall,
                slicer_ir::ExtrusionRole::InnerWall,
                slicer_ir::ExtrusionRole::InnerWall,
            ],
        ),
        (
            "OuterInner",
            vec![2, 1, 0],
            vec![
                slicer_ir::ExtrusionRole::InnerWall,
                slicer_ir::ExtrusionRole::InnerWall,
                slicer_ir::ExtrusionRole::OuterWall,
            ],
        ),
        (
            "InnerOuterInner",
            vec![1, 0, 2],
            vec![
                slicer_ir::ExtrusionRole::InnerWall,
                slicer_ir::ExtrusionRole::OuterWall,
                slicer_ir::ExtrusionRole::InnerWall,
            ],
        ),
    ] {
        let engine = wasm_cache::shared_engine();
        let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
        let arachne = live_module(
            &engine,
            ARACHNE_ID,
            "Layer::Perimeters",
            "arachne-perimeters/arachne-perimeters.wasm",
            [
                ("wall_count", ConfigValue::Int(3)),
                (
                    "wall_sequence",
                    ConfigValue::String(wall_sequence.to_string()),
                ),
            ],
        );
        let path_opt = live_module(
            &engine,
            PATH_OPT_ID,
            "Layer::PathOptimization",
            "path-optimization-default/path-optimization-default.wasm",
            [],
        );

        let committed_layer_one = Arc::new(Mutex::new(None));
        let runner = LiveArachneThenPathOpt {
            dispatcher: Arc::new(dispatcher),
            arachne,
            path_opt,
            committed_layer_one: Arc::clone(&committed_layer_one),
        };

        let plan = plan();
        let mesh = Arc::new(slicer_ir::MeshIR::default());
        let mut blackboard = Blackboard::new(mesh, 2);
        blackboard
            .commit_slice_ir(Arc::new(vec![slice(0), slice(1)]))
            .expect("slice fixture must be accepted");

        let layers = execute_per_layer(&plan, &blackboard, &runner, &Default::default())
            .expect("live Arachne and path optimization must execute");
        let layer_one = &layers[1].ordered_entities;
        let committed = committed_layer_one
            .lock()
            .expect("Arachne capture mutex poisoned")
            .clone()
            .expect("layer 1 must contain a live Arachne perimeter commit");
        let source_walls = &committed.regions[0].walls;

        let identities: Vec<u32> = layer_one
            .iter()
            .map(|entity| {
                source_walls
                    .iter()
                    .find(|wall| wall.path == entity.path)
                    .map(|wall| wall.perimeter_index)
                    .expect("optimized entity must originate from live Arachne output")
            })
            .collect();
        let roles: Vec<_> = layer_one.iter().map(|entity| entity.role.clone()).collect();

        assert_eq!(
            identities, expected_identities,
            "wall_sequence={wall_sequence}"
        );
        assert_eq!(roles, expected_roles, "wall_sequence={wall_sequence}");
    }
}

struct LiveArachneThenPathOpt {
    dispatcher: Arc<WasmRuntimeDispatcher>,
    arachne: LiveModule,
    path_opt: LiveModule,
    committed_layer_one: Arc<Mutex<Option<slicer_ir::PerimeterIR>>>,
}

impl LayerStageRunner for LiveArachneThenPathOpt {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &slicer_ir::GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        let module = match stage_id.as_str() {
            "Layer::Perimeters" => &self.arachne,
            "Layer::PathOptimization" => &self.path_opt,
            other => panic!("unexpected stage in fixture: {other}"),
        };
        let live = module.as_live();
        let commit = self.dispatcher.run_stage(stage_id, layer, &live, input)?;
        if layer.index == 1 {
            if let Some(LayerStageCommit::Perimeters(perimeter)) = &commit {
                *self
                    .committed_layer_one
                    .lock()
                    .expect("Arachne capture mutex poisoned") = Some(perimeter.clone());
            }
        }
        Ok(commit)
    }
}

struct LiveModule {
    module: CompiledModule,
    pool: Arc<WasmInstancePool>,
    component: Arc<WasmComponent>,
}

impl LiveModule {
    fn as_live(&self) -> CompiledModuleLive<'_> {
        CompiledModuleLive::new(
            self.module.module_id(),
            Arc::clone(&self.pool),
            Some(Arc::clone(&self.component)),
            self.module.claims(),
            Arc::clone(self.module.config_view()),
        )
    }
}

fn live_module(
    engine: &WasmEngine,
    id: &str,
    stage: &str,
    relative_wasm: &str,
    config: impl IntoIterator<Item = (&'static str, ConfigValue)>,
) -> LiveModule {
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules")
        .join(relative_wasm);
    assert!(
        wasm_path.exists(),
        "live guest missing: {}",
        wasm_path.display()
    );
    let component = Arc::new(
        engine
            .compile_component(&std::fs::read(&wasm_path).expect("read live guest"))
            .expect("compile live guest"),
    );
    let loaded = slicer_runtime::manifest::LoadedModuleBuilder::new(
        id,
        semver(),
        stage,
        "slicer:world-layer@1.0.0",
        wasm_path,
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
        .expect("build live guest pool"),
    );
    let config = ConfigView::from_map(
        config
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    );
    LiveModule {
        module: CompiledModuleBuilder::new(id.to_string())
            .config_view(Arc::new(config))
            .build(),
        pool,
        component,
    }
}

fn slice(layer: u32) -> SliceIR {
    let side = slicer_ir::mm_to_units(10.0);
    SliceIR {
        global_layer_index: layer,
        z: 0.2 * (layer as f32 + 1.0),
        regions: vec![SlicedRegion {
            object_id: "arachne-fixture".to_string(),
            region_id: 0,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: side, y: 0 },
                        Point2 { x: side, y: side },
                        Point2 { x: 0, y: side },
                    ],
                },
                holes: Vec::new(),
            }],
            effective_layer_height: 0.2,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn plan() -> ExecutionPlan {
    let stages = |(stage_id, module_id): (&str, &str)| CompiledStage {
        stage_id: stage_id.to_string(),
        modules: vec![CompiledModuleBuilder::new(module_id.to_string()).build()],
    };
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            stages(("Layer::Perimeters", ARACHNE_ID)),
            stages(("Layer::PathOptimization", PATH_OPT_ID)),
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(
            (0..2)
                .map(|index| slicer_ir::GlobalLayer {
                    index,
                    z: 0.2 * (index as f32 + 1.0),
                    active_regions: vec![ActiveRegion {
                        object_id: "arachne-fixture".to_string(),
                        region_id: 0,
                        resolved_config: slicer_ir::ResolvedConfig::default(),
                        effective_layer_height: 0.2,
                        nonplanar_shell: None,
                        is_catchup_layer: false,
                        catchup_z_bottom: 0.0,
                        tool_index: 0,
                    }],
                    has_nonplanar: false,
                    is_sync_layer: index == 0,
                })
                .collect(),
        ),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: std::collections::BTreeMap::new(),
    }
}
