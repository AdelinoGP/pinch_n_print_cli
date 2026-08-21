use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, GlobalLayer, MeshIR,
    Point3, SliceIR,
};
use slicer_runtime::layer_executor::execute_anchored_event_collections_with_accounting;
use slicer_runtime::{Blackboard, NoopLayerProgressSink};
use slicer_scheduler::execution_plan::ExecutionPlan;

fn plan() -> ExecutionPlan {
    ExecutionPlan {
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
        ..Default::default()
    }
}

fn event(local_id: u64, z: f32, feature: &str) -> AnchoredEntity {
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    AnchoredEntity {
        local_id,
        anchor_global_layer_index: 1,
        geometry: AnchoredGeometryContract::Planar {
            z: slicer_ir::mm_to_units(z),
        },
        input_capabilities: Vec::new(),
        output_capabilities: vec!["Layer::PathOptimization".to_string()],
        provenance: AnchoredEntityProvenance {
            requesting_feature: feature.to_string(),
            source_plan_entry: feature.to_string(),
        },
        path_points: vec![
            Point3 {
                x: z + local_id as f32,
                y: 1.0,
                z,
            },
            Point3 { x: z, y: 0.0, z },
        ],
    }
}

fn run(
    entities: &[AnchoredEntity],
) -> (
    Vec<slicer_ir::OrderedEventCollection>,
    Vec<slicer_runtime::layer_executor::AnchoredEventAccounting>,
) {
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

    // Exercise the anchored worker boundary as well as its accounting seam.
    slicer_runtime::layer_executor::execute_per_layer_with_anchored_events(
        &plan,
        &blackboard,
        &NoopLayerRunner,
        &NoopLayerProgressSink,
        &std::collections::HashMap::new(),
        entities,
    )
    .expect("anchored worker must execute");

    execute_anchored_event_collections_with_accounting(&plan, entities, 40.0)
        .expect("anchored accounting must execute")
}

pub fn anchored_event_accounting() {
    let entities = vec![
        event(2, 0.3000, "later-event"),
        event(1, 0.2500, "first-event"),
    ];
    let (collections, accounting) = run(&entities);
    assert_eq!(collections.len(), 1);
    assert_eq!(
        collections[0]
            .events
            .iter()
            .map(|e| e.local_id)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(
        accounting
            .iter()
            .map(|a| a.event_local_id)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(
        accounting.iter().map(|a| a.topo_order).collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(accounting.iter().all(|a| a.cooling_fan_speed > 0.0));
    assert_ne!(
        accounting[0].cooling_fan_speed,
        accounting[1].cooling_fan_speed
    );
    assert!(accounting.iter().all(|a| a.time_s > 0.0));
    assert!(collections[0].events.iter().all(|event| {
        event.path_points.first().expect("event path").x
            < event.path_points.last().expect("event path").x
    }));
    assert_eq!(
        collections[0].events[0].provenance.requesting_feature,
        "later-event"
    );
    assert_eq!(
        collections[0].events[1].provenance.requesting_feature,
        "first-event"
    );

    let (collections_again, accounting_again) = run(&entities);
    assert_eq!(collections, collections_again);
    assert_eq!(accounting, accounting_again);
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
