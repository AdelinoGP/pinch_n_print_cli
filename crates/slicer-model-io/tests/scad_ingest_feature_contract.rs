//! Feature-routing contract for SCAD model ingestion (AC-1).
//!
//! This target is intentionally ungated: both feature states must run real
//! assertions, so a feature-off run cannot be a green zero-test binary.
//!
//! What is proved here is routing, not evaluation: `.scad`/`.SCAD` detection is
//! available in every build, `load_model` refuses SCAD before it opens or sniffs
//! the file, and the refusal names the feature-appropriate activation path.

use std::path::PathBuf;

use slicer_model_io::loader::{detect_format, load_model, ModelFormat, ModelLoadError};

fn write_file(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("write fixture");
    path
}

/// Assert the feature-appropriate refusal, including its stable display fragment.
#[cfg(not(feature = "scad"))]
fn assert_refused(err: &ModelLoadError, context: &str) {
    assert!(
        matches!(err, ModelLoadError::ScadSupportDisabled),
        "{context}: expected ScadSupportDisabled, got {err:?}"
    );
    assert!(
        err.to_string().contains("SCAD_SUPPORT_DISABLED"),
        "{context}: display must carry SCAD_SUPPORT_DISABLED: {err}"
    );
}

/// Assert the feature-appropriate refusal, including its stable display fragment.
#[cfg(feature = "scad")]
fn assert_refused(err: &ModelLoadError, context: &str) {
    assert!(
        matches!(err, ModelLoadError::ScadOptionsRequired),
        "{context}: expected ScadOptionsRequired, got {err:?}"
    );
    assert!(
        err.to_string().contains("SCAD_OPTIONS_REQUIRED"),
        "{context}: display must carry SCAD_OPTIONS_REQUIRED: {err}"
    );
}

#[test]
fn scad_model_feature_routing() {
    // Detection is feature-independent and case-insensitive.
    assert_eq!(detect_format("model.scad").unwrap(), ModelFormat::Scad);
    assert_eq!(detect_format("model.SCAD").unwrap(), ModelFormat::Scad);
    assert_eq!(detect_format("model.stl").unwrap(), ModelFormat::Stl);
    assert_eq!(detect_format("model.obj").unwrap(), ModelFormat::Obj);
    assert_eq!(detect_format("model.3mf").unwrap(), ModelFormat::ThreeMf);
    assert!(matches!(
        detect_format("model.step"),
        Err(ModelLoadError::UnsupportedFormat(ext)) if ext == "step"
    ));

    let dir = tempfile::tempdir().expect("temp dir");
    let invalid = write_file(&dir, "invalid.scad", b"this is not valid scad (((");
    let err = load_model(&invalid).expect_err("load_model must refuse SCAD input");
    assert_refused(&err, "invalid .scad");

    // No parsing and no I/O: a non-existent `.scad` path must produce the same
    // refusal variant, not `ModelLoadError::Io`.
    let missing = dir.path().join("never-created.scad");
    assert!(!missing.exists(), "the missing-path probe must not exist");
    let missing_err = load_model(&missing).expect_err("a missing SCAD path must still be refused");
    assert_refused(&missing_err, "missing .scad");

    // Uppercase extension takes the identical branch.
    let upper = write_file(&dir, "INVALID.SCAD", b"this is not valid scad (((");
    let upper_err = load_model(&upper).expect_err("uppercase SCAD input must be refused");
    assert_refused(&upper_err, "uppercase .SCAD");

    // Both diagnostic texts exist in both builds. The disabled message names
    // both activation paths; the options message names the explicit entry.
    let disabled = ModelLoadError::ScadSupportDisabled.to_string();
    assert!(
        disabled.contains("pnp-cli/scad"),
        "the disabled message must name the CLI feature: {disabled}"
    );
    assert!(
        disabled.contains("slicer-model-io/scad"),
        "the disabled message must name the library feature: {disabled}"
    );
    let options = ModelLoadError::ScadOptionsRequired.to_string();
    assert!(
        options.contains("load_scad_model"),
        "the options message must name the explicit entry: {options}"
    );
    assert!(
        options.contains("never evaluates"),
        "the options message must state that load_model does not evaluate implicitly: {options}"
    );
}

#[test]
fn scad_detection_is_case_insensitive_and_not_evaluation() {
    // `detect_format` inspects only the extension: it must not open the path.
    let missing = std::path::Path::new("no-such-directory/never-created.SCAD");
    assert!(!missing.exists(), "the detection probe must not exist");
    assert_eq!(detect_format(missing).unwrap(), ModelFormat::Scad);

    // A `.scad` file whose bytes are a valid ASCII STL is still routed to the
    // SCAD refusal: `load_model` neither sniffs content nor falls back to a
    // mesh parser by payload.
    let dir = tempfile::tempdir().expect("temp dir");
    let stl_payload = b"solid cube\n  facet normal 0 0 -1\n    outer loop\n      vertex 0 0 0\n      vertex 1 1 0\n      vertex 1 0 0\n    endloop\n  endfacet\nendsolid cube\n";
    let disguised = write_file(&dir, "disguised.scad", stl_payload);
    let err = load_model(&disguised).expect_err("an STL payload named .scad must not be parsed");
    assert_refused(&err, "disguised STL payload");

    // Valid SCAD text is refused by the same non-evaluating entry.
    let valid = write_file(&dir, "valid.scad", b"cube([2,2,2]);");
    let err = load_model(&valid).expect_err("valid SCAD text must still be refused");
    assert_refused(&err, "valid SCAD text");
}
