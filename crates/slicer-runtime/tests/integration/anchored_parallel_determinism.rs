use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, GlobalLayer, Point3,
};
use slicer_runtime::layer_executor::execute_anchored_event_collections_with_mode;
use slicer_scheduler::execution_plan::ExecutionPlan;
use slicer_scheduler::manifest::load_module_from_paths;

fn plan() -> ExecutionPlan {
    ExecutionPlan {
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            ..Default::default()
        }]),
        ..Default::default()
    }
}

fn event(local_id: u64, z: f32) -> AnchoredEntity {
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    AnchoredEntity {
        local_id,
        anchor_global_layer_index: 0,
        geometry: AnchoredGeometryContract::Planar {
            z: slicer_ir::mm_to_units(0.2),
        },
        input_capabilities: vec!["geometry".into()],
        output_capabilities: vec!["toolpath".into()],
        provenance: AnchoredEntityProvenance {
            requesting_feature: format!("feature-{local_id}"),
            source_plan_entry: format!("entry-{local_id}"),
        },
        path_points: vec![
            Point3 {
                x: z,
                y: 0.0,
                z: 0.2,
            },
            Point3 {
                x: z + 1.0,
                y: 1.0,
                z: 0.2,
            },
        ],
    }
}

pub fn anchored_parallel_determinism() {
    let entities = vec![event(3, 2.0), event(1, 0.0), event(2, 1.0)];
    let plan = plan();
    let directory = tempfile::tempdir().expect("manifest fixture directory must be created");
    let manifest_path = directory.path().join("anchored.toml");
    let wasm_path = directory.path().join("anchored.wasm");
    std::fs::write(
        &manifest_path,
        r#"
[module]
id = "test.anchored"
version = "1.0.0"

[stage]
id = "Layer::PathOptimization"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "0.1.0"
max-ir-schema = "1.0.0"

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
layer-parallel-safe = true
"#,
    )
    .expect("manifest fixture must be written");
    std::fs::write(&wasm_path, b"fixture").expect("wasm fixture must be written");
    let module = load_module_from_paths(&manifest_path, &wasm_path)
        .expect("manifest fixture must load through the scheduler");
    assert!(module.layer_parallel_safe());
    let serial = execute_anchored_event_collections_with_mode(&plan, &entities, false, &module)
        .expect("serial anchored execution must succeed");
    let parallel = execute_anchored_event_collections_with_mode(&plan, &entities, true, &module)
        .expect("parallel anchored execution must succeed");

    assert_eq!(serial, parallel);
    assert_eq!(serial.0[0].events, parallel.0[0].events);
    assert_eq!(serial.1, parallel.1);

    let safe = plan.anchored_invocation(&entities[0], module.layer_parallel_safe());
    assert!(safe.layer_parallel_safe);

    std::fs::write(
        &manifest_path,
        std::fs::read_to_string(&manifest_path)
            .expect("safe manifest must be readable")
            .replace("layer-parallel-safe = true", "layer-parallel-safe = false"),
    )
    .expect("unsafe manifest fixture must be written");
    let unsafe_module = load_module_from_paths(&manifest_path, &wasm_path)
        .expect("unsafe manifest fixture must load through the scheduler");
    assert!(!unsafe_module.layer_parallel_safe());
    let gated =
        execute_anchored_event_collections_with_mode(&plan, &entities, true, &unsafe_module)
            .expect("unsafe anchored execution must fall back to serial mode");
    assert_eq!(serial, gated);
}
