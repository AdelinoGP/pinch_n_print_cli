//! Host-side test support: locating the `pnp_cli` binary and asserting it is
//! not stale before a test, bench, or integration harness spawns it.
//!
//! **Home decision: see `docs/adr/0054-host-side-test-support-crate.md`
//! (ADR-0054).** That ADR fixes three rules this crate must keep:
//!
//! 1. **std-only** — this crate declares no `[dependencies]`. It is a
//!    dev-dependency of `slicer-runtime` and `slicer-scheduler`, so any
//!    dependency added here is taxed onto every narrow
//!    `cargo test -p slicer-runtime` / `cargo test -p slicer-scheduler` build.
//! 2. **dev-dependency only** — no production crate may depend on it.
//! 3. **host-side only** — it never compiles into guest WASM. Guest-side test
//!    support is `slicer_sdk::test_support`, governed by **ADR-0004**; the two
//!    surfaces are disjoint and neither re-exports the other.
//!
//! Before ADR-0054 this logic was copy-pasted across seven host-side sites.
//! Packet 162 fixed three of them in place and deferred the extraction; the
//! other four had no freshness gate at all and branched on a `PROFILE`
//! environment variable that Cargo never sets for test binaries. `slicer-test`,
//! the crate that might otherwise have hosted this, was deleted by packet 78.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Canonicalized workspace root (the parent of `crates/`).
///
/// Derived from this crate's own `CARGO_MANIFEST_DIR`
/// (`<root>/crates/slicer-test-support`), so two `parent()` levels reach the
/// workspace root regardless of which crate's test is calling.
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root canonicalize")
}

/// Newest mtime across the sources that actually link into `pnp_cli`.
///
/// Scope: every file under `crates/*/src/**`, each `crates/*/Cargo.toml`, every
/// `*.wit` under `crates/slicer-schema/wit/**`, and the workspace `Cargo.toml`.
///
/// Deliberately **excludes** `tests/`, `benches/`, and `modules/` — those do not
/// link into the binary, and a scan that fires on every test-file edit is a gate
/// that gets disabled.
///
/// Note the accepted over-approximation recorded in ADR-0054: this crate's own
/// `src/` sits under `crates/*/src/**` and therefore counts toward `pnp_cli`
/// staleness even though it never links into that binary. That fails loud, not
/// silent.
pub fn newest_source_mtime(root: &Path) -> SystemTime {
    fn visit(path: &Path, extension: Option<&str>, newest: &mut SystemTime) {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, extension, newest);
            } else if path.is_file() {
                let matches_extension = match extension {
                    Some(wanted) => path.extension().and_then(|s| s.to_str()) == Some(wanted),
                    None => true,
                };
                if !matches_extension {
                    continue;
                }
                if let Ok(mtime) = std::fs::metadata(&path).and_then(|metadata| metadata.modified())
                {
                    *newest = (*newest).max(mtime);
                }
            }
        }
    }

    let mut newest = UNIX_EPOCH;
    let crates_root = root.join("crates");
    if let Ok(entries) = std::fs::read_dir(&crates_root) {
        for entry in entries.flatten() {
            let crate_root = entry.path();
            if !crate_root.is_dir() {
                continue;
            }
            visit(&crate_root.join("src"), None, &mut newest);
            let manifest = crate_root.join("Cargo.toml");
            if let Ok(mtime) = std::fs::metadata(manifest).and_then(|metadata| metadata.modified())
            {
                newest = newest.max(mtime);
            }
        }
    }
    visit(
        &root.join("crates/slicer-schema/wit"),
        Some("wit"),
        &mut newest,
    );
    if let Ok(mtime) =
        std::fs::metadata(root.join("Cargo.toml")).and_then(|metadata| metadata.modified())
    {
        newest = newest.max(mtime);
    }
    newest
}

/// Return a diagnostic when the CLI artifact is absent or older than sources.
///
/// This is the pure, testable seam: `None` for a fresh artifact, `Some(reason)`
/// otherwise. Every message names `pnp_cli`, contains the word `stale`, and
/// carries the remedy `cargo build -p pnp-cli`.
///
/// **This is a deliberate mirror of `is_stale` in `xtask/src/build_guests.rs`,
/// not an import.** `xtask` is a bin-only crate — it declares no `[lib]` and
/// mounts `build_guests` as a private `mod` of `xtask/src/main.rs` — so there is
/// nothing for a test target to `use`. The two arms here correspond one-for-one
/// with the two mtime disjuncts of `is_stale`: a missing artifact is stale
/// (`artifact_mtime.is_none_or(..)`), and `newest_mtime > artifact` is stale.
///
/// `is_stale` has a **third** disjunct this mirror deliberately omits: it also
/// reports stale when `metadata_matches` finds that the fingerprint recorded at
/// `fingerprint_metadata_path` differs from the freshly computed
/// `FreshnessSnapshot::fingerprint` for that guest. That is a content hash over
/// the guest's resolved input set — shared crates, the guest's own inputs, and
/// its per-stage WIT package — persisted under `target/guest-fingerprints/`, and
/// it exists to catch input-set changes that leave mtimes looking fine (a file
/// removed from the set, a checkout that rewinds content, a stage's WIT dir
/// swapped). It has no analogue here: `pnp_cli` is an ordinary Cargo binary with
/// no per-artifact fingerprint sidecar, and Cargo itself owns the equivalent
/// rebuild decision. Omission is intentional, not drift.
///
/// Keep the two functions legible as siblings if either changes (ADR-0054).
pub fn staleness_reason(
    bin_mtime: Option<SystemTime>,
    newest_src_mtime: SystemTime,
) -> Option<String> {
    match bin_mtime {
        None => Some(
            "pnp_cli is stale because its resolved path is absent; \
             run `cargo build -p pnp-cli`."
                .to_string(),
        ),
        Some(artifact_mtime) if newest_src_mtime > artifact_mtime => Some(
            "pnp_cli is stale: it is older than crates/*/src/**; \
             run `cargo build -p pnp-cli` to rebuild it."
                .to_string(),
        ),
        Some(_) => None,
    }
}

/// Path to the compiled `pnp_cli` binary, asserted fresh.
///
/// Resolves the binary whose build profile matches the caller's own. Cargo lays
/// out test/bench executables at
/// `target/{debug,release}/deps/<bucket>-<hash>{.exe}`, so the caller's profile
/// dir is `current_exe().parent().parent()` and the sibling `pnp_cli{.exe}` is
/// the right-profile binary.
///
/// There is **no release/debug fallback probe**: packet 162 removed it, because
/// silently falling back to a binary from the other profile is exactly how a
/// stale binary produces a plausible-but-wrong measurement instead of a failure.
/// Do not re-introduce one.
///
/// # Panics
///
/// Panics when the binary is absent or stale, and when the caller's own
/// executable path cannot be resolved.
pub fn pnp_cli_bin() -> PathBuf {
    let exe_name = if cfg!(windows) {
        "pnp_cli.exe"
    } else {
        "pnp_cli"
    };

    if let Ok(test_exe) = std::env::current_exe() {
        if let Some(profile_dir) = test_exe.parent().and_then(|p| p.parent()) {
            let bin = profile_dir.join(exe_name);
            let root = workspace_root();
            let newest_src_mtime = newest_source_mtime(&root);
            if let Some(reason) = staleness_reason(
                std::fs::metadata(&bin)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok()),
                newest_src_mtime,
            ) {
                panic!(
                    "{reason} Resolved path: {}. Note: a narrow `cargo test -p <crate>` \
                     does not rebuild another package's binary.",
                    bin.display(),
                );
            }
            return bin;
        }
    }
    panic!(
        "could not resolve the pnp_cli path from the test executable; \
         run `cargo build -p pnp-cli` first."
    );
}
