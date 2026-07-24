//! TDD tests for runtime wiring â€” manifest â†’ plan â†’ pipeline integration.
//!
//! These tests verify that the host can discover modules from manifests,
//! build an execution plan, and run the pipeline with real module metadata.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigView, GCodeIR, GlobalLayer, LayerCollectionIR, LayerPlanIR,
    LayerStageCommit, MeshIR, Point3, PrintMetadata, SemVer, StageId,
};
use slicer_runtime::pipeline::{run_pipeline, PipelineConfig, PipelineStageRunners};
use slicer_runtime::{
    build_config_schema_json, build_execution_plan, build_wasm_instance_pool,
    load_modules_from_roots, CompiledModule, CompiledModuleBuilder, CompiledModuleLive,
    CompiledStage, ExecutionModuleBinding, ExecutionPlan, ExecutionPlanRequest, FinalizationError,
    FinalizationOutput, FinalizationStageInput, FinalizationStageRunner, GCodeEmitError,
    GCodeEmitter, GCodeSerializer, IrAccessMask, LayerStageError, LayerStageInput,
    LayerStageRunner, LoadDiagnostic, LoadedModuleBuilder, PostpassError, PostpassOutput,
    PostpassStageInput, PostpassStageRunner, PrepassRunnerError, PrepassStageInput,
    PrepassStageOutput, PrepassStageRunner, SortedStageModules, WasmArtifactMetadata,
};
use tempfile::TempDir;

// â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn make_dummy_compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver(1, 0, 0),
        stage_id,
        slicer_schema::WORLD_PREPASS,
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
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
        .expect("fixture module should build a pool"),
    );
    CompiledModuleBuilder::new(module_id).build()
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

struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: FinalizationStageInput<'_>,
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
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        _commands: &mut Vec<slicer_ir::GCodeCommand>,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }
    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}

struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(&self, _layer_irs: &[LayerCollectionIR]) -> Result<GCodeIR, GCodeEmitError> {
        Ok(minimal_gcode_ir())
    }
}

struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
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
    fs::write(
        subdir.join(format!("{stem}.wasm")),
        b"\x00asm\x01\x00\x00\x00",
    )
    .unwrap();
}

// â”€â”€ Tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn manifest_driven_plan_has_correct_stage_buckets() {
    let tmp = TempDir::new().unwrap();

    write_module_fixture(
        tmp.path(),
        "infill-mod",
        "infill-mod",
        "com.test.infill",
        "Layer::Infill",
        slicer_schema::WORLD_LAYER,
        true,
    );
    write_module_fixture(
        tmp.path(),
        "support-mod",
        "support-mod",
        "com.test.support",
        "Layer::Support",
        slicer_schema::WORLD_LAYER,
        true,
    );
    write_module_fixture(
        tmp.path(),
        "mesh-mod",
        "mesh-mod",
        "com.test.mesh",
        "PrePass::MeshAnalysis",
        slicer_schema::WORLD_PREPASS,
        true,
    );
    write_module_fixture(
        tmp.path(),
        "wipe-mod",
        "wipe-mod",
        "com.test.wipe",
        "PostPass::LayerFinalization",
        slicer_schema::WORLD_FINALIZATION,
        false,
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
            let parallelism = if m.layer_parallel_safe() { 4 } else { 1 };
            let _pool = Arc::new(
                build_wasm_instance_pool(
                    m.id(),
                    m.stage(),
                    m.layer_parallel_safe(),
                    parallelism,
                    WasmArtifactMetadata {
                        uses_shared_memory: false,
                    },
                )
                .unwrap(),
            );
            ExecutionModuleBinding {
                module: m.clone(),
                config_view: Arc::new(ConfigView::from_map(HashMap::new())),
            }
        })
        .collect();

    let request = ExecutionPlanRequest {
        sorted_stages,
        module_bindings: bindings,
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
    };

    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let plan = build_execution_plan(&request, &mut diagnostics).unwrap();

    assert_eq!(plan.prepass_stages.len(), 1);
    assert_eq!(plan.prepass_stages[0].stage_id, "PrePass::MeshAnalysis");

    // `build_execution_plan` auto-injects an always-on
    // `Layer::PaintRegionAnnotation` stage so the host annotator can run
    // before downstream stages need `segment_annotations` (packet-64). Filter
    // it out when validating the user-declared per-layer pipeline shape.
    let user_stage_ids: Vec<&str> = plan
        .per_layer_stages
        .iter()
        .map(|s| s.stage_id.as_str())
        .filter(|s| *s != "Layer::PaintRegionAnnotation")
        .collect();
    assert_eq!(user_stage_ids, vec!["Layer::Infill", "Layer::Support"]);

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
        tmp.path(),
        "infill-mod",
        "infill-mod",
        "com.test.infill",
        "Layer::Infill",
        slicer_schema::WORLD_LAYER,
        true,
    );

    let report = load_modules_from_roots(&[tmp.path().to_path_buf()]).unwrap();
    assert_eq!(report.modules.len(), 1);

    // Build a minimal plan with the loaded module
    let m = &report.modules[0];
    let _pool = Arc::new(
        build_wasm_instance_pool(
            m.id(),
            m.stage(),
            m.layer_parallel_safe(),
            4,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    let compiled_module = CompiledModuleBuilder::new(m.id().to_string())
        .ir_read_mask(IrAccessMask {
            paths: m.ir_reads().to_vec(),
        })
        .ir_write_mask(IrAccessMask {
            paths: m.ir_writes().to_vec(),
        })
        .config_view(Arc::new(ConfigView::from_map(HashMap::new())))
        .build();

    // Prepass runner that seeds slice_ir by emitting a 1-layer LayerPlan so
    // Phase-2 builtins (RegionMapping + Slice) run before per-layer executes.
    struct OneLayerPrepass;
    impl PrepassStageRunner for OneLayerPrepass {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModuleLive<'_>,
            _input: PrepassStageInput<'_>,
        ) -> Result<PrepassStageOutput, PrepassRunnerError> {
            Ok(PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
                global_layers: vec![GlobalLayer {
                    index: 0,
                    z: 0.2,
                    active_regions: Vec::new(),
                    has_nonplanar: false,
                    is_sync_layer: false,
                }],
                ..Default::default()
            })))
        }
    }

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![make_dummy_compiled_module(
                "PrePass::LayerPlanning",
                "layer-planner",
            )],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![compiled_module],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(OneLayerPrepass),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
        resolved_configs: std::sync::Arc::new(std::collections::BTreeMap::new()),
        default_resolved_config: std::sync::Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: std::sync::Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
        support_tools: Default::default(),
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "manifest-driven pipeline should complete: {:?}",
        result.err()
    );
}

#[test]
fn config_schema_json_is_empty_array_for_modules_without_config() {
    let tmp = TempDir::new().unwrap();

    write_module_fixture(
        tmp.path(),
        "infill-mod",
        "infill-mod",
        "com.test.infill",
        "Layer::Infill",
        slicer_schema::WORLD_LAYER,
        true,
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
wit-world = "{world}"

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
step = 0.05
display = "Infill Density"
description = "Fraction of solid coverage"
group = "Pattern"
unit = "ratio"
advanced = true
tags = ["infill", "advanced"]

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
"#
    .replace("{world}", slicer_schema::WORLD_LAYER);
    fs::write(subdir.join("my-mod.toml"), &manifest).unwrap();
    fs::write(subdir.join("my-mod.wasm"), b"\x00asm\x01\x00\x00\x00").unwrap();

    let report = load_modules_from_roots(&[tmp.path().to_path_buf()]).unwrap();
    assert_eq!(report.modules.len(), 1);

    let json = build_config_schema_json(&report.modules);
    let schema = json.get("schema").unwrap().as_array().unwrap();
    assert_eq!(
        schema.len(),
        1,
        "module with config should appear in schema"
    );
    assert_eq!(schema[0]["module"], "com.test.configured");
    let fields = schema[0]["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2, "should have 2 config fields");

    // Verify field keys are present
    let keys: Vec<&str> = fields.iter().map(|f| f["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"density"));
    assert!(keys.contains(&"pattern"));

    // Round-trip per-field keys added in the wire-shape backfill: every field
    // emits the full key set, with TOML values surfacing on the wire.
    let density = fields
        .iter()
        .find(|f| f["key"] == "density")
        .expect("density field must exist");
    assert_eq!(density["step"], 0.05);
    assert_eq!(density["description"], "Fraction of solid coverage");
    assert_eq!(density["unit"], "ratio");
    assert_eq!(density["advanced"], true);
    assert_eq!(density["tags"], serde_json::json!(["infill", "advanced"]));

    let pattern = fields
        .iter()
        .find(|f| f["key"] == "pattern")
        .expect("pattern field must exist");
    // Untagged fields emit [] (never null, never absent).
    assert!(pattern["tags"].is_array());
    assert_eq!(pattern["tags"].as_array().unwrap().len(), 0);
    assert_eq!(
        pattern["values"],
        serde_json::json!(["rectilinear", "gyroid"]),
        "enum values must round-trip from TOML into the JSON wire"
    );
    // Optional keys absent in TOML emit as JSON null.
    assert!(pattern["unit"].is_null());
    assert!(pattern["description"].is_null());
    assert_eq!(pattern["advanced"], false);
}

#[test]
fn config_schema_json_matches_documented_shape() {
    // Per docs/01_system_architecture.md, the response shape is:
    // {"schema_version": "1.0.0",
    //  "schema": [{"module": "...", "fields": [{"key": "...", "type": "..."}]}]}
    let json = build_config_schema_json(&[]);
    assert_eq!(
        json["schema_version"].as_str(),
        Some("1.0.0"),
        "top-level schema_version must equal the wire-format constant '1.0.0'"
    );
    assert!(
        json.get("schema").is_some(),
        "response must have 'schema' key"
    );
    assert!(json["schema"].is_array(), "'schema' must be an array");
    assert_eq!(
        json["schema"].as_array().unwrap().len(),
        0,
        "empty modules = empty schema array"
    );
}

#[test]
fn core_modules_build_a_multi_tier_execution_plan() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root]).unwrap();

    // Group by stage prefix to verify tier coverage
    let prepass_count = report
        .modules
        .iter()
        .filter(|m| m.stage().starts_with("PrePass::"))
        .count();
    let layer_count = report
        .modules
        .iter()
        .filter(|m| m.stage().starts_with("Layer::"))
        .count();
    let finalization_count = report
        .modules
        .iter()
        .filter(|m| m.stage() == "PostPass::LayerFinalization")
        .count();
    let postpass_count = report
        .modules
        .iter()
        .filter(|m| {
            m.stage().starts_with("PostPass::") && m.stage() != "PostPass::LayerFinalization"
        })
        .count();

    assert!(
        prepass_count >= 2,
        "should have prepass modules, got {prepass_count}"
    );
    assert!(
        layer_count >= 5,
        "should have layer modules, got {layer_count}"
    );
    assert!(
        finalization_count >= 1,
        "should have finalization modules, got {finalization_count}"
    );
    // postpass modules are optional in core set
    let _ = postpass_count;

    // Verify we can build execution bindings for all of them
    let bindings: Vec<ExecutionModuleBinding> = report
        .modules
        .iter()
        .map(|m| {
            let _pool = Arc::new(
                build_wasm_instance_pool(
                    m.id(),
                    m.stage(),
                    m.layer_parallel_safe(),
                    if m.layer_parallel_safe() { 4 } else { 1 },
                    WasmArtifactMetadata {
                        uses_shared_memory: false,
                    },
                )
                .unwrap(),
            );
            ExecutionModuleBinding {
                module: m.clone(),
                config_view: Arc::new(ConfigView::from_map(HashMap::new())),
            }
        })
        .collect();

    assert_eq!(bindings.len(), report.modules.len());
}
