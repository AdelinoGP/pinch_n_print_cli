//! Packet 137: `PrePass::LightningTreeGen` skip / commit behavior (AC-3).
//!
//! Verifies the host built-in is **skipped** (no commit, slot stays `None`)
//! when the print's `sparse_fill_holder` is not `lightning-infill`, and
//! commits an empty-but-valid `LightningTreeIR` when it is.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::SliceIR;
use slicer_ir::{
    BoundingBox3, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectLayerRef, ObjectMesh,
    Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SurfaceClassificationIR,
    Transform3d,
};
use slicer_runtime::{
    execute_prepass_with_builtins_configured, Blackboard, ExecutionPlan, PrepassStageInput,
    PrepassStageOutput, PrepassStageRunner,
};

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
                    variant_chain: Vec::new(),
                },
                RegionPlan::default(),
            );
        }
    }

    let mut bb = Blackboard::new(Arc::new(mesh), 0);
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
    let stub_slices: Vec<SliceIR> = (0..num_layers)
        .map(|i| SliceIR {
            global_layer_index: i,
            ..SliceIR::default()
        })
        .collect();
    bb.commit_slice_ir(Arc::new(stub_slices))
        .expect("commit_slice_ir must succeed");
    bb.commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("pre-seeding SurfaceClassificationIR must succeed");
    bb
}

struct NoopRunner;

impl PrepassStageRunner for NoopRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_runtime::CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, slicer_ir::PrepassRunnerError> {
        Ok(PrepassStageOutput::None)
    }
}

fn empty_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

#[test]
fn lightning_prepass_is_skipped_when_sparse_fill_holder_is_rectilinear() {
    let mesh = minimal_mesh();
    let mut blackboard = blackboard_with_prereqs(mesh);

    let default_resolved = ResolvedConfig::default();
    assert_ne!(default_resolved.sparse_fill_holder, "lightning-infill");

    let result = execute_prepass_with_builtins_configured(
        &empty_plan(),
        &mut blackboard,
        &NoopRunner,
        &BTreeMap::new(),
        &default_resolved,
        &HashMap::new(),
        &slicer_runtime::ConfigBoundsIndex::default(),
        &HashMap::new(),
    );

    result.expect("prepass execution must succeed when lightning is not configured");
    assert!(
        blackboard.lightning_tree_ir().is_none(),
        "LightningTreeIR slot must stay None when sparse_fill_holder != lightning-infill (AC-3 skip promise)"
    );
}

#[test]
fn lightning_prepass_commits_when_sparse_fill_holder_is_lightning() {
    let mesh = minimal_mesh();
    let mut blackboard = blackboard_with_prereqs(mesh);

    let default_resolved = ResolvedConfig {
        sparse_fill_holder: String::from("lightning-infill"),
        ..ResolvedConfig::default()
    };

    let result = execute_prepass_with_builtins_configured(
        &empty_plan(),
        &mut blackboard,
        &NoopRunner,
        &BTreeMap::new(),
        &default_resolved,
        &HashMap::new(),
        &slicer_runtime::ConfigBoundsIndex::default(),
        &HashMap::new(),
    );

    result.expect("prepass execution must succeed when lightning is configured");
    assert!(
        blackboard.lightning_tree_ir().is_some(),
        "LightningTreeIR must be committed when sparse_fill_holder == lightning-infill"
    );
}
