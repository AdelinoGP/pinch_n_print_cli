//! TDD tests for paint-segmentation prepass module.

use slicer_sdk::prelude::*;
use slicer_sdk::prepass_builders::PaintValueInput;

use paint_segmentation::PaintSegmentation;

fn empty_config() -> ConfigView {
    ConfigView::new()
}

/// Identity 4x4 column-major matrix.
fn identity_matrix() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, // col 0
        0.0, 1.0, 0.0, 0.0, // col 1
        0.0, 0.0, 1.0, 0.0, // col 2
        0.0, 0.0, 0.0, 1.0, // col 3
    ]
}

/// Translation matrix: translate by (tx, ty, 0) in column-major order.
fn translation_matrix(tx: f64, ty: f64) -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, // col 0
        0.0, 1.0, 0.0, 0.0, // col 1
        0.0, 0.0, 1.0, 0.0, // col 2
        tx, ty, 0.0, 1.0, // col 3
    ]
}

/// Helper: make a PaintValueView with kind="flag" and flag=true.
fn flag_value(val: bool) -> PaintValueView {
    PaintValueView {
        kind: "flag".to_string(),
        flag: Some(val),
        scalar: None,
        tool_index: None,
    }
}

/// Helper: make a PaintValueView with kind="tool_index".
fn tool_index_value(idx: u32) -> PaintValueView {
    PaintValueView {
        kind: "tool_index".to_string(),
        flag: None,
        scalar: None,
        tool_index: Some(idx),
    }
}

/// Helper: scalar paint value.
fn scalar_value(val: f32) -> PaintValueView {
    PaintValueView {
        kind: "scalar".to_string(),
        flag: None,
        scalar: Some(val),
        tool_index: None,
    }
}

/// Single triangle object with one painted facet and one participating layer.
fn single_painted_object(
    object_id: &str,
    paint_layers: Vec<PaintLayerView>,
    transform: [f64; 16],
    participating: Vec<u32>,
) -> PaintSegmentationObjectView {
    PaintSegmentationObjectView {
        object_id: object_id.to_string(),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        paint_layers,
        transform_matrix: transform,
        participating_layer_indices: participating,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn on_print_start_defaults() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config);
    assert!(module.is_ok());
}

#[test]
fn no_paint_data_passthrough() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects = vec![PaintSegmentationObjectView {
        object_id: "obj1".to_string(),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        paint_layers: vec![],
        transform_matrix: identity_matrix(),
        participating_layer_indices: vec![0],
    }];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert!(
        output.regions().is_empty(),
        "no regions should be emitted when there are no paint layers"
    );
}

#[test]
fn single_facet_single_layer() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects = vec![single_painted_object(
        "obj1",
        vec![PaintLayerView {
            semantic: "support_enforcer".to_string(),
            facet_values: vec![Some(flag_value(true))],
            strokes: vec![],
        }],
        identity_matrix(),
        vec![0],
    )];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(output.regions().len(), 1, "should produce exactly 1 region");
    let region = &output.regions()[0];
    assert_eq!(region.layer_index, 0);
    assert_eq!(region.semantic, "support_enforcer");
    assert_eq!(region.object_id, "obj1");
    assert_eq!(region.value, PaintValueInput::Flag(true));
    assert_eq!(region.paint_order, 0);
    assert_eq!(
        region.polygons[0].contour.len(),
        3,
        "triangle has 3 vertices"
    );
}

#[test]
fn projects_through_transform() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();

    // Identity matrix: vertices stay at (0,0), (1,0), (0,1)
    let objects_identity = vec![single_painted_object(
        "obj1",
        vec![PaintLayerView {
            semantic: "material".to_string(),
            facet_values: vec![Some(flag_value(true))],
            strokes: vec![],
        }],
        identity_matrix(),
        vec![0],
    )];
    let mut output_identity = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects_identity, &mut output_identity, &config)
        .unwrap();

    // Translation matrix: shift by (10, 20)
    let objects_translated = vec![single_painted_object(
        "obj1",
        vec![PaintLayerView {
            semantic: "material".to_string(),
            facet_values: vec![Some(flag_value(true))],
            strokes: vec![],
        }],
        translation_matrix(10.0, 20.0),
        vec![0],
    )];
    let mut output_translated = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects_translated, &mut output_translated, &config)
        .unwrap();

    let id_pts = &output_identity.regions()[0].polygons[0].contour;
    let tr_pts = &output_translated.regions()[0].polygons[0].contour;

    // Each translated point should be offset by (10, 20) from the identity point
    for i in 0..3 {
        let dx = tr_pts[i][0] - id_pts[i][0];
        let dy = tr_pts[i][1] - id_pts[i][1];
        assert!(
            (dx - 10.0).abs() < 1e-6,
            "X offset should be 10.0, got {dx}"
        );
        assert!(
            (dy - 20.0).abs() < 1e-6,
            "Y offset should be 20.0, got {dy}"
        );
    }
}

#[test]
fn multiple_semantics() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects = vec![single_painted_object(
        "obj1",
        vec![
            PaintLayerView {
                semantic: "material".to_string(),
                facet_values: vec![Some(flag_value(true))],
                strokes: vec![],
            },
            PaintLayerView {
                semantic: "fuzzy_skin".to_string(),
                facet_values: vec![Some(scalar_value(0.5))],
                strokes: vec![],
            },
        ],
        identity_matrix(),
        vec![0],
    )];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(
        output.regions().len(),
        2,
        "should produce 2 regions for 2 semantics"
    );

    let semantics: Vec<&str> = output
        .regions()
        .iter()
        .map(|r| r.semantic.as_str())
        .collect();
    assert!(semantics.contains(&"material"));
    assert!(semantics.contains(&"fuzzy_skin"));
}

#[test]
fn paint_order_preserved() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects = vec![single_painted_object(
        "obj1",
        vec![
            PaintLayerView {
                semantic: "material".to_string(),
                facet_values: vec![Some(flag_value(true))],
                strokes: vec![],
            },
            PaintLayerView {
                semantic: "tool".to_string(),
                facet_values: vec![Some(tool_index_value(1))],
                strokes: vec![],
            },
        ],
        identity_matrix(),
        vec![0],
    )];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(output.regions().len(), 2);
    // Paint order should be 0 for first layer, 1 for second
    let orders: Vec<u64> = output.regions().iter().map(|r| r.paint_order).collect();
    assert!(orders.contains(&0));
    assert!(orders.contains(&1));
}

#[test]
fn groups_same_value() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    // Two-triangle object, both painted with same value in same layer
    let objects = vec![PaintSegmentationObjectView {
        object_id: "obj1".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [1, 3, 2]],
        paint_layers: vec![PaintLayerView {
            semantic: "material".to_string(),
            facet_values: vec![Some(flag_value(true)), Some(flag_value(true))],
            strokes: vec![],
        }],
        transform_matrix: identity_matrix(),
        participating_layer_indices: vec![0],
    }];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    // Both facets have same (semantic, value, object_id, paint_order) so they
    // should be grouped — but our simple builder just collects entries.
    // The module emits 2 entries (one per facet), both with same grouping key.
    // Grouping is done downstream, so we verify the entries share the same key.
    assert_eq!(output.regions().len(), 2);
    assert_eq!(output.regions()[0].semantic, output.regions()[1].semantic);
    assert_eq!(output.regions()[0].value, output.regions()[1].value);
    assert_eq!(
        output.regions()[0].paint_order,
        output.regions()[1].paint_order
    );
    assert_eq!(output.regions()[0].object_id, output.regions()[1].object_id);
}

#[test]
fn empty_objects_no_output() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects: Vec<PaintSegmentationObjectView> = vec![];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert!(output.regions().is_empty());
}

#[test]
fn multi_layer_participation() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects = vec![single_painted_object(
        "obj1",
        vec![PaintLayerView {
            semantic: "support_enforcer".to_string(),
            facet_values: vec![Some(flag_value(true))],
            strokes: vec![],
        }],
        identity_matrix(),
        vec![0, 1, 2],
    )];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(
        output.regions().len(),
        3,
        "should produce 1 region at each of 3 participating layers"
    );
    let layer_indices: Vec<u32> = output.regions().iter().map(|r| r.layer_index).collect();
    assert!(layer_indices.contains(&0));
    assert!(layer_indices.contains(&1));
    assert!(layer_indices.contains(&2));
}

#[test]
fn unpainted_facets_skipped() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    let objects = vec![PaintSegmentationObjectView {
        object_id: "obj1".to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [1, 3, 2]],
        paint_layers: vec![PaintLayerView {
            semantic: "material".to_string(),
            facet_values: vec![Some(flag_value(true)), None],
            strokes: vec![],
        }],
        transform_matrix: identity_matrix(),
        participating_layer_indices: vec![0],
    }];
    let mut output = PaintSegmentationOutput::new();
    module
        .run_paint_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(
        output.regions().len(),
        1,
        "only the painted facet should produce a region"
    );
}

#[test]
fn facet_values_length_mismatch() {
    let config = empty_config();
    let module = PaintSegmentation::on_print_start(&config).unwrap();
    // Object has 1 triangle but facet_values has 2 entries
    let objects = vec![single_painted_object(
        "obj1",
        vec![PaintLayerView {
            semantic: "material".to_string(),
            facet_values: vec![Some(flag_value(true)), Some(flag_value(false))],
            strokes: vec![],
        }],
        identity_matrix(),
        vec![0],
    )];
    let mut output = PaintSegmentationOutput::new();
    let result = module.run_paint_segmentation(&objects, &mut output, &config);
    assert!(
        result.is_err(),
        "should return error on facet_values length mismatch"
    );
}
