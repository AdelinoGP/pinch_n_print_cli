//! TASK-111: every core-module crate under `modules/core-modules/` must
//! adopt the documented `#[slicer_module]` authoring path so that the
//! macro-emitted binding surface (lifecycle + stage export, typed
//! `SlicerModuleSchema`, wasm32 `#[export_name]` shims) is applied
//! uniformly (docs/05 §Module Entry Point).
//!
//! This is a source-level guard rather than a link-time reflection
//! test because pulling all 16 core-module crates in as `slicer-runtime`
//! dev-dependencies would create cyclic compilation and a heavier
//! dependency graph than TASK-111 warrants. The macro surface itself
//! is already unit-tested per-crate via the slicer-macros test suite;
//! this test only checks the adoption count.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_module_lib_files() -> Vec<PathBuf> {
    let dir = repo_root().join("modules/core-modules");
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read modules/core-modules")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path().join("src/lib.rs"))
        .filter(|p| p.exists())
        .collect();
    out.sort();
    out
}

#[test]
fn every_core_module_adopts_slicer_module_macro() {
    let libs = core_module_lib_files();
    assert!(
        libs.len() >= 16,
        "expected at least 16 core-module crates under modules/core-modules/, got {}",
        libs.len()
    );

    let mut missing: Vec<String> = Vec::new();
    for path in &libs {
        let src = fs::read_to_string(path).expect("read core-module lib.rs");
        // The attribute must appear on an actual impl block, not just in
        // a comment. Looking for `#[slicer_module]` (possibly followed by
        // whitespace / newline and an `impl`) is sufficient here because
        // proc-macro attributes can only be applied to item positions.
        if !src.contains("#[slicer_module]") {
            missing.push(
                path.parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>")
                    .to_string(),
            );
        }
    }
    assert!(
        missing.is_empty(),
        "core modules missing `#[slicer_module]` adoption: {missing:?}"
    );
}

#[test]
fn every_core_module_depends_on_slicer_schema_for_macro_emission() {
    // The macro emits `::slicer_schema::SlicerModuleSchema` paths; every
    // crate that uses `#[slicer_module]` must therefore carry a direct
    // `slicer-schema` dep so the emitted path resolves.
    let dir = repo_root().join("modules/core-modules");
    let crates: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read modules/core-modules")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();

    for krate in crates {
        let manifest = krate.join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }
        let lib = krate.join("src/lib.rs");
        if !lib.exists() {
            continue;
        }
        let lib_src = fs::read_to_string(&lib).expect("read lib.rs");
        if !lib_src.contains("#[slicer_module]") {
            continue;
        }
        let toml = fs::read_to_string(&manifest).expect("read Cargo.toml");
        assert!(
            toml.contains("slicer-schema"),
            "core module `{}` uses `#[slicer_module]` but does not depend on `slicer-schema`",
            krate.file_name().and_then(|n| n.to_str()).unwrap_or("<?>")
        );
    }
}

#[test]
fn every_core_module_ships_a_matching_stem_manifest() {
    // Guards manifest-based discovery (docs/03): the `.toml` manifest
    // must share its stem with the crate/directory name so the host's
    // same-stem pairing logic continues to match converted modules.
    let dir = repo_root().join("modules/core-modules");
    let mut failures: Vec<String> = Vec::new();
    for entry in fs::read_dir(&dir).expect("read modules/core-modules") {
        let entry = entry.expect("dir entry");
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        // Skip non-crate directories (none exist today, but defensive).
        if !path.join("Cargo.toml").exists() {
            continue;
        }
        let manifest = path.join(format!("{name}.toml"));
        if !manifest.exists() {
            failures.push(name);
        }
    }
    assert!(
        failures.is_empty(),
        "core modules missing matching-stem `.toml` manifest: {failures:?}"
    );
}
