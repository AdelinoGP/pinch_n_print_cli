use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuestTree {
    Core,
    TestGuest,
}

#[derive(Debug, Clone)]
pub struct GuestSpec {
    pub crate_name: String,
    pub lib_name: String,
    pub manifest_path: PathBuf,
    pub guest_dir: PathBuf,
    pub artifact_path: PathBuf,
    pub tree: GuestTree,
}

/// Locate the workspace root by popping one level from the xtask crate dir.
pub fn workspace_root() -> PathBuf {
    let xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ws = xtask_dir
        .parent()
        .expect("xtask/ must have a parent directory (workspace root)")
        .to_path_buf();
    ws.canonicalize().unwrap_or(ws)
}

/// Check if the parsed TOML table has [lib] crate-type containing "cdylib".
fn has_cdylib(tab: &toml::Table) -> bool {
    tab.get("lib")
        .and_then(|v| v.get("crate-type"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|e| e.as_str() == Some("cdylib")))
        .unwrap_or(false)
}

/// Check if the TOML table has a [workspace] key present.
fn has_workspace_sentinel(tab: &toml::Table) -> bool {
    tab.get("workspace").is_some()
}

/// Check if [dependencies] contains any entry with path = "..".
fn has_parent_path_dep(tab: &toml::Table) -> bool {
    tab.get("dependencies")
        .and_then(|v| v.as_table())
        .map(|deps| {
            deps.values().any(|v| {
                v.as_table()
                    .and_then(|t| t.get("path"))
                    .and_then(|p| p.as_str())
                    == Some("..")
            })
        })
        .unwrap_or(false)
}

/// Check if [dependencies] declares wit-bindgen (any form).
fn has_wit_bindgen(tab: &toml::Table) -> bool {
    tab.get("dependencies")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("wit-bindgen"))
        .is_some()
}

/// Get the package name from the manifest table.
fn package_name(tab: &toml::Table) -> Option<String> {
    tab.get("package")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Get the lib name: explicit [lib].name if present, else package name with hyphens→underscores.
fn lib_name(tab: &toml::Table, pkg_name: &str) -> String {
    tab.get("lib")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| pkg_name.replace('-', "_"))
}

/// Walk the two guest trees and return (validated_guests, skip_reasons).
pub fn discover_guests(ws_root: &Path) -> (Vec<GuestSpec>, Vec<String>) {
    let mut guests: Vec<GuestSpec> = Vec::new();
    let mut skips: Vec<String> = Vec::new();

    // --- Core-modules tree ---
    let core_root = ws_root.join("modules").join("core-modules");
    if let Ok(entries) = fs::read_dir(&core_root) {
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        dirs.sort();

        for dir in dirs {
            let manifest = dir.join("wit-guest").join("Cargo.toml");
            if !manifest.exists() {
                continue;
            }

            let rel = manifest
                .strip_prefix(ws_root)
                .unwrap_or(&manifest)
                .to_string_lossy()
                .replace('\\', "/");

            let content = match fs::read_to_string(&manifest) {
                Ok(c) => c,
                Err(e) => {
                    skips.push(format!("SKIP: {rel} (read error: {e})"));
                    continue;
                }
            };

            let tab: toml::Table = match toml::from_str(&content) {
                Ok(t) => t,
                Err(e) => {
                    skips.push(format!("SKIP: {rel} (toml parse error: {e})"));
                    continue;
                }
            };

            // Validation
            if !has_cdylib(&tab) {
                skips.push(format!(
                    "SKIP: {rel} ([lib].crate-type does not contain cdylib)"
                ));
                continue;
            }
            if !has_workspace_sentinel(&tab) {
                skips.push(format!("SKIP: {rel} (missing [workspace] sentinel)"));
                continue;
            }
            if !has_parent_path_dep(&tab) {
                skips.push(format!("SKIP: {rel} (no parent path dep path = \"..\")"));
                continue;
            }

            let crate_name = match package_name(&tab) {
                Some(n) => n,
                None => {
                    skips.push(format!("SKIP: {rel} (missing [package].name)"));
                    continue;
                }
            };

            let dir_name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let lib_name = lib_name(&tab, &crate_name);
            let artifact_path =
                PathBuf::from(format!("modules/core-modules/{dir_name}/{dir_name}.wasm"));

            guests.push(GuestSpec {
                crate_name,
                lib_name,
                manifest_path: manifest,
                guest_dir: dir.join("wit-guest"),
                artifact_path,
                tree: GuestTree::Core,
            });
        }
    }

    // --- Test-guests tree ---
    let tg_root = ws_root.join("crates/slicer-wasm-host/test-guests");
    if let Ok(entries) = fs::read_dir(&tg_root) {
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        dirs.sort();

        for dir in dirs {
            let manifest = dir.join("Cargo.toml");
            if !manifest.exists() {
                continue;
            }

            let rel = manifest
                .strip_prefix(ws_root)
                .unwrap_or(&manifest)
                .to_string_lossy()
                .replace('\\', "/");

            let content = match fs::read_to_string(&manifest) {
                Ok(c) => c,
                Err(e) => {
                    skips.push(format!("SKIP: {rel} (read error: {e})"));
                    continue;
                }
            };

            let tab: toml::Table = match toml::from_str(&content) {
                Ok(t) => t,
                Err(e) => {
                    skips.push(format!("SKIP: {rel} (toml parse error: {e})"));
                    continue;
                }
            };

            // Validation
            if !has_cdylib(&tab) {
                skips.push(format!(
                    "SKIP: {rel} ([lib].crate-type does not contain cdylib)"
                ));
                continue;
            }
            if !has_workspace_sentinel(&tab) {
                skips.push(format!("SKIP: {rel} (missing [workspace] sentinel)"));
                continue;
            }
            if !has_wit_bindgen(&tab) {
                skips.push(format!("SKIP: {rel} (no wit-bindgen dependency)"));
                continue;
            }

            let crate_name = match package_name(&tab) {
                Some(n) => n,
                None => {
                    skips.push(format!("SKIP: {rel} (missing [package].name)"));
                    continue;
                }
            };

            let dir_name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let lib_name = lib_name(&tab, &crate_name);
            let artifact_path = PathBuf::from(format!(
                "crates/slicer-wasm-host/test-guests/{dir_name}.component.wasm"
            ));

            guests.push(GuestSpec {
                crate_name,
                lib_name,
                manifest_path: manifest,
                guest_dir: dir,
                artifact_path,
                tree: GuestTree::TestGuest,
            });
        }
    }

    // Sort: Core first, then TestGuest; alphabetical within each tree.
    guests.sort_by(|a, b| a.tree.cmp(&b.tree).then(a.crate_name.cmp(&b.crate_name)));

    (guests, skips)
}

/// Print discovered guests to stdout (tab-separated), skip reasons to stderr.
pub fn list_command(ws_root: &Path) -> std::io::Result<i32> {
    let (guests, skips) = discover_guests(ws_root);

    for reason in &skips {
        eprintln!("{reason}");
    }

    for g in &guests {
        let manifest_rel = g
            .manifest_path
            .strip_prefix(ws_root)
            .unwrap_or(&g.manifest_path)
            .to_string_lossy()
            .replace('\\', "/");
        let artifact_rel = g.artifact_path.to_string_lossy().replace('\\', "/");
        println!("{}\t{}\t{}", g.crate_name, manifest_rel, artifact_rel);
    }

    Ok(0)
}

// ---------------------------------------------------------------------------
// Build error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum BuildError {
    CargoFailed { guest: String, stderr_tail: String },
    WasmToolsFailed { guest: String, stderr_tail: String },
    MissingIntermediate { guest: String, expected: PathBuf },
    WasmToolsNotFound,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::CargoFailed { guest, stderr_tail } => {
                write!(f, "cargo build failed for '{guest}':\n{stderr_tail}")
            }
            BuildError::WasmToolsFailed { guest, stderr_tail } => {
                write!(
                    f,
                    "wasm-tools component new failed for '{guest}':\n{stderr_tail}"
                )
            }
            BuildError::MissingIntermediate { guest, expected } => {
                write!(
                    f,
                    "intermediate wasm not found for '{guest}': {}",
                    expected.display()
                )
            }
            BuildError::WasmToolsNotFound => {
                write!(
                    f,
                    "wasm-tools not found on PATH; install with 'cargo install wasm-tools'"
                )
            }
        }
    }
}

fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

// ---------------------------------------------------------------------------
// Preflight check
// ---------------------------------------------------------------------------

pub fn ensure_wasm_tools_available() -> Result<(), BuildError> {
    match Command::new("wasm-tools").arg("--version").output() {
        Ok(out) if out.status.success() => Ok(()),
        _ => Err(BuildError::WasmToolsNotFound),
    }
}

// ---------------------------------------------------------------------------
// Build one guest
// ---------------------------------------------------------------------------

pub fn build_one(spec: &GuestSpec, ws_root: &Path) -> Result<(), BuildError> {
    println!("building: {}", spec.crate_name);

    // Step A: cargo build
    // For test-guests, use a single shared CARGO_TARGET_DIR to avoid per-guest target dirs.
    let shared_target_dir = ws_root.join("crates/slicer-wasm-host/test-guests/target");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&spec.guest_dir).args([
        "build",
        "--target",
        "wasm32-unknown-unknown",
        "--release",
        "--quiet",
    ]);
    if spec.tree == GuestTree::TestGuest {
        cmd.env("CARGO_TARGET_DIR", &shared_target_dir);
    }
    let out = cmd.output().map_err(|e| BuildError::CargoFailed {
        guest: spec.crate_name.clone(),
        stderr_tail: format!("failed to spawn cargo: {e}"),
    })?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(BuildError::CargoFailed {
            guest: spec.crate_name.clone(),
            stderr_tail: tail_lines(&stderr, 20),
        });
    }

    // Step B: locate intermediate wasm
    let intermediate_base = if spec.tree == GuestTree::TestGuest {
        shared_target_dir.join("wasm32-unknown-unknown/release")
    } else {
        spec.guest_dir.join("target/wasm32-unknown-unknown/release")
    };
    let intermediate = intermediate_base.join(format!("{}.wasm", spec.lib_name));

    if !intermediate.exists() {
        return Err(BuildError::MissingIntermediate {
            guest: spec.crate_name.clone(),
            expected: intermediate,
        });
    }

    // Step C: wasm-tools component new
    let output_path = ws_root.join(&spec.artifact_path);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let wt_out = Command::new("wasm-tools")
        .args(["component", "new"])
        .arg(&intermediate)
        .arg("-o")
        .arg(&output_path)
        .output()
        .map_err(|e| BuildError::WasmToolsFailed {
            guest: spec.crate_name.clone(),
            stderr_tail: format!("failed to spawn wasm-tools: {e}"),
        })?;

    if !wt_out.status.success() {
        let stderr = String::from_utf8_lossy(&wt_out.stderr);
        return Err(BuildError::WasmToolsFailed {
            guest: spec.crate_name.clone(),
            stderr_tail: tail_lines(&stderr, 20),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Top-level build command
// ---------------------------------------------------------------------------

pub fn build_command(ws_root: &Path) -> i32 {
    if let Err(e) = ensure_wasm_tools_available() {
        eprintln!("error: {e}");
        return 1;
    }

    let (guests, skips) = discover_guests(ws_root);

    for reason in &skips {
        eprintln!("{reason}");
    }

    let mut count = 0usize;
    for spec in &guests {
        match build_one(spec, ws_root) {
            Ok(()) => count += 1,
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        }
    }

    println!("built {count} guest(s)");
    0
}

// ---------------------------------------------------------------------------
// Freshness-check helpers
// ---------------------------------------------------------------------------

/// Return the maximum mtime across all files reachable from `root`.
/// Returns `None` if `root` doesn't exist or contains no files.
pub fn newest_mtime_recursive(root: &Path) -> Option<SystemTime> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .max()
}

/// Return the mtime of a single file, or `None` if it doesn't exist.
pub fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Return the larger of two `Option<SystemTime>` values.
/// `None` is treated as "no constraint" (smaller than anything).
pub fn max_opt<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// Compute the shared mtime once per `--check` invocation.
/// Covers: `crates/slicer-schema/wit/**/*.wit` + 4 shared crates (slicer-macros, slicer-sdk, slicer-ir, slicer-schema).
pub fn compute_shared_mtime(ws_root: &Path) -> SystemTime {
    // --- wit/**/*.wit ---
    let wit_mtime: Option<SystemTime> =
        walkdir::WalkDir::new(ws_root.join("crates").join("slicer-schema").join("wit"))
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("wit"))
            .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
            .max();

    // --- 4 shared crates ---
    let shared_crates = ["slicer-macros", "slicer-sdk", "slicer-ir", "slicer-schema"];
    let mut shared_mtime: Option<SystemTime> = wit_mtime;

    for krate in &shared_crates {
        let crate_root = ws_root.join("crates").join(krate);
        // src/** (all files, no extension filter — bash uses `find … -type f`)
        let src_mtime = newest_mtime_recursive(&crate_root.join("src"));
        // Cargo.toml
        let toml_mtime = file_mtime(&crate_root.join("Cargo.toml"));
        shared_mtime = max_opt(shared_mtime, max_opt(src_mtime, toml_mtime));
    }

    shared_mtime.unwrap_or(UNIX_EPOCH)
}

/// Return `true` if the guest artifact is absent or older than its sources.
pub fn is_stale(spec: &GuestSpec, ws_root: &Path, shared_mtime: SystemTime) -> bool {
    // Per-guest source inputs
    let guest_src = newest_mtime_recursive(&spec.guest_dir.join("src"));
    let guest_toml = file_mtime(&spec.manifest_path);
    let mut per_guest = max_opt(guest_src, guest_toml);

    // Core tree: also track the parent module crate (one level above wit-guest/)
    if spec.tree == GuestTree::Core {
        let parent_dir = spec
            .guest_dir
            .parent()
            .expect("wit-guest/ must have a parent directory");
        let parent_src = newest_mtime_recursive(&parent_dir.join("src"));
        let parent_toml = file_mtime(&parent_dir.join("Cargo.toml"));
        per_guest = max_opt(per_guest, max_opt(parent_src, parent_toml));
    }

    let newest_src = match max_opt(per_guest, Some(shared_mtime)) {
        Some(t) => t,
        None => UNIX_EPOCH,
    };

    let artifact_mtime = file_mtime(&ws_root.join(&spec.artifact_path));

    match artifact_mtime {
        None => true,                  // artifact missing → stale
        Some(art) => newest_src > art, // source newer than artifact → stale
    }
}

/// Freshness check: print `STALE: <crate_name>` for every stale guest; exit 1 if any.
pub fn check_command(ws_root: &Path) -> i32 {
    let shared = compute_shared_mtime(ws_root);
    let (guests, _skips) = discover_guests(ws_root);

    let mut any_stale = false;
    for spec in &guests {
        if is_stale(spec, ws_root, shared) {
            println!("STALE: {}", spec.crate_name);
            any_stale = true;
        }
    }

    if any_stale {
        1
    } else {
        0
    }
}
