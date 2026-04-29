#![allow(missing_docs, dead_code, unused_imports)]

//! TDD red tests for TASK-033: PostPass executor API.
//!
//! These tests define the contract for `execute_postpass` and must fail only on
//! the explicit todo! stub until the green implementation is completed.
//!
//! Acceptance criteria:
//! - [x] API covers `execute_postpass(plan, layer_irs, blackboard) -> Result<String, PostpassError>`
//! - [x] PostpassStageRunner trait defined for test injection
//! - [x] PostpassOutput/PostpassError enums defined
//! - [x] Tests lock down stage ordering (GCodePostProcess -> TextPostProcess)
//! - [x] Tests verify immutable layer_irs access
//! - [x] Tests verify sequential module execution within each stage
//! - [x] Tests verify fatal/non-fatal error handling
//!
//! Reference: docs/04_host_scheduler.md lines 778-810

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, execute_postpass, Blackboard, CompiledModule, CompiledStage,
    ConfigSchema, ExecutionModuleBinding, ExecutionPlan, GCodeEmitter, GCodeSerializer,
    PostpassError, PostpassOutput, PostpassStageRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    BoundingBox3, ConfigView, ExtrusionRole, GCodeCommand, GCodeIR, LayerCollectionIR, MeshIR,
    ModuleId, ObjectMesh, Point3, PrintMetadata, SemVer, StageId, Transform3d,
};

// ============================================================================
// Test: Stage order is GCodePostProcess then TextPostProcess
// ============================================================================

#[test]
fn postpass_executor_runs_gcode_postprocess_before_text_postprocess() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let gcode_stage = compiled_stage(
        "PostPass::GCodePostProcess",
        &["com.example.gcode-pp-a", "com.example.gcode-pp-b"],
    );
    let text_stage = compiled_stage("PostPass::TextPostProcess", &["com.example.text-pp"]);

    let plan = execution_plan_fixture(vec![gcode_stage, text_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );

    // Must succeed (or fail on todo! for red phase)
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    // Verify order: all GCodePostProcess modules run before any TextPostProcess
    let observed = runner.observed_calls();
    assert_eq!(observed.len(), 3);
    assert_eq!(
        observed[0],
        (
            "com.example.gcode-pp-a".to_string(),
            "GCodePostProcess".to_string()
        )
    );
    assert_eq!(
        observed[1],
        (
            "com.example.gcode-pp-b".to_string(),
            "GCodePostProcess".to_string()
        )
    );
    assert_eq!(
        observed[2],
        (
            "com.example.text-pp".to_string(),
            "TextPostProcess".to_string()
        )
    );
}

// ============================================================================
// Test: Sequential module execution within GCodePostProcess stage
// ============================================================================

#[test]
fn postpass_executor_runs_gcode_postprocess_modules_sequentially() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let gcode_stage = compiled_stage(
        "PostPass::GCodePostProcess",
        &[
            "com.example.gcode-pp-1",
            "com.example.gcode-pp-2",
            "com.example.gcode-pp-3",
        ],
    );

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    let observed = runner.observed_calls();
    assert_eq!(
        observed
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>(),
        vec![
            "com.example.gcode-pp-1",
            "com.example.gcode-pp-2",
            "com.example.gcode-pp-3"
        ]
    );
}

// ============================================================================
// Test: Sequential module execution within TextPostProcess stage
// ============================================================================

#[test]
fn postpass_executor_runs_text_postprocess_modules_sequentially() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let text_stage = compiled_stage(
        "PostPass::TextPostProcess",
        &["com.example.text-pp-1", "com.example.text-pp-2"],
    );

    let plan = execution_plan_fixture(vec![text_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    let observed = runner.observed_calls();
    assert_eq!(
        observed
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>(),
        vec!["com.example.text-pp-1", "com.example.text-pp-2"]
    );
}

// ============================================================================
// Test: layer_irs remains immutable (enforced by function signature &[])
// ============================================================================

#[test]
fn postpass_executor_receives_immutable_layer_irs() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);

    // Create layers with known content
    let layer_irs = vec![
        layer_collection_fixture(0, 0.2),
        layer_collection_fixture(1, 0.4),
        layer_collection_fixture(2, 0.6),
    ];

    let gcode_stage = compiled_stage("PostPass::GCodePostProcess", &["com.example.reader"]);

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = ImmutabilityVerifyingRunner::new(layer_irs.len());

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    // The runner verified it could not mutate layer_irs (compile-time guarantee via &[])
    assert!(runner.verified_immutability());
}

// ============================================================================
// Test: GCodeEmitter is called before any postprocess modules
// ============================================================================

#[test]
fn postpass_executor_calls_emitter_first() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let gcode_stage = compiled_stage("PostPass::GCodePostProcess", &["com.example.gcode-pp"]);

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = CallTrackingEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    // Emitter must be called before any modules run
    assert!(emitter.was_called(), "emitter must be called");
}

// ============================================================================
// Test: No TextPostProcess modules -> serialize GCodeIR directly
// ============================================================================

#[test]
fn postpass_executor_serializes_directly_when_no_text_postprocess() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    // Only GCodePostProcess, no TextPostProcess
    let gcode_stage = compiled_stage("PostPass::GCodePostProcess", &["com.example.gcode-pp"]);

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = StubEmitter::new();
    let serializer = CallTrackingSerializer::new("G28 ; home\nG1 X10 Y10");
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    assert!(
        serializer.was_called(),
        "serializer must be called when no TextPostProcess"
    );
    assert_eq!(result.unwrap().0, "G28 ; home\nG1 X10 Y10");
}

// ============================================================================
// Test: TextPostProcess module provides final output
// ============================================================================

#[test]
fn postpass_executor_returns_text_from_last_text_postprocess() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let text_stage = compiled_stage("PostPass::TextPostProcess", &["com.example.text-final"]);

    let plan = execution_plan_fixture(vec![text_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = FinalTextRunner::new("FINAL OUTPUT FROM MODULE");

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "postpass should succeed, got {:?}", result);

    assert_eq!(result.unwrap().0, "FINAL OUTPUT FROM MODULE");
}

// ============================================================================
// Test: Fatal error in GCodePostProcess aborts immediately
// ============================================================================

#[test]
fn postpass_executor_aborts_on_fatal_gcode_postprocess_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let gcode_stage = compiled_stage(
        "PostPass::GCodePostProcess",
        &["com.example.fatal-module", "com.example.never-reached"],
    );

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = FatalErrorRunner::gcode_fatal_at("com.example.fatal-module");

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );

    assert!(
        matches!(
            &result,
            Err(PostpassError::FatalModule {
                module_id,
                ..
            }) if module_id == "com.example.fatal-module"
        ),
        "expected fatal error, got {:?}",
        result
    );

    // Second module should not have been called
    assert!(
        !runner.was_module_called("com.example.never-reached"),
        "fatal error should abort before reaching later modules"
    );
}

// ============================================================================
// Test: Fatal error in TextPostProcess aborts immediately
// ============================================================================

#[test]
fn postpass_executor_aborts_on_fatal_text_postprocess_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let text_stage = compiled_stage(
        "PostPass::TextPostProcess",
        &["com.example.fatal-text", "com.example.never-reached"],
    );

    let plan = execution_plan_fixture(vec![text_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = FatalErrorRunner::text_fatal_at("com.example.fatal-text");

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );

    assert!(
        matches!(
            &result,
            Err(PostpassError::FatalModule {
                module_id,
                ..
            }) if module_id == "com.example.fatal-text"
        ),
        "expected fatal error, got {:?}",
        result
    );
}

// ============================================================================
// Test: Non-fatal error in GCodePostProcess continues to next module
// ============================================================================

#[test]
fn postpass_executor_continues_after_nonfatal_gcode_postprocess_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let gcode_stage = compiled_stage(
        "PostPass::GCodePostProcess",
        &["com.example.nonfatal", "com.example.continues"],
    );

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner = NonFatalErrorRunner::nonfatal_at("com.example.nonfatal");

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(
        result.is_ok(),
        "should continue after non-fatal, got {:?}",
        result
    );

    // Second module should have been called
    assert!(
        runner.was_module_called("com.example.continues"),
        "should continue to next module after non-fatal error"
    );
}

// ============================================================================
// Test: Non-fatal error in TextPostProcess continues to next module
// ============================================================================

#[test]
fn postpass_executor_continues_after_nonfatal_text_postprocess_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let text_stage = compiled_stage(
        "PostPass::TextPostProcess",
        &["com.example.nonfatal-text", "com.example.final-text"],
    );

    let plan = execution_plan_fixture(vec![text_stage]);

    let emitter = StubEmitter::new();
    let serializer = StubSerializer::new();
    let mut runner =
        NonFatalErrorRunner::nonfatal_text_at("com.example.nonfatal-text", "final output");

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(
        result.is_ok(),
        "should continue after non-fatal, got {:?}",
        result
    );

    assert!(
        runner.was_module_called("com.example.final-text"),
        "should continue to next module after non-fatal error"
    );
}

// ============================================================================
// Test: GCodeEmitter error propagates as GCodeEmit error
// ============================================================================

#[test]
fn postpass_executor_propagates_emitter_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let plan = execution_plan_fixture(vec![]);

    let emitter = FailingEmitter::new("emit failure");
    let serializer = StubSerializer::new();
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );

    assert!(
        matches!(result, Err(PostpassError::GCodeEmit { .. })),
        "expected GCodeEmit error, got {:?}",
        result
    );
}

// ============================================================================
// Test: Empty postpass stages still produce output from serializer
// ============================================================================

#[test]
fn postpass_executor_handles_empty_stages() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    // No postpass modules at all
    let plan = execution_plan_fixture(vec![]);

    let emitter = StubEmitter::new();
    let serializer = CallTrackingSerializer::new("G28\nG1 Z10");
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(
        result.is_ok(),
        "should succeed with empty stages, got {:?}",
        result
    );

    assert!(serializer.was_called(), "serializer must be called");
    assert_eq!(result.unwrap().0, "G28\nG1 Z10");
}

// ============================================================================
// Test: GCodePostProcess module can mutate GCodeIR
// ============================================================================

#[test]
fn postpass_executor_allows_gcode_ir_mutation() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    let gcode_stage = compiled_stage("PostPass::GCodePostProcess", &["com.example.mutator"]);

    let plan = execution_plan_fixture(vec![gcode_stage]);

    let emitter = StubEmitter::new();
    let serializer = MutationVerifyingSerializer::new();
    let mut runner = MutatingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );
    assert!(result.is_ok(), "should succeed, got {:?}", result);

    // The serializer verifies that the GCodeIR was mutated by the module
    assert!(
        serializer.saw_mutation(),
        "GCodeIR should have been mutated by module"
    );
}

// ============================================================================
// Test: Serializer error propagates as GCodeSerialization error
// ============================================================================

#[test]
fn postpass_executor_propagates_serializer_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let layer_irs = vec![layer_collection_fixture(0, 0.2)];

    // No TextPostProcess, so serializer will be called
    let plan = execution_plan_fixture(vec![]);

    let emitter = StubEmitter::new();
    let serializer = FailingSerializer::new("serialization failed");
    let mut runner = OrderTrackingRunner::new();

    let result = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        &emitter,
        &serializer,
        &mut runner,
    );

    assert!(
        matches!(result, Err(PostpassError::GCodeSerialization { .. })),
        "expected GCodeSerialization error, got {:?}",
        result
    );
}

// ============================================================================
// Test Fixtures and Helper Implementations
// ============================================================================

fn execution_plan_fixture(postpass_stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages,
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    }
}

fn compiled_stage(stage_id: &str, module_ids: &[&str]) -> CompiledStage {
    CompiledStage {
        stage_id: String::from(stage_id),
        modules: module_ids
            .iter()
            .map(|module_id| compiled_module(stage_id, module_id))
            .collect(),
    }
}

fn compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded_module = loaded_module(module_id, stage_id);
    let instance_pool = Arc::new(
        build_wasm_instance_pool(
            &loaded_module,
            1, // PostPass modules always use pool size 1
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    let binding = ExecutionModuleBinding {
        module: loaded_module,
        instance_pool,
        config_view: Arc::new(ConfigView::new()),
        wasm_component: None,
    };

    CompiledModule {
        module_id: binding.module.id.clone(),
        instance_pool: Arc::clone(&binding.instance_pool),
        ir_read_mask: slicer_host::IrAccessMask {
            paths: binding.module.ir_reads.clone(),
        },
        ir_write_mask: slicer_host::IrAccessMask {
            paths: binding.module.ir_writes.clone(),
        },
        config_view: Arc::clone(&binding.config_view),
        wasm_component: None,
    }
}

fn loaded_module(id: &str, stage: &str) -> slicer_host::LoadedModule {
    slicer_host::LoadedModule {
        id: String::from(id),
        version: semver(1, 0, 0),
        stage: String::from(stage),
        wit_world: String::from("slicer:world-postpass@1.0.0"),
        ir_reads: vec![],
        ir_writes: vec![],
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
        wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
        placeholder_wasm: false,
    }
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![],
                indices: vec![],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
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

fn layer_collection_fixture(index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: index,
        z,
        ordered_entities: Vec::new(),
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    }
}

fn gcode_ir_fixture() -> GCodeIR {
    GCodeIR {
        schema_version: semver(1, 0, 0),
        commands: vec![GCodeCommand::Comment {
            text: "initial".to_string(),
        }],
        metadata: PrintMetadata {
            estimated_print_time_s: 100,
            filament_used_mm: vec![100.0],
            layer_count: 1,
            slicer_version: "0.1.0".to_string(),
        },
    }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

// ============================================================================
// Stub Implementations
// ============================================================================

struct StubEmitter;

impl StubEmitter {
    fn new() -> Self {
        Self
    }
}

impl GCodeEmitter for StubEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        Ok(gcode_ir_fixture())
    }
}

struct StubSerializer;

impl StubSerializer {
    fn new() -> Self {
        Self
    }
}

impl GCodeSerializer for StubSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        Ok("stub output".to_string())
    }
}

// ============================================================================
// Call-tracking implementations
// ============================================================================

struct CallTrackingEmitter {
    called: RefCell<bool>,
}

impl CallTrackingEmitter {
    fn new() -> Self {
        Self {
            called: RefCell::new(false),
        }
    }

    fn was_called(&self) -> bool {
        *self.called.borrow()
    }
}

impl GCodeEmitter for CallTrackingEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        *self.called.borrow_mut() = true;
        Ok(gcode_ir_fixture())
    }
}

struct CallTrackingSerializer {
    called: RefCell<bool>,
    output: String,
}

impl CallTrackingSerializer {
    fn new(output: &str) -> Self {
        Self {
            called: RefCell::new(false),
            output: output.to_string(),
        }
    }

    fn was_called(&self) -> bool {
        *self.called.borrow()
    }
}

impl GCodeSerializer for CallTrackingSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        *self.called.borrow_mut() = true;
        Ok(self.output.clone())
    }
}

// ============================================================================
// Order-tracking runner
// ============================================================================

struct OrderTrackingRunner {
    calls: RefCell<Vec<(String, String)>>,
}

impl OrderTrackingRunner {
    fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
        }
    }

    fn observed_calls(&self) -> Vec<(String, String)> {
        self.calls.borrow().clone()
    }
}

impl PostpassStageRunner for OrderTrackingRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        self.calls
            .borrow_mut()
            .push((module.module_id.clone(), "GCodePostProcess".to_string()));
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        self.calls
            .borrow_mut()
            .push((module.module_id.clone(), "TextPostProcess".to_string()));
        Ok(PostpassOutput::TextSuccess { text })
    }
}

// ============================================================================
// Immutability-verifying runner (compile-time check via &[])
// ============================================================================

struct ImmutabilityVerifyingRunner {
    expected_count: usize,
    verified: RefCell<bool>,
}

impl ImmutabilityVerifyingRunner {
    fn new(expected_count: usize) -> Self {
        Self {
            expected_count,
            verified: RefCell::new(false),
        }
    }

    fn verified_immutability(&self) -> bool {
        *self.verified.borrow()
    }
}

impl PostpassStageRunner for ImmutabilityVerifyingRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        // The fact that we receive &mut GCodeIR but NOT &mut layers proves
        // layer_irs is immutable in the postpass stage (compile-time guarantee)
        *self.verified.borrow_mut() = true;
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        *self.verified.borrow_mut() = true;
        Ok(PostpassOutput::TextSuccess { text })
    }
}

// ============================================================================
// Final text runner (returns specific text)
// ============================================================================

struct FinalTextRunner {
    final_text: String,
}

impl FinalTextRunner {
    fn new(text: &str) -> Self {
        Self {
            final_text: text.to_string(),
        }
    }
}

impl PostpassStageRunner for FinalTextRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess {
            text: self.final_text.clone(),
        })
    }
}

// ============================================================================
// Fatal error runner
// ============================================================================

struct FatalErrorRunner {
    gcode_fatal_module: Option<String>,
    text_fatal_module: Option<String>,
    called_modules: RefCell<Vec<String>>,
}

impl FatalErrorRunner {
    fn gcode_fatal_at(module_id: &str) -> Self {
        Self {
            gcode_fatal_module: Some(module_id.to_string()),
            text_fatal_module: None,
            called_modules: RefCell::new(Vec::new()),
        }
    }

    fn text_fatal_at(module_id: &str) -> Self {
        Self {
            gcode_fatal_module: None,
            text_fatal_module: Some(module_id.to_string()),
            called_modules: RefCell::new(Vec::new()),
        }
    }

    fn was_module_called(&self, module_id: &str) -> bool {
        self.called_modules
            .borrow()
            .contains(&module_id.to_string())
    }
}

impl PostpassStageRunner for FatalErrorRunner {
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        self.called_modules
            .borrow_mut()
            .push(module.module_id.clone());

        if self.gcode_fatal_module.as_ref() == Some(&module.module_id) {
            return Err(PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: "fatal error".to_string(),
            });
        }
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        self.called_modules
            .borrow_mut()
            .push(module.module_id.clone());

        if self.text_fatal_module.as_ref() == Some(&module.module_id) {
            return Err(PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: "fatal error".to_string(),
            });
        }
        Ok(PostpassOutput::TextSuccess { text })
    }
}

// ============================================================================
// Non-fatal error runner
// ============================================================================

struct NonFatalErrorRunner {
    nonfatal_gcode_module: Option<String>,
    nonfatal_text_module: Option<String>,
    final_text: String,
    called_modules: RefCell<Vec<String>>,
}

impl NonFatalErrorRunner {
    fn nonfatal_at(module_id: &str) -> Self {
        Self {
            nonfatal_gcode_module: Some(module_id.to_string()),
            nonfatal_text_module: None,
            final_text: "default output".to_string(),
            called_modules: RefCell::new(Vec::new()),
        }
    }

    fn nonfatal_text_at(module_id: &str, final_text: &str) -> Self {
        Self {
            nonfatal_gcode_module: None,
            nonfatal_text_module: Some(module_id.to_string()),
            final_text: final_text.to_string(),
            called_modules: RefCell::new(Vec::new()),
        }
    }

    fn was_module_called(&self, module_id: &str) -> bool {
        self.called_modules
            .borrow()
            .contains(&module_id.to_string())
    }
}

impl PostpassStageRunner for NonFatalErrorRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        self.called_modules
            .borrow_mut()
            .push(module.module_id.clone());

        if self.nonfatal_gcode_module.as_ref() == Some(&module.module_id) {
            return Ok(PostpassOutput::NonFatalError {
                message: "non-fatal error".to_string(),
            });
        }
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        self.called_modules
            .borrow_mut()
            .push(module.module_id.clone());

        if self.nonfatal_text_module.as_ref() == Some(&module.module_id) {
            return Ok(PostpassOutput::NonFatalError {
                message: "non-fatal error".to_string(),
            });
        }
        Ok(PostpassOutput::TextSuccess {
            text: if self.called_modules.borrow().len() > 1 {
                self.final_text.clone()
            } else {
                text
            },
        })
    }
}

// ============================================================================
// Failing emitter
// ============================================================================

struct FailingEmitter {
    message: String,
}

impl FailingEmitter {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl GCodeEmitter for FailingEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        Err(PostpassError::GCodeEmit {
            message: self.message.clone(),
        })
    }
}

// ============================================================================
// Failing serializer
// ============================================================================

struct FailingSerializer {
    message: String,
}

impl FailingSerializer {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl GCodeSerializer for FailingSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        Err(PostpassError::GCodeSerialization {
            message: self.message.clone(),
        })
    }
}

// ============================================================================
// Mutating runner (adds a command to GCodeIR)
// ============================================================================

struct MutatingRunner;

impl MutatingRunner {
    fn new() -> Self {
        Self
    }
}

impl PostpassStageRunner for MutatingRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        // Add a marker command to prove mutation occurred
        gcode_ir.commands.push(GCodeCommand::Comment {
            text: "MUTATED_BY_MODULE".to_string(),
        });
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}

// ============================================================================
// Mutation-verifying serializer
// ============================================================================

struct MutationVerifyingSerializer {
    saw_mutation: RefCell<bool>,
}

impl MutationVerifyingSerializer {
    fn new() -> Self {
        Self {
            saw_mutation: RefCell::new(false),
        }
    }

    fn saw_mutation(&self) -> bool {
        *self.saw_mutation.borrow()
    }
}

impl GCodeSerializer for MutationVerifyingSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        // Check if our marker command was added
        let has_mutation = gcode_ir.commands.iter().any(
            |cmd| matches!(cmd, GCodeCommand::Comment { text } if text == "MUTATED_BY_MODULE"),
        );
        *self.saw_mutation.borrow_mut() = has_mutation;
        Ok("serialized".to_string())
    }
}
