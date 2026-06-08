//! TDD tests for `region_split::aggregate_region_splits` and
//! `region_split::canonical_variant_chain_order`.
//!
//! Covers AC-7, AC-8, and AC-N2 from packet 92.

#![allow(missing_docs)]

use std::fs;
use std::path::Path;

use slicer_scheduler::{
    region_split::{aggregate_region_splits, canonical_variant_chain_order},
    DiagnosticLevel, LoadDiagnostic,
};
use tempfile::TempDir;

// -- fixture helpers ----------------------------------------------------------

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/region_split_manifests/aggregation"
);

/// Copy a fixture TOML from the `aggregation/` subdirectory into a fresh
/// TempDir with a dummy `.wasm` beside it. Returns `(TempDir, manifest_path)`.
fn stage_fixture(name: &str) -> (TempDir, std::path::PathBuf) {
    let source = Path::new(FIXTURES).join(name);
    let tmp = tempfile::Builder::new()
        .prefix(&format!("rs-agg-{}-", name.trim_end_matches(".toml")))
        .tempdir()
        .expect("create temp dir");

    let stem = name.trim_end_matches(".toml");
    let manifest_path = tmp.path().join(format!("{stem}.toml"));
    let wasm_path = tmp.path().join(format!("{stem}.wasm"));

    fs::copy(&source, &manifest_path).expect("copy fixture TOML");
    fs::write(&wasm_path, b"placeholder wasm").expect("write dummy wasm");

    (tmp, manifest_path)
}

/// Load a module from a staged fixture using `slicer_scheduler::load_module_from_paths`.
fn load_fixture(name: &str) -> (TempDir, slicer_scheduler::LoadedModule) {
    let (tmp, manifest) = stage_fixture(name);
    let wasm = manifest.with_extension("wasm");
    let module = slicer_scheduler::load_module_from_paths(&manifest, &wasm)
        .unwrap_or_else(|e| panic!("fixture {name} should load cleanly: {e:?}"));
    (tmp, module)
}

// -- AC-8: canonical order ----------------------------------------------------

/// AC-8: `canonical_variant_chain_order` returns semantics in `(priority, name)`
/// ascending order across three modules: `material@100`, `fuzzy_skin@200`,
/// `com.example.expansion@1500`.
#[test]
fn region_split_aggregation_canonical_order() {
    let (_ta, mod_a) = load_fixture("a.toml");
    let (_tb, mod_b) = load_fixture("b.toml");
    let (_tc, mod_c) = load_fixture("c.toml");

    let modules = [mod_a, mod_b, mod_c];
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();

    let agg = aggregate_region_splits(&modules, &mut diagnostics);

    // No warnings expected for three distinct priorities.
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for three distinct-priority semantics, got: {diagnostics:?}"
    );

    let order = canonical_variant_chain_order(&agg);
    assert_eq!(
        order,
        vec!["material", "fuzzy_skin", "com.example.expansion"],
        "canonical order must be (priority, name) ascending"
    );
}

// -- AC-7: tied priority WARN -------------------------------------------------

/// AC-7: two modules declaring distinct semantics at the same priority must
/// produce a `DiagnosticLevel::Warning` diagnostic whose message contains
/// "Tied", the first semantic name, and the second semantic name.
/// The returned BTreeMap must be non-empty (aggregation succeeds).
#[test]
fn region_split_tied_priority_warn() {
    let (_ta, mod_alpha) = load_fixture("tied_alpha.toml");
    let (_tb, mod_beta) = load_fixture("tied_beta.toml");

    let modules = [mod_alpha, mod_beta];
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();

    let agg = aggregate_region_splits(&modules, &mut diagnostics);

    // Aggregation must succeed and contain both semantics.
    assert!(
        !agg.is_empty(),
        "aggregated map must be non-empty after tied-priority input"
    );
    assert!(
        agg.contains_key("com.example.alpha"),
        "alpha must be in the map"
    );
    assert!(
        agg.contains_key("com.example.beta"),
        "beta must be in the map"
    );

    // Exactly one tied-priority warning.
    let tied_warn = diagnostics.iter().any(|d| {
        d.level == DiagnosticLevel::Warning
            && d.message.contains("Tied")
            && d.message.contains("com.example.alpha")
            && d.message.contains("com.example.beta")
    });
    assert!(
        tied_warn,
        "expected a Warning diagnostic mentioning 'Tied', 'com.example.alpha', \
         and 'com.example.beta'; diagnostics: {diagnostics:?}"
    );
}

// -- AC-N2: empty input -------------------------------------------------------

/// AC-N2: `aggregate_region_splits` on an empty module slice returns an empty
/// BTreeMap and emits no diagnostics.
#[test]
fn region_split_aggregation_empty_default() {
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let agg = aggregate_region_splits(&[], &mut diagnostics);

    assert!(agg.is_empty(), "empty input must yield an empty BTreeMap");
    assert!(
        diagnostics.is_empty(),
        "empty input must yield no diagnostics, got: {diagnostics:?}"
    );
}
