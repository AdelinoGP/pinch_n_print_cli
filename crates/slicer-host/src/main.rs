//! Binary entry point for the slicer-host runtime.
//!
//! Parses CLI arguments via clap and dispatches to the pipeline orchestration
//! or config-schema query functions.

use std::sync::Arc;

use clap::Parser;
use slicer_host::model_loader::load_model;
use slicer_host::pipeline::{run_pipeline, PipelineConfig, PipelineStageRunners};
use slicer_host::{
    DefaultGCodeEmitter, DefaultGCodeSerializer, HostCli, HostCommands,
};

/// No-op prepass runner for MVP (no WASM modules loaded yet).
struct NoopPrepassRunner;
impl slicer_host::PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
    ) -> Result<slicer_host::PrepassStageOutput, slicer_host::PrepassExecutionError> {
        Ok(slicer_host::PrepassStageOutput::None)
    }
}

/// No-op layer runner for MVP.
struct NoopLayerRunner;
impl slicer_host::LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _layer: &slicer_ir::GlobalLayer,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _arena: &mut slicer_host::LayerArena,
    ) -> Result<slicer_host::LayerStageOutput, slicer_host::LayerStageError> {
        Ok(slicer_host::LayerStageOutput::Success)
    }
}

/// No-op finalization runner for MVP.
struct NoopFinalizationRunner;
impl slicer_host::FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _layers: &mut Vec<slicer_ir::LayerCollectionIR>,
    ) -> Result<slicer_host::FinalizationOutput, slicer_host::FinalizationError> {
        Ok(slicer_host::FinalizationOutput::Success)
    }
}

/// No-op postpass runner for MVP.
struct NoopPostpassRunner;
impl slicer_host::PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _gcode_ir: &mut slicer_ir::GCodeIR,
    ) -> Result<slicer_host::PostpassOutput, slicer_host::PostpassError> {
        Ok(slicer_host::PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        text: String,
    ) -> Result<slicer_host::PostpassOutput, slicer_host::PostpassError> {
        Ok(slicer_host::PostpassOutput::TextSuccess { text })
    }
}

fn main() {
    let cli = HostCli::parse();
    match cli.command {
        HostCommands::Run {
            module: _,
            model,
            config: _,
            output,
            module_dir: _,
        } => {
            // Load model
            let model_path = std::path::Path::new(&model);
            let mesh_ir = match load_model(model_path) {
                Ok(ir) => Arc::new(ir),
                Err(e) => {
                    eprintln!("error: failed to load model: {e}");
                    std::process::exit(1);
                }
            };

            // MVP: empty execution plan (no WASM modules)
            let plan = slicer_host::ExecutionPlan {
                prepass_stages: Vec::new(),
                per_layer_stages: Vec::new(),
                layer_finalization_stage: None,
                postpass_stages: Vec::new(),
                global_layers: Arc::new(Vec::new()),
                region_plans: Arc::new(std::collections::HashMap::new()),
            };

            let config = PipelineConfig {
                mesh_ir,
                plan,
                runners: PipelineStageRunners {
                    prepass: Box::new(NoopPrepassRunner),
                    layer: Box::new(NoopLayerRunner),
                    finalization: Box::new(NoopFinalizationRunner),
                    postpass: Box::new(NoopPostpassRunner),
                    emitter: Box::new(DefaultGCodeEmitter::new("slicer-host 0.1.0".into())),
                    serializer: Box::new(DefaultGCodeSerializer::new()),
                },
            };

            match run_pipeline(config) {
                Ok(result) => {
                    if let Some(out_path) = output {
                        if let Err(e) = std::fs::write(&out_path, &result.gcode_text) {
                            eprintln!("error: failed to write output: {e}");
                            std::process::exit(1);
                        }
                    } else {
                        print!("{}", result.gcode_text);
                    }
                }
                Err(e) => {
                    eprintln!("error: pipeline failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        HostCommands::ConfigSchema { module_dir: _ } => {
            // MVP: emit empty JSON object (no modules loaded)
            println!("{{}}");
        }
    }
}
