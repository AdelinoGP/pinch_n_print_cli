//! TDD tests for runtime wiring — manifest → plan → pipeline integration.
//!
//! These tests verify that the host can discover modules from manifests,
//! build an execution plan, and run the pipeline with real module metadata.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slicer_host::pipeline::{run_pipeline, PipelineConfig, PipelineStageRunners};
use slicer_host::{
    build_config_schema_json, build_execution_plan, build_wasm_instance_pool,
    load_modules_from_roots, Blackboard, CompiledModule, CompiledStage, ExecutionModuleBinding,
    ExecutionPlan, ExecutionPlanRequest, FinalizationError, FinalizationOutput,
    FinalizationStageRunner, GCodeEmitter, GCodeSerializer, IrAccessMask, LayerArena,
    LayerStageError, LayerStageOutput, LayerStageRunner, PostpassError, PostpassOutput,
    PostpassStageRunner, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner,
    SortedStageModules, WasmArtifactMetadata,
};
use slicer_ir::{
    BoundingBox3, ConfigView, GCodeIR, GlobalLayer, LayerCollectionIR, MeshIR, Point3,
    PrintMetadata, SemVer, StageId,
};
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────

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
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 0.0, y: 0.0, z: 0.0 },
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
    ) -> Result<(LayerStageOutput, Vec<String>), LayerStageError> {
        Ok((LayerStageOutput::Success, Vec::new()))
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
    ) -> Result<PostpassOutput, PostpassError> {
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

fn write_module_fixture(
    root: &Path,
    dir_name: &str,
    stem: &str,
    module_id: &str,
    stage: &str,
    wit_world: &str,
    parallel_safe: bool,
) {
    let subdir = root.join(dir_name);
    fs::create_dir_all(&subdir).unwrap();
    let manifest = format!(
        r#"
[module]
id = "{module_id}"
version = "1.0.0"
display-name = "Test Module"
description = "fixture"
author = "test"
license = "MIT"
wit-world = "{wit_world}"

[stage]
id = "{stage}"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema]

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = {parallel_safe}
"#
    );
    fs::write(subdir.join(format!("{stem}.toml")), manifest).unwrap();
    fs::write(subdir.join(format!("{stem}.wasm")), b"\x00asm\x01\x00\x00\x00").unwrap();
}

// ── Tests ────────────────────────────────────────────────────────────────

#[test]
fn manifest_driven_plan_has_correct_stage_buckets() {
    let tmp = TempDir::new().unwrap();

    write_module_fixture(
        tmp.path(), "infill-mod", "infill-mod",
        "com.test.infill", "Layer::Infill", "slicer:world-layer@1.0.0", true,
    );
    write_module_fixture(
        tmp.path(), "support-mod", "support-mod",
        "com.test.support", "Layer::Support", "slicer:world-layer@1.0.0", true,
    );
    write_module_fixture(
        tmp.path(), "mesh-mod", "mesh-mod",
        "com.test.mesh", "PrePass::MeshAnalysis", "slicer:world-prepass@1.0.0", true,
    );
    write_module_fixture(
        tmp.path(), "wipe-mod", "wipe-mod",
        "com.test.wipe", "PostPass::LayerFinalization", "slicer:world-finalization@1.0.0", false,
    );

    let report = load_modules_from_roots(&[tmp.path().to_path_buf()]).unwrap();
    assert_eq!(report.modules.len(), 4);

    // Build plan using build_execution_plan
    let sorted_stages = vec![
        SortedStageModules {
            stage_id: "PrePass::MeshAnalysis".into(),
            module_ids: vec!["com.test.mesh".into()],
        },
        SortedStageModules {
            stage_id: "Layer::Infill".into(),
            module_ids: vec!["com.test.infill".into()],
        },
        SortedStageModules {
            stage_id: "Layer::Support".into(),
            module_ids: vec!["com.test.support".into()],
        },
        SortedStageModules {
            stage_id: "PostPass::LayerFinalization".into(),
            module_ids: vec!["com.test.wipe".into()],
        },
    ];

    let bindings: Vec<ExecutionModuleBinding> = report
        .modules
        .iter()
        .map(|m| {
            let parallelism = if m.layer_parallel_safe { 4 } else { 1 };
            let pool = Arc::new(
                build_wasm_instance_pool(
                    m,
                    parallelism,
                    WasmArtifactMetadata { uses_shared_memory: false },
                )
                .unwrap(),
            );
            ExecutionModuleBinding {
                module: m.clone(),
                instance_pool: pool,
                config_view: Arc::new(ConfigView::from_map(HashMap::new())),
                wasm_component: None,
            }
        })
        .collect();

    let request = ExecutionPlanRequest {
        sorted_stages,
        module_bindings: bindings,
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
    };

    let plan = build_execution_plan(&request).unwrap();

    assert_eq!(plan.prepass_stages.len(), 1);
    assert_eq!(plan.prepass_stages[0].stage_id, "PrePass::MeshAnalysis");

    assert_eq!(plan.per_layer_stages.len(), 2);
    assert_eq!(plan.per_layer_stages[0].stage_id, "Layer::Infill");
    assert_eq!(plan.per_layer_stages[1].stage_id, "Layer::Support");

    assert!(plan.layer_finalization_stage.is_some());
    assert_eq!(
        plan.layer_finalization_stage.as_ref().unwrap().stage_id,
        "PostPass::LayerFinalization"
    );

    assert!(plan.postpass_stages.is_empty());
}

#[test]
fn manifest_driven_pipeline_runs_to_completion() {
    let tmp = TempDir::new().unwrap();

    write_module_fixture(
        tmp.path(), "infill-mod", "infill-mod",
        "com.test.infill", "Layer::Infill", "slicer:world-layer@1.0.0", true,
    );

    let report = load_modules_from_roots(&[tmp.path().to_path_buf()]).unwrap();
    assert_eq!(report.modules.len(), 1);

    // Build a minimal plan with the loaded module
    let m = &report.modules[0];
    let pool = Arc::new(
        build_wasm_instance_pool(m, 4, WasmArtifactMetadata { uses_shared_memory: false }).unwrap(),
    );
    let compiled_module = CompiledModule {
        module_id: m.id.clone(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: m.ir_reads.clone() },
        ir_write_mask: IrAccessMask { paths: m.ir_writes.clone() },
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
        wasm_component: None,
    };

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![compiled_module],
        }],
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
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "manifest-driven pipeline should complete: {:?}", result.err());
}

#[test]
fn config_schema_json_is_empty_array_for_modules_without_config() {
    let tmp = TempDir::new().unwrap();

    write_module_fixture(
        tmp.path(), "infill-mod", "infill-mod",
        "com.test.infill", "Layer::Infill", "slicer:world-layer@1.0.0", true,
    );

    let report = load_modules_from_roots(&[tmp.path().to_path_buf()]).unwrap();
    let json = build_config_schema_json(&report.modules);

    let schema = json.get("schema").expect("response must have 'schema' key");
    assert!(schema.is_array());
    // Modules with empty config.schema produce no entries
    assert_eq!(schema.as_array().unwrap().len(), 0);
}

#[test]
fn config_schema_json_includes_modules_with_config_fields() {
    let tmp = TempDir::new().unwrap();
    let subdir = tmp.path().join("my-mod");
    fs::create_dir_all(&subdir).unwrap();

    let manifest = r#"
[module]
id = "com.test.configured"
version = "1.0.0"
display-name = "Configured Module"
description = "has config"
author = "test"
license = "MIT"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema.density]
type = "float"
default = 0.15
min = 0.05
max = 0.95
display = "Infill Density"
group = "Pattern"

[config.schema.pattern]
type = "enum"
values = ["rectilinear", "gyroid"]
default = "rectilinear"
display = "Pattern Type"
group = "Pattern"

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
estimated-ms-per-layer = 10
layer-parallel-safe = true
"#;
    fs::write(subdir.join("my-mod.toml"), manifest).unwrap();
    fs::write(subdir.join("my-mod.wasm"), b"\x00asm\x01\x00\x00\x00").unwrap();

    let report = load_modules_from_roots(&[tmp.path().to_path_buf()]).unwrap();
    assert_eq!(report.modules.len(), 1);

    let json = build_config_schema_json(&report.modules);
    let schema = json.get("schema").unwrap().as_array().unwrap();
    assert_eq!(schema.len(), 1, "module with config should appear in schema");
    assert_eq!(schema[0]["module"], "com.test.configured");
    let fields = schema[0]["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2, "should have 2 config fields");

    // Verify field keys are present
    let keys: Vec<&str> = fields.iter().map(|f| f["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"density"));
    assert!(keys.contains(&"pattern"));
}

#[test]
fn config_schema_json_matches_documented_shape() {
    // Per docs/01_system_architecture.md, the response shape is:
    // {"schema": [{"module": "...", "fields": [{"key": "...", "type": "..."}]}]}
    let json = build_config_schema_json(&[]);
    assert!(json.get("schema").is_some(), "response must have 'schema' key");
    assert!(json["schema"].is_array(), "'schema' must be an array");
    assert_eq!(json["schema"].as_array().unwrap().len(), 0, "empty modules = empty schema array");
}

#[test]
fn core_modules_build_a_multi_tier_execution_plan() {
    let core_modules_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root]).unwrap();

    // Group by stage prefix to verify tier coverage
    let prepass_count = report.modules.iter().filter(|m| m.stage.starts_with("PrePass::")).count();
    let layer_count = report.modules.iter().filter(|m| m.stage.starts_with("Layer::")).count();
    let finalization_count = report.modules.iter().filter(|m| m.stage == "PostPass::LayerFinalization").count();
    let postpass_count = report.modules.iter().filter(|m| {
        m.stage.starts_with("PostPass::") && m.stage != "PostPass::LayerFinalization"
    }).count();

    assert!(prepass_count >= 2, "should have prepass modules, got {prepass_count}");
    assert!(layer_count >= 5, "should have layer modules, got {layer_count}");
    assert!(finalization_count >= 1, "should have finalization modules, got {finalization_count}");
    // postpass modules are optional in core set
    let _ = postpass_count;

    // Verify we can build execution bindings for all of them
    let bindings: Vec<ExecutionModuleBinding> = report
        .modules
        .iter()
        .map(|m| {
            let pool = Arc::new(
                build_wasm_instance_pool(
                    m,
                    if m.layer_parallel_safe { 4 } else { 1 },
                    WasmArtifactMetadata { uses_shared_memory: false },
                )
                .unwrap(),
            );
            ExecutionModuleBinding {
                module: m.clone(),
                instance_pool: pool,
                config_view: Arc::new(ConfigView::from_map(HashMap::new())),
                wasm_component: None,
            }
        })
        .collect();

    assert_eq!(bindings.len(), report.modules.len());
}
