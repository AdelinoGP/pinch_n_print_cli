#![allow(missing_docs)]

//! TDD tests for TASK-104: Python text-postprocess bridge.
//!
//! Proves that:
//! - a minimal Python-backed text postprocess module executes through the
//!   real `execute_postpass` path and produces the expected transformed text,
//! - script/runner failures surface as structured `PostpassError::FatalModule`
//!   diagnostics carrying the python bridge phase and stderr text,
//! - repeated runs over deterministic input produce byte-identical output.
//!
//! Reference: docs/05_module_sdk.md §"Python Bridge (TextPostProcess tier)"
//! and docs/04_host_scheduler.md `execute_postpass` pseudocode.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, execute_postpass, Blackboard, CompiledModule, CompiledStage,
    ConfigSchema, ExecutionPlan, GCodeEmitter, GCodeSerializer, PostpassError, PythonBinding,
    PythonBridge, PythonBridgeError, PythonBridgePhase, PythonPostpassRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, GCodeCommand, GCodeIR, IndexedTriangleSet,
    LayerCollectionIR, MeshIR, ObjectConfig, ObjectMesh, Point3, PrintMetadata, SemVer,
    Transform3d,
};

// The interpreter is embedded via pyo3 `auto-initialize`, so these tests
// require `libpython` at build/link time but no subprocess at runtime.

// ----------------------------------------------------------------------
// Test 1 — minimal python-backed text postprocess runs on real path
// ----------------------------------------------------------------------

#[test]
fn python_bridge_runs_minimal_text_postprocess_on_real_path() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("pp.py");
    write_file(
        &script,
        r#"
def process_gcode(text, config):
    amp = config.get("amplitude", 1.0)
    return f"; amp={amp}\n" + text.upper()
"#,
    );

    let module_id = "com.example.py-upper".to_string();
    let stage = python_text_stage(&module_id, config_with_amplitude(0.5));
    let plan = plan_with_postpass(vec![stage]);

    let bindings = HashMap::from([(
        module_id.clone(),
        PythonBinding {
            script_path: script.clone(),
            entry: "process_gcode".to_string(),
        },
    )]);
    let mut runner = PythonPostpassRunner::new(bindings);

    let blackboard = Blackboard::new(Arc::new(mesh_fixture()), 0);
    let layers = vec![layer_collection_fixture(0, 0.2)];
    let emitter = StubEmitter;
    let serializer = StubSerializer { text: "g1 x1\ng1 x2".to_string() };

    let (text, _audits) = execute_postpass(&plan, &layers, &blackboard, &emitter, &serializer, &mut runner)
        .expect("postpass should succeed");

    assert_eq!(text, "; amp=0.5\nG1 X1\nG1 X2");
}

// ----------------------------------------------------------------------
// Test 2 — script failure surfaces as structured fatal diagnostic
// ----------------------------------------------------------------------

#[test]
fn python_bridge_script_failure_surfaces_as_structured_diagnostic() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("boom.py");
    write_file(
        &script,
        r#"
def process_gcode(text, config):
    raise ValueError("synthetic failure for TASK-104 test")
"#,
    );

    let module_id = "com.example.py-boom".to_string();
    let stage = python_text_stage(&module_id, empty_config());
    let plan = plan_with_postpass(vec![stage]);

    let bindings = HashMap::from([(
        module_id.clone(),
        PythonBinding { script_path: script, entry: "process_gcode".to_string() },
    )]);
    let mut runner = PythonPostpassRunner::new(bindings);

    let blackboard = Blackboard::new(Arc::new(mesh_fixture()), 0);
    let layers = vec![layer_collection_fixture(0, 0.2)];

    let err = execute_postpass(
        &plan,
        &layers,
        &blackboard,
        &StubEmitter,
        &StubSerializer { text: "irrelevant".to_string() },
        &mut runner,
    )
    .expect_err("expected fatal error");

    match err {
        PostpassError::FatalModule { module_id: m, message, .. } => {
            assert_eq!(m, "com.example.py-boom");
            assert!(
                message.contains("synthetic failure for TASK-104 test"),
                "expected stderr bubble-up, got: {message}"
            );
            assert!(
                message.contains("ScriptError") || message.contains("python exit="),
                "expected structured phase detail, got: {message}"
            );
        }
        other => panic!("expected FatalModule, got {other:?}"),
    }
}

// ----------------------------------------------------------------------
// Test 3 — deterministic input yields byte-identical repeated output
// ----------------------------------------------------------------------

#[test]
fn python_bridge_is_deterministic_for_deterministic_input() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("det.py");
    write_file(
        &script,
        r#"
def process_gcode(text, config):
    lines = text.splitlines()
    # Deterministic transform: prepend a sorted config digest, reverse lines.
    items = sorted((str(k), str(v)) for k, v in config.items())
    header = "; cfg=" + ",".join(f"{k}={v}" for k, v in items)
    body = "\n".join(reversed(lines))
    return header + "\n" + body
"#,
    );

    let module_id = "com.example.py-det".to_string();
    let bindings = HashMap::from([(
        module_id.clone(),
        PythonBinding { script_path: script, entry: "process_gcode".to_string() },
    )]);
    let mut runner = PythonPostpassRunner::new(bindings);

    let stage = python_text_stage(&module_id, config_with_amplitude(0.25));
    let plan = plan_with_postpass(vec![stage]);
    let blackboard = Blackboard::new(Arc::new(mesh_fixture()), 0);
    let layers = vec![layer_collection_fixture(0, 0.2)];
    let input = "G28\nG1 X10 Y20\nG1 Z5";
    let emitter = StubEmitter;
    let serializer = StubSerializer { text: input.to_string() };

    let (a_text, _a_audits) = execute_postpass(&plan, &layers, &blackboard, &emitter, &serializer, &mut runner).unwrap();
    let (b_text, _b_audits) = execute_postpass(&plan, &layers, &blackboard, &emitter, &serializer, &mut runner).unwrap();
    let (c_text, _c_audits) = execute_postpass(&plan, &layers, &blackboard, &emitter, &serializer, &mut runner).unwrap();

    assert_eq!(a_text, b_text, "run 1 vs 2 must be identical");
    assert_eq!(b_text, c_text, "run 2 vs 3 must be identical");
    assert!(a_text.starts_with("; cfg=amplitude=0.25\n"), "unexpected: {a_text}");
    assert!(a_text.contains("G1 Z5"));
}

// ----------------------------------------------------------------------
// Test 4 — missing script surfaces MissingScript phase directly
// ----------------------------------------------------------------------

#[test]
fn python_bridge_missing_script_reports_missing_script_phase() {
    let bridge = PythonBridge::default();
    let binding = PythonBinding {
        script_path: PathBuf::from("/definitely/does/not/exist/__slicer_missing__.py"),
        entry: "process_gcode".to_string(),
    };
    let err: PythonBridgeError = bridge
        .run_text(
            &binding,
            &empty_config(),
            "G28\n",
            &"com.example.py-missing".to_string(),
            &"PostPass::TextPostProcess".to_string(),
        )
        .expect_err("must fail when script is absent");

    assert_eq!(err.phase, PythonBridgePhase::MissingScript);
    assert!(err.message.contains("script not found"), "got: {}", err.message);
}

// ----------------------------------------------------------------------
// Test 5 — non-str return surfaces OutputEncoding phase
// ----------------------------------------------------------------------

#[test]
fn python_bridge_non_string_return_reports_output_encoding_phase() {
    let tmp = tempfile::tempdir().unwrap();
    let script = tmp.path().join("bad_ret.py");
    write_file(
        &script,
        r#"
def process_gcode(text, config):
    return 42  # not a str
"#,
    );

    let bridge = PythonBridge::default();
    let binding = PythonBinding { script_path: script, entry: "process_gcode".to_string() };
    let err = bridge
        .run_text(
            &binding,
            &empty_config(),
            "irrelevant",
            &"com.example.py-badret".to_string(),
            &"PostPass::TextPostProcess".to_string(),
        )
        .expect_err("must fail when entry returns non-str");

    assert_eq!(err.phase, PythonBridgePhase::OutputEncoding);
    assert!(
        err.message.contains("must return str"),
        "got: {}",
        err.message
    );
}

// ----------------------------------------------------------------------
// Fixtures
// ----------------------------------------------------------------------

fn write_file(path: &std::path::Path, body: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

fn empty_config() -> ConfigView {
    ConfigView::from_map(HashMap::new())
}

fn config_with_amplitude(v: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("amplitude".to_string(), ConfigValue::Float(v));
    ConfigView::from_map(fields)
}

fn plan_with_postpass(stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: stages,
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
    }
}

fn python_text_stage(module_id: &str, config: ConfigView) -> CompiledStage {
    CompiledStage {
        stage_id: "PostPass::TextPostProcess".to_string(),
        modules: vec![compiled_module(
            "PostPass::TextPostProcess",
            module_id,
            config,
        )],
    }
}

fn compiled_module(stage_id: &str, module_id: &str, config: ConfigView) -> CompiledModule {
    let loaded = slicer_host::LoadedModule {
        id: module_id.to_string(),
        version: semver(1, 0, 0),
        stage: stage_id.to_string(),
        wit_world: "slicer:world-postpass@1.0.0".to_string(),
        ir_reads: vec![],
        ir_writes: vec![],
        claims: vec![],
        requires_claims: vec![],
        incompatible_with: vec![],
        requires_modules: vec![],
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: vec![],
        overridable_per_layer: vec![],
        layer_parallel_safe: false,
        wasm_path: PathBuf::from(format!("fixtures/{module_id}.wasm")),
        placeholder_wasm: false,
    };
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata { uses_shared_memory: false },
        )
        .expect("pool"),
    );
    CompiledModule {
        module_id: module_id.to_string(),
        instance_pool: pool,
        ir_read_mask: slicer_host::IrAccessMask { paths: vec![] },
        ir_write_mask: slicer_host::IrAccessMask { paths: vec![] },
        config_view: Arc::new(config),
        wasm_component: None,
    }
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet { vertices: vec![], indices: vec![] },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig { data: HashMap::new() },
            modifier_volumes: vec![],
            paint_data: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    }
}

fn layer_collection_fixture(index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: index,
        z,
        ordered_entities: vec![],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
    }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

// ----------------------------------------------------------------------
// Stub emitter/serializer — real `execute_postpass` drives through these.
// ----------------------------------------------------------------------

struct StubEmitter;
impl GCodeEmitter for StubEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        Ok(GCodeIR {
            schema_version: semver(1, 0, 0),
            commands: vec![GCodeCommand::Comment { text: "stub".to_string() }],
            metadata: PrintMetadata {
                estimated_print_time_s: 0,
                filament_used_mm: vec![0.0],
                layer_count: 1,
                slicer_version: "test".to_string(),
            },
        })
    }
}

struct StubSerializer {
    text: String,
}
impl GCodeSerializer for StubSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        Ok(self.text.clone())
    }
}
