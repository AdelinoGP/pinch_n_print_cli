#![allow(missing_docs)]

//! TDD tests for packet 106 (PrePass overhang foundation), Step 2 / O-T010,
//! O-T011: `OverhangRegion.xy_footprint` population at `MeshAnalysis`.
//!
//! Proves that:
//! - a mesh with overhang facets produces an `OverhangRegion` whose
//!   `xy_footprint` is a non-empty facet-cluster XY projection,
//! - a flat-top cube (no overhang facets) produces no overhang regions and
//!   does not panic.
//!
//! Reference: `.ralph/specs/106_overhang-pipeline-prepass-foundation/` Step 2.

use std::collections::HashMap;

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};
use slicer_runtime::execute_mesh_analysis;

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
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
    }
}

/// Mesh with a single clearly-down-facing overhang triangle (same normal
/// construction as `mesh_analysis_tdd::mesh_analysis_classifies_known_facets_and_emits_overhang_region`):
/// v0=(0,0,0), v1=(0,1,0), v2=(1,0,-0.75) -> normal ~= (-0.6, 0, -0.8), a
/// strong overhang well past the default threshold.
fn overhang_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "overhang-probe".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![p3(0.0, 0.0, 0.0), p3(0.0, 1.0, 0.0), p3(1.0, 0.0, -0.75)],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}

/// Flat-top cube shell: two opposed horizontal triangles (top +Z, bottom
/// -Z). Neither faces down past the overhang threshold, so no overhang
/// facets/regions should be produced.
fn flat_top_cube_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "flat-top-cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    // top (normal +Z)
                    p3(0.0, 0.0, 1.0),
                    p3(1.0, 0.0, 1.0),
                    p3(0.0, 1.0, 1.0),
                    // bottom (normal -Z)
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 1.0, 0.0),
                    p3(1.0, 0.0, 0.0),
                ],
                indices: vec![0, 1, 2, 3, 4, 5],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}

#[test]
fn overhang_facets_produce_non_empty_xy_footprint() {
    let mesh = overhang_mesh();
    let ir = execute_mesh_analysis(&mesh).expect("analysis should succeed");
    let obj = ir
        .per_object
        .get("overhang-probe")
        .expect("overhang-probe object present");

    assert_eq!(
        obj.overhang_regions.len(),
        1,
        "expected exactly one overhang region for the down-facing facet"
    );
    let region = &obj.overhang_regions[0];
    assert!(
        !region.xy_footprint.is_empty(),
        "xy_footprint must be populated (non-empty) for a real overhang cluster"
    );
}

#[test]
fn flat_top_cube_has_empty_xy_footprint_and_does_not_panic() {
    let mesh = flat_top_cube_mesh();
    let ir = execute_mesh_analysis(&mesh).expect("analysis should succeed on flat-top cube");
    let obj = ir
        .per_object
        .get("flat-top-cube")
        .expect("flat-top-cube object present");

    assert!(
        obj.overhang_regions.is_empty(),
        "flat-top cube has no down-facing facets, so no overhang regions should be produced"
    );

    // Defensive: if any region were ever produced, its footprint must not
    // panic to compute and must be empty for this geometry.
    for region in &obj.overhang_regions {
        assert!(region.xy_footprint.is_empty());
    }
}
