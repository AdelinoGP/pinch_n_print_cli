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
//!
//! ## On-disk staging
//!
//! Earlier revisions of this cache materialized "filtered" copies of
//! `modules/core-modules` (e.g. the tree minus `traditional-support`)
//! into `tempfile::TempDir`s held in `static OnceLock<TempDir>`. Rust
//! never runs `Drop` on values held by `static` bindings at process
//! exit, so every test process leaked a full recursive copy of the
//! core-modules tree — every `.wasm` artifact included — under
//! `%TEMP%`. The fix in this file: expose the kept module subdirs as a
//! list of `--module-dir` arguments instead of staging a filtered
//! tree, and route the `.gcode` outputs through a stable
//! `target/test-staging/` path that survives across runs (gitignored
//! via the existing `/target` rule).

#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

/// Discriminator for `--module-dir` scenarios. Each variant maps to a
/// stable list of paths the cache feeds the binary as repeated
/// `--module-dir` flags.
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
/// under `target/test-staging/slicer-cache-output/` likewise persist
/// until process exit (and across runs — `target/` is the canonical
/// per-repo throwaway location).
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
static MODULE_DIR_PATHS: OnceLock<Mutex<HashMap<ModuleDirKind, Vec<PathBuf>>>> = OnceLock::new();

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
    let exe_name = if cfg!(windows) {
        "pnp_cli.exe"
    } else {
        "pnp_cli"
    };

    // Prefer the binary whose profile matches our own test binary. Cargo lays
    // out integration-test executables at `target/{debug,release}/deps/<bucket>-<hash>{.exe}`,
    // so the test's profile dir is `current_exe().parent().parent()` and the
    // sibling `pnp_cli{.exe}` is the right-profile binary.
    if let Ok(test_exe) = std::env::current_exe() {
        if let Some(profile_dir) = test_exe.parent().and_then(|p| p.parent()) {
            let candidate = profile_dir.join(exe_name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // Fallback when profile inference failed (unusual): prefer release over
    // debug — if both are present the user almost certainly wants the fast one.
    let root = repo_root();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join(exe_name);
        if p.exists() {
            return p;
        }
    }
    panic!(
        "pnp_cli binary not found under {}/target/{{debug,release}}/{exe_name}. \
         Run `cargo build --workspace` (or `--release`) first.",
        root.display(),
    );
}

pub fn fixture_stl() -> PathBuf {
    repo_root().join("resources/regression_wedge.stl")
}

pub fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

/// Root of a stable, gitignored area under `target/` where the cache
/// stages anything that needs an on-disk path (cached `.gcode` outputs
/// and the empty scratch dir for `ModuleDirKind::Empty`). Replaces the
/// previous `tempfile::TempDir`-in-`static` pattern that leaked under
/// `%TEMP%` because Rust does not drop statics at process exit.
fn test_staging_root() -> PathBuf {
    let p = repo_root().join("target/test-staging");
    std::fs::create_dir_all(&p).expect("create target/test-staging");
    p
}

/// Empty scratch directory for `ModuleDirKind::Empty`. Idempotent: the
/// path is stable across runs, so a once-created empty dir is reused.
fn empty_staging_dir() -> PathBuf {
    let p = test_staging_root().join("slicer-cache-empty");
    std::fs::create_dir_all(&p).expect("create slicer-cache-empty");
    p
}

/// Where cached `.gcode` outputs live. The per-call `SEQ` counter
/// inside [`execute_slicer`] makes filenames unique across cache slots
/// so older entries from a prior process don't collide.
fn output_staging_dir() -> PathBuf {
    let p = test_staging_root().join("slicer-cache-output");
    std::fs::create_dir_all(&p).expect("create slicer-cache-output");
    p
}

/// Enumerate the per-module subdirs of `modules/core-modules`, omitting
/// any whose directory name matches `excluded`. The CLI's `--module-dir`
/// flag is repeatable and each module manifest sits one subdirectory
/// level deep, so an exclusion is naturally expressed as a list of the
/// kept subdirs — no recursive copy needed.
fn core_module_subdirs(excluded: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(core_modules_dir()).expect("read core-modules dir") {
        let entry = entry.expect("read_dir entry");
        if !entry.file_type().expect("file_type").is_dir() {
            continue;
        }
        if entry.file_name() == excluded {
            continue;
        }
        out.push(entry.path());
    }
    out.sort();
    out
}

/// Resolve a [`ModuleDirKind`] to the list of `--module-dir` paths the
/// cache will hand `pnp_cli`. Exposed so the determinism tests (which
/// call [`run_pnp_cli_uncached`] directly) can use the same
/// materialized list the cache uses.
///
/// Cached per-kind in a process-local map so the read of
/// `modules/core-modules/` happens once per `ModuleDirKind` per process.
pub fn module_dir_paths(kind: &ModuleDirKind) -> Vec<PathBuf> {
    let mut map = MODULE_DIR_PATHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("module dir paths mutex");
    map.entry(kind.clone())
        .or_insert_with(|| match kind {
            ModuleDirKind::CoreModules => vec![core_modules_dir()],
            ModuleDirKind::Empty => vec![empty_staging_dir()],
            ModuleDirKind::TreeSupportFiltered => core_module_subdirs("traditional-support"),
            ModuleDirKind::PartCoolingFiltered => core_module_subdirs("part-cooling"),
        })
        .clone()
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
    let modules = module_dir_paths(module_dir);
    let out_dir = output_staging_dir();
    // Unique filename per call to avoid clobbering between cache slots.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out_path = out_dir.join(format!("cached_run_{seq}.gcode"));

    let proc_out = run_pnp_cli_uncached(model, &modules, &out_path, config);
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
/// `run_slicer_host` helpers that previously lived in each test file,
/// except `module_dirs` is now a slice so each entry becomes its own
/// `--module-dir` argument (the CLI accepts the flag repeated).
pub fn run_pnp_cli_uncached(
    model: &Path,
    module_dirs: &[PathBuf],
    output: &Path,
    config: Option<&Path>,
) -> std::process::Output {
    let bin = pnp_cli_bin();
    let mut cmd = Command::new(bin);
    cmd.args(["slice", "--model", model.to_str().unwrap()]);
    for dir in module_dirs {
        cmd.arg("--module-dir").arg(dir);
    }
    cmd.args(["--output", output.to_str().unwrap()]);
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
