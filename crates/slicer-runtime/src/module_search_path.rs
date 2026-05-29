//! Multi-root module search-path assembly.
//!
//! The host walks search roots in priority order, deduplicates by canonical
//! absolute path, then hands the resulting `Vec<PathBuf>` to
//! [`crate::manifest::load_modules_from_roots`] (which dedups modules by
//! `module.id`, first root wins). See `docs/01_system_architecture.md`
//! §"Module Search Path" for the authoritative specification.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Environment variable consulted for additional search roots.
pub const SLICER_MODULE_PATH_ENV: &str = "SLICER_MODULE_PATH";

/// Debug-print env var: when set (to any value), the assembled roots are
/// emitted on stderr in priority order. The host has no logger init, so
/// this is the practical diagnostic channel.
pub const DEBUG_ENV: &str = "SLICER_DEBUG_PATHS";

/// Assemble the ordered, deduplicated list of module search roots.
///
/// Priority (highest first):
/// 1. `cli_dirs` — entries from `--module-dir`, in CLI order.
/// 2. `SLICER_MODULE_PATH` env var — OS path-list (split via
///    [`std::env::split_paths`]).
/// 3. `{config_dir}/modules/` — from
///    `directories::ProjectDirs::from("", "", "modular-slicer")`. Silently
///    skipped if absent on disk.
/// 4. `{executable_dir}/modules/` — from
///    [`std::env::current_exe`]. Silently skipped if absent on disk.
///
/// Tiers 3 and 4 are omitted when `no_default_paths` is `true`. Tiers 1 and
/// 2 always apply.
///
/// Deduplication is by canonical absolute path
/// ([`std::fs::canonicalize`], falling back to [`std::path::absolute`] for
/// paths that do not exist yet). The first occurrence wins; later
/// duplicates are dropped silently.
pub fn assemble_search_roots(cli_dirs: &[PathBuf], no_default_paths: bool) -> Vec<PathBuf> {
    let env_value = std::env::var(SLICER_MODULE_PATH_ENV).ok();
    assemble_search_roots_with(cli_dirs, env_value.as_deref(), no_default_paths)
}

/// Inner assembler exposed for testing without touching the process
/// environment. The `env_value` argument is what
/// [`assemble_search_roots`] would have read from `SLICER_MODULE_PATH`.
/// Production code should call [`assemble_search_roots`] instead.
pub fn assemble_search_roots_with(
    cli_dirs: &[PathBuf],
    env_value: Option<&str>,
    no_default_paths: bool,
) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    candidates.extend(cli_dirs.iter().cloned());

    if let Some(s) = env_value {
        if !s.is_empty() {
            for path in std::env::split_paths(s) {
                candidates.push(path);
            }
        }
    }

    if !no_default_paths {
        if let Some(p) = platform_config_modules_dir() {
            if p.is_dir() {
                candidates.push(p);
            }
        }
        if let Some(p) = executable_modules_dir() {
            if p.is_dir() {
                candidates.push(p);
            }
        }
    }

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut result: Vec<PathBuf> = Vec::with_capacity(candidates.len());
    for p in candidates {
        let canonical = canonicalize_for_dedup(&p);
        if seen.insert(canonical) {
            result.push(p);
        }
    }

    if std::env::var(DEBUG_ENV).is_ok() {
        eprintln!("pnp_cli: module search roots ({}):", result.len());
        for (idx, p) in result.iter().enumerate() {
            eprintln!("  [{idx}] {}", p.display());
        }
    }

    result
}

fn canonicalize_for_dedup(p: &Path) -> PathBuf {
    std::fs::canonicalize(p)
        .or_else(|_| std::path::absolute(p))
        .unwrap_or_else(|_| p.to_path_buf())
}

fn platform_config_modules_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "modular-slicer").map(|d| d.config_dir().join("modules"))
}

fn executable_modules_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(|p| p.join("modules"))
}
