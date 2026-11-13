//! `cargo xtask check-fmt` / `cargo xtask fmt`: per-package formatting.
//!
//! # Why this exists instead of `cargo fmt --all`
//!
//! `cargo fmt --all` builds **one** argument vector containing every file of
//! every workspace member. This workspace has ~1,200 Rust files, which is far
//! past the Windows `CreateProcess` command-line limit; the invocation dies with
//! `The filename or extension is too long. (os error 206)` before rustfmt is
//! ever spawned, so a Windows developer cannot reproduce the CI formatting gate
//! at all. Running rustfmt once per package keeps each command line small and
//! produces the same answer on every platform.
//!
//! The package list comes from `cargo metadata`'s `workspace_members`, so this
//! covers exactly the packages `cargo fmt --all` covers — no more (guest crates
//! that carry their own `[workspace]` sentinel are separate workspaces and are
//! not part of the root gate).

use std::path::Path;
use std::process::Command;

/// One package's formatting result.
struct PackageResult {
    name: String,
    success: bool,
    output: String,
}

/// Format (`apply == true`) or check every workspace member.
///
/// Returns 0 when every package is (or became) formatted, 1 when at least one
/// package still has differences, 2 when the package list could not be read.
pub(crate) fn run(workspace: &Path, apply: bool) -> i32 {
    let names = match workspace_member_names(workspace) {
        Ok(names) => names,
        Err(error) => {
            eprintln!("check-fmt: cannot list workspace packages: {error}");
            return 2;
        }
    };
    if names.is_empty() {
        eprintln!("check-fmt: cargo metadata reported no workspace members");
        return 2;
    }

    let results: Vec<PackageResult> = names
        .into_iter()
        .map(|name| {
            let mut command = Command::new("cargo");
            command
                .current_dir(workspace)
                .arg("fmt")
                .arg("-p")
                .arg(&name);
            if !apply {
                command.arg("--").arg("--check");
            }
            match command.output() {
                Ok(output) if output.status.success() => PackageResult {
                    name,
                    success: true,
                    output: String::new(),
                },
                Ok(output) => {
                    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
                    text.push_str(&String::from_utf8_lossy(&output.stderr));
                    PackageResult {
                        name,
                        success: false,
                        output: text,
                    }
                }
                Err(error) => PackageResult {
                    name,
                    success: false,
                    output: format!("could not run cargo fmt: {error}"),
                },
            }
        })
        .collect();

    let failed: Vec<&PackageResult> = results.iter().filter(|r| !r.success).collect();
    if failed.is_empty() {
        // Report the count so a zero-package run cannot read as success.
        println!(
            "check-fmt: {} package(s) {}",
            results.len(),
            if apply { "formatted" } else { "clean" }
        );
        return 0;
    }

    for result in &failed {
        eprintln!("check-fmt: {} needs formatting", result.name);
        if !result.output.is_empty() {
            eprint!("{}", result.output);
        }
    }
    eprintln!(
        "check-fmt: {} of {} package(s) {}",
        failed.len(),
        results.len(),
        if apply {
            "still need formatting after the run"
        } else {
            "are not formatted"
        }
    );
    1
}

/// Names of the root workspace's member packages, in metadata order.
fn workspace_member_names(workspace: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("cargo")
        .current_dir(workspace)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|error| format!("could not run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata did not return JSON: {error}"))?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata carried no packages array".to_string())?;
    let members = metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata carried no workspace_members array".to_string())?;
    let mut names = Vec::with_capacity(members.len());
    for member in members {
        let id = member
            .as_str()
            .ok_or_else(|| "workspace_members entry was not a string".to_string())?;
        let package = packages
            .iter()
            .find(|package| package.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .ok_or_else(|| format!("workspace member {id} had no package record"))?;
        let name = package
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("workspace member {id} had no name"))?;
        names.push(name.to_string());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_member_names_are_non_empty_and_unique_for_this_workspace() {
        let workspace = crate::build_guests::workspace_root();
        let names = workspace_member_names(&workspace).expect("cargo metadata must resolve");
        assert!(
            names.len() > 10,
            "the root workspace has many members, got {}",
            names.len()
        );
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "member names must be unique");
        assert!(
            names.iter().any(|name| name == "pnp-cli"),
            "the CLI must be a member: {names:?}"
        );
    }
}
