use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, ExtrusionRole, GlobalLayer,
    MeshIR, Point3, Point3WithWidth, SliceIR,
};
use slicer_runtime::layer_executor::execute_per_layer_with_anchored_events;
use slicer_runtime::{Blackboard, NoopLayerProgressSink};
use slicer_scheduler::execution_plan::ExecutionPlan;

fn plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            ..Default::default()
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        ..Default::default()
    }
}

fn entity(path_points: Vec<Point3>) -> AnchoredEntity {
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    AnchoredEntity {
        local_id: 7,
        anchor_global_layer_index: 0,
        geometry: AnchoredGeometryContract::ZSpanning {
            min_z: 2_000,
            max_z: 3_000,
        },
        input_capabilities: Vec::new(),
        output_capabilities: Vec::new(),
        provenance: AnchoredEntityProvenance {
            requesting_feature: "z-span-test".to_string(),
            source_plan_entry: "z-span-test".to_string(),
        },
        path_points: path_points
            .into_iter()
            .map(|point| Point3WithWidth {
                x: point.x,
                y: point.y,
                z: point.z,
                width: 0.45,
                flow_factor: 1.0,
                ..Default::default()
            })
            .collect(),
        role: ExtrusionRole::SupportMaterial,
    }
}

fn commit(
    entities: &[AnchoredEntity],
) -> Result<
    Vec<slicer_ir::OrderedEventCollection>,
    slicer_runtime::layer_executor::LayerExecutionError,
> {
    let plan = plan();
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_slice_ir(Arc::new(vec![SliceIR {
            global_layer_index: 0,
            z: 0.2,
            ..Default::default()
        }]))
        .expect("slice prepass output must be committed");
    let runner = NoopLayerRunner;
    let (_layers, _audits, collections) = execute_per_layer_with_anchored_events(
        &plan,
        &blackboard,
        &runner,
        &NoopLayerProgressSink,
        &HashMap::new(),
        entities,
    )?;
    Ok(collections)
}

pub fn anchored_z_span_validation() {
    let collections = commit(&[entity(vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.2,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.25,
        },
        Point3 {
            x: 2.0,
            y: 2.0,
            z: 0.3,
        },
    ])])
    .expect("valid Z-spanning path must commit");
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].events.len(), 1);
    assert_eq!(collections[0].events[0].path_points.len(), 3);
    assert_eq!(collections[0].events[0].path_points[1].z, 0.25);
}

pub fn rejects_out_of_range_point() {
    let error = commit(&[entity(vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.2,
        },
        Point3 {
            x: 1.0,
            y: 1.0,
            z: 0.31,
        },
    ])])
    .expect_err("out-of-range Z-spanning path must be rejected");
    assert!(error
        .to_string()
        .contains("anchored entity z-span violation"));
}

struct NoopLayerRunner;

impl slicer_runtime::LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _layer: &GlobalLayer,
        _module: &slicer_runtime::CompiledModuleLive<'_>,
        _input: slicer_runtime::LayerStageInput<'_>,
    ) -> Result<Option<slicer_ir::LayerStageCommit>, slicer_ir::LayerStageError> {
        Ok(None)
    }
}
