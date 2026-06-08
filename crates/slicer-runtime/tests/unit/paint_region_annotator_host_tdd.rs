//! TDD coverage for the host-side paint-annotation fallback.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, LayerPaintMap, PaintRegionIR, PaintSemantic, PaintValue, Point2, Polygon, SliceIR,
    SlicedRegion, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::slice_postprocess::{
    execute_slice_postprocess_paint_annotation, SlicePostProcessPaintAnnotationRequest,
};

fn square_polygon(x: i64, y: i64, size: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x, y },
                Point2 { x: x + size, y },
                Point2 {
                    x: x + size,
                    y: y + size,
                },
                Point2 { x, y: y + size },
            ],
        },
        holes: vec![],
    }
}

fn large_paint_region() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: -1_000_000,
                    y: -1_000_000,
                },
                Point2 {
                    x: 1_000_000,
                    y: -1_000_000,
                },
                Point2 {
                    x: 1_000_000,
                    y: 1_000_000,
                },
                Point2 {
                    x: -1_000_000,
                    y: 1_000_000,
                },
            ],
        },
        holes: vec![],
    }
}

fn empty_paint_ir() -> PaintRegionIR {
    PaintRegionIR {
        per_layer: HashMap::new(),
        ..Default::default()
    }
}

fn make_slice_ir(regions: Vec<SlicedRegion>) -> SliceIR {
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.2,
        regions,
    }
}

fn make_region(obj: &str, region_id: u64, polygons: Vec<ExPolygon>) -> SlicedRegion {
    SlicedRegion {
        object_id: obj.to_string(),
        region_id,
        polygons,
        effective_layer_height: 0.2,
        ..Default::default()
    }
}

#[test]
fn empty_paint_regions_produce_no_segment_annotations_updates() {
    let slice_ir = make_slice_ir(vec![make_region(
        "obj1",
        0,
        vec![square_polygon(0, 0, 10_000)],
    )]);
    let paint_regions = Arc::new(empty_paint_ir());

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![],
            modifier_projections: vec![],
        })
        .unwrap();

    assert!(result.slice_ir.regions[0].segment_annotations.is_empty());
}

#[test]
fn segment_annotations_lengths_match_contour_point_counts() {
    let poly = square_polygon(0, 0, 10_000);
    let point_count = poly.contour.points.len();
    let slice_ir = make_slice_ir(vec![make_region("obj1", 0, vec![poly])]);
    let paint_regions = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![slicer_ir::SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::ToolIndex(1),
                        paint_order: 0,
                        aabb: None,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
        })
        .unwrap();

    let bp = &result.slice_ir.regions[0].segment_annotations;
    let material_bp = bp.get(&PaintSemantic::Material).unwrap();
    assert_eq!(material_bp.len(), 1);
    assert_eq!(material_bp[0].len(), point_count);
    for entry in &material_bp[0] {
        assert_eq!(entry, &Some(PaintValue::ToolIndex(1)));
    }
}

#[test]
fn multiple_polygons_per_region_each_get_segment_annotations() {
    let poly1 = square_polygon(0, 0, 10_000);
    let poly2 = square_polygon(50_000, 0, 20_000);
    let slice_ir = make_slice_ir(vec![make_region("obj1", 0, vec![poly1, poly2])]);
    let paint_regions = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::FuzzySkin,
                    vec![slicer_ir::SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::Flag(true),
                        paint_order: 0,
                        aabb: None,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
        })
        .unwrap();

    let bp = &result.slice_ir.regions[0].segment_annotations;
    let fuzzy_bp = bp.get(&PaintSemantic::FuzzySkin).unwrap();
    assert_eq!(fuzzy_bp.len(), 2);
    assert_eq!(fuzzy_bp[0].len(), 4);
    assert_eq!(fuzzy_bp[1].len(), 4);
}

#[test]
fn multiple_semantics_handled_independently() {
    let poly = square_polygon(0, 0, 10_000);
    let slice_ir = make_slice_ir(vec![make_region("obj1", 0, vec![poly])]);
    let paint_regions = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([
                    (
                        PaintSemantic::Material,
                        vec![slicer_ir::SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::ToolIndex(2),
                            paint_order: 0,
                            aabb: None,
                        }],
                    ),
                    (
                        PaintSemantic::FuzzySkin,
                        vec![slicer_ir::SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::Flag(true),
                            paint_order: 0,
                            aabb: None,
                        }],
                    ),
                ]),
            },
        )]),
        ..Default::default()
    });

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![PaintSemantic::Material, PaintSemantic::FuzzySkin],
            modifier_projections: vec![],
        })
        .unwrap();

    let bp = &result.slice_ir.regions[0].segment_annotations;
    assert!(bp.contains_key(&PaintSemantic::Material));
    assert!(bp.contains_key(&PaintSemantic::FuzzySkin));

    let mat_bp = bp.get(&PaintSemantic::Material).unwrap();
    assert_eq!(mat_bp[0].len(), 4);
    for v in &mat_bp[0] {
        assert_eq!(v, &Some(PaintValue::ToolIndex(2)));
    }

    let fuzzy_bp = bp.get(&PaintSemantic::FuzzySkin).unwrap();
    assert_eq!(fuzzy_bp[0].len(), 4);
    for v in &fuzzy_bp[0] {
        assert_eq!(v, &Some(PaintValue::Flag(true)));
    }
}

#[test]
fn multiple_regions_each_get_segment_annotations() {
    let poly = square_polygon(0, 0, 10_000);
    let slice_ir = make_slice_ir(vec![
        make_region("obj1", 0, vec![poly.clone()]),
        make_region("obj1", 1, vec![square_polygon(100_000, 0, 10_000)]),
    ]);
    let paint_regions = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![slicer_ir::SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::ToolIndex(3),
                        paint_order: 0,
                        aabb: None,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
        })
        .unwrap();

    assert_eq!(result.slice_ir.regions.len(), 2);
    for region in &result.slice_ir.regions {
        assert!(region.segment_annotations.contains_key(&PaintSemantic::Material));
    }
}

#[test]
fn highest_paint_order_wins_for_overlapping_region() {
    let poly = square_polygon(0, 0, 10_000);
    let slice_ir = make_slice_ir(vec![make_region("obj1", 0, vec![poly])]);
    let paint_regions = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![
                        slicer_ir::SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::ToolIndex(1),
                            paint_order: 0,
                            aabb: None,
                        },
                        slicer_ir::SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::ToolIndex(5),
                            paint_order: 10,
                            aabb: None,
                        },
                    ],
                )]),
            },
        )]),
        ..Default::default()
    });

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
        })
        .unwrap();

    let bp = &result.slice_ir.regions[0].segment_annotations;
    let mat_bp = bp.get(&PaintSemantic::Material).unwrap();
    for entry in &mat_bp[0] {
        assert_eq!(entry, &Some(PaintValue::ToolIndex(5)));
    }
}

#[test]
fn region_with_no_polygons_produces_empty_segment_annotations() {
    let slice_ir = make_slice_ir(vec![make_region("obj1", 0, vec![])]);
    let paint_regions = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![slicer_ir::SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::ToolIndex(1),
                        paint_order: 0,
                        aabb: None,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir,
            paint_regions,
            paint_region_rtree: None,
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
        })
        .unwrap();

    let bp = &result.slice_ir.regions[0].segment_annotations;
    let mat_bp = bp.get(&PaintSemantic::Material).unwrap();
    assert!(mat_bp.is_empty());
}
