//! Canonical visibility sampling and seam-candidate fidelity tests.

#![allow(missing_docs)]

#[path = "../src/comparator.rs"]
mod comparator;
#[path = "../src/contours.rs"]
mod contours;
#[path = "../src/visibility.rs"]
mod visibility;

use contours::{extract_layer_contours, test_mesh};
use slicer_ir::{PaintSemantic, PaintValue};
use visibility::{
    build_seam_candidates, build_seam_candidates_with_sample_count, compute_global_visibility,
    LayerInfo, RAYS_PER_SIDE, VISIBILITY_SAMPLES_COUNT,
};

fn prism_setup() -> (
    Vec<[f32; 3]>,
    Vec<[u32; 3]>,
    Vec<LayerInfo>,
    Vec<Vec<contours::Contour>>,
) {
    let (vertices, triangles) = test_mesh::cuboid(10.0, 10.0, 4.0);
    let layers: Vec<LayerInfo> = [0.1, 0.3]
        .into_iter()
        .map(|z| LayerInfo {
            z,
            height: 0.2,
            layer_angle: 0.0,
        })
        .collect();
    let contours: Vec<Vec<contours::Contour>> = layers
        .iter()
        .map(|layer| extract_layer_contours(&vertices, &triangles, layer.z))
        .collect();
    (vertices, triangles, layers, contours)
}

fn assert_visibility_bits_equal(
    left: &visibility::GlobalVisibility,
    right: &visibility::GlobalVisibility,
) {
    assert_eq!(left.total_area.to_bits(), right.total_area.to_bits());
    assert_eq!(left.samples.len(), right.samples.len());
    for (left, right) in left.samples.iter().zip(&right.samples) {
        for axis in 0..3 {
            assert_eq!(
                left.position[axis].to_bits(),
                right.position[axis].to_bits()
            );
            assert_eq!(left.normal[axis].to_bits(), right.normal[axis].to_bits());
        }
        assert_eq!(left.visibility.to_bits(), right.visibility.to_bits());
    }
}

fn assert_candidate_bits_equal(
    left: &comparator::SeamCandidate,
    right: &comparator::SeamCandidate,
) {
    for axis in 0..3 {
        assert_eq!(
            left.position[axis].to_bits(),
            right.position[axis].to_bits()
        );
    }
    assert_eq!(left.visibility.to_bits(), right.visibility.to_bits());
    assert_eq!(left.overhang.to_bits(), right.overhang.to_bits());
    assert_eq!(
        left.unsupported_dist.to_bits(),
        right.unsupported_dist.to_bits()
    );
    assert_eq!(
        left.embedded_distance.to_bits(),
        right.embedded_distance.to_bits()
    );
    assert_eq!(
        left.local_ccw_angle.to_bits(),
        right.local_ccw_angle.to_bits()
    );
    assert_eq!(left.layer_angle.to_bits(), right.layer_angle.to_bits());
    assert_eq!(left.central_enforcer, right.central_enforcer);
    assert_eq!(left.flow_width.to_bits(), right.flow_width.to_bits());
}

#[test]
fn sample_count_matches_canonical_30000() {
    assert_eq!(VISIBILITY_SAMPLES_COUNT, 30000);
}

#[test]
fn ray_count_matches_canonical_25() {
    assert_eq!(RAYS_PER_SIDE * RAYS_PER_SIDE, 25);
}

#[test]
fn aligned_back_visibility_stays_in_extended_range() {
    let (vertices, triangles, _, _) = prism_setup();
    let visibility = compute_global_visibility(&vertices, &triangles, true, 17, Some(100));
    assert!(visibility
        .samples
        .iter()
        .all(|sample| (0.0..=2.0).contains(&sample.visibility)));
}

#[test]
fn determinism_with_fixed_seed() {
    let (vertices, triangles, _, _) = prism_setup();
    let left = compute_global_visibility(&vertices, &triangles, false, 1234, Some(100));
    let right = compute_global_visibility(&vertices, &triangles, false, 1234, Some(100));
    assert_visibility_bits_equal(&left, &right);
}

#[test]
fn flow_width_from_resolved_config() {
    let (vertices, triangles, layers, contours) = prism_setup();
    let candidates = build_seam_candidates(
        &vertices, &triangles, &layers, &contours, false, 0.8, None, 0,
    );
    let candidate = &candidates[1][0];

    assert_eq!(candidate.flow_width, 0.8);
    assert!((candidate.unsupported_dist - 0.32).abs() < 1e-3);
    assert!((candidate.embedded_distance - 0.52).abs() < 1e-3);
}

#[test]
fn determinism_two_runs_bit_identical() {
    let (vertices, triangles, layers, contours) = prism_setup();
    let left = build_seam_candidates_with_sample_count(
        &vertices,
        &triangles,
        &layers,
        &contours,
        true,
        0.4,
        None,
        9876,
        Some(100),
    );
    let right = build_seam_candidates_with_sample_count(
        &vertices,
        &triangles,
        &layers,
        &contours,
        true,
        0.4,
        None,
        9876,
        Some(100),
    );

    assert_eq!(left.len(), right.len());
    for (left_layer, right_layer) in left.iter().zip(&right) {
        assert_eq!(left_layer.len(), right_layer.len());
        for (left, right) in left_layer.iter().zip(right_layer) {
            assert_candidate_bits_equal(left, right);
        }
    }
}

#[test]
fn paint_annotations_set_point_type() {
    let (vertices, triangles, layers, contours) = prism_setup();
    let seam_values = vec![vec![
        Some(PaintValue::Custom("enforced".to_string())),
        Some(PaintValue::Custom("enforced".to_string())),
        Some(PaintValue::Custom("blocked".to_string())),
        None,
    ]];
    let paint_annotations = [(
        PaintSemantic::Custom("seam".to_string()),
        seam_values.as_slice(),
    )];

    let candidates = build_seam_candidates_with_sample_count(
        &vertices,
        &triangles,
        &layers,
        &contours,
        false,
        0.4,
        Some(&paint_annotations),
        0,
        Some(100),
    );
    let layer = &candidates[0];

    assert_eq!(
        layer[0].point_type,
        comparator::EnforcedBlockedSeamPoint::Enforced
    );
    assert!(layer[0].central_enforcer);
    assert_eq!(
        layer[1].point_type,
        comparator::EnforcedBlockedSeamPoint::Enforced
    );
    assert!(!layer[1].central_enforcer);
    assert_eq!(
        layer[2].point_type,
        comparator::EnforcedBlockedSeamPoint::Blocked
    );
    assert_eq!(
        layer[3].point_type,
        comparator::EnforcedBlockedSeamPoint::Neutral
    );
}
