//! Binary entry point for the slicer-host runtime.
//!
//! Parses CLI arguments via clap and dispatches to the pipeline orchestration
//! or config-schema query functions.

use clap::Parser;
use slicer_host::{HostCli, HostCommands};

fn main() {
    let cli = HostCli::parse();
    match cli.command {
        HostCommands::Run {
            module: _,
            model: _,
            config: _,
            output: _,
            module_dir: _,
        } => {
            // TODO(TASK-076): Wire up model loading, module discovery, and pipeline execution.
            // For now, print a placeholder message indicating the pipeline would run.
            eprintln!("slicer-host: run command not yet fully wired (see TASK-076)");
            std::process::exit(1);
        }
        HostCommands::ConfigSchema { module_dir: _ } => {
            // TODO: Wire up module discovery and schema aggregation.
            eprintln!("slicer-host: config-schema command not yet fully wired");
            std::process::exit(1);
        }
    }
}
