//! Round-trip test for prepass diagnostics (packet 118).
//!
//! AC-3:  The sdk-support-diagnostic-guest emits a diagnostic with
//!        code=99, severity=Warn, layer=Some(-1), object_id=Some("cube"),
//!        message="round-trip". The prepass audit must contain exactly
//!        one record with those five field values.
//! AC-N2: The same guest emits code=99 (outside the support-planner
//!        allocation convention); the host must not enforce a code range.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer,
    Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins, instance_pool::WasmArtifactMetadata,
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledStage, ExecutionPlan,
    LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};

use crate::common::{wasm_cache, TestModuleBundle};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn cube_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 10.0,
                        ..Default::default()
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 10.0,
                        ..Default::default()
                    },
                ],
                indices: vec![0, 1, 2, 0, 2, 3],
            },
            transform: Transform3d {
                matrix: identity4(),
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

fn blackboard_with_layer_plan(mesh: MeshIR) -> Blackboard {
    let num_layers = 5u32;
    let layer_height = 0.2f32;
    let object_ids: Vec<String> = mesh.objects.iter().map(|o| o.id.clone()).collect();
    let global_layers: Vec<GlobalLayer> = (0..num_layers)
        .map(|i| {
            let regions = object_ids
                .iter()
                .map(|oid| ActiveRegion {
                    object_id: oid.clone(),
                    region_id: 0,
                    resolved_config: slicer_ir::ResolvedConfig::default(),
                    effective_layer_height: layer_height,
                    nonplanar_shell: None,
                    is_catchup_layer: false,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                })
                .collect();
            GlobalLayer {
                index: i,
                z: (i + 1) as f32 * layer_height,
                active_regions: regions,
                has_nonplanar: false,
                is_sync_layer: false,
            }
        })
        .collect();
    let mut object_participation = HashMap::new();
    for obj in &mesh.objects {
        object_participation.insert(
            obj.id.clone(),
            (0..num_layers)
                .map(|i| slicer_ir::ObjectLayerRef {
                    local_layer_index: i,
                    global_layer_index: i,
                    effective_layer_height: layer_height,
                })
                .collect(),
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
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
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

fn compile_diagnostic_guest(
    _engine: &Arc<WasmEngine>,
    config: HashMap<String, ConfigValue>,
) -> TestModuleBundle {
    let component = wasm_cache::compiled_guest("sdk-support-diagnostic-guest");
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-wasm-host")
        .join("test-guests")
        .join("sdk-support-diagnostic-guest.component.wasm");
    let loaded = LoadedModuleBuilder::new(
        "com.test.diagnostic-guest",
        semver(0, 1, 0),
        "PrePass::SupportGeometry",
        slicer_schema::WORLD_PREPASS,
        wasm_path,
    )
    .ir_reads(Vec::<String>::new())
    .ir_writes(vec!["SupportPlanIR.entries".into()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(ConfigView::from_map(config)))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn execution_plan_with_support_geometry(module: CompiledModule) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SupportGeometry".to_string(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

fn base_config() -> HashMap<String, ConfigValue> {
    let mut map = HashMap::new();
    map.insert("enable_support".to_string(), ConfigValue::Bool(true));
    map.insert("line_width".to_string(), ConfigValue::Float(0.4));
    map
}

/// AC-3: The diagnostic guest emits code=99, severity=Warn, layer=Some(-1),
/// object_id=Some("cube"), message="round-trip". The audit must contain
/// exactly one record with those five field values.
#[test]
fn support_geometry_diagnostic_round_trips() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let config = base_config();
    let bundle = compile_diagnostic_guest(&engine, config);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);
    let mut blackboard = blackboard_with_layer_plan(cube_mesh());

    let audits = execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &wasm_handles)
        .expect("execute_prepass_with_builtins must succeed");

    assert!(
        !audits.is_empty(),
        "expected at least one ModuleAccessAudit; got empty"
    );

    let audit = &audits[0];
    let diags = &audit.diagnostics;

    assert_eq!(
        diags.len(),
        1,
        "expected exactly 1 diagnostic; got {}: {:?}",
        diags.len(),
        diags
    );

    let d = &diags[0];
    assert_eq!(
        d.severity,
        slicer_ir::DiagnosticSeverity::Warn,
        "expected severity=Warn; got {:?}",
        d.severity
    );
    assert_eq!(d.code, 99, "expected code=99; got {}", d.code);
    assert_eq!(
        d.layer,
        Some(-1),
        "expected layer=Some(-1); got {:?}",
        d.layer
    );
    assert_eq!(
        d.object_id,
        Some("cube".to_string()),
        "expected object_id=Some(\"cube\"); got {:?}",
        d.object_id
    );
    assert_eq!(
        d.message, "round-trip",
        "expected message=\"round-trip\"; got {:?}",
        d.message
    );
}

/// AC-N2: The same guest emits code=99 outside the support-planner
/// allocation convention; the host must not enforce a code range.
#[test]
fn out_of_range_code_is_captured() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let config = base_config();
    let bundle = compile_diagnostic_guest(&engine, config);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);
    let mut blackboard = blackboard_with_layer_plan(cube_mesh());

    let audits = execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &wasm_handles)
        .expect("execute_prepass_with_builtins must succeed");

    assert!(
        !audits.is_empty(),
        "expected at least one ModuleAccessAudit; got empty"
    );

    let audit = &audits[0];
    let diags = &audit.diagnostics;

    assert!(
        !diags.is_empty(),
        "expected at least one diagnostic; got empty"
    );

    // AC-N2: code=99 must survive (no host range enforcement).
    assert_eq!(
        diags[0].code, 99,
        "expected code=99 to survive; got {}",
        diags[0].code
    );
}
