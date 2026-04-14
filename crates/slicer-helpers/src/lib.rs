//! Pre-pipeline mesh processing operations for ModularSlicer.
//!
//! This crate provides mesh repair, decimation, and STEP import functionality
//! that runs before the WASM module pipeline. All operations input and output
//! values in Pinch_n_Print's internal coordinate system (1 unit = 100 nm).

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod decimate;
pub mod import;
pub mod repair;

// Re-export all public types for convenient access.
pub use decimate::{DecimateConfig, DecimateError, DecimateResult};
pub use import::step::{NamedMesh, StepImportError, StepImportResult, StepLengthUnit, StepWarning};
pub use repair::{RepairError, RepairResult, RepairStats, RepairWarning, MAX_REPAIR_CAP_VERTICES};

// Re-export public functions.
pub use decimate::decimate;
pub use import::step::{import_step, merge_step_meshes};
pub use repair::repair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_all_types_accessible() {
        // Verify RepairWarning variants can be constructed.
        let w = RepairWarning::LargeCapLoop { vertex_count: 300 };
        assert!(matches!(
            w,
            RepairWarning::LargeCapLoop { vertex_count: 300 }
        ));

        let w2 = RepairWarning::MultipleComponents { count: 3 };
        assert!(matches!(w2, RepairWarning::MultipleComponents { count: 3 }));

        // Verify RepairStats default.
        let stats = RepairStats::default();
        assert_eq!(stats.degenerate_removed, 0);
        assert_eq!(stats.faces_reoriented, 0);
        assert_eq!(stats.open_edges_closed, 0);
        assert_eq!(stats.components, 0);
        assert!(stats.warnings.is_empty());

        // Verify DecimateConfig default.
        let config = DecimateConfig::default();
        assert!(config.target_count.is_none());
        assert!(config.target_ratio.is_none());
        assert!((config.max_error - 0.01).abs() < f32::EPSILON);
        assert!(!config.aggressive);

        // Verify StepLengthUnit variants.
        assert_eq!(StepLengthUnit::Millimetre, StepLengthUnit::Millimetre);
        assert_eq!(StepLengthUnit::Metre, StepLengthUnit::Metre);
        assert_eq!(StepLengthUnit::Inch, StepLengthUnit::Inch);
        assert_eq!(StepLengthUnit::Micrometre, StepLengthUnit::Micrometre);
        assert_eq!(StepLengthUnit::Unknown, StepLengthUnit::Unknown);

        // Verify MAX_REPAIR_CAP_VERTICES constant.
        assert_eq!(MAX_REPAIR_CAP_VERTICES, 256);
    }

    #[test]
    fn smoke_error_types_are_errors() {
        // Verify error types implement std::error::Error via Display.
        let e = RepairError::EmptyMesh;
        assert_eq!(format!("{e}"), "input mesh is empty");

        let e = DecimateError::EmptyMesh;
        assert_eq!(format!("{e}"), "input mesh is empty");

        let e = DecimateError::InvalidConfig("both targets set".to_string());
        assert!(format!("{e}").contains("both targets set"));

        let e = StepImportError::FileNotFound(std::path::PathBuf::from("/tmp/missing.step"));
        assert!(format!("{e}").contains("/tmp/missing.step"));

        let e = StepImportError::ParseError("bad syntax".to_string());
        assert!(format!("{e}").contains("bad syntax"));

        let e = StepImportError::NoGeometry;
        assert_eq!(format!("{e}"), "no geometry found in STEP file");
    }

    #[test]
    fn smoke_named_mesh_construction() {
        use slicer_ir::{BoundingBox3, MeshIR, Point3, SemVer};

        let mesh = MeshIR {
            schema_version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            objects: vec![],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
        };

        let named = NamedMesh {
            name: Some("test_solid".to_string()),
            mesh: mesh.clone(),
        };
        assert_eq!(named.name.as_deref(), Some("test_solid"));

        let unnamed = NamedMesh { name: None, mesh };
        assert!(unnamed.name.is_none());
    }

    #[test]
    fn smoke_step_warning_variants() {
        let w = StepWarning::UnsupportedSchema {
            schema: "AP242".to_string(),
        };
        assert!(matches!(w, StepWarning::UnsupportedSchema { .. }));

        let w = StepWarning::UnknownUnit;
        assert!(matches!(w, StepWarning::UnknownUnit));

        let w = StepWarning::RepairApplied {
            component_index: 0,
            stats: RepairStats::default(),
        };
        assert!(matches!(w, StepWarning::RepairApplied { .. }));

        let w = StepWarning::MultipleComponents { count: 5 };
        assert!(matches!(w, StepWarning::MultipleComponents { count: 5 }));
    }
}
