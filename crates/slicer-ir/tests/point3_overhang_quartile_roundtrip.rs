#![allow(missing_docs)]

//! TDD scaffold for packet 57 (TASK-182), AC-6:
//! JSON roundtrip for `Point3WithWidth.overhang_quartile`.
//!
//! These tests may PASS in RED because `#[serde(default)]` is already wired.
//! They are included to lock the contract and catch regressions.

use slicer_ir::Point3WithWidth;

/// Serialize a `Point3WithWidth` with `overhang_quartile: Some(2)`, then
/// deserialize and confirm the field survives the roundtrip.
#[test]
fn overhang_quartile_roundtrip_some() {
    let original = Point3WithWidth {
        x: 1.0,
        y: 2.0,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: Some(2),
    };

    let json = serde_json::to_string(&original).expect("serialization failed");
    let deserialized: Point3WithWidth =
        serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(
        deserialized.overhang_quartile,
        Some(2),
        "overhang_quartile must survive JSON roundtrip; got {:?}",
        deserialized.overhang_quartile
    );
}

/// Deserializing a legacy JSON payload that lacks the `overhang_quartile` field
/// must produce `overhang_quartile: None` (via `#[serde(default)]`).
#[test]
fn overhang_quartile_missing_field_defaults_to_none() {
    // Old payload: no overhang_quartile key.
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
        deserialized.overhang_quartile, None,
        "Legacy JSON without overhang_quartile must deserialize to None; got {:?}",
        deserialized.overhang_quartile
    );
}
