//! Generates STEP test fixture files from truck-modeling primitives.
//!
//! Fixtures are written to `tests/resources/` on first use. Once generated
//! they are stable — delete the directory to regenerate.

use std::path::{Path, PathBuf};
use std::sync::Once;

use truck_modeling::*;
use truck_stepio::out::*;
use truck_topology::compress::CompressedShell as CShell;
use truck_topology::compress::CompressedSolid as CSolid;

static INIT: Once = Once::new();

/// Resource directory path.
fn resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources")
}

/// Ensure all fixture files exist. Called once per test run.
pub fn ensure_fixtures() {
    INIT.call_once(|| {
        let dir = resources_dir();
        std::fs::create_dir_all(&dir).unwrap();

        write_cube_mm(&dir);
        write_cube_metres(&dir);
        write_assembly(&dir);
        write_open_face(&dir);
        write_no_unit(&dir);
    });
}

/// Build a 10mm cube solid centred at origin using truck-modeling.
fn make_cube(size: f64) -> Solid {
    let half = size / 2.0;
    let v0 = builder::vertex(Point3::new(-half, -half, -half));
    let v1 = builder::vertex(Point3::new(half, -half, -half));
    let v2 = builder::vertex(Point3::new(half, half, -half));
    let v3 = builder::vertex(Point3::new(-half, half, -half));

    let e0 = builder::line(&v0, &v1);
    let e1 = builder::line(&v1, &v2);
    let e2 = builder::line(&v2, &v3);
    let e3 = builder::line(&v3, &v0);

    let wire = Wire::from(vec![e0, e1, e2, e3]);
    let face = builder::try_attach_plane(&[wire]).unwrap();
    builder::tsweep(&face, Vector3::new(0.0, 0.0, size))
}

/// Compress a solid and format as a STEP string.
fn solid_to_step(solid: &Solid) -> String {
    let compressed: CSolid<Point3, Curve, Surface> = solid.compress();
    CompleteStepDisplay::new(StepModel::from(&compressed), Default::default()).to_string()
}

/// Compress a shell and format as a STEP string.
fn shell_to_step(shell: &Shell) -> String {
    let compressed: CShell<Point3, Curve, Surface> = shell.compress();
    CompleteStepDisplay::new(StepModel::from(&compressed), Default::default()).to_string()
}

/// Compress multiple solids and format as a single STEP string.
fn solids_to_step(solids: &[Solid]) -> String {
    let compressed: Vec<CSolid<Point3, Curve, Surface>> =
        solids.iter().map(|s| s.compress()).collect();
    CompleteStepDisplay::new(StepModels::from_iter(&compressed), Default::default()).to_string()
}

// ---------------------------------------------------------------------------
// cube.step — single 10 mm cube, mm units
// ---------------------------------------------------------------------------
fn write_cube_mm(dir: &Path) {
    let path = dir.join("cube.step");
    if path.exists() {
        return;
    }
    // truck already emits SI_UNIT(.MILLI.,.METRE.) so no injection needed.
    let solid = make_cube(10.0);
    let step = solid_to_step(&solid);
    std::fs::write(&path, step).unwrap();
}

// ---------------------------------------------------------------------------
// cube_metres.step — 10 mm cube in metre units (size = 0.01 m)
// ---------------------------------------------------------------------------
fn write_cube_metres(dir: &Path) {
    let path = dir.join("cube_metres.step");
    if path.exists() {
        return;
    }
    // 10 mm = 0.01 m
    let solid = make_cube(0.01);
    let mut step = solid_to_step(&solid);
    // Replace the truck-generated .MILLI. with $ (no prefix = base metre).
    step = step.replace(".MILLI.,.METRE.", "$,.METRE.");
    std::fs::write(&path, step).unwrap();
}

// ---------------------------------------------------------------------------
// assembly.step — two distinct solids
// ---------------------------------------------------------------------------
fn write_assembly(dir: &Path) {
    let path = dir.join("assembly.step");
    if path.exists() {
        return;
    }
    let solid1 = make_cube(10.0);
    let solid2 = builder::translated(&make_cube(5.0), Vector3::new(20.0, 0.0, 0.0));
    let step = solids_to_step(&[solid1, solid2]);
    std::fs::write(&path, step).unwrap();
}

// ---------------------------------------------------------------------------
// step_open_face.step — cube with one face removed (open shell)
// ---------------------------------------------------------------------------
fn write_open_face(dir: &Path) {
    let path = dir.join("step_open_face.step");
    if path.exists() {
        return;
    }
    // Create a cube and extract its boundary shell, then remove the last face
    // to create a non-manifold (open) mesh.
    let solid = make_cube(10.0);
    let shell = solid.into_boundaries().pop().unwrap();
    // Remove the last face to make it open.
    let faces: Vec<_> = shell.face_into_iter().collect();
    let open_shell: Shell = faces[..faces.len() - 1].iter().cloned().collect();
    let step = shell_to_step(&open_shell);
    std::fs::write(&path, step).unwrap();
}

// ---------------------------------------------------------------------------
// no_unit.step — STEP with no unit declaration
// ---------------------------------------------------------------------------
fn write_no_unit(dir: &Path) {
    let path = dir.join("no_unit.step");
    if path.exists() {
        return;
    }
    // Generate a STEP file and strip all unit-related entities so there's no
    // LENGTH_UNIT declaration at all.
    let solid = make_cube(10.0);
    let mut step = solid_to_step(&solid);
    // Remove lines containing unit-related entities.
    let lines: Vec<&str> = step
        .lines()
        .filter(|line| {
            let upper = line.to_uppercase();
            // Remove SI_UNIT / LENGTH_UNIT / PLANE_ANGLE_UNIT / SOLID_ANGLE_UNIT lines
            !(upper.contains("LENGTH_UNIT")
                || upper.contains("PLANE_ANGLE_UNIT")
                || upper.contains("SOLID_ANGLE_UNIT")
                || upper.contains("UNCERTAINTY_MEASURE_WITH_UNIT")
                || upper.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT"))
        })
        .collect();
    // Also strip the REPRESENTATION_CONTEXT line that references unit context.
    // Lines are left as-is — the parser should still work.
    step = lines.join("\n") + "\n";
    std::fs::write(&path, step).unwrap();
}
