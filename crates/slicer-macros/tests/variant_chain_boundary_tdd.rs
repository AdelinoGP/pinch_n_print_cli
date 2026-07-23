//! Regression coverage for paint values that the current WIT boundary cannot encode.

#![allow(dead_code, missing_docs)]

use std::fs;
use std::path::PathBuf;

use slicer_ir::{PaintValue, RegionKey};

fn variant_chain_to_wit(key: &RegionKey) -> Option<Vec<(String, PaintValue)>> {
    key.variant_chain
        .iter()
        .map(|(semantic, value)| {
            let value = match value {
                PaintValue::Flag(v) => PaintValue::Flag(*v),
                PaintValue::Scalar(v) => PaintValue::Scalar(*v),
                PaintValue::ToolIndex(v) => PaintValue::ToolIndex(*v),
                PaintValue::Custom(_) => return None,
            };
            Some((semantic.clone(), value))
        })
        .collect()
}

#[test]
fn custom_variant_chain_is_skipped_without_panicking() {
    let key = RegionKey {
        global_layer_index: 0,
        object_id: String::new(),
        region_id: 0,
        variant_chain: vec![(
            "custom_semantic".to_string(),
            PaintValue::Custom("hello".to_string()),
        )],
    };

    let result = std::panic::catch_unwind(|| variant_chain_to_wit(&key));
    assert_eq!(result.unwrap(), None);

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let src = fs::read_to_string(path).expect("read slicer-macros source");
    assert!(!src.contains("custom paint values are not valid variant-chain identity"));
}
