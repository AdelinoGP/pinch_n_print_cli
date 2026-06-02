#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::mesh_analysis::{
    execute_mesh_analysis, execute_mesh_analysis_with, MeshAnalysisConfig, MeshAnalysisError,
};
use slicer_ir::{
    BoundingBox3, FacetClass, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
    Transform3d,
};

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
        min: p3(0.0, 0.0, 0.0),
        max: p3(200.0, 200.0, 200.0),
    }
}

fn triangle_mesh(id: &str) -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: id.to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0)],
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

fn cube_like_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    p3(0.0, 0.0, 1.0),
                    p3(1.0, 0.0, 1.0),
                    p3(0.0, 1.0, 1.0),
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
fn classifies_known_facets_and_emits_overhang_region() {
    let mesh = MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "probe".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    p3(0.0, 0.0, 1.0),
                    p3(1.0, 0.0, 1.0),
                    p3(0.0, 1.0, 1.0),
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 1.0, 0.0),
                    p3(1.0, 0.0, 0.0),
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 1.0, 0.0),
                    p3(1.0, 0.0, -0.75),
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 0.0, 1.0),
                    p3(0.0, 1.0, 0.0),
                ],
                indices: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
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
    };

    let ir = execute_mesh_analysis(&mesh).expect("analysis should succeed");
    let obj = ir.per_object.get("probe").expect("probe object present");

    assert_eq!(obj.facet_classes.len(), 4);
    assert!(matches!(obj.facet_classes[0], FacetClass::TopSurface));
    assert!(matches!(obj.facet_classes[1], FacetClass::BottomSurface));
    assert!(matches!(obj.facet_classes[2], FacetClass::Overhang { .. }));
    assert!(matches!(obj.facet_classes[3], FacetClass::Normal));

    assert_eq!(obj.surface_groups.len(), 1);
    assert!(obj.surface_groups[0].printable);
    assert!(obj.surface_groups[0].area_mm2 > 0.0);

    assert_eq!(obj.overhang_regions.len(), 1);
    assert_eq!(obj.overhang_regions[0].facet_indices, vec![2]);
    assert!(obj.overhang_regions[0].needs_support);
}

#[test]
fn rejects_index_buffer_not_multiple_of_three() {
    let mut mesh = triangle_mesh("bad");
    mesh.objects[0].mesh.indices.push(0);

    let err = execute_mesh_analysis(&mesh).expect_err("must fail");
    assert!(matches!(
        err,
        MeshAnalysisError::IndicesNotMultipleOfThree {
            ref object_id,
            count: 4
        } if object_id == "bad"
    ));
}

#[test]
fn rejects_out_of_range_vertex_index() {
    let mut mesh = triangle_mesh("oor");
    mesh.objects[0].mesh.indices[2] = 99;

    let err = execute_mesh_analysis(&mesh).expect_err("must fail");
    assert!(matches!(
        err,
        MeshAnalysisError::InvalidVertexIndex {
            ref object_id,
            index: 99,
            vertex_count: 3
        } if object_id == "oor"
    ));
}

#[test]
fn is_deterministic_for_same_input() {
    let mesh = cube_like_mesh();

    let a = execute_mesh_analysis(&mesh).unwrap();
    let b = execute_mesh_analysis(&mesh).unwrap();
    let c = execute_mesh_analysis(&mesh).unwrap();

    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn default_config_matches_explicit_default() {
    let mesh = cube_like_mesh();

    let a = execute_mesh_analysis(&mesh).unwrap();
    let b = execute_mesh_analysis_with(&mesh, MeshAnalysisConfig::default()).unwrap();

    assert_eq!(a, b);
}
