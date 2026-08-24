//! AC-N2: disabled support produces no support artifacts or paths.

use std::sync::{Arc, Mutex};

use crate::common::wasm_cache;
use slicer_ir::{BoundingBox3, GlobalLayer, MeshIR, Point3, SliceIR};
use slicer_runtime::{
    execute_per_layer_with_anchored_events, execute_prepass_with_builtins_configured, Blackboard,
    CompiledModuleBuilder, CompiledStage, ConfigBoundsIndex, ExecutionPlan, LayerStageRunner,
    NoopLayerProgressSink,
};

pub fn support_disabled_no_output() {
    let mesh = Arc::new(MeshIR {
        build_volume: BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 10.0,
                y: 10.0,
                z: 10.0,
            },
        },
        ..Default::default()
    });
    let mut blackboard = Blackboard::new(mesh, 1);
    blackboard
        .commit_slice_ir(Arc::new(vec![SliceIR {
            global_layer_index: 0,
            z: 0.2,
            ..Default::default()
        }]))
        .unwrap();
    let resolved = slicer_ir::ResolvedConfig {
        support_enabled: false,
        ..Default::default()
    };
    execute_prepass_with_builtins_configured(
        &ExecutionPlan::default(),
        &mut blackboard,
        &slicer_runtime::WasmRuntimeDispatcher::new(wasm_cache::shared_engine()),
        &std::collections::BTreeMap::new(),
        &resolved,
        &std::collections::HashMap::new(),
        &ConfigBoundsIndex::empty(),
        &std::collections::HashMap::new(),
    )
    .unwrap();

    let analysis = blackboard
        .support_analysis()
        .expect("support analysis must be committed");
    assert!(analysis.candidates.is_empty());
    assert!(blackboard
        .support_plan()
        .is_none_or(|plan| plan.entries.is_empty()));

    let layer_plan = ExecutionPlan {
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            ..Default::default()
        }]),
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Support".into(),
            modules: vec![CompiledModuleBuilder::new("com.test.disabled-support").build()],
        }],
        ..Default::default()
    };
    let support_commit = Arc::new(Mutex::new(None));
    let (layers, _audits, anchored) = execute_per_layer_with_anchored_events(
        &layer_plan,
        &blackboard,
        &NoSupportLayerRunner {
            support_commit: Arc::clone(&support_commit),
        },
        &NoopLayerProgressSink,
        &std::collections::HashMap::new(),
        &[],
    )
    .expect("disabled support layer execution must succeed");

    assert!(
        anchored.is_empty(),
        "disabled support must emit no anchored events"
    );
    assert_eq!(layers.len(), 1);
    assert!(layers[0].ordered_entities.is_empty());
    assert!(layers[0].ordered_entities.iter().all(|entity| {
        entity.role != slicer_ir::ExtrusionRole::SupportMaterial && entity.path.points.is_empty()
    }));
    let support = support_commit
        .lock()
        .expect("support commit lock must not be poisoned")
        .clone()
        .expect("Layer::Support must commit through the layer harness");
    assert!(support.entries.is_empty());
    assert!(support.entries.iter().all(|entry| entry.paths.is_empty()));
}

struct NoSupportLayerRunner {
    support_commit: Arc<Mutex<Option<slicer_ir::SupportIR>>>,
}

impl LayerStageRunner for NoSupportLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _layer: &GlobalLayer,
        _module: &slicer_runtime::CompiledModuleLive<'_>,
        input: slicer_runtime::LayerStageInput<'_>,
    ) -> Result<Option<slicer_ir::LayerStageCommit>, slicer_ir::LayerStageError> {
        assert!(input
            .support_plan
            .as_ref()
            .is_none_or(|plan| plan.entries.is_empty()));
        let support = slicer_ir::SupportIR::default();
        *self
            .support_commit
            .lock()
            .expect("support commit lock must not be poisoned") = Some(support.clone());
        Ok(Some(slicer_ir::LayerStageCommit::Support(support)))
    }
}
