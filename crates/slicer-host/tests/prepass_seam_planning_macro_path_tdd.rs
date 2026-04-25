//! Live-dispatch regression test: `PrePass::SeamPlanning` round-trips through
//! the `#[slicer_module]` macro path via wasmtime.
//!
//! This test was added to lock down a latent bug discovered during packet-28
//! cleanup. Before the fix:
//!
//! - The `STAGES` table in `slicer-schema` did not include `run_seam_planning`,
//!   so `detect_stage_methods` returned empty for `seam-planner-default`'s
//!   `impl PrepassModule` block. The macro then emitted the catch-all
//!   `Ok(())` for the seam arm, and `seam-planner-default.wasm` silently
//!   no-op-ed at the WIT boundary even though it had been "implemented".
//! - The macro's seam arm itself had a type mismatch: it called an SDK helper
//!   (`__slicer_point3_with_width_from_sdk`) that returns
//!   `slicer_ir::Point3WithWidth`, then assigned the result to wit-bindgen's
//!   `Point3WithWidth` field. Once the STAGES entry was added, the macro
//!   emission would fail to compile.
//!
//! Both bugs are fixed; this test verifies the seam-planner-default actually
//! emits at least one `SeamPlanIR.entry` when given a cube fixture by
//! exercising the full host → wasmtime → guest round-trip and committing the
//! resulting `SeamPlanIR` to the blackboard.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, execute_prepass_with_builtins,
    instance_pool::WasmArtifactMetadata, Blackboard, CompiledModule, CompiledStage, ConfigSchema,
    ExecutionPlan, IrAccessMask, LoadedModule, WasmEngine, WasmRuntimeDispatcher,
};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point3, SemVer, Transform3d,
};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn seam_planner_wasm() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/seam-planner-default/seam-planner-default.wasm")
}

/// Minimal cube mesh — seam-planner-default's corner-detection algorithm
/// finds high-curvature vertices, every cube vertex shares 3 mutually
/// perpendicular faces so each is a strong seam candidate.
fn cube_mesh() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 { x: 0.0, y: 0.0, z: 0.0 },
                    Point3 { x: 1.0, y: 0.0, z: 0.0 },
                    Point3 { x: 1.0, y: 1.0, z: 0.0 },
                    Point3 { x: 0.0, y: 1.0, z: 0.0 },
                    Point3 { x: 0.0, y: 0.0, z: 1.0 },
                    Point3 { x: 1.0, y: 0.0, z: 1.0 },
                    Point3 { x: 1.0, y: 1.0, z: 1.0 },
                    Point3 { x: 0.0, y: 1.0, z: 1.0 },
                ],
                indices: vec![
                    0, 2, 1, 0, 3, 2,
                    4, 5, 6, 4, 6, 7,
                    0, 1, 5, 0, 5, 4,
                    1, 2, 6, 1, 6, 5,
                    2, 3, 7, 2, 7, 6,
                    3, 0, 4, 3, 4, 7,
                ],
            },
            transform: Transform3d { matrix: identity4() },
            config: slicer_ir::ObjectConfig { data: HashMap::new() },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    }
}

fn loaded_seam_planner(wasm_path: std::path::PathBuf) -> LoadedModule {
    LoadedModule {
        id: "com.core.seam-planner-default".into(),
        version: semver(0, 1, 0),
        stage: "PrePass::SeamPlanning".into(),
        wit_world: "slicer:world-prepass@1.0.0".into(),
        ir_reads: vec![
            "MeshIR.objects".into(),
            "SurfaceClassificationIR.per_object".into(),
            "LayerPlanIR.global_layers".into(),
        ],
        ir_writes: vec!["SeamPlanIR.entries".into()],
        claims: vec!["seam-planner".into()],
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path,
        placeholder_wasm: false,
    }
}

fn compile_seam_planner(engine: &Arc<WasmEngine>) -> CompiledModule {
    let wasm_path = seam_planner_wasm();
    let bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| {
        panic!(
            "seam-planner-default.wasm not found at {}. Build with: \
             ./modules/core-modules/build-core-modules.sh",
            wasm_path.display()
        )
    });
    let component = Arc::new(
        engine.compile_component(&bytes).expect("seam-planner-default.wasm must compile"),
    );
    let loaded = loaded_seam_planner(wasm_path);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata { uses_shared_memory: false },
        )
        .expect("instance pool must build"),
    );
    let mut config_map = HashMap::new();
    config_map.insert(
        "seam_mode".to_string(),
        ConfigValue::String("nearest".to_string()),
    );
    CompiledModule {
        module_id: loaded.id.clone(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(ConfigView::from_map(config_map)),
        wasm_component: Some(component),
    }
}

#[test]
fn seam_planner_default_live_dispatch_emits_seam_plan_entries() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = compile_seam_planner(&engine);
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SeamPlanning".to_string(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let mesh = Arc::new(cube_mesh());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    // PrePass::SeamPlanning's prerequisite check requires LayerPlanIR.
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            schema_version: semver(1, 0, 0),
            global_layers: Vec::new(),
            object_participation: HashMap::new(),
        }))
        .expect("commit_layer_plan must succeed");

    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher)
        .expect("execute_prepass_with_builtins must succeed for seam planning");

    let seam_plan = blackboard
        .seam_plan()
        .expect("SeamPlanIR must be committed by the seam-planner via the macro path");
    assert!(
        !seam_plan.entries.is_empty(),
        "cube fixture must yield at least one SeamPlanEntry; got {} \
         (regression: macro seam arm previously no-op-ed silently)",
        seam_plan.entries.len()
    );

    // Sanity: every entry's chosen seam position has finite coordinates,
    // proving the wit-bindgen Point3WithWidth marshalling round-trips.
    for entry in &seam_plan.entries {
        let pt = &entry.chosen_candidate.point;
        assert!(
            pt.x.is_finite() && pt.y.is_finite() && pt.z.is_finite(),
            "SeamPlanEntry.chosen_candidate.point must have finite coordinates; got {pt:?}"
        );
    }
}
