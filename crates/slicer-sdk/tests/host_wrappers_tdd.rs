//! TDD coverage for SDK host-service wrappers (`slicer_sdk::host`).
//!
//! Locks in the post-placeholder contract: logging is observable,
//! geometry helpers delegate to `slicer-core`, mesh queries route through
//! an installable `MeshSource`, `object_bounds` returns an explicit
//! `HostUnavailable` error when no source is installed, and `now_us` is
//! a monotonic process-start timestamp.

#![allow(missing_docs)]

use slicer_ir::{BoundingBox3, ExPolygon, Point2, Point3, Polygon};

use slicer_sdk::host;
use slicer_sdk::host::{
    test_support, ClipOperation, HostUnavailable, LogLevel, MeshSource, OffsetJoinType,
};

// ── Logging ──────────────────────────────────────────────────────────────

#[test]
fn log_capture_records_all_levels_in_order() {
    test_support::install_log_capture();

    host::log_trace("t");
    host::log_debug("d");
    host::log_info("i");
    host::log_warn("w");
    host::log_error("e");

    let msgs = test_support::take_log_messages();
    assert_eq!(
        msgs,
        vec![
            (LogLevel::Trace, "t".to_string()),
            (LogLevel::Debug, "d".to_string()),
            (LogLevel::Info, "i".to_string()),
            (LogLevel::Warn, "w".to_string()),
            (LogLevel::Error, "e".to_string()),
        ],
        "log levels and message ordering must round-trip through the capture sink",
    );
}

#[test]
fn log_level_str_matches_host_mapping() {
    assert_eq!(LogLevel::Trace.as_str(), "trace");
    assert_eq!(LogLevel::Debug.as_str(), "debug");
    assert_eq!(LogLevel::Info.as_str(), "info");
    assert_eq!(LogLevel::Warn.as_str(), "warn");
    assert_eq!(LogLevel::Error.as_str(), "error");
}

#[test]
fn take_log_messages_uninstalls_sink() {
    test_support::install_log_capture();
    host::log_info("first");
    let _ = test_support::take_log_messages();

    // After draining, no sink is installed; this call must not panic and
    // must not be observable through the capture API.
    host::log_info("second");
    assert!(test_support::take_log_messages().is_empty());
}

// ── Mesh queries / failure mode ──────────────────────────────────────────

struct StubMesh {
    bounds: BoundingBox3,
    z_hit: Option<f32>,
    normal: Option<Point3>,
}

impl MeshSource for StubMesh {
    fn raycast_z_down(&self, _object_id: &str, _x: f32, _y: f32, _start_z: f32) -> Option<f32> {
        self.z_hit
    }
    fn surface_normal_at(&self, _object_id: &str, _x: f32, _y: f32, _z: f32) -> Option<Point3> {
        self.normal
    }
    fn object_bounds(&self, object_id: &str) -> Option<BoundingBox3> {
        if object_id == "obj-1" {
            Some(self.bounds)
        } else {
            None
        }
    }
}

// KEEP-review (core-cross): explicit HostUnavailable (not a zero box) when no MeshSource is installed.
#[test]
fn object_bounds_returns_host_unavailable_without_source() {
    test_support::clear_mesh_source();
    let err: HostUnavailable = host::object_bounds("obj-1").unwrap_err();
    assert_eq!(err.service, "object_bounds");
    assert_eq!(err.subject, "obj-1");
    let msg = err.to_string();
    assert!(msg.contains("object_bounds"));
    assert!(msg.contains("obj-1"));
}

// KEEP-review (core-cross): SDK raycast/bounds route through the MeshSource trait, distinct from core AabbTree; catches source-lookup or trait-plumbing regressions.
#[test]
fn raycast_and_normal_route_through_installed_mesh_source() {
    test_support::install_mesh_source(StubMesh {
        bounds: BoundingBox3 {
            min: Point3 {
                x: -1.0,
                y: -2.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 2.0,
                z: 5.0,
            },
        },
        z_hit: Some(3.25),
        normal: Some(Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }),
    });

    assert_eq!(host::raycast_z_down("obj-1", 0.0, 0.0, 10.0), Some(3.25));
    assert_eq!(
        host::surface_normal_at("obj-1", 0.0, 0.0, 0.0),
        Some(Point3 {
            x: 0.0,
            y: 0.0,
            z: 1.0
        })
    );
    let bb = host::object_bounds("obj-1").expect("bounds available");
    assert_eq!(bb.max.z, 5.0);

    // Unknown object → still HostUnavailable, not a zero box.
    assert!(host::object_bounds("unknown").is_err());

    test_support::clear_mesh_source();
}

// KEEP-review (core-cross): documented None signal for missing source; negative control for the raycast contract.
#[test]
fn raycast_returns_none_without_source_documented_signal() {
    test_support::clear_mesh_source();
    assert_eq!(host::raycast_z_down("obj-1", 0.0, 0.0, 10.0), None);
    assert_eq!(host::surface_normal_at("obj-1", 0.0, 0.0, 0.0), None);
}

// ── Geometry parity with slicer-core ─────────────────────────────────────

fn square(min: i64, max: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min, y: min },
                Point2 { x: max, y: min },
                Point2 { x: max, y: max },
                Point2 { x: min, y: max },
            ],
        },
        holes: vec![],
    }
}

#[test]
fn clip_polygons_union_produces_nonempty_result_for_real_input() {
    let a = vec![square(0, 100_000)];
    let b = vec![square(50_000, 150_000)];
    let result = host::clip_polygons(&a, &b, ClipOperation::Union);
    assert!(
        !result.is_empty(),
        "real Clipper2 union of overlapping squares must produce a non-empty merged polygon",
    );
}

// KEEP-review (core-cross): thin-delegate cover — the wrapper must keep routing through slicer_core::polygon_ops::offset and return non-empty output for both delta signs (±1.0 mm Miter on the 10 mm square); smoke-level delegate-path check, not a geometric oracle (the miter-limit test carries the geometric oracle).
#[test]
fn offset_polygons_shrinks_and_grows() {
    let a = vec![square(0, 100_000)]; // 10mm × 10mm square
    let grown = host::offset_polygons(&a, 1.0, OffsetJoinType::Miter, 0.0);
    let shrunk = host::offset_polygons(&a, -1.0, OffsetJoinType::Miter, 0.0);
    assert!(!grown.is_empty(), "positive offset must produce output");
    assert!(
        !shrunk.is_empty(),
        "negative offset within bounds must produce output"
    );
}

// KEEP-review (core-cross): sole cover of the Some(miter_limit) delegation arm → slicer_core::polygon_ops::offset_with_miter_limit; a dropped or ignored limit flattens clamped to the default-limit contour.
#[test]
fn offset_polygons_with_miter_limit_clamps_sharp_miter_corners() {
    let a = vec![square(0, 100_000)]; // 10mm × 10mm square
    let grown = host::offset_polygons(&a, 1.0, OffsetJoinType::Miter, 0.0);
    let clamped = host::offset_polygons_with_miter_limit(&a, 1.0, OffsetJoinType::Miter, 0.0, 1.2);
    assert!(
        !grown.is_empty(),
        "default-limit offset must produce output"
    );
    assert!(
        !clamped.is_empty(),
        "miter-limited offset must produce output"
    );
    assert_ne!(
        grown, clamped,
        "clamped miter (limit 1.2) must differ from the default-limit (2.0) contour"
    );
    let grown_pts = grown[0].contour.points.len();
    let clamped_pts = clamped[0].contour.points.len();
    assert!(
        clamped_pts > grown_pts,
        "clamping a 1.414 corner ratio above the 1.2 limit must add vertices: {clamped_pts} vs {grown_pts}"
    );
}

// KEEP-review (core-cross): distinct inline collinear-drop impl (tolerance reserved for a future DP variant), kept separate from core expolygons_simplify (RDP).
#[test]
fn simplify_polygon_drops_collinear_vertices() {
    // Square with an extra collinear vertex on the bottom edge.
    let poly = Polygon {
        points: vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 50_000, y: 0 }, // collinear with neighbors
            Point2 { x: 100_000, y: 0 },
            Point2 {
                x: 100_000,
                y: 100_000,
            },
            Point2 { x: 0, y: 100_000 },
        ],
    };
    let simplified = host::simplify_polygon(&poly, 0.0);
    assert_eq!(
        simplified.points.len(),
        4,
        "collinear midpoint must be dropped: {:?}",
        simplified.points
    );
}

// KEEP-review (core-cross): len<3 early-return witness for the distinct inline simplify.
#[test]
fn simplify_polygon_short_input_returned_as_is() {
    let poly = Polygon {
        points: vec![Point2 { x: 0, y: 0 }, Point2 { x: 1, y: 1 }],
    };
    let simplified = host::simplify_polygon(&poly, 0.0);
    assert_eq!(simplified.points, poly.points);
}

// ── Time ─────────────────────────────────────────────────────────────────

#[test]
fn now_us_is_monotonic_within_a_thread() {
    let a = host::now_us();
    let b = host::now_us();
    let c = host::now_us();
    assert!(
        a <= b && b <= c,
        "now_us must be non-decreasing: {a},{b},{c}"
    );
}
