//! TDD test: `Blackboard` `commit_support_geometry` / `support_geometry` round-trip.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{Blackboard, BlackboardError};
use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
    SupportGeometryIR, Transform3d,
};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn minimal_mesh() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("obj"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                // column-major identity matrix
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    })
}

/// Constructs an empty blackboard, commits `SupportGeometryIR`, then reads it
/// back and asserts the returned `Arc<SupportGeometryIR>` has the same schema
/// version as the committed value (i.e., the round-trip is lossless).
#[test]
fn support_geometry_slot_roundtrip() {
    let mesh = minimal_mesh();
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    assert!(
        blackboard.support_geometry().is_none(),
        "slot must start empty"
    );

    let ir = Arc::new(SupportGeometryIR {
        schema_version: semver(1, 0, 0),
        support_layer_height_mm: 0.2,
        support_top_z_distance_mm: 0.1,
        entries: HashMap::new(),
    });

    blackboard
        .commit_support_geometry(Arc::clone(&ir))
        .expect("first commit must succeed");

    let retrieved = blackboard
        .support_geometry()
        .expect("support_geometry() must return Some after commit");

    assert_eq!(
        retrieved.schema_version, ir.schema_version,
        "retrieved IR schema_version must match committed value"
    );
    assert!(
        retrieved.entries.is_empty(),
        "empty entries map must survive the round-trip"
    );

    // Write-once: a second commit must return an error.
    let err = blackboard
        .commit_support_geometry(Arc::clone(&ir))
        .expect_err("second commit must be rejected (write-once contract)");
    assert!(
        matches!(err, BlackboardError::DuplicatePrepassCommit { .. }),
        "second commit must be DuplicatePrepassCommit, got {err:?}"
    );
}
