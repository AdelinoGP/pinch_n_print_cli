//! TDD tests for `[[region_split]]` manifest ingestion validators.
//!
//! Covers AC-1, AC-3, AC-4, AC-5, AC-6, and AC-N3 from packet 92.
//! Each test loads a static fixture TOML (copied into a TempDir alongside a
//! dummy `.wasm`) via the public `load_module_from_paths` API.

#![allow(missing_docs)]

use std::fs;
use std::path::Path;

use slicer_scheduler::{load_module_from_paths, LoadErrorKind, RegionSplitValueType};
use tempfile::TempDir;

// -- helpers ------------------------------------------------------------------

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/region_split_manifests"
);

/// Copy a static fixture TOML into a fresh TempDir and write a dummy `.wasm`
/// beside it. Returns the `TempDir` (must stay alive) and the manifest path.
fn stage_fixture(name: &str) -> (TempDir, std::path::PathBuf) {
    let source = Path::new(FIXTURES).join(name);
    let tmp = tempfile::Builder::new()
        .prefix(&format!("rs-manifest-{}-", name.trim_end_matches(".toml")))
        .tempdir()
        .expect("create temp dir for fixture");

    let stem = name.trim_end_matches(".toml");
    let manifest_path = tmp.path().join(format!("{stem}.toml"));
    let wasm_path = tmp.path().join(format!("{stem}.wasm"));

    fs::copy(&source, &manifest_path).expect("copy fixture TOML");
    fs::write(&wasm_path, b"placeholder wasm").expect("write dummy wasm");

    (tmp, manifest_path)
}

// -- AC-1: basic round-trip ---------------------------------------------------

/// AC-1: a minimal manifest with one valid `[[region_split]]` loads cleanly
/// and the resulting `LoadedModule` exposes the declared split.
#[test]
fn region_split_manifest_basic() {
    let (_tmp, manifest) = stage_fixture("basic.toml");
    let wasm = manifest.with_extension("wasm");

    let module =
        load_module_from_paths(&manifest, &wasm).expect("basic fixture should load without error");

    assert_eq!(
        module.region_splits().len(),
        1,
        "expected exactly one region_split declaration"
    );
    let decl = &module.region_splits()[0];
    assert_eq!(decl.semantic, "material");
    assert_eq!(decl.priority, 100);
    assert_eq!(decl.value_type, RegionSplitValueType::ToolIndex);
}

// -- AC-3: duplicate semantic rejection ---------------------------------------

/// AC-3: two `[[region_split]]` entries sharing a `semantic` are rejected with
/// `LoadErrorKind::DuplicateRegionSplitSemantic`.
#[test]
fn region_split_duplicate_semantic_rejected() {
    let (_tmp, manifest) = stage_fixture("duplicate_semantic.toml");
    let wasm = manifest.with_extension("wasm");

    let err =
        load_module_from_paths(&manifest, &wasm).expect_err("duplicate semantics must be rejected");

    match &err.kind {
        LoadErrorKind::DuplicateRegionSplitSemantic { semantic, .. } => {
            assert_eq!(semantic, "material");
        }
        other => panic!("expected DuplicateRegionSplitSemantic, got: {other:?}"),
    }
}

// -- AC-4: scalar value_type rejection ----------------------------------------

/// AC-4: `value_type = "scalar"` is architecturally forbidden and must surface
/// as `LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit`.
#[test]
fn region_split_scalar_rejected() {
    let (_tmp, manifest) = stage_fixture("scalar_value_type.toml");
    let wasm = manifest.with_extension("wasm");

    let err =
        load_module_from_paths(&manifest, &wasm).expect_err("scalar value_type must be rejected");

    match &err.kind {
        LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit { semantic } => {
            // The semantic field was readable from the raw TOML.
            assert_eq!(semantic, "material");
        }
        other => panic!("expected ScalarValueTypeNotAllowedInRegionSplit, got: {other:?}"),
    }
}

// -- AC-5: community priority below floor -------------------------------------

/// AC-5: a community semantic (not in `CORE_REGION_SPLIT_PRIORITIES`) with a
/// priority below `COMMUNITY_PRIORITY_FLOOR` (1000) is rejected.
#[test]
fn region_split_community_priority_floor() {
    let (_tmp, manifest) = stage_fixture("community_below_floor.toml");
    let wasm = manifest.with_extension("wasm");

    let err = load_module_from_paths(&manifest, &wasm)
        .expect_err("community priority below floor must be rejected");

    match &err.kind {
        LoadErrorKind::CommunityPriorityBelowFloor {
            semantic,
            given_priority,
            floor,
        } => {
            assert_eq!(semantic, "com.example.foo");
            assert_eq!(*given_priority, 250);
            assert_eq!(*floor, 1000);
        }
        other => panic!("expected CommunityPriorityBelowFloor, got: {other:?}"),
    }
}

// -- AC-6: core priority mismatch ---------------------------------------------

/// AC-6: a core semantic declared with a priority other than its registered
/// value is rejected as `LoadErrorKind::CorePriorityMismatch`.
#[test]
fn region_split_core_priority_mismatch() {
    let (_tmp, manifest) = stage_fixture("core_priority_mismatch.toml");
    let wasm = manifest.with_extension("wasm");

    let err = load_module_from_paths(&manifest, &wasm)
        .expect_err("core priority mismatch must be rejected");

    match &err.kind {
        LoadErrorKind::CorePriorityMismatch {
            semantic,
            given_priority,
            expected_priority,
        } => {
            assert_eq!(semantic, "material");
            assert_eq!(*given_priority, 100000);
            assert_eq!(*expected_priority, 100);
        }
        other => panic!("expected CorePriorityMismatch, got: {other:?}"),
    }
}

// -- AC-N3: priority type mismatch (string instead of u32) --------------------

/// AC-N3: `priority = "not-a-number"` (string where u32 expected) surfaces as
/// `LoadErrorKind::TomlParse` or `LoadErrorKind::Schema` (toml-serde
/// deserialization failure).
#[test]
fn region_split_priority_type_mismatch() {
    let (_tmp, manifest) = stage_fixture("priority_type_mismatch.toml");
    let wasm = manifest.with_extension("wasm");

    let err = load_module_from_paths(&manifest, &wasm)
        .expect_err("non-integer priority must be rejected");

    assert!(
        matches!(&err.kind, LoadErrorKind::TomlParse | LoadErrorKind::Schema),
        "expected TomlParse or Schema for non-integer priority, got: {:?}",
        err.kind
    );
}
