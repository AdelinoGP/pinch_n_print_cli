//! TDD tests for the `mesh-segmentation` prepass module after STEP H.
//!
//! The module is now config-driven: it parses `mesh_seg_mark:*` entries
//! from the declared `ConfigView` and emits one per-triangle paint mark
//! for each, then drains them through the SDK
//! `MeshSegmentationOutput::mark_triangle_paint` API (matched one-to-one
//! to the WIT `mesh-segmentation-output::mark-triangle-paint` method by
//! the `#[slicer_module]` macro arm). The previous stroke-based
//! barycentric resolution algorithm was removed because its access
//! pattern (`MeshObjectView.paint_layers.strokes`) has no route through
//! the current `run-mesh-segmentation` WIT surface, which carries only
//! `list<object-id>` + `config-view`.

use mesh_segmentation::MeshSegmentation;
use slicer_sdk::prelude::*;
use slicer_sdk::test_prelude::*;

fn object_view(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices: Vec::new(),
        triangles: Vec::new(),
        paint_layers: Vec::new(),
    }
}

#[test]
fn on_print_start_defaults() {
    let cfg = ConfigViewBuilder::new().build();
    let module = MeshSegmentation::on_print_start(&cfg);
    assert!(module.is_ok());
}

#[test]
fn empty_config_emits_no_marks() {
    let cfg = ConfigViewBuilder::new().build();
    let module = MeshSegmentation::on_print_start(&cfg).unwrap();
    let objects = vec![object_view("obj-1")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &cfg)
        .unwrap();
    assert!(
        output.triangle_paint_marks().is_empty(),
        "no mesh_seg_mark:* keys → zero drained marks"
    );
}

#[test]
fn single_mark_reaches_output_verbatim() {
    let cfg = ConfigViewBuilder::new()
        .string("mesh_seg_mark:obj-1:3:support_enforcer", "enabled")
        .build();
    let module = MeshSegmentation::on_print_start(&cfg).unwrap();
    let objects = vec![object_view("obj-1")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &cfg)
        .unwrap();
    let marks = output.triangle_paint_marks();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].object_id, "obj-1");
    assert_eq!(marks[0].facet_index, 3);
    assert_eq!(marks[0].semantic, "support_enforcer");
    assert_eq!(marks[0].value, "enabled");
}

#[test]
fn marks_are_sorted_by_object_then_facet_then_semantic() {
    // Two objects, marks intentionally shuffled; sort must be:
    //   obj-A first (host order), facet asc, semantic asc,
    //   then obj-B.
    let cfg = ConfigViewBuilder::new()
        .int("mesh_seg_mark:obj-B:0:tool", 1)
        .string("mesh_seg_mark:obj-A:1:seam", "x")
        .string("mesh_seg_mark:obj-A:0:seam", "y")
        .bool("mesh_seg_mark:obj-A:0:fuzzy_skin", true)
        .build();
    let module = MeshSegmentation::on_print_start(&cfg).unwrap();
    let objects = vec![object_view("obj-A"), object_view("obj-B")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &cfg)
        .unwrap();

    let keys: Vec<(String, u32, String)> = output
        .triangle_paint_marks()
        .iter()
        .map(|m| (m.object_id.clone(), m.facet_index, m.semantic.clone()))
        .collect();

    assert_eq!(
        keys,
        vec![
            ("obj-A".to_string(), 0, "fuzzy_skin".to_string()),
            ("obj-A".to_string(), 0, "seam".to_string()),
            ("obj-A".to_string(), 1, "seam".to_string()),
            ("obj-B".to_string(), 0, "tool".to_string()),
        ],
    );
}

#[test]
fn non_string_values_are_coerced_to_strings() {
    let cfg = ConfigViewBuilder::new()
        .int("mesh_seg_mark:obj-1:0:tool", 7)
        .bool("mesh_seg_mark:obj-1:1:flag", false)
        .float("mesh_seg_mark:obj-1:2:scalar", 1.5)
        .build();
    let module = MeshSegmentation::on_print_start(&cfg).unwrap();
    let objects = vec![object_view("obj-1")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &cfg)
        .unwrap();
    let marks = output.triangle_paint_marks();
    let values: Vec<&str> = marks.iter().map(|m| m.value.as_str()).collect();
    assert_eq!(values, vec!["7", "false", "1.5"]);
}

#[test]
fn malformed_marks_are_silently_skipped() {
    // Unrelated keys, missing segments, empty fields, list values.
    let cfg = ConfigViewBuilder::new()
        .float("layer_height", 0.2)
        .string("mesh_seg_mark:obj:5", "x") // missing semantic
        .string("mesh_seg_mark::5:sem", "x") // empty object id
        .string("mesh_seg_mark:obj:not-a-number:sem", "x")
        .list("mesh_seg_mark:obj:0:sem", vec![]) // unsupported value kind
        .build();
    let module = MeshSegmentation::on_print_start(&cfg).unwrap();
    let objects = vec![object_view("obj")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &cfg)
        .unwrap();
    assert!(output.triangle_paint_marks().is_empty());
}

#[test]
fn unknown_object_ids_still_emit_but_sort_after_known_ones() {
    let cfg = ConfigViewBuilder::new()
        .string("mesh_seg_mark:unknown:0:sem", "x")
        .string("mesh_seg_mark:obj-1:0:sem", "y")
        .build();
    let module = MeshSegmentation::on_print_start(&cfg).unwrap();
    let objects = vec![object_view("obj-1")];
    let mut output = MeshSegmentationOutput::new();
    module
        .run_mesh_segmentation(&objects, &mut output, &cfg)
        .unwrap();
    let ids: Vec<&str> = output
        .triangle_paint_marks()
        .iter()
        .map(|m| m.object_id.as_str())
        .collect();
    assert_eq!(ids, vec!["obj-1", "unknown"]);
}
