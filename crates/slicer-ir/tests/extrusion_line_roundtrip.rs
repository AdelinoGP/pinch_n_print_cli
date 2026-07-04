#![allow(missing_docs)]

//! TDD scaffold for packet 112 (T-224), AC-5 + AC-N2:
//! JSON roundtrip and legacy-deserialization coverage for the new
//! `ExtrusionLine` / `ExtrusionJunction` Arachne beading-strategy-stack IR
//! types, plus a lock on the `SliceIR` schema-version bump to 4.7.0.
//!
//! `ExtrusionLine` / `ExtrusionJunction` are not yet re-exported at the
//! `slicer_ir` crate root (this packet is pure additive IR with no WASM-
//! boundary consumer), so they are referenced via the public `slice_ir`
//! module path: `slicer_ir::slice_ir::{ExtrusionLine, ExtrusionJunction}`.

use slicer_ir::slice_ir::{ExtrusionJunction, ExtrusionLine};
use slicer_ir::Point3WithWidth;

fn sample_junction(x: f32, width: f32, perimeter_index: u32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y: 1.0,
            z: 0.2,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        perimeter_index,
    }
}

/// AC-5: an `ExtrusionLine` with multiple junctions of varied width must
/// survive a `serde_json` roundtrip byte-for-byte (field-for-field).
#[test]
fn extrusion_line_roundtrip() {
    let original = ExtrusionLine {
        junctions: vec![
            sample_junction(0.0, 0.4, 0),
            sample_junction(1.0, 0.42, 0),
            sample_junction(2.0, 0.38, 1),
        ],
        inset_idx: 0,
        is_odd: true,
        is_closed: true,
    };

    let json = serde_json::to_string(&original).expect("serialization failed");
    let deserialized: ExtrusionLine = serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(
        deserialized, original,
        "ExtrusionLine must survive JSON roundtrip unchanged"
    );
}

/// Locks the `SliceIR` schema-version bump performed by this packet
/// (T-224): additive `ExtrusionLine`/`ExtrusionJunction` types bump
/// `CURRENT_SLICE_IR_SCHEMA_VERSION` from 4.6.0 to 4.7.0 (minor, additive).
#[test]
fn slice_ir_schema_version_is_4_7() {
    assert_eq!(
        slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION.major,
        4,
        "major version must remain 4 for this additive change"
    );
    assert_eq!(
        slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION.minor,
        7,
        "minor version must be bumped to 7 by the ExtrusionLine/ExtrusionJunction addition"
    );
}

/// AC-N2: legacy JSON that omits `is_odd`, `is_closed` on `ExtrusionLine` and
/// `perimeter_index` on a nested `ExtrusionJunction` must still deserialize,
/// with `#[serde(default)]` filling in `false` / `false` / `0` respectively.
#[test]
fn extrusion_line_legacy_deserialization() {
    let legacy_json = r#"{
        "junctions": [
            {
                "p": {
                    "x": 0.0,
                    "y": 1.0,
                    "z": 0.2,
                    "width": 0.4,
                    "flow_factor": 1.0
                }
            }
        ],
        "inset_idx": 0
    }"#;

    let deserialized: ExtrusionLine =
        serde_json::from_str(legacy_json).expect("deserialization of legacy payload failed");

    assert!(
        !deserialized.is_odd,
        "legacy payload without is_odd must default to false"
    );
    assert!(
        !deserialized.is_closed,
        "legacy payload without is_closed must default to false"
    );
    assert_eq!(
        deserialized.junctions[0].perimeter_index, 0,
        "legacy junction without perimeter_index must default to 0"
    );
}
