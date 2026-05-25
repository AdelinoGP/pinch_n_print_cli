//! CLI argument parsing for the slicer-host binary.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

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

/// Top-level CLI parser for the slicer-host binary.
#[derive(Parser, Debug)]
#[command(name = "slicer-host", version, about = "ModularSlicer host runtime")]
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
}

/// Validated runtime options derived from CLI arguments.
#[derive(Debug, Clone)]
pub struct HostRunOptions {
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
}
