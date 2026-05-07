//! Validation helpers for IR schemas.

use crate::LayerCollectionIR;
use std::collections::HashSet;

/// Validates that all `TravelMove` entries in a layer reference entity IDs that exist
/// in `layer.ordered_entities`.
///
/// Returns `Err(String)` if a dangling travel anchor is found (travel's entity_id not in layer).
/// The error message includes the offending entity_id.
pub fn validate_travel_anchors(layer: &LayerCollectionIR) -> Result<(), String> {
    // Build a set of valid entity IDs from ordered_entities
    let valid_ids: HashSet<u64> = layer
        .ordered_entities
        .iter()
        .map(|entity| entity.entity_id)
        .collect();

    // Check each travel move
    for travel in &layer.travel_moves {
        if !valid_ids.contains(&travel.entity_id) {
            return Err(format!(
                "dangling travel anchor: TravelMove.entity_id = {} not present in layer.ordered_entities",
                travel.entity_id
            ));
        }
    }

    Ok(())
}
