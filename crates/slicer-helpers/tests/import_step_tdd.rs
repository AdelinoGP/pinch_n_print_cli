//! TDD tests for STEP import (TASK-058).
//!
//! These tests verify the `import_step` function against fixture files
//! generated from truck-modeling primitives.

use slicer_helpers::{
    import_step, merge_step_meshes, StepImportError, StepLengthUnit, StepWarning,
};
use std::path::{Path, PathBuf};

mod step_fixtures;
use step_fixtures::ensure_fixtures;

/// Directory containing generated STEP test fixtures.
fn resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources")
}

// ---------------------------------------------------------------------------
// Test 1: import_step_single_solid
// ---------------------------------------------------------------------------
#[test]
fn import_step_single_solid() {
    ensure_fixtures();
    let path = resources_dir().join("cube.step");
    let result = import_step(&path).expect("import_step should succeed for cube.step");

    // Single solid → exactly 1 mesh.
    assert_eq!(result.meshes.len(), 1, "expected 1 mesh for single solid");

    // Source unit is millimetres.
    assert_eq!(result.source_unit, StepLengthUnit::Millimetre);

    // Vertices should be in internal units (10 mm cube → span 100_000 units per axis).
    let mesh = &result.meshes[0].mesh;
    assert!(
        !mesh.objects.is_empty(),
        "mesh should have at least one object"
    );
    let obj = &mesh.objects[0];
    assert!(
        !obj.mesh.vertices.is_empty(),
        "object should have vertices"
    );
    assert!(
        !obj.mesh.indices.is_empty(),
        "object should have triangle indices"
    );

    // Verify vertex extent is roughly 10 mm = 100_000 internal units.
    let xs: Vec<f32> = obj.mesh.vertices.iter().map(|v| v.x).collect();
    let span_x = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
        - xs.iter().cloned().fold(f32::INFINITY, f32::min);
    // Point3 uses f32 mm, so span should be ~10.0 mm
    assert!(
        (span_x - 10.0).abs() < 1.0,
        "expected ~10 mm span, got {span_x}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: import_step_unit_metre
// ---------------------------------------------------------------------------
#[test]
fn import_step_unit_metre() {
    ensure_fixtures();
    let path_mm = resources_dir().join("cube.step");
    let path_m = resources_dir().join("cube_metres.step");

    let result_mm = import_step(&path_mm).expect("mm cube");
    let result_m = import_step(&path_m).expect("metre cube");

    assert_eq!(result_m.source_unit, StepLengthUnit::Metre);

    // Both represent a 10 mm cube. Metre file stores 0.01 m.
    // After unit conversion both should have the same vertex extent in mm.
    let obj_mm = &result_mm.meshes[0].mesh.objects[0];
    let obj_m = &result_m.meshes[0].mesh.objects[0];

    let span_mm = vertex_span_x(obj_mm);
    let span_m = vertex_span_x(obj_m);

    // After proper unit conversion, both spans should be ~10 mm.
    assert!(
        (span_mm - span_m).abs() < 1.0,
        "mm span {span_mm} vs m span {span_m} should be close"
    );
}

// ---------------------------------------------------------------------------
// Test 3: import_step_multi_solid
// ---------------------------------------------------------------------------
#[test]
fn import_step_multi_solid() {
    ensure_fixtures();
    let path = resources_dir().join("assembly.step");
    let result = import_step(&path).expect("assembly import");

    assert!(
        result.meshes.len() >= 2,
        "expected >= 2 meshes for assembly, got {}",
        result.meshes.len()
    );
}

// ---------------------------------------------------------------------------
// Test 4: import_step_merge_components
// ---------------------------------------------------------------------------
#[test]
fn import_step_merge_components() {
    ensure_fixtures();
    let path = resources_dir().join("assembly.step");
    let result = import_step(&path).expect("assembly import");
    let multi_count = result.meshes.len();
    assert!(multi_count >= 2, "need at least 2 meshes to test merge");

    let merged = merge_step_meshes(result);
    assert_eq!(merged.meshes.len(), 1, "merged should be 1 mesh");

    // Merged mesh should have combined vertex count.
    let merged_verts = merged.meshes[0].mesh.objects[0].mesh.vertices.len();
    assert!(merged_verts > 0, "merged mesh should have vertices");
}

// ---------------------------------------------------------------------------
// Test 5: import_step_repair_applied
// ---------------------------------------------------------------------------
#[test]
fn import_step_repair_applied() {
    ensure_fixtures();
    let path = resources_dir().join("step_open_face.step");
    let result = import_step(&path).expect("open face import");

    let has_repair_warning = result.warnings.iter().any(|w| {
        matches!(w, StepWarning::RepairApplied { .. })
    });
    assert!(
        has_repair_warning,
        "expected StepWarning::RepairApplied for open-face STEP"
    );
}

// ---------------------------------------------------------------------------
// Test 6: import_step_unknown_unit_warning
// ---------------------------------------------------------------------------
#[test]
fn import_step_unknown_unit_warning() {
    ensure_fixtures();
    let path = resources_dir().join("no_unit.step");
    let result = import_step(&path).expect("no-unit import");

    // Should default to mm and emit UnknownUnit warning.
    assert_eq!(result.source_unit, StepLengthUnit::Unknown);
    let has_unknown_unit = result
        .warnings
        .iter()
        .any(|w| matches!(w, StepWarning::UnknownUnit));
    assert!(
        has_unknown_unit,
        "expected StepWarning::UnknownUnit for STEP with no unit declaration"
    );
}

// ---------------------------------------------------------------------------
// Test 7: import_step_not_found_error
// ---------------------------------------------------------------------------
#[test]
fn import_step_not_found_error() {
    let result = import_step(Path::new("/tmp/nonexistent_file_12345.step"));
    assert!(
        matches!(result, Err(StepImportError::FileNotFound(_))),
        "expected FileNotFound, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 8: import_step_invalid_file_error
// ---------------------------------------------------------------------------
#[test]
fn import_step_invalid_file_error() {
    ensure_fixtures();
    let path = resources_dir().join("garbage.bin");
    // Write binary garbage.
    std::fs::write(&path, &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03]).unwrap();

    let result = import_step(&path);
    assert!(
        matches!(result, Err(StepImportError::ParseError(_))),
        "expected ParseError, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn vertex_span_x(obj: &slicer_ir::ObjectMesh) -> f32 {
    let xs: Vec<f32> = obj.mesh.vertices.iter().map(|v| v.x).collect();
    xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
        - xs.iter().cloned().fold(f32::INFINITY, f32::min)
}
