//! Implementation of the `slicer test` subcommand.
//!
//! Runs the module's test suite via `cargo nextest run` (falling back to `cargo test`
//! if nextest is not installed) and optionally writes a coverage report to
//! `target/slicer/coverage/` using `cargo llvm-cov`.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Errors that can occur during the test process.
#[derive(Debug)]
pub enum TestError {
    /// No Cargo.toml found in the current directory.
    MissingCargoToml,
    /// Cargo.toml could not be parsed or is missing required fields.
    CargoTomlParseError(String),
    /// The test runner returned a non-zero exit code.
    TestRunnerFailed(String),
    /// Coverage generation failed (non-fatal in `execute`, but representable).
    CoverageFailed(String),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCargoToml => write!(f, "Cargo.toml not found in the current directory"),
            Self::CargoTomlParseError(msg) => write!(f, "failed to parse Cargo.toml: {msg}"),
            Self::TestRunnerFailed(msg) => write!(f, "test runner failed: {msg}"),
            Self::CoverageFailed(msg) => write!(f, "coverage generation failed: {msg}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl From<std::io::Error> for TestError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Parse the module (package) name from `Cargo.toml` in the given directory.
///
/// # Errors
///
/// Returns [`TestError::MissingCargoToml`] if no `Cargo.toml` exists,
/// or [`TestError::CargoTomlParseError`] if parsing fails or `[package].name` is missing.
///
/// # Examples
///
/// ```no_run
/// # use slicer_cli::cmd_test;
/// let name = cmd_test::parse_module_name(std::path::Path::new(".")).unwrap();
/// println!("Module: {name}");
/// ```
pub fn parse_module_name(dir: &Path) -> Result<String, TestError> {
    let cargo_path = dir.join("Cargo.toml");
    if !cargo_path.exists() {
        return Err(TestError::MissingCargoToml);
    }

    let content = fs::read_to_string(&cargo_path)?;
    let table: toml::Table = content
        .parse()
        .map_err(|e: toml::de::Error| TestError::CargoTomlParseError(e.to_string()))?;

    table
        .get("package")
        .and_then(|pkg| pkg.get("name"))
        .and_then(|name| name.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| TestError::CargoTomlParseError("missing [package].name".to_string()))
}

/// Check whether `cargo nextest` is available on the system.
pub fn is_nextest_available() -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check whether `cargo llvm-cov` is available on the system.
pub fn is_llvm_cov_available() -> bool {
    Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Build the argument list for `cargo nextest run [<extra-args>...]`.
pub fn nextest_args(extra: &[String]) -> Vec<String> {
    let mut args = vec!["nextest".to_string(), "run".to_string()];
    for a in extra {
        args.push(a.clone());
    }
    args
}

/// Build the argument list for `cargo test [<extra-args>...]`.
pub fn cargo_test_args(extra: &[String]) -> Vec<String> {
    let mut args = vec!["test".to_string()];
    for a in extra {
        args.push(a.clone());
    }
    args
}

/// The directory where coverage reports are written.
pub fn coverage_output_dir() -> PathBuf {
    PathBuf::from("target/slicer/coverage")
}

/// Build the argument list for `cargo llvm-cov` with output to the given directory.
pub fn llvm_cov_args(output_dir: &Path) -> Vec<String> {
    vec![
        "llvm-cov".to_string(),
        "--html".to_string(),
        "--output-dir".to_string(),
        output_dir.to_string_lossy().to_string(),
    ]
}

/// Execute the `slicer test [-- <args>]` workflow.
///
/// 1. Parse module name from `Cargo.toml` in the current directory.
/// 2. Run `cargo nextest run` (or fall back to `cargo test`) with forwarded args.
/// 3. Attempt coverage via `cargo llvm-cov` (non-fatal if unavailable).
///
/// # Errors
///
/// Returns a [`TestError`] if the test runner fails. Coverage failure is non-fatal
/// and only produces a warning on stderr.
pub fn execute(args: &[String]) -> Result<(), TestError> {
    let cwd = std::env::current_dir()?;
    let module_name = parse_module_name(&cwd)?;

    println!("Testing module: {module_name}");

    // Step 1: Run test suite
    let test_args = if is_nextest_available() {
        println!("Using cargo nextest");
        nextest_args(args)
    } else {
        println!("cargo nextest not found, falling back to cargo test");
        cargo_test_args(args)
    };

    let output = Command::new("cargo")
        .args(&test_args)
        .status()
        .map_err(TestError::Io)?;

    if !output.success() {
        return Err(TestError::TestRunnerFailed(format!(
            "exit code: {}",
            output.code().unwrap_or(-1)
        )));
    }

    // Step 2: Attempt coverage (non-fatal)
    if is_llvm_cov_available() {
        let cov_dir = coverage_output_dir();
        if let Err(e) = fs::create_dir_all(&cov_dir) {
            eprintln!("warning: could not create coverage directory: {e}");
        } else {
            let cov_args = llvm_cov_args(&cov_dir);
            match Command::new("cargo").args(&cov_args).status() {
                Ok(status) if status.success() => {
                    println!("Coverage report written to {}", cov_dir.display());
                }
                Ok(status) => {
                    eprintln!(
                        "warning: coverage generation failed (exit code: {})",
                        status.code().unwrap_or(-1)
                    );
                }
                Err(e) => {
                    eprintln!("warning: could not run cargo llvm-cov: {e}");
                }
            }
        }
    } else {
        eprintln!("warning: cargo llvm-cov not found, skipping coverage report");
    }

    println!("All tests passed for {module_name}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_dir_is_under_target_slicer() {
        let dir = coverage_output_dir();
        assert!(dir.starts_with("target/slicer"));
    }

    #[test]
    fn nextest_args_empty() {
        let args = nextest_args(&[]);
        assert_eq!(args, vec!["nextest", "run"]);
    }

    #[test]
    fn cargo_test_args_empty() {
        let args = cargo_test_args(&[]);
        assert_eq!(args, vec!["test"]);
    }

    #[test]
    fn llvm_cov_args_includes_output_dir() {
        let dir = PathBuf::from("target/slicer/coverage");
        let args = llvm_cov_args(&dir);
        assert!(args.contains(&"llvm-cov".to_string()));
        assert!(args.contains(&"target/slicer/coverage".to_string()));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err = TestError::from(io_err);
        assert!(matches!(err, TestError::Io(_)));
    }
}
