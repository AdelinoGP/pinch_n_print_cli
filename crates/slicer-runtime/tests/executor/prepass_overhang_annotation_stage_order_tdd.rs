//! TDD (Step 5, O-T020/O-T023): `PrePass::OverhangAnnotation` host built-in
//! wiring and stage-order enforcement.
//!
//! AC-6 (positive): running the instrumented builtins prepass pipeline on a
//! small overhang fixture (two axis-aligned boxes, the upper box laterally
//! offset so it partially overhangs the lower box's footprint) must (a)
//! record `PrePass::OverhangAnnotation` in the stage-order trace strictly
//! after `PrePass::MeshAnalysis` and `PrePass::LayerPlanning`, and (b) leave
//! the committed `SurfaceClassificationIR.overhang_quartile_polygons`
//! non-empty for at least one global layer index.
//!
//! AC-N2 (negative, `violation_case`): a hand-built plan that places
//! `PrePass::OverhangAnnotation` BEFORE `PrePass::LayerPlanning` must be
//! rejected by [`slicer_runtime::ensure_stage_prerequisites`] /
//! `execute_prepass` with a deterministic `MissingRequiredPrepass { slot:
//! LayerPlan }` error, mirroring the ordering-violation shape asserted by
//! `prepass_execution_order_tdd.rs`.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_ir::{
    BoundingBox3, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ModuleId, ObjectLayerRef,
    ObjectMesh, Point3, PrepassRunnerError, ResolvedConfig, SemVer, StageId, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass, execute_prepass_with_builtins_configured_instr,
    Blackboard, BlackboardPrepassSlot, CompiledModule, CompiledModuleBuilder, CompiledModuleLive,
    CompiledStage, ConfigBoundsIndex, ExecutionPlan, LoadedModuleBuilder, Phase,
    PipelineInstrumentation, PrepassExecutionError, PrepassStageInput, PrepassStageOutput,
    PrepassStageRunner, SerialEdge, TierKind, WasmArtifactMetadata,
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

/// Column-major 4x4 identity transform translated by `(0, 0, z_mm)` — used by
/// the non-identity-transform regression test (packet 106 fix-iteration
/// finding 1) to prove `commit_overhang_annotation_builtin` applies
/// `object.transform` before cross-sectioning, per `cross_section_at_z`'s
/// documented contract (`crates/slicer-core/src/algos/mesh_cross_section.rs`).
fn z_translation_transform(z_mm: f64) -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, z_mm, 1.0,
        ],
    }
}

/// Axis-aligned box triangle soup (12 triangles), same winding convention as
/// `slicer_core::algos::overhang_annotation`'s own `flat_cube_mesh` fixture
/// and `mesh_cross_section`'s tests: bottom CW-from-above, top
/// CCW-from-above.
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

/// Two 10x10x1mm boxes stacked in Z: the lower box spans x:[0,10], the upper
/// box spans x:[5,15] — laterally offset by 5mm so the upper layer's
/// footprint (at z=1.5) is NOT fully supported by the lower layer's footprint
/// (at z=0.5), producing a real overhang region for `annotate_overhangs`.
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

/// Same geometry as [`overhang_ramp_mesh`] but with `transform` applied at
/// the `ObjectMesh` level instead of the identity transform — used by the
/// non-identity-transform regression test to prove
/// `commit_overhang_annotation_builtin` applies `object.transform` before
/// cross-sectioning (packet 106 fix-iteration finding 1).
fn overhang_ramp_mesh_with_transform(transform: Transform3d) -> MeshIR {
    let (mut vertices, mut indices) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 1.0));
    let (v2, i2) = box_triangles(vertices.len() as u32, (5.0, 0.0, 1.0), (15.0, 10.0, 2.0));
    vertices.extend(v2);
    indices.extend(i2);

    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("ramp"),
            mesh: IndexedTriangleSet { vertices, indices },
            transform,
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

/// Stub runner that returns a fixed `LayerPlanIR` for
/// `PrePass::LayerPlanning` and a stage-order-violating `PrepassRunnerError`
/// otherwise is never exercised (the plan only ever contains the one stage
/// this test needs).
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

// ============================================================================
// AC-6 (positive): stage-order trace + non-empty overhang data
// ============================================================================

/// Minimal `PipelineInstrumentation` that records `(on_stage_start, stage_id)`
/// / `(on_stage_end, stage_id)` events in call order; every other hook is a
/// no-op (this test only asserts prepass stage ordering).
#[derive(Default)]
struct RecordingInstrumentation {
    events: Mutex<Vec<(&'static str, String)>>,
}

impl PipelineInstrumentation for RecordingInstrumentation {
    fn on_phase_start(&self, _phase: Phase) {}
    fn on_phase_end(&self, _phase: Phase) {}
    fn on_stage_start(&self, stage: &StageId, _layer: Option<u32>) {
        self.events
            .lock()
            .unwrap()
            .push(("start", stage.to_string()));
    }
    fn on_stage_end(&self, stage: &StageId, _layer: Option<u32>) {
        self.events.lock().unwrap().push(("end", stage.to_string()));
    }
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

#[test]
fn overhang_annotation_runs_after_mesh_analysis_and_layer_planning() {
    let mesh = overhang_ramp_mesh();
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
        global_layers: two_global_layers(),
    };
    let instrumentation = RecordingInstrumentation::default();
    let empty_resolved: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    let default_resolved = ResolvedConfig::default();
    let empty_raw: HashMap<String, slicer_ir::ConfigValue> = HashMap::new();
    let empty_bounds = ConfigBoundsIndex::empty();
    let wasm_handles = HashMap::new();

    // Downstream builtins (RegionMapping/Slice/ShellClassification/
    // PaintSegmentation/SupportGeometry) are exercised here too, since this is
    // the real `execute_prepass_with_builtins*` entry point; this test only
    // asserts on state committed up through `PrePass::OverhangAnnotation`; a
    // later builtin's `Err` (if any, e.g. from the intentionally-tiny fixture
    // not satisfying some downstream-only invariant) does not unwind earlier
    // blackboard commits, so those assertions remain valid regardless of the
    // overall `Result`.
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

    // (a) Stage-order trace: PrePass::OverhangAnnotation strictly after both
    // PrePass::MeshAnalysis and PrePass::LayerPlanning.
    let events = instrumentation.events.lock().unwrap();
    let start_index = |stage: &str| {
        events
            .iter()
            .position(|(kind, s)| *kind == "start" && s == stage)
    };
    let mesh_analysis_start =
        start_index("PrePass::MeshAnalysis").expect("PrePass::MeshAnalysis must have started");
    let layer_planning_start =
        start_index("PrePass::LayerPlanning").expect("PrePass::LayerPlanning must have started");
    let overhang_start = start_index("PrePass::OverhangAnnotation")
        .expect("PrePass::OverhangAnnotation must have started");

    assert!(
        overhang_start > mesh_analysis_start,
        "PrePass::OverhangAnnotation ({overhang_start}) must start strictly after \
         PrePass::MeshAnalysis ({mesh_analysis_start}); trace: {events:?}"
    );
    assert!(
        overhang_start > layer_planning_start,
        "PrePass::OverhangAnnotation ({overhang_start}) must start strictly after \
         PrePass::LayerPlanning ({layer_planning_start}); trace: {events:?}"
    );

    // (b) Non-empty overhang quartile-band data for at least one layer index.
    let surface_classification = blackboard
        .surface_classification()
        .expect("SurfaceClassificationIR must be committed by PrePass::MeshAnalysis");
    assert!(
        !surface_classification.overhang_quartile_polygons.is_empty(),
        "expected at least one global layer index with overhang quartile bands, got: {:?}",
        surface_classification.overhang_quartile_polygons
    );
}

/// `two_global_layers()` shifted by `z_offset_mm` in Z — the global-layer Z
/// heights a builtin producer sees are always in the same (global) space
/// regardless of any individual object's local-space transform; this fixture
/// pairs with a mesh whose `ObjectMesh::transform` translates the mesh's
/// local-space geometry into that same global-Z range.
fn two_global_layers_shifted(z_offset_mm: f32) -> Vec<GlobalLayer> {
    vec![
        GlobalLayer {
            index: 0,
            z: 0.5 + z_offset_mm,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        },
        GlobalLayer {
            index: 1,
            z: 1.5 + z_offset_mm,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        },
    ]
}

/// Regression test (packet 106 fix-iteration finding 1): the ramp/overhang
/// fixture's mesh is built in LOCAL space at z:[0,2] (same triangle soup as
/// [`overhang_ramp_mesh`]) but carries a non-identity `ObjectMesh::transform`
/// that translates it by `+5mm` in Z into GLOBAL space z:[5,7]. The global
/// layer Z heights (5.5, 6.5) are declared in global space to match.
///
/// `commit_overhang_annotation_builtin`
/// (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`)
/// must apply `object.transform` before calling `annotate_overhangs` /
/// `cross_section_at_z` — per `cross_section_at_z`'s documented contract
/// (`crates/slicer-core/src/algos/mesh_cross_section.rs`: "Callers slicing a
/// `MeshIR` object should pass `object_mesh.mesh` (applying any needed
/// transform beforehand)"). If the producer instead passes the untransformed
/// LOCAL-space mesh against these GLOBAL-space layer Zs (5.5, 6.5), the
/// cross-sections at those heights fall entirely outside the local mesh's
/// z:[0,2] extent and `overhang_quartile_polygons` comes back empty — this is
/// exactly the bug this test catches (fails on the pre-fix code, passes once
/// the transform is applied).
#[test]
fn overhang_annotation_applies_object_transform_before_cross_sectioning() {
    let z_offset_mm = 5.0_f64;
    let transform = z_translation_transform(z_offset_mm);
    let mesh = overhang_ramp_mesh_with_transform(transform);
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
        global_layers: two_global_layers_shifted(z_offset_mm as f32),
    };
    let instrumentation = RecordingInstrumentation::default();
    let empty_resolved: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    let default_resolved = ResolvedConfig::default();
    let empty_raw: HashMap<String, slicer_ir::ConfigValue> = HashMap::new();
    let empty_bounds = ConfigBoundsIndex::empty();
    let wasm_handles = HashMap::new();

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

    let surface_classification = blackboard
        .surface_classification()
        .expect("SurfaceClassificationIR must be committed by PrePass::MeshAnalysis");
    assert!(
        !surface_classification.overhang_quartile_polygons.is_empty(),
        "expected at least one global layer index with overhang quartile bands \
         after applying the object's +{z_offset_mm}mm Z transform, got: {:?} \
         (this is empty when the producer wrongly cross-sections the \
         untransformed local-space mesh against global-space layer Zs)",
        surface_classification.overhang_quartile_polygons
    );
    // Layer 0 has no previous layer and is never overhanging (module
    // doc-comment "Empty-layer semantics"); the only layer that can carry
    // overhang data here is global layer index 1.
    assert!(
        surface_classification
            .overhang_quartile_polygons
            .contains_key(&1u32),
        "expected overhang quartile bands at the shifted global layer index 1 \
         (z=6.5mm, corresponding to local z=1.5mm through the +{z_offset_mm}mm \
         transform), got keys: {:?}",
        surface_classification
            .overhang_quartile_polygons
            .keys()
            .collect::<Vec<_>>()
    );
}

/// Same ramp geometry as [`overhang_ramp_mesh`], duplicated as TWO separate
/// `ObjectMesh` entries: object "ramp-a" at y:[0,10] and object "ramp-b" at
/// y:[20,30] (built directly in world coordinates, identity transforms).
/// Both objects produce overhang at global layer index 1.
fn two_object_overhang_mesh() -> MeshIR {
    let (mut va, mut ia) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 1.0));
    let (v2, i2) = box_triangles(va.len() as u32, (5.0, 0.0, 1.0), (15.0, 10.0, 2.0));
    va.extend(v2);
    ia.extend(i2);

    let (mut vb, mut ib) = box_triangles(0, (0.0, 20.0, 0.0), (10.0, 30.0, 1.0));
    let (v4, i4) = box_triangles(vb.len() as u32, (5.0, 20.0, 1.0), (15.0, 30.0, 2.0));
    vb.extend(v4);
    ib.extend(i4);

    MeshIR {
        objects: vec![
            ObjectMesh {
                id: String::from("ramp-a"),
                mesh: IndexedTriangleSet {
                    vertices: va,
                    indices: ia,
                },
                transform: identity_transform(),
                ..Default::default()
            },
            ObjectMesh {
                id: String::from("ramp-b"),
                mesh: IndexedTriangleSet {
                    vertices: vb,
                    indices: ib,
                },
                transform: identity_transform(),
                ..Default::default()
            },
        ],
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

/// Regression test (packet 106 audit finding): the multi-object merge in
/// `commit_overhang_annotation_builtin` must aggregate per-object results
/// **by quartile** — at most one `QuartileBand` per quartile per layer, with
/// all objects' polygons concatenated into that band — preserving design.md's
/// locked assumption ("inner Vec carries one `QuartileBand` per quartile").
/// The pre-fix code concatenated whole per-object band *lists*, so N objects
/// produced up to 4×N entries per layer and any consumer doing
/// `bands.iter().find(|b| b.quartile == k)` silently dropped every object but
/// the first.
#[test]
fn overhang_annotation_merges_multi_object_bands_by_quartile() {
    let mesh = two_object_overhang_mesh();
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
        global_layers: two_global_layers(),
    };
    let instrumentation = RecordingInstrumentation::default();
    let empty_resolved: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    let default_resolved = ResolvedConfig::default();
    let empty_raw: HashMap<String, slicer_ir::ConfigValue> = HashMap::new();
    let empty_bounds = ConfigBoundsIndex::empty();
    let wasm_handles = HashMap::new();

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

    let surface_classification = blackboard
        .surface_classification()
        .expect("SurfaceClassificationIR must be committed by PrePass::MeshAnalysis");
    let bands = surface_classification
        .overhang_quartile_polygons
        .get(&1u32)
        .expect("both ramp objects overhang at global layer index 1");

    assert!(
        bands.len() <= 4,
        "at most one QuartileBand per quartile per layer regardless of object \
         count; got {} bands: quartiles {:?}",
        bands.len(),
        bands.iter().map(|b| b.quartile).collect::<Vec<_>>()
    );
    let quartiles: Vec<u8> = bands.iter().map(|b| b.quartile).collect();
    let mut deduped = quartiles.clone();
    deduped.dedup();
    assert_eq!(
        quartiles, deduped,
        "quartiles must be unique (and sorted) within a layer's band list"
    );
    assert!(
        quartiles.windows(2).all(|w| w[0] < w[1]),
        "bands must be sorted by quartile, got {quartiles:?}"
    );
    // Both objects contribute identical (Y-translated) overhang geometry, so
    // every present band must carry polygons from BOTH objects — ≥ 2 polygons
    // per band. The pre-fix concatenation instead produced duplicate
    // single-polygon bands per quartile.
    for band in bands {
        assert!(
            band.polygons.len() >= 2,
            "band {} must contain polygons merged from both objects, got {} \
             polygon(s)",
            band.quartile,
            band.polygons.len()
        );
    }
}

// ============================================================================
// AC-N2 (negative, violation_case)
// ============================================================================

/// Stub runner used only by the violation-case test: `PrePass::
/// OverhangAnnotation` is never expected to actually dispatch a module here
/// because `ensure_stage_prerequisites` must reject the stage before any
/// module runs.
struct UnreachableRunner;

impl PrepassStageRunner for UnreachableRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        panic!("stage {stage_id} must be rejected by ensure_stage_prerequisites before dispatch");
    }
}

/// AC-N2 (`violation_case`): a plan that places `PrePass::OverhangAnnotation`
/// BEFORE `PrePass::LayerPlanning` must be rejected deterministically with
/// `PrepassExecutionError::MissingRequiredPrepass { slot: LayerPlan, .. }`
/// naming the violated dependency, before any module dispatch occurs.
#[test]
fn overhang_annotation_before_layer_planning_violation_case_is_rejected() {
    let mesh = overhang_ramp_mesh();
    let mesh_arc = Arc::new(mesh);
    let mut blackboard = Blackboard::new(Arc::clone(&mesh_arc), 0);

    // Pre-seed SurfaceClassificationIR so the failure is unambiguously about
    // the LayerPlan ordering violation, not the (also-required)
    // SurfaceClassification prerequisite.
    blackboard
        .commit_surface_classification(Arc::new(slicer_ir::SurfaceClassificationIR::default()))
        .expect("pre-seeding SurfaceClassificationIR must succeed");

    // Deliberately mis-ordered: PrePass::OverhangAnnotation appears before
    // PrePass::LayerPlanning in the stage list.
    let plan = ExecutionPlan {
        prepass_stages: vec![
            CompiledStage {
                stage_id: String::from("PrePass::OverhangAnnotation"),
                modules: vec![compiled_stub_module(
                    "PrePass::OverhangAnnotation",
                    "com.test.overhang-annotation-stub",
                )],
            },
            CompiledStage {
                stage_id: String::from("PrePass::LayerPlanning"),
                modules: vec![compiled_stub_module(
                    "PrePass::LayerPlanning",
                    "com.test.layer-planning-stub",
                )],
            },
        ],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    };

    let runner = UnreachableRunner;
    let result = execute_prepass(&plan, &mut blackboard, &runner, &Default::default());

    match result {
        Err(PrepassExecutionError::MissingRequiredPrepass { stage_id, slot }) => {
            assert_eq!(stage_id, "PrePass::OverhangAnnotation");
            assert_eq!(
                slot,
                BlackboardPrepassSlot::LayerPlan,
                "violation must name the missing LayerPlan dependency"
            );
        }
        other => panic!(
            "expected PrepassExecutionError::MissingRequiredPrepass {{ slot: LayerPlan, .. }} \
             for the mis-ordered plan, got: {other:?}"
        ),
    }
}
