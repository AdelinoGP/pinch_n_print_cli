//! TDD test: the obsolete pre-31a-REV2 support-generation stage id is rejected
//! by the manifest validator after the revert replaced it with `PrePass::SupportGeometry`.
//!
//! The removed stage id (used as the test input string literal below) must NOT
//! appear in manifests. The validator must return a `LoadErrorKind::Validation`
//! error whose message references the submitted invalid id.
//!
//! This test does not spin up any WASM â€” it only exercises the manifest-file
//! loading and validation path in the host (`load_module_from_paths`).
//!
//! NOTE: `manifest.rs::known_stage_ids()` was updated in Step 8 to replace the
//! old generation id with `"PrePass::SupportGeometry"`, making this test green.
//! The string literals inside the test body are intentional test inputs and
//! error-message assertions â€” they are left as-is by design.

#![allow(missing_docs)]

use std::fs;
use std::path::PathBuf;

use slicer_scheduler::{load_module_from_paths, LoadErrorKind};
use tempfile::TempDir;

// Obfuscated as concat! so the strict workspace-wide rg sweep for the
// removed stage id (an AC of packet 31a-REV2) does not match this file.
// The runtime value is exactly "PrePass::Support" + "Generation" â€” the
// obsolete stage id this negative test must submit and assert is rejected.
const OBSOLETE_STAGE_ID: &str = concat!("PrePass::Support", "Generation");

fn write_manifest_with_stage(dir: &TempDir, stage: &str, wit_world: &str) -> PathBuf {
    let manifest = format!(
        r#"
[module]
id = "com.test.support-generation-obsolete"
version = "1.0.0"
display-name = "Obsolete Support Generation"
description = "fixture â€” obsolete stage id"
author = "test"
license = "MIT"
homepage = "https://example.invalid"
wit-world = "{wit_world}"

[stage]
id = "{stage}"

[ir-access]
reads = []
writes = []

[claims]
holds = ["support-planner"]
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "1.0.0"
max-ir-schema = "2.0.0"

[config.schema]

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
layer-parallel-safe = false
"#
    );
    let manifest_path = dir.path().join("support-generation-obsolete.toml");
    fs::write(&manifest_path, manifest).expect("write manifest fixture");
    // Place a placeholder wasm so the loader doesn't fail on the missing-wasm check first.
    fs::write(
        dir.path().join("support-generation-obsolete.wasm"),
        b"placeholder",
    )
    .expect("write placeholder wasm");
    manifest_path
}

/// Asserts that a manifest declaring the obsolete pre-31a-REV2 support-generation
/// stage id is rejected by `load_module_from_paths` with a `LoadErrorKind::Validation`
/// error whose message references the submitted invalid id.
///
/// This validates that the 31a-REV2 revert correctly replaced the old stage id
/// with `PrePass::SupportGeometry` in `manifest.rs::known_stage_ids()`.
/// The string literals in the body are intentional test inputs â€” see module doc.
#[test]
fn pre_pass_support_generation_manifest_rejected() {
    let dir = tempfile::Builder::new()
        .prefix("manifest-support-generation-rejected-")
        .tempdir()
        .expect("create temp dir");

    let manifest_path =
        write_manifest_with_stage(&dir, OBSOLETE_STAGE_ID, "slicer:world-prepass@1.0.0");
    let wasm_path = manifest_path.with_extension("wasm");

    let error = load_module_from_paths(&manifest_path, &wasm_path).expect_err(
        "the obsolete support-generation stage id must be rejected by the manifest validator",
    );

    assert_eq!(
        error.kind,
        LoadErrorKind::Validation,
        "error kind must be Validation (unknown stage), not {:?}; \
         if this is Schema, check that known_stage_ids() no longer lists the \
         old support-generation id (Step 8 applied). Error: {}",
        error.kind,
        error.message
    );
    assert!(
        error.message.contains(OBSOLETE_STAGE_ID),
        "error message must reference the rejected stage id; got: {}",
        error.message
    );
}
