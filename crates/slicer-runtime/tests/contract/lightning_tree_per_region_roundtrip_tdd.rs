//! Packet 139 AC-N3: lightning tree segments stay isolated by region.

#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{LightningTreeEntry, LightningTreeIR, Point2};
use slicer_sdk::PaintRegionLayerView;

#[test]
fn lightning_tree_segments_roundtrip_isolated_by_region() {
    let region_one_segment = [Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 10 }];
    let region_two_segment = [Point2 { x: 100, y: 100 }, Point2 { x: 110, y: 110 }];
    let ir = Arc::new(LightningTreeIR {
        entries: vec![
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 1,
                tree_edge_segments: vec![region_one_segment],
            },
            LightningTreeEntry {
                object_id: "obj-1".to_string(),
                global_layer_index: 5,
                region_id: 2,
                tree_edge_segments: vec![region_two_segment],
            },
        ],
        ..LightningTreeIR::default()
    });
    let view = PaintRegionLayerView::new(5).with_lightning_tree_ir(ir);

    let region_one = view.lightning_tree_segments_for("obj-1", 1);
    assert_eq!(region_one.len(), 1);
    assert_eq!(region_one[0], region_one_segment);

    let region_two = view.lightning_tree_segments_for("obj-1", 2);
    assert_eq!(region_two.len(), 1);
    assert_eq!(region_two[0], region_two_segment);
}
