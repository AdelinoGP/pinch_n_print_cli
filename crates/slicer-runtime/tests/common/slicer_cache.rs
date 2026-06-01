//! Process-local cache for `pnp_cli` binary invocations.
//!
//! Many e2e tests slice the same fixture with the same modules and config
//! but each repeats the full binary run. This module exposes
//! [`cached_run`] which executes a given (model, module-dir, config) tuple
//! at most once per `cargo test` process; later callers block on the same
//! `OnceLock` and read the previously computed outcome.
//!
//! Tests that require two independent runs (determinism asserts) must
//! call [`run_pnp_cli_uncached`] instead.

#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

/// Discriminator for `--module-dir` scenarios. Each variant maps to a
/// stable path materialized once per process.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModuleDirKind {
    /// `modules/core-modules` — the full production tree.
    CoreModules,
    /// A scratch directory with no module manifests.
    Empty,
    /// `modules/core-modules` minus `traditional-support`, so
    /// `tree-support` becomes the active `support-generator` holder.
    TreeSupportFiltered,
    /// `modules/core-modules` minus `part-cooling`, so the slice runs
    /// without any fan / cooling module emitting M106.
    PartCoolingFiltered,
}

/// Captured outcome of a `pnp_cli` invocation. `success == false` is a
/// real cached value (not an error) — failure-asserting tests
/// (`cli_rejects_top_shell_layers_string`) read this directly.
///
/// `gcode` and `stderr` are loaded eagerly into the cached `Arc`, so they
/// live for the test process lifetime. With ~20 distinct cache keys at a
/// few MB each, total resident size is small; the output `.gcode` files
/// behind `OUTPUT_TMP` likewise persist until process exit.
#[derive(Clone, Debug)]
pub struct RunOutcome {
    pub success: bool,
    pub exit_code: Option<i32>,
    /// Contents of the `--output` file, or `""` if the binary did not
    /// write one (typical for failed runs).
    pub gcode: String,
    /// UTF-8-lossy stderr.
    pub stderr: String,
    /// Whether the `--output` file existed on disk after the binary
    /// returned. Distinguishes a missing output file from one that
    /// happened to be empty.
    pub output_written: bool,
}

/// Propagated only when the cached run itself panicked (a bug in the
/// driver, not a pnp_cli failure). Regular non-zero exits are
/// returned as `Ok(RunOutcome { success: false, .. })`.
#[derive(Clone, Debug)]
pub struct RunError {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RunKey {
    model: PathBuf,
    module_dir: ModuleDirKind,
    config_digest: Option<u64>,
}

type CacheCell = Arc<OnceLock<Arc<Result<RunOutcome, RunError>>>>;
type CacheMap = HashMap<RunKey, CacheCell>;

static CACHE: OnceLock<Mutex<CacheMap>> = OnceLock::new();
static EMPTY_MODULE_TMP: OnceLock<tempfile::TempDir> = OnceLock::new();
static FILTERED_TREE_SUPPORT_TMP: OnceLock<tempfile::TempDir> = OnceLock::new();
static FILTERED_PART_COOLING_TMP: OnceLock<tempfile::TempDir> = OnceLock::new();
static OUTPUT_TMP: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Canonicalized repo root (parent of `crates/`).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

/// Path to the compiled `pnp_cli` binary for integration tests.
///
/// Checks the standard `target/debug/` directory relative to the workspace
/// root. Tests must be run via `cargo test` (which builds binaries first) or
/// after `cargo build --workspace`.
pub fn pnp_cli_bin() -> PathBuf {
    // When running under `cargo test`, the binary should already be built.
    // We locate it by walking up to the workspace root's target/ dir.
    let exe_name = if cfg!(windows) {
        "pnp_cli.exe"
    } else {
        "pnp_cli"
    };
    // Try debug first (cargo test default), then release.
    let root = repo_root();
    let debug = root.join("target").join("debug").join(exe_name);
    if debug.exists() {
        return debug;
    }
    let release = root.join("target").join("release").join(exe_name);
    if release.exists() {
        return release;
    }
    panic!(
        "pnp_cli binary not found at {} or {}. Run `cargo build --workspace` first.",
        debug.display(),
        release.display()
    )
}

pub fn fixture_stl() -> PathBuf {
    repo_root().join("resources/benchy.stl")
}

pub fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

fn empty_module_dir_path() -> PathBuf {
    let td = EMPTY_MODULE_TMP
        .get_or_init(|| tempfile::tempdir().expect("create empty-module-dir tempdir"));
    let p = td.path().join("empty-module-dir");
    if !p.exists() {
        std::fs::create_dir_all(&p).expect("mkdir empty-module-dir");
    }
    p
}

fn tree_support_filtered_dir_path() -> PathBuf {
    let td = FILTERED_TREE_SUPPORT_TMP.get_or_init(|| {
        let tmp = tempfile::tempdir().expect("create filtered-module-dir tempdir");
        let src = core_modules_dir();
        let dst = tmp.path().join("tree-support-modules");
        std::fs::create_dir_all(&dst).expect("mkdir tree-support-modules");
        for entry in std::fs::read_dir(&src).expect("read core-modules dir") {
            let entry = entry.expect("read_dir entry");
            let name = entry.file_name();
            if name.to_string_lossy() == "traditional-support" {
                continue;
            }
            let target = dst.join(&name);
            if entry.file_type().expect("file_type").is_dir() {
                recurse_copy(&entry.path(), &target).expect("recurse_copy dir");
            } else {
                std::fs::copy(&entry.path(), &target).expect("copy file");
            }
        }
        tmp
    });
    td.path().join("tree-support-modules")
}

fn part_cooling_filtered_dir_path() -> PathBuf {
    let td = FILTERED_PART_COOLING_TMP.get_or_init(|| {
        let tmp = tempfile::tempdir().expect("create filtered-module-dir tempdir");
        let src = core_modules_dir();
        let dst = tmp.path().join("no-part-cooling-modules");
        std::fs::create_dir_all(&dst).expect("mkdir no-part-cooling-modules");
        for entry in std::fs::read_dir(&src).expect("read core-modules dir") {
            let entry = entry.expect("read_dir entry");
            let name = entry.file_name();
            if name.to_string_lossy() == "part-cooling" {
                continue;
            }
            let target = dst.join(&name);
            if entry.file_type().expect("file_type").is_dir() {
                recurse_copy(&entry.path(), &target).expect("recurse_copy dir");
            } else {
                std::fs::copy(&entry.path(), &target).expect("copy file");
            }
        }
        tmp
    });
    td.path().join("no-part-cooling-modules")
}

fn recurse_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            recurse_copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Resolve a [`ModuleDirKind`] to a concrete on-disk path. Exposed so
/// the determinism tests (which call [`run_pnp_cli_uncached`]
/// directly) can use the same materialized directories the cache uses,
/// without having to re-implement the empty / filtered-tree-support
/// staging logic.
pub fn module_dir_path(kind: &ModuleDirKind) -> PathBuf {
    match kind {
        ModuleDirKind::CoreModules => core_modules_dir(),
        ModuleDirKind::Empty => empty_module_dir_path(),
        ModuleDirKind::TreeSupportFiltered => tree_support_filtered_dir_path(),
        ModuleDirKind::PartCoolingFiltered => part_cooling_filtered_dir_path(),
    }
}

fn hash_config_file(path: &Path) -> u64 {
    let content = std::fs::read(path)
        .unwrap_or_else(|e| panic!("read config file {} for cache key: {}", path.display(), e));
    let mut h = DefaultHasher::new();
    content.hash(&mut h);
    h.finish()
}

/// Blocking lookup. First caller through executes the binary; later
/// callers receive the cached `Arc<Result<RunOutcome, RunError>>`.
///
/// A panic during the cached run is converted to `Err(RunError { .. })`
/// and surfaced to every reader — no silent `Mutex` poisoning.
pub fn cached_run(
    model: &Path,
    module_dir: ModuleDirKind,
    config: Option<&Path>,
) -> Arc<Result<RunOutcome, RunError>> {
    let key = RunKey {
        model: model.canonicalize().unwrap_or_else(|_| model.to_path_buf()),
        module_dir: module_dir.clone(),
        config_digest: config.map(hash_config_file),
    };

    let cell: CacheCell = {
        let mut map = CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("slicer cache mutex");
        map.entry(key)
            .or_insert_with(|| Arc::new(OnceLock::new()))
            .clone()
    };

    cell.get_or_init(|| {
        let model_owned = model.to_path_buf();
        let module_dir_owned = module_dir.clone();
        let config_owned: Option<PathBuf> = config.map(Path::to_path_buf);
        let computed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_slicer(&model_owned, &module_dir_owned, config_owned.as_deref())
        }));
        let result = match computed {
            Ok(outcome) => Ok(outcome),
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = payload.downcast_ref::<&'static str>() {
                    s.to_string()
                } else {
                    "panic in cached slicer run (opaque payload)".to_string()
                };
                Err(RunError { message: msg })
            }
        };
        Arc::new(result)
    })
    .clone()
}

fn execute_slicer(model: &Path, module_dir: &ModuleDirKind, config: Option<&Path>) -> RunOutcome {
    let modules_path = module_dir_path(module_dir);
    let out_tmp = OUTPUT_TMP.get_or_init(|| tempfile::tempdir().expect("output tempdir"));
    // Unique filename per call to avoid clobbering between cache slots.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out_path = out_tmp.path().join(format!("cached_run_{seq}.gcode"));

    let proc_out = run_pnp_cli_uncached(model, &modules_path, &out_path, config);
    let output_written = out_path.exists();
    let gcode = if output_written {
        std::fs::read_to_string(&out_path).unwrap_or_default()
    } else {
        String::new()
    };
    RunOutcome {
        success: proc_out.status.success(),
        exit_code: proc_out.status.code(),
        gcode,
        stderr: String::from_utf8_lossy(&proc_out.stderr).into_owned(),
        output_written,
    }
}

/// Escape hatch for determinism tests and other intentionally
/// non-cached callers. Identical wire shape to the per-file
/// `run_slicer_host` helpers that previously lived in each test file.
pub fn run_pnp_cli_uncached(
    model: &Path,
    module_dir: &Path,
    output: &Path,
    config: Option<&Path>,
) -> std::process::Output {
    let bin = pnp_cli_bin();
    let mut cmd = Command::new(bin);
    cmd.args([
        "slice",
        "--model",
        model.to_str().unwrap(),
        "--module-dir",
        module_dir.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    if let Some(config_path) = config {
        cmd.arg("--config").arg(config_path);
    }
    cmd.output().expect("pnp_cli binary should execute")
}

/// Helper for the common case: cached_run + unwrap-or-panic on
/// `RunError`. Returns a borrowed `&RunOutcome` keyed off the supplied
/// `Arc` so the caller can keep field references for the duration of
/// the test.
pub fn expect_outcome(arc: &Arc<Result<RunOutcome, RunError>>) -> &RunOutcome {
    arc.as_ref()
        .as_ref()
        .unwrap_or_else(|e| panic!("cached slicer run panicked: {}", e.message))
}
