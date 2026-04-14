//! Implementation of the `slicer build` subcommand.
//!
//! Compiles the current module to WASM by running `cargo build --target wasm32-unknown-unknown`
//! followed by `wasm-tools component new` to produce a Component Model binary.
//! Output is placed at `target/slicer/<module-name>.wasm`.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Errors that can occur during the build process.
#[derive(Debug)]
pub enum BuildError {
    /// No Cargo.toml found in the current directory.
    MissingCargoToml,
    /// Cargo.toml could not be parsed or is missing required fields.
    CargoTomlParseError(String),
    /// `cargo build` returned a non-zero exit code.
    CargoBuildFailed(String),
    /// `wasm-tools component new` returned a non-zero exit code.
    WasmToolsFailed(String),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCargoToml => write!(f, "Cargo.toml not found in the current directory"),
            Self::CargoTomlParseError(msg) => write!(f, "failed to parse Cargo.toml: {msg}"),
            Self::CargoBuildFailed(msg) => write!(f, "cargo build failed: {msg}"),
            Self::WasmToolsFailed(msg) => write!(f, "wasm-tools component new failed: {msg}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Parse the module (package) name from `Cargo.toml` in the given directory.
///
/// # Errors
///
/// Returns [`BuildError::MissingCargoToml`] if no `Cargo.toml` exists,
/// or [`BuildError::CargoTomlParseError`] if parsing fails or `[package].name` is missing.
///
/// # Examples
///
/// ```no_run
/// # use slicer_cli::cmd_build;
/// let name = cmd_build::parse_module_name(std::path::Path::new(".")).unwrap();
/// println!("Module: {name}");
/// ```
pub fn parse_module_name(dir: &Path) -> Result<String, BuildError> {
    let cargo_path = dir.join("Cargo.toml");
    if !cargo_path.exists() {
        return Err(BuildError::MissingCargoToml);
    }

    let content = fs::read_to_string(&cargo_path)?;
    let table: toml::Table = content
        .parse()
        .map_err(|e: toml::de::Error| BuildError::CargoTomlParseError(e.to_string()))?;

    table
        .get("package")
        .and_then(|pkg| pkg.get("name"))
        .and_then(|name| name.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| BuildError::CargoTomlParseError("missing [package].name".to_string()))
}

/// Check whether the `Cargo.toml` in the given directory declares `crate-type = ["cdylib"]`.
///
/// Returns `false` if no `[lib]` section exists or if `cdylib` is not in the crate-type list.
///
/// # Errors
///
/// Returns an error if `Cargo.toml` cannot be read or parsed.
pub fn has_cdylib(dir: &Path) -> Result<bool, BuildError> {
    let cargo_path = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_path)?;
    let table: toml::Table = content
        .parse()
        .map_err(|e: toml::de::Error| BuildError::CargoTomlParseError(e.to_string()))?;

    let has = table
        .get("lib")
        .and_then(|lib| lib.get("crate-type"))
        .and_then(|ct| ct.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some("cdylib")))
        .unwrap_or(false);

    Ok(has)
}

/// Compute the path where `cargo build` places the core WASM module.
///
/// Cargo outputs to `target/wasm32-unknown-unknown/{debug|release}/{name}.wasm`,
/// where hyphens in the crate name are replaced with underscores.
pub fn core_wasm_output_path(module_name: &str, release: bool) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    let file_name = format!("{}.wasm", module_name.replace('-', "_"));
    PathBuf::from("target")
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(file_name)
}

/// Compute the final output path: `target/slicer/<module-name>.wasm`.
pub fn final_output_path(module_name: &str) -> PathBuf {
    PathBuf::from("target")
        .join("slicer")
        .join(format!("{module_name}.wasm"))
}

/// Build the argument list for `cargo build --target wasm32-unknown-unknown [--release]`.
pub fn cargo_build_args(release: bool) -> Vec<&'static str> {
    let mut args = vec!["build", "--target", "wasm32-unknown-unknown"];
    if release {
        args.push("--release");
    }
    args
}

/// Build the argument list for `wasm-tools component new <core> -o <output>`.
pub fn wasm_tools_args<'a>(core_path: &'a Path, output_path: &'a Path) -> Vec<&'a str> {
    vec![
        "component",
        "new",
        core_path.to_str().expect("core path must be valid UTF-8"),
        "-o",
        output_path
            .to_str()
            .expect("output path must be valid UTF-8"),
    ]
}

/// Execute the `slicer build [--release]` workflow.
///
/// 1. Parse module name from `Cargo.toml` in the current directory.
/// 2. Warn if `crate-type = ["cdylib"]` is missing.
/// 3. Run `cargo build --target wasm32-unknown-unknown [--release]`.
/// 4. Run `wasm-tools component new` on the core WASM output.
/// 5. Copy the result to `target/slicer/<module-name>.wasm`.
///
/// # Errors
///
/// Returns a [`BuildError`] if any step fails.
pub fn execute(release: bool) -> Result<(), BuildError> {
    let cwd = std::env::current_dir()?;
    let module_name = parse_module_name(&cwd)?;

    // Warn (but don't fail) if cdylib is missing
    if !has_cdylib(&cwd)? {
        eprintln!(
            "warning: Cargo.toml does not declare crate-type = [\"cdylib\"]. \
             WASM modules typically require cdylib."
        );
    }

    // Step 1: cargo build
    let args = cargo_build_args(release);
    let output = Command::new("cargo")
        .args(&args)
        .output()
        .map_err(BuildError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::CargoBuildFailed(stderr.to_string()));
    }

    // Step 2: wasm-tools component new
    let core_path = core_wasm_output_path(&module_name, release);
    let output_path = final_output_path(&module_name);

    // Ensure target/slicer/ exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let wt_args = wasm_tools_args(&core_path, &output_path);
    let wt_output = Command::new("wasm-tools")
        .args(&wt_args)
        .output()
        .map_err(BuildError::Io)?;

    if !wt_output.status.success() {
        let stderr = String::from_utf8_lossy(&wt_output.stderr);
        return Err(BuildError::WasmToolsFailed(stderr.to_string()));
    }

    println!("Built module: {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_wasm_path_converts_hyphens() {
        let p = core_wasm_output_path("my-infill", false);
        assert!(p.ends_with("my_infill.wasm"));
    }

    #[test]
    fn core_wasm_path_no_hyphens() {
        let p = core_wasm_output_path("infill", true);
        assert!(p.ends_with("infill.wasm"));
    }

    #[test]
    fn final_path_preserves_hyphens() {
        let p = final_output_path("my-infill");
        assert_eq!(p, PathBuf::from("target/slicer/my-infill.wasm"));
    }

    #[test]
    fn cargo_args_debug_mode() {
        let args = cargo_build_args(false);
        assert!(!args.contains(&"--release"));
    }

    #[test]
    fn cargo_args_release_mode() {
        let args = cargo_build_args(true);
        assert!(args.contains(&"--release"));
    }
}
