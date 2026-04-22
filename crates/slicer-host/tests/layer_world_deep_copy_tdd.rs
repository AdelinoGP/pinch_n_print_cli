#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    execute_per_layer, Blackboard, CompiledModule, CompiledStage, ExecutionPlan, IrAccessMask,
    LayerArena, LayerStageError, LayerStageOutput, LayerStageRunner, LoadedModule, WasmEngine,
    WasmRuntimeDispatcher,
};
use slicer_ir::{
    BoundingBox3, ExPolygon, GlobalLayer, LoopType, MeshIR, PerimeterIR, PerimeterRegion, Point2,
    Point3, Point3WithWidth, Polygon, SemVer, StageId, WallBoundaryType, WallFeatureFlags,
    WallLoop, WidthProfile,
};

const LAYER_GUEST_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/sdk-layer-pathopt-guest.component.wasm"
);

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 1.0, y: 1.0, z: 1.0 },
        },
    })
}

fn load_layer_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    let path = PathBuf::from(LAYER_GUEST_COMPONENT);
    assert!(path.exists(), "layer guest component missing at {}", path.display());
    let bytes = std::fs::read(&path).expect("read layer guest component");
    Arc::new(engine.compile_component(&bytes).expect("compile layer guest component"))
}

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: stage.to_string(),
        wit_world: "slicer:world-layer@1.0.0".to_string(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: Default::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path: PathBuf::from("/dev/null"),
        placeholder_wasm: false,
    }
}

fn make_module(id: &str, stage: &str, component: Arc<slicer_host::WasmComponent>) -> CompiledModule {
    let loaded = make_loaded_module(id, stage);
    let pool = Arc::new(
        build_wasm_instance_pool(&loaded, 1, WasmArtifactMetadata { uses_shared_memory: false })
            .expect("build instance pool"),
    );
    CompiledModule {
        module_id: id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(slicer_ir::ConfigView::new()),
        wasm_component: Some(component),
    }
}

fn make_wall_loop(perimeter_index: u32, z: f32, speed_factor: f32) -> WallLoop {
    let points = vec![
        Point3WithWidth {
            x: perimeter_index as f32,
            y: z,
            z,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: perimeter_index as f32 + 0.5,
            y: z + 0.25,
            z,
            width: 0.45,
            flow_factor: 0.9,
        },
    ];
    WallLoop {
        perimeter_index,
        loop_type: LoopType::Outer,
        path: slicer_ir::ExtrusionPath3D {
            points: points.clone(),
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor,
        },
        width_profile: WidthProfile {
            widths: points.iter().map(|point| point.width).collect(),
        },
        feature_flags: points
            .iter()
            .map(|_| WallFeatureFlags {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: HashMap::new(),
            })
            .collect(),
        boundary_type: WallBoundaryType::Interior,
    }
}

fn make_perimeter_ir_with_ids(layer_index: u32, ids: &[(&str, u64)]) -> PerimeterIR {
    let regions = ids
        .iter()
        .enumerate()
        .map(|(index, (object_id, region_id))| PerimeterRegion {
            object_id: (*object_id).to_string(),
            region_id: *region_id,
            walls: vec![make_wall_loop(index as u32, 0.2 + index as f32 * 0.1, 1.0 + index as f32)],
            infill_areas: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 1, y: 0 },
                        Point2 { x: 1, y: 1 },
                    ],
                },
                holes: Vec::new(),
            }],
            seam_candidates: Vec::new(),
            resolved_seam: None,
        })
        .collect();
    PerimeterIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: layer_index,
        regions,
    }
}

struct SeedingRunner<'a> {
    inner: &'a WasmRuntimeDispatcher,
    perimeter: Mutex<Option<PerimeterIR>>,
}

impl<'a> LayerStageRunner for SeedingRunner<'a> {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>), LayerStageError> {
        if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
            if let Some(perimeter) = self.perimeter.lock().expect("lock seed perimeter").take() {
                arena.set_perimeter(perimeter).expect("seed perimeter into arena");
                return Ok((LayerStageOutput::Success, Vec::new()));
            }
        }

        LayerStageRunner::run_stage(self.inner, stage_id, layer, module, blackboard, arena)
    }
}

#[test]
fn layer_world_builder_commit_preserves_entities_tool_changes_and_z_hops() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_layer_guest(&engine);

    let ids = [("alpha", 11u64), ("beta", 22u64)];
    let seeded_perimeter = make_perimeter_ir_with_ids(0, &ids);
    let expected_paths: Vec<_> = seeded_perimeter
        .regions
        .iter()
        .map(|region| region.walls[0].path.clone())
        .collect();

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            CompiledStage {
                stage_id: "Layer::Perimeters".to_string(),
                modules: vec![make_module(
                    "com.test.layer-world-seed",
                    "Layer::Perimeters",
                    Arc::clone(&component),
                )],
            },
            CompiledStage {
                stage_id: "Layer::PathOptimization".to_string(),
                modules: vec![make_module(
                    "com.test.layer-world-pathopt",
                    "Layer::PathOptimization",
                    component,
                )],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let runner = SeedingRunner {
        inner: &dispatcher,
        perimeter: Mutex::new(Some(seeded_perimeter)),
    };

    let layers = execute_per_layer(&plan, &blackboard, &runner).expect("execute per-layer plan");
    assert_eq!(layers.len(), 1);
    let layer = &layers[0];

    assert_eq!(layer.ordered_entities.len(), 2);
    for (index, entity) in layer.ordered_entities.iter().enumerate() {
        assert_eq!(entity.path, expected_paths[index]);
        assert_eq!(entity.role, slicer_ir::ExtrusionRole::OuterWall);
        assert_eq!(entity.region_key.global_layer_index, 0);
        assert_eq!(entity.region_key.object_id, ids[index].0);
        assert_eq!(entity.region_key.region_id, ids[index].1);
        assert_eq!(entity.topo_order, index as u32);
    }

    assert_eq!(layer.tool_changes.len(), 2);
    for (index, tool_change) in layer.tool_changes.iter().enumerate() {
        assert_eq!(tool_change.after_entity_index, 1);
        assert_eq!(tool_change.from_tool, index as u32);
        assert_eq!(tool_change.to_tool, index as u32 + 1);
    }

    assert_eq!(layer.z_hops.len(), 2);
    for z_hop in &layer.z_hops {
        assert_eq!(z_hop.after_entity_index, 0);
        assert!((z_hop.hop_height - 0.5).abs() < 1e-6);
    }
}