#![allow(missing_docs)]

//! TDD red test for packet 193, AC-N3:
//! JSON roundtrip for `Point3WithWidth.overhang_distance_mm`.
//!
//! An ABSENT `overhang_distance_mm` in a serialized 1.x payload must
//! deserialize as `None` (via `#[serde(default)]` on an `Option<f32>`),
//! mirroring the `overhang_quartile` precedent in
//! `point3_overhang_quartile_roundtrip.rs`. No fixture needs re-recording;
//! this is what makes the schema bump additive rather than breaking.
//!
//! References `Point3WithWidth::overhang_distance_mm`, which does not exist
//! yet — this binary MUST fail to compile until the field lands.

use slicer_ir::Point3WithWidth;

/// Deserializing a legacy JSON payload that lacks the `overhang_distance_mm`
/// field must produce `overhang_distance_mm: None` (via `#[serde(default)]`).
#[test]
fn absent_overhang_distance_deserializes_as_none() {
    // Old payload: no overhang_distance_mm key (mirrors the quartile
    // precedent, which also omits dist_to_top_mm).
    let legacy_json = r#"{
        "x": 3.0,
        "y": 4.0,
        "z": 0.4,
        "width": 0.42,
        "flow_factor": 1.0
    }"#;

    let deserialized: Point3WithWidth =
        serde_json::from_str(legacy_json).expect("deserialization of legacy payload failed");

    assert_eq!(
        deserialized.overhang_distance_mm, None,
        "Legacy JSON without overhang_distance_mm must deserialize to None; got {:?}",
        deserialized.overhang_distance_mm
    );
}
