//! Packet 107 (O-T050) + T-024-WIRE-VIEW-CONSUMER: end-to-end overhang-quartile
//! propagation integration.
//!
//! Exercises the REAL upstream half of the pipeline (P106's
//! `PrePass::OverhangAnnotation` builtin, via
//! `execute_prepass_with_builtins_configured_instr` — the same production
//! entry point `run_pipeline` uses) against an overhang-ramp mesh, and the
//! REAL downstream half (P104/T-024's `slicer_core::perimeter_utils::expolygon_to_path3d`,
//! the exact function every `Layer::Perimeters` wall-emission path calls to
//! build `Point3WithWidth` vertices) to prove full propagation: PrePass produces
//! real quartile-banded overhang data, and `expolygon_to_path3d` — given that
//! layer's bands — now stamps `Some(1..=4)` onto overhanging wall vertices.
//!
//! `overhang-classifier-default` (see
//! `modules/core-modules/overhang-classifier-default/src/lib.rs`) is a WASM
//! guest module; invoking it here would require full instance-pool dispatch
//! plumbing outside this packet's context budget. Per this packet's execution
//! rules, the propagation assertions instead mirror the classifier's exact,
//! already-read per-entity governing rule (max per-vertex quartile over
//! `entity.path.points`, `lib.rs` line ~63) directly against the real
//! `expolygon_to_path3d` output — this is not a fabricated re-implementation
//! of unrelated logic, it is the same one-line `Option<u8>` reduction the
//! guest performs, applied to production-real vertex data instead of a mocked
//! struct.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, GlobalLayer, IndexedTriangleSet, LayerPlanIR,
    MeshIR, ModuleId, ObjectLayerRef, ObjectMesh, Point2, Point3, Polygon, PrepassRunnerError,
    PrintEntity, RegionKey, ResolvedConfig, SemVer, StageId, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins_configured_instr, Blackboard,
    CompiledModule, CompiledModuleBuilder, CompiledModuleLive, CompiledStage, ConfigBoundsIndex,
    ExecutionPlan, LoadedModuleBuilder, Phase, PipelineInstrumentation, PrepassStageInput,
    PrepassStageOutput, PrepassStageRunner, SerialEdge, TierKind, WasmArtifactMetadata,
};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Axis-aligned box triangle soup (12 triangles); same winding convention as
/// `prepass_overhang_annotation_stage_order_tdd.rs`'s `box_triangles`.
fn box_triangles(
    base_index: u32,
    (x0, y0, z0): (f32, f32, f32),
    (x1, y1, z1): (f32, f32, f32),
) -> (Vec<Point3>, Vec<u32>) {
    let vertices = vec![
        Point3 {
            x: x0,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z1,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z1,
        },
    ];
    let b = base_index;
    #[rustfmt::skip]
    let indices = vec![
        b, b + 1, b + 2,   b, b + 2, b + 3,
        b + 4, b + 5, b + 6,   b + 4, b + 6, b + 7,
        b, b + 1, b + 5,   b, b + 5, b + 4,
        b + 1, b + 2, b + 6,   b + 1, b + 6, b + 5,
        b + 2, b + 3, b + 7,   b + 2, b + 7, b + 6,
        b + 3, b, b + 4,   b + 3, b + 4, b + 7,
    ];
    (vertices, indices)
}

/// Two 10x10x1mm boxes stacked in Z, upper offset laterally by 5mm so its
/// footprint is NOT fully supported — a real overhang region for
/// `annotate_overhangs`. Same fixture geometry as
/// `prepass_overhang_annotation_stage_order_tdd.rs::overhang_ramp_mesh`.
fn overhang_ramp_mesh() -> MeshIR {
    let (mut vertices, mut indices) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 1.0));
    let (v2, i2) = box_triangles(vertices.len() as u32, (5.0, 0.0, 1.0), (15.0, 10.0, 2.0));
    vertices.extend(v2);
    indices.extend(i2);

    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("ramp"),
            mesh: IndexedTriangleSet { vertices, indices },
            transform: identity_transform(),
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

/// Flat-top single 10x10x2mm cube: no overhang anywhere (AC-N1 fixture).
fn flat_cube_mesh() -> MeshIR {
    let (vertices, indices) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 2.0));
    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: IndexedTriangleSet { vertices, indices },
            transform: identity_transform(),
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

fn two_global_layers() -> Vec<GlobalLayer> {
    vec![
        GlobalLayer {
            index: 0,
            z: 0.5,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        },
        GlobalLayer {
            index: 1,
            z: 1.5,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        },
    ]
}

fn compiled_stub_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver(0, 1, 0),
        stage_id,
        "slicer:world-prepass@1.0.0",
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
    .claims(vec!["layer-planner".to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build();
    let _pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool must build"),
    );
    CompiledModuleBuilder::new(loaded.id().to_string()).build()
}

/// Stub `PrePass::LayerPlanning` runner — mirrors
/// `prepass_overhang_annotation_stage_order_tdd.rs::LayerPlanningStubRunner`.
struct LayerPlanningStubRunner {
    mesh: MeshIR,
    global_layers: Vec<GlobalLayer>,
}

impl PrepassStageRunner for LayerPlanningStubRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        assert_eq!(stage_id, "PrePass::LayerPlanning");
        let mut object_participation = HashMap::new();
        for obj in &self.mesh.objects {
            object_participation.insert(
                obj.id.clone(),
                vec![
                    ObjectLayerRef {
                        local_layer_index: 0,
                        global_layer_index: 0,
                        effective_layer_height: 1.0,
                    },
                    ObjectLayerRef {
                        local_layer_index: 1,
                        global_layer_index: 1,
                        effective_layer_height: 1.0,
                    },
                ],
            );
        }
        Ok(PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
            global_layers: self.global_layers.clone(),
            object_participation,
            ..Default::default()
        })))
    }
}

/// Minimal `PipelineInstrumentation` — no-op; we only need the real prepass
/// builtins to run, not the stage-order trace this packet's sibling test
/// already covers.
#[derive(Default)]
struct NoopInstrumentation;

impl PipelineInstrumentation for NoopInstrumentation {
    fn on_phase_start(&self, _phase: Phase) {}
    fn on_phase_end(&self, _phase: Phase) {}
    fn on_stage_start(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_stage_end(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_module_start(&self, _stage: &StageId, _layer: Option<u32>, _module: &ModuleId) {}
    fn on_module_end(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &ModuleId,
        _wasm_initial_bytes: u64,
        _wasm_peak_bytes: u64,
    ) {
    }
    fn on_layer_start(&self, _layer: u32, _z_mm: f32) {}
    fn on_layer_end(&self, _layer: u32) {}
    fn record_edges(&self, _stage: &StageId, _tier: TierKind, _edges: &[SerialEdge]) {}
}

/// Runs the real `PrePass::OverhangAnnotation` builtin (through the same
/// `execute_prepass_with_builtins_configured_instr` entry point `run_pipeline`
/// uses) over `mesh`/`global_layers` and returns the committed blackboard.
fn run_real_overhang_prepass(mesh: MeshIR, global_layers: Vec<GlobalLayer>) -> Blackboard {
    let mesh_arc = Arc::new(mesh.clone());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh_arc), 0);

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: String::from("PrePass::LayerPlanning"),
            modules: vec![compiled_stub_module(
                "PrePass::LayerPlanning",
                "com.test.layer-planning-stub",
            )],
        }],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    };

    let runner = LayerPlanningStubRunner {
        mesh,
        global_layers,
    };
    let instrumentation = NoopInstrumentation;
    let empty_resolved: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    let default_resolved = ResolvedConfig::default();
    let empty_raw: HashMap<String, slicer_ir::ConfigValue> = HashMap::new();
    let empty_bounds = ConfigBoundsIndex::empty();
    let wasm_handles = HashMap::new();

    // Same caveat as the upstream stage-order test: a downstream builtin may
    // return `Err` on this intentionally-tiny fixture without unwinding the
    // earlier `SurfaceClassificationIR` commit we assert on below.
    let _ = execute_prepass_with_builtins_configured_instr(
        &plan,
        &mut blackboard,
        &runner,
        &empty_resolved,
        &default_resolved,
        &empty_raw,
        &empty_bounds,
        &instrumentation,
        &wasm_handles,
    );

    blackboard
}

/// Builds one `PrintEntity` (an outer-wall loop) using the REAL production
/// function `slicer_core::perimeter_utils::expolygon_to_path3d` — the exact
/// call every `Layer::Perimeters` wall-emission path uses to turn a polygon
/// contour into `Point3WithWidth` vertices. `overhang_bands` should be the
/// real `SurfaceClassificationIR.overhang_quartile_polygons` entry for the
/// wall's own layer (empty slice for layers with no overhang), so this helper
/// does NOT hand-construct any `overhang_quartile` value — it comes entirely
/// from calling the real function with real band data.
fn real_wall_entity(
    entity_id: u64,
    square_mm: (f32, f32, f32, f32),
    z: f32,
    overhang_bands: &[slicer_ir::slice_ir::QuartileBand],
) -> PrintEntity {
    let (x0, y0, x1, y1) = square_mm;
    let contour = Polygon {
        points: vec![
            Point2::from_mm(x0, y0),
            Point2::from_mm(x1, y0),
            Point2::from_mm(x1, y1),
            Point2::from_mm(x0, y1),
        ],
    };
    let points =
        slicer_core::perimeter_utils::expolygon_to_path3d(&contour, z, 0.4, overhang_bands);
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points,
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey::default(),
        topo_order: 0,
        tool_index: 0,
    }
}

/// Mirrors `OverhangClassifierDefault::run_finalization`'s per-entity
/// governing rule (`modules/core-modules/overhang-classifier-default/src/lib.rs`
/// line ~63): "MAX per-vertex quartile governs the whole segment" — `None` if
/// no vertex carries a quartile. Applied here to real `expolygon_to_path3d`
/// output (not a mock), so it changes in lockstep with production reality.
fn classifier_governing_quartile(entity: &PrintEntity) -> Option<u8> {
    entity
        .path
        .points
        .iter()
        .filter_map(|p| p.overhang_quartile)
        .max()
}

// ============================================================================
// AC-5 / T-024-WIRE-VIEW-CONSUMER: full propagation (P106 + P104 both wired)
// ============================================================================

/// Supersedes the old `overhang_pipeline_partial_state_quartile_none` gap
/// tripwire, retired per its own instructions now that T-024-WIRE-VIEW-CONSUMER
/// has landed: `expolygon_to_path3d` no longer unconditionally writes `None` —
/// given the real per-layer quartile bands, it stamps `Some(1..=4)` on
/// overhanging wall vertices.
#[test]
fn overhang_pipeline_full_propagation() {
    // (a) Real PrePass::OverhangAnnotation builtin produces non-empty
    // quartile-banded overhang data for the ramp mesh.
    let blackboard = run_real_overhang_prepass(overhang_ramp_mesh(), two_global_layers());
    let surface_classification = blackboard
        .surface_classification()
        .expect("SurfaceClassificationIR must be committed by PrePass::MeshAnalysis");
    assert!(
        !surface_classification.overhang_quartile_polygons.is_empty(),
        "expected PrePass::OverhangAnnotation to produce at least one global layer index \
         with overhang quartile bands on the ramp mesh, got: {:?}",
        surface_classification.overhang_quartile_polygons
    );

    // (b) The upper box occupies global_layer_index 1 (z=1.5 in
    // `two_global_layers()`) and its footprint (x:[5,15]) extends beyond the
    // lower box's footprint (x:[0,10]) — the x:[10,15] strip is a real
    // overhang region annotated against layer 1's own index. Feed that
    // layer's real bands into `expolygon_to_path3d` via `real_wall_entity`,
    // exactly as `classic-perimeters::emit_walls` does in production.
    let bands = surface_classification
        .overhang_quartile_polygons
        .get(&1)
        .cloned()
        .unwrap_or_default();
    let upper_wall = real_wall_entity(1, (5.0, 0.0, 15.0, 10.0), 1.5, &bands);
    let quartile = classifier_governing_quartile(&upper_wall);
    assert!(
        matches!(quartile, Some(1..=4)),
        "expected an overhang wall vertex to carry Some(1..=4) once T-024 propagation \
         is wired, got {:?}. Points: {:?}",
        quartile,
        upper_wall.path.points
    );

    // (c) Consequently: the classifier's governing rule (max per-vertex
    // quartile) yields a real SetSpeedFactor deviation, not a no-op.
    assert_ne!(
        classifier_governing_quartile(&upper_wall),
        None,
        "with a vertex carrying overhang_quartile, the classifier's max-reduction \
         must yield Some, meaning a real SetSpeedFactor deviation would be emitted"
    );
}

// ============================================================================
// AC-N1: no-overhang case — flat-top cube, no quartile data anywhere
// ============================================================================

#[test]
fn no_overhang_case() {
    let blackboard = run_real_overhang_prepass(flat_cube_mesh(), two_global_layers());
    let surface_classification = blackboard
        .surface_classification()
        .expect("SurfaceClassificationIR must be committed even with zero overhangs");

    // Flat-top cube: no facet is unsupported, so no quartile bands anywhere.
    let has_any_bands = surface_classification
        .overhang_quartile_polygons
        .values()
        .any(|bands| !bands.is_empty());
    assert!(
        !has_any_bands,
        "flat-top cube must produce zero overhang quartile bands, got: {:?}",
        surface_classification.overhang_quartile_polygons
    );

    // Wall vertices carry overhang_quartile = None: real expolygon_to_path3d
    // output fed the real (empty) bands for this layer, not mocked.
    let bands = surface_classification
        .overhang_quartile_polygons
        .get(&1)
        .cloned()
        .unwrap_or_default();
    let wall = real_wall_entity(1, (0.0, 0.0, 10.0, 10.0), 1.5, &bands);
    assert!(
        wall.path
            .points
            .iter()
            .all(|p| p.overhang_quartile.is_none()),
        "no-overhang wall vertices must carry overhang_quartile = None, got: {:?}",
        wall.path.points
    );

    // Zero SetSpeedFactor-derived deviations: classifier's max-reduction is None.
    assert_eq!(
        classifier_governing_quartile(&wall),
        None,
        "no-overhang case must yield zero SetSpeedFactor deviations from the classifier"
    );
}
