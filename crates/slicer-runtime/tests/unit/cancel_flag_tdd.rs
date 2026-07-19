#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

use slicer_ir::{BoundingBox3, GlobalLayer, LayerStageCommit, MeshIR, Point3, SemVer, StageId};
use slicer_runtime::layer_executor::execute_per_layer_with_instrumentation;
use slicer_runtime::progress_events::{ProgressEvent, ProgressEventType, SliceEventCollector};
use slicer_runtime::run::run_slice_with_collector;
use slicer_runtime::{
    Blackboard, CompiledModuleLive, ExecutionPlan, LayerStageError, LayerStageInput,
    LayerStageRunner, NoopInstrumentation, NoopLayerProgressSink, SliceOutcome,
};

fn workspace_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo test");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn run_cancel_test_scenario(cancel_flag: Arc<AtomicBool>) -> Result<SliceOutcome, String> {
    let root = workspace_root();
    let model = root.join("resources").join("regression_wedge.stl");
    let module_dir = root.join("modules").join("core-modules");
    let mesh = Arc::new(slicer_model_io::load_model(&model).map_err(|e| e.to_string())?);
    let opts = slicer_runtime::SliceRunOptions {
        mesh,
        model_label: model.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        progress_events: false,
        cancel_flag: Some(cancel_flag),
        config_overrides: HashMap::new(),
    };
    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    match run_slice_with_collector(opts, Some(Arc::clone(&collector))) {
        Ok(outcome) => {
            let events = collector
                .lock()
                .expect("progress collector must not be poisoned")
                .events()
                .to_vec();
            assert!(
                events
                    .iter()
                    .any(|event| event.event == ProgressEventType::SliceComplete),
                "uncancelled run must emit slice_complete"
            );
            assert!(
                events
                    .iter()
                    .all(|event| event.event != ProgressEventType::Cancelled),
                "uncancelled run must not emit cancelled"
            );
            Ok(outcome)
        }
        Err(error) => {
            let events = collector
                .lock()
                .expect("progress collector must not be poisoned")
                .events()
                .to_vec();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.event == ProgressEventType::Cancelled)
                    .count(),
                1,
                "cancelled run must emit exactly one cancelled event"
            );
            assert!(
                events
                    .iter()
                    .all(|event| event.event != ProgressEventType::SliceComplete),
                "cancelled run must not emit slice_complete"
            );
            Err(error.to_string())
        }
    }
}

fn empty_mesh() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
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

fn one_layer_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: std::collections::BTreeMap::new(),
    }
}

struct NoopLayerRunner;

impl LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        Ok(None)
    }
}

#[test]
fn progress_event_cancelled_serializes_with_required_fields() {
    let event = ProgressEvent::cancelled("test-slice-id".to_string(), 12345u64);
    let json = serde_json::to_string(&event).expect("cancelled event should serialize");

    assert!(json.contains("\"event\":\"cancelled\""));
    assert!(json.contains("\"schema_version\":\"1.3.0\""));
    assert!(json.contains("\"timestamp_ms\":12345"));
    assert!(json.contains("\"slice_id\":\"test-slice-id\""));
}

#[test]
fn cancel_flag_preset_returns_err_with_cancelled_event() {
    let result = run_cancel_test_scenario(Arc::new(AtomicBool::new(true)));
    assert!(result.is_err(), "a preset cancellation must fail the slice");
}

#[test]
fn cancel_flag_direct_layer_executor_returns_cancelled() {
    let flag = AtomicBool::new(true);
    let blackboard = Blackboard::new(empty_mesh(), 1);
    let runner = NoopLayerRunner;
    let sink = NoopLayerProgressSink;
    let result = execute_per_layer_with_instrumentation(
        &one_layer_plan(),
        &blackboard,
        &runner,
        &sink,
        &NoopInstrumentation,
        &HashMap::new(),
        Some(&flag),
    );

    assert_eq!(result, Err(slicer_runtime::LayerExecutionError::Cancelled));
}

#[test]
fn cancel_flag_unset_inert() {
    run_cancel_test_scenario(Arc::new(AtomicBool::new(false)))
        .expect("an unset cancellation flag must not cancel the slice");
}
