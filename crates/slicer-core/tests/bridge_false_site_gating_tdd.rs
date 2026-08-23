#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::algos::prepass_slice::{
    assemble_bridge_areas, gate_bridge_areas_by_unsupported_span,
};
use slicer_core::polygon_ops::intersection;
use slicer_ir::{
    BridgeRegion, ExPolygon, ObjectSurfaceData, Point2, Polygon, SlicedRegion,
    SurfaceClassificationIR,
};

fn square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

fn region_with_bridge(bridge: ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: "object".to_string(),
        bridge_areas: vec![bridge.clone()],
        infill_areas: vec![bridge],
        ..Default::default()
    }
}

#[test]
fn solid_underneath_span_produces_no_bridge_area() {
    let bridge = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_bridge(bridge.clone());
    gate_bridge_areas_by_unsupported_span(&mut region, Some(&[bridge]));
    assert!(region.bridge_areas.is_empty());
}

#[test]
fn unsupported_span_retains_bridge_area() {
    let bridge = square(0.0, 0.0, 10.0, 10.0);
    let anchor = square(0.0, 0.0, 4.0, 10.0);
    let mut region = region_with_bridge(bridge.clone());
    gate_bridge_areas_by_unsupported_span(&mut region, Some(&[anchor.clone()]));
    assert!(!region.bridge_areas.is_empty());
    assert!(region.bridge_areas.iter().any(|area| area != &bridge));
    assert!(intersection(&region.bridge_areas, &[anchor]).is_empty());
}

#[test]
fn fully_supported_candidate_rejected_zero_bridge_area() {
    let bridge = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_bridge(bridge.clone());
    gate_bridge_areas_by_unsupported_span(&mut region, Some(&[bridge]));
    assert!(region.bridge_areas.is_empty());
}

#[test]
fn ungated_candidates_cannot_silently_return() {
    let bridge = square(0.0, 0.0, 10.0, 10.0);
    // Start with EMPTY bridge_areas so the ungated candidate provably comes
    // from `assemble_bridge_areas` stamping, not from the fixture.
    let mut ungated = SlicedRegion {
        object_id: "object".to_string(),
        bridge_areas: Vec::new(),
        infill_areas: vec![bridge.clone()],
        ..Default::default()
    };
    let surface = SurfaceClassificationIR {
        per_object: HashMap::from([(
            "object".to_string(),
            ObjectSurfaceData {
                bridge_regions: vec![BridgeRegion {
                    is_valid: true,
                    xy_footprint: vec![bridge.clone()],
                    ..Default::default()
                }],
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    assemble_bridge_areas(&mut ungated, Some(&surface));
    assert!(!ungated.bridge_areas.is_empty());

    let mut gated = ungated.clone();
    gate_bridge_areas_by_unsupported_span(&mut gated, Some(&[bridge]));
    assert_ne!(gated.bridge_areas, ungated.bridge_areas);
    assert!(gated.bridge_areas.is_empty());
}

#[test]
fn no_lower_layer_clears_bridge_areas() {
    let bridge = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_bridge(bridge);
    gate_bridge_areas_by_unsupported_span(&mut region, None);
    assert!(region.bridge_areas.is_empty());
}

#[test]
fn existing_empty_lower_layer_retains_bridge_area() {
    let bridge = square(0.0, 0.0, 10.0, 10.0);
    let mut region = region_with_bridge(bridge.clone());
    gate_bridge_areas_by_unsupported_span(&mut region, Some(&[]));
    assert_eq!(region.bridge_areas, vec![bridge]);
}
