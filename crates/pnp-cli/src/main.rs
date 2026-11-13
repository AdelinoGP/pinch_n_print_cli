//! Binary entry point for the pnp_cli (Pinch'n'Print) unified CLI.
//!
//! Thin clap dispatcher over a noun-namespaced verb tree:
//! - `slice`         — one-shot slice: model → G-code
//! - `module`        — module-author commands (new / diagnose / config-schema)
//! - `mesh`          — mesh ops (repair / decimate / import)
//! - `dag`           — DAG introspection (stages / stage / depends / claims)

use pnp_cli::io::{write_with_parents, OutputFormat};
use pnp_cli::module_new;
use pnp_cli::profile_cmd::run_profile_from;

// Install the host-allocation accounting wrapper as the process global
// allocator so `--report` can attribute per-bracket bytes. The wrapper's fast
// path when accounting is disabled is a single relaxed atomic load per call;
// `report_alloc::enable()` flips it on for the duration of a `--report` run.
// Gated by the `report` feature so `--no-default-features` builds don't pull
// the symbol in.
//
// See docs/16_slicer_report.md §"Global allocator contract" for the binary
// installation contract. The wrapped allocator is mimalloc rather than the
// system heap (wayfinder ticket 102 candidate).
#[cfg(feature = "report")]
#[global_allocator]
static ALLOC: slicer_runtime::report::AccountingAllocator<mimalloc::MiMalloc> =
    slicer_runtime::report::AccountingAllocator::new(mimalloc::MiMalloc);

mod helpers_cmd;
mod support_preview;
mod visual_debug;

use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand};
use slicer_runtime::{runtime_builtins, SliceRunOptions};
use slicer_scheduler::dag_cli::{run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages};
use slicer_scheduler::{
    assemble_search_roots, build_config_schema_json, load_modules_from_roots_with_integrated,
    LoadedModule, Producer,
};

/// Exit status used when a slice is cancelled by a signal or stdin EOF.
pub const EXIT_CODE_CANCELLED: i32 = 130;

// ---------------------------------------------------------------------------
// Top-level CLI
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "pnp_cli", version, about = "Pinch'n'Print modular slicer CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// One-shot slice: model → G-code
    Slice {
        /// Path to the input 3D model (STL, OBJ, or 3MF).
        #[arg(long)]
        model: PathBuf,
        #[cfg(feature = "scad")]
        /// Ordered `NAME=EXPR` SCAD overrides; may be repeated.
        #[arg(long = "scad-param", value_name = "NAME=EXPR")]
        scad_param: Vec<String>,
        #[cfg(feature = "scad")]
        /// Explicit 2D extrusion height in millimetres.
        #[arg(
            long = "scad-extrude-height",
            value_name = "MM",
            allow_hyphen_values = true
        )]
        scad_extrude_height: Option<f64>,
        /// Path to a JSON configuration file.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Path to the output G-code file (default: stdout).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Directory to search for additional modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
        /// Path to a PNG thumbnail image to embed in the G-code header.
        #[arg(long)]
        thumbnail: Option<PathBuf>,
        /// Optional path for an HTML slicer report.
        #[cfg(feature = "report")]
        #[arg(long, value_name = "PATH.html")]
        report: Option<PathBuf>,
        /// Verbose report mode (per-layer-per-module rows). Requires `--report`.
        #[cfg(feature = "report")]
        #[arg(long, requires = "report")]
        report_verbose: bool,
        /// Emit per-stage/per-module timing events on stderr JSONL stream.
        #[arg(long = "instrument-stderr")]
        instrument_stderr: bool,
        /// Meter wasmtime fuel, attribute it to scopes, and print a ranked
        /// summary to stderr at slice end (ADR-0055). Costs throughput.
        #[arg(long)]
        profile: bool,
        /// Also attach each dispatch call's scope fold to its `module_complete`
        /// event. Requires `--profile`.
        #[arg(long = "profile-verbose", requires = "profile")]
        profile_verbose: bool,
        /// Suppress the default JSONL progress-event stream on stderr.
        #[arg(long = "no-progress-events")]
        no_progress_events: bool,
        /// Cancel the slice when stdin is closed (EOF). The process exits with code 130.
        #[arg(long = "cancel-on-stdin-eof")]
        cancel_on_stdin_eof: bool,
    },
    /// Re-summarise a captured progress-event stream without re-slicing.
    ///
    /// Reads the `profile_summary` event out of a `--profile` capture and
    /// prints the same ranked table `slice --profile` printed live. Mirrors
    /// `cargo xtask test --summary-from`: the expensive work already happened,
    /// and re-running it to see the numbers again would be the mistake.
    Profile {
        /// Path to a JSONL capture produced by `slice --profile` (the stderr
        /// stream), or `-` to read stdin.
        #[arg(long = "from", value_name = "EVENTS.JSONL")]
        from: PathBuf,
        /// Print the `profile_summary` event's JSON payload instead of the
        /// ranked table.
        #[arg(long)]
        json: bool,
    },
    /// Generate a support-geometry preview JSON document.
    SupportPreview {
        /// Path to the input 3D model (STL, OBJ, or 3MF).
        #[arg(long)]
        input: PathBuf,
        /// Path to the output support preview JSON file.
        #[arg(long)]
        output: PathBuf,
        /// Path to a JSON configuration file.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Directory to search for additional modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
    },
    /// Generate a versioned visual-debug bundle.
    VisualDebug {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    /// Module-author commands
    Module {
        #[command(subcommand)]
        action: ModuleCmd,
    },
    /// Mesh operations
    Mesh {
        #[command(subcommand)]
        action: MeshCmd,
    },
    /// DAG introspection (manifest-only; no WASM compilation)
    Dag {
        #[command(subcommand)]
        action: DagCmd,
    },
}

// ---------------------------------------------------------------------------
// Module subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum ModuleCmd {
    /// Scaffold a new module crate (Step 7 implements the body)
    New {
        /// Directory to create the new module crate in.
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        /// Module name (reverse-domain, e.g. `com.example.my_infill`).
        #[arg(long)]
        name: String,
        /// Pipeline stage this module targets (e.g. `Layer::Infill`).
        #[arg(long)]
        stage: String,
    },
    /// Validate the discovered module set and emit structured diagnostics.
    Diagnose {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
    },
    /// Emit combined config schema JSON from loaded modules.
    ConfigSchema {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
    },
}

// ---------------------------------------------------------------------------
// Mesh subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum MeshCmd {
    /// Repair a mesh: degenerate removal, orientation normalization, open-edge closure.
    Repair {
        /// Input mesh file (STL, OBJ, or 3MF).
        #[arg(long)]
        input: PathBuf,
        /// Output mesh file path.
        #[arg(long)]
        output: PathBuf,
        /// Output format (default: infer from extension).
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
        /// Print repair statistics to stderr as line-delimited JSON.
        #[arg(long)]
        stats: bool,
    },
    /// Reduce triangle count via QEM edge collapse.
    #[command(group = ArgGroup::new("decimate_target")
        .required(true)
        .multiple(false)
        .args(["target_count", "target_ratio"]))]
    Decimate {
        /// Input mesh file (STL, OBJ, or 3MF).
        #[arg(long)]
        input: PathBuf,
        /// Output mesh file path.
        #[arg(long)]
        output: PathBuf,
        /// Absolute target triangle count. Mutually exclusive with `--target-ratio`.
        #[arg(long)]
        target_count: Option<usize>,
        /// Fraction of original triangle count to retain (0.0–1.0). Mutually exclusive with `--target-count`.
        #[arg(long)]
        target_ratio: Option<f32>,
        /// Maximum allowed quadric error budget.
        #[arg(long, default_value_t = 0.01)]
        max_error: f32,
        /// Use sloppy simplification (faster, lower quality near boundaries).
        #[arg(long)]
        aggressive: bool,
        /// Print decimation statistics to stderr as line-delimited JSON.
        #[arg(long)]
        stats: bool,
    },
    /// Import a STEP/STP file as a triangulated mesh.
    Import {
        /// Input STEP or STP file.
        #[arg(long)]
        input: PathBuf,
        #[cfg(feature = "scad")]
        /// Ordered `NAME=EXPR` SCAD overrides; may be repeated.
        #[arg(long = "scad-param", value_name = "NAME=EXPR")]
        scad_param: Vec<String>,
        #[cfg(feature = "scad")]
        /// Explicit 2D extrusion height in millimetres.
        #[arg(
            long = "scad-extrude-height",
            value_name = "MM",
            allow_hyphen_values = true
        )]
        scad_extrude_height: Option<f64>,
        /// Output mesh file path.
        #[arg(long)]
        output: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Stl)]
        output_format: OutputFormat,
        /// Merge all solids into a single mesh before output.
        #[arg(long)]
        merge_components: bool,
        /// Skip the automatic repair pass applied after tessellation.
        #[arg(long)]
        no_repair: bool,
        /// Print import statistics to stderr as line-delimited JSON.
        #[arg(long)]
        stats: bool,
    },
    /// Convert a mesh file between formats, optionally splitting or merging connected components.
    Convert {
        /// Input mesh file (STL, OBJ, or 3MF). STEP/STP not accepted — use `mesh import`.
        #[arg(long)]
        input: PathBuf,
        /// Output mesh file path.
        #[arg(long)]
        output: PathBuf,
        /// Output format (default: infer from extension).
        #[arg(long, value_enum, alias = "format")]
        output_format: Option<OutputFormat>,
        /// Keep all connected components merged into one object per input object (no splitting).
        #[arg(long)]
        merge_components: bool,
        /// Apply mesh repair before writing output.
        #[arg(long)]
        repair: bool,
    },
}

// ---------------------------------------------------------------------------
// DAG subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
enum DagCmd {
    /// List every discovered stage with tier, module count, and claim count.
    Stages {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
        /// Optional model file for object-id context.
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
    },
    /// Full detail for a single stage (canonical id, e.g. `Layer::Infill`).
    Stage {
        /// Canonical stage id (e.g. `PrePass::MeshAnalysis`).
        id: String,
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
        /// Optional model file.
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
    },
    /// Upstream and downstream edges for a single module, across all stages.
    Depends {
        /// Reverse-domain module id (e.g. `com.example.cubic_infill`).
        module_id: String,
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
        /// Optional model file.
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
    },
    /// List every claim with its holders, requesters, and interchangeability.
    Claims {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Disable the integrated-module tier (tier 5) entirely.
        #[arg(long = "no-integrated-modules")]
        no_integrated_modules: bool,
        /// Optional model file.
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn cli_integrated_registrations(
    no_integrated_modules: bool,
) -> Vec<slicer_scheduler::IntegratedModuleRegistration> {
    if no_integrated_modules {
        Vec::new()
    } else {
        slicer_integrated_modules::integrated_registrations()
    }
}

fn load_dag_modules(
    module_dir: &[PathBuf],
    no_default_module_paths: bool,
    no_integrated_modules: bool,
) -> Vec<LoadedModule> {
    let search_roots = assemble_search_roots(module_dir, no_default_module_paths);
    match load_modules_from_roots_with_integrated(
        &search_roots,
        &cli_integrated_registrations(no_integrated_modules),
    ) {
        Ok(r) => r.modules,
        Err(e) => {
            eprintln!("error loading modules: {e:?}");
            std::process::exit(2);
        }
    }
}

fn dag_producers<'a>(loaded: &'a [LoadedModule]) -> Vec<&'a dyn Producer> {
    let mut producers: Vec<&'a dyn Producer> = runtime_builtins()
        .into_iter()
        .map(|p| p as &'a dyn Producer)
        .collect();
    producers.extend(loaded.iter().map(|m| m as &dyn Producer));
    producers
}

fn object_ids_from_model(path: &std::path::Path) -> Option<Vec<String>> {
    match slicer_model_io::load_model(path) {
        Ok(ir) => Some(ir.objects.iter().map(|o| o.id.clone()).collect()),
        Err(e) => {
            eprintln!("warning: failed to load --model {}: {e}", path.display());
            None
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

/// Parse and validate the command-local SCAD flags into adapter options.
///
/// The raw `NAME=EXPR` tokens are split at their **first** `=` and forwarded in
/// input order with duplicates, identifier case, `$` names and expression bytes
/// preserved. Nothing here parses the value as JSON, shell code or slicer
/// config; the adapter owns expression grammar.
///
/// Returns the CLI-facing message (already carrying its fragment) on invalid
/// input; callers print it as `error: <message>` and exit 2.
#[cfg(feature = "scad")]
fn scad_cli_options(
    raw_parameters: &[String],
    extrude_height_mm: Option<f64>,
) -> Result<helpers_cmd::ScadImportOptions, String> {
    let mut parameters = Vec::with_capacity(raw_parameters.len());
    for raw in raw_parameters {
        let Some((name, expression)) = raw.split_once('=') else {
            return Err(format!(
                "SCAD_INVALID_PARAMETER_ARGUMENT: '{raw}' is not NAME=EXPR; expected NAME=EXPR"
            ));
        };
        if name.is_empty() {
            return Err(format!(
                "SCAD_INVALID_PARAMETER_ARGUMENT: '{raw}' has an empty name; \
                 expected NAME=EXPR with a non-empty name"
            ));
        }
        if expression.is_empty() {
            return Err(format!(
                "SCAD_INVALID_PARAMETER_ARGUMENT: '{raw}' has an empty expression; \
                 expected NAME=EXPR with a non-empty expression"
            ));
        }
        parameters.push(slicer_model_io::ScadParameter {
            name: name.to_owned(),
            expression: expression.to_owned(),
        });
    }

    if let Some(height) = extrude_height_mm {
        if !height.is_finite() || height <= 0.0 {
            return Err(format!(
                "SCAD_INVALID_EXTRUSION_HEIGHT: {height} is not a valid extrusion height; \
                 the height must be finite and strictly positive"
            ));
        }
    }

    Ok(helpers_cmd::ScadImportOptions::new(
        parameters,
        extrude_height_mm,
    ))
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// Install the process-wide `log` implementation.
///
/// Without an implementation crate the `log` facade discards every record —
/// including the module logs `forward_module_logs` fans out with target
/// `slicer_module::<module_id>` and every native `log::` call site in the
/// workspace. `RUST_LOG` is honoured verbatim (so
/// `RUST_LOG=slicer_module::com.example.infill=debug` filters per module);
/// when it is unset the filter defaults to `warn`.
///
/// This writes to stderr through `env_logger`'s own writer and does not touch
/// the direct `eprintln!` diagnostics (e.g. the `warning: startup DAG ...`
/// advisories in `run_slice`), which stay unconditional.
fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
}

fn main() {
    init_logging();
    let cli = Cli::parse();
    match cli.cmd {
        // ── slice ──────────────────────────────────────────────────────────
        Cmd::Slice {
            model,
            #[cfg(feature = "scad")]
            scad_param,
            #[cfg(feature = "scad")]
            scad_extrude_height,
            config,
            output,
            module_dir,
            no_default_module_paths,
            no_integrated_modules,
            thumbnail,
            #[cfg(feature = "report")]
            report,
            #[cfg(feature = "report")]
            report_verbose,
            instrument_stderr,
            profile,
            profile_verbose,
            no_progress_events,
            cancel_on_stdin_eof,
        } => {
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
            #[cfg(feature = "scad")]
            let scad_options = match scad_cli_options(&scad_param, scad_extrude_height) {
                Ok(options) => options,
                Err(message) => {
                    eprintln!("error: {message}");
                    std::process::exit(2);
                }
            };
            #[cfg(not(feature = "scad"))]
            let scad_options = helpers_cmd::ScadImportOptions::default();
            #[cfg(not(feature = "scad"))]
            let _ = &scad_options;
            #[cfg(feature = "scad")]
            {
                if !scad_options.is_empty()
                    && !matches!(
                        slicer_model_io::detect_format(&model),
                        Ok(slicer_model_io::ModelFormat::Scad)
                    )
                {
                    eprintln!(
                        "error: SCAD_OPTIONS_REQUIRE_SCAD_INPUT: \
                         --scad-param/--scad-extrude-height require a .scad input ({})",
                        model.display()
                    );
                    std::process::exit(2);
                }
            }
            let output_path = output.clone();
            let model_label = model.to_string_lossy().into_owned();
            // A `.scad` input is evaluated through the explicit adapter; every
            // other format keeps the non-evaluating `load_model` path. The
            // branch keys on the *detected* format, not on the flags: a plain
            // `--model model.scad` slices with default options, while a
            // non-SCAD input carrying SCAD flags was already rejected above.
            #[cfg(feature = "scad")]
            let mesh = if matches!(
                slicer_model_io::detect_format(&model),
                Ok(slicer_model_io::ModelFormat::Scad)
            ) {
                match slicer_model_io::load_scad_model(&model, &scad_options.eval_options()) {
                    Ok(loaded) => {
                        // Accepted information goes to stderr before any G-code
                        // exists. Slice has no `--stats`/JSONL diagnostic event;
                        // the plain `note:` line matches `place_model_on_bed`'s
                        // convention.
                        for diagnostic in &loaded.diagnostics {
                            eprintln!("note: scad {}: {}", diagnostic.code, diagnostic.message);
                        }
                        loaded.mesh
                    }
                    Err(e) => {
                        eprintln!("error: failed to load model {}: {e}", model.display());
                        std::process::exit(1);
                    }
                }
            } else {
                match slicer_model_io::load_model(&model) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("error: failed to load model {}: {e}", model.display());
                        std::process::exit(1);
                    }
                }
            };
            #[cfg(not(feature = "scad"))]
            let mesh = match slicer_model_io::load_model(&model) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("error: failed to load model {}: {e}", model.display());
                    std::process::exit(1);
                }
            };
            let mesh = std::sync::Arc::new(place_model_on_bed(mesh, &model, config.as_deref()));
            // Model-authored layer ranges live only in a 3MF's
            // `Metadata/layer_config_ranges.xml` member, so bare meshes
            // (STL/OBJ) can never carry them and are not probed. Parse the
            // member once here — the runtime never touches model bytes — and
            // map its one-based object ordinals onto the loaded MeshIR object
            // order (post-`place_model_on_bed`; that step is a pass-through
            // for 3MF). A malformed member or unmappable ordinal fails the
            // whole slice before any module loads, atomically.
            let layer_ranges: Vec<slicer_config::LayerRangeInput> = if matches!(
                slicer_model_io::detect_format(&model),
                Ok(slicer_model_io::ModelFormat::ThreeMf)
            ) {
                let raw = match slicer_model_io::read_3mf_layer_config_ranges(&model) {
                    Ok(raw) => raw,
                    Err(e) => {
                        eprintln!(
                            "error: failed to read layer ranges from {}: {e}",
                            model.display()
                        );
                        std::process::exit(1);
                    }
                };
                let object_ids: Vec<slicer_ir::ObjectId> = mesh
                    .objects
                    .iter()
                    .map(|object| object.id.clone())
                    .collect();
                match slicer_model_io::map_layer_config_ranges(&raw, &object_ids) {
                    Ok(mapped) => mapped
                        .into_iter()
                        .map(|range| slicer_config::LayerRangeInput {
                            object_id: range.object_id,
                            source_index: range.source_index,
                            min_z: range.min_z,
                            max_z: range.max_z,
                            values: range.values,
                        })
                        .collect(),
                    Err(e) => {
                        eprintln!(
                            "error: failed to map layer ranges from {}: {e}",
                            model.display()
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                Vec::new()
            };
            #[cfg(feature = "report")]
            let (report_opt, report_verbose_opt) = (report, report_verbose);
            #[cfg(not(feature = "report"))]
            let (report_opt, report_verbose_opt): (Option<PathBuf>, bool) = (None, false);
            // Seed the model's authored config (from a 3MF project's
            // project_settings.config) so the G-code / pipeline sees the
            // real, per-model OrcaSlicer settings rather than the
            // serializer's hardcoded defaults. The extractor is generic:
            // every key in the sidecar is ingested with heuristic string
            // coercion (see `coerce_string_to_config_value`), so any
            // OrcaSlicer key PNP learns to consume later is already
            // available in `config_overrides` without further wiring.
            let mut config_overrides: std::collections::HashMap<String, slicer_ir::ConfigValue> =
                std::collections::HashMap::new();
            if let Some(sidecar) = slicer_model_io::read_3mf_project_settings(&model) {
                // `filament_colour` / `extruder_colour` come through as
                // `ConfigValue::List` from the generic extractor (they are
                // JSON arrays in `project_settings.config`). Downstream
                // consumers (`serialize.rs::resolve_filament_colour_csv`,
                // `visual_debug.rs::filament_tool_colors`) accept both
                // `String` (semicolon-joined) and `List`, so we forward the
                // list form directly.
                for (key, value) in sidecar {
                    if key == "thumbnail_path" {
                        // `thumbnail_path` is an invocation-time routing
                        // key for `pipeline.rs` to read a PNG from disk;
                        // a 3MF should never override it — the CLI
                        // `--thumbnail` flag is the correct source.
                        continue;
                    }
                    config_overrides.insert(key, value);
                }
            }
            let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let signal_cancel_flag = cancel_flag.clone();
            if let Err(e) = ctrlc::set_handler(move || {
                signal_cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }) {
                eprintln!("warning: failed to install cancellation signal handler: {e}");
            }
            if cancel_on_stdin_eof {
                let stdin_cancel_flag = cancel_flag.clone();
                std::thread::spawn(move || {
                    let mut stdin = std::io::stdin();
                    let mut buffer = [0_u8; 4096];
                    loop {
                        match std::io::Read::read(&mut stdin, &mut buffer) {
                            Ok(0) => {
                                stdin_cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                                break;
                            }
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                });
            }
            let opts = SliceRunOptions {
                mesh,
                model_label,
                config_path: config,
                output_path: output,
                module_dirs: module_dir,
                no_default_module_paths,
                no_integrated_modules,
                thumbnail,
                report: report_opt,
                report_verbose: report_verbose_opt,
                instrument_stderr,
                profile,
                profile_verbose,
                progress_events: !no_progress_events,
                cancel_flag: Some(cancel_flag.clone()),
                config_overrides,
                layer_ranges,
            };
            match slicer_runtime::run_slice(opts) {
                Ok(outcome) => {
                    // Printed to stderr, after the JSONL stream, so G-code on
                    // stdout is never polluted by a diagnostic.
                    if let Some(summary) = outcome.profile.as_ref() {
                        eprint!("{}", slicer_runtime::format_profile_summary(summary));
                    }
                    if let Some(out_path) = output_path {
                        if let Err(e) = write_with_parents(&out_path, outcome.gcode_text.as_bytes())
                        {
                            eprintln!("error: failed to write output {}: {e}", out_path.display());
                            std::process::exit(1);
                        }
                    } else {
                        print!("{}", outcome.gcode_text);
                    }
                }
                Err(e) => {
                    if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                        if let Some(ref out_path) = output_path {
                            let _ = std::fs::remove_file(out_path);
                        }
                        let output_label = output_path.as_ref().map_or_else(
                            || "<stdout>".to_owned(),
                            |path| path.display().to_string(),
                        );
                        eprintln!("slice cancelled; output not written: {output_label}");
                        std::process::exit(EXIT_CODE_CANCELLED);
                    }
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }

        Cmd::Profile { from, json } => match run_profile_from(&from, json) {
            Ok(rendered) => print!("{rendered}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },

        Cmd::SupportPreview {
            input,
            output,
            config,
            module_dir,
            no_default_module_paths,
        } => {
            if support_preview::run_support_preview(
                &input,
                &output,
                config.as_deref(),
                &module_dir,
                no_default_module_paths,
            )
            .map_err(|e| {
                eprintln!("support-preview failed: {e}");
                e
            })
            .is_err()
            {
                std::process::exit(1);
            }
        }

        Cmd::VisualDebug {
            request,
            output,
            overwrite,
        } => match visual_debug::run_cli(&request, &output, overwrite) {
            Ok(manifest_path) => println!("{}", manifest_path.display()),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },

        // ── module ─────────────────────────────────────────────────────────
        Cmd::Module { action } => match action {
            ModuleCmd::New { dir, name, stage } => {
                if let Err(e) = module_new::execute_in(&dir, &name, &stage) {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
            ModuleCmd::Diagnose {
                module_dir,
                no_default_module_paths,
                no_integrated_modules,
            } => {
                std::process::exit(slicer_runtime::diagnose::run_diagnose(
                    &module_dir,
                    no_default_module_paths,
                    no_integrated_modules,
                ));
            }
            ModuleCmd::ConfigSchema {
                module_dir,
                no_default_module_paths,
                no_integrated_modules,
            } => {
                let search_roots = assemble_search_roots(&module_dir, no_default_module_paths);
                let report = match load_modules_from_roots_with_integrated(
                    &search_roots,
                    &cli_integrated_registrations(no_integrated_modules),
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("error loading modules: {e:?}");
                        std::process::exit(1);
                    }
                };
                let json = build_config_schema_json(&report.modules);
                println!("{}", json);
            }
        },

        // ── mesh ───────────────────────────────────────────────────────────
        Cmd::Mesh { action } => match action {
            MeshCmd::Repair {
                input,
                output,
                format,
                stats,
            } => {
                std::process::exit(helpers_cmd::run_repair(&input, &output, format, stats));
            }
            MeshCmd::Decimate {
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
            MeshCmd::Import {
                input,
                #[cfg(feature = "scad")]
                scad_param,
                #[cfg(feature = "scad")]
                scad_extrude_height,
                output,
                output_format,
                merge_components,
                no_repair,
                stats,
            } => {
                #[cfg(feature = "scad")]
                let scad_options = match scad_cli_options(&scad_param, scad_extrude_height) {
                    Ok(options) => options,
                    Err(message) => {
                        eprintln!("error: {message}");
                        std::process::exit(2);
                    }
                };
                #[cfg(not(feature = "scad"))]
                let scad_options = helpers_cmd::ScadImportOptions::default();
                #[cfg(feature = "scad")]
                {
                    if !scad_options.is_empty()
                        && !matches!(
                            slicer_model_io::detect_format(&input),
                            Ok(slicer_model_io::ModelFormat::Scad)
                        )
                    {
                        eprintln!(
                            "error: SCAD_OPTIONS_REQUIRE_SCAD_INPUT: \
                             --scad-param/--scad-extrude-height require a .scad input ({})",
                            input.display()
                        );
                        std::process::exit(2);
                    }
                }
                std::process::exit(helpers_cmd::run_import_command(
                    &input,
                    &output,
                    output_format,
                    merge_components,
                    no_repair,
                    stats,
                    scad_options,
                ));
            }
            MeshCmd::Convert {
                input,
                output,
                output_format,
                merge_components,
                repair,
            } => {
                std::process::exit(helpers_cmd::run_convert(
                    &input,
                    &output,
                    output_format,
                    merge_components,
                    repair,
                ));
            }
        },

        // ── dag ────────────────────────────────────────────────────────────
        Cmd::Dag { action } => match action {
            DagCmd::Stages {
                module_dir,
                no_default_module_paths,
                no_integrated_modules,
                model: _,
            } => {
                let loaded =
                    load_dag_modules(&module_dir, no_default_module_paths, no_integrated_modules);
                let producers = dag_producers(&loaded);
                print_json(&run_dag_stages(&producers));
            }
            DagCmd::Stage {
                id,
                module_dir,
                no_default_module_paths,
                no_integrated_modules,
                model: _,
            } => {
                let loaded =
                    load_dag_modules(&module_dir, no_default_module_paths, no_integrated_modules);
                let producers = dag_producers(&loaded);
                match run_dag_stage(&producers, &id) {
                    Some(out) => print_json(&out),
                    None => {
                        eprintln!("error: no modules in stage {id:?}");
                        std::process::exit(1);
                    }
                }
            }
            DagCmd::Depends {
                module_id,
                module_dir,
                no_default_module_paths,
                no_integrated_modules,
                model,
            } => {
                let loaded =
                    load_dag_modules(&module_dir, no_default_module_paths, no_integrated_modules);
                let producers = dag_producers(&loaded);
                let object_ids = model.as_deref().and_then(object_ids_from_model);
                match run_dag_depends(&producers, &module_id, object_ids.as_deref()) {
                    Some(out) => print_json(&out),
                    None => {
                        eprintln!("error: module {module_id:?} not found in any loaded manifest");
                        std::process::exit(1);
                    }
                }
            }
            DagCmd::Claims {
                module_dir,
                no_default_module_paths,
                no_integrated_modules,
                model: _,
            } => {
                let loaded =
                    load_dag_modules(&module_dir, no_default_module_paths, no_integrated_modules);
                let producers = dag_producers(&loaded);
                print_json(&run_dag_claims(&producers));
            }
        },
    }
}

/// Place a freshly loaded model on the print bed.
///
/// Bare meshes carry no plate coordinate system — their coordinates are
/// whatever the authoring tool emitted — so they are dropped onto the bed and
/// centred on it. That covers STL and OBJ, plus a SCAD script that was
/// *explicitly* loaded through `load_scad_model` (source coordinates, exactly
/// like an STL). A 3MF carries authored plate placement and is returned
/// untouched.
///
/// Mirrors OrcaSlicer's `ModelObject::ensure_on_bed` and
/// `Model::center_instances_around_point` (`Model.cpp`); canonical's CLI
/// applies the former unconditionally on load (`OrcaSlicer.cpp`).
fn place_model_on_bed(
    mut mesh: slicer_ir::MeshIR,
    model_path: &std::path::Path,
    config_path: Option<&std::path::Path>,
) -> slicer_ir::MeshIR {
    match slicer_model_io::detect_format(model_path) {
        Ok(
            slicer_model_io::ModelFormat::Stl
            | slicer_model_io::ModelFormat::Obj
            | slicer_model_io::ModelFormat::Scad,
        ) => {}
        // 3MF placement is authored; leave it alone. An undetectable format
        // has already failed in `load_model`, so treat it as pass-through.
        _ => return mesh,
    }

    let bed_shape = read_bed_shape(config_path);
    let Some(center) = slicer_model_io::bed_center_mm(&bed_shape) else {
        eprintln!("warning: bed_shape does not describe a usable plate; skipping bed placement");
        return mesh;
    };

    if let Some(p) = slicer_model_io::place_bare_mesh_on_bed(&mut mesh, center) {
        if p.dx != 0.0 || p.dy != 0.0 || p.dz != 0.0 {
            eprintln!(
                "note: placed model on bed centre ({:.1}, {:.1}) mm (shift dx={:.2} dy={:.2} dz={:.2} mm)",
                center.0, center.1, p.dx, p.dy, p.dz
            );
        }
        // `world_z_extent` is cached when the object is assembled at load time.
        // Placement translates the geometry afterwards, so refresh the cache or
        // it would keep describing the pre-placement Z range (a model loaded
        // below Z=0 would still report a negative `z_min` while resting on the
        // plate).
        for object in &mut mesh.objects {
            let extent = slicer_model_io::loader::object_world_z_extent(object);
            object.world_z_extent = extent;
        }
    }
    warn_if_off_bed(&mesh, &bed_shape);
    mesh
}

/// Warn when the model's XY footprint spills past the plate.
///
/// Nothing downstream knows the printer volume (`MeshIR::build_volume` is the
/// model AABB, not the plate), so without this a too-large model slices
/// silently and prints off the bed. Warn rather than fail: `bed_shape` falls
/// back to a 250 x 250 default when unconfigured, and refusing to slice on an
/// unconfigured bed would be worse than printing a warning.
fn warn_if_off_bed(mesh: &slicer_ir::MeshIR, bed_shape: &[f64]) {
    if let Some((over_x, over_y)) = slicer_model_io::bed_overflow_mm(mesh, bed_shape) {
        eprintln!(
            "warning: model extends past the print bed by {over_x:.2} mm in X and              {over_y:.2} mm in Y; slicing anyway, but the print will not fit"
        );
    }
}

/// `bed_shape` from the `--config` JSON, falling back to the resolved default.
fn read_bed_shape(config_path: Option<&std::path::Path>) -> Vec<f64> {
    let from_config = config_path
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| {
            value
                .get("bed_shape")?
                .as_array()?
                .iter()
                .map(serde_json::Value::as_f64)
                .collect::<Option<Vec<f64>>>()
        });
    from_config.unwrap_or_else(|| slicer_ir::ResolvedConfig::default().bed_shape)
}

// ---------------------------------------------------------------------------
// SCAD bed placement (inline, feature-gated)
// ---------------------------------------------------------------------------

/// Placement coverage for the production `place_model_on_bed` helper.
///
/// Each test builds its mesh through the real `load_scad_model` ingestion path,
/// then calls the production placement function directly. Every expected
/// coordinate is derived analytically from the unplaced AABB and the selected
/// bed centre, never from observed placed output, so a wrong translation cannot
/// ratify itself.
///
/// Vertices are `f32`, and every fixture coordinate and shift below is an exact
/// integer multiple of 1 mm, so `EPS_MM = 1e-3` absorbs `f32` round-off only.
#[cfg(all(test, feature = "scad"))]
mod scad_placement_tests {
    use std::path::{Path, PathBuf};

    use super::place_model_on_bed;

    /// Tolerance for placed coordinates: `f32` vertices with
    /// integer-millimetre fixtures and shifts.
    const EPS_MM: f32 = 1e-3;

    /// Two disjoint bodies, deliberately off-centre and below the plate.
    /// Unplaced AABB: `x[-30,-18] y[5,11] z[-4,4]`.
    const TWO_BODY: &str =
        "translate([-30,5,-4]) cube([4,6,8]);\ntranslate([-20,7,-2]) cube([2,2,4]);\n";

    fn write_fixture(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).expect("write fixture");
        path
    }

    /// The default `bed_shape` as the resolved config exposes it, so the
    /// expected centre is derived rather than hardcoded.
    fn default_bed_shape() -> Vec<f64> {
        slicer_ir::ResolvedConfig::default().bed_shape
    }

    /// Load a fixture through the production SCAD ingest path (`load_model`
    /// refuses implicit evaluation by design).
    fn load_scad(path: &Path) -> slicer_ir::MeshIR {
        slicer_model_io::load_scad_model(path, &slicer_model_io::ScadEvalOptions::default())
            .unwrap_or_else(|e| panic!("load_scad_model {}: {e}", path.display()))
            .mesh
    }

    fn aabb_of(vertices: impl Iterator<Item = (f32, f32, f32)>) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for (x, y, z) in vertices {
            min[0] = min[0].min(x);
            min[1] = min[1].min(y);
            min[2] = min[2].min(z);
            max[0] = max[0].max(x);
            max[1] = max[1].max(y);
            max[2] = max[2].max(z);
        }
        (min, max)
    }

    /// One vertex in world space, applying the object transform with the host's
    /// zero-matrix-is-identity convention.
    ///
    /// Placement folds its shift into `transform.matrix` (the loader assembles
    /// objects with a real identity matrix, and `place_bare_mesh_on_bed` only
    /// rewrites vertices for an all-zero matrix), so raw vertices are *not*
    /// where the placed geometry lives. Reading them directly would inspect the
    /// pre-placement coordinates and ratify a broken translation.
    fn world_vertex(object: &slicer_ir::ObjectMesh, vertex: &slicer_ir::Point3) -> (f32, f32, f32) {
        let m = &object.transform.matrix;
        if m.iter().all(|value| *value == 0.0) {
            return (vertex.x, vertex.y, vertex.z);
        }
        let (x, y, z) = (vertex.x as f64, vertex.y as f64, vertex.z as f64);
        (
            (m[0] * x + m[4] * y + m[8] * z + m[12]) as f32,
            (m[1] * x + m[5] * y + m[9] * z + m[13]) as f32,
            (m[2] * x + m[6] * y + m[10] * z + m[14]) as f32,
        )
    }

    /// World-space AABB of the vertices matching a geometric filter. Splitting
    /// the two cubes by their documented X gap avoids depending on triangle
    /// connectivity and cannot absorb a placement error.
    fn subset_aabb(
        mesh: &slicer_ir::MeshIR,
        keep: impl Fn(f32, f32, f32) -> bool,
    ) -> ([f32; 3], [f32; 3]) {
        aabb_of(
            mesh.objects
                .iter()
                .flat_map(|object| {
                    object
                        .mesh
                        .vertices
                        .iter()
                        .map(move |vertex| world_vertex(object, vertex))
                })
                .filter(|(x, y, z)| keep(*x, *y, *z)),
        )
    }

    fn mesh_aabb(mesh: &slicer_ir::MeshIR) -> ([f32; 3], [f32; 3]) {
        subset_aabb(mesh, |_, _, _| true)
    }

    fn assert_aabb(
        mesh: &slicer_ir::MeshIR,
        expected_min: [f32; 3],
        expected_max: [f32; 3],
        context: &str,
    ) {
        let (min, max) = mesh_aabb(mesh);
        for axis in 0..3 {
            assert!(
                (min[axis] - expected_min[axis]).abs() <= EPS_MM,
                "{context}: min[{axis}] expected {} got {} (tolerance {EPS_MM})",
                expected_min[axis],
                min[axis]
            );
            assert!(
                (max[axis] - expected_max[axis]).abs() <= EPS_MM,
                "{context}: max[{axis}] expected {} got {} (tolerance {EPS_MM})",
                expected_max[axis],
                max[axis]
            );
        }
    }

    fn assert_vertices_untouched(
        placed: &slicer_ir::MeshIR,
        original: &slicer_ir::MeshIR,
        context: &str,
    ) {
        assert_eq!(
            placed.objects.len(),
            original.objects.len(),
            "{context}: object count"
        );
        for (placed_object, original_object) in placed.objects.iter().zip(&original.objects) {
            assert_eq!(placed_object.id, original_object.id, "{context}: object id");
            assert_eq!(
                placed_object.mesh.vertices.len(),
                original_object.mesh.vertices.len(),
                "{context}: vertex count"
            );
            for (index, (placed_vertex, original_vertex)) in placed_object
                .mesh
                .vertices
                .iter()
                .zip(&original_object.mesh.vertices)
                .enumerate()
            {
                assert_eq!(
                    placed_vertex, original_vertex,
                    "{context}: vertex {index} must be untouched"
                );
            }
            assert_eq!(
                placed_object.mesh.indices, original_object.mesh.indices,
                "{context}: indices"
            );
            assert_eq!(
                placed_object.transform.matrix, original_object.transform.matrix,
                "{context}: transform"
            );
            assert_eq!(
                placed_object.world_z_extent, original_object.world_z_extent,
                "{context}: cached world Z extent"
            );
        }
        assert_eq!(
            placed.build_volume, original.build_volume,
            "{context}: build volume"
        );
    }

    #[test]
    fn scad_placement_centers_and_drops_to_bed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let scad = write_fixture(dir.path(), "two_body.scad", TWO_BODY);
        let unplaced = load_scad(&scad);

        // Fixture AABB, measured from the unplaced source coordinates.
        let (min, max) = mesh_aabb(&unplaced);
        assert_aabb(
            &unplaced,
            [-30.0, 5.0, -4.0],
            [-18.0, 11.0, 4.0],
            "unplaced two-body fixture",
        );

        // Analytic expectation: whole-model translation onto the bed centre,
        // dropped so `min_z` becomes 0.
        let center = slicer_model_io::bed_center_mm(&default_bed_shape())
            .expect("the default bed_shape describes a plate");
        assert!(
            (center.0 - 125.0).abs() <= EPS_MM && (center.1 - 125.0).abs() <= EPS_MM,
            "default bed centre must be (125, 125), got {center:?}"
        );
        let dx = center.0 - (min[0] + max[0]) / 2.0;
        let dy = center.1 - (min[1] + max[1]) / 2.0;
        let dz = -min[2];
        assert!(
            (dx - 149.0).abs() <= EPS_MM && (dy - 117.0).abs() <= EPS_MM,
            "expected dx=149 dy=117 from the unplaced AABB, got ({dx}, {dy})"
        );
        assert!((dz - 4.0).abs() <= EPS_MM, "expected dz=4, got {dz}");

        let original_id = unplaced.objects[0].id.clone();
        let placed = place_model_on_bed(unplaced, &scad, None);

        assert_eq!(
            placed.objects.len(),
            1,
            "SCAD ingestion fuses into a single object; placement must not split it"
        );
        assert_eq!(
            placed.objects[0].id, original_id,
            "bed placement must not re-identify the object"
        );
        assert_aabb(
            &placed,
            [min[0] + dx, min[1] + dy, min[2] + dz],
            [max[0] + dx, max[1] + dy, max[2] + dz],
            "placed two-body fixture (analytic)",
        );
        // The same expectation in absolute millimetres: x[119,131] y[122,128]
        // z[0,8].
        assert_aabb(
            &placed,
            [119.0, 122.0, 0.0],
            [131.0, 128.0, 8.0],
            "placed two-body fixture (absolute)",
        );

        let extent = placed.objects[0]
            .world_z_extent
            .expect("a placed solid keeps a world Z extent");
        assert!(
            (extent.0 - 0.0).abs() <= EPS_MM && (extent.1 - 8.0).abs() <= EPS_MM,
            "cached world_z_extent must follow placement: expected (0, 8), got {extent:?}"
        );
    }

    #[test]
    fn scad_placement_preserves_disconnected_offsets() {
        let dir = tempfile::tempdir().expect("tempdir");
        let scad = write_fixture(dir.path(), "two_body.scad", TWO_BODY);
        let unplaced = load_scad(&scad);

        // The two cubes are disjoint. Split them at the midpoint of the X gap
        // (-26 .. -20) so each body's AABB is measured independently.
        let (a_min, a_max) = subset_aabb(&unplaced, |x, _, _| x < -25.0);
        let (b_min, b_max) = subset_aabb(&unplaced, |x, _, _| x >= -25.0);
        assert!(
            (a_min[0] - (-30.0)).abs() <= EPS_MM
                && (a_max[0] - (-26.0)).abs() <= EPS_MM
                && (a_min[1] - 5.0).abs() <= EPS_MM
                && (a_max[1] - 11.0).abs() <= EPS_MM
                && (a_min[2] - (-4.0)).abs() <= EPS_MM
                && (a_max[2] - 4.0).abs() <= EPS_MM,
            "first body pre-placement AABB must be x[-30,-26] y[5,11] z[-4,4], got {a_min:?}..{a_max:?}"
        );
        assert!(
            (b_min[0] - (-20.0)).abs() <= EPS_MM
                && (b_max[0] - (-18.0)).abs() <= EPS_MM
                && (b_min[1] - 7.0).abs() <= EPS_MM
                && (b_max[1] - 9.0).abs() <= EPS_MM
                && (b_min[2] - (-2.0)).abs() <= EPS_MM
                && (b_max[2] - 2.0).abs() <= EPS_MM,
            "second body pre-placement AABB must be x[-20,-18] y[7,9] z[-2,2], got {b_min:?}..{b_max:?}"
        );

        let center = slicer_model_io::bed_center_mm(&default_bed_shape())
            .expect("the default bed_shape describes a plate");
        let placed = place_model_on_bed(unplaced, &scad, None);

        // The whole model is one rigid translation, so the bodies stay on their
        // own sides of the X gap and can be re-split with the same filter.
        let (placed_a_min, placed_a_max) = subset_aabb(&placed, |x, _, _| x < 125.0);
        let (placed_b_min, placed_b_max) = subset_aabb(&placed, |x, _, _| x >= 125.0);

        // Both bodies must report the same delta. Per-body centring would move
        // them differently and fail here.
        let delta_a = [
            placed_a_min[0] - a_min[0],
            placed_a_min[1] - a_min[1],
            placed_a_min[2] - a_min[2],
        ];
        let delta_b = [
            placed_b_min[0] - b_min[0],
            placed_b_min[1] - b_min[1],
            placed_b_min[2] - b_min[2],
        ];
        for axis in 0..3 {
            assert!(
                (delta_a[axis] - delta_b[axis]).abs() <= EPS_MM,
                "both bodies must receive the same shift; axis {axis}: {delta_a:?} vs {delta_b:?}"
            );
        }

        // The inter-body offset vector is invariant under a rigid translation.
        for axis in 0..3 {
            assert!(
                ((placed_b_min[axis] - placed_a_min[axis]) - (b_min[axis] - a_min[axis])).abs()
                    <= EPS_MM,
                "inter-body offset min[{axis}] changed under placement"
            );
            assert!(
                ((placed_b_max[axis] - placed_a_max[axis]) - (b_max[axis] - a_max[axis])).abs()
                    <= EPS_MM,
                "inter-body offset max[{axis}] changed under placement"
            );
        }

        // Analytic absolute placement, from the same whole-model expectation as
        // the centring test: body A x[119,123] y[122,128] z[0,8], body B
        // x[129,131] y[124,126] z[2,6].
        let dx = center.0 - (-24.0);
        let dy = center.1 - 8.0;
        let dz = 4.0;
        assert!(
            (placed_a_min[0] - (a_min[0] + dx)).abs() <= EPS_MM
                && (placed_a_max[0] - (a_max[0] + dx)).abs() <= EPS_MM
                && (placed_b_min[0] - (b_min[0] + dx)).abs() <= EPS_MM
                && (placed_b_max[0] - (b_max[0] + dx)).abs() <= EPS_MM
                && (placed_a_min[1] - (a_min[1] + dy)).abs() <= EPS_MM
                && (placed_a_max[1] - (a_max[1] + dy)).abs() <= EPS_MM
                && (placed_a_min[2] - (a_min[2] + dz)).abs() <= EPS_MM
                && (placed_a_max[2] - (a_max[2] + dz)).abs() <= EPS_MM
                && (placed_b_min[2] - (b_min[2] + dz)).abs() <= EPS_MM
                && (placed_b_max[2] - (b_max[2] + dz)).abs() <= EPS_MM,
            "per-body placed AABBs must equal their pre-placement AABB plus ({dx}, {dy}, {dz}); \
             got A {placed_a_min:?}..{placed_a_max:?}, B {placed_b_min:?}..{placed_b_max:?}"
        );
    }

    #[test]
    fn scad_placement_uses_configured_bed_centre() {
        let dir = tempfile::tempdir().expect("tempdir");
        let scad = write_fixture(dir.path(), "two_body.scad", TWO_BODY);
        let config = write_fixture(
            dir.path(),
            "bed.json",
            r#"{"bed_shape":[0,0,100,0,100,100,0,100]}"#,
        );
        let unplaced = load_scad(&scad);
        let (min, max) = mesh_aabb(&unplaced);

        let configured_bed_shape = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let center = slicer_model_io::bed_center_mm(&configured_bed_shape)
            .expect("the configured bed_shape describes a plate");
        assert!(
            (center.0 - 50.0).abs() <= EPS_MM && (center.1 - 50.0).abs() <= EPS_MM,
            "configured bed centre must be (50, 50), got {center:?}"
        );
        let dx = center.0 - (min[0] + max[0]) / 2.0;
        let dy = center.1 - (min[1] + max[1]) / 2.0;
        let dz = -min[2];
        assert!(
            (dx - 74.0).abs() <= EPS_MM
                && (dy - 42.0).abs() <= EPS_MM
                && (dz - 4.0).abs() <= EPS_MM,
            "expected dx=74 dy=42 dz=4 from the unplaced AABB, got ({dx}, {dy}, {dz})"
        );

        let placed = place_model_on_bed(unplaced, &scad, Some(&config));
        // x[-30,-18]+74 = [44,56]; y[5,11]+42 = [47,53]; z[-4,4]+4 = [0,8].
        assert_aabb(
            &placed,
            [44.0, 47.0, 0.0],
            [56.0, 53.0, 8.0],
            "configured-bed placement",
        );
    }

    #[test]
    fn scad_placement_leaves_3mf_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let scad = write_fixture(dir.path(), "two_body.scad", TWO_BODY);
        let unplaced = load_scad(&scad);
        assert_aabb(
            &unplaced,
            [-30.0, 5.0, -4.0],
            [-18.0, 11.0, 4.0],
            "3MF pass-through input (deliberately off-centre and below Z=0)",
        );

        // The helper branches on the detected extension alone, and
        // `detect_format` reads only the extension, so this path never has to
        // exist: if placement ever tried to open the model, the test would fail
        // loudly instead of silently passing.
        let authored_path = dir.path().join("authored.3mf");
        assert!(
            !authored_path.exists(),
            "the 3MF pass-through path is intentionally absent"
        );

        let placed = place_model_on_bed(unplaced.clone(), &authored_path, None);
        assert_vertices_untouched(&placed, &unplaced, "3MF pass-through");
        assert_eq!(placed, unplaced, "a 3MF model must be returned unchanged");
    }

    #[test]
    fn scad_placement_skips_when_bed_shape_is_unusable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let scad = write_fixture(dir.path(), "two_body.scad", TWO_BODY);
        let config = write_fixture(dir.path(), "empty_bed.json", r#"{"bed_shape":[]}"#);
        let unplaced = load_scad(&scad);

        // `bed_center_mm` rejects a polygon with fewer than three points, so the
        // existing warning path returns the mesh in source coordinates.
        assert!(
            slicer_model_io::bed_center_mm(&[]).is_none(),
            "an empty bed_shape must not describe a plate"
        );

        let placed = place_model_on_bed(unplaced.clone(), &scad, Some(&config));
        assert_aabb(
            &placed,
            [-30.0, 5.0, -4.0],
            [-18.0, 11.0, 4.0],
            "unusable bed_shape keeps source coordinates",
        );
        assert_vertices_untouched(&placed, &unplaced, "unusable bed_shape");
        assert_eq!(
            placed, unplaced,
            "an unusable bed_shape must leave the mesh unchanged"
        );
    }
}
