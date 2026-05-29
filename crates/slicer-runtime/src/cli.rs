//! CLI argument parsing types reused by the `pnp_cli` binary.

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

/// Output mesh formats accepted by the `repair`, `decimate`, and `import`
/// subcommands. Only [`OutputFormat::Stl`] is wired through at present —
/// `Obj` and `ThreeMf` parse cleanly but produce a runtime error at the
/// write step until the corresponding writers land.
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
#[value(rename_all = "lower")]
pub enum OutputFormat {
    /// Binary STL.
    Stl,
    /// Wavefront OBJ. Not yet implemented; will error at runtime.
    Obj,
    /// 3MF. Not yet implemented; will error at runtime.
    #[value(name = "3mf")]
    ThreeMf,
}

/// Write `contents` to `path`, creating any missing parent directories first.
///
/// Centralises the "create parent dir, then write" pattern used by the CLI for
/// both `--output` G-code and `--report` HTML writes so each call site reports
/// directory-creation failures distinctly from file-write failures.
pub fn write_with_parents(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)
}

/// Top-level CLI parser type, retained as a library surface for parser-shape
/// tests. The `pnp_cli` binary defines its own clap structure (noun-namespaced
/// verb tree); `HostCli` is no longer the program entry point.
#[derive(Parser, Debug)]
#[command(name = "pnp_cli", version, about = "ModularSlicer host runtime")]
pub struct HostCli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: HostCommands,
}

/// Available host subcommands.
#[derive(Subcommand, Debug)]
pub enum HostCommands {
    /// Run the slicing pipeline on a model.
    Run {
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
        /// When omitted, only platform default paths and
        /// `SLICER_MODULE_PATH` (env) entries are searched.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths
        /// (`{config_dir}/modules/` and `{executable_dir}/modules/`).
        /// `--module-dir` and `SLICER_MODULE_PATH` still take effect.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Path to a PNG thumbnail image to embed in the G-code header.
        #[arg(long)]
        thumbnail: Option<PathBuf>,
        /// Optional path for an HTML slicer report (timing / memory /
        /// parallelism explainer). When absent, no report-related
        /// instrumentation is installed — zero overhead.
        #[arg(long, value_name = "PATH.html")]
        report: Option<PathBuf>,
        /// Verbose report mode (per-layer-per-module rows). Requires
        /// `--report`. Defaults to off to keep page size small.
        #[arg(long, requires = "report")]
        report_verbose: bool,
        /// Emit per-stage / per-module timing events on the stderr JSONL
        /// stream during the slice. Bumps the event-schema version to
        /// `"1.1.0"` and is composable with `--report`.
        #[arg(long = "instrument-stderr")]
        instrument_stderr: bool,
    },
    /// Query the combined config schema from loaded modules.
    ConfigSchema {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths
        /// (`{config_dir}/modules/` and `{executable_dir}/modules/`).
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
    },
    /// Introspect the static module DAG produced by manifest TOML parsing
    /// alone (no WASM compilation, no slicing). See
    /// `docs/specs/agent-cli-debugging.md` §4.3.
    Dag {
        /// The dag introspection subcommand to run.
        #[command(subcommand)]
        cmd: DagSubcommand,
    },
    /// Validate the discovered module set and emit structured diagnostics.
    /// Exits 0 on pass, 1 on errors, 2 on unreadable files.
    Diagnose {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
    },
    /// Repair a mesh: degenerate removal, orientation normalization,
    /// open-edge closure. See `docs/13_slicer_helpers_crate.md`.
    Repair {
        /// Input mesh file (STL, OBJ, or 3MF).
        #[arg(long)]
        input: PathBuf,
        /// Output mesh file path.
        #[arg(long)]
        output: PathBuf,
        /// Output format. Defaults to inferring from the `--output` extension
        /// (or matching the input format if extension is absent).
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
        /// Print repair statistics to stderr as line-delimited JSON.
        #[arg(long)]
        stats: bool,
    },
    /// Reduce triangle count via QEM edge collapse. See
    /// `docs/13_slicer_helpers_crate.md`.
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
        /// Absolute target triangle count. Mutually exclusive with
        /// `--target-ratio`.
        #[arg(long)]
        target_count: Option<usize>,
        /// Fraction of original triangle count to retain (0.0–1.0). Mutually
        /// exclusive with `--target-count`.
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
    /// Import a STEP/STP file as a triangulated mesh. See
    /// `docs/13_slicer_helpers_crate.md`.
    Import {
        /// Input STEP or STP file.
        #[arg(long)]
        input: PathBuf,
        /// Output mesh file path. With multiple solids and no
        /// `--merge-components`, used as a stem: `<stem>_0.<ext>`, etc.
        #[arg(long)]
        output: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Stl)]
        output_format: OutputFormat,
        /// Merge all solids into a single mesh before output.
        #[arg(long)]
        merge_components: bool,
        /// Skip the automatic repair pass applied after tessellation
        /// (not recommended).
        #[arg(long)]
        no_repair: bool,
        /// Print import statistics to stderr as line-delimited JSON.
        #[arg(long)]
        stats: bool,
    },
}

/// Subcommands under `pnp_cli dag` for DAG introspection.
#[derive(Subcommand, Debug)]
pub enum DagSubcommand {
    /// List every discovered stage with its tier, module count, and claim count.
    Stages {
        /// Directory to search for modules. May be repeated.
        #[arg(long = "module-dir", value_name = "PATH")]
        module_dir: Vec<PathBuf>,
        /// Disable the platform default module search paths.
        #[arg(long = "no-default-module-paths")]
        no_default_module_paths: bool,
        /// Optional model file. When supplied, object ids extracted from the
        /// 3MF sidecar are surfaced where relevant — the module set itself is
        /// unchanged.
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
        /// Optional model file (see `dag stages --model` for semantics).
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
        /// Optional model file. When supplied, object ids extracted from the
        /// 3MF sidecar are attached to the depends output.
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
        /// Optional model file (see `dag stages --model` for semantics).
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
    },
}

/// Validated runtime options derived from CLI arguments.
#[derive(Debug, Clone)]
pub struct SliceRunOptions {
    /// Path to the input 3D model.
    pub model_path: PathBuf,
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
    /// Optional path for an HTML slicer report.
    pub report: Option<PathBuf>,
    /// Verbose report mode (per-layer-per-module rows).
    pub report_verbose: bool,
    /// When true, emit per-stage / per-module timing events on the stderr
    /// JSONL stream during the slice (schema version `"1.1.0"`).
    pub instrument_stderr: bool,
}
