//! Integration tests for `run_pipeline_with_instrumentation`.
//!
//! The existing `slicer_report_html_tdd.rs` exercises `Collector` directly
//! through the `PipelineInstrumentation` trait surface. These tests drive the
//! real pipeline with a `Collector` to cover the hook-wiring in:
//!
//! - `pipeline.rs::run_pipeline_with_instrumentation` (phase brackets +
//!   `record_edges` fan-out at plan freeze)
//! - `layer_executor.rs::execute_single_layer` (layer / stage / module
//!   brackets inside the rayon parallel iterator)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_host::pipeline::{
    run_pipeline_with_instrumentation, PipelineConfig, PipelineStageRunners,
};
use slicer_host::report::Collector;
use slicer_host::{
    build_wasm_instance_pool, Blackboard, CompiledModule, CompiledStage, ConfigSchema, EdgeReason,
    ExecutionPlan, FinalizationError, FinalizationOutput, FinalizationStageRunner, GCodeEmitter,
    GCodeSerializer, IrAccessMask, LayerArena, LayerStageError, LayerStageOutput, LayerStageRunner,
    LoadedModule, NoopInstrumentation, NoopLayerProgressSink, PipelineInstrumentation,
    PostpassError, PostpassStageRunner, PrepassExecutionError, PrepassStageOutput,
    PrepassStageRunner, SerialEdge, TierKind, WasmArtifactMetadata,
};
use slicer_ir::{
    BoundingBox3, ConfigView, GCodeIR, GlobalLayer, LayerCollectionIR, MeshIR, Point3,
    PrintMetadata, SemVer, StageId,
};

// ── helpers (mirrored from pipeline_tdd.rs — duplication across integration
//    test files is the codebase norm) ──────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
    })
}

fn minimal_gcode_ir() -> GCodeIR {
    GCodeIR {
        schema_version: semver(1, 0, 0),
        commands: Vec::new(),
        metadata: PrintMetadata {
            slicer_version: "test".into(),
            estimated_print_time_s: 0,
            filament_used_mm: Vec::new(),
            layer_count: 0,
        },
    }
}

fn make_global_layer(index: u32, z: f32) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

fn make_dummy_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModule {
        id: module_id.into(),
        version: semver(1, 0, 0),
        stage: stage_id.into(),
        wit_world: "slicer:world-prepass@1.0.0".into(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
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
        .expect("fixture module should build a pool"),
    );
    CompiledModule {
        module_id: module_id.into(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(ConfigView::new()),
        claims: Vec::new(),
        wasm_component: None,
        requires_modules: Vec::new(),
    }
}

struct NoopPrepassRunner;
impl PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        Ok((PrepassStageOutput::None, Vec::new()))
    }
}

struct NoopLayerRunner;
impl LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        Ok((LayerStageOutput::Success, Vec::new(), Vec::new()))
    }
}

struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        Ok(FinalizationOutput::Success)
    }
}

struct NoopPostpassRunner;
impl PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<slicer_host::PostpassOutput, PostpassError> {
        Ok(slicer_host::PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<slicer_host::PostpassOutput, PostpassError> {
        Ok(slicer_host::PostpassOutput::TextSuccess { text })
    }
}

struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        Ok(minimal_gcode_ir())
    }
}

struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        Ok(String::new())
    }
}

fn noop_runners() -> PipelineStageRunners {
    PipelineStageRunners {
        prepass: Box::new(NoopPrepassRunner),
        layer: Box::new(NoopLayerRunner),
        finalization: Box::new(NoopFinalizationRunner),
        postpass: Box::new(NoopPostpassRunner),
        emitter: Box::new(MinimalEmitter),
        serializer: Box::new(MinimalSerializer),
    }
}

fn empty_raw_config() -> HashMap<slicer_ir::ConfigKey, slicer_ir::ConfigValue> {
    HashMap::new()
}

// ── Test 1 ────────────────────────────────────────────────────────────────

/// Sanity: `&NoopInstrumentation` dispatches cleanly through
/// `run_pipeline_with_instrumentation` and the pipeline succeeds.
#[test]
fn run_with_noop_instrumentation_succeeds_and_collects_nothing() {
    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan: ExecutionPlan {
            prepass_stages: Vec::new(),
            per_layer_stages: Vec::new(),
            layer_finalization_stage: None,
            postpass_stages: Vec::new(),
            global_layers: Arc::new(Vec::new()),
            region_plans: Arc::new(HashMap::new()),
            module_region_index: HashMap::new(),
        },
        runners: noop_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
    };

    let result = run_pipeline_with_instrumentation(
        config,
        &empty_raw_config(),
        &NoopLayerProgressSink,
        &NoopInstrumentation,
    );
    assert!(
        result.is_ok(),
        "pipeline with NoopInstrumentation must succeed: {:?}",
        result.err()
    );
}

// ── Test 2 ────────────────────────────────────────────────────────────────

/// `Collector` receives layer + stage brackets when the plan has a prepass
/// stage and a per-layer stage running over two `GlobalLayer`s.
///
/// Assertions:
/// - `report.layers.len() == 2`
/// - `report.slice_meta.module_count >= 2` (one per-layer module × 2 layers)
/// - Both layer records have a non-empty `stages` vec with
///   `stage_id == "Layer::Perimeters"`.
#[test]
fn run_with_collector_records_phase_and_layer_brackets() {
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::MeshAnalysis".into(),
            modules: vec![make_dummy_module("PrePass::MeshAnalysis", "mesh-analyzer")],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Perimeters".into(),
            modules: vec![make_dummy_module("Layer::Perimeters", "perimeter-gen")],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![make_global_layer(0, 0.2), make_global_layer(1, 0.4)]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let collector = Arc::new(Collector::new("test-model.stl"));

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: noop_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
    };

    // `Arc<Collector>` implements `PipelineInstrumentation` via the inner
    // `Collector`; we coerce to `&dyn PipelineInstrumentation` via a borrow
    // on the contained value.
    let result = run_pipeline_with_instrumentation(
        config,
        &empty_raw_config(),
        &NoopLayerProgressSink,
        collector.as_ref(),
    );
    assert!(result.is_ok(), "pipeline must succeed: {:?}", result.err());

    let report = collector.finalize();

    // Two global layers → two layer records.
    assert_eq!(
        report.layers.len(),
        2,
        "expected 2 layer records, got {}",
        report.layers.len()
    );

    // One per-layer module × 2 layers = 2 module calls (at minimum).
    assert!(
        report.slice_meta.module_count >= 2,
        "expected at least 2 module calls (1 module × 2 layers), got {}",
        report.slice_meta.module_count
    );

    // Every layer record must have a non-empty stages vec whose only entry is
    // the "Layer::Perimeters" stage.
    for layer_rec in &report.layers {
        assert!(
            !layer_rec.stages.is_empty(),
            "layer {} must have at least one stage record",
            layer_rec.layer_index
        );
        let stage_ids: Vec<&str> = layer_rec
            .stages
            .iter()
            .map(|s| s.stage_id.as_str())
            .collect();
        assert!(
            stage_ids.contains(&"Layer::Perimeters"),
            "layer {} stages must include 'Layer::Perimeters', got {:?}",
            layer_rec.layer_index,
            stage_ids
        );
    }
}

// ── Test 3 ────────────────────────────────────────────────────────────────

/// A custom `PipelineInstrumentation` that records every `record_edges` call.
struct EdgeCapture {
    calls: Mutex<Vec<(StageId, TierKind, Vec<SerialEdge>)>>,
}

impl EdgeCapture {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl PipelineInstrumentation for EdgeCapture {
    fn on_phase_start(&self, _phase: slicer_host::Phase) {}
    fn on_phase_end(&self, _phase: slicer_host::Phase) {}
    fn on_stage_start(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_stage_end(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_module_start(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &slicer_ir::ModuleId,
    ) {
    }
    fn on_module_end(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &slicer_ir::ModuleId,
        _wasm_before: u64,
        _wasm_after: u64,
    ) {
    }
    fn on_layer_start(&self, _layer: u32, _z_mm: f32) {}
    fn on_layer_end(&self, _layer: u32) {}

    fn record_edges(&self, stage: &StageId, tier: TierKind, edges: &[SerialEdge]) {
        if let Ok(mut v) = self.calls.lock() {
            v.push((stage.clone(), tier, edges.to_vec()));
        }
    }
}

/// `record_edges` fires at plan freeze for every stage. When two per-layer
/// modules have an overlapping IR write/read path, the emitted edge's
/// `writer_path` must match the declared path.
///
/// Plan:
/// - one prepass stage (one dummy module)
/// - one per-layer stage `Layer::Perimeters` with two modules where module-a
///   writes `"PerimeterIR.regions.walls"` and module-b reads it
/// - two `GlobalLayer`s
///
/// Assertions:
/// - `record_edges` was called at least once for `"Layer::Perimeters"`
/// - the captured edge for that stage has `writer_path == "PerimeterIR.regions.walls"`
#[test]
fn record_edges_fires_for_every_stage_at_plan_freeze() {
    // Build two per-layer modules with overlapping IR paths so
    // `compute_serial_edges_from_compiled` emits a real IrWriteRead edge.
    let shared_path = "PerimeterIR.regions.walls".to_string();

    let mut module_a = make_dummy_module("Layer::Perimeters", "perimeter-writer");
    module_a.ir_write_mask = IrAccessMask {
        paths: vec![shared_path.clone()],
    };

    let mut module_b = make_dummy_module("Layer::Perimeters", "perimeter-reader");
    module_b.ir_read_mask = IrAccessMask {
        paths: vec![shared_path.clone()],
    };

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::MeshAnalysis".into(),
            modules: vec![make_dummy_module("PrePass::MeshAnalysis", "mesh-analyzer")],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Perimeters".into(),
            modules: vec![module_a, module_b],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![make_global_layer(0, 0.2), make_global_layer(1, 0.4)]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let capture = EdgeCapture::new();

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: noop_runners(),
        resolved_configs: Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
    };

    let result = run_pipeline_with_instrumentation(
        config,
        &empty_raw_config(),
        &NoopLayerProgressSink,
        &capture,
    );
    assert!(result.is_ok(), "pipeline must succeed: {:?}", result.err());

    let calls = capture.calls.lock().unwrap();

    // `record_edges` must have been called at least for the per-layer stage.
    let perimeter_entry = calls
        .iter()
        .find(|(stage_id, _, _)| stage_id == "Layer::Perimeters");
    assert!(
        perimeter_entry.is_some(),
        "record_edges must have been called for 'Layer::Perimeters'; calls: {:?}",
        calls.iter().map(|(s, _, _)| s.as_str()).collect::<Vec<_>>()
    );

    let (_, _, edges) = perimeter_entry.unwrap();

    // Exactly one IrWriteRead edge between the two modules.
    assert!(
        !edges.is_empty(),
        "expected at least one serial edge for 'Layer::Perimeters', got none"
    );

    let ir_edge = edges.iter().find(|e| {
        matches!(&e.reason, EdgeReason::IrWriteRead { writer_path } if writer_path == &shared_path)
    });
    assert!(
        ir_edge.is_some(),
        "expected an IrWriteRead edge with writer_path == {:?}; edges: {:?}",
        shared_path,
        edges
    );

    let edge = ir_edge.unwrap();
    assert_eq!(
        edge.from, "perimeter-writer",
        "edge 'from' must be the writing module"
    );
    assert_eq!(
        edge.to, "perimeter-reader",
        "edge 'to' must be the reading module"
    );
    match &edge.reason {
        EdgeReason::IrWriteRead { writer_path } => {
            assert_eq!(
                writer_path, &shared_path,
                "writer_path must be {:?}",
                shared_path
            );
        }
        other => panic!("expected IrWriteRead reason, got {:?}", other),
    }
}
