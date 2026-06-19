//! TDD tests for the widened `WallBoundaryType::MaterialBoundary` (packet 102 step 2).
//!
//! Verifies:
//!  - New shape with `segments: Vec<MaterialBoundarySegment>` round-trips.
//!  - Old `{ adjacent_tool: u32 }` shape deserializes via the migration adapter.

use slicer_ir::{
    MaterialBoundarySegment, SemVer, WallBoundaryType, CURRENT_SLICE_IR_SCHEMA_VERSION,
};

#[test]
fn schema_version_is_4_2_0() {
    // AC-3: the MaterialBoundary widening is an additive minor bump 4.1.0 -> 4.2.0.
    assert_eq!(
        CURRENT_SLICE_IR_SCHEMA_VERSION,
        SemVer {
            major: 4,
            minor: 2,
            patch: 0
        },
        "MaterialBoundary widening must bump the schema to 4.2.0"
    );
}

#[test]
fn three_transition_polygon_carries_three_segments() {
    // AC-3: a polygon with three transitions across four tool indices [1,2,3,1]
    // must carry a `Vec<MaterialBoundarySegment>` of length 3 — one per
    // transition, not just the first — and survive a round-trip unchanged.
    let boundary = WallBoundaryType::MaterialBoundary {
        segments: vec![
            MaterialBoundarySegment {
                point_range: 1..2,
                near_tool: Some(1),
                far_tool: Some(2),
            },
            MaterialBoundarySegment {
                point_range: 3..4,
                near_tool: Some(2),
                far_tool: Some(3),
            },
            MaterialBoundarySegment {
                point_range: 5..6,
                near_tool: Some(3),
                far_tool: Some(1),
            },
        ],
    };
    let WallBoundaryType::MaterialBoundary { ref segments } = boundary else {
        panic!("expected MaterialBoundary");
    };
    assert_eq!(
        segments.len(),
        3,
        "three transitions must yield three segments"
    );

    let serialized = postcard::to_allocvec(&boundary).expect("serialize");
    let deserialized: WallBoundaryType = postcard::from_bytes(&serialized).expect("deserialize");
    assert_eq!(boundary, deserialized);
}

#[test]
fn new_material_boundary_roundtrips() {
    let original = WallBoundaryType::MaterialBoundary {
        segments: vec![
            MaterialBoundarySegment {
                point_range: 0..5,
                near_tool: Some(1),
                far_tool: Some(2),
            },
            MaterialBoundarySegment {
                point_range: 5..10,
                near_tool: Some(2),
                far_tool: Some(1),
            },
        ],
    };
    let serialized = postcard::to_allocvec(&original).expect("serialize");
    let deserialized: WallBoundaryType = postcard::from_bytes(&serialized).expect("deserialize");
    assert_eq!(original, deserialized);
}

#[test]
fn old_single_tool_shape_deserializes_via_migration() {
    // Simulate the pre-4.2.0 shape: `{ adjacent_tool: 2 }`
    let old_json = serde_json::json!({
        "MaterialBoundary": {
            "adjacent_tool": 2
        }
    });
    let deserialized: WallBoundaryType =
        serde_json::from_value(old_json).expect("migrate old shape");
    assert_eq!(
        deserialized,
        WallBoundaryType::MaterialBoundary {
            segments: vec![MaterialBoundarySegment {
                point_range: 0..1,
                near_tool: None,
                far_tool: Some(2),
            }]
        }
    );
}

#[test]
fn empty_segments_is_valid() {
    let original = WallBoundaryType::MaterialBoundary {
        segments: Vec::new(),
    };
    let serialized = postcard::to_allocvec(&original).expect("serialize");
    let deserialized: WallBoundaryType = postcard::from_bytes(&serialized).expect("deserialize");
    assert_eq!(original, deserialized);
}

#[test]
fn exterior_surface_and_interior_unchanged() {
    let ext = WallBoundaryType::ExteriorSurface;
    let ser = postcard::to_allocvec(&ext).expect("serialize");
    let de: WallBoundaryType = postcard::from_bytes(&ser).expect("deserialize");
    assert_eq!(ext, de);

    let int = WallBoundaryType::Interior;
    let ser = postcard::to_allocvec(&int).expect("serialize");
    let de: WallBoundaryType = postcard::from_bytes(&ser).expect("deserialize");
    assert_eq!(int, de);
}
