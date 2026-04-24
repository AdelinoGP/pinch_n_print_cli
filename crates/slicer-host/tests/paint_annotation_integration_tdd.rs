//! TASK-108: host-built-in paint-annotation wiring into the per-layer path.
//!
//! Verifies that `execute_slice_postprocess_paint_annotation` is invoked
//! during the real per-layer pipeline path after `Layer::Slice` stages the
//! `SliceIR`, that non-fatal fallback warnings reach the `LayerProgressSink`
//! via `paint_annotation_warning_to_progress_event` with `fatal=false` and
//! stable code 504, that repeated runs produce byte-identical sinks, and
//! that missing paint-region data for a required semantic surfaces as a
//! typed fatal `LayerExecutionError::PaintAnnotation`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slicer_host::{
    execute_per_layer_with_events, Blackboard, CompiledModule, ExecutionPlan, LayerArena,
    LayerExecutionError, LayerProgressSink, LayerStageError, LayerStageOutput, LayerStageRunner,
    SlicePostProcessPaintAnnotationError,
};
use slicer_host::progress_events::{ProgressEvent, ProgressPhase};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, InfillType,
    LayerPaintMap, MeshIR, ObjectConfig, ObjectMesh, PaintRegionIR, PaintSemantic, PaintValue,
    Point2, Point3, Polygon, ResolvedConfig, SemVer, SemanticRegion, StageId, SupportType,
    Transform3d, WallGenerator,
};

fn unit_tetra() -> IndexedTriangleSet {
    // 10mm tetra: slice at z=0.1 produces a triangle with vertices near
    // (0,0), (9.9, 0), (0, 9.9) in mm.
    IndexedTriangleSet {
        vertices: vec![
            Point3 { x: 0.0, y: 0.0, z: 0.0 },
            Point3 { x: 10.0, y: 0.0, z: 0.0 },
            Point3 { x: 0.0, y: 10.0, z: 0.0 },
            Point3 { x: 0.0, y: 0.0, z: 10.0 },
        ],
        indices: vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
    }
}

fn identity_transform() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

fn default_resolved() -> ResolvedConfig {
    ResolvedConfig {
        layer_height: 0.2,
        first_layer_height: 0.2,
        line_width: 0.4,
        first_layer_line_width: 0.4,
        wall_count: 2,
        outer_wall_speed: 50.0,
        inner_wall_speed: 50.0,
        wall_generator: WallGenerator::Classic,
        arachne_min_feature_size: None,
        infill_type: InfillType::Grid,
        infill_density: 0.2,
        infill_angle: 45.0,
        infill_speed: 50.0,
        solid_infill_speed: 50.0,
        top_shell_layers: 3,
        bottom_shell_layers: 3,
        support_enabled: false,
        support_type: SupportType::Traditional,
        support_overhang_angle: 45.0,
        nonplanar_max_angle_deg: None,
        nonplanar_shell_count: None,
        nonplanar_amplitude: None,
        smoothificator_target_height: None,
        smoothificator_adaptive: None,
        extensions: HashMap::new(),
    }
}

fn tetra_mesh_ir(object_id: &str) -> MeshIR {
    MeshIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: unit_tetra(),
            transform: identity_transform(),
            config: ObjectConfig { data: HashMap::new() },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 10.0, y: 10.0, z: 10.0 },
        },
    }
}

fn layer_at(index: u32, z: f32, object_id: &str) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            resolved_config: default_resolved(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        }],
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

fn polygon(points_mm: Vec<(f32, f32)>) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: points_mm
                .into_iter()
                .map(|(x, y)| Point2::from_mm(x, y))
                .collect(),
        },
        holes: Vec::new(),
    }
}

fn plan_empty(layers: Vec<GlobalLayer>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(layers),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    }
}

/// A paint_regions map with one triangular Material region on `layer_index`.
/// Point (10.0, 0.0001) on a square contour will fall just outside the
/// triangle (0,0)-(10,0)-(0,10), triggering the numerical-edge-ambiguity
/// fallback on that specific point.
fn ambiguous_triangle_paint_regions(layer_index: u32) -> PaintRegionIR {
    let mut semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>> = HashMap::new();
    semantic_regions.insert(
        PaintSemantic::Material,
        vec![SemanticRegion {
            object_id: "obj-a".to_string(),
            // Triangle slightly smaller than the z=0.1 slice so its vertices
            // at ~(9.9,0) and ~(0,9.9) land ~0.0001 mm outside the paint
            // triangle's hypotenuse endpoints — within the annotator's
            // numerical-edge epsilon — forcing deterministic fallback.
            polygons: vec![polygon(vec![(0.0, 0.0), (9.8999, 0.0), (0.0, 9.8999)])],
            value: PaintValue::ToolIndex(3),
            paint_order: 0,
        }],
    );
    let mut per_layer = HashMap::new();
    per_layer.insert(
        layer_index,
        LayerPaintMap {
            global_layer_index: layer_index,
            semantic_regions,
        },
    );
    PaintRegionIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        per_layer,
    }
}

/// Paint-region map that declares a Material semantic on a *different*
/// layer than the one being executed, so the required-semantic check
/// triggers `MissingPaintRegionSemantic` for layer 0.
fn material_only_on_other_layer() -> PaintRegionIR {
    ambiguous_triangle_paint_regions(7)
}

struct VecSink(Mutex<Vec<ProgressEvent>>);

impl LayerProgressSink for VecSink {
    fn record(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct NoopRunner;

impl LayerStageRunner for NoopRunner {
    fn run_stage(
        &self,
        _s: &StageId,
        _l: &GlobalLayer,
        _m: &CompiledModule,
        _b: &Blackboard,
        _a: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        Ok((LayerStageOutput::Success, Vec::new(), Vec::new()))
    }
}

#[test]
fn paint_annotation_is_invoked_on_real_per_layer_path_and_warnings_reach_sink() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let layer = layer_at(0, 0.1, "obj-a");
    let plan = plan_empty(vec![layer]);
    let mut bb = Blackboard::new(Arc::clone(&mesh), plan.global_layers.len());
    bb.commit_paint_regions(Arc::new(ambiguous_triangle_paint_regions(0)))
        .unwrap();

    let sink = VecSink(Mutex::new(Vec::new()));
    let (layer_irs, _layer_audits) = execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink).expect("ok");
    assert_eq!(layer_irs.len(), 1);

    let events = sink.0.lock().unwrap().clone();
    assert!(
        !events.is_empty(),
        "paint annotator must produce at least one progress event on the real per-layer path"
    );
    let event = &events[0];
    let err = event.error.as_ref().expect("module_error must carry error payload");
    assert_eq!(err.code, 504);
    assert!(!err.fatal, "paint-fallback warnings must be non-fatal");
    assert_eq!(event.phase, Some(ProgressPhase::PerLayer));
    assert_eq!(event.stage.as_deref(), Some("Layer::SlicePostProcess"));
}

#[test]
fn paint_annotation_degraded_fallback_is_deterministic_across_repeated_runs() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan1 = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let plan2 = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let mut bb1 = Blackboard::new(Arc::clone(&mesh), 1);
    let mut bb2 = Blackboard::new(Arc::clone(&mesh), 1);
    bb1.commit_paint_regions(Arc::new(ambiguous_triangle_paint_regions(0))).unwrap();
    bb2.commit_paint_regions(Arc::new(ambiguous_triangle_paint_regions(0))).unwrap();

    let sink_a = VecSink(Mutex::new(Vec::new()));
    let sink_b = VecSink(Mutex::new(Vec::new()));
    let (_layer_irs_a, _audits_a) = execute_per_layer_with_events(&plan1, &bb1, &NoopRunner, &sink_a).unwrap();
    let (_layer_irs_b, _audits_b) = execute_per_layer_with_events(&plan2, &bb2, &NoopRunner, &sink_b).unwrap();

    let a = sink_a.0.lock().unwrap().clone();
    let b = sink_b.0.lock().unwrap().clone();
    assert_eq!(a, b, "paint-fallback events must be byte-identical across runs");
    assert!(!a.is_empty());
}

#[test]
fn paint_annotation_missing_required_semantic_surfaces_typed_fatal_error() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let mut bb = Blackboard::new(Arc::clone(&mesh), plan.global_layers.len());
    bb.commit_paint_regions(Arc::new(material_only_on_other_layer())).unwrap();

    let sink = VecSink(Mutex::new(Vec::new()));
    let err = execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink)
        .expect_err("layer 0 must fail because Material is required but absent");
    match err {
        LayerExecutionError::PaintAnnotation {
            layer_index,
            source:
                SlicePostProcessPaintAnnotationError::MissingPaintRegionSemantic {
                    code,
                    global_layer_index,
                    semantic,
                },
        } => {
            assert_eq!(layer_index, 0);
            assert_eq!(global_layer_index, 0);
            assert_eq!(code, 501);
            assert_eq!(semantic, PaintSemantic::Material);
        }
        other => panic!("expected typed PaintAnnotation fatal, got {other:?}"),
    }
}

#[test]
fn paint_annotation_no_op_when_no_paint_regions_committed() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let bb = Blackboard::new(mesh, plan.global_layers.len());

    let sink = VecSink(Mutex::new(Vec::new()));
    execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink).expect("ok");
    assert!(sink.0.lock().unwrap().is_empty());
}

// ── Runtime sink wiring (TASK-108 production-path closure) ─────────────

use slicer_host::pipeline::{run_pipeline_with_events, PipelineConfig, PipelineStageRunners};
use slicer_host::progress_events::{
    JsonLinesEmitter, ProgressEventEmitter, RuntimeProgressSink, SliceEventCollector,
};
use slicer_host::{
    CompiledModule as CompiledModuleAlias, FinalizationError, FinalizationOutput,
    FinalizationStageRunner, GCodeEmitter, GCodeSerializer, PostpassError, PostpassOutput,
    PostpassStageRunner, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner,
};
use slicer_ir::{GCodeIR, LayerCollectionIR, PrintMetadata};

struct NoopPrepassRunner;
impl PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _s: &StageId,
        _m: &CompiledModuleAlias,
        _b: &Blackboard,
        ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
            Ok((PrepassStageOutput::None, Vec::new()))
    }
}
struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _s: &StageId,
        _m: &CompiledModuleAlias,
        _b: &Blackboard,
        _l: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        Ok(FinalizationOutput::Success)
    }
}
struct NoopPostpassRunner;
impl PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _s: &StageId,
        _m: &CompiledModuleAlias,
        _b: &Blackboard,
        _g: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }
    fn run_text_postprocess(
        &self,
        _s: &StageId,
        _m: &CompiledModuleAlias,
        _b: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}
struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(
        &self,
        _l: &[LayerCollectionIR],
        _b: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        Ok(GCodeIR {
            schema_version: SemVer { major: 1, minor: 0, patch: 0 },
            commands: Vec::new(),
            metadata: PrintMetadata {
                slicer_version: "test".into(),
                estimated_print_time_s: 0,
                filament_used_mm: Vec::new(),
                layer_count: 0,
            },
        })
    }
}
struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _g: &GCodeIR) -> Result<String, PostpassError> {
        Ok(String::new())
    }
}

fn minimal_runners() -> PipelineStageRunners {
    PipelineStageRunners {
        prepass: Box::new(NoopPrepassRunner),
        layer: Box::new(NoopRunner),
        finalization: Box::new(NoopFinalizationRunner),
        postpass: Box::new(NoopPostpassRunner),
        emitter: Box::new(MinimalEmitter),
        serializer: Box::new(MinimalSerializer),
    }
}

fn capture_emitter() -> (Arc<JsonLinesEmitter<Vec<u8>>>, Arc<dyn ProgressEventEmitter>) {
    let e = Arc::new(JsonLinesEmitter::new(Vec::<u8>::new()));
    let as_emitter: Arc<dyn ProgressEventEmitter> = e.clone();
    (e, as_emitter)
}

#[test]
fn runtime_sink_forwards_paint_warning_to_both_jsonl_emitter_and_slice_event_collector() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let mut bb = Blackboard::new(Arc::clone(&mesh), plan.global_layers.len());
    bb.commit_paint_regions(Arc::new(ambiguous_triangle_paint_regions(0))).unwrap();

    let (raw_emitter, emitter) = capture_emitter();
    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    let sink = RuntimeProgressSink::new(emitter, Arc::clone(&collector));

    execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink).expect("ok");

    // Collector side: degraded=true and exactly one non-fatal entry.
    let c = collector.lock().unwrap();
    assert!(c.is_degraded(), "collector must observe degraded slice");
    assert!(c.non_fatal_count() >= 1);
    assert_eq!(c.fatal_count(), 0);

    // Emitter side: at least one JSONL event with code 504 + fatal=false.
    let bytes = raw_emitter.writer.lock().unwrap().clone();
    let text = String::from_utf8(bytes).expect("emitter output must be valid UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len() as u32,
        c.non_fatal_count(),
        "JSONL line count must match collector non_fatal_count"
    );
    assert!(lines.iter().all(|l| l.contains("\"code\":504")));
    assert!(lines.iter().all(|l| l.contains("\"fatal\":false")));
    assert!(lines.iter().all(|l| l.contains("\"event\":\"module_error\"")));
    assert!(lines
        .iter()
        .all(|l| l.contains("\"stage\":\"Layer::SlicePostProcess\"")));
}

#[test]
fn runtime_sink_jsonl_output_is_byte_identical_across_repeated_runs() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));

    let run_once = || -> Vec<u8> {
        let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
        let mut bb = Blackboard::new(Arc::clone(&mesh), 1);
        bb.commit_paint_regions(Arc::new(ambiguous_triangle_paint_regions(0))).unwrap();
        let (raw, emitter) = capture_emitter();
        let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
        let sink = RuntimeProgressSink::new(emitter, collector);
        execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink).unwrap();
        let bytes = raw.writer.lock().unwrap().clone();
        bytes
    };

    let a = run_once();
    let b = run_once();
    let c = run_once();
    assert!(!a.is_empty());
    assert_eq!(a, b, "JSONL bytes must be deterministic across runs");
    assert_eq!(b, c);
}

#[test]
fn run_pipeline_with_events_on_empty_plan_emits_no_spurious_events() {
    // Exercises the same production entry pattern main.rs uses: construct a
    // PipelineConfig with no paint regions, route through a real
    // `RuntimeProgressSink`, and verify neither the JSONL transport nor the
    // collector observe any paint-annotation events.
    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let (raw_emitter, emitter) = capture_emitter();
    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    let sink = RuntimeProgressSink::new(emitter, Arc::clone(&collector));

    let config = PipelineConfig {
        mesh_ir: Arc::new(tetra_mesh_ir("obj-a")),
        plan,
        runners: minimal_runners(),
    };
    run_pipeline_with_events(config, &sink).expect("empty pipeline must succeed");

    let c = collector.lock().unwrap();
    assert!(!c.is_degraded(), "no paint warnings means not degraded");
    assert_eq!(c.non_fatal_count(), 0);
    assert_eq!(c.fatal_count(), 0);
    let bytes = raw_emitter.writer.lock().unwrap().clone();
    assert!(
        bytes.is_empty(),
        "empty pipeline must not write any JSONL events (got {} bytes)",
        bytes.len()
    );
}

#[test]
fn main_production_entry_path_uses_run_pipeline_with_events() {
    // Guards the TASK-108 production-path closure: the slicer-host binary's
    // Run arm must invoke `run_pipeline_with_events` rather than the
    // sink-less `run_pipeline` (which dropped paint warnings into
    // `NoopLayerProgressSink`). If this regresses, paint-annotation
    // degraded-success events stop reaching the documented transport.
    let main_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/main.rs"
    ))
    .expect("read main.rs");
    assert!(
        main_src.contains("run_pipeline_with_events"),
        "main.rs must call run_pipeline_with_events"
    );
    assert!(
        main_src.contains("RuntimeProgressSink::new"),
        "main.rs must construct a RuntimeProgressSink for the real transport"
    );
    assert!(
        main_src.contains("JsonLinesEmitter::new"),
        "main.rs must wire the JSONL emitter as the documented transport"
    );
}
