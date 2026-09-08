//! TDD (packet 240a, AC-6 Rust half): `SlicedRegion` must carry a `raft_fill`
//! polygon list so a later packet can hand raft areas to the fill stage.
//!
//! The field is additive and `#[serde(default)]`, so fixtures written against
//! the pre-bump `SliceIR` schema (`4.8.0` on disk when this test was authored)
//! must still deserialize, with `raft_fill` coming back empty.

use slicer_ir::slice_ir::{ExPolygon, Point2, Polygon, SlicedRegion};

/// Fixture predating `raft_fill`. `schema_version` is the CURRENT pre-bump
/// `CURRENT_SLICE_IR_SCHEMA_VERSION` value (4.8.0); the object is a `SliceIR`
/// carrying one region that has no `raft_fill` key at all.
const PRE_RAFT_FILL_SLICE_IR_JSON: &str = r#"{
  "schema_version": { "major": 4, "minor": 8, "patch": 0 },
  "global_layer_index": 0,
  "z": 0.2,
  "regions": [
    {
      "object_id": "obj-0",
      "region_id": 0,
      "polygons": [],
      "infill_areas": [],
      "nonplanar_surface": null,
      "effective_layer_height": 0.2
    }
  ]
}"#;

fn square_mm() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        ..Default::default()
    }
}

/// AC-6: `raft_fill` defaults empty, and a populated one survives a round-trip.
#[test]
fn raft_fill_defaults_empty_and_survives_roundtrip() {
    let default_region = SlicedRegion::default();
    assert!(
        default_region.raft_fill.is_empty(),
        "SlicedRegion::default().raft_fill must be empty"
    );

    let region = SlicedRegion {
        raft_fill: vec![square_mm()],
        ..Default::default()
    };

    let json = serde_json::to_string(&region).expect("SlicedRegion serializes");
    let back: SlicedRegion = serde_json::from_str(&json).expect("SlicedRegion deserializes");
    assert_eq!(back, region, "raft_fill must round-trip unchanged");
    assert_eq!(back.raft_fill.len(), 1);
    assert_eq!(back.raft_fill[0], square_mm());
}

/// AC-6: `#[serde(default)]` keeps pre-bump fixtures (schema 4.8.0, no
/// `raft_fill` key) deserializable, yielding an empty `raft_fill`.
#[test]
fn raft_fill_is_serde_default_backward_compatible() {
    let ir: slicer_ir::slice_ir::SliceIR =
        serde_json::from_str(PRE_RAFT_FILL_SLICE_IR_JSON).expect("pre-raft_fill fixture parses");

    assert_eq!(ir.regions.len(), 1);
    assert!(
        ir.regions[0].raft_fill.is_empty(),
        "a fixture with no raft_fill key must default to an empty raft_fill"
    );
}
