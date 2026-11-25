//! Independent test-only membership oracle for the travel-anchor charter.

use std::fmt;

use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_ir::LayerCollectionIR;

/// The first missing anchor, in travel order rather than numeric ID order.
#[derive(Debug, PartialEq, Eq)]
pub struct AnchorOracleError {
    /// ID absent from the layer's ordered entities.
    pub entity_id: u64,
}

impl fmt::Display for AnchorOracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "first missing travel anchor entity_id = {}",
            self.entity_id
        )
    }
}

/// Determine membership using only the IDs in `layer.ordered_entities`.
///
/// Oracle independence: sorted-vector binary search derives the answer directly
/// from the set-membership contract. It neither calls the production validator
/// nor shares its hash-set implementation or consults other entity carriers.
pub fn travel_anchor_oracle(layer: &LayerCollectionIR) -> Result<(), AnchorOracleError> {
    let mut allowed: Vec<u64> = layer
        .ordered_entities
        .iter()
        .map(|entity| entity.entity_id)
        .collect();
    allowed.sort_unstable();
    allowed.dedup();
    for travel in &layer.travel_moves {
        if allowed.binary_search(&travel.entity_id).is_err() {
            return Err(AnchorOracleError {
                entity_id: travel.entity_id,
            });
        }
    }
    Ok(())
}

/// Compare a validator against the independent oracle and diagnostic charter.
///
/// Passing the validator as a callback permits negative controls without
/// replacing the oracle or registering a synthetic production property.
pub fn check_travel_anchors_charter<F>(layer: &LayerCollectionIR, validator: F) -> TestCaseResult
where
    F: Fn(&LayerCollectionIR) -> Result<(), String>,
{
    let expected = travel_anchor_oracle(layer);
    let actual = validator(layer);
    match (expected, actual) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(message)) => Err(TestCaseError::fail(format!(
            "all travel anchors belong to layer.ordered_entities, but validator rejected: {message}"
        ))),
        (Err(missing), Ok(())) => Err(TestCaseError::fail(format!(
            "validator accepted missing TravelMove.entity_id = {} from layer.ordered_entities",
            missing.entity_id
        ))),
        (Err(missing), Err(message)) => {
            let id = missing.entity_id.to_string();
            let names_field = message
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .any(|token| token == "entity_id");
            let names_id = message
                .split(|c: char| !c.is_ascii_digit())
                .any(|token| token == id);
            if names_field && names_id && message.contains("layer.ordered_entities") {
                Ok(())
            } else {
                Err(TestCaseError::fail(format!(
                    "expected diagnostic naming entity_id, first missing decimal token {id}, and layer.ordered_entities; received: {message}"
                )))
            }
        }
    }
}
