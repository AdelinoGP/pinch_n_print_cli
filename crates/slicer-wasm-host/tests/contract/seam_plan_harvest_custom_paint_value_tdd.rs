//! Production-boundary test for `harvest_seam_plan_ir_from`.
//!
//! The WIT `paint-value` variant does not currently include a `Custom` case,
//! so a `Custom` value cannot cross the WIT boundary today. The marshal
//! still has a defensive `Custom` match arm to guard against a future WIT
//! change. This test asserts that:
//!  1. The marshal function is reachable from outside the crate (re-export
//!     surface check), so production-boundary tests can exercise it.
//!  2. A valid WIT-shaped `SeamPlanEntry` round-trips through the marshal
//!     to a well-formed `SeamPlanIR` with the `variant_chain` field
//!     preserved (the production path that AC-2 depends on).
//!
//! The IR-side `Custom`-rejection regression coverage lives in
//! `crates/slicer-macros/tests/variant_chain_boundary_tdd.rs`.

#![allow(missing_docs)]

use slicer_wasm_host::host::prepass::slicer::types::geometry::SeamPoint3WithWidth;
use slicer_wasm_host::host::prepass::{PaintValue, ScoredSeamCandidate, SeamPlanEntry, SeamReason};
use slicer_wasm_host::marshal::harvest_seam_plan_ir_from;

fn make_point(x: f32, y: f32, z: f32) -> SeamPoint3WithWidth {
    SeamPoint3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

#[test]
fn harvest_round_trip_preserves_variant_chain_flag() {
    let entry = SeamPlanEntry {
        variant_chain: vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))],
        global_layer_index: 0,
        object_id: "obj-a".to_string(),
        region_id: "1".to_string(),
        chosen_position: make_point(1.0, 2.0, 0.2),
        chosen_wall_index: 0,
        scored_candidates: vec![ScoredSeamCandidate {
            position: make_point(1.0, 2.0, 0.2),
            score: 1.0,
            reason: SeamReason {
                tag: "aligned".to_string(),
            },
        }],
    };

    let ir = harvest_seam_plan_ir_from(vec![entry]).expect("valid entry must commit");
    assert_eq!(ir.entries.len(), 1);
    let e = &ir.entries[0];
    assert_eq!(e.region_key.global_layer_index, 0);
    assert_eq!(e.region_key.object_id, "obj-a");
    assert_eq!(e.region_key.region_id, 1);
    assert_eq!(e.region_key.variant_chain.len(), 1);
    assert_eq!(e.region_key.variant_chain[0].0, "fuzzy_skin");
    assert!(matches!(
        e.region_key.variant_chain[0].1,
        slicer_ir::PaintValue::Flag(true)
    ));
}

#[test]
fn harvest_round_trip_preserves_variant_chain_tool_index() {
    let entry = SeamPlanEntry {
        variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(2))],
        global_layer_index: 1,
        object_id: "obj-b".to_string(),
        region_id: "5".to_string(),
        chosen_position: make_point(3.0, 4.0, 0.4),
        chosen_wall_index: 0,
        scored_candidates: vec![],
    };

    let ir = harvest_seam_plan_ir_from(vec![entry]).expect("valid entry must commit");
    assert_eq!(ir.entries.len(), 1);
    let e = &ir.entries[0];
    assert_eq!(e.region_key.variant_chain.len(), 1);
    assert_eq!(e.region_key.variant_chain[0].0, "material");
    assert!(matches!(
        e.region_key.variant_chain[0].1,
        slicer_ir::PaintValue::ToolIndex(2)
    ));
}
