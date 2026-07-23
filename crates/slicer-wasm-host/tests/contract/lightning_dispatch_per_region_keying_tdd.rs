//! Contract coverage for dispatch-side lightning segment keying.

#![allow(missing_docs)]

use std::collections::BTreeSet;

use slicer_ir::{LightningTreeEntry, LightningTreeIR, Point2};
use slicer_sdk::PaintRegionLayerView;
use slicer_wasm_host::dispatch::build_paint_layer_data_for_test;

fn lightning_tree_ir_with_two_regions() -> LightningTreeIR {
    LightningTreeIR {
        entries: vec![
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 1,
                tree_edge_segments: vec![[Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }]],
            },
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 2,
                tree_edge_segments: vec![[Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }]],
            },
        ],
        ..LightningTreeIR::default()
    }
}

#[test]
fn lightning_dispatch_per_region_keying() {
    let ir = lightning_tree_ir_with_two_regions();
    let data = build_paint_layer_data_for_test(5, &ir);

    assert_eq!(data.lightning_tree_segments.len(), 2);
    assert!(data
        .lightning_tree_segments
        .contains_key(&(String::from("obj-1"), String::from("1"))));
    assert!(data
        .lightning_tree_segments
        .contains_key(&(String::from("obj-1"), String::from("2"))));
    assert!(!data
        .lightning_tree_segments
        .contains_key(&(String::from("obj-1"), String::from("*"))));

    let region_one = data
        .lightning_tree_segments
        .get(&(String::from("obj-1"), String::from("1")))
        .expect("region one dispatch bucket");
    let region_two = data
        .lightning_tree_segments
        .get(&(String::from("obj-1"), String::from("2")))
        .expect("region two dispatch bucket");
    assert_eq!(region_one.len(), 1);
    assert_eq!(region_two.len(), 1);
    assert_eq!(region_one[0][0].x, 0.0);
    assert_eq!(region_one[0][1].x, 0.001);
    assert_eq!(region_two[0][0].x, 0.01);

    let repeated_data = build_paint_layer_data_for_test(5, &ir);
    assert_eq!(
        data.lightning_tree_segments
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        repeated_data
            .lightning_tree_segments
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
    );

    let view = PaintRegionLayerView::new(5).with_lightning_tree_ir(std::sync::Arc::new(ir));
    let region_one_segments = view.lightning_tree_segments_for("obj-1", 1);
    let region_two_segments = view.lightning_tree_segments_for("obj-1", 2);
    assert_eq!(
        region_one_segments,
        vec![[Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }]]
    );
    assert_eq!(
        region_two_segments,
        vec![[Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }]]
    );
    assert_eq!(
        view.lightning_tree_segments_for("obj-1", 1),
        region_one_segments
    );
    assert_eq!(
        view.lightning_tree_segments_for("obj-1", 2),
        region_two_segments
    );
}
