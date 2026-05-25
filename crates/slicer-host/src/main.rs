//! Binary entry point for the slicer-host runtime.
//!
//! Parses CLI arguments via clap and dispatches to the pipeline orchestration
//! or config-schema query functions.

mod helpers_cmd;

#[global_allocator]
static ALLOC: AccountingAllocator<std::alloc::System> =
    AccountingAllocator::new(std::alloc::System);

use std::sync::{Arc, Mutex};

use clap::Parser;
use slicer_host::dispatch::WasmRuntimeDispatcher;
use slicer_host::model_loader::load_model;
use slicer_host::pipeline::{
    run_pipeline_with_instrumentation, run_pipeline_with_raw_config, PipelineConfig,
    PipelineStageRunners,
};
use slicer_host::progress_events::{
    JsonLinesEmitter, ProgressEventEmitter, RuntimeProgressSink, SliceEventCollector,
};
use slicer_host::report::{allocator as report_alloc, AccountingAllocator, Collector};
use slicer_host::{
    assemble_search_roots, build_config_schema_json, build_live_execution_plan,
    load_live_modules_for_plan, load_modules_from_roots, parse_cli_config_source,
    resolve_global_config, resolve_per_object_configs, write_with_parents, ConfigBoundsIndex,
    DefaultGCodeEmitter, DefaultGCodeSerializer, HostCli, HostCommands, HostRunOptions,
};

/// No-op prepass runner for MVP (no WASM modules loaded yet).
#[allow(dead_code)]
struct NoopPrepassRunner;
impl slicer_host::PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
    ) -> Result<(slicer_host::PrepassStageOutput, Vec<String>), slicer_host::PrepassExecutionError>
    {
        Ok((slicer_host::PrepassStageOutput::None, Vec::new()))
    }
}

/// No-op layer runner for MVP.
#[allow(dead_code)]
struct NoopLayerRunner;
impl slicer_host::LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        _layer: &slicer_ir::GlobalLayer,
        _module: &slicer_host::CompiledModule,
        _blackboard: &slicer_host::Blackboard,
        _arena: &mut slicer_host::LayerArena,
    ) -> Result<
        (slicer_host::LayerStageOutput, Vec<String>, Vec<String>),
        slicer_host::LayerStageError,
    > {
        Ok((
            slicer_host::LayerStageOutput::Success,
            Vec::new(),
            Vec::new(),
        ))
    }
}

/// No-op finalization runner for MVP.
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Conservative default for host parallelism when building instance pools
/// for `layer-parallel-safe` modules. The scheduler clamps to `>= 1`, so
/// keeping this at the process's logical-core count (falling back to 1)
/// is safe without pulling in a `num_cpus` dependency.
fn num_cpus_guess() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn main() {
    let cli = HostCli::parse();
    match cli.command {
        HostCommands::Run {
            model,
            config,
            output,
            module_dir,
            no_default_module_paths,
            thumbnail,
            report,
            report_verbose,
        } => {
            // Inline existence checks before constructing HostRunOptions.
            if !model.exists() {
                eprintln!("error: model file not found: {}", model.display());
                std::process::exit(1);
            }
            if let Some(ref cfg) = config {
                if !cfg.exists() {
                    eprintln!("error: config file not found: {}", cfg.display());
                    std::process::exit(1);
                }
            }

            let opts = HostRunOptions {
                model_path: model,
                config_path: config,
                output_path: output,
                module_dirs: module_dir,
                no_default_module_paths,
                thumbnail,
                report,
                report_verbose,
            };

            // Load model
            let mesh_ir = match load_model(&opts.model_path) {
                Ok(ir) => Arc::new(ir),
                Err(e) => {
                    eprintln!("error: failed to load model: {e}");
                    std::process::exit(1);
                }
            };

            // Parse the user-facing JSON config (if supplied) into the raw
            // `HashMap<ConfigKey, ConfigValue>` that every per-module
            // `Arc<ConfigView>` will be filtered from via
            // `bind_module_config_view` inside `build_live_execution_plan`.
            // Passing an empty source here is explicitly NOT a placeholder:
            // it's the real config source when the user doesn't supply one.
            let mut config_source = match opts.config_path.as_ref() {
                Some(path) => match std::fs::read_to_string(path) {
                    Ok(text) => match parse_cli_config_source(&text) {
                        Ok(map) => map,
                        Err(e) => {
                            eprintln!("error: failed to parse --config: {e}");
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("error: failed to read --config file: {e}");
                        std::process::exit(1);
                    }
                },
                None => std::collections::HashMap::new(),
            };

            // Insert thumbnail_path into config_source when --thumbnail is supplied.
            if let Some(ref thumb_path) = opts.thumbnail {
                config_source.insert(
                    "thumbnail_path".to_string(),
                    slicer_ir::ConfigValue::String(thumb_path.to_string_lossy().to_string()),
                );
            }

            // Seed planner-visible per-object world heights from the cached
            // `ObjectMesh.world_z_extent` field before module binding.
            for object in &mesh_ir.objects {
                let key = format!("object_height:{}", object.id);
                if config_source.contains_key(&key) {
                    continue;
                }
                if let Some((z_min, z_max)) = object.world_z_extent {
                    config_source
                        .insert(key, slicer_ir::ConfigValue::Float((z_max - z_min) as f64));
                }
            }

            // Seed planner-visible per-object config from `ObjectMesh.config.data`
            // (populated by the 3MF loader from object-scoped sidecar metadata).
            // `resolve_per_object_configs` consumes the `object_config:<id>:<key>`
            // prefix. CLI overrides set in `config_source` already win.
            for object in &mesh_ir.objects {
                for (subkey, value) in &object.config.data {
                    let key = format!("object_config:{}:{}", object.id, subkey);
                    if config_source.contains_key(&key) {
                        continue;
                    }
                    config_source.insert(key, value.clone());
                }
            }

            // Discover and plan every module under --module-dir.
            let search_roots =
                assemble_search_roots(&opts.module_dirs, opts.no_default_module_paths);
            let loaded = match load_live_modules_for_plan(&search_roots, num_cpus_guess()) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!(
                        "error: failed to load modules from {} root(s) {:?}: {e}",
                        search_roots.len(),
                        search_roots,
                    );
                    std::process::exit(1);
                }
            };
            for diag in &loaded.diagnostics {
                eprintln!(
                    "{level:?}: {path}: {msg}",
                    level = diag.level,
                    path = diag.path.display(),
                    msg = diag.message,
                );
            }
            let config_bounds =
                ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));

            let default_resolved_config =
                match resolve_global_config(&config_source, &config_bounds) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("error: config resolution failed: {e}");
                        std::process::exit(1);
                    }
                };
            let object_ids: Vec<&str> = mesh_ir.objects.iter().map(|o| o.id.as_str()).collect();
            let resolved_configs_map = match resolve_per_object_configs(
                &default_resolved_config,
                &config_source,
                &object_ids,
                &config_bounds,
            ) {
                Ok(map) => map,
                Err(e) => {
                    eprintln!("error: config resolution failed: {e}");
                    std::process::exit(1);
                }
            };

            let plan = match build_live_execution_plan(
                loaded.sorted_stages,
                loaded.bindings,
                &config_source,
                Arc::new(Vec::new()),
                Arc::new(std::collections::HashMap::new()),
            ) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("error: failed to build execution plan: {e}");
                    std::process::exit(1);
                }
            };

            let engine = Arc::clone(&loaded.engine);
            let config = PipelineConfig {
                mesh_ir,
                plan,
                runners: PipelineStageRunners {
                    prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
                    emitter: Box::new(
                        DefaultGCodeEmitter::new("slicer-host 0.1.0".into())
                            .with_resolved_config(default_resolved_config.clone()),
                    ),
                    serializer: {
                        let relative = match config_source.get("use_relative_e_distances") {
                            Some(slicer_ir::ConfigValue::Bool(b)) => *b,
                            _ => true,
                        };
                        Box::new(DefaultGCodeSerializer::with_extrusion_mode(relative))
                    },
                },
                resolved_configs: Arc::new(resolved_configs_map),
                default_resolved_config: Arc::new(default_resolved_config),
                bounds: Arc::new(config_bounds),
            };

            let emitter: Arc<dyn ProgressEventEmitter> =
                Arc::new(JsonLinesEmitter::new(std::io::stderr()));
            let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
            let sink = RuntimeProgressSink::new(emitter, Arc::clone(&collector));

            let result = if let Some(ref report_path) = opts.report {
                if let Some(parent) = report_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!(
                            "warning: failed to create report parent directory {}: {e}",
                            parent.display()
                        );
                    }
                }
                report_alloc::enable();
                let collector = Arc::new(Collector::new_with_verbose(
                    opts.model_path.to_string_lossy().to_string(),
                    opts.report_verbose,
                ));
                let r = run_pipeline_with_instrumentation(
                    config,
                    &config_source,
                    &sink,
                    collector.as_ref(),
                );
                report_alloc::disable();
                if let Err(e) = collector.finish_and_render_to(report_path) {
                    eprintln!("warning: failed to write slicer report: {e}");
                }
                r
            } else {
                run_pipeline_with_raw_config(config, &config_source, &sink)
            };

            match result {
                Ok(result) => {
                    if let Some(out_path) = opts.output_path {
                        if let Err(e) = write_with_parents(&out_path, result.gcode_text.as_bytes())
                        {
                            eprintln!("error: failed to write output {}: {e}", out_path.display());
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
        HostCommands::ConfigSchema {
            module_dir,
            no_default_module_paths,
        } => {
            let search_roots = assemble_search_roots(&module_dir, no_default_module_paths);
            let report = match load_modules_from_roots(&search_roots) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error loading modules: {e:?}");
                    std::process::exit(1);
                }
            };
            let json = build_config_schema_json(&report.modules);
            println!("{}", json);
        }
        HostCommands::Repair {
            input,
            output,
            format,
            stats,
        } => {
            std::process::exit(helpers_cmd::run_repair(&input, &output, format, stats));
        }
        HostCommands::Decimate {
            input,
            output,
            target_count,
            target_ratio,
            max_error,
            aggressive,
            stats,
        } => {
            std::process::exit(helpers_cmd::run_decimate(
                &input,
                &output,
                target_count,
                target_ratio,
                max_error,
                aggressive,
                stats,
            ));
        }
        HostCommands::Import {
            input,
            output,
            output_format,
            merge_components,
            no_repair,
            stats,
        } => {
            std::process::exit(helpers_cmd::run_import(
                &input,
                &output,
                output_format,
                merge_components,
                no_repair,
                stats,
            ));
        }
    }
}

#[cfg(any())]
mod _stale_build_plan {
    /// Build an ExecutionPlan from loaded modules with compiled WASM components.
    ///                                                                                                                                                                                                                           
    /// For each loaded module, reads the companion .wasm file, compiles it to a                                                                                                                                                  
    /// WASM component via the shared engine, and stores it in the CompiledModule.                                                                                                                                         
    /// Compilation failures are reported as diagnostics (stderr) but do not prevent                                                                                                                                       
    /// plan construction — the dispatcher will surface a structured error at call time.                                                                                                                                   
    fn build_plan_from_loaded_modules(
        modules: &[slicer_host::LoadedModule],
        engine: &Arc<WasmEngine>,
    ) -> slicer_host::ExecutionPlan {
        use slicer_host::{
            build_wasm_instance_pool, CompiledModule, CompiledModuleBuilder, CompiledStage,
            ExecutionPlan, IrAccessMask, WasmArtifactMetadata,
        };

        let mut prepass_stages: Vec<CompiledStage> = Vec::new();
        let mut per_layer_stages: Vec<CompiledStage> = Vec::new();
        let mut layer_finalization_stage: Option<CompiledStage> = None;
        let mut postpass_stages: Vec<CompiledStage> = Vec::new();

        // Group modules by stage
        let mut stage_groups: std::collections::BTreeMap<String, Vec<&slicer_host::LoadedModule>> =
            std::collections::BTreeMap::new();
        for module in modules {
            stage_groups
                .entry(module.stage.clone())
                .or_default()
                .push(module);
        }

        // Build compiled stages per group
        for (stage_id, stage_modules) in &stage_groups {
            let compiled_modules: Vec<CompiledModule> = stage_modules
                .iter()
                .map(|m| {
                    let parallelism = if m.layer_parallel_safe { 4 } else { 1 };
                    let pool = Arc::new(
                        build_wasm_instance_pool(
                            m,
                            parallelism,
                            WasmArtifactMetadata {
                                uses_shared_memory: false,
                            },
                        )
                        .expect("pool build should succeed"),
                    );

                    // Compile the WASM component from the module's .wasm file.
                    let wasm_component = match std::fs::read(&m.wasm_path) {
                        Ok(bytes) => match engine.compile_component(&bytes) {
                            Ok(component) => Some(Arc::new(component)),
                            Err(e) => {
                                eprintln!(
                                    "warning: failed to compile WASM for module '{}': {}",
                                    m.id, e
                                );
                                None
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "warning: failed to read WASM file '{}' for module '{}': {}",
                                m.wasm_path.display(),
                                m.id,
                                e
                            );
                            None
                        }
                    };

                    CompiledModuleBuilder::new(m.id.clone(), pool)
                        .ir_read_mask(IrAccessMask {
                            paths: m.ir_reads.clone(),
                        })
                        .ir_write_mask(IrAccessMask {
                            paths: m.ir_writes.clone(),
                        })
                        .claims(m.claims.clone())
                        .wasm_component(wasm_component)
                        .requires_modules(m.requires_modules.clone())
                        .build()
                })
                .collect();

            let compiled_stage = CompiledStage {
                stage_id: stage_id.clone(),
                modules: compiled_modules,
            };

            if stage_id.starts_with("PrePass::") {
                prepass_stages.push(compiled_stage);
            } else if stage_id.starts_with("Layer::") {
                per_layer_stages.push(compiled_stage);
            } else if stage_id == "PostPass::LayerFinalization" {
                layer_finalization_stage = Some(compiled_stage);
            } else if stage_id.starts_with("PostPass::") {
                postpass_stages.push(compiled_stage);
            }
        }

        ExecutionPlan {
            prepass_stages,
            per_layer_stages,
            layer_finalization_stage,
            postpass_stages,
            global_layers: Arc::new(Vec::new()),
            region_plans: Arc::new(HashMap::new()),
        }
    }
}
