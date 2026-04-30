//! TDD test: `SupportGeometryIR` key tuple shape and sentinel value.

#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{ExPolygon, Polygon, SemVer, SupportGeometryIR, SupportGeometryKey};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_expolygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    }
}

/// Verifies:
/// 1. The map key is `SupportGeometryKey` with fields
///    `(global_support_layer_index: u32, object_id: ObjectId, region_id: RegionId)`.
/// 2. An entry keyed by `global_support_layer_index = u32::MAX` is retrievable
///    and is the designated intermediate-model-resolution-outline sentinel.
#[test]
fn support_geometry_ir_keys_and_sentinel() {
    let sentinel_key = SupportGeometryKey {
        global_support_layer_index: u32::MAX,
        object_id: String::from("obj-1"),
        region_id: 0,
    };
    let normal_key = SupportGeometryKey {
        global_support_layer_index: 3,
        object_id: String::from("obj-1"),
        region_id: 1,
    };

    let mut entries = HashMap::new();
    entries.insert(sentinel_key.clone(), vec![empty_expolygon()]);
    entries.insert(
        normal_key.clone(),
        vec![empty_expolygon(), empty_expolygon()],
    );

    let ir = SupportGeometryIR {
        schema_version: semver(1, 0, 0),
        support_layer_height_mm: 0.2,
        support_top_z_distance_mm: 0.1,
        entries,
    };

    // Key tuple shape: (u32, ObjectId/String, RegionId/u64)
    let sentinel = ir
        .entries
        .get(&sentinel_key)
        .expect("sentinel entry must be retrievable by u32::MAX key");
    assert_eq!(sentinel.len(), 1, "sentinel entry should have one polygon");

    // u32::MAX is the documented sentinel for intermediate model-resolution outlines.
    assert_eq!(
        sentinel_key.global_support_layer_index,
        u32::MAX,
        "sentinel key must be u32::MAX"
    );

    let normal = ir
        .entries
        .get(&normal_key)
        .expect("normal entry must be retrievable");
    assert_eq!(normal.len(), 2, "normal entry should have two polygons");

    assert_eq!(ir.entries.len(), 2);
}
