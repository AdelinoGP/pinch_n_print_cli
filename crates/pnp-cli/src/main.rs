//! Binary entry point for the pnp_cli (Pinch'n'Print) unified CLI.
//!
//! Thin clap dispatcher over a noun-namespaced verb tree:
//! - `slice`         — one-shot slice: model → G-code
//! - `module`        — module-author commands (new / diagnose / config-schema)
//! - `mesh`          — mesh ops (repair / decimate / import)
//! - `dag`           — DAG introspection (stages / stage / depends / claims)

use pnp_cli::io::{write_with_parents, OutputFormat};
use pnp_cli::module_new;

mod helpers_cmd;

use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand};
use slicer_runtime::{runtime_builtins, SliceRunOptions};
use slicer_scheduler::dag_cli::{run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages};
use slicer_scheduler::{
    assemble_search_roots, build_config_schema_json, load_modules_from_roots, LoadedModule,
    Producer,
};

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
    },
    /// Emit combined config schema JSON from loaded modules.
    ConfigSchema {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
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
        /// Optional model file.
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_dag_modules(module_dir: &[PathBuf], no_default_module_paths: bool) -> Vec<LoadedModule> {
    let search_roots = assemble_search_roots(module_dir, no_default_module_paths);
    match load_modules_from_roots(&search_roots) {
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

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();
    match cli.cmd {
        // ── slice ──────────────────────────────────────────────────────────
        Cmd::Slice {
            model,
            config,
            output,
            module_dir,
            no_default_module_paths,
            thumbnail,
            #[cfg(feature = "report")]
            report,
            #[cfg(feature = "report")]
            report_verbose,
            instrument_stderr,
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
            let output_path = output.clone();
            let model_label = model.to_string_lossy().into_owned();
            let mesh = match slicer_model_io::load_model(&model) {
                Ok(m) => std::sync::Arc::new(m),
                Err(e) => {
                    eprintln!("error: failed to load model {}: {e}", model.display());
                    std::process::exit(1);
                }
            };
            #[cfg(feature = "report")]
            let (report_opt, report_verbose_opt) = (report, report_verbose);
            #[cfg(not(feature = "report"))]
            let (report_opt, report_verbose_opt): (Option<PathBuf>, bool) = (None, false);
            let opts = SliceRunOptions {
                mesh,
                model_label,
                config_path: config,
                output_path: output,
                module_dirs: module_dir,
                no_default_module_paths,
                thumbnail,
                report: report_opt,
                report_verbose: report_verbose_opt,
                instrument_stderr,
            };
            match slicer_runtime::run_slice(opts) {
                Ok(outcome) => {
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
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }

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
            } => {
                std::process::exit(slicer_runtime::diagnose::run_diagnose(
                    &module_dir,
                    no_default_module_paths,
                ));
            }
            ModuleCmd::ConfigSchema {
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
                model: _,
            } => {
                let loaded = load_dag_modules(&module_dir, no_default_module_paths);
                let producers = dag_producers(&loaded);
                print_json(&run_dag_stages(&producers));
            }
            DagCmd::Stage {
                id,
                module_dir,
                no_default_module_paths,
                model: _,
            } => {
                let loaded = load_dag_modules(&module_dir, no_default_module_paths);
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
                model,
            } => {
                let loaded = load_dag_modules(&module_dir, no_default_module_paths);
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
                model: _,
            } => {
                let loaded = load_dag_modules(&module_dir, no_default_module_paths);
                let producers = dag_producers(&loaded);
                print_json(&run_dag_claims(&producers));
            }
        },
    }
}
