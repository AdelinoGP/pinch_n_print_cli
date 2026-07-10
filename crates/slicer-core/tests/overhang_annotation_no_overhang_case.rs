#![allow(missing_docs)]
//! TDD (AC-N1): a straight (non-overhanging) cube produces an entirely
//! empty result map — every key absent, no panic.

use slicer_core::algos::overhang_annotation::annotate_overhangs;
use slicer_core::slice_mesh_ex;
use slicer_ir::{ExPolygon, IndexedTriangleSet, Point3};

/// Slice `mesh` at each Z and pair each footprint with its position index,
/// producing the `annotate_overhangs` input (which now consumes pre-computed
/// per-layer cross-sections instead of a mesh).
fn footprints(mesh: &IndexedTriangleSet, layer_zs: &[f32]) -> Vec<(u32, Vec<ExPolygon>)> {
    slice_mesh_ex(mesh, layer_zs)
        .into_iter()
        .enumerate()
        .map(|(i, poly)| (i as u32, poly))
        .collect()
}

/// Straight 10x10x10mm cube: identical cross-section at every Z, so no layer
/// is ever overhanging relative to its predecessor.
fn straight_cube_mesh() -> IndexedTriangleSet {
    let p3 = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        p3(0.0, 0.0, 0.0),
        p3(10.0, 0.0, 0.0),
        p3(10.0, 10.0, 0.0),
        p3(0.0, 10.0, 0.0),
        p3(0.0, 0.0, 10.0),
        p3(10.0, 0.0, 10.0),
        p3(10.0, 10.0, 10.0),
        p3(0.0, 10.0, 10.0),
    ];
    #[rustfmt::skip]
    let indices = vec![
        0, 1, 2,  0, 2, 3,
        4, 5, 6,  4, 6, 7,
        0, 1, 5,  0, 5, 4,
        1, 2, 6,  1, 6, 5,
        2, 3, 7,  2, 7, 6,
        3, 0, 4,  3, 4, 7,
    ];
    IndexedTriangleSet { vertices, indices }
}

#[test]
fn straight_cube_has_no_overhang_at_any_layer() {
    let mesh = straight_cube_mesh();
    let layer_zs = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32, 5.0_f32];

    let result = annotate_overhangs(&footprints(&mesh, &layer_zs), 0.4);

    assert!(
        result.is_empty(),
        "a straight (non-overhanging) cube must yield an empty map (every key absent), got {result:?}"
    );
}

#[test]
fn empty_layer_list_does_not_panic() {
    let mesh = straight_cube_mesh();
    let result = annotate_overhangs(&footprints(&mesh, &[]), 0.4);
    assert!(result.is_empty());
}

#[test]
fn single_layer_has_no_previous_and_does_not_panic() {
    let mesh = straight_cube_mesh();
    let result = annotate_overhangs(&footprints(&mesh, &[5.0]), 0.4);
    assert!(result.is_empty());
}
