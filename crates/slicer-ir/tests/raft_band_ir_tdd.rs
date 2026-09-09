//! TDD (packet 240a, AC-1): `GlobalLayer` must carry an explicit `is_raft`
//! flag so raft-ness is never inferred from the layer index.
//!
//! Layer indices stay `u32`: raft layers occupy global indices `0..N-1` and
//! model layers shift to `N..`. The flag is additive and `#[serde(default)]`,
//! so `LayerPlanIR` fixtures written before it still deserialize with
//! `is_raft == false`.

use slicer_ir::slice_ir::{GlobalLayer, LayerPlanIR};

/// Fixture predating `is_raft`: two global layers, neither carrying the key.
const PRE_IS_RAFT_LAYER_PLAN_IR_JSON: &str = r#"{
  "schema_version": { "major": 1, "minor": 0, "patch": 0 },
  "global_layers": [
    {
      "index": 0,
      "z": 0.2,
      "active_regions": [],
      "has_nonplanar": false,
      "is_sync_layer": false
    },
    {
      "index": 1,
      "z": 0.4,
      "active_regions": [],
      "has_nonplanar": false,
      "is_sync_layer": false
    }
  ],
  "object_participation": {}
}"#;

fn layer(index: u32, z: f32, is_raft: bool) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        is_raft,
        ..Default::default()
    }
}

/// AC-1: the flag defaults `false`, a raft-prefixed plan round-trips unchanged,
/// and every layer-index field is still `u32`.
#[test]
fn is_raft_defaults_false_and_survives_roundtrip() {
    assert!(
        !GlobalLayer::default().is_raft,
        "GlobalLayer::default().is_raft must be false"
    );

    // Raft band of 2 at global indices 0..1; model layers shift to 2...
    let plan = LayerPlanIR {
        global_layers: vec![
            layer(0, 0.3, true),
            layer(1, 0.6, true),
            layer(2, 0.8, false),
            layer(3, 1.0, false),
        ],
        ..Default::default()
    };

    let json = serde_json::to_string(&plan).expect("LayerPlanIR serializes");
    let back: LayerPlanIR = serde_json::from_str(&json).expect("LayerPlanIR deserializes");
    assert_eq!(back, plan, "LayerPlanIR with a raft band must round-trip");

    let markers: Vec<(u32, bool)> = back
        .global_layers
        .iter()
        .map(|l| (l.index, l.is_raft))
        .collect();
    assert_eq!(
        markers,
        vec![(0u32, true), (1, true), (2, false), (3, false)],
        "raft-ness must be carried by is_raft, and indices must stay 0.. in order"
    );
}

/// AC-1: `#[serde(default)]` keeps pre-`is_raft` fixtures deserializable.
#[test]
fn is_raft_is_serde_default_backward_compatible() {
    let plan: LayerPlanIR = serde_json::from_str(PRE_IS_RAFT_LAYER_PLAN_IR_JSON)
        .expect("pre-is_raft LayerPlanIR fixture parses");

    assert_eq!(plan.global_layers.len(), 2);
    assert!(
        plan.global_layers.iter().all(|l| !l.is_raft),
        "a fixture with no is_raft key must default every layer to is_raft == false"
    );
}
