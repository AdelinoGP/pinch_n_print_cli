//! Packet 137: `lightning-tree-segments` WIT read-view roundtrip (AC-4).
//!
//! Host commits a fixture `LightningTreeIR` → SDK accessor returns the
//! committed segments with matching count and endpoint equality. A
//! `Layer::Infill` guest calls the same accessor through the WIT method
//! `lightning-tree-segments`; this test exercises the accessor shape and
//! the per-layer filter, which is the contract the guest depends on.

#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{LightningTreeEntry, LightningTreeIR, Point2};
use slicer_sdk::PaintRegionLayerView;

fn fixture_entry(
    object_id: &str,
    layer_index: i32,
    region_id: u64,
    pts: &[(Point2, Point2)],
) -> LightningTreeEntry {
    let segments = pts.iter().map(|(a, b)| [*a, *b]).collect::<Vec<_>>();
    LightningTreeEntry {
        object_id: object_id.to_string(),
        global_layer_index: layer_index,
        region_id,
        tree_edge_segments: segments,
    }
}

#[test]
fn sdk_accessor_returns_host_committed_segments_count_and_endpoints() {
    let p00 = Point2 { x: 0, y: 0 };
    let p11 = Point2 { x: 10, y: 10 };
    let p22 = Point2 { x: 20, y: 20 };
    let p33 = Point2 { x: 30, y: 30 };
    let p44 = Point2 { x: 40, y: 40 };
    let p55 = Point2 { x: 50, y: 50 };

    let ir = Arc::new(LightningTreeIR {
        entries: vec![
            fixture_entry("cube", 7, 0, &[(p00, p11), (p11, p22)]),
            fixture_entry("cube", 7, 0, &[(p33, p44)]),
            fixture_entry("cube", 8, 0, &[(p00, p55)]),
            fixture_entry("other", 7, 0, &[(p22, p33)]),
            fixture_entry("cube", 7, 1, &[(p44, p55)]),
        ],
        ..LightningTreeIR::default()
    });

    let view = PaintRegionLayerView::new(7).with_lightning_tree_ir(ir);

    let segs = view.lightning_tree_segments_for("cube", 0);
    assert_eq!(segs.len(), 3, "layer 7 + object cube must yield 3 segments");
    assert_eq!(segs[0], [p00, p11]);
    assert_eq!(segs[1], [p11, p22]);
    assert_eq!(segs[2], [p33, p44]);

    let region_one = view.lightning_tree_segments_for("cube", 1);
    assert_eq!(region_one, vec![[p44, p55]]);

    let segs_other = view.lightning_tree_segments_for("other", 0);
    assert_eq!(segs_other.len(), 1);
    assert_eq!(segs_other[0], [p22, p33]);

    let view_l8 = PaintRegionLayerView::new(8);
    let _ = view_l8;
}

#[test]
fn sdk_accessor_returns_empty_when_no_ir_attached() {
    let view = PaintRegionLayerView::new(0);
    assert!(view.lightning_tree_ir().is_none());
    assert!(view.lightning_tree_segments_for("cube", 0).is_empty());
}

#[test]
fn sdk_accessor_filters_by_layer_index() {
    let ir = Arc::new(LightningTreeIR {
        entries: vec![
            fixture_entry(
                "cube",
                5,
                0,
                &[(Point2 { x: 1, y: 1 }, Point2 { x: 2, y: 2 })],
            ),
            fixture_entry(
                "cube",
                6,
                0,
                &[(Point2 { x: 3, y: 3 }, Point2 { x: 4, y: 4 })],
            ),
        ],
        ..LightningTreeIR::default()
    });
    let view = PaintRegionLayerView::new(6).with_lightning_tree_ir(ir);
    let segs = view.lightning_tree_segments_for("cube", 0);
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0][0], Point2 { x: 3, y: 3 });
    assert_eq!(segs[0][1], Point2 { x: 4, y: 4 });
}
