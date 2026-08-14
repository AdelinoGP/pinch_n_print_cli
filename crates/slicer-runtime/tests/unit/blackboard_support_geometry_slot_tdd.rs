//! TDD test: `Blackboard` `commit_support_geometry` / `support_geometry` round-trip.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
    SupportGeometryIR, SupportPlanEntry, SupportPlanIR, Transform3d,
};
use slicer_runtime::{Blackboard, BlackboardError};

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
        objects: vec![
            // exhaustive: ObjectMesh fixture intentionally supplies explicit mesh data
            ObjectMesh {
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
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: vec![],
                paint_data: None,
                world_z_extent: None,
            },
        ],
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

fn support_entry(family_id: &str, layer: i32) -> SupportPlanEntry {
    SupportPlanEntry {
        global_layer_index: layer,
        object_id: "object".into(),
        region_id: 0,
        family_id: family_id.into(),
        demand_ids: vec![format!("demand-{family_id}")],
        body_ids: vec![format!("body-{family_id}")],
        anchor_layer_index: layer as u32,
        anchor_z: i64::from(layer),
        roles: vec![],
        skeleton: None,
        capabilities: vec!["structural".into()],
        provenance: vec![family_id.into()],
        decline_reason: None,
    }
}

#[test]
fn support_plan_family_commits_merge_entries_and_metadata() {
    let mesh = minimal_mesh();
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    let first = Arc::new(SupportPlanIR {
        schema_version: semver(2, 0, 0),
        entries: vec![support_entry("tree", 0)],
        raft_plan: Some(slicer_ir::RaftPlan {
            raft_layers: 2,
            raft_first_layer_density: 0.4,
            base_raft_layers: 1,
            interface_raft_layers: 1,
        }),
    });
    let second = Arc::new(SupportPlanIR {
        schema_version: semver(2, 0, 0),
        entries: vec![support_entry("traditional", 1)],
        raft_plan: None,
    });

    blackboard.commit_support_plan(first).unwrap();
    blackboard.commit_support_plan(second).unwrap();

    let merged = blackboard.support_plan().unwrap();
    assert_eq!(merged.schema_version, semver(2, 0, 0));
    assert_eq!(merged.entries.len(), 2);
    assert_eq!(
        merged
            .entries
            .iter()
            .map(|entry| entry.family_id.as_str())
            .collect::<Vec<_>>(),
        vec!["tree", "traditional"]
    );
    assert_eq!(merged.raft_plan.as_ref().unwrap().raft_layers, 2);
}

#[test]
fn support_plan_merge_rejects_duplicate_region_identity() {
    let mesh = minimal_mesh();
    let mut blackboard = Blackboard::new(mesh, 0);
    let first = Arc::new(SupportPlanIR {
        schema_version: semver(2, 0, 0),
        entries: vec![support_entry("tree", 0)],
        raft_plan: None,
    });
    let duplicate = Arc::new(SupportPlanIR {
        schema_version: semver(2, 0, 0),
        entries: vec![support_entry("traditional", 0)],
        raft_plan: None,
    });

    blackboard.commit_support_plan(first).unwrap();
    let error = blackboard
        .commit_support_plan(duplicate)
        .expect_err("duplicate support region identity must be rejected");
    assert_eq!(
        error,
        BlackboardError::DuplicateSupportPlanEntry {
            global_layer_index: 0,
            object_id: "object".into(),
            region_id: 0,
        }
    );
    assert_eq!(blackboard.support_plan().unwrap().entries.len(), 1);
}
