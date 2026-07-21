//! Smoke tests for the slicer-sdk crate.

use slicer_ir::ConfigValue;
use slicer_sdk::coords::{mm_to_units, units_to_mm, SCALING_FACTOR};
use slicer_sdk::host;
use slicer_sdk::prelude::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth, Polygon,
};
use slicer_sdk::test_support::assert_paths::assert_paths_planar;
use slicer_sdk::test_support::capture::InfillOutputCapture;
use slicer_sdk::test_support::fixtures::{
    square_polygon, ConfigViewBuilder, SliceRegionViewBuilder,
};
use slicer_sdk::test_support::mock_host::MockHost;

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

// ---------------------------------------------------------------------------
// Absorbed from former `slicer-test` smoke tests.
// ---------------------------------------------------------------------------

fn sample_path(z: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

#[test]
fn mock_host_tracks_calls_and_logs() {
    // Install a log capture sink so the host wrapper's output is
    // observable through `MockHost::log_contains`.
    slicer_sdk::host::test_support::install_log_capture();

    let mut host = MockHost::new();
    host.record_call("clip_polygons");
    host.record_call("clip_polygons");
    host.log_warn("density near limit");

    assert_eq!(host.call_count("clip_polygons"), 2);
    host.assert_call_count("clip_polygons", 2);
    // NOTE: `log_contains` drains the capture buffer.
    assert!(MockHost::log_contains("density"));
}

#[test]
fn config_builder_creates_key_value_view() {
    let config = ConfigViewBuilder::new()
        .float("density", 0.2)
        .string("pattern", "grid")
        .int("multiline-count", 2)
        .build();

    assert!(matches!(
        config.get("density"),
        Some(ConfigValue::Float(v)) if (*v - 0.2).abs() < f64::EPSILON
    ));
    assert!(matches!(
        config.get("pattern"),
        Some(ConfigValue::String(v)) if v == "grid"
    ));
    assert!(matches!(
        config.get("multiline-count"),
        Some(ConfigValue::Int(2))
    ));
}

#[test]
fn slice_region_fixture_builder_uses_scaled_coordinates() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(7)
        .effective_layer_height(0.24)
        .add_polygon(square_polygon(0.0, 0.0, 2.0))
        .build();

    assert_eq!(region.object_id(), "obj-1");
    assert_eq!(*region.region_id(), 7);
    assert!((region.effective_layer_height() - 0.24).abs() < f32::EPSILON);
    assert_eq!(region.polygons().len(), 1);
    assert_eq!(region.polygons()[0].contour.points[0].x, mm_to_units(-1.0));
}

#[test]
fn infill_capture_collects_paths() {
    let mut capture = InfillOutputCapture::new();
    capture.push_sparse_path(sample_path(0.2));
    capture.push_sparse_path(sample_path(0.2));

    assert_eq!(capture.sparse_paths().len(), 2);
}

#[test]
fn planar_assertion_accepts_consistent_z() {
    let paths = vec![sample_path(0.2), sample_path(0.2)];
    assert_paths_planar(&paths, 0.2, 1e-5);
}
