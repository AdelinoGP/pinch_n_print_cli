use slicer_sdk::coords::{mm_to_units, units_to_mm, SCALING_FACTOR};
use slicer_sdk::host;
use slicer_sdk::prelude::*;

#[test]
fn prelude_reexports_are_available() {
    let p = Point2::from_mm(1.0, 2.0);
    let (x_mm, y_mm) = p.to_mm();
    assert!((x_mm - 1.0).abs() < 1e-6);
    assert!((y_mm - 2.0).abs() < 1e-6);

    let role = ExtrusionRole::SparseInfill;
    assert!(matches!(role, ExtrusionRole::SparseInfill));

    let _: u64 = host::now_us();
    assert_eq!(SCALING_FACTOR, 10_000);
}

#[test]
fn coords_round_trip() {
    let values = [0.0_f32, 0.1, 0.2, 0.4, 1.25, 12.3456];

    for value in values {
        let units = mm_to_units(value);
        let mm = units_to_mm(units);
        assert!((mm - value).abs() <= 0.0001);
    }
}

#[test]
fn host_wrappers_have_real_behavior() {
    let object_id = String::from("obj-1");
    let degenerate = vec![ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    }];

    // Mesh queries with no source installed: ray/normal return None
    // (documented "no surface" signal); object_bounds returns an
    // explicit error rather than a meaningless zero box.
    host::test_support::clear_mesh_source();
    assert_eq!(host::raycast_z_down(&object_id, 10.0, 20.0, 5.0), None);
    assert_eq!(host::surface_normal_at(&object_id, 10.0, 20.0, 5.0), None);
    assert!(host::object_bounds(&object_id).is_err());

    // Clipping/offsetting degenerate empty input still yields empty
    // output, but via real Clipper2 — no longer a silent no-op.
    assert!(host::clip_polygons(&degenerate, &degenerate, host::ClipOperation::Union).is_empty());
    assert!(host::offset_polygons(&degenerate, 0.2, host::OffsetJoinType::Miter).is_empty());

    let simplified = host::simplify_polygon(&degenerate[0].contour, 0.05);
    assert_eq!(simplified.points.len(), 0);
}
