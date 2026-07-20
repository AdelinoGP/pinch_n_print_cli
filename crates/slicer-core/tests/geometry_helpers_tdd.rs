#![allow(missing_docs)]

use slicer_core::{distribute_points, flow_correction, path_length, seg_len_3d, segment_path};
use slicer_ir::{units_to_mm, Point2, Point3WithWidth};

const EPS: f32 = 1.0e-5;

fn point(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.45,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn assert_close(actual: f32, expected: f32) {
    let delta = (actual - expected).abs();
    assert!(
        delta <= EPS,
        "expected {expected}, got {actual}, delta {delta}"
    );
}

fn assert_point2_mm(actual: Point2, expected_x: f32, expected_y: f32) {
    let (x_mm, y_mm) = actual.to_mm();
    assert_close(x_mm, expected_x);
    assert_close(y_mm, expected_y);
}

fn assert_point3(actual: Point3WithWidth, expected_x: f32, expected_y: f32, expected_z: f32) {
    assert_close(actual.x, expected_x);
    assert_close(actual.y, expected_y);
    assert_close(actual.z, expected_z);
}

fn segment_lengths_mm(points: &[Point2]) -> Vec<f32> {
    points
        .windows(2)
        .map(|pair| {
            let dx = units_to_mm(pair[1].x - pair[0].x);
            let dy = units_to_mm(pair[1].y - pair[0].y);
            (dx * dx + dy * dy).sqrt()
        })
        .collect()
}

#[test]
fn segment_path_subdivides_long_segment_with_endpoints_preserved() {
    let points = segment_path(Point2::from_mm(0.0, 0.0), Point2::from_mm(10.0, 0.0), 3.0);

    assert_eq!(points.len(), 5);
    assert_point2_mm(points[0], 0.0, 0.0);
    assert_point2_mm(points[1], 2.5, 0.0);
    assert_point2_mm(points[2], 5.0, 0.0);
    assert_point2_mm(points[3], 7.5, 0.0);
    assert_point2_mm(points[4], 10.0, 0.0);

    for len in segment_lengths_mm(&points) {
        assert!(len <= 3.0 + EPS, "segment length {len} exceeded max length");
    }
}

#[test]
fn segment_path_handles_exact_division_short_segment_and_zero_length() {
    let exact = segment_path(Point2::from_mm(0.0, 0.0), Point2::from_mm(9.0, 0.0), 3.0);
    assert_eq!(exact.len(), 4);
    assert_point2_mm(exact[0], 0.0, 0.0);
    assert_point2_mm(exact[1], 3.0, 0.0);
    assert_point2_mm(exact[2], 6.0, 0.0);
    assert_point2_mm(exact[3], 9.0, 0.0);

    let short = segment_path(Point2::from_mm(1.0, 2.0), Point2::from_mm(2.0, 2.0), 3.0);
    assert_eq!(short.len(), 2);
    assert_point2_mm(short[0], 1.0, 2.0);
    assert_point2_mm(short[1], 2.0, 2.0);

    let zero = segment_path(Point2::from_mm(4.0, 4.0), Point2::from_mm(4.0, 4.0), 3.0);
    assert_eq!(zero.len(), 1);
    assert_point2_mm(zero[0], 4.0, 4.0);
}

#[test]
fn segment_path_never_emits_segments_longer_than_requested_limit_for_known_cases() {
    let cases = [
        (1.0_f32, 0.5_f32),
        (1.0_f32, 0.75_f32),
        (2.5_f32, 0.7_f32),
        (5.0_f32, 2.0_f32),
        (11.25_f32, 3.0_f32),
    ];

    for (length_mm, max_len_mm) in cases {
        let points = segment_path(
            Point2::from_mm(0.0, 0.0),
            Point2::from_mm(length_mm, 0.0),
            max_len_mm,
        );

        assert!(!points.is_empty());
        assert_point2_mm(points[0], 0.0, 0.0);
        assert_point2_mm(*points.last().unwrap(), length_mm, 0.0);

        for len in segment_lengths_mm(&points) {
            assert!(
                len <= max_len_mm + EPS,
                "segment length {len} exceeded limit {max_len_mm}"
            );
        }
    }
}

#[test]
fn path_length_handles_empty_single_point_and_cornered_polyline() {
    assert_close(path_length(&[]), 0.0);
    assert_close(path_length(&[point(1.0, 2.0, 3.0)]), 0.0);

    let path = vec![
        point(0.0, 0.0, 0.2),
        point(3.0, 0.0, 0.2),
        point(3.0, 4.0, 0.2),
    ];
    assert_close(path_length(&path), 7.0);
}

#[test]
fn distribute_points_preserves_endpoints_and_even_spacing_on_a_straight_path() {
    let path = vec![point(0.0, 0.0, 0.2), point(10.0, 0.0, 0.2)];

    let samples = distribute_points(&path, 5);

    assert_eq!(samples.len(), 5);
    assert_point3(samples[0], 0.0, 0.0, 0.2);
    assert_point3(samples[1], 2.5, 0.0, 0.2);
    assert_point3(samples[2], 5.0, 0.0, 0.2);
    assert_point3(samples[3], 7.5, 0.0, 0.2);
    assert_point3(samples[4], 10.0, 0.0, 0.2);
}

#[test]
fn distribute_points_follows_corners_deterministically() {
    let path = vec![
        point(0.0, 0.0, 0.2),
        point(4.0, 0.0, 0.2),
        point(4.0, 3.0, 0.2),
    ];

    let samples = distribute_points(&path, 5);

    assert_eq!(samples.len(), 5);
    assert_point3(samples[0], 0.0, 0.0, 0.2);
    assert_point3(samples[1], 1.75, 0.0, 0.2);
    assert_point3(samples[2], 3.5, 0.0, 0.2);
    assert_point3(samples[3], 4.0, 1.25, 0.2);
    assert_point3(samples[4], 4.0, 3.0, 0.2);
}

#[test]
fn distribute_points_returns_empty_for_zero_requested_samples() {
    let path = vec![point(0.0, 0.0, 0.2), point(1.0, 0.0, 0.2)];

    let samples = distribute_points(&path, 0);

    assert!(samples.is_empty());
}

#[test]
fn seg_len_3d_returns_euclidean_length() {
    assert_close(seg_len_3d(3.0, 4.0, 12.0), 13.0);
}

#[test]
fn flow_correction_is_finite_and_monotonic_for_positive_z_deviation() {
    let planar = flow_correction(5.0, 0.0, 0.0);
    let non_planar = flow_correction(5.0, 0.0, 2.0);

    assert!(planar.is_finite(), "planar correction should stay finite");
    assert!(
        non_planar.is_finite(),
        "non-planar correction should stay finite"
    );
    assert!(planar > 0.0, "planar correction should remain positive");
    assert!(
        non_planar >= planar,
        "positive z deviation should not reduce correction: planar={planar}, non_planar={non_planar}"
    );
}
