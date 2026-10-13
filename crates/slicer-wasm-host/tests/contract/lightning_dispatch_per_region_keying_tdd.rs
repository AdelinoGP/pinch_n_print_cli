//! Contract coverage for per-origin lightning segment keying.
//!
//! The dispatch-built `lightning-query-view` resource (W7a split) is keyed by
//! the FULL origin `(object_id, region_id, variant_chain)`; its construction
//! is not publicly observable in-crate, so the module-facing layer view below
//! asserts the same keying. The rebuilt NEW-contract guest round-trip (W14)
//! covers the full dispatch path including the units→mm projection.

#![allow(missing_docs)]

use slicer_ir::{LightningTreeEntry, LightningTreeIR, PaintValue, Point2};
use slicer_sdk::PaintRegionLayerView;

fn chain(tool: u32) -> Vec<(String, PaintValue)> {
    vec![("material".into(), PaintValue::ToolIndex(tool))]
}

fn lightning_tree_ir_with_two_regions() -> LightningTreeIR {
    LightningTreeIR {
        entries: vec![
            // exhaustive: dispatch identity fixture pins every lightning entry field.
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 1,
                tree_edge_segments: vec![[Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }]],
                variant_chain: chain(1),
            },
            // exhaustive: dispatch identity fixture pins every lightning entry field.
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 2,
                tree_edge_segments: vec![[Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }]],
                variant_chain: chain(2),
            },
        ],
        ..LightningTreeIR::default()
    }
}

#[test]
fn lightning_dispatch_per_region_keying() {
    let ir = lightning_tree_ir_with_two_regions();
    let view = PaintRegionLayerView::new(5).with_lightning_tree_ir(std::sync::Arc::new(ir));

    // Exact per-origin keying: each region sees exactly its own segment set
    // through the full `(layer, object_id, region_id, variant_chain)` identity.
    let region_one_segments = view.lightning_tree_segments_for("obj-1", 1, &chain(1));
    let region_two_segments = view.lightning_tree_segments_for("obj-1", 2, &chain(2));
    assert_eq!(
        region_one_segments,
        vec![[Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }]]
    );
    assert_eq!(
        region_two_segments,
        vec![[Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }]]
    );

    // No wildcard/pair fallback: a wrong chain or an unknown region sees
    // nothing even though the pair `(object, region)` partially matches.
    assert!(
        view.lightning_tree_segments_for("obj-1", 1, &chain(2))
            .is_empty(),
        "a pair-only lookup would return region one's segments"
    );
    assert!(
        view.lightning_tree_segments_for("obj-1", 9, &chain(1))
            .is_empty(),
        "an unknown region cannot borrow either bucket"
    );

    // Rebuilding the view from the same IR yields identical keying.
    let repeated_view = PaintRegionLayerView::new(5)
        .with_lightning_tree_ir(std::sync::Arc::new(lightning_tree_ir_with_two_regions()));
    assert_eq!(
        repeated_view.lightning_tree_segments_for("obj-1", 1, &chain(1)),
        region_one_segments
    );
    assert_eq!(
        repeated_view.lightning_tree_segments_for("obj-1", 2, &chain(2)),
        region_two_segments
    );

    // Repeat queries are stable.
    assert_eq!(
        view.lightning_tree_segments_for("obj-1", 1, &chain(1)),
        region_one_segments
    );
    assert_eq!(
        view.lightning_tree_segments_for("obj-1", 2, &chain(2)),
        region_two_segments
    );
}

#[test]
fn same_number_sources_discriminate_chains() {
    // Two sources sharing object, layer and region number, separated only by
    // their ordered chains: a pair-only lookup would return both segment sets
    // for either query, so each exact-chain query must see exactly its own.
    let ir = LightningTreeIR {
        entries: vec![
            // exhaustive: same-number identity fixture pins every lightning entry field.
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 1,
                tree_edge_segments: vec![[Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }]],
                variant_chain: chain(1),
            },
            // exhaustive: same-number identity fixture pins every lightning entry field.
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 1,
                tree_edge_segments: vec![[Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }]],
                variant_chain: chain(2),
            },
        ],
        ..LightningTreeIR::default()
    };
    let view = PaintRegionLayerView::new(5).with_lightning_tree_ir(std::sync::Arc::new(ir));
    assert_eq!(
        view.lightning_tree_segments_for("obj-1", 1, &chain(1)),
        vec![[Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }]],
        "chain one keeps only its own segments"
    );
    assert_eq!(
        view.lightning_tree_segments_for("obj-1", 1, &chain(2)),
        vec![[Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }]],
        "chain two keeps only its own segments"
    );
}
