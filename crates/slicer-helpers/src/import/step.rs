//! STEP/STP to MeshIR import pipeline via truck.

use std::path::{Path, PathBuf};

use slicer_ir::MeshIR;

use crate::repair::RepairStats;

/// Result of importing a STEP file.
#[derive(Debug, Clone)]
pub struct StepImportResult {
    /// One mesh per solid found in the STEP file.
    pub meshes: Vec<NamedMesh>,
    /// The length unit declared in the STEP file header.
    pub source_unit: StepLengthUnit,
    /// Non-fatal warnings encountered during import.
    pub warnings: Vec<StepWarning>,
}

/// A mesh with an optional STEP entity label.
#[derive(Debug, Clone)]
pub struct NamedMesh {
    /// STEP entity label if present.
    pub name: Option<String>,
    /// The triangulated mesh.
    pub mesh: MeshIR,
}

/// Length unit declared in a STEP file header.
#[derive(Debug, Clone, PartialEq)]
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

/// Import a STEP file. Returns one [`MeshIR`] per solid found in the file.
///
/// Repair (Phase 1 + Phase 2) is applied automatically to each component.
pub fn import_step(_path: &Path) -> Result<StepImportResult, StepImportError> {
    todo!("TASK-058: implement STEP import")
}
