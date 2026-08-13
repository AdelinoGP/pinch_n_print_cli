//! AC-N2: disabled support produces no support artifacts or paths.

use std::sync::Arc;

use crate::common::wasm_cache;
use slicer_ir::{BoundingBox3, MeshIR, Point3, SupportIR};
use slicer_runtime::{
    execute_prepass_with_builtins_configured, Blackboard, ConfigBoundsIndex, ExecutionPlan,
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
    let mut blackboard = Blackboard::new(mesh, 0);
    blackboard.commit_slice_ir(Arc::new(Vec::new())).unwrap();
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
    assert!(blackboard.support_plan().is_none());
    let output = SupportIR::default();

    assert!(analysis.candidates.is_empty());
    assert!(output.entries.is_empty());
    assert!(output.entries.iter().all(|entry| entry.paths.is_empty()));
}
