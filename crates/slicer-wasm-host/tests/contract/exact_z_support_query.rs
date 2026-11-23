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

#[test]
pub fn exact_z_memoizes_z_independent_terms_per_object() {
    let mut mesh = cube_mesh();
    let mut neighbouring = mesh.objects[0].clone();
    neighbouring.id = "object-b".into();
    // Translate the second cube +20 mm in X so its plate differs from object-a's.
    neighbouring.transform.matrix[12] = 20.0;
    mesh.objects.push(neighbouring);
    let service = ExactZQueryService::new(Arc::new(mesh));

    let low = service.query("object-a", 7, &[], 4.321).expect("query");
    assert_eq!(
        service.geometry_build_count(),
        1,
        "first query builds the object's z-independent terms"
    );
    let above = service.query("object-a", 7, &[], 11.0).expect("second Z");
    assert_eq!(
        service.geometry_build_count(),
        1,
        "a new requested Z must not rebuild the object's terms"
    );

    // The z-independent terms are identical at both Z values; only occupancy
    // (and its blocker copy) follows the requested Z.
    assert_eq!(low.termination_surfaces, above.termination_surfaces);
    assert_eq!(low.baseline_envelope, above.baseline_envelope);
    assert_ne!(
        low.occupancy, above.occupancy,
        "requested-Z occupancy still tracks the requested Z"
    );

    let other = service
        .query("object-b", 7, &[], 4.321)
        .expect("other object");
    assert_eq!(
        service.geometry_build_count(),
        2,
        "each object builds its own terms"
    );
    assert_ne!(
        low.baseline_envelope, other.baseline_envelope,
        "one object's plate is never shared with another object"
    );

    let third = service.query("object-a", 7, &[], 8.0).expect("third Z");
    assert_eq!(service.geometry_build_count(), 2);
    assert_eq!(third.termination_surfaces, low.termination_surfaces);
}

/// The batched prefetch must serve exactly what the single-Z path computes.
///
/// `slice_mesh_ex` builds every requested plane independently, so batching the
/// planes is a work-removal, not a geometry change — this pins that claim on the
/// real API, including the two degenerate planes (exactly on the mesh bottom
/// and top) and one outside the Z extent.
#[test]
pub fn prefetched_cross_sections_match_the_single_z_queries() {
    let zs = [4.321_f32, 8.0, 0.0, 10.0, 11.0];
    let single = ExactZQueryService::new(Arc::new(cube_mesh()));
    let batched = ExactZQueryService::new(Arc::new(cube_mesh()));

    let expected: Vec<_> = zs
        .iter()
        .map(|z| {
            single
                .query("object-a", 7, &[], *z)
                .expect("single-Z query")
                .occupancy
                .clone()
        })
        .collect();

    batched.prefetch_cross_sections("object-a", &zs);
    assert_eq!(
        batched.geometry_build_count(),
        1,
        "the prefetch builds the object's z-independent terms exactly once"
    );

    for (z, want) in zs.iter().zip(&expected) {
        let got = batched
            .query("object-a", 7, &[], *z)
            .expect("batched query");
        assert_eq!(
            &got.occupancy, want,
            "prefetched cross-section at Z={z} must equal the single-Z result"
        );
    }
}

/// A prefetch that cannot resolve its object is inert: the per-query path keeps
/// its own error contract, so entry validation still degrades per entry.
#[test]
pub fn prefetch_of_an_unknown_object_leaves_the_query_contract_untouched() {
    let service = ExactZQueryService::new(Arc::new(cube_mesh()));
    service.prefetch_cross_sections("object-missing", &[1.0, 2.0]);
    assert_eq!(
        service.geometry_build_count(),
        0,
        "an unknown object must not publish geometry terms"
    );
    assert!(
        service.query("object-missing", 7, &[], 1.0).is_err(),
        "the query path still reports the unknown object"
    );
}
