//! TDD tests for `enumerate_canonical_chains` (P93 Step 2, AC-3).

use slicer_ir::region_split_registry::enumerate_canonical_chains;
use slicer_ir::PaintValue;
use std::collections::{HashMap, HashSet};

#[test]
fn enumerate_canonical_chains_empty_input_returns_single_empty_chain() {
    let variants: HashMap<String, Vec<PaintValue>> = HashMap::new();
    let canonical_order: Vec<String> = Vec::new();
    let result = enumerate_canonical_chains(&variants, &canonical_order);
    assert_eq!(result.len(), 1);
    assert!(result[0].is_empty());
}

#[test]
fn enumerate_canonical_chains_one_semantic_four_values_returns_five_chains() {
    let mut variants: HashMap<String, Vec<PaintValue>> = HashMap::new();
    variants.insert(
        "material".to_string(),
        vec![
            PaintValue::ToolIndex(1),
            PaintValue::ToolIndex(2),
            PaintValue::ToolIndex(3),
            PaintValue::ToolIndex(4),
        ],
    );
    let canonical_order = vec!["material".to_string()];
    let result = enumerate_canonical_chains(&variants, &canonical_order);

    assert_eq!(result.len(), 5);
    assert_eq!(result[0], Vec::<(String, PaintValue)>::new());
    for (i, tool) in [1u32, 2, 3, 4].iter().enumerate() {
        assert_eq!(
            result[i + 1],
            vec![("material".to_string(), PaintValue::ToolIndex(*tool))]
        );
    }
}

#[test]
fn enumerate_canonical_chains_two_semantics_2x1_returns_six_chains_in_canonical_order() {
    let mut variants: HashMap<String, Vec<PaintValue>> = HashMap::new();
    variants.insert(
        "material".to_string(),
        vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
    );
    variants.insert("fuzzy_skin".to_string(), vec![PaintValue::Flag(true)]);
    let canonical_order = vec!["material".to_string(), "fuzzy_skin".to_string()];
    let result = enumerate_canonical_chains(&variants, &canonical_order);

    assert_eq!(result.len(), 6);
    assert_eq!(result[0], Vec::<(String, PaintValue)>::new());

    let expected: HashSet<Vec<(String, PaintValue)>> = [
        vec![],
        vec![("material".to_string(), PaintValue::ToolIndex(1))],
        vec![("material".to_string(), PaintValue::ToolIndex(2))],
        vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))],
        vec![
            ("material".to_string(), PaintValue::ToolIndex(1)),
            ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
        ],
        vec![
            ("material".to_string(), PaintValue::ToolIndex(2)),
            ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
        ],
    ]
    .into_iter()
    .collect();
    let actual: HashSet<Vec<(String, PaintValue)>> = result.iter().cloned().collect();
    assert_eq!(actual, expected);

    // Within each chain, pairs must appear in canonical-order sequence.
    for chain in &result {
        let indices: Vec<usize> = chain
            .iter()
            .map(|(sem, _)| canonical_order.iter().position(|s| s == sem).unwrap())
            .collect();
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }
}
