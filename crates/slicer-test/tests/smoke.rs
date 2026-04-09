//! Smoke tests for `slicer-test` scaffold APIs.

use slicer_ir::{mm_to_units, ConfigValue, ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
use slicer_test::assert_paths::assert_paths_planar;
use slicer_test::capture::InfillOutputCapture;
use slicer_test::fixtures::{square_polygon, ConfigViewBuilder, SliceRegionViewBuilder};
use slicer_test::mock_host::LogLevel;
use slicer_test::MockHost;

fn sample_path(z: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
            },
            Point3WithWidth {
                x: 10.0,
                y: 0.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
            },
        ],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

#[test]
fn mock_host_tracks_calls_and_logs() {
    let mut host = MockHost::new();
    host.record_call("clip_polygons");
    host.record_call("clip_polygons");
    host.enable_logging();
    host.log_warn("density near limit");

    assert_eq!(host.call_count("clip_polygons"), 2);
    host.assert_call_count("clip_polygons", 2);
    assert!(host.log_contains(LogLevel::Warn, "density"));
}

#[test]
fn config_builder_creates_key_value_view() {
    let config = ConfigViewBuilder::new()
        .float("density", 0.2)
        .string("pattern", "grid")
        .int("multiline-count", 2)
        .build();

    assert!(matches!(
        config.fields.get("density"),
        Some(ConfigValue::Float(v)) if (*v - 0.2).abs() < f64::EPSILON
    ));
    assert!(matches!(
        config.fields.get("pattern"),
        Some(ConfigValue::String(v)) if v == "grid"
    ));
    assert!(matches!(
        config.fields.get("multiline-count"),
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
    capture.push_path(sample_path(0.2));
    capture.push_path(sample_path(0.2));

    assert_eq!(capture.paths().len(), 2);
}

#[test]
fn planar_assertion_accepts_consistent_z() {
    let paths = vec![sample_path(0.2), sample_path(0.2)];
    assert_paths_planar(&paths, 0.2, 1e-5);
}
