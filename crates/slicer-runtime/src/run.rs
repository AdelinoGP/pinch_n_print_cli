//! Library entry point for one-shot slicing. Extracted from main.rs::HostCommands::Run.

/// Default for the `use_relative_e_distances` host config key (M83 relative-E)
/// when the user does not set it. Mirrored in `docs/config/host-keys.toml`
/// (`[host_runtime]`) and locked by `gcode_emit::host_keys_doc_lock`.
pub const DEFAULT_USE_RELATIVE_E_DISTANCES: bool = true;

use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
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
    JsonLinesEmitter, NullEmitter, ProgressError, ProgressEvent, ProgressEventEmitter,
    ProgressPhase, ProgressStatus, RuntimeProgressSink, SliceEventCollector,
};
use crate::progress_instrumentation::{now_unix_ms, ProgressPipelineInstrumentation, ProgressTier};
#[cfg(feature = "report")]
use crate::report::{allocator as report_alloc, Collector};
use crate::validation::{validate_startup_dag, DagValidationPass, StageDag};
use slicer_gcode::{
    estimate_print, DefaultGCodeEmitter, DefaultGCodeSerializer, EstimatorLimits, GcodeFlavor,
};
use slicer_wasm_host::WasmRuntimeDispatcher;
use slicer_wasm_host::{build_live_execution_plan, load_live_modules_for_plan_with_config};

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
    /// JSONL stream during the slice (schema version `"1.2.0"`).
    pub instrument_stderr: bool,
    /// When true (the default for `pnp_cli slice`), emit the docs/09 core
    /// progress-event contract (phase/layer/validation/module_error/
    /// slice_complete) as JSONL on stderr. When false
    /// (`--no-progress-events`), nothing is written to stderr, though error
    /// aggregation still runs internally.
    pub progress_events: bool,
    /// Cooperative cancellation flag checked by the slicing pipeline.
    pub cancel_flag: Option<Arc<AtomicBool>>,
    /// Config values derived from the loaded model (e.g. the 3MF project's
    /// `filament_colour`) that seed `config_source` as defaults. An explicit
    /// `--config` key always wins over an override with the same name.
    pub config_overrides: std::collections::HashMap<String, ConfigValue>,
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

/// The slice-wide progress-event channel: one emitter, one collector, one
/// `slice_id`, constructed once per `run_slice` call (before validation) so
/// validation events and pipeline events share the same stream.
struct ProgressChannel {
    slice_id: String,
    sink: Arc<RuntimeProgressSink>,
}

/// Build the progress channel. `emit_to_stderr == false`
/// (`--no-progress-events`) swaps the JSONL stderr emitter for a
/// [`NullEmitter`]: the collector still aggregates error counts but nothing
/// reaches stderr.
fn build_progress_channel(
    emit_to_stderr: bool,
    collector: Option<Arc<Mutex<SliceEventCollector>>>,
) -> ProgressChannel {
    let emitter_arc: Arc<dyn ProgressEventEmitter> = if emit_to_stderr {
        Arc::new(JsonLinesEmitter::new(std::io::stderr()))
    } else {
        Arc::new(NullEmitter)
    };
    let collector = collector.unwrap_or_else(|| Arc::new(Mutex::new(SliceEventCollector::new())));
    let sink = Arc::new(RuntimeProgressSink::new(
        emitter_arc,
        Arc::clone(&collector),
    ));
    ProgressChannel {
        slice_id: format!("slice-{}", now_unix_ms()),
        sink,
    }
}

/// Select the pipeline execution path based on report and progress options.
///
/// This is the 4-way instrumentation fork originally in `main.rs`, now a
/// private helper inside `run.rs`. Uses the slice-wide `ProgressChannel`
/// built by [`run_slice`] before validation.
fn run_pipeline_fork(
    opts: &SliceRunOptions,
    channel: &ProgressChannel,
    config: PipelineConfig,
    config_source: &std::collections::HashMap<String, ConfigValue>,
    #[cfg(feature = "report")] dag_snapshot: Option<crate::report::ReportDagSnapshot>,
) -> Result<crate::pipeline::PipelineOutput, SliceRunError> {
    let sink_arc = Arc::clone(&channel.sink);

    let progress_pi = if opts.progress_events {
        let tier = if opts.instrument_stderr {
            ProgressTier::Instrumented
        } else {
            ProgressTier::Core
        };
        let sink_dyn: Arc<dyn LayerProgressSink + Send + Sync> =
            Arc::clone(&sink_arc) as Arc<dyn LayerProgressSink + Send + Sync>;
        Some(ProgressPipelineInstrumentation::with_tier(
            sink_dyn,
            channel.slice_id.clone(),
            tier,
        ))
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

    result.map_err(|e| {
        if opts
            .cancel_flag
            .as_ref()
            .is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed))
        {
            channel.sink.record(ProgressEvent::cancelled(
                channel.slice_id.clone(),
                now_unix_ms(),
            ));
        }
        SliceRunError(format!("{e}"))
    })
}

/// Whole-print statistics for the `slice_stats` progress event (packet 169).
///
/// Derived in [`run_slice`] from `slicer_gcode::estimate_print` over the
/// final postpass `GCodeIR` plus the resolved config
/// (`filament_density`, `first_layer_height`).
#[derive(Debug, Clone, PartialEq)]
pub struct SliceStatsInputs {
    /// Estimated print time in whole seconds.
    pub gcode_prediction_seconds: u64,
    /// Estimated filament weight in grams; `None` when `filament_density`
    /// is not configured (the event key is then omitted entirely).
    pub gcode_weight_grams: Option<f64>,
    /// Total filament length across all tools, in mm.
    pub gcode_filament_length_mm: f64,
    /// Number of emitted layers (`GCodeIR.metadata.layer_count`).
    pub layer_count: u32,
    /// First layer height in mm (`ResolvedConfig.first_layer_height`).
    pub first_layer_height_mm: f32,
    /// Extruded volume per extruder index, in mm³.
    pub extruded_volume_mm3: std::collections::BTreeMap<u32, f64>,
    /// Number of tool changes in the print.
    pub toolchange_count: u32,
}

/// Production end-of-slice emission path (packet 169 Step 3).
///
/// Records exactly one `slice_stats` event (when `stats` is `Some`, i.e. the
/// slice produced a G-code artifact — including degraded-but-successful
/// runs), strictly followed by exactly one `slice_complete` event built from
/// the sink's [`SliceEventCollector`] aggregate counts. `slice_complete`
/// status stays `Ok` even when degraded — "degraded success" is signalled by
/// the degraded flag plus `non_fatal_error_count` (docs/09 §Required Events).
///
/// Public so integration tests can assert the emitted JSONL stream from the
/// same code path `run_slice` uses in production.
pub fn emit_end_of_slice_events(
    sink: &RuntimeProgressSink,
    slice_id: &str,
    wallclock_ms: u64,
    stats: Option<SliceStatsInputs>,
) {
    if let Some(s) = stats {
        sink.record(ProgressEvent::slice_stats(
            slice_id.to_string(),
            now_unix_ms(),
            s.gcode_prediction_seconds,
            s.gcode_weight_grams,
            s.gcode_filament_length_mm,
            s.layer_count,
            s.first_layer_height_mm,
            s.extruded_volume_mm3,
            s.toolchange_count,
        ));
    }

    let (fatal, non_fatal, degraded) = {
        let collector = sink.collector();
        let c = collector
            .lock()
            .expect("slice event collector mutex poisoned");
        (c.fatal_count(), c.non_fatal_count(), c.is_degraded())
    };
    sink.record(ProgressEvent::slice_complete(
        slice_id.to_string(),
        now_unix_ms(),
        wallclock_ms,
        ProgressStatus::Ok,
        degraded,
        fatal,
        non_fatal,
    ));
}

/// One-shot slice. Drives the pipeline end-to-end and returns the produced G-code.
///
/// Composes the 4-way instrumentation fork (report, progress, both, none)
/// internally based on `opts.report` and `opts.instrument_stderr`.
pub fn run_slice(opts: SliceRunOptions) -> Result<SliceOutcome, SliceRunError> {
    run_slice_with_collector(opts, None)
}

/// Test-support entry point that lets callers inspect the same collector used
/// by [`run_slice`], including when the pipeline returns an error.
#[doc(hidden)]
pub fn run_slice_with_collector(
    opts: SliceRunOptions,
    collector: Option<Arc<Mutex<SliceEventCollector>>>,
) -> Result<SliceOutcome, SliceRunError> {
    let t0 = Instant::now();
    let channel = build_progress_channel(opts.progress_events, collector);

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

    // Seed model-derived config (e.g. the 3MF project's filament_colour) as
    // defaults: only fill keys the user did not set explicitly via --config.
    for (key, value) in &opts.config_overrides {
        config_source
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

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

    // Seed the host-injected `slice_has_paint` gate (classic-perimeters.toml
    // `[config.schema.slice_has_paint]`, "Slice contains painted regions
    // (host-injected)"): the module manifest declares this key expecting the
    // host to populate it, but nothing ever did, so `medial_axis_enabled`'s
    // painted-slice gate (added 2026-06-24 for exactly this boostvoronoi
    // instability) was permanently inert — `_config.get_bool("slice_has_paint")`
    // always saw `None` and fell back to `false` regardless of actual paint
    // data. Set `true` whenever any object in the mesh carries paint data;
    // never overrides an explicit user-supplied value.
    if mesh_ir.objects.iter().any(|o| o.paint_data.is_some())
        && !config_source.contains_key("slice_has_paint")
    {
        config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
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
    // Config-aware loader: resolves the `perimeter-generator` claim
    // collision (classic-perimeters vs arachne-perimeters) via the user's
    // `wall_generator` config key rather than alphabetical module-id order
    // (packet 112 Step 10 — see `load_live_modules_for_plan_with_config`'s
    // doc comment for the production defect this closes).
    let mut loaded =
        load_live_modules_for_plan_with_config(&search_roots, num_cpus_guess(), &config_source)
            .map_err(|e| {
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

    // 14-pass startup DAG validation, bracketed by phase_start/phase_complete
    // (validation) on the progress stream (docs/09 §Required Events). Fatal
    // failures emit a validation_error + phase_complete(fatal_error) before
    // bailing; advisories/warnings stay human-readable on stderr (the
    // validation_error wire event is fatal-only by construction).
    {
        use slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION;

        let vstart = Instant::now();
        channel.sink.record(ProgressEvent::phase_start(
            channel.slice_id.clone(),
            ProgressPhase::Validation,
            now_unix_ms(),
        ));
        // Emit the fatal-failure triple (validation_error + fatal
        // phase_complete) for a given stable code/message. The caller bails
        // immediately afterwards.
        let emit_validation_failure = |code: u32, message: &str| {
            channel.sink.record(ProgressEvent::validation_error(
                channel.slice_id.clone(),
                now_unix_ms(),
                ProgressError {
                    code,
                    message: message.to_string(),
                    fatal: true,
                    suggestion: None,
                    reason: None,
                },
            ));
            channel.sink.record(ProgressEvent::phase_complete(
                channel.slice_id.clone(),
                ProgressPhase::Validation,
                now_unix_ms(),
                vstart.elapsed().as_millis() as u64,
                ProgressStatus::FatalError,
            ));
        };

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
                    let msg = format!(
                        "intra-stage DAG construction failed for {}: {err:?}",
                        stage_entry.stage_id
                    );
                    emit_validation_failure(
                        crate::progress_events::VALIDATION_DAG_CONSTRUCTION_CODE,
                        &msg,
                    );
                    return Err(SliceRunError(msg));
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
        let host_version = crate::manifest::parse_semver(env!("CARGO_PKG_VERSION"))
            .expect("slicer-runtime CARGO_PKG_VERSION must be valid semver");
        let request = crate::validation::DagValidationRequest {
            modules: dag_modules,
            stage_dags,
            host_ir_schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            host_version,
            claim_holders,
            access_audits: Vec::new(),
        };
        let report = validate_startup_dag(&request);

        let version_errors: Vec<_> = report
            .errors
            .iter()
            .filter(|d| {
                matches!(
                    d.pass,
                    DagValidationPass::IrVersionCompatibility
                        | DagValidationPass::HostVersionCompatibility
                )
            })
            .collect();
        if !version_errors.is_empty() {
            let detail = version_errors
                .iter()
                .map(|d| format!("{:?}", d.detail))
                .collect::<Vec<_>>()
                .join("; ");
            let msg = format!("startup DAG version-compatibility validation failed: {detail}");
            emit_validation_failure(crate::progress_events::VALIDATION_VERSION_COMPAT_CODE, &msg);
            return Err(SliceRunError(msg));
        }

        for diag in &report.errors {
            if matches!(
                diag.pass,
                DagValidationPass::IrVersionCompatibility
                    | DagValidationPass::HostVersionCompatibility
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

        channel.sink.record(ProgressEvent::phase_complete(
            channel.slice_id.clone(),
            ProgressPhase::Validation,
            now_unix_ms(),
            vstart.elapsed().as_millis() as u64,
            ProgressStatus::Ok,
        ));
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
    let flavor = match config_source.get("gcode_flavor") {
        Some(ConfigValue::String(value)) => GcodeFlavor::from_config_str(value),
        _ => GcodeFlavor::Marlin,
    };
    let relative = match config_source.get("use_relative_e_distances") {
        Some(ConfigValue::Bool(b)) => *b,
        _ => DEFAULT_USE_RELATIVE_E_DISTANCES,
    };

    // Packet 169 Step 3: capture the estimator inputs the slice_stats event
    // needs before `default_resolved_config` / `per_tool_configs_map` are
    // moved into the pipeline config. Tool diameters mirror the emitter's own
    // `tool_configs` map (the estimator defaults missing tools to 1.75 mm).
    let estimator_limits = EstimatorLimits::from_config(&default_resolved_config);
    let stats_tool_diameters: std::collections::BTreeMap<u32, f32> = per_tool_configs_map
        .iter()
        .map(|(&tool, cfg)| (tool, cfg.filament_diameter))
        .collect();
    let stats_filament_density = default_resolved_config.filament_density;
    let stats_first_layer_height_mm = default_resolved_config.first_layer_height as f32;

    let pipeline_config = PipelineConfig {
        cancel_flag: opts.cancel_flag.clone(),
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
            serializer: Box::new(
                DefaultGCodeSerializer::with_extrusion_mode(relative).with_flavor(flavor),
            ),
        },
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved_config),
        bounds: Arc::new(config_bounds),
        wasm_handles,
    };

    // Run the pipeline through the 4-way instrumentation fork.
    let pipeline_output = run_pipeline_fork(
        &opts,
        &channel,
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

    // slice_stats + slice_complete (packet 169 Step 3): slice_stats is
    // emitted whenever the slice produced a G-code artifact (including
    // degraded-but-successful runs), strictly before the success-only,
    // exactly-once slice_complete (docs/09 §Required Events). On fatal
    // failure the stream ends at the error event with neither. The whole-print
    // numbers come from `estimate_print` over the final post-postprocess
    // `GCodeIR` surfaced by `crate::postpass::take_final_gcode_ir` — no
    // estimator math is re-implemented here.
    let stats = crate::postpass::take_final_gcode_ir().map(|ir| {
        let estimate = estimate_print(&ir, &estimator_limits, &stats_tool_diameters);
        let total_volume_mm3: f64 = estimate.extruded_volume_mm3.values().sum();
        SliceStatsInputs {
            gcode_prediction_seconds: estimate.total_time_s.round() as u64,
            // Weight only when filament_density (g/cm³) is configured; the
            // event key is omitted otherwise (never 0, never null). The
            // serializer's header default density is deliberately not used.
            gcode_weight_grams: stats_filament_density
                .map(|density| (total_volume_mm3 / 1000.0) * f64::from(density)),
            gcode_filament_length_mm: estimate.filament_length_mm.values().sum(),
            layer_count: ir.metadata.layer_count,
            first_layer_height_mm: stats_first_layer_height_mm,
            extruded_volume_mm3: estimate.extruded_volume_mm3,
            toolchange_count: estimate.toolchange_count,
        }
    });
    emit_end_of_slice_events(&channel.sink, &channel.slice_id, wallclock_ms, stats);

    Ok(SliceOutcome {
        gcode_text: pipeline_output.gcode_text,
        layer_count,
        wallclock_ms,
    })
}

/// Assembled scheduler/runtime context after prepass execution, ready for a
/// bounded per-layer closure (e.g.
/// [`crate::layer_executor::execute_captured_stages`]).
///
/// Built by [`prepare_prepass_context`]; used by `pnp-cli`'s visual-debug
/// command (packet 158) so it does not have to duplicate module loading,
/// config resolution, and plan construction to run a typed-tap capture.
pub struct PrepassContext {
    /// Execution plan with `global_layers` promoted from the
    /// prepass-committed `LayerPlanIR` (mirrors `run_pipeline_core`'s Step 2b).
    pub plan: crate::ExecutionPlan,
    /// Blackboard after prepass execution.
    pub blackboard: crate::Blackboard,
    /// Per-module wasmtime handles, keyed by `ModuleId`.
    pub wasm_handles: std::collections::HashMap<
        slicer_ir::ModuleId,
        (
            Arc<slicer_wasm_host::WasmInstancePool>,
            Option<Arc<slicer_wasm_host::WasmComponent>>,
        ),
    >,
    /// Dispatcher ready to run per-layer (Tier 2) stages against the same
    /// `wasmtime::Engine` used for prepass.
    pub layer_runner: WasmRuntimeDispatcher,
    /// The global fallback [`slicer_ir::ResolvedConfig`] this context resolved
    /// (the same value `run_slice` passes to prepass).
    ///
    /// Exposed for printer-level keys a caller needs but no stage output
    /// carries — `bed_shape` for visual-debug's `frame: "plate"` viewport
    /// being the first. Per-object overlays are irrelevant to those keys: the
    /// bed is a property of the printer, not of any object on it.
    pub default_resolved_config: Arc<slicer_ir::ResolvedConfig>,
}

/// Load modules, resolve config, build the live execution plan, and run
/// prepass — the shared prefix of [`run_slice`] up through Tier 1, factored
/// out so callers that only need a bounded Tier 2 closure (typed tap
/// capture, packet 158) do not have to duplicate it.
///
/// Deliberately narrower than `run_slice`'s setup: it skips the 14-pass
/// startup DAG validation, thumbnail/CONFIG_BLOCK wiring, relative-E and
/// MMU wipe-tower heuristics, `validate_support_layer_heights`, and
/// per-tool config resolution — none of which affect per-layer arena
/// commits, and all of which belong to gcode-emission concerns this entry
/// point never reaches.
///
/// # Errors
///
/// Returns [`SliceRunError`] if module loading, config resolution, plan
/// construction, or prepass execution fails.
pub fn prepare_prepass_context(
    mesh_ir: Arc<MeshIR>,
    mut config_source: std::collections::HashMap<String, ConfigValue>,
    module_dirs: &[PathBuf],
    no_default_module_paths: bool,
) -> Result<PrepassContext, SliceRunError> {
    // Seed planner-visible per-object world heights — required for
    // `layer-planner-default` (and any layer planner) to produce a
    // non-empty `LayerPlanIR`; without it prepass fails fatally with
    // "no objects with positive height" (mirrors `run_slice`).
    for object in &mesh_ir.objects {
        let key = format!("object_height:{}", object.id);
        if config_source.contains_key(&key) {
            continue;
        }
        if let Some((z_min, z_max)) = object.world_z_extent {
            config_source.insert(key, ConfigValue::Float((z_max - z_min) as f64));
        }
    }

    // Seed the host-injected `slice_has_paint` gate (mirrors `run_slice`):
    // set `true` whenever any object carries paint data, never overriding
    // an explicit user-supplied value.
    if mesh_ir.objects.iter().any(|o| o.paint_data.is_some())
        && !config_source.contains_key("slice_has_paint")
    {
        config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
    }

    // Seed per-object config from `ObjectMesh.config.data` (mirrors `run_slice`).
    for object in &mesh_ir.objects {
        for (subkey, value) in &object.config.data {
            let key = format!("object_config:{}:{}", object.id, subkey);
            if config_source.contains_key(&key) {
                continue;
            }
            config_source.insert(key, value.clone());
        }
    }

    let search_roots = assemble_search_roots(module_dirs, no_default_module_paths);
    let mut loaded =
        load_live_modules_for_plan_with_config(&search_roots, num_cpus_guess(), &config_source)
            .map_err(|e| {
                SliceRunError(format!(
                    "failed to load modules from {} root(s) {:?}: {e}",
                    search_roots.len(),
                    search_roots
                ))
            })?;

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

    let mut plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &config_source,
        Arc::new(Vec::new()),
        Arc::new(std::collections::HashMap::new()),
        &mut loaded.diagnostics,
    )
    .map_err(|e| SliceRunError(format!("failed to build execution plan: {e}")))?;

    let engine = Arc::clone(&loaded.engine);
    let prepass_runner = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let mut blackboard = crate::Blackboard::new(Arc::clone(&mesh_ir), 0);
    let empty_raw: std::collections::HashMap<slicer_ir::ConfigKey, ConfigValue> =
        std::collections::HashMap::new();

    crate::prepass::execute_prepass_with_builtins_configured(
        &plan,
        &mut blackboard,
        &prepass_runner,
        &resolved_configs_map,
        &default_resolved_config,
        &empty_raw,
        &config_bounds,
        &wasm_handles,
    )
    .map_err(|e| SliceRunError(format!("prepass failed: {e}")))?;

    if let Some(layer_plan) = blackboard.layer_plan() {
        plan.global_layers = Arc::new(layer_plan.global_layers.clone());
    }

    let layer_runner = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    Ok(PrepassContext {
        plan,
        blackboard,
        wasm_handles,
        layer_runner,
        default_resolved_config: Arc::new(default_resolved_config),
    })
}
