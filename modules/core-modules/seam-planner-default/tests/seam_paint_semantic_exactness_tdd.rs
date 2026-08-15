//! Seam paint semantic classification tests.

#![allow(missing_docs)]

#[path = "../src/comparator.rs"]
mod comparator;
#[path = "../src/contours.rs"]
mod contours;
#[path = "../src/visibility.rs"]
mod visibility;

use contours::{extract_layer_contours, test_mesh};
use slicer_ir::{PaintSemantic, PaintValue};
use visibility::{build_seam_candidates, LayerInfo};

fn prism_setup() -> (
    Vec<[f32; 3]>,
    Vec<[u32; 3]>,
    Vec<LayerInfo>,
    Vec<Vec<contours::Contour>>,
) {
    let (vertices, triangles) = test_mesh::cuboid(10.0, 10.0, 4.0);
    let layers: Vec<LayerInfo> = [0.1]
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

fn candidates_for(semantic: PaintSemantic) -> Vec<comparator::SeamCandidate> {
    candidates_for_value(semantic, PaintValue::Flag(true))
}

fn candidates_for_value(
    semantic: PaintSemantic,
    value: PaintValue,
) -> Vec<comparator::SeamCandidate> {
    let (vertices, triangles, layers, contours) = prism_setup();
    let vertex_count = contours[0][0].points.len();
    let values = vec![vec![Some(value); vertex_count]];
    let annotations = [(semantic, values.as_slice())];
    build_seam_candidates(
        &vertices,
        &triangles,
        &layers,
        &contours,
        false,
        0.4,
        Some(&annotations),
        0,
    )
    .remove(0)
}

#[test]
fn support_semantics() {
    for semantic in [
        PaintSemantic::SupportEnforcer,
        PaintSemantic::SupportBlocker,
    ] {
        assert!(
            candidates_for(semantic)
                .iter()
                .all(|candidate| candidate.point_type
                    == comparator::EnforcedBlockedSeamPoint::Neutral)
        );
    }
}

#[test]
fn exact_seam_semantics() {
    assert!(candidates_for(PaintSemantic::Custom("seam_enforcer".to_string()))
        .iter()
        .all(|candidate| candidate.point_type == comparator::EnforcedBlockedSeamPoint::Enforced));
    assert!(
        candidates_for(PaintSemantic::Custom("seam_blocker".to_string()))
            .iter()
            .all(|candidate| candidate.point_type == comparator::EnforcedBlockedSeamPoint::Blocked)
    );
    assert!(candidates_for_value(
        PaintSemantic::Custom("seam_enforcer".to_string()),
        PaintValue::Flag(false),
    )
    .iter()
    .all(|candidate| candidate.point_type == comparator::EnforcedBlockedSeamPoint::Neutral));
}

#[test]
fn support_named_custom_semantics_are_neutral() {
    for semantic in [
        PaintSemantic::Custom("SupportEnforcer".to_string()),
        PaintSemantic::Custom("SupportBlocker".to_string()),
    ] {
        assert!(
            candidates_for(semantic)
                .iter()
                .all(|candidate| candidate.point_type
                    == comparator::EnforcedBlockedSeamPoint::Neutral)
        );
    }
}
