//! AC-1: the feature boundary is a real, observable contract, not a doc claim.
//!
//! This target is deliberately **not** gated on `openrscad`. It builds and runs
//! in both configurations, and asserts the appropriate state in each:
//!
//! * without the feature, no engine can be reached at all — `engine_info` is
//!   `None` and `evaluate` fails `BackendDisabled` *before* parsing, which the
//!   second half of `scad_backend_feature_contract` proves with a source that
//!   does not parse;
//! * with the feature, the compiled identity matches the vendored pin and a
//!   trivial cube renders to real geometry.
//!
//! A whole-file `#![cfg(feature = "openrscad")]` would make the feature-off run
//! a green zero-test binary, which is the failure this file exists to avoid.

use slicer_scad::{engine_info, evaluate, ScadEvalOptions, ScadSource};

#[cfg(feature = "openrscad")]
use slicer_scad::{ScadEvalOutput, ScadGeometry};

#[cfg(not(feature = "openrscad"))]
const SYNTAX_ERROR_SOURCE: &str = "cube(; this is not SCAD";

#[test]
fn scad_backend_feature_contract() {
    let info = engine_info();

    #[cfg(not(feature = "openrscad"))]
    {
        assert!(
            info.is_none(),
            "a feature-off build must expose no engine identity, got {info:?}"
        );

        // A valid source.
        let source = ScadSource {
            label: "feature-off-cube".to_string(),
            text: "cube(1);".to_string(),
        };
        let error = evaluate(&source, &ScadEvalOptions::default())
            .expect_err("a feature-off build must not evaluate");
        assert_eq!(error.kind, slicer_scad::ScadErrorKind::BackendDisabled);
        assert_eq!(error.source_label, "feature-off-cube");
        assert!(
            error.message.contains("openrscad"),
            "the error must name the missing feature: {}",
            error.message
        );

        // A source that does not parse: `BackendDisabled` still wins, which is
        // only possible if the source was never handed to a parser.
        let broken = ScadSource {
            label: "feature-off-broken".to_string(),
            text: SYNTAX_ERROR_SOURCE.to_string(),
        };
        let error = evaluate(&broken, &ScadEvalOptions::default())
            .expect_err("a feature-off build must not evaluate");
        assert_eq!(
            error.kind,
            slicer_scad::ScadErrorKind::BackendDisabled,
            "feature-off must reject before parsing, not report a parse error"
        );
    }

    #[cfg(feature = "openrscad")]
    {
        let info = info.expect("an enabled build must expose its engine identity");
        assert_eq!(
            info.revision, "1a45bd33f2886c32bda32fe6e8358a67ab03f5ef",
            "the compiled revision must be the vendored pin"
        );
        assert_eq!(
            info.backend, "manifold_cpp",
            "the enabled Windows build selects the native C++ Manifold kernel"
        );
        let patch_digest = info
            .patch_digest
            .as_deref()
            .expect("a patched vendored engine must report its patch digest");
        assert_eq!(
            patch_digest,
            recomputed_patch_digest(),
            "the reported patch digest must match the committed patch files"
        );
        assert_eq!(patch_digest.len(), 64, "a SHA-256 hex digest is 64 chars");

        // AC-1 also requires real geometry, not just metadata.
        let source = ScadSource {
            label: "enabled-cube".to_string(),
            text: "cube(1);".to_string(),
        };
        let output: ScadEvalOutput =
            evaluate(&source, &ScadEvalOptions::default()).expect("a 1 mm cube must evaluate");
        let ScadGeometry::Mesh(mesh) = &output.geometry else {
            panic!("a cube is 3D geometry, got {:?}", output.geometry);
        };
        assert!(!mesh.vertices.is_empty(), "the cube must have vertices");
        assert_eq!(
            mesh.indices.len() % 3,
            0,
            "the index stream must be a whole number of triangles"
        );
        assert!(!mesh.indices.is_empty(), "the cube must have triangles");
        assert_eq!(output.provenance.engine, info);
        assert_eq!(output.provenance.source_label, "enabled-cube");
        assert_eq!(
            output.provenance.source_digest.len(),
            64,
            "the source digest is a lowercase SHA-256 hex"
        );
        assert!(
            output
                .diagnostics
                .iter()
                .all(|d| d.class == slicer_scad::ScadDiagnosticClass::Information),
            "a successful evaluation may only carry informational diagnostics"
        );
    }
}

/// Recompute the ordered patch-set digest exactly as
/// `vendor/openrscad/PATCHES.md` defines it: SHA-256 over the patch files'
/// bytes, concatenated in filename order.
#[cfg(feature = "openrscad")]
fn recomputed_patch_digest() -> String {
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;

    let patches = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/openrscad/patches");
    let mut names: Vec<_> = std::fs::read_dir(&patches)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", patches.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".patch"))
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "the vendored patch set must not be empty"
    );

    let mut hasher = Sha256::new();
    for name in names {
        let bytes = std::fs::read(patches.join(&name))
            .unwrap_or_else(|e| panic!("cannot read patch {name}: {e}"));
        hasher.update(&bytes);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}
