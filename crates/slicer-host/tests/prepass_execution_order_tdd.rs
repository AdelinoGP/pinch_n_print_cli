//! TDD test: prepass execution order — `PrePass::SupportGeometry` can succeed
//! without a `PrePass::LayerPlanning` stage in the plan, provided `LayerPlanIR`
//! is already committed on the blackboard (the 31a-REV1 carry-forward).

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, instance_pool::WasmArtifactMetadata, Blackboard,
    BlackboardPrepassSlot, CompiledModule, CompiledStage, ConfigSchema, ExecutionPlan,
    IrAccessMask, LoadedModule, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner,
};
use slicer_ir::{
    BoundingBox3, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectLayerRef,
    ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer, SupportGeometryIR,
    SupportPlanIR, SurfaceClassificationIR, Transform3d,
};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn minimal_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 1.0,
                        ..Default::default()
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

fn blackboard_with_prereqs(mesh: MeshIR) -> Blackboard {
    let num_layers = 2u32;
    let layer_height = 0.2f32;
    let global_layers: Vec<GlobalLayer> = (0..num_layers)
        .map(|i| GlobalLayer {
            index: i,
            z: (i + 1) as f32 * layer_height,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();

    let mut object_participation = HashMap::new();
    for obj in &mesh.objects {
        object_participation.insert(
            obj.id.clone(),
            (0..num_layers)
                .map(|i| ObjectLayerRef {
                    local_layer_index: i,
                    global_layer_index: i,
                    effective_layer_height: layer_height,
                })
                .collect::<Vec<_>>(),
        );
    }

    let mut region_entries = HashMap::new();
    for obj in &mesh.objects {
        for i in 0..num_layers {
            region_entries.insert(
                RegionKey {
                    global_layer_index: i,
                    object_id: obj.id.clone(),
                    region_id: 0,
                },
                RegionPlan::default(),
            );
        }
    }

    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(Arc::clone(&mesh_arc), 0);

    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers,
        object_participation,
        ..Default::default()
    }))
    .expect("commit_layer_plan must succeed");

    bb.commit_region_map(Arc::new(RegionMapIR {
        entries: region_entries,
        ..Default::default()
    }))
    .expect("commit_region_map must succeed");

    bb
}

fn compiled_stub_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModule {
        id: module_id.into(),
        version: semver(0, 1, 0),
        stage: stage_id.to_string(),
        wit_world: "slicer:world-prepass@1.0.0".to_string(),
        ir_reads: vec![],
        ir_writes: vec![],
        claims: vec!["support-planner".to_string()],
        requires_claims: vec![],
        incompatible_with: vec![],
        requires_modules: vec![],
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: vec![],
        overridable_per_layer: vec![],
        layer_parallel_safe: false,
        wasm_path: PathBuf::from(format!("fixtures/{module_id}.wasm")),
        placeholder_wasm: false,
    };
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool must build"),
    );
    CompiledModule {
        module_id: loaded.id.clone(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
        claims: Vec::new(),
        wasm_component: None,
        requires_modules: Vec::new(),
    }
}

/// A stub runner that returns an empty `SupportPlanIR` for any stage call,
/// simulating a tree-support guest that runs inside `PrePass::SupportGeometry`.
struct TreeSupportStubRunner;

impl PrepassStageRunner for TreeSupportStubRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        Ok((
            PrepassStageOutput::SupportPlan(Arc::new(SupportPlanIR::default())),
            vec![],
        ))
    }
}

/// Verifies that a prepass execution plan whose stage list does NOT contain
/// `PrePass::LayerPlanning` but DOES contain `PrePass::SupportGeometry` (and a
/// tree-support-style guest stub) succeeds when `LayerPlanIR`, `RegionMapIR`,
/// and `SupportGeometryIR` are already committed on the blackboard.
///
/// This validates the 31a-REV1 carry-forward: `LayerPlanIR` is available to the
/// host built-in even though no `LayerPlanning` stage is in the plan — callers
/// may pre-seed it externally, and the run must not produce a
/// `PrepassExecutionError::MissingRequiredPrepass` for the `LayerPlan` slot.
#[test]
fn tree_support_plan_succeeds_without_layer_planning_stage() {
    let mesh = minimal_mesh();
    let mut blackboard = blackboard_with_prereqs(mesh);

    // Pre-seed SurfaceClassificationIR (normally produced by the MeshAnalysis
    // built-in; here we seed it directly to isolate the ordering assertion).
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("pre-seeding SurfaceClassificationIR must succeed");

    // Pre-seed SupportGeometryIR (host built-in; satisfy the prereq directly).
    blackboard
        .commit_support_geometry(Arc::new(SupportGeometryIR {
            support_layer_height_mm: 0.2,
            support_top_z_distance_mm: 0.1,
            ..Default::default()
        }))
        .expect("pre-seeding SupportGeometryIR must succeed");

    // Build a plan with ONLY PrePass::SupportGeometry — no LayerPlanning stage.
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: String::from("PrePass::SupportGeometry"),
            modules: vec![compiled_stub_module(
                "PrePass::SupportGeometry",
                "com.test.tree-support-stub",
            )],
        }],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let runner = TreeSupportStubRunner;
    let result = slicer_host::execute_prepass(&plan, &mut blackboard, &runner);

    // Must not fail with a LayerPlan-related prerequisite error.
    if let Err(PrepassExecutionError::MissingRequiredPrepass { ref slot, .. }) = result {
        assert_ne!(
            *slot,
            BlackboardPrepassSlot::LayerPlan,
            "must not fail with missing LayerPlan when it is pre-committed"
        );
    }

    result.expect(
        "execute_prepass must succeed: all prerequisites are pre-committed, \
         no LayerPlanning stage required in the plan",
    );

    // After execution, SupportPlanIR must be committed.
    assert!(
        blackboard.support_plan().is_some(),
        "SupportPlanIR must be committed by the stub runner after successful execution"
    );
}
