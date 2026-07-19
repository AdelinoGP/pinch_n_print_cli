#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ConfigKey, ConfigValue, GlobalLayer, LayerCollectionIR, LayerStageCommit, StageId,
};
use slicer_model_io::load_model;
use slicer_runtime::pipeline::{
    run_pipeline_with_raw_config, PipelineConfig, PipelineStageRunners,
};
use slicer_runtime::{
    CompiledModuleLive, DefaultGCodeEmitter, DefaultGCodeSerializer, ExecutionPlan,
    FinalizationError, FinalizationOutput, FinalizationStageInput, FinalizationStageRunner,
    LayerStageError, LayerStageInput, LayerStageRunner, NoopLayerProgressSink, PostpassError,
    PostpassOutput, PostpassStageInput, PostpassStageRunner, PrepassRunnerError, PrepassStageInput,
    PrepassStageOutput, PrepassStageRunner,
};

fn empty_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

struct NoopPrepassRunner;
impl PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        Ok(PrepassStageOutput::None)
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

fn gcode_with_flavor(flavor: &str) -> String {
    gcode_with_optional_flavor(Some(flavor))
}

fn gcode_with_optional_flavor(flavor: Option<&str>) -> String {
    let mesh_ir = Arc::new(
        load_model(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../resources/test_stl/ASCII/20mmbox-LF.stl"),
        )
        .expect("fixture load"),
    );
    let mut raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    if let Some(flavor) = flavor {
        raw.insert(
            "gcode_flavor".to_string(),
            ConfigValue::String(flavor.to_string()),
        );
    }

    let config = PipelineConfig {
        mesh_ir,
        plan: empty_plan(),
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(DefaultGCodeEmitter::new("pnp_cli-test 0.1.0".into())),
            serializer: Box::new(DefaultGCodeSerializer::new()),
        },
        resolved_configs: Arc::new(BTreeMap::new()),
        default_resolved_config: Arc::new(slicer_ir::ResolvedConfig::default()),
        bounds: Arc::new(slicer_runtime::ConfigBoundsIndex::empty()),
        wasm_handles: Default::default(),
        cancel_flag: None,
    };

    run_pipeline_with_raw_config(config, &raw, &NoopLayerProgressSink)
        .expect("pipeline should succeed")
        .gcode_text
}

#[test]
fn gcode_flavor_config_block() {
    let gcode = gcode_with_flavor("klipper");
    let klipper_line = "; gcode_flavor = klipper";
    let block_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present")
        + "; CONFIG_BLOCK_START".len();
    let block_end = gcode
        .find("; CONFIG_BLOCK_END")
        .expect("CONFIG_BLOCK_END must be present");
    let config_block = &gcode[block_start..block_end];

    assert_eq!(config_block.matches(klipper_line).count(), 1);
    assert!(!config_block.contains("; gcode_flavor = marlin"));
    assert!(gcode.contains("; CONFIG_BLOCK_START"));
    assert!(gcode.contains("; CONFIG_BLOCK_END"));
}

#[test]
fn default_gcode_flavor_config_block_is_marlin() {
    let gcode = gcode_with_optional_flavor(None);
    let block_start = gcode
        .find("; CONFIG_BLOCK_START")
        .expect("CONFIG_BLOCK_START must be present")
        + "; CONFIG_BLOCK_START".len();
    let block_end = gcode
        .find("; CONFIG_BLOCK_END")
        .expect("CONFIG_BLOCK_END must be present");
    let config_block = &gcode[block_start..block_end];

    assert_eq!(config_block.matches("; gcode_flavor = marlin").count(), 1);
}
