//! Integration TDD tests: `PrePass::SupportGeometry` stage contract.
//!
//! Verifies that the new prepass stage wiring (TASK-161) round-trips end
//! to end:
//!   - the `support-planner` core-module emits non-empty `SupportPlanIR`
//!     entries for a fixture with overhang geometry (LIVE WASM dispatch)
//!   - empty overhangs produce an empty plan with no error (LIVE WASM dispatch)
//!   - the stage fails fast with `PrepassExecutionError::MissingRequiredPrepass`
//!     when `LayerPlanIR` is missing
//!   - two modules declaring `holds = ["support-planner"]` on the same stage
//!     dedup to the alphabetical first-winner with an Info diagnostic
//!   - repeated identical runs produce byte-identical plan output
//!     (LIVE WASM dispatch)
//!   - the host built-in commits `SupportGeometryIR` strictly before the
//!     guest's `run-support-geometry` is invoked, and the guest observes a
//!     non-empty `SupportGeometryView`
//!
//! The "LIVE WASM dispatch" tests load `support-planner.wasm`, instantiate
//! it through wasmtime via `WasmRuntimeDispatcher::run_stage`, and verify the
//! `SupportPlanIR` committed to the Blackboard. They do not call any
//! `support_planner` Rust trait method directly â€” every code path crosses
//! the WIT boundary.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer,
    SupportGeometryIR, SupportPlanIR, SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, dedup_same_claim_modules_for_test, execute_prepass,
    execute_prepass_with_builtins, instance_pool::WasmArtifactMetadata, Blackboard,
    BlackboardPrepassSlot, CompiledModule, CompiledModuleBuilder, CompiledStage, DiagnosticLevel,
    ExecutionPlan, LoadDiagnostic, LoadedModule, LoadedModuleBuilder, PrepassExecutionError,
    PrepassStageOutput, PrepassStageRunner, WasmEngine, WasmRuntimeDispatcher,
};

// â”€â”€ Fixtures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn support_planner_wasm() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/support-planner/support-planner.wasm")
}

/// Construct a mesh containing an overhanging plate (downward-facing
/// triangles at zâ‰ˆ1.8 with a build-plate anchor at the origin). The
/// support-planner derives overhangs from the triangle normals visible
/// through `MeshObjectView`, so this is the actual WIT-boundary input
/// shape the wasm guest sees.
fn overhang_plate_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "plate".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    // Anchor so object bounds span z=0..2.0.
                    Point3::default(),
                    // Lower face of the floating plate at z=1.8.
                    Point3 {
                        z: 1.8,
                        ..Default::default()
                    },
                    Point3 {
                        x: 4.0,
                        z: 1.8,
                        ..Default::default()
                    },
                    Point3 {
                        x: 4.0,
                        y: 4.0,
                        z: 1.8,
                    },
                    Point3 {
                        y: 4.0,
                        z: 1.8,
                        ..Default::default()
                    },
                ],
                // CW winding (when viewed from above) â†’ normals point down.
                indices: vec![1, 3, 2, 1, 4, 3],
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

/// Construct a mesh containing only a flat cube â€” every overhanging facet
/// sits on the build plate and is excluded by the planner's layer-0
/// short-circuit. Result: empty plan.
fn flat_cube_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        x: 1.0,
                        y: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        z: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        x: 1.0,
                        z: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                    Point3 {
                        y: 1.0,
                        z: 1.0,
                        ..Default::default()
                    },
                ],
                indices: vec![
                    0, 2, 1, 0, 3, 2, // bottom (overhang at z=0; planner skips layer 0)
                    4, 5, 6, 4, 6, 7, // top
                    0, 1, 5, 0, 5, 4, // front
                    1, 2, 6, 1, 6, 5, // right
                    2, 3, 7, 2, 7, 6, // back
                    3, 0, 4, 3, 4, 7, // left
                ],
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

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn loaded_support_planner_module(id: &str, wasm_path: PathBuf) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(0, 1, 0),
        "PrePass::SupportGeometry",
        "slicer:world-prepass@1.0.0",
        wasm_path,
    )
    .ir_reads(vec![
        "MeshIR.objects".into(),
        "SurfaceClassificationIR.per_object".into(),
        "LayerPlanIR.global_layers".into(),
        "PaintRegionIR.per_layer".into(),
    ])
    .ir_writes(vec!["SupportPlanIR.entries".into()])
    .claims(vec!["support-planner".into()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

fn compile_support_planner(engine: &Arc<WasmEngine>) -> CompiledModule {
    let wasm_path = support_planner_wasm();
    let bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| {
        panic!(
            "support-planner.wasm not found at {}. Build with: \
             ./modules/core-modules/build-core-modules.sh",
            wasm_path.display()
        )
    });
    let component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("support-planner.wasm must compile"),
    );
    let loaded = loaded_support_planner_module("com.core.support-planner", wasm_path);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );
    CompiledModuleBuilder::new(loaded.id().to_string(), pool)
        .config_view(Arc::new(ConfigView::from_map(default_planner_config_map())))
        .wasm_component(Some(component))
        .build()
}

fn default_planner_config_map() -> HashMap<String, ConfigValue> {
    let mut map = HashMap::new();
    map.insert("support_enabled".to_string(), ConfigValue::Bool(true));
    map.insert(
        "support_branch_angle_deg".to_string(),
        ConfigValue::Float(45.0),
    );
    map.insert(
        "support_branch_merge_distance_mm".to_string(),
        ConfigValue::Float(0.8),
    );
    map.insert(
        "support_max_branches_per_layer".to_string(),
        ConfigValue::Int(1024),
    );
    map.insert("line_width".to_string(), ConfigValue::Float(0.4));
    map
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
    }
}

/// Build a Blackboard with the given mesh and pre-commit
/// `LayerPlanIR` (so `PrePass::SupportGeometry`'s prerequisite check
/// passes). `SurfaceClassificationIR` is committed by
/// `execute_prepass_with_builtins`'s built-in mesh-analysis step.
fn blackboard_with_layer_plan(mesh: MeshIR) -> Blackboard {
    // Build a LayerPlanIR with uniform 0.2mm layers covering z=0..2.0.
    // The overhang fixture spans z=0..2.0, so 10 layers at 0.2mm each.
    let num_layers = 10u32;
    let layer_height = 0.2f32;
    // Collect object IDs so active_regions can reference them below.
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
    // Build RegionMapIR entries: one region per (layer, object) pair.
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

fn run_live_support_geometry(mesh: MeshIR) -> Arc<SupportPlanIR> {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = compile_support_planner(&engine);
    let plan = execution_plan_with_support_geometry(module);

    let mut blackboard = blackboard_with_layer_plan(mesh);
    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher)
        .expect("execute_prepass_with_builtins must succeed");

    Arc::clone(
        blackboard
            .support_plan()
            .expect("SupportPlanIR must be committed after live dispatch"),
    )
}

// â”€â”€ Acceptance: positive overhang fixture produces branches (LIVE WASM) â”€â”€

#[test]
fn support_planner_produces_branches_for_overhang_fixture() {
    let plan = run_live_support_geometry(overhang_plate_mesh());

    assert!(
        !plan.entries.is_empty(),
        "overhang fixture must yield a non-empty SupportPlanIR; got 0 entries"
    );

    for entry in &plan.entries {
        assert!(
            !entry.branch_segments.is_empty(),
            "every entry must carry at least one branch segment (entry layer={})",
            entry.global_layer_index
        );
        for segment in &entry.branch_segments {
            assert!(
                segment.points.len() >= 2,
                "every segment must have â‰¥2 points; got {}",
                segment.points.len()
            );
            for p in &segment.points {
                assert!(p.width.is_finite());
                assert!(p.flow_factor.is_finite());
                assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
            }
        }
    }
}

// â”€â”€ Acceptance: determinism across repeated runs (LIVE WASM) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_planner_is_deterministic_across_runs() {
    let first = run_live_support_geometry(overhang_plate_mesh());
    let second = run_live_support_geometry(overhang_plate_mesh());

    assert_eq!(
        first.entries.len(),
        second.entries.len(),
        "entry count must match across repeated runs"
    );

    for (a, b) in first.entries.iter().zip(second.entries.iter()) {
        assert_eq!(a.global_layer_index, b.global_layer_index);
        assert_eq!(a.object_id, b.object_id);
        assert_eq!(a.region_id, b.region_id);
        assert_eq!(
            a.branch_segments.len(),
            b.branch_segments.len(),
            "branch_segments.len() must match across runs"
        );
        for (seg_a, seg_b) in a.branch_segments.iter().zip(b.branch_segments.iter()) {
            assert_eq!(seg_a.points.len(), seg_b.points.len());
            for (pa, pb) in seg_a.points.iter().zip(seg_b.points.iter()) {
                assert_eq!(pa.x.to_bits(), pb.x.to_bits(), "x bits must match");
                assert_eq!(pa.y.to_bits(), pb.y.to_bits(), "y bits must match");
                assert_eq!(pa.z.to_bits(), pb.z.to_bits(), "z bits must match");
                assert_eq!(pa.width.to_bits(), pb.width.to_bits());
                assert_eq!(pa.flow_factor.to_bits(), pb.flow_factor.to_bits());
            }
        }
    }
}

// â”€â”€ Negative: empty overhangs (LIVE WASM) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_planner_emits_empty_plan_when_no_overhangs() {
    let plan = run_live_support_geometry(flat_cube_mesh());
    assert!(
        plan.entries.is_empty(),
        "no-overhang fixture must yield empty SupportPlanIR; got {} entries",
        plan.entries.len()
    );
}

// â”€â”€ Negative: missing LayerPlanIR prerequisite â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn prepass_support_geometry_fails_without_layer_plan() {
    // Arrange: a prepass plan with a PrePass::SupportGeometry stage but
    // no LayerPlanning stage (and no pre-committed LayerPlanIR), so the
    // stage prerequisite check fails before any module runs.
    let mesh = Arc::new(minimal_mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("surface classification pre-commit must succeed");

    let plan = execution_plan_fixture_native(vec![compiled_native_stage(
        "PrePass::SupportGeometry",
        &["com.test.support-planner"],
    )]);

    let runner = NullRunner;

    let err = execute_prepass(&plan, &mut blackboard, &runner).unwrap_err();
    match err {
        PrepassExecutionError::MissingRequiredPrepass { stage_id, slot } => {
            assert_eq!(stage_id, "PrePass::SupportGeometry");
            assert_eq!(
                slot,
                BlackboardPrepassSlot::LayerPlan,
                "missing slot must be LayerPlan; got {slot:?}"
            );
        }
        other => panic!(
            "expected PrepassExecutionError::MissingRequiredPrepass {{ slot: LayerPlan }}, got {other:?}"
        ),
    }
}

// â”€â”€ Negative: claim dedup for two `support-planner` holders â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn support_planner_claim_dedup() {
    let mut modules = vec![
        loaded_support_planner_module(
            "com.core.support-planner-b",
            PathBuf::from("fixtures/com.core.support-planner-b.wasm"),
        ),
        loaded_support_planner_module(
            "com.core.support-planner-a",
            PathBuf::from("fixtures/com.core.support-planner-a.wasm"),
        ),
    ];
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let kept = dedup_same_claim_modules_for_test(&mut modules, &mut diagnostics);

    assert_eq!(
        kept.len(),
        1,
        "exactly one holder of `support-planner` must survive"
    );
    assert_eq!(
        kept[0].id(),
        "com.core.support-planner-a",
        "alphabetical first winner (support-planner-a) must be kept"
    );
    assert_eq!(diagnostics.len(), 1);
    let diag = &diagnostics[0];
    assert_eq!(
        diag.level,
        DiagnosticLevel::Info,
        "claim-dedup diagnostic must be Info level; got {:?}",
        diag.level
    );
    assert!(
        diag.message.contains("dropped: claim 'support-planner'"),
        "diagnostic message must mention dropped support-planner claim; got: {}",
        diag.message
    );
}

// â”€â”€ Anti-regression: blackboard commit path carries SupportPlanIR â”€â”€â”€â”€â”€â”€

#[test]
fn blackboard_accepts_and_returns_support_plan_ir() {
    let mesh = Arc::new(minimal_mesh_fixture());
    let mut blackboard = Blackboard::new(mesh, 0);
    let ir = Arc::new(SupportPlanIR::default());
    blackboard
        .commit_support_plan(Arc::clone(&ir))
        .expect("first commit must succeed");
    assert!(blackboard.support_plan().is_some());

    let second = Arc::clone(&ir);
    match blackboard.commit_support_plan(second) {
        Err(slicer_runtime::BlackboardError::DuplicatePrepassCommit { slot }) => {
            assert_eq!(slot, BlackboardPrepassSlot::SupportPlan);
        }
        other => panic!("expected DuplicatePrepassCommit for SupportPlan; got {other:?}"),
    }
}

#[test]
fn layer_plan_committed_plus_support_geometry_proceeds() {
    // Happy-path mirror of the missing-LayerPlanIR test: when LayerPlanIR is
    // committed beforehand, execute_prepass calls into the runner without a
    // MissingRequiredPrepass error. Uses a synthetic runner returning an
    // empty SupportPlanIR (no module needed; this is host wiring only).
    let mesh = Arc::new(minimal_mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .unwrap();
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR::default()))
        .unwrap();
    blackboard
        .commit_region_map(Arc::new(RegionMapIR::default()))
        .unwrap();
    // SliceIR was added as a SupportGeometry prerequisite in the prepass
    // promotion refactor (Commit 1). Direct execute_prepass tests that bypass
    // the host built-ins must pre-commit an empty Vec to satisfy the check.
    blackboard.commit_slice_ir(Arc::new(Vec::new())).unwrap();
    // PrePass::SupportGeometry built-in (committed via execute_prepass_with_builtins in the
    // real pipeline) produces SupportGeometryIR. For this direct execute_prepass test
    // we pre-commit it so the stage prerequisite check passes.
    blackboard
        .commit_support_geometry(Arc::new(SupportGeometryIR::default()))
        .unwrap();

    let plan = execution_plan_fixture_native(vec![compiled_native_stage(
        "PrePass::SupportGeometry",
        &["com.test.support-planner"],
    )]);

    let runner = EmptyPlanRunner;
    let _ = execute_prepass(&plan, &mut blackboard, &runner)
        .expect("execute_prepass must succeed when LayerPlanIR is present");
    assert!(
        blackboard.support_plan().is_some(),
        "empty SupportPlanIR must be committed by the runner"
    );
}

// â”€â”€ Native (no-WASM) host fixtures used by negative tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn minimal_mesh_fixture() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "plate".to_string(),
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

fn execution_plan_fixture_native(prepass_stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages,
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    }
}

fn compiled_native_stage(stage_id: &str, module_ids: &[&str]) -> CompiledStage {
    CompiledStage {
        stage_id: String::from(stage_id),
        modules: module_ids
            .iter()
            .map(|id| compiled_native_module(stage_id, id))
            .collect(),
    }
}

fn compiled_native_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver(0, 1, 0),
        stage_id,
        "slicer:world-prepass@1.0.0",
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
    .claims(vec!["support-planner".to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture instance pool must build"),
    );
    CompiledModuleBuilder::new(loaded.id().to_string(), pool).build()
}

#[derive(Default)]
struct NullRunner;

impl PrepassStageRunner for NullRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        panic!(
            "prerequisite check must reject stage before any module runs; \
             run_stage should never be called"
        );
    }
}

struct EmptyPlanRunner;

impl PrepassStageRunner for EmptyPlanRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        Ok((
            PrepassStageOutput::SupportPlan(Arc::new(SupportPlanIR::default())),
            Vec::new(),
        ))
    }
}

// â”€â”€ AC: host built-in commits SupportGeometryIR before guest runs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Verifies that in a `PrePass::SupportGeometry` stage with one guest module,
/// the host built-in commits `SupportGeometryIR` strictly before the guest's
/// `run-support-geometry` is invoked, and the guest observes a non-empty
/// `SupportGeometryView`.
///
/// Implementation: uses a spy runner that captures whether `SupportGeometryIR`
/// was already committed on the blackboard at the moment `run_stage` is called.
/// After `execute_prepass_with_builtins` returns, we assert:
///   1. The spy observed a committed (non-None) `SupportGeometryIR`.
///   2. The committed `SupportGeometryIR` has at least one entry.
///   3. The live support-planner module (WASM) ran and committed `SupportPlanIR`.
#[test]
fn host_builtin_runs_before_guest() {
    use std::sync::Mutex;

    // Spy runner: records whether SupportGeometryIR was present when
    // run_stage was called, then delegates to the real WASM dispatcher.
    struct SpyRunner {
        engine: Arc<WasmEngine>,
        saw_support_geometry_before_guest: Arc<Mutex<Option<bool>>>,
    }

    impl PrepassStageRunner for SpyRunner {
        fn run_stage(
            &self,
            stage_id: &slicer_ir::StageId,
            module: &CompiledModule,
            blackboard: &Blackboard,
        ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
            if stage_id == "PrePass::SupportGeometry" {
                // Record whether SupportGeometryIR is already on the blackboard.
                let present = blackboard.support_geometry().is_some();
                *self.saw_support_geometry_before_guest.lock().unwrap() = Some(present);
            }
            // Delegate to the real WasmRuntimeDispatcher.
            let real = WasmRuntimeDispatcher::new(Arc::clone(&self.engine));
            real.run_stage(stage_id, module, blackboard)
        }
    }

    let engine = Arc::new(WasmEngine::new());
    let saw = Arc::new(Mutex::new(None::<bool>));

    let spy = SpyRunner {
        engine: Arc::clone(&engine),
        saw_support_geometry_before_guest: Arc::clone(&saw),
    };

    // Compile the real support-planner guest module.
    let module = compile_support_planner(&engine);
    let plan = execution_plan_with_support_geometry(module);
    let mut blackboard = blackboard_with_layer_plan(overhang_plate_mesh());

    execute_prepass_with_builtins(&plan, &mut blackboard, &spy)
        .expect("execute_prepass_with_builtins must succeed");

    // Assert 1: spy observed SupportGeometryIR committed before the guest ran.
    let observed = saw
        .lock()
        .unwrap()
        .expect("spy must have been called for PrePass::SupportGeometry");
    assert!(
        observed,
        "host built-in must commit SupportGeometryIR before the guest's \
         run-support-geometry is invoked"
    );

    // Assert 2: the committed SupportGeometryIR is non-empty (built-in
    // actually analysed the overhang mesh).
    let sg_ir = blackboard
        .support_geometry()
        .expect("SupportGeometryIR must be present after prepass");
    assert!(
        !sg_ir.entries.is_empty(),
        "host built-in SupportGeometryIR must have at least one entry for \
         the overhang fixture; got 0"
    );

    // Assert 3: the guest ran and committed SupportPlanIR.
    assert!(
        blackboard.support_plan().is_some(),
        "guest run-support-geometry must commit SupportPlanIR after the \
         host built-in ran"
    );
}
