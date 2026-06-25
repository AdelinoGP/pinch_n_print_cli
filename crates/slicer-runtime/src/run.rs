//! Library entry point for one-shot slicing. Extracted from main.rs::HostCommands::Run.

/// Default for the `use_relative_e_distances` host config key (M83 relative-E)
/// when the user does not set it. Mirrored in `docs/config/host-keys.toml`
/// (`[host_runtime]`) and locked by `gcode_emit::host_keys_doc_lock`.
pub const DEFAULT_USE_RELATIVE_E_DISTANCES: bool = true;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use slicer_ir::{ConfigValue, MeshIR};

use crate::config_resolution::{
    resolve_global_config, resolve_per_object_configs, resolve_per_tool_configs,
    validate_support_layer_heights, ConfigBoundsIndex,
};
use crate::dag::Producer;
use crate::execution_plan::parse_cli_config_source;
#[cfg(feature = "report")]
use crate::instrumentation::CompositeInstrumentation;
use crate::layer_executor::LayerProgressSink;
use crate::module_search_path::assemble_search_roots;
use crate::pipeline::{
    run_pipeline_with_instrumentation, run_pipeline_with_raw_config, PipelineConfig,
    PipelineStageRunners,
};
use crate::progress_events::{
    JsonLinesEmitter, ProgressEventEmitter, RuntimeProgressSink, SliceEventCollector,
};
use crate::progress_instrumentation::ProgressPipelineInstrumentation;
#[cfg(feature = "report")]
use crate::report::{allocator as report_alloc, Collector};
use crate::validation::{validate_startup_dag, DagValidationPass, StageDag};
use slicer_gcode::{DefaultGCodeEmitter, DefaultGCodeSerializer};
use slicer_wasm_host::WasmRuntimeDispatcher;
use slicer_wasm_host::{build_live_execution_plan, load_live_modules_for_plan};

/// Validated runtime options derived from CLI arguments.
///
/// Hosted in `run` rather than a CLI module because the runtime library — not
/// the CLI — defines and consumes this contract. The `pnp_cli` binary builds
/// a `SliceRunOptions` value and hands it to [`run_slice`].
#[derive(Debug, Clone)]
pub struct SliceRunOptions {
    /// Pre-loaded mesh IR. Loaded by the caller (e.g., `pnp-cli`) before invoking `run_slice`.
    pub mesh: Arc<MeshIR>,
    /// Display label for the mesh source (file path, "<stdin>", etc.); used in the HTML report.
    pub model_label: String,
    /// Optional path to a JSON configuration file.
    pub config_path: Option<PathBuf>,
    /// Optional path to the output G-code file.
    pub output_path: Option<PathBuf>,
    /// Directories to search for additional modules, in CLI order.
    pub module_dirs: Vec<PathBuf>,
    /// When true, suppress the platform default module search paths.
    pub no_default_module_paths: bool,
    /// Optional path to a PNG thumbnail image for the G-code header.
    pub thumbnail: Option<PathBuf>,
    /// Optional path for an HTML slicer report. When the `report` feature is
    /// disabled, supplying a non-`None` value causes [`run_slice`] to return
    /// an error explaining the build was compiled without report support.
    pub report: Option<PathBuf>,
    /// Verbose report mode (per-layer-per-module rows).
    pub report_verbose: bool,
    /// When true, emit per-stage / per-module timing events on the stderr
    /// JSONL stream during the slice (schema version `"1.1.0"`).
    pub instrument_stderr: bool,
}

/// Output produced by a successful `run_slice` call.
#[derive(Debug, Clone)]
pub struct SliceOutcome {
    /// The final G-code text.
    pub gcode_text: String,
    /// Number of layers sliced (best-effort: derived from gcode markers).
    pub layer_count: u32,
    /// Wall-clock time of the pipeline in milliseconds.
    pub wallclock_ms: u64,
}

/// Error returned by `run_slice`.
///
/// Wraps the underlying cause as a formatted string so the library does not
/// require `anyhow` as a public dependency.
#[derive(Debug)]
pub struct SliceRunError(pub String);

impl std::fmt::Display for SliceRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SliceRunError {}

impl From<String> for SliceRunError {
    fn from(s: String) -> Self {
        SliceRunError(s)
    }
}

impl From<&str> for SliceRunError {
    fn from(s: &str) -> Self {
        SliceRunError(s.to_string())
    }
}

/// Convenience macro: return `Err(SliceRunError)` with a formatted message.
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err(SliceRunError(format!($($arg)*)))
    };
}

fn num_cpus_guess() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Build the static-DAG snapshot that the HTML report renders in its
/// "Pipeline (DAG)" section. One `StageOut` per stage in canonical
/// `STAGE_ORDER` — empty stages are kept with `modules: []` so the
/// section mirrors the pipeline shape rather than only the populated
/// subset.
#[cfg(feature = "report")]
fn build_report_dag_snapshot(producers: &[&dyn Producer]) -> crate::report::ReportDagSnapshot {
    use slicer_scheduler::dag_cli::{
        run_dag_claims, run_dag_global_edges, run_dag_stage, StageOut,
    };
    use slicer_scheduler::execution_plan::STAGE_ORDER;
    use slicer_scheduler::stage_order::tier_of;

    let stages: Vec<StageOut> = STAGE_ORDER
        .iter()
        .map(|stage_id| {
            run_dag_stage(producers, &(*stage_id).to_string()).unwrap_or_else(|| StageOut {
                id: (*stage_id).to_string(),
                tier: tier_of(stage_id).to_string(),
                modules: Vec::new(),
                serial_edges: Vec::new(),
            })
        })
        .collect();

    crate::report::ReportDagSnapshot {
        stages,
        cross_stage_edges: run_dag_global_edges(producers),
        claims: Some(run_dag_claims(producers)),
    }
}

/// Select the pipeline execution path based on report and progress options.
///
/// This is the 4-way instrumentation fork originally in `main.rs`, now a
/// private helper inside `run.rs`. Creates the event sink internally.
fn run_pipeline_fork(
    opts: &SliceRunOptions,
    config: PipelineConfig,
    config_source: &std::collections::HashMap<String, ConfigValue>,
    #[cfg(feature = "report")] dag_snapshot: Option<crate::report::ReportDagSnapshot>,
) -> Result<crate::pipeline::PipelineOutput, SliceRunError> {
    let emitter_arc: Arc<dyn ProgressEventEmitter> =
        Arc::new(JsonLinesEmitter::new(std::io::stderr()));
    let collector = Arc::new(Mutex::new(SliceEventCollector::new()));
    let sink_arc: Arc<RuntimeProgressSink> = Arc::new(RuntimeProgressSink::new(
        emitter_arc,
        Arc::clone(&collector),
    ));

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
        #[cfg(feature = "report")]
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
                opts.model_label.clone(),
                opts.report_verbose,
            ));
            if let Some(snap) = dag_snapshot {
                report_collector.set_dag_snapshot(snap);
            }
            let r = if let Some(progress_pi) = maybe_progress_pi {
                let composite = CompositeInstrumentation::new(
                    progress_pi as &dyn crate::instrumentation::PipelineInstrumentation,
                    report_collector.as_ref()
                        as &dyn crate::instrumentation::PipelineInstrumentation,
                );
                run_pipeline_with_instrumentation(
                    config,
                    config_source,
                    sink_arc.as_ref(),
                    &composite,
                )
            } else {
                run_pipeline_with_instrumentation(
                    config,
                    config_source,
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
        #[cfg(not(feature = "report"))]
        (Some(_), _) => {
            return Err(SliceRunError(
                "--report support not compiled (build with default features or --features report)"
                    .to_string(),
            ));
        }
        (None, Some(progress_pi)) => {
            run_pipeline_with_instrumentation(config, config_source, sink_arc.as_ref(), progress_pi)
        }
        (None, None) => run_pipeline_with_raw_config(config, config_source, sink_arc.as_ref()),
    };

    result.map_err(|e| SliceRunError(format!("{e}")))
}

/// One-shot slice. Drives the pipeline end-to-end and returns the produced G-code.
///
/// Composes the 4-way instrumentation fork (report, progress, both, none)
/// internally based on `opts.report` and `opts.instrument_stderr`.
pub fn run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError> {
    let t0 = Instant::now();

    // Mesh is pre-loaded by the caller (see SliceRunOptions::mesh).
    let mesh_ir = Arc::clone(&opts.mesh);

    // Parse user-facing JSON config (empty map when not supplied).
    let mut config_source = match opts.config_path.as_ref() {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| SliceRunError(format!("failed to read --config file: {e}")))?;
            parse_cli_config_source(&text)
                .map_err(|e| SliceRunError(format!("failed to parse --config: {e}")))?
        }
        None => std::collections::HashMap::new(),
    };

    // Insert thumbnail_path into config_source when --thumbnail is supplied.
    if let Some(ref thumb_path) = opts.thumbnail {
        config_source.insert(
            "thumbnail_path".to_string(),
            ConfigValue::String(thumb_path.to_string_lossy().to_string()),
        );
    }

    // Seed planner-visible per-object world heights.
    for object in &mesh_ir.objects {
        let key = format!("object_height:{}", object.id);
        if config_source.contains_key(&key) {
            continue;
        }
        if let Some((z_min, z_max)) = object.world_z_extent {
            config_source.insert(key, ConfigValue::Float((z_max - z_min) as f64));
        }
    }

    // Seed per-object config from `ObjectMesh.config.data`.
    for object in &mesh_ir.objects {
        for (subkey, value) in &object.config.data {
            let key = format!("object_config:{}:{}", object.id, subkey);
            if config_source.contains_key(&key) {
                continue;
            }
            config_source.insert(key, value.clone());
        }
    }

    // MMU wipe-tower auto-enable (diagnose 2026-06-24, gap #3). OrcaSlicer turns
    // on a prime/wipe tower automatically for multi-tool prints so colour
    // transitions are purged off-part. Mirror that: if the model paints >= 2
    // distinct tool indices and the user did NOT explicitly set
    // `wipe_tower_enabled`, enable it. Single-colour / unpainted prints keep the
    // default (false) and are unaffected. (The wipe-tower module is fully
    // implemented and wired; it was simply gated off by the resolved-config
    // default of `false` with no auto-enable signal.)
    if !config_source.contains_key("wipe_tower_enabled") {
        use std::collections::BTreeSet;
        let mut tools: BTreeSet<u32> = BTreeSet::new();
        for object in &mesh_ir.objects {
            if let Some(pd) = &object.paint_data {
                for layer in &pd.layers {
                    for fv in layer.facet_values.iter().flatten() {
                        if let slicer_ir::PaintValue::ToolIndex(n) = fv {
                            tools.insert(*n);
                        }
                    }
                }
            }
        }
        if tools.len() >= 2 {
            config_source.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
        }
    }

    // Discover and plan every module under --module-dir.
    let search_roots = assemble_search_roots(&opts.module_dirs, opts.no_default_module_paths);
    let mut loaded = load_live_modules_for_plan(&search_roots, num_cpus_guess()).map_err(|e| {
        SliceRunError(format!(
            "failed to load modules from {} root(s) {:?}: {e}",
            search_roots.len(),
            search_roots
        ))
    })?;
    for diag in &loaded.diagnostics {
        eprintln!(
            "{level:?}: {path}: {msg}",
            level = diag.level,
            path = diag.path.display(),
            msg = diag.message,
        );
    }

    // Static-DAG snapshot for the HTML report's "Pipeline (DAG)" section.
    // Captured here so it can borrow the same `dag_producers` slice the
    // validator builds below; the snapshot itself stores owned strings so
    // it can outlive that borrow.
    #[cfg(feature = "report")]
    let mut dag_snapshot: Option<crate::report::ReportDagSnapshot> = None;

    // 13-pass startup DAG validation.
    {
        use slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION;

        let mut dag_producers: Vec<&dyn Producer> = crate::runtime_builtins();
        dag_producers.extend(loaded.bindings.iter().map(|b| &b.module as &dyn Producer));

        #[cfg(feature = "report")]
        if opts.report.is_some() {
            dag_snapshot = Some(build_report_dag_snapshot(&dag_producers));
        }

        let mut stage_dags: Vec<StageDag> = Vec::with_capacity(loaded.sorted_stages.len());
        for stage_entry in &loaded.sorted_stages {
            match crate::dag::build_intra_stage_dag(stage_entry.stage_id.clone(), &dag_producers) {
                Ok(nodes) => stage_dags.push(StageDag {
                    stage: stage_entry.stage_id.clone(),
                    nodes,
                }),
                Err(err) => {
                    bail!(
                        "intra-stage DAG construction failed for {}: {err:?}",
                        stage_entry.stage_id
                    );
                }
            }
        }

        let dag_modules: Vec<crate::manifest::LoadedModule> =
            loaded.bindings.iter().map(|b| b.module.clone()).collect();
        // Build global-scope ClaimHolder entries from each loaded module's
        // declared `claims` so the validator can resolve fill-role-claim
        // owners (claim:sparse-fill, claim:top-fill, claim:bottom-fill,
        // claim:bridge-fill, claim:ironing, etc.). Pre-fix this was an
        // empty Vec which produced startup `MissingDependency` warnings
        // for every fill-role claim — see `docs/specs/infill-fill-partition-plan.md`
        // Phase A2 and the user-reproducible cube slice in DEV-065 notes.
        let claim_holders: Vec<crate::validation::ClaimHolder> = dag_modules
            .iter()
            .flat_map(|m| {
                m.claims()
                    .iter()
                    .map(|claim| crate::validation::ClaimHolder {
                        claim: claim.clone(),
                        module_id: m.id().to_string(),
                        scope: crate::validation::ConflictScope::Global,
                    })
            })
            .collect();
        let request = crate::validation::DagValidationRequest {
            modules: dag_modules,
            stage_dags,
            host_ir_schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            claim_holders,
            access_audits: Vec::new(),
        };
        let report = validate_startup_dag(&request);

        let ir_errors: Vec<_> = report
            .errors
            .iter()
            .filter(|d| matches!(d.pass, DagValidationPass::IrVersionCompatibility))
            .collect();
        if !ir_errors.is_empty() {
            let detail = ir_errors
                .iter()
                .map(|d| format!("{:?}", d.detail))
                .collect::<Vec<_>>()
                .join("; ");
            bail!("startup DAG IR-version validation failed: {}", detail);
        }

        for diag in &report.errors {
            if matches!(diag.pass, DagValidationPass::IrVersionCompatibility) {
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

    let config_bounds = ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));

    let default_resolved_config = resolve_global_config(&config_source, &config_bounds)
        .map_err(|e| SliceRunError(format!("config resolution failed: {e}")))?;

    let object_ids: Vec<&str> = mesh_ir.objects.iter().map(|o| o.id.as_str()).collect();
    let resolved_configs_map = resolve_per_object_configs(
        &default_resolved_config,
        &config_source,
        &object_ids,
        &config_bounds,
    )
    .map_err(|e| SliceRunError(format!("config resolution failed: {e}")))?;

    validate_support_layer_heights(&resolved_configs_map)
        .map_err(|e| SliceRunError(format!("{e}")))?;

    // Per-tool/extruder config overlays (`tool_config:<idx>:<key>`). Applied at
    // emit time (the entity's tool is only known there). Empty unless the user
    // sets `tool_config:` keys, so default behaviour is unchanged.
    let per_tool_configs_map =
        resolve_per_tool_configs(&default_resolved_config, &config_source, &config_bounds)
            .map_err(|e| SliceRunError(format!("config resolution failed: {e}")))?;

    // Build wasm_handles side-table before consuming bindings.
    let wasm_handles: std::collections::HashMap<
        slicer_ir::ModuleId,
        (
            Arc<slicer_wasm_host::WasmInstancePool>,
            Option<Arc<slicer_wasm_host::WasmComponent>>,
        ),
    > = loaded
        .bindings
        .iter()
        .map(|b| {
            (
                b.module.id().to_string(),
                (Arc::clone(&b.instance_pool), b.wasm_component.clone()),
            )
        })
        .collect();

    let plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &config_source,
        Arc::new(Vec::new()),
        Arc::new(std::collections::HashMap::new()),
        &mut loaded.diagnostics,
    )
    .map_err(|e| SliceRunError(format!("failed to build execution plan: {e}")))?;

    let engine = Arc::clone(&loaded.engine);
    let relative = match config_source.get("use_relative_e_distances") {
        Some(ConfigValue::Bool(b)) => *b,
        _ => DEFAULT_USE_RELATIVE_E_DISTANCES,
    };

    let pipeline_config = PipelineConfig {
        mesh_ir,
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(
                DefaultGCodeEmitter::new(concat!("pnp_cli ", env!("CARGO_PKG_VERSION")).into())
                    .with_resolved_config(default_resolved_config.clone())
                    .with_tool_configs(per_tool_configs_map.clone()),
            ),
            serializer: Box::new(DefaultGCodeSerializer::with_extrusion_mode(relative)),
        },
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved_config),
        bounds: Arc::new(config_bounds),
        wasm_handles,
    };

    // Run the pipeline through the 4-way instrumentation fork.
    let pipeline_output = run_pipeline_fork(
        &opts,
        pipeline_config,
        &config_source,
        #[cfg(feature = "report")]
        dag_snapshot,
    )?;

    let wallclock_ms = t0.elapsed().as_millis() as u64;

    // Derive layer_count: count layer-change markers in gcode (best-effort proxy).
    let layer_count = pipeline_output
        .gcode_text
        .lines()
        .filter(|l| l.starts_with(";LAYER_CHANGE") || l.starts_with("; layer"))
        .count() as u32;

    Ok(SliceOutcome {
        gcode_text: pipeline_output.gcode_text,
        layer_count,
        wallclock_ms,
    })
}
