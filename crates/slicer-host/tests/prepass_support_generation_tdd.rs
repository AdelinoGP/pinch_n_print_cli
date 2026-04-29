//! Integration TDD tests: `PrePass::SupportGeneration` stage contract.
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
//!
//! The "LIVE WASM dispatch" tests load `support-planner.wasm`, instantiate
//! it through wasmtime via `WasmRuntimeDispatcher::run_stage`, and verify the
//! `SupportPlanIR` committed to the Blackboard. They do not call any
//! `support_planner` Rust trait method directly — every code path crosses
//! the WIT boundary.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, dedup_same_claim_modules_for_test, execute_prepass,
    execute_prepass_with_builtins, instance_pool::WasmArtifactMetadata, Blackboard,
    BlackboardPrepassSlot, CompiledModule, CompiledStage, ConfigSchema, DiagnosticLevel,
    ExecutionPlan, IrAccessMask, LoadDiagnostic, LoadedModule, PrepassExecutionError,
    PrepassStageOutput, PrepassStageRunner, WasmEngine, WasmRuntimeDispatcher,
};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point3, SemVer, SupportPlanIR, SurfaceClassificationIR, Transform3d,
};

// ── Fixtures ──────────────────────────────────────────────────────────────

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
/// triangles at z≈1.8 with a build-plate anchor at the origin). The
/// support-planner derives overhangs from the triangle normals visible
/// through `MeshObjectView`, so this is the actual WIT-boundary input
/// shape the wasm guest sees.
fn overhang_plate_mesh() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "plate".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    // Anchor so object bounds span z=0..2.0.
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    // Lower face of the floating plate at z=1.8.
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.8,
                    },
                    Point3 {
                        x: 4.0,
                        y: 0.0,
                        z: 1.8,
                    },
                    Point3 {
                        x: 4.0,
                        y: 4.0,
                        z: 1.8,
                    },
                    Point3 {
                        x: 0.0,
                        y: 4.0,
                        z: 1.8,
                    },
                ],
                // CW winding (when viewed from above) → normals point down.
                indices: vec![1, 3, 2, 1, 4, 3],
            },
            transform: Transform3d {
                matrix: identity4(),
            },
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

/// Construct a mesh containing only a flat cube — every overhanging facet
/// sits on the build plate and is excluded by the planner's layer-0
/// short-circuit. Result: empty plan.
fn flat_cube_mesh() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 1.0,
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
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn loaded_support_planner_module(id: &str, wasm_path: PathBuf) -> LoadedModule {
    LoadedModule {
        id: id.into(),
        version: semver(0, 1, 0),
        stage: "PrePass::SupportGeneration".into(),
        wit_world: "slicer:world-prepass@1.0.0".into(),
        ir_reads: vec![
            "MeshIR.objects".into(),
            "SurfaceClassificationIR.per_object".into(),
            "LayerPlanIR.global_layers".into(),
            "PaintRegionIR.per_layer".into(),
        ],
        ir_writes: vec!["SupportPlanIR.entries".into()],
        claims: vec!["support-planner".into()],
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
    CompiledModule {
        module_id: loaded.id.clone(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(ConfigView::from_map(default_planner_config_map())),
        wasm_component: Some(component),
    }
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

fn execution_plan_with_support_generation(module: CompiledModule) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SupportGeneration".to_string(),
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
/// `LayerPlanIR` (so `PrePass::SupportGeneration`'s prerequisite check
/// passes). `SurfaceClassificationIR` is committed by
/// `execute_prepass_with_builtins`'s built-in mesh-analysis step.
fn blackboard_with_layer_plan(mesh: MeshIR) -> Blackboard {
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers: Vec::new(),
        object_participation: HashMap::new(),
    }))
    .expect("commit_layer_plan must succeed");
    bb
}

fn run_live_support_generation(mesh: MeshIR) -> Arc<SupportPlanIR> {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = compile_support_planner(&engine);
    let plan = execution_plan_with_support_generation(module);

    let mut blackboard = blackboard_with_layer_plan(mesh);
    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher)
        .expect("execute_prepass_with_builtins must succeed");

    Arc::clone(
        blackboard
            .support_plan()
            .expect("SupportPlanIR must be committed after live dispatch"),
    )
}

// ── Acceptance: positive overhang fixture produces branches (LIVE WASM) ──

#[test]
fn support_planner_produces_branches_for_overhang_fixture() {
    let plan = run_live_support_generation(overhang_plate_mesh());

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
                "every segment must have ≥2 points; got {}",
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

// ── Acceptance: determinism across repeated runs (LIVE WASM) ─────────────

#[test]
fn support_planner_is_deterministic_across_runs() {
    let first = run_live_support_generation(overhang_plate_mesh());
    let second = run_live_support_generation(overhang_plate_mesh());

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

// ── Negative: empty overhangs (LIVE WASM) ──────────────────────────────

#[test]
fn support_planner_emits_empty_plan_when_no_overhangs() {
    let plan = run_live_support_generation(flat_cube_mesh());
    assert!(
        plan.entries.is_empty(),
        "no-overhang fixture must yield empty SupportPlanIR; got {} entries",
        plan.entries.len()
    );
}

// ── Negative: missing LayerPlanIR prerequisite ────────────────────────

#[test]
fn prepass_support_generation_fails_without_layer_plan() {
    // Arrange: a prepass plan with a PrePass::SupportGeneration stage but
    // no LayerPlanning stage (and no pre-committed LayerPlanIR), so the
    // stage prerequisite check fails before any module runs.
    let mesh = Arc::new(minimal_mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR {
            schema_version: semver(1, 0, 0),
            per_object: HashMap::new(),
        }))
        .expect("surface classification pre-commit must succeed");

    let plan = execution_plan_fixture_native(vec![compiled_native_stage(
        "PrePass::SupportGeneration",
        &["com.test.support-planner"],
    )]);

    let runner = NullRunner::default();

    let err = execute_prepass(&plan, &mut blackboard, &runner).unwrap_err();
    match err {
        PrepassExecutionError::MissingRequiredPrepass { stage_id, slot } => {
            assert_eq!(stage_id, "PrePass::SupportGeneration");
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

// ── Negative: claim dedup for two `support-planner` holders ───────────

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
        kept[0].id, "com.core.support-planner-a",
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

// ── Anti-regression: blackboard commit path carries SupportPlanIR ──────

#[test]
fn blackboard_accepts_and_returns_support_plan_ir() {
    let mesh = Arc::new(minimal_mesh_fixture());
    let mut blackboard = Blackboard::new(mesh, 0);
    let ir = Arc::new(SupportPlanIR {
        schema_version: semver(1, 0, 0),
        entries: Vec::new(),
    });
    blackboard
        .commit_support_plan(Arc::clone(&ir))
        .expect("first commit must succeed");
    assert!(blackboard.support_plan().is_some());

    let second = Arc::clone(&ir);
    match blackboard.commit_support_plan(second) {
        Err(slicer_host::BlackboardError::DuplicatePrepassCommit { slot }) => {
            assert_eq!(slot, BlackboardPrepassSlot::SupportPlan);
        }
        other => panic!("expected DuplicatePrepassCommit for SupportPlan; got {other:?}"),
    }
}

#[test]
fn layer_plan_committed_plus_support_generation_proceeds() {
    // Happy-path mirror of the missing-LayerPlanIR test: when LayerPlanIR is
    // committed beforehand, execute_prepass calls into the runner without a
    // MissingRequiredPrepass error. Uses a synthetic runner returning an
    // empty SupportPlanIR (no module needed; this is host wiring only).
    let mesh = Arc::new(minimal_mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR {
            schema_version: semver(1, 0, 0),
            per_object: HashMap::new(),
        }))
        .unwrap();
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            schema_version: semver(1, 0, 0),
            global_layers: Vec::new(),
            object_participation: HashMap::new(),
        }))
        .unwrap();

    let plan = execution_plan_fixture_native(vec![compiled_native_stage(
        "PrePass::SupportGeneration",
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

// ── Native (no-WASM) host fixtures used by negative tests ─────────────

fn minimal_mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "plate".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: identity4(),
            },
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
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
    let loaded = LoadedModule {
        id: module_id.into(),
        version: semver(0, 1, 0),
        stage: stage_id.to_string(),
        wit_world: "slicer:world-prepass@1.0.0".to_string(),
        ir_reads: vec![],
        ir_writes: vec![],
        claims: vec!["support-planner".to_string()],
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
        wasm_path: PathBuf::from(format!("fixtures/{module_id}.wasm")),
        placeholder_wasm: false,
    };
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
    CompiledModule {
        module_id: loaded.id.clone(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
        wasm_component: None,
    }
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
            PrepassStageOutput::SupportPlan(Arc::new(SupportPlanIR {
                schema_version: semver(1, 0, 0),
                entries: Vec::new(),
            })),
            Vec::new(),
        ))
    }
}
