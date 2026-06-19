//! TDD: AC-3 — `variable_width` converts a `ThickPolyline` into an `ExtrusionPath3D` (z=0, flow_factor=1, overhang_quartile=None).

/// AC-3: `variable_width` converts a `ThickPolyline` into an `ExtrusionPath3D`
/// with canonical defaults (z=0, flow_factor=1, overhang_quartile=None,
/// speed_factor=1, role preserved).
use slicer_ir::{variable_width, ExtrusionRole, Point2WithWidth, ThickPolyline};

#[test]
fn variable_width_ac3() {
    let tp = ThickPolyline {
        points: vec![
            Point2WithWidth {
                x: 0.0,
                y: 0.0,
                width: 0.4,
            },
            Point2WithWidth {
                x: 5.0,
                y: 0.0,
                width: 0.6,
            },
            Point2WithWidth {
                x: 10.0,
                y: 0.0,
                width: 0.4,
            },
        ],
    };

    let path = variable_width(&tp, ExtrusionRole::ThinWall);

    assert_eq!(path.points.len(), 3, "should produce 3 points");
    assert_eq!(path.role, ExtrusionRole::ThinWall, "role must be preserved");
    assert!(
        (path.speed_factor - 1.0).abs() < f32::EPSILON,
        "speed_factor must be 1.0"
    );

    let expected = [(0.0f32, 0.0f32, 0.4f32), (5.0, 0.0, 0.6), (10.0, 0.0, 0.4)];
    for (i, (pt, (ex, ey, ew))) in path.points.iter().zip(expected.iter()).enumerate() {
        assert!(
            (pt.x - ex).abs() < f32::EPSILON,
            "point[{i}].x: got {}, expected {ex}",
            pt.x
        );
        assert!(
            (pt.y - ey).abs() < f32::EPSILON,
            "point[{i}].y: got {}, expected {ey}",
            pt.y
        );
        assert!(
            (pt.z - 0.0).abs() < f32::EPSILON,
            "point[{i}].z must be 0.0, got {}",
            pt.z
        );
        assert!(
            (pt.width - ew).abs() < f32::EPSILON,
            "point[{i}].width: got {}, expected {ew}",
            pt.width
        );
        assert!(
            (pt.flow_factor - 1.0).abs() < f32::EPSILON,
            "point[{i}].flow_factor must be 1.0, got {}",
            pt.flow_factor
        );
        assert_eq!(
            pt.overhang_quartile, None,
            "point[{i}].overhang_quartile must be None"
        );
    }
}
