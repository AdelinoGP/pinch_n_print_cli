use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, ExtrusionRole, GlobalLayer,
    MeshIR, SliceIR,
};
use slicer_runtime::layer_executor::{
    execute_per_layer_with_committed_anchored_events, CommittedLayerEvent,
};
use slicer_runtime::{Blackboard, NoopLayerProgressSink};
use slicer_scheduler::execution_plan::ExecutionPlan;

fn event(
    local_id: u64,
    anchor_global_layer_index: u32,
    geometry: AnchoredGeometryContract,
    feature: &str,
) -> AnchoredEntity {
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    AnchoredEntity {
        local_id,
        anchor_global_layer_index,
        geometry,
        input_capabilities: Vec::new(),
        output_capabilities: Vec::new(),
        provenance: AnchoredEntityProvenance {
            requesting_feature: feature.to_string(),
            source_plan_entry: feature.to_string(),
        },
        path_points: Vec::new(),
        role: ExtrusionRole::SupportMaterial,
    }
}

fn plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                ..Default::default()
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                ..Default::default()
            },
        ]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        ..Default::default()
    }
}

pub fn anchored_event_ordering() {
    let plan = plan();
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 2);
    blackboard
        .commit_slice_ir(Arc::new(vec![
            SliceIR {
                global_layer_index: 0,
                z: 0.2,
                ..Default::default()
            },
            SliceIR {
                global_layer_index: 1,
                z: 0.4,
                ..Default::default()
            },
        ]))
        .expect("slice prepass output must be committed");

    let entities = vec![
        event(
            2,
            1,
            AnchoredGeometryContract::Planar { z: 3000 },
            "later-planar",
        ),
        event(1, 1, AnchoredGeometryContract::Planar { z: 2500 }, "planar"),
        event(
            3,
            1,
            AnchoredGeometryContract::Planar { z: 4000 },
            "same-z-support",
        ),
    ];
    let runner = NoopLayerRunner;
    let (committed, _audits) = execute_per_layer_with_committed_anchored_events(
        &plan,
        &blackboard,
        &runner,
        &NoopLayerProgressSink,
        &HashMap::new(),
        &entities,
    )
    .expect("global-layer worker must execute");

    assert_eq!(committed.len(), 3);
    let CommittedLayerEvent::Model(first_model) = &committed[0] else {
        panic!("layer zero ordinary model event must be first");
    };
    let CommittedLayerEvent::Anchored(collection) = &committed[1] else {
        panic!("anchored collection must precede its ordinary model event");
    };
    let CommittedLayerEvent::Model(anchor_model) = &committed[2] else {
        panic!("anchor ordinary model event must follow its anchored collection");
    };
    assert_eq!(first_model.global_layer_index, 0);
    assert_eq!(collection.anchor_global_layer_index, 1);
    assert_eq!(anchor_model.global_layer_index, 1);
    assert_eq!(anchor_model.z, 0.4);
    assert_eq!(anchor_model.global_layer_index, 1);
    assert!(anchor_model
        .ordered_entities
        .iter()
        .any(|entity| entity.entity_id == 3));
    assert_eq!(collection.events[0].local_id, 1);
    assert_eq!(collection.events[1].local_id, 2);
    assert_eq!(
        collection.events[0].geometry,
        AnchoredGeometryContract::Planar { z: 2500 }
    );
    assert!(2500 < (anchor_model.z * 10_000.0) as i64);
    assert_eq!(collection.events.len(), 2);
    assert!(collection.events.iter().all(|entity| entity.local_id != 3));
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
