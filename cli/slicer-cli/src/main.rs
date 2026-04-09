//! `slicer` — developer CLI for the ModularSlicer module SDK.

use slicer_cli::cmd_build;
use slicer_cli::cmd_new;
use slicer_cli::cmd_test;

use clap::{Parser, Subcommand};

/// ModularSlicer developer CLI.
#[derive(Parser, Debug)]
#[command(name = "slicer", version, about = "ModularSlicer module development CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Scaffold a new module project.
    New {
        /// Module name (kebab-case, e.g. "my-infill").
        name: String,
        /// Pipeline stage for the module.
        #[arg(long, default_value = "Layer::Infill")]
        stage: String,
    },
    /// Compile the current module to WASM.
    Build {
        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },
    /// Run the module's test suite.
    Test {
        /// Extra arguments passed to cargo test.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Validate the module manifest without building.
    Validate,
    /// Run the local module against a real model.
    Run {
        /// Path to the input STL model.
        #[arg(long)]
        model: String,
        /// Path to the config JSON.
        #[arg(long)]
        config: Option<String>,
        /// Path to the output G-code file (default: stdout).
        #[arg(long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, stage } => {
            if let Err(e) = cmd_new::execute(&name, &stage) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Build { release } => {
            if let Err(e) = cmd_build::execute(release) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Test { args } => {
            if let Err(e) = cmd_test::execute(&args) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Validate => {
            eprintln!("slicer validate: not yet implemented");
            std::process::exit(1);
        }
        Commands::Run { .. } => {
            eprintln!("slicer run: not yet implemented");
            std::process::exit(1);
        }
    }
}
