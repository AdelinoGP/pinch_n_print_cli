// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/[Various]
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! STEP/STP to MeshIR import pipeline via truck.

use std::path::{Path, PathBuf};

use slicer_ir::{IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, Transform3d};
use truck_meshalgo::prelude::*;
use truck_stepio::r#in::*;

use crate::repair::{self, RepairStats};

/// Tessellation tolerance in the STEP file's native coordinate space.
/// This is refined per-shell using the bounding box diagonal.
const INITIAL_TESSELLATION_TOL: f64 = 0.01;

/// Relative tessellation tolerance (fraction of bounding box diagonal).
const RELATIVE_TOL: f64 = 0.001;

/// Result of importing a STEP file.
#[derive(Debug, Clone, Default)]
pub struct StepImportResult {
    /// One mesh per solid found in the STEP file.
    pub meshes: Vec<NamedMesh>,
    /// The length unit declared in the STEP file header.
    pub source_unit: StepLengthUnit,
    /// Non-fatal warnings encountered during import.
    pub warnings: Vec<StepWarning>,
}

/// A mesh with an optional STEP entity label.
#[derive(Debug, Clone, Default)]
pub struct NamedMesh {
    /// STEP entity label if present.
    pub name: Option<String>,
    /// The triangulated mesh.
    pub mesh: MeshIR,
}

/// Length unit declared in a STEP file header.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum StepLengthUnit {
    /// Millimetres (most common).
    Millimetre,
    /// Metres.
    Metre,
    /// Inches.
    Inch,
    /// Micrometres.
    Micrometre,
    /// No unit declared; defaults to millimetres with a warning.
    #[default]
    Unknown,
}

/// Non-fatal warnings from the STEP import process.
#[derive(Debug, Clone)]
pub enum StepWarning {
    /// The STEP file uses an unsupported schema (e.g. AP242).
    UnsupportedSchema {
        /// The schema identifier string.
        schema: String,
    },
    /// No length unit was declared in the STEP file header.
    UnknownUnit,
    /// Automatic repair was applied to a component.
    RepairApplied {
        /// Index of the component in the output meshes vector.
        component_index: usize,
        /// Repair statistics for this component.
        stats: RepairStats,
    },
    /// The STEP file contained multiple disconnected solids.
    MultipleComponents {
        /// Number of solids found.
        count: usize,
    },
}

/// Errors that can occur during STEP import.
#[derive(Debug, thiserror::Error)]
pub enum StepImportError {
    /// The input file was not found.
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
    /// The STEP file could not be parsed.
    #[error("parse error: {0}")]
    ParseError(String),
    /// The STEP file contains no recognisable geometry.
    #[error("no geometry found in STEP file")]
    NoGeometry,
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Options for [`import_step_with_options`].
///
/// `Default::default()` (`skip_repair = false`) matches the behavior of the
/// no-options [`import_step`] entry point.
#[derive(Debug, Clone, Default)]
pub struct StepImportOptions {
    /// When `true`, skip the automatic Phase 1+2 repair pass applied to each
    /// tessellated component. Exposed so the CLI's `--no-repair` flag can
    /// disable it. Not recommended in normal use — truck's tessellation can
    /// produce non-manifold output on degenerate B-Rep input.
    pub skip_repair: bool,
}

/// Import a STEP file. Returns one [`MeshIR`] per solid found in the file.
///
/// Equivalent to [`import_step_with_options`] with [`StepImportOptions`]
/// default values. Repair (Phase 1 + Phase 2) is applied automatically to
/// each component.
pub fn import_step(path: &Path) -> Result<StepImportResult, StepImportError> {
    import_step_with_options(path, StepImportOptions::default())
}

/// Import a STEP file with custom options.
///
/// See [`StepImportOptions`] for the available knobs. Returns one [`MeshIR`]
/// per solid found in the file.
pub fn import_step_with_options(
    path: &Path,
    opts: StepImportOptions,
) -> Result<StepImportResult, StepImportError> {
    // 1. Read file.
    if !path.exists() {
        return Err(StepImportError::FileNotFound(path.to_path_buf()));
    }
    let step_string = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            return Err(StepImportError::ParseError(
                "file is not valid UTF-8 text".to_string(),
            ));
        }
        Err(e) => return Err(StepImportError::IoError(e)),
    };

    // 2. Parse STEP into Table.
    let table = Table::from_step(&step_string)
        .ok_or_else(|| StepImportError::ParseError("failed to parse STEP data".to_string()))?;

    // 3. Detect length unit from the raw STEP text.
    let mut warnings: Vec<StepWarning> = Vec::new();
    let source_unit = detect_unit(&step_string);
    if source_unit == StepLengthUnit::Unknown {
        warnings.push(StepWarning::UnknownUnit);
    }

    // Unit conversion factor: STEP native units → f32 mm for Point3.
    let unit_to_mm = match &source_unit {
        StepLengthUnit::Millimetre => 1.0_f64,
        StepLengthUnit::Metre => 1000.0,
        StepLengthUnit::Inch => 25.4,
        StepLengthUnit::Micrometre => 0.001,
        StepLengthUnit::Unknown => 1.0, // default to mm
    };

    // 4. Tessellate each shell.
    let shell_count = table.shell.len();
    if shell_count == 0 {
        return Err(StepImportError::NoGeometry);
    }

    if shell_count > 1 {
        warnings.push(StepWarning::MultipleComponents { count: shell_count });
    }

    let mut meshes: Vec<NamedMesh> = Vec::with_capacity(shell_count);

    for (_idx, step_shell) in table.shell.iter() {
        let compressed = match table.to_compressed_shell(step_shell) {
            Ok(c) => c,
            Err(e) => {
                return Err(StepImportError::ParseError(format!(
                    "failed to convert shell: {e:?}"
                )));
            }
        };

        // Two-pass tessellation: coarse for bounding box, then refined.
        let coarse_poly = compressed
            .robust_triangulation(INITIAL_TESSELLATION_TOL)
            .to_polygon();
        let bdd = coarse_poly.bounding_box();
        let diag = bdd.diameter();
        let tol = (diag * RELATIVE_TOL).max(1e-9);

        let tessellated = compressed.robust_triangulation(tol);
        let mut poly = tessellated.to_polygon();
        poly.put_together_same_attrs(TOLERANCE * 50.0)
            .remove_degenerate_faces()
            .remove_unused_attrs();

        // 5. Convert PolygonMesh to our IndexedTriangleSet.
        let mesh_ir = polygon_to_mesh_ir(&poly, unit_to_mm);

        meshes.push(NamedMesh {
            name: None,
            mesh: mesh_ir,
        });
    }

    // 6. Apply repair (Phase 1 + Phase 2) to each component, unless the
    //    caller opted out via `StepImportOptions::skip_repair`.
    if !opts.skip_repair {
        for (comp_idx, named) in meshes.iter_mut().enumerate() {
            match repair::repair(named.mesh.clone()) {
                Ok(repair_result) => {
                    let did_repair = repair_result.stats.degenerate_removed > 0
                        || repair_result.stats.faces_reoriented > 0
                        || repair_result.stats.open_edges_closed > 0;
                    if did_repair {
                        warnings.push(StepWarning::RepairApplied {
                            component_index: comp_idx,
                            stats: repair_result.stats,
                        });
                    }
                    named.mesh = repair_result.mesh;
                }
                Err(_) => {
                    // Repair failed — keep the original mesh, no warning needed.
                }
            }
        }
    }

    Ok(StepImportResult {
        meshes,
        source_unit,
        warnings,
    })
}

/// Merge all meshes in a [`StepImportResult`] into a single mesh.
///
/// Vertices are concatenated and indices are offset accordingly. The resulting
/// mesh name is `None`. Warnings and source unit are preserved.
pub fn merge_step_meshes(mut result: StepImportResult) -> StepImportResult {
    if result.meshes.len() <= 1 {
        return result;
    }

    let mut all_objects = Vec::new();
    for named in result.meshes.drain(..) {
        all_objects.extend(named.mesh.objects);
    }

    let merged_mesh = MeshIR {
        objects: all_objects,
        ..Default::default()
    };

    result.meshes.push(NamedMesh {
        name: None,
        mesh: merged_mesh,
    });
    result
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Detect the length unit from raw STEP text by scanning for SI_UNIT / LENGTH_UNIT entities.
fn detect_unit(step_text: &str) -> StepLengthUnit {
    // Look for SI_UNIT patterns with LENGTH_UNIT.
    // Patterns: SI_UNIT(.MILLI.,.METRE.), SI_UNIT($,.METRE.), etc.
    let upper = step_text.to_uppercase();

    // Check for LENGTH_UNIT presence — if absent, no unit declaration.
    if !upper.contains("LENGTH_UNIT") {
        return StepLengthUnit::Unknown;
    }

    // Check for CONVERSION_BASED_UNIT with INCH.
    if upper.contains("CONVERSION_BASED_UNIT") && upper.contains("INCH") {
        return StepLengthUnit::Inch;
    }

    // Check SI_UNIT prefix for LENGTH_UNIT.
    // The pattern is typically: ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(<prefix>,.<base>.) )
    if upper.contains("SI_UNIT") {
        if upper.contains(".MILLI.") {
            return StepLengthUnit::Millimetre;
        }
        if upper.contains(".MICRO.") {
            return StepLengthUnit::Micrometre;
        }
        // SI_UNIT($,.METRE.) — no prefix means base unit (metre).
        if upper.contains(".METRE.") {
            return StepLengthUnit::Metre;
        }
    }

    StepLengthUnit::Unknown
}

/// Convert a truck `PolygonMesh` to our `MeshIR`.
fn polygon_to_mesh_ir(poly: &PolygonMesh, unit_to_mm: f64) -> MeshIR {
    let positions = poly.positions();
    let vertices: Vec<Point3> = positions
        .iter()
        .map(|p| {
            // truck Point3 is f64 in STEP native units → convert to f32 mm.
            Point3 {
                x: (p.x * unit_to_mm) as f32,
                y: (p.y * unit_to_mm) as f32,
                z: (p.z * unit_to_mm) as f32,
            }
        })
        .collect();

    // Collect all faces as triangles (auto-triangulates quads and n-gons).
    let mut indices: Vec<u32> = Vec::new();
    for tri in poly.faces().triangle_iter() {
        indices.push(tri[0].pos as u32);
        indices.push(tri[1].pos as u32);
        indices.push(tri[2].pos as u32);
    }

    let its = IndexedTriangleSet { vertices, indices };

    // Compute world-space Z extent from mesh vertices (transform is identity here).
    let world_z_extent = {
        let mut z_min = f32::INFINITY;
        let mut z_max = f32::NEG_INFINITY;
        for v in &its.vertices {
            if v.z < z_min {
                z_min = v.z;
            }
            if v.z > z_max {
                z_max = v.z;
            }
        }
        if z_min.is_finite() && z_max.is_finite() && z_max > z_min {
            Some((z_min, z_max))
        } else {
            None
        }
    };

    let object = ObjectMesh {
        id: uuid_v4(),
        mesh: its,
        transform: Transform3d {
            matrix: IDENTITY_MATRIX,
        },
        config: ObjectConfig {
            data: std::collections::HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent,
    };

    MeshIR {
        objects: vec![object],
        ..Default::default()
    }
}

/// Generate a simple UUID-like string for object IDs.
fn uuid_v4() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("step-import-{n:016x}")
}

/// Identity 4×4 matrix in column-major order.
const IDENTITY_MATRIX: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0, //
];
