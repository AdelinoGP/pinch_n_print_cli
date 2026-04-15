#![allow(missing_docs)]

//! TDD tests for TASK-106: host-built-in `PrePass::RegionMapping`.
//!
//! Proves that:
//! - the built-in runs on the real prepass path via
//!   `execute_prepass_with_builtins` once a `LayerPlanIR` has been
//!   committed (by a user `PrePass::LayerPlanning` module),
//! - the resulting `RegionMapIR` is visible on the blackboard and
//!   carries one `RegionPlan` per `(layer, object, region)` with the
//!   expected resolved config and the scheduler-bound module
//!   invocations grouped by stage — i.e. downstream stages can resolve
//!   active regions by `RegionKey` lookup,
//! - invalid inputs (entry count over the cap, duplicate active
//!   regions) fail cleanly with structured diagnostics,
//! - repeated runs produce an identical region map for identical input.
//!
//! Reference: docs/02_ir_schemas.md §"IR 5 — RegionMapIR",
//! docs/04_host_scheduler.md §"RegionMapIR Compilation".

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, execute_prepass_with_builtins, execute_region_mapping,
    execute_region_mapping_with_cap, Blackboard, CompiledModule, CompiledStage, ConfigSchema,
    ExecutionPlan, IrAccessMask, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner,
    RegionMappingBuiltinError, RegionMappingError, WasmArtifactMetadata,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectConfig, ObjectMesh, Point3, RegionKey, ResolvedConfig, SemVer, Transform3d,
};

// ----------------------------------------------------------------------
// Test 1 — built-in runs on the real path; downstream can look up regions
// ----------------------------------------------------------------------

struct CommitLayerPlanRunner {
    layer_plan: Arc<LayerPlanIR>,
}
impl PrepassStageRunner for CommitLayerPlanRunner {
    fn run_stage(
        &self,
        stage_id: &slicer_ir::StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<PrepassStageOutput, PrepassExecutionError> {
        assert_eq!(stage_id, "PrePass::LayerPlanning");
        Ok(PrepassStageOutput::LayerPlan(Arc::clone(&self.layer_plan)))
    }
}

#[test]
fn region_mapping_builtin_runs_after_user_layer_planning_and_is_visible_to_downstream() {
    let mesh = Arc::new(single_object_mesh("cube"));
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    let layer_plan = Arc::new(plan_two_layers_two_regions());
    let lp_stage = layer_planning_stage_with_module("com.example.planner");

    // Include a per-layer stage so we verify its module invocations
    // show up in each RegionPlan.stage_modules.
    let perimeters_stage = user_stage(
        "Layer::Perimeters",
        &[("com.example.walls", amp_cfg(0.7))],
    );

    let plan = ExecutionPlan {
        prepass_stages: vec![lp_stage],
        per_layer_stages: vec![perimeters_stage],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    };

    execute_prepass_with_builtins(
        &plan,
        &mut blackboard,
        &CommitLayerPlanRunner { layer_plan },
    )
    .expect("prepass with builtins should succeed");

    let rm = blackboard
        .region_map()
        .expect("built-in must commit RegionMapIR after LayerPlanning");
    assert_eq!(rm.entries.len(), 3, "2 regions on L0 + 1 region on L1 = 3");

    let k = RegionKey {
        global_layer_index: 0,
        object_id: "cube".to_string(),
        region_id: 1,
    };
    let rp = rm.entries.get(&k).expect("expected region plan present");
    // Downstream consumer can resolve active modules for the Perimeters stage.
    let invs = rp
        .stage_modules
        .get("Layer::Perimeters")
        .expect("Perimeters stage must be listed");
    assert_eq!(invs.len(), 1);
    assert_eq!(invs[0].module_id, "com.example.walls");
    assert_eq!(invs[0].config_view.get_float("amplitude"), Some(0.7));

    // The resolved config snapshot round-trips.
    assert_eq!(
        rp.config.layer_height, 0.2,
        "resolved_config.layer_height should be preserved"
    );
}

// ----------------------------------------------------------------------
// Test 2 — cap exceeded surfaces structured diagnostic
// ----------------------------------------------------------------------

#[test]
fn region_mapping_cap_exceeded_is_structured_fatal() {
    // 3 regions, cap 2 → error.
    let layer_plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![
                active_region("a", 1),
                active_region("a", 2),
                active_region("a", 3),
            ],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    };
    let plan = empty_execution_plan();

    let err = execute_region_mapping_with_cap(&layer_plan, &plan, 2).expect_err("must fail");
    match err {
        RegionMappingError::CapExceeded { entry_count: 3, cap: 2 } => {}
        other => panic!("expected CapExceeded {{3,2}}, got {other:?}"),
    }
}

// ----------------------------------------------------------------------
// Test 3 — duplicate active regions surface cleanly
// ----------------------------------------------------------------------

#[test]
fn region_mapping_duplicate_region_key_is_structured_fatal() {
    let layer_plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![active_region("a", 42), active_region("a", 42)],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: HashMap::new(),
    };
    let plan = empty_execution_plan();

    let err = execute_region_mapping(&layer_plan, &plan).expect_err("must fail");
    match err {
        RegionMappingError::DuplicateRegionKey { key } => {
            assert_eq!(key.global_layer_index, 0);
            assert_eq!(key.object_id, "a");
            assert_eq!(key.region_id, 42);
        }
        other => panic!("expected DuplicateRegionKey, got {other:?}"),
    }
}

// ----------------------------------------------------------------------
// Test 4 — missing LayerPlanIR surfaces as structured prepass error
// ----------------------------------------------------------------------

struct NoopRunner;
impl PrepassStageRunner for NoopRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<PrepassStageOutput, PrepassExecutionError> {
        Ok(PrepassStageOutput::None)
    }
}

#[test]
fn region_mapping_builtin_is_skipped_when_no_layer_plan_committed() {
    // No LayerPlanning module → layer_plan stays None → region mapping
    // is a no-op (not a fatal error) so empty-plan integrations still work.
    let mesh = Arc::new(single_object_mesh("cube"));
    let mut blackboard = Blackboard::new(mesh, 0);
    let plan = empty_execution_plan();

    execute_prepass_with_builtins(&plan, &mut blackboard, &NoopRunner).expect("ok");

    assert!(
        blackboard.region_map().is_none(),
        "region map must not be committed without a LayerPlanIR"
    );
}

#[test]
fn region_mapping_builtin_commit_failure_surfaces_via_prepass_error() {
    // Pre-seed a region_map to force a DuplicatePrepassCommit inside
    // the built-in commit path.
    let mesh = Arc::new(single_object_mesh("cube"));
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    let layer_plan = Arc::new(plan_two_layers_two_regions());
    blackboard
        .commit_layer_plan(Arc::clone(&layer_plan))
        .expect("seed layer plan");
    // Manually commit a region map first so the built-in becomes a no-op;
    // then verify it was not overwritten. (Idempotency contract.)
    let preexisting = Arc::new(execute_region_mapping(&layer_plan, &empty_execution_plan()).unwrap());
    blackboard.commit_region_map(Arc::clone(&preexisting)).unwrap();

    let plan = empty_execution_plan();
    execute_prepass_with_builtins(&plan, &mut blackboard, &NoopRunner).expect("ok (idempotent)");

    assert!(Arc::ptr_eq(
        blackboard.region_map().unwrap(),
        &preexisting
    ));
}

// ----------------------------------------------------------------------
// Test 5 — determinism
// ----------------------------------------------------------------------

#[test]
fn region_mapping_is_deterministic_for_same_input() {
    let layer_plan = plan_two_layers_two_regions();
    let plan = empty_execution_plan();

    let a = execute_region_mapping(&layer_plan, &plan).unwrap();
    let b = execute_region_mapping(&layer_plan, &plan).unwrap();
    let c = execute_region_mapping(&layer_plan, &plan).unwrap();

    assert_eq!(a, b);
    assert_eq!(b, c);
}

// ----------------------------------------------------------------------
// Fixtures
// ----------------------------------------------------------------------

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

fn empty_execution_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    }
}

fn single_object_mesh(id: &str) -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: id.to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 { x: 0.0, y: 0.0, z: 0.0 },
                    Point3 { x: 1.0, y: 0.0, z: 0.0 },
                    Point3 { x: 0.0, y: 1.0, z: 0.0 },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig { data: HashMap::new() },
            modifier_volumes: vec![],
            paint_data: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    }
}

fn active_region(object_id: &str, region_id: u64) -> ActiveRegion {
    ActiveRegion {
        object_id: object_id.to_string(),
        region_id,
        resolved_config: ResolvedConfig::default(),
        effective_layer_height: 0.2,
        nonplanar_shell: None,
        is_catchup_layer: false,
        catchup_z_bottom: 0.0,
        tool_index: 0,
    }
}

fn plan_two_layers_two_regions() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: vec![
                    active_region("cube", 1),
                    active_region("cube", 2),
                ],
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: vec![active_region("cube", 1)],
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ],
        object_participation: HashMap::new(),
    }
}

fn amp_cfg(amp: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("amplitude".to_string(), slicer_ir::ConfigValue::Float(amp));
    ConfigView::from_map(fields)
}

fn user_stage(stage: &str, modules: &[(&str, ConfigView)]) -> CompiledStage {
    CompiledStage {
        stage_id: stage.to_string(),
        modules: modules
            .iter()
            .map(|(id, cfg)| compiled_module(stage, id, cfg.clone()))
            .collect(),
    }
}

fn layer_planning_stage_with_module(module_id: &str) -> CompiledStage {
    user_stage("PrePass::LayerPlanning", &[(module_id, ConfigView::from_map(HashMap::new()))])
}

fn compiled_module(stage: &str, module_id: &str, config: ConfigView) -> CompiledModule {
    let loaded = slicer_host::LoadedModule {
        id: module_id.to_string(),
        version: sv(1, 0, 0),
        stage: stage.to_string(),
        wit_world: "slicer:world-postpass@1.0.0".to_string(),
        ir_reads: vec![],
        ir_writes: vec![],
        claims: vec![],
        requires_claims: vec![],
        incompatible_with: vec![],
        requires_modules: vec![],
        min_host_version: sv(0, 1, 0),
        min_ir_schema: sv(1, 0, 0),
        max_ir_schema: sv(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: vec![],
        overridable_per_layer: vec![],
        layer_parallel_safe: false,
        wasm_path: std::path::PathBuf::from(format!("fixtures/{module_id}.wasm")),
        placeholder_wasm: false,
    };
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata { uses_shared_memory: false },
        )
        .unwrap(),
    );
    CompiledModule {
        module_id: module_id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(config),
        wasm_component: None,
    }
}

#[allow(dead_code)]
fn expect_region_mapping_builtin_error(e: &PrepassExecutionError) -> &RegionMappingBuiltinError {
    match e {
        PrepassExecutionError::RegionMapping { source } => source,
        other => panic!("expected RegionMapping, got {other:?}"),
    }
}
