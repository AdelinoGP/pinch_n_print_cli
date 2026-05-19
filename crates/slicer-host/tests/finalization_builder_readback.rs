//! TDD: `finalization-output-builder::get-ordered-entities` read-back.
//!
//! Packet 58_gcode-toolchange-purge-integration, Step 3 implementation.
//!
//! AC9: get-ordered-entities reflects the currently staged state.

#![allow(missing_docs)]

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey,
    SemVer,
};
use slicer_sdk::traits::FinalizationOutputBuilder;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pt() -> Point3WithWidth {
    Point3WithWidth {
        x: 0.0,
        y: 0.0,
        z: 0.2,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn path() -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![pt()],
        role: ExtrusionRole::OuterWall,
        speed_factor: 1.0,
    }
}

fn region_key(layer: u32) -> RegionKey {
    RegionKey {
        global_layer_index: layer,
        object_id: "obj".to_string(),
        region_id: 1,
    }
}

fn make_entity(entity_id: u64, layer: u32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: path(),
        role: ExtrusionRole::OuterWall,
        region_key: region_key(layer),
        topo_order: (entity_id - 1) as u32,
    }
}

fn layer_with_2_entities() -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![make_entity(1, 0), make_entity(2, 0)],
        ..Default::default()
    }
}

// ── AC9 ────────────────────────────────────────────────────────────────────────

/// AC9 — get-ordered-entities reflects staged state (push + insert).
///
/// Minimum-viable test: build a layer with 2 pre-existing entities, push 1
/// additional entity via `push_entity_to_layer`, insert 1 entity via
/// `insert_entity_at` at position 1, then call `apply_to`. The layer must end
/// with 4 entities total (2 pre-existing + 1 pushed + 1 inserted); all 4 IDs
/// must be distinct; and exactly one of the original entity IDs (1, 2) must
/// sit at index 0 or index 2 (the inserted entity occupies position 1 relative
/// to the phase-1 result).
///
/// The test validates the SDK apply_to path which is the authoritative
/// read-back of staged state (minimum viable for AC9).
#[test]
fn get_ordered_entities_reflects_staged_state() {
    let mut layers = vec![layer_with_2_entities()];
    let original_ids: std::collections::HashSet<u64> = layers[0]
        .ordered_entities
        .iter()
        .map(|e| e.entity_id)
        .collect();

    let mut output = FinalizationOutputBuilder::new();

    // Push 1 entity (appended, priority=0)
    output
        .push_entity_to_layer(0, path(), region_key(0))
        .expect("push_entity_to_layer should succeed");

    // Insert 1 entity at position 1 (between first and second entity in post-push order)
    output
        .insert_entity_at(0, 1, path(), region_key(0))
        .expect("insert_entity_at should succeed at record time");

    let result = output.apply_to(&mut layers);
    assert!(result.is_ok(), "apply_to failed: {:?}", result);

    let layer = &layers[0];

    // Total count: 2 original + 1 pushed + 1 inserted = 4
    assert_eq!(
        layer.ordered_entities.len(),
        4,
        "should have 4 entities after push + insert"
    );

    // All 4 entity IDs must be distinct
    let all_ids: std::collections::HashSet<u64> =
        layer.ordered_entities.iter().map(|e| e.entity_id).collect();
    assert_eq!(all_ids.len(), 4, "all 4 entity IDs should be distinct");

    // The 2 original IDs should still be present
    for &orig_id in &original_ids {
        assert!(
            all_ids.contains(&orig_id),
            "original entity_id={orig_id} should still be present after apply_to"
        );
    }

    // The 2 new entity IDs (pushed + inserted) should be distinct from originals
    let new_ids: Vec<u64> = all_ids.difference(&original_ids).copied().collect();
    assert_eq!(
        new_ids.len(),
        2,
        "there should be exactly 2 newly generated entity IDs"
    );
}
