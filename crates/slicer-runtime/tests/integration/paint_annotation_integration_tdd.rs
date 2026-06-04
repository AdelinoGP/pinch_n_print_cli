//! TASK-108: host-built-in paint-annotation wiring into the per-layer path.
//!
//! Verifies that `execute_slice_postprocess_paint_annotation` is invoked
//! during the real per-layer pipeline path after `Layer::Slice` stages the
//! `SliceIR`, that non-fatal fallback warnings reach the `LayerProgressSink`
//! via `paint_annotation_warning_to_progress_event` with `fatal=false` and
//! stable code 504, that repeated runs produce byte-identical sinks, and
//! that missing paint-region data for a required semantic surfaces as a
//! typed fatal `LayerExecutionError::PaintAnnotation`.

use crate::common::seed::seed_slice_ir;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rstar::RTree;
use slicer_core::paint_region::{PaintRegionRTreeEntry, PaintRegionRTreeIndex};
use slicer_ir::slice_ir::BoundingBox2;
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GCodeCommand, GlobalLayer, IndexedTriangleSet,
    LayerPaintMap, LayerStageCommitData, MeshIR, ObjectConfig, ObjectMesh, PaintRegionIR,
    PaintSemantic, PaintValue, Point2, Point3, Polygon, ResolvedConfig, SemanticRegion, StageId,
    Transform3d,
};
use slicer_runtime::progress_events::{EventReason, ProgressEvent, ProgressPhase};
use slicer_runtime::{
    execute_per_layer_with_events, execute_prepass_with_builtins, Blackboard, CompiledModuleLive,
    ExecutionPlan, FinalizationStageInput, LayerExecutionError, LayerProgressSink, LayerStageError,
    LayerStageInput, LayerStageRunner, PostpassStageInput, PrepassStageInput,
    SlicePostProcessPaintAnnotationError,
};

fn unit_tetra() -> IndexedTriangleSet {
    // 10mm tetra: slice at z=0.1 produces a triangle with vertices near
    // (0,0), (9.9, 0), (0, 9.9) in mm.
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
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
    ResolvedConfig::default()
}

fn tetra_mesh_ir(object_id: &str) -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: unit_tetra(),
            transform: identity_transform(),
            config: ObjectConfig {
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
                x: 10.0,
                y: 10.0,
                z: 10.0,
            },
        },
        ..Default::default()
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
            // triangle's hypotenuse endpoints â€” within the annotator's
            // numerical-edge epsilon â€” forcing deterministic fallback.
            polygons: vec![polygon(vec![(0.0, 0.0), (9.8999, 0.0), (0.0, 9.8999)])],
            value: PaintValue::ToolIndex(3),
            paint_order: 0,
            aabb: None,
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
        per_layer,
        ..Default::default()
    }
}

/// Paint-region map that declares a Material semantic on a *different*
/// layer than the one being executed, so the required-semantic check
/// triggers `MissingPaintRegionSemantic` for layer 0.
fn material_only_on_other_layer() -> PaintRegionIR {
    ambiguous_triangle_paint_regions(7)
}

/// Build a single-layer `SliceIR` vec whose contour vertex at (4.95, 4.95)
/// lies ~0.7 integer-units (~70 nm) beyond the paint triangle's hypotenuse
/// `x + y = 9.8999` (in mm space). The paint annotator's `EPSILON_UNITS = 1`
/// numerical-edge tolerance â€” an AABB query with Â±1-unit eps around the
/// point â€” catches this edge segment, and `point_in_paint_region` returns
/// `Ok(None)` because the contour vertex is outside the triangle (4.95 +
/// 4.95 = 9.9 > 9.8999). Annotator falls through to
/// `NumericalEdgeAmbiguity`, emitting the fatal-false code-504 warning
/// that the test pipelines expect to see on the sink.
fn ambiguity_inducing_slice_ir(layer_index: u32, object_id: &str) -> Vec<slicer_ir::SliceIR> {
    vec![slicer_ir::SliceIR {
        schema_version: slicer_ir::SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer_index,
        z: 0.1,
        regions: vec![slicer_ir::SlicedRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            polygons: vec![polygon(vec![
                (4.95, 4.95),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
            ])],
            ..Default::default()
        }],
    }]
}

/// Build an `Arc<PaintRegionRTreeIndex>` companion for a `PaintRegionIR`,
/// computing per-region AABBs where `aabb` is `None`.
fn build_paint_region_rtree_index(ir: &PaintRegionIR) -> Arc<PaintRegionRTreeIndex> {
    let mut trees: HashMap<u32, HashMap<PaintSemantic, RTree<PaintRegionRTreeEntry>>> =
        HashMap::new();
    for (&layer_index, layer_map) in &ir.per_layer {
        let mut semantic_map: HashMap<PaintSemantic, RTree<PaintRegionRTreeEntry>> = HashMap::new();
        for (semantic, regions) in &layer_map.semantic_regions {
            let entries: Vec<PaintRegionRTreeEntry> = regions
                .iter()
                .enumerate()
                .map(|(region_index, region)| {
                    let aabb = region
                        .aabb
                        .unwrap_or_else(|| aabb_from_expolygons(&region.polygons));
                    PaintRegionRTreeEntry {
                        min_x: aabb.min.x as f64,
                        min_y: aabb.min.y as f64,
                        max_x: aabb.max.x as f64,
                        max_y: aabb.max.y as f64,
                        region_index,
                    }
                })
                .collect();
            let tree = if entries.is_empty() {
                RTree::new()
            } else {
                RTree::bulk_load(entries)
            };
            semantic_map.insert(semantic.clone(), tree);
        }
        trees.insert(layer_index, semantic_map);
    }
    Arc::new(PaintRegionRTreeIndex { trees })
}

fn expoly_vertex_aabb(polygons: &[ExPolygon]) -> BoundingBox2 {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for expoly in polygons {
        for pt in &expoly.contour.points {
            min_x = min_x.min(pt.x);
            min_y = min_y.min(pt.y);
            max_x = max_x.max(pt.x);
            max_y = max_y.max(pt.y);
        }
        for hole in &expoly.holes {
            for pt in &hole.points {
                min_x = min_x.min(pt.x);
                min_y = min_y.min(pt.y);
                max_x = max_x.max(pt.x);
                max_y = max_y.max(pt.y);
            }
        }
    }
    BoundingBox2 {
        min: Point2 { x: min_x, y: min_y },
        max: Point2 { x: max_x, y: max_y },
    }
}

fn aabb_from_expolygons(polygons: &[ExPolygon]) -> BoundingBox2 {
    expoly_vertex_aabb(polygons)
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
        _m: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<LayerStageCommitData, LayerStageError> {
        Ok(LayerStageCommitData::default())
    }
}

#[test]
fn paint_annotation_is_invoked_on_real_per_layer_path_and_warnings_reach_sink() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let layer = layer_at(0, 0.1, "obj-a");
    let plan = plan_empty(vec![layer]);
    let mut bb = Blackboard::new(Arc::clone(&mesh), plan.global_layers.len());
    bb.commit_slice_ir(Arc::new(ambiguity_inducing_slice_ir(0, "obj-a")))
        .expect("commit slice_ir");
    let ir = Arc::new(ambiguous_triangle_paint_regions(0));
    let rtree = build_paint_region_rtree_index(&ir);
    bb.commit_paint_regions(ir, rtree).unwrap();

    let sink = VecSink(Mutex::new(Vec::new()));
    let (layer_irs, _layer_audits) =
        execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink, &Default::default())
            .expect("ok");
    assert_eq!(layer_irs.len(), 1);

    let events = sink.0.lock().unwrap().clone();
    assert!(
        events.iter().any(|e| {
            e.error.as_ref().map_or(false, |err| {
                err.code == 504 && matches!(err.reason, Some(EventReason::NumericalEdgeAmbiguity))
            })
        }),
        "expected NumericalEdgeAmbiguity (code 504), got {events:?}"
    );
    let event = events
        .iter()
        .find(|e| {
            e.error.as_ref().map_or(false, |err| {
                err.code == 504 && matches!(err.reason, Some(EventReason::NumericalEdgeAmbiguity))
            })
        })
        .expect("found NumericalEdgeAmbiguity (code 504) event");
    let err = event
        .error
        .as_ref()
        .expect("module_error must carry error payload");
    assert_eq!(err.code, 504);
    assert!(!err.fatal, "paint-fallback warnings must be non-fatal");
    assert_eq!(event.phase, Some(ProgressPhase::PerLayer));
    assert_eq!(event.stage.as_deref(), Some("Layer::PaintRegionAnnotation"));
}

#[test]
fn paint_annotation_degraded_fallback_is_deterministic_across_repeated_runs() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan1 = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let plan2 = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let mut bb1 = Blackboard::new(Arc::clone(&mesh), 1);
    let mut bb2 = Blackboard::new(Arc::clone(&mesh), 1);
    bb1.commit_slice_ir(Arc::new(ambiguity_inducing_slice_ir(0, "obj-a")))
        .expect("commit slice_ir bb1");
    bb2.commit_slice_ir(Arc::new(ambiguity_inducing_slice_ir(0, "obj-a")))
        .expect("commit slice_ir bb2");
    let ir1 = Arc::new(ambiguous_triangle_paint_regions(0));
    let rt1 = build_paint_region_rtree_index(&ir1);
    bb1.commit_paint_regions(ir1, rt1).unwrap();
    let ir2 = Arc::new(ambiguous_triangle_paint_regions(0));
    let rt2 = build_paint_region_rtree_index(&ir2);
    bb2.commit_paint_regions(ir2, rt2).unwrap();

    let sink_a = VecSink(Mutex::new(Vec::new()));
    let sink_b = VecSink(Mutex::new(Vec::new()));
    let (_layer_irs_a, _audits_a) =
        execute_per_layer_with_events(&plan1, &bb1, &NoopRunner, &sink_a, &Default::default())
            .unwrap();
    let (_layer_irs_b, _audits_b) =
        execute_per_layer_with_events(&plan2, &bb2, &NoopRunner, &sink_b, &Default::default())
            .unwrap();

    let a = sink_a.0.lock().unwrap().clone();
    let b = sink_b.0.lock().unwrap().clone();
    assert_eq!(
        a, b,
        "paint-fallback events must be byte-identical across runs"
    );
    assert!(
        a.iter().any(|e| {
            e.error.as_ref().map_or(false, |err| {
                err.code == 504 && matches!(err.reason, Some(EventReason::NumericalEdgeAmbiguity))
            })
        }),
        "expected NumericalEdgeAmbiguity (code 504), got {a:?}"
    );
}

#[test]
fn paint_annotation_missing_required_semantic_surfaces_typed_fatal_error() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let mut bb = Blackboard::new(Arc::clone(&mesh), plan.global_layers.len());
    seed_slice_ir(&mut bb, &plan);
    let ir = Arc::new(material_only_on_other_layer());
    let rtree = build_paint_region_rtree_index(&ir);
    bb.commit_paint_regions(ir, rtree).unwrap();

    let sink = VecSink(Mutex::new(Vec::new()));
    let err = execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink, &Default::default())
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
    let mut bb = Blackboard::new(mesh, plan.global_layers.len());
    seed_slice_ir(&mut bb, &plan);

    let sink = VecSink(Mutex::new(Vec::new()));
    execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink, &Default::default()).expect("ok");
    assert!(sink.0.lock().unwrap().is_empty());
}

// â”€â”€ Runtime sink wiring (TASK-108 production-path closure) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use slicer_ir::{GCodeIR, LayerCollectionIR, PrintMetadata};
use slicer_runtime::pipeline::{run_pipeline_with_events, PipelineConfig, PipelineStageRunners};
use slicer_runtime::progress_events::{
    JsonLinesEmitter, ProgressEventEmitter, RuntimeProgressSink, SliceEventCollector,
};
use slicer_runtime::{
    FinalizationError, FinalizationOutput, FinalizationStageRunner, GCodeEmitError, GCodeEmitter,
    GCodeSerializer, PostpassError, PostpassOutput, PostpassStageRunner, PrepassRunnerError,
    PrepassStageOutput, PrepassStageRunner,
};

struct NoopPrepassRunner;
impl PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _s: &StageId,
        _m: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        Ok(PrepassStageOutput::None)
    }
}
struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _s: &StageId,
        _m: &CompiledModuleLive<'_>,
        _input: FinalizationStageInput<'_>,
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
        _m: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        _commands: &mut Vec<GCodeCommand>,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }
    fn run_text_postprocess(
        &self,
        _s: &StageId,
        _m: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}
struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(&self, _l: &[LayerCollectionIR]) -> Result<GCodeIR, GCodeEmitError> {
        Ok(GCodeIR {
            metadata: PrintMetadata {
                slicer_version: "test".into(),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}
struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _g: &GCodeIR) -> Result<String, GCodeEmitError> {
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

fn capture_emitter() -> (
    Arc<JsonLinesEmitter<Vec<u8>>>,
    Arc<dyn ProgressEventEmitter>,
) {
    let e = Arc::new(JsonLinesEmitter::new(Vec::<u8>::new()));
    let as_emitter: Arc<dyn ProgressEventEmitter> = e.clone();
    (e, as_emitter)
}

#[test]
fn runtime_sink_forwards_paint_warning_to_both_jsonl_emitter_and_slice_event_collector() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
    let mut bb = Blackboard::new(Arc::clone(&mesh), plan.global_layers.len());
    bb.commit_slice_ir(Arc::new(ambiguity_inducing_slice_ir(0, "obj-a")))
        .expect("commit slice_ir");
    let ir = Arc::new(ambiguous_triangle_paint_regions(0));
    let rtree = build_paint_region_rtree_index(&ir);
    bb.commit_paint_regions(ir, rtree).unwrap();

    let (raw_emitter, emitter) = capture_emitter();
    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    let sink = RuntimeProgressSink::new(emitter, Arc::clone(&collector));

    execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink, &Default::default()).expect("ok");

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
    assert!(lines
        .iter()
        .all(|l| l.contains("\"event\":\"module_error\"")));
    assert!(lines
        .iter()
        .all(|l| l.contains("\"stage\":\"Layer::PaintRegionAnnotation\"")));
    assert!(
        lines
            .iter()
            .all(|l| l.contains("\"reason\":\"numerical-edge-ambiguity\"")),
        "all JSONL events must carry reason=numerical-edge-ambiguity"
    );
}

#[test]
fn runtime_sink_jsonl_output_is_byte_identical_across_repeated_runs() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));

    let run_once = || -> Vec<u8> {
        let plan = plan_empty(vec![layer_at(0, 0.1, "obj-a")]);
        let mut bb = Blackboard::new(Arc::clone(&mesh), 1);
        bb.commit_slice_ir(Arc::new(ambiguity_inducing_slice_ir(0, "obj-a")))
            .expect("commit slice_ir");
        let ir = Arc::new(ambiguous_triangle_paint_regions(0));
        let rtree = build_paint_region_rtree_index(&ir);
        bb.commit_paint_regions(ir, rtree).unwrap();
        let (raw, emitter) = capture_emitter();
        let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
        let sink = RuntimeProgressSink::new(emitter, collector);
        execute_per_layer_with_events(&plan, &bb, &NoopRunner, &sink, &Default::default()).unwrap();
        let bytes = raw.writer.lock().unwrap().clone();
        bytes
    };

    let a = run_once();
    let b = run_once();
    let c = run_once();
    let text_a = String::from_utf8(a.clone()).expect("valid UTF-8");
    assert!(
        text_a.contains("\"code\":504")
            && text_a.contains("\"reason\":\"numerical-edge-ambiguity\""),
        "JSONL output must contain NumericalEdgeAmbiguity (code 504) with typed reason: {text_a}"
    );
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
        resolved_configs: std::sync::Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: std::sync::Arc::new(ResolvedConfig::default()),
        bounds: std::sync::Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
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
    // Guards the TASK-108 production-path closure: the slicer-runtime
    // library's run.rs entry point (after pnp-cli-unification) must invoke a
    // pipeline entry point that threads a `LayerProgressSink` (not the
    // sink-less `run_pipeline`, which dropped paint warnings into
    // `NoopLayerProgressSink`). If this regresses, paint-annotation
    // degraded-success events stop reaching the documented transport. Either
    // `run_pipeline_with_raw_config` (no-report path) or
    // `run_pipeline_with_instrumentation` (--report path) is acceptable;
    // both forward the supplied sink.
    let run_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/run.rs"))
        .expect("read run.rs");
    assert!(
        run_src.contains("run_pipeline_with_raw_config")
            || run_src.contains("run_pipeline_with_events")
            || run_src.contains("run_pipeline_with_instrumentation"),
        "run.rs must call a sink-aware pipeline entry point"
    );
    assert!(
        run_src.contains("RuntimeProgressSink::new"),
        "run.rs must construct a RuntimeProgressSink for the real transport"
    );
    assert!(
        run_src.contains("JsonLinesEmitter::new"),
        "run.rs must wire the JSONL emitter as the documented transport"
    );
}

#[test]
fn empty_plan_with_no_layers_does_not_trigger_prepass_error() {
    let mesh = Arc::new(tetra_mesh_ir("obj-a"));
    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let mut bb = Blackboard::new(Arc::clone(&mesh), 0);

    let result =
        execute_prepass_with_builtins(&plan, &mut bb, &NoopPrepassRunner, &Default::default());
    assert!(
        result.is_ok(),
        "prepass must complete without error when no layer plan exists"
    );
}
