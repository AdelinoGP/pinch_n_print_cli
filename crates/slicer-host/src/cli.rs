//! CLI argument parsing for the slicer-host binary.

use clap::{Parser, Subcommand};
use std::fmt;
use std::path::PathBuf;

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
        /// Path to a compiled WASM module (legacy; modules are normally
        /// discovered from --module-dir directories).
        #[arg(long)]
        module: Option<String>,
        /// Path to the input 3D model (STL, OBJ, or 3MF).
        #[arg(long)]
        model: String,
        /// Path to a JSON configuration file.
        #[arg(long)]
        config: Option<String>,
        /// Path to the output G-code file (default: stdout).
        #[arg(long)]
        output: Option<String>,
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
    /// Optional path to a compiled WASM module (legacy).
    pub module_path: Option<PathBuf>,
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
}

/// Errors from CLI argument validation.
#[derive(Debug)]
pub enum CliError {
    /// Module WASM file not found.
    MissingModule(PathBuf),
    /// Model file not found.
    MissingModel(PathBuf),
    /// Config file not found.
    MissingConfig(PathBuf),
    /// Module directory not found.
    MissingModuleDir(PathBuf),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::MissingModule(p) => write!(f, "module file not found: {}", p.display()),
            CliError::MissingModel(p) => write!(f, "model file not found: {}", p.display()),
            CliError::MissingConfig(p) => write!(f, "config file not found: {}", p.display()),
            CliError::MissingModuleDir(p) => {
                write!(f, "module directory not found: {}", p.display())
            }
        }
    }
}

impl std::error::Error for CliError {}

/// Validate parsed CLI run arguments into [`HostRunOptions`].
///
/// Checks that module, model, and config (if provided) files exist, and
/// that every supplied module directory exists. Path-assembly precedence
/// (CLI > env > platform defaults) is the responsibility of
/// [`crate::assemble_search_roots`]; this validator only confirms that
/// each user-supplied directory is present.
///
/// # Errors
///
/// Returns [`CliError`] if any required path does not exist.
pub fn validate_run_options(
    module: Option<&str>,
    model: &str,
    config: Option<&str>,
    output: Option<&str>,
    module_dirs: &[&str],
) -> Result<HostRunOptions, CliError> {
    let module_path = if let Some(module) = module {
        let p = PathBuf::from(module);
        if !p.exists() {
            return Err(CliError::MissingModule(p));
        }
        Some(p)
    } else {
        None
    };

    let model_path = PathBuf::from(model);
    if !model_path.exists() {
        return Err(CliError::MissingModel(model_path));
    }

    let config_path = if let Some(cfg) = config {
        let p = PathBuf::from(cfg);
        if !p.exists() {
            return Err(CliError::MissingConfig(p));
        }
        Some(p)
    } else {
        None
    };

    let mut module_dir_paths: Vec<PathBuf> = Vec::with_capacity(module_dirs.len());
    for dir in module_dirs {
        let p = PathBuf::from(dir);
        if !p.exists() {
            return Err(CliError::MissingModuleDir(p));
        }
        module_dir_paths.push(p);
    }

    let output_path = output.map(PathBuf::from);

    Ok(HostRunOptions {
        module_path,
        model_path,
        config_path,
        output_path,
        module_dirs: module_dir_paths,
        no_default_module_paths: false,
    })
}
