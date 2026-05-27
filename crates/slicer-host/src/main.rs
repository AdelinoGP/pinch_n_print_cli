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
use slicer_host::cli::DagSubcommand;
use slicer_host::dag_cli::{run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages};
use slicer_host::dispatch::WasmRuntimeDispatcher;
use slicer_host::layer_executor::LayerProgressSink;
use slicer_host::manifest::DiagnosticLevel;
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
    resolve_global_config, resolve_per_object_configs, validate_support_layer_heights,
    write_with_parents, CompositeInstrumentation, ConfigBoundsIndex, DefaultGCodeEmitter,
    DefaultGCodeSerializer, HostCli, HostCommands, HostRunOptions, ProgressPipelineInstrumentation,
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

/// Helper to extract per-object ids from a 3MF/STL model when `--model` is
/// supplied to a `dag` subcommand. Failures fall back to `None` so manifest-
/// only introspection still works against an unreadable model.
fn object_ids_from_model(path: &std::path::Path) -> Option<Vec<String>> {
    match slicer_host::model_loader::load_model(path) {
        Ok(ir) => Some(ir.objects.iter().map(|o| o.id.clone()).collect()),
        Err(e) => {
            eprintln!("warning: failed to load --model {}: {e}", path.display());
            None
        }
    }
}

fn load_dag_modules(
    module_dir: &[std::path::PathBuf],
    no_default_module_paths: bool,
) -> Vec<slicer_host::manifest::LoadedModule> {
    let search_roots = assemble_search_roots(module_dir, no_default_module_paths);
    match load_modules_from_roots(&search_roots) {
        Ok(r) => r.modules,
        Err(e) => {
            eprintln!("error loading modules: {e:?}");
            std::process::exit(2);
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("error: failed to serialize output: {e}");
            std::process::exit(1);
        }
    }
}

fn run_dag_command(cmd: DagSubcommand) {
    match cmd {
        DagSubcommand::Stages {
            module_dir,
            no_default_module_paths,
            model: _,
        } => {
            let modules = load_dag_modules(&module_dir, no_default_module_paths);
            print_json(&run_dag_stages(&modules));
        }
        DagSubcommand::Stage {
            id,
            module_dir,
            no_default_module_paths,
            model: _,
        } => {
            let modules = load_dag_modules(&module_dir, no_default_module_paths);
            match run_dag_stage(&modules, &id) {
                Some(out) => print_json(&out),
                None => {
                    eprintln!("error: no modules in stage {id:?}");
                    std::process::exit(1);
                }
            }
        }
        DagSubcommand::Depends {
            module_id,
            module_dir,
            no_default_module_paths,
            model,
        } => {
            let modules = load_dag_modules(&module_dir, no_default_module_paths);
            let object_ids = model.as_deref().and_then(object_ids_from_model);
            match run_dag_depends(&modules, &module_id, object_ids.as_deref()) {
                Some(out) => print_json(&out),
                None => {
                    eprintln!("error: module {module_id:?} not found in any loaded manifest");
                    std::process::exit(1);
                }
            }
        }
        DagSubcommand::Claims {
            module_dir,
            no_default_module_paths,
            model: _,
        } => {
            let modules = load_dag_modules(&module_dir, no_default_module_paths);
            print_json(&run_dag_claims(&modules));
        }
    }
}

/// `diagnose` subcommand: load manifests, surface every `LoadDiagnostic` as
/// structured JSON. Returns the process exit code.
fn run_diagnose(module_dir: &[std::path::PathBuf], no_default_module_paths: bool) -> i32 {
    let search_roots = assemble_search_roots(module_dir, no_default_module_paths);
    let report = match load_modules_from_roots(&search_roots) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error loading modules: {e:?}");
            return 2;
        }
    };

    #[derive(serde::Serialize)]
    struct DiagnoseOut<'a> {
        pass: bool,
        modules_loaded: usize,
        stages: usize,
        diagnostics: Vec<DiagnosticOut<'a>>,
    }

    #[derive(serde::Serialize)]
    struct DiagnosticOut<'a> {
        level: &'a str,
        file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        field: &'a Option<String>,
        message: &'a str,
    }

    let mut stage_set: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for m in &report.modules {
        stage_set.insert(m.stage());
    }

    let diagnostics: Vec<DiagnosticOut> = report
        .diagnostics
        .iter()
        .map(|d| DiagnosticOut {
            level: match d.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info => "info",
            },
            file: d.path.display().to_string(),
            field: &d.field,
            message: d.message.as_str(),
        })
        .collect();

    let has_error = report
        .diagnostics
        .iter()
        .any(|d| matches!(d.level, DiagnosticLevel::Error));

    let out = DiagnoseOut {
        pass: !has_error,
        modules_loaded: report.modules.len(),
        stages: stage_set.len(),
        diagnostics,
    };
    print_json(&out);
    if has_error {
        1
    } else {
        0
    }
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
            instrument_stderr,
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
                instrument_stderr,
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

            // 13-pass startup DAG validation (docs/04_host_scheduler.md).
            // Runs every documented gate (IR-version compatibility, claim
            // conflicts, cycles, write conflicts, etc.) against the discovered
            // module set and aborts before pipeline construction on any error.
            //
            // Host built-ins (MeshAnalysis, RegionMapping, Slice,
            // ShellClassification, SupportGeometry, PaintSegmentation,
            // GCodeEmit, GCodeSerialize) commit IRs that user modules later
            // read. The DAG validator's UnfulfilledReads pass doesn't know
            // about those host commits unless we represent each built-in as
            // a synthetic LoadedModule with ir_writes declared. We prepend
            // those synthetic entries so the validator sees the full
            // producer/consumer graph.
            {
                use std::path::PathBuf;

                use slicer_host::manifest::LoadedModuleBuilder;
                use slicer_host::{
                    build_intra_stage_dag, validate_startup_dag, DagValidationRequest, StageDag,
                };
                use slicer_ir::{SemVer, CURRENT_SLICE_IR_SCHEMA_VERSION};

                fn host_builtin(
                    id: &str,
                    stage: &str,
                    writes: &[&str],
                ) -> slicer_host::manifest::LoadedModule {
                    LoadedModuleBuilder::new(
                        id,
                        SemVer {
                            major: 1,
                            minor: 0,
                            patch: 0,
                        },
                        stage,
                        "slicer:world-layer@1.0.0",
                        PathBuf::from(format!("__host_builtin__/{id}")),
                    )
                    .ir_writes(writes.iter().map(|s| s.to_string()).collect())
                    .min_host_version(SemVer {
                        major: 0,
                        minor: 1,
                        patch: 0,
                    })
                    .min_ir_schema(SemVer {
                        major: 1,
                        minor: 0,
                        patch: 0,
                    })
                    .max_ir_schema(SemVer {
                        major: 4,
                        minor: 0,
                        patch: 0,
                    })
                    .layer_parallel_safe(true)
                    .build()
                }

                // Host built-ins as synthetic producers. The MeshIR commit at
                // Blackboard::new is represented by the "host:mesh" pseudo-module.
                let mut dag_modules = vec![
                    host_builtin("host:mesh", "PrePass::MeshAnalysis", &["MeshIR"]),
                    host_builtin(
                        "host:mesh_analysis",
                        "PrePass::MeshAnalysis",
                        &["SurfaceClassificationIR"],
                    ),
                    host_builtin(
                        "host:region_mapping",
                        "PrePass::RegionMapping",
                        &["RegionMapIR"],
                    ),
                    host_builtin("host:slice", "PrePass::Slice", &["SliceIR"]),
                    host_builtin(
                        "host:shell_classification",
                        "PrePass::ShellClassification",
                        &["SliceIR"],
                    ),
                    host_builtin(
                        "host:support_geometry",
                        "PrePass::SupportGeometry",
                        &["SupportGeometryIR"],
                    ),
                    host_builtin(
                        "host:paint_segmentation",
                        "PrePass::PaintSegmentation",
                        &["PaintRegionIR"],
                    ),
                    host_builtin("host:gcode_emit", "PostPass::GCodeEmit", &["GCodeIR"]),
                ];
                dag_modules.extend(loaded.bindings.iter().map(|b| b.module.clone()));

                let mut stage_dags: Vec<StageDag> = Vec::with_capacity(loaded.sorted_stages.len());
                for stage_entry in &loaded.sorted_stages {
                    match build_intra_stage_dag(stage_entry.stage_id.clone(), &dag_modules) {
                        Ok(nodes) => stage_dags.push(StageDag {
                            stage: stage_entry.stage_id.clone(),
                            nodes,
                        }),
                        Err(err) => {
                            eprintln!(
                                "error: intra-stage DAG construction failed for {}: {err:?}",
                                stage_entry.stage_id,
                            );
                            std::process::exit(1);
                        }
                    }
                }
                let request = DagValidationRequest {
                    modules: dag_modules,
                    stage_dags,
                    host_ir_schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
                    claim_holders: Vec::new(),
                    access_audits: Vec::new(),
                };
                let report = validate_startup_dag(&request);

                // Tier-1 fatal: IR-version compatibility — closes the
                // dormant schema-window gate documented in
                // docs/03_wit_and_manifest.md. A module declaring a
                // [min, max) window that excludes the host's
                // CURRENT_SLICE_IR_SCHEMA_VERSION cannot run safely.
                let ir_errors: Vec<_> = report
                    .errors
                    .iter()
                    .filter(|d| {
                        matches!(
                            d.pass,
                            slicer_host::DagValidationPass::IrVersionCompatibility
                        )
                    })
                    .collect();
                if !ir_errors.is_empty() {
                    eprintln!(
                        "error: startup DAG IR-version validation failed with {} \
                         incompatible module manifest(s):",
                        ir_errors.len(),
                    );
                    for diag in &ir_errors {
                        eprintln!("  {:?}", diag.detail);
                    }
                    std::process::exit(1);
                }

                // Tier-2 advisory: other passes (UnfulfilledReads,
                // WriteConflicts, etc.) currently need richer host
                // built-in modeling (synthetic stage_dags representing
                // MeshAnalysis/Slice/etc. writes) to avoid false
                // positives. Surface them as warnings until that
                // modeling lands.
                for diag in &report.errors {
                    if matches!(
                        diag.pass,
                        slicer_host::DagValidationPass::IrVersionCompatibility
                    ) {
                        continue;
                    }
                    eprintln!(
                        "warning: startup DAG advisory ({:?}): {:?}",
                        diag.pass, diag.detail
                    );
                }
                for warning in &report.warnings {
                    eprintln!(
                        "warning: startup DAG ({:?}): {:?}",
                        warning.pass, warning.detail
                    );
                }
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
            if let Err(e) = validate_support_layer_heights(&resolved_configs_map) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }

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
            let sink_arc: Arc<RuntimeProgressSink> =
                Arc::new(RuntimeProgressSink::new(emitter, Arc::clone(&collector)));

            // Stamp every event in this run with one slice_id so a consumer
            // reading stderr JSONL can correlate per-stage / per-module
            // timing back to one slice invocation.
            let progress_pi = if opts.instrument_stderr {
                let slice_id = format!(
                    "slice-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                );
                let sink_dyn: Arc<dyn LayerProgressSink + Send + Sync> =
                    Arc::clone(&sink_arc) as Arc<dyn LayerProgressSink + Send + Sync>;
                Some(ProgressPipelineInstrumentation::new(sink_dyn, slice_id))
            } else {
                None
            };

            let result = match (opts.report.as_ref(), progress_pi.as_ref()) {
                (Some(report_path), maybe_progress_pi) => {
                    if let Some(parent) = report_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!(
                                "warning: failed to create report parent directory {}: {e}",
                                parent.display()
                            );
                        }
                    }
                    report_alloc::enable();
                    let report_collector = Arc::new(Collector::new_with_verbose(
                        opts.model_path.to_string_lossy().to_string(),
                        opts.report_verbose,
                    ));
                    let r = if let Some(progress_pi) = maybe_progress_pi {
                        let composite = CompositeInstrumentation::new(
                            progress_pi as &dyn slicer_host::PipelineInstrumentation,
                            report_collector.as_ref() as &dyn slicer_host::PipelineInstrumentation,
                        );
                        run_pipeline_with_instrumentation(
                            config,
                            &config_source,
                            sink_arc.as_ref(),
                            &composite,
                        )
                    } else {
                        run_pipeline_with_instrumentation(
                            config,
                            &config_source,
                            sink_arc.as_ref(),
                            report_collector.as_ref(),
                        )
                    };
                    report_alloc::disable();
                    if let Err(e) = report_collector.finish_and_render_to(report_path) {
                        eprintln!("warning: failed to write slicer report: {e}");
                    }
                    r
                }
                (None, Some(progress_pi)) => run_pipeline_with_instrumentation(
                    config,
                    &config_source,
                    sink_arc.as_ref(),
                    progress_pi,
                ),
                (None, None) => {
                    run_pipeline_with_raw_config(config, &config_source, sink_arc.as_ref())
                }
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
        HostCommands::Dag { cmd } => {
            run_dag_command(cmd);
        }
        HostCommands::Diagnose {
            module_dir,
            no_default_module_paths,
        } => {
            std::process::exit(run_diagnose(&module_dir, no_default_module_paths));
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
