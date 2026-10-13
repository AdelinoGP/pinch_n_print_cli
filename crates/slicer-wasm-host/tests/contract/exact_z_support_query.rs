//! Contract coverage for normalized, exact-Z support analysis queries.

use std::sync::Arc;

use slicer_ir::{
    IndexedTriangleSet, MeshIR, ObjectMesh, PaintValue, Point3, SliceIR, SlicedRegion, Transform3d,
};
use slicer_wasm_host::exact_z_query::ExactZQueryService;

fn cube_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "object-a".into(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 10.0,
                    },
                ],
                indices: vec![
                    0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6,
                    7, 2, 7, 3, 3, 7, 4, 3, 4, 0,
                ],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
pub fn exact_z_support_query() {
    let service = ExactZQueryService::new(Arc::new(cube_mesh()));
    // Authoritative committed SOURCE row: the query identity (object id, region
    // id, ordered variant chain, physical Z) is derived from the committed
    // `SliceIR` region instead of fabricated literals.
    let source = SliceIR {
        global_layer_index: 7,
        z: 4.321,
        regions: vec![SlicedRegion {
            object_id: "object-a".to_string(),
            region_id: 7,
            variant_chain: vec![("wall".to_string(), PaintValue::ToolIndex(1))],
            ..Default::default()
        }],
        ..Default::default()
    };
    let region = &source.regions[0];

    let first = service
        .query(
            &region.object_id,
            region.region_id,
            &region.variant_chain,
            source.z,
        )
        .expect("query");

    assert_eq!(first.z_units, 43_210);
    assert!(!first.occupancy.is_empty(), "occupancy at intermediate Z");
    assert!(!first.blockers.is_empty(), "occupied geometry is a blocker");
    assert!(!first.termination_surfaces.is_empty(), "plate termination");
    assert!(!first.baseline_envelope.is_empty(), "baseline envelope");

    let second = service
        .query(
            &region.object_id,
            region.region_id,
            &region.variant_chain,
            source.z,
        )
        .expect("cached query");
    assert!(
        Arc::ptr_eq(&first, &second),
        "identical inputs use immutable cache"
    );
}
