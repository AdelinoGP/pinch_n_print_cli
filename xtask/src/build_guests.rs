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
    CargoFailed {
        guest: String,
        stderr_tail: String,
    },
    ComponentInputFailed {
        guest: String,
        stderr_tail: String,
    },
    WasmToolsFailed {
        guest: String,
        stderr_tail: String,
    },
    MissingIntermediate {
        guest: String,
        expected: PathBuf,
    },
    FingerprintMetadataFailed {
        guest: String,
        path: PathBuf,
        error: String,
    },
    WasmToolsNotFound,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::CargoFailed { guest, stderr_tail } => {
                write!(f, "cargo build failed for '{guest}':\n{stderr_tail}")
            }
            BuildError::ComponentInputFailed { guest, stderr_tail } => {
                write!(f, "wasm-tools strip failed for '{guest}':\n{stderr_tail}")
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
            BuildError::FingerprintMetadataFailed { guest, path, error } => {
                write!(
                    f,
                    "could not write freshness metadata for '{guest}' at {}: {error}",
                    path.display()
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

pub fn tail_lines(s: &str, n: usize) -> String {
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

    // Step C: remove conflicting SDK helper metadata before componentization.
    let output_path = ws_root.join(&spec.artifact_path);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    // SDK helper bindings carry an older copy of shared WIT metadata. Keep the
    // module's canonical world metadata and remove only those conflicting helpers.
    let component_input = intermediate_base.join(format!("{}-component-input.wasm", spec.lib_name));
    let strip_out = Command::new("wasm-tools")
        .args(["strip", "--delete", "^component-type:.*:slicer:sdk-"])
        .arg(&intermediate)
        .args(["-o"])
        .arg(&component_input)
        .output()
        .map_err(|e| BuildError::ComponentInputFailed {
            guest: spec.crate_name.clone(),
            stderr_tail: format!("failed to spawn wasm-tools: {e}"),
        })?;

    if !strip_out.status.success() {
        let stderr = String::from_utf8_lossy(&strip_out.stderr);
        return Err(BuildError::ComponentInputFailed {
            guest: spec.crate_name.clone(),
            stderr_tail: tail_lines(&stderr, 20),
        });
    }

    let wt_out = Command::new("wasm-tools")
        .args(["component", "new"])
        .arg(&component_input)
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

    // Record the inputs only after both cargo and componentization succeeded.
    let shared = compute_shared_freshness(ws_root);
    let freshness = compute_guest_freshness(spec, ws_root, &shared);
    let metadata_path = fingerprint_metadata_path(ws_root, spec);
    if let Some(parent) = metadata_path.parent() {
        fs::create_dir_all(parent).map_err(|e| BuildError::FingerprintMetadataFailed {
            guest: spec.crate_name.clone(),
            path: metadata_path.clone(),
            error: e.to_string(),
        })?;
    }
    fs::write(&metadata_path, freshness.fingerprint.as_bytes()).map_err(|e| {
        BuildError::FingerprintMetadataFailed {
            guest: spec.crate_name.clone(),
            path: metadata_path,
            error: e.to_string(),
        }
    })?;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FingerprintEntry {
    path: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FreshnessSnapshot {
    pub newest_mtime: SystemTime,
    pub fingerprint: String,
    entries: Vec<FingerprintEntry>,
}

/// Return all files below `root`, sorted by path, optionally restricted by extension.
fn input_files(root: &Path, extension: Option<&str>) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            extension
                .is_none_or(|wanted| e.path().extension().and_then(|s| s.to_str()) == Some(wanted))
        })
        .map(|e| e.into_path())
        .collect();
    paths.sort();
    paths
}

/// Reuse the existing shared source set for both mtime and content freshness.
fn shared_input_paths(ws_root: &Path) -> Vec<PathBuf> {
    let wit_root = ws_root.join("crates/slicer-schema/wit");
    let mut paths = input_files(&wit_root, Some("wit"));

    let shared_crates = ["slicer-macros", "slicer-sdk", "slicer-ir", "slicer-schema"];
    for krate in shared_crates {
        let crate_root = ws_root.join("crates").join(krate);
        paths.extend(input_files(&crate_root.join("src"), None));
        for file in ["Cargo.toml", "build.rs"] {
            let path = crate_root.join(file);
            if path.is_file() {
                paths.push(path);
            }
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

fn guest_input_paths(spec: &GuestSpec) -> Vec<PathBuf> {
    let mut paths = input_files(&spec.guest_dir.join("src"), None);
    paths.push(spec.manifest_path.clone());

    // Core guests compile the parent module through the path dependency.
    if spec.tree == GuestTree::Core {
        let parent_dir = spec
            .guest_dir
            .parent()
            .expect("wit-guest/ must have a parent directory");
        paths.extend(input_files(&parent_dir.join("src"), None));
        paths.push(parent_dir.join("Cargo.toml"));
    }

    paths.sort();
    paths.dedup();
    paths
}

fn relative_input_path(ws_root: &Path, path: &Path) -> String {
    path.strip_prefix(ws_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn fingerprint_entries(entries: &[FingerprintEntry]) -> String {
    let mut ordered = entries.to_vec();
    ordered.sort_by(|a, b| a.path.cmp(&b.path).then(a.bytes.cmp(&b.bytes)));

    let mut hash = [0xcbf29ce484222325_u64, 0x84222325cbf29ce4_u64];
    for entry in ordered {
        hash_update(&mut hash, &(entry.path.len() as u64).to_le_bytes());
        hash_update(&mut hash, entry.path.as_bytes());
        hash_update(&mut hash, &[0]);
        hash_update(&mut hash, &(entry.bytes.len() as u64).to_le_bytes());
        hash_update(&mut hash, &entry.bytes);
    }
    format!("v1-{:016x}{:016x}", hash[0], hash[1])
}

fn hash_update(hash: &mut [u64; 2], bytes: &[u8]) {
    const FNV_PRIME: u64 = 1_099_511_628_211;
    for byte in bytes {
        hash[0] ^= u64::from(*byte);
        hash[0] = hash[0].wrapping_mul(FNV_PRIME);
        hash[1] ^= u64::from(!*byte);
        hash[1] = hash[1].wrapping_mul(FNV_PRIME);
    }
}

fn snapshot_from_paths(ws_root: &Path, paths: &[PathBuf]) -> FreshnessSnapshot {
    let mut entries = Vec::with_capacity(paths.len());
    let mut newest_mtime = UNIX_EPOCH;

    for path in paths {
        if let Some(mtime) = file_mtime(path) {
            newest_mtime = newest_mtime.max(mtime);
        }
        let bytes = fs::read(path).unwrap_or_else(|_| b"<unreadable-input>".to_vec());
        entries.push(FingerprintEntry {
            path: relative_input_path(ws_root, path),
            bytes,
        });
    }

    FreshnessSnapshot {
        fingerprint: fingerprint_entries(&entries),
        newest_mtime,
        entries,
    }
}

/// Compute shared mtime and content inputs once per freshness invocation.
pub fn compute_shared_freshness(ws_root: &Path) -> FreshnessSnapshot {
    snapshot_from_paths(ws_root, &shared_input_paths(ws_root))
}

fn compute_guest_freshness(
    spec: &GuestSpec,
    ws_root: &Path,
    shared: &FreshnessSnapshot,
) -> FreshnessSnapshot {
    let guest = snapshot_from_paths(ws_root, &guest_input_paths(spec));
    let mut entries = shared.entries.clone();
    entries.extend(guest.entries);
    FreshnessSnapshot {
        fingerprint: fingerprint_entries(&entries),
        newest_mtime: shared.newest_mtime.max(guest.newest_mtime),
        entries,
    }
}

pub fn fingerprint_metadata_path(ws_root: &Path, spec: &GuestSpec) -> PathBuf {
    ws_root
        .join("target/guest-fingerprints")
        .join(format!("{}.fingerprint", spec.crate_name))
}

fn metadata_matches(path: &Path, expected: &str) -> bool {
    fs::read_to_string(path)
        .map(|actual| actual.trim() == expected)
        .unwrap_or(false)
}

/// Return the mtime of a single file, or `None` if it doesn't exist.
pub fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Return `true` if the guest artifact is absent or older than its sources.
pub fn is_stale(spec: &GuestSpec, ws_root: &Path, shared: &FreshnessSnapshot) -> bool {
    let freshness = compute_guest_freshness(spec, ws_root, shared);
    let artifact_mtime = file_mtime(&ws_root.join(&spec.artifact_path));
    artifact_mtime.is_none_or(|artifact| freshness.newest_mtime > artifact)
        || !metadata_matches(
            &fingerprint_metadata_path(ws_root, spec),
            &freshness.fingerprint,
        )
}

/// Freshness check: print `STALE: <crate_name>` for every stale guest; exit 1 if any.
pub fn check_command(ws_root: &Path) -> i32 {
    let shared = compute_shared_freshness(ws_root);
    let (guests, _skips) = discover_guests(ws_root);

    let mut any_stale = false;
    for spec in &guests {
        if is_stale(spec, ws_root, &shared) {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "pnp-xtask-fingerprint-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temporary test directory");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn fingerprint_is_deterministic_and_content_sensitive() {
        let entries = vec![
            FingerprintEntry {
                path: "b.wit".to_string(),
                bytes: b"second".to_vec(),
            },
            FingerprintEntry {
                path: "a.wit".to_string(),
                bytes: b"first".to_vec(),
            },
        ];
        assert_eq!(fingerprint_entries(&entries), fingerprint_entries(&entries));

        let mut changed = entries.clone();
        changed[1].bytes = b"changed".to_vec();
        assert_ne!(fingerprint_entries(&entries), fingerprint_entries(&changed));
    }

    #[test]
    fn missing_fingerprint_metadata_is_stale() {
        let temp = TempDir::new();
        let guest_dir = temp.0.join("guest");
        fs::create_dir_all(guest_dir.join("src")).expect("create guest source directory");
        let manifest_path = guest_dir.join("Cargo.toml");
        fs::write(&manifest_path, "[package]\nname = \"guest\"\n").expect("write manifest");
        fs::write(guest_dir.join("src/lib.rs"), "fn main() {}\n").expect("write source");
        let artifact_path = temp.0.join("guest.wasm");
        fs::write(&artifact_path, b"artifact").expect("write artifact");

        let spec = GuestSpec {
            crate_name: "guest".to_string(),
            lib_name: "guest".to_string(),
            manifest_path,
            guest_dir,
            artifact_path: PathBuf::from("guest.wasm"),
            tree: GuestTree::TestGuest,
        };
        let shared = compute_shared_freshness(&temp.0);
        assert!(is_stale(&spec, &temp.0, &shared));

        let freshness = compute_guest_freshness(&spec, &temp.0, &shared);
        let metadata_path = fingerprint_metadata_path(&temp.0, &spec);
        fs::create_dir_all(metadata_path.parent().expect("metadata parent"))
            .expect("create metadata directory");
        fs::write(&metadata_path, freshness.fingerprint).expect("write metadata");
        assert!(!is_stale(&spec, &temp.0, &shared));
    }
}
