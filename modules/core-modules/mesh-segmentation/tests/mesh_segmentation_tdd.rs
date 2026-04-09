//! TDD tests for mesh-segmentation prepass module.

use slicer_sdk::prelude::*;
use std::collections::HashMap;

use mesh_segmentation::MeshSegmentation;

fn empty_config() -> ConfigView {
    ConfigView {
        fields: HashMap::new(),
    }
}

/// Helper: single triangle at z=0 with vertices (0,0,0), (1,0,0), (0,1,0).
fn single_triangle_object(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        paint_layers: vec![],
    }
}

/// Helper: single triangle with one paint layer that has no strokes.
fn single_triangle_no_strokes(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        triangles: vec![[0, 1, 2]],
        paint_layers: vec![PaintLayerView {
            semantic: "support_enforcer".to_string(),
            facet_values: vec![None],
            strokes: vec![],
        }],
    }
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

/// Helper: make a stroke whose centroid lies inside the single triangle.
fn stroke_inside_triangle(value: PaintValueView) -> PaintStrokeView {
    // Centroid of this stroke triangle is roughly (0.2, 0.2, 0.0), inside the mesh triangle.
    PaintStrokeView {
        triangles: vec![[[0.1, 0.1, 0.0], [0.3, 0.1, 0.0], [0.2, 0.4, 0.0]]],
        semantic: "support_enforcer".to_string(),
        value,
    }
}

/// Helper: make a stroke whose centroid is outside all mesh triangles.
fn stroke_outside_triangle(value: PaintValueView) -> PaintStrokeView {
    // Centroid of this stroke is at roughly (5.0, 5.0, 0.0), well outside the mesh triangle.
    PaintStrokeView {
        triangles: vec![[[4.5, 4.5, 0.0], [5.5, 4.5, 0.0], [5.0, 5.5, 0.0]]],
        semantic: "support_enforcer".to_string(),
        value,
    }
}

/// Two-triangle mesh: (0,0,0)-(1,0,0)-(0,1,0) and (1,0,0)-(1,1,0)-(0,1,0).
fn two_triangle_object(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [1, 3, 2]],
        paint_layers: vec![],
    }
}

#[test]
fn on_print_start_defaults() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config);
    assert!(module.is_ok());
}

#[test]
fn no_strokes_passthrough() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let objects = vec![single_triangle_no_strokes("obj1")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert!(
        output.modifications().is_empty(),
        "no modifications should be pushed when there are no strokes"
    );
}

#[test]
fn single_stroke_assigns_facet() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = single_triangle_object("obj1");
    obj.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_inside_triangle(flag_value(true))],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(output.modifications().len(), 1);
    let m = &output.modifications()[0];
    assert_eq!(m.object_id, "obj1");
    // The facet should now have a paint value assigned
    assert!(
        m.updated_facet_values[0][0].is_some(),
        "facet 0 should have paint value after stroke normalization"
    );
    let pv = m.updated_facet_values[0][0].as_ref().unwrap();
    assert_eq!(pv.kind, "flag");
    assert_eq!(pv.flag, Some(true));
}

#[test]
fn facet_values_updated_tool_index() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = single_triangle_object("obj1");
    obj.paint_layers.push(PaintLayerView {
        semantic: "tool".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_inside_triangle(tool_index_value(1))],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(output.modifications().len(), 1);
    let pv = output.modifications()[0].updated_facet_values[0][0]
        .as_ref()
        .unwrap();
    assert_eq!(pv.kind, "tool_index");
    assert_eq!(pv.tool_index, Some(1));
}

#[test]
fn strokes_cleared_after_process() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = single_triangle_object("obj1");
    obj.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_inside_triangle(flag_value(true))],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert!(output.modifications()[0].strokes_cleared);
}

#[test]
fn multiple_paint_layers() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = single_triangle_object("obj1");
    obj.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_inside_triangle(flag_value(true))],
    });
    obj.paint_layers.push(PaintLayerView {
        semantic: "tool".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_inside_triangle(tool_index_value(2))],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(output.modifications().len(), 1);
    let m = &output.modifications()[0];
    assert_eq!(m.updated_facet_values.len(), 2);
    // Layer 0: flag
    assert_eq!(
        m.updated_facet_values[0][0].as_ref().unwrap().kind,
        "flag"
    );
    // Layer 1: tool_index
    assert_eq!(
        m.updated_facet_values[1][0].as_ref().unwrap().kind,
        "tool_index"
    );
}

#[test]
fn no_matching_facet_skipped() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = single_triangle_object("obj1");
    obj.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_outside_triangle(flag_value(true))],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    // The module still produces a modification (strokes were present), but facet values stay None
    assert_eq!(output.modifications().len(), 1);
    let m = &output.modifications()[0];
    assert!(
        m.updated_facet_values[0][0].is_none(),
        "facet value should remain None when stroke doesn't match any facet"
    );
}

#[test]
fn empty_objects_no_output() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let objects: Vec<MeshObjectView> = vec![];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert!(output.modifications().is_empty());
}

#[test]
fn preserves_unstroked_facets() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = two_triangle_object("obj1");
    let existing_val = Some(flag_value(false));
    obj.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![existing_val.clone(), existing_val.clone()],
        // Stroke covers only the first triangle (centroid ~(0.2,0.2) is in tri 0)
        strokes: vec![stroke_inside_triangle(flag_value(true))],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    let m = &output.modifications()[0];
    // First facet should be updated
    assert_eq!(m.updated_facet_values[0][0].as_ref().unwrap().flag, Some(true));
    // Second facet should preserve its original value
    assert_eq!(
        m.updated_facet_values[0][1].as_ref().unwrap().flag,
        Some(false),
        "unstroked facet should preserve original paint value"
    );
}

#[test]
fn multiple_objects() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let obj1 = single_triangle_no_strokes("obj1");
    let mut obj2 = single_triangle_object("obj2");
    obj2.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![None],
        strokes: vec![stroke_inside_triangle(flag_value(true))],
    });
    let objects = vec![obj1, obj2];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    // Only obj2 has strokes, so only 1 modification
    assert_eq!(output.modifications().len(), 1);
    assert_eq!(output.modifications()[0].object_id, "obj2");
}

#[test]
fn large_stroke_assigns_multiple() {
    let config = empty_config();
    let module = MeshSegmentation::on_print_start(&config).unwrap();
    let mut obj = two_triangle_object("obj1");
    // A large stroke that covers the centroids of both triangles
    // Tri 0 centroid: (0.33, 0.33, 0), Tri 1 centroid: (0.67, 0.67, 0)
    let big_stroke = PaintStrokeView {
        triangles: vec![
            // Stroke tri covering first mesh tri centroid
            [[0.1, 0.1, 0.0], [0.5, 0.1, 0.0], [0.2, 0.5, 0.0]],
            // Stroke tri covering second mesh tri centroid
            [[0.5, 0.5, 0.0], [0.9, 0.5, 0.0], [0.7, 0.9, 0.0]],
        ],
        semantic: "support_enforcer".to_string(),
        value: flag_value(true),
    };
    obj.paint_layers.push(PaintLayerView {
        semantic: "support_enforcer".to_string(),
        facet_values: vec![None, None],
        strokes: vec![big_stroke],
    });
    let objects = vec![obj];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &config)
        .unwrap();
    assert_eq!(output.modifications().len(), 1);
    let m = &output.modifications()[0];
    // Both facets should be updated
    assert!(m.updated_facet_values[0][0].is_some(), "facet 0 should be painted");
    assert!(m.updated_facet_values[0][1].is_some(), "facet 1 should be painted");
}
