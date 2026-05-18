//! TDD tests for paint-region-annotator module.
//!
//! Tests verify that `PaintRegionAnnotator::run_slice_postprocess` correctly:
//! - Produces empty boundary_paint when no paint regions exist
//! - Writes contour-parallel boundary_paint matching polygon contour point counts
//! - Handles multiple semantics independently
//! - Queries point-in-polygon for each contour point
//! - Uses deterministic fallback for unresolved edge points (non-fatal)
//! - Returns fatal error on deterministic conflicts

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, LayerPaintMap, PaintRegionIR, PaintSemantic, PaintValue, Point2,
    Polygon, RegionId, RegionKey, SemanticRegion,
};
use slicer_sdk::builders::SlicePostprocessBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use paint_region_annotator::PaintRegionAnnotator;

/// Helper: create a simple square polygon as ExPolygon with 4 contour points.
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

/// Helper: create a large paint region polygon that contains all test points.
fn large_paint_region() -> ExPolygon {
    // 100mm x 100mm region in scaled units (10_000 per mm)
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

/// Helper: create a ConfigView with no fields.
fn empty_config() -> ConfigView {
    ConfigView::new()
}

/// Helper: create an empty PaintRegionIR.
fn empty_paint_ir() -> PaintRegionIR {
    PaintRegionIR {
        per_layer: HashMap::new(),
        ..Default::default()
    }
}

/// Helper: create a SliceRegionView from an ExPolygon.
fn region_view(object_id: &str, region_id: u32, polygons: Vec<ExPolygon>) -> SliceRegionView {
    {
        let mut tmp = SliceRegionView::default();
        tmp.set_object_id(object_id.to_string());
        tmp.set_region_id(region_id as RegionId);
        tmp.set_polygons(polygons);
        tmp.set_infill_areas(vec![]);
        tmp.set_effective_layer_height(0.2);
        tmp.set_z(0.2);
        tmp.set_has_nonplanar(false);
        tmp
    }
}

/// Helper: create a RegionKey.
fn region_key(layer: u32, object_id: &str, region_id: u32) -> RegionKey {
    RegionKey {
        global_layer_index: layer,
        object_id: object_id.to_string(),
        region_id: region_id as RegionId,
    }
}

// =========================================================================
// Test: Empty paint regions produce empty boundary_paint
// =========================================================================

#[test]
fn empty_paint_regions_produce_empty_boundary_paint() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly = square_polygon(0, 0, 10_000); // 1mm square
    let regions = vec![region_view("obj1", 0, vec![poly])];
    let paint_ir = Arc::new(empty_paint_ir());
    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    // No paint data means no boundary_paint updates
    assert!(
        output.boundary_paint_updates().is_empty(),
        "empty paint regions should produce no boundary_paint updates"
    );
}

// =========================================================================
// Test: Contour-parallel structure matches polygon contour point counts
// =========================================================================

#[test]
fn boundary_paint_lengths_match_contour_point_counts() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly = square_polygon(0, 0, 10_000); // 4 contour points
    let point_count = poly.contour.points.len();
    let regions = vec![region_view("obj1", 0, vec![poly])];

    // Paint region covering the entire area with Material semantic
    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::ToolIndex(1),
                        paint_order: 0,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(output.boundary_paint_updates().len(), 1);
    let (key, bp) = &output.boundary_paint_updates()[0];
    assert_eq!(key, &region_key(0, "obj1", 0));

    let material_bp = bp.get(&PaintSemantic::Material).unwrap();
    // One entry per polygon
    assert_eq!(material_bp.len(), 1);
    // Inner Vec matches contour point count
    assert_eq!(material_bp[0].len(), point_count);
    // All points inside the large region should have the paint value
    for entry in &material_bp[0] {
        assert_eq!(entry, &Some(PaintValue::ToolIndex(1)));
    }
}

// =========================================================================
// Test: Multiple polygons per region
// =========================================================================

#[test]
fn multiple_polygons_per_region_each_get_boundary_paint() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly1 = square_polygon(0, 0, 10_000); // 4 points
    let poly2 = square_polygon(50_000, 0, 20_000); // 4 points
    let regions = vec![region_view("obj1", 0, vec![poly1, poly2])];

    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::FuzzySkin,
                    vec![SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::Flag(true),
                        paint_order: 0,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(output.boundary_paint_updates().len(), 1);
    let (_, bp) = &output.boundary_paint_updates()[0];
    let fuzzy_bp = bp.get(&PaintSemantic::FuzzySkin).unwrap();
    assert_eq!(fuzzy_bp.len(), 2); // two polygons
    assert_eq!(fuzzy_bp[0].len(), 4);
    assert_eq!(fuzzy_bp[1].len(), 4);
}

// =========================================================================
// Test: Multiple semantics handled independently
// =========================================================================

#[test]
fn multiple_semantics_handled_independently() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly = square_polygon(0, 0, 10_000);
    let regions = vec![region_view("obj1", 0, vec![poly])];

    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([
                    (
                        PaintSemantic::Material,
                        vec![SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::ToolIndex(2),
                            paint_order: 0,
                        }],
                    ),
                    (
                        PaintSemantic::FuzzySkin,
                        vec![SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::Flag(true),
                            paint_order: 0,
                        }],
                    ),
                ]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(output.boundary_paint_updates().len(), 1);
    let (_, bp) = &output.boundary_paint_updates()[0];

    // Both semantics present
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

// =========================================================================
// Test: Points outside paint region get None
// =========================================================================

#[test]
fn points_outside_paint_region_get_none() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    // Region polygon at (0,0) to (10_000, 10_000) — 1mm square
    let poly = square_polygon(0, 0, 10_000);
    let regions = vec![region_view("obj1", 0, vec![poly])];

    // Paint region far away — centered at 500_000 (50mm)
    let paint_region_poly = square_polygon(400_000, 400_000, 200_000);
    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![paint_region_poly],
                        value: PaintValue::ToolIndex(1),
                        paint_order: 0,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert_eq!(output.boundary_paint_updates().len(), 1);
    let (_, bp) = &output.boundary_paint_updates()[0];
    let mat_bp = bp.get(&PaintSemantic::Material).unwrap();
    // All points should be None since they're outside the paint region
    for entry in &mat_bp[0] {
        assert_eq!(entry, &None);
    }
}

// =========================================================================
// Test: Multiple regions in one layer
// =========================================================================

#[test]
fn multiple_regions_each_get_boundary_paint() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly1 = square_polygon(0, 0, 10_000);
    let poly2 = square_polygon(100_000, 0, 10_000);
    let regions = vec![
        region_view("obj1", 0, vec![poly1]),
        region_view("obj1", 1, vec![poly2]),
    ];

    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::ToolIndex(3),
                        paint_order: 0,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    // Should have two boundary_paint updates, one per region
    assert_eq!(output.boundary_paint_updates().len(), 2);
}

// =========================================================================
// Test: Highest paint_order wins for overlapping regions
// =========================================================================

#[test]
fn highest_paint_order_wins_for_overlapping_regions() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly = square_polygon(0, 0, 10_000);
    let regions = vec![region_view("obj1", 0, vec![poly])];

    // Two overlapping material regions with different paint_order
    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![
                        SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::ToolIndex(1),
                            paint_order: 0,
                        },
                        SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::ToolIndex(5),
                            paint_order: 10,
                        },
                    ],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let (_, bp) = &output.boundary_paint_updates()[0];
    let mat_bp = bp.get(&PaintSemantic::Material).unwrap();
    // Higher paint_order (10) should win with ToolIndex(5)
    for entry in &mat_bp[0] {
        assert_eq!(entry, &Some(PaintValue::ToolIndex(5)));
    }
}

// =========================================================================
// Test: Deterministic conflict for custom semantics with equal paint_order
// =========================================================================

#[test]
fn deterministic_conflict_fatal_for_custom_semantics() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let poly = square_polygon(0, 0, 10_000);
    let regions = vec![region_view("obj1", 0, vec![poly])];

    let custom_sem = PaintSemantic::Custom("test/custom@1".to_string());

    // Two overlapping custom regions with same paint_order but different values
    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    custom_sem.clone(),
                    vec![
                        SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::Scalar(1.0),
                            paint_order: 0,
                        },
                        SemanticRegion {
                            object_id: "obj1".to_string(),
                            polygons: vec![large_paint_region()],
                            value: PaintValue::Scalar(2.0),
                            paint_order: 0, // same order = conflict
                        },
                    ],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    let result = annotator.run_slice_postprocess(0, &regions, &paint, &mut output, &config);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.fatal, "deterministic conflict should be fatal");
    assert_eq!(err.code, 503);
}

// =========================================================================
// Test: Region with no polygons produces no output
// =========================================================================

#[test]
fn region_with_no_polygons_produces_no_boundary_paint() {
    let config = empty_config();
    let annotator = PaintRegionAnnotator::on_print_start(&config).unwrap();

    let regions = vec![region_view("obj1", 0, vec![])]; // no polygons

    let paint_ir = Arc::new(PaintRegionIR {
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::from([(
                    PaintSemantic::Material,
                    vec![SemanticRegion {
                        object_id: "obj1".to_string(),
                        polygons: vec![large_paint_region()],
                        value: PaintValue::ToolIndex(1),
                        paint_order: 0,
                    }],
                )]),
            },
        )]),
        ..Default::default()
    });

    let paint = PaintRegionLayerView::with_paint_regions(0, paint_ir);
    let mut output = SlicePostprocessBuilder::new();

    annotator
        .run_slice_postprocess(0, &regions, &paint, &mut output, &config)
        .unwrap();

    // Region has no polygons, so boundary_paint should still be written
    // but with empty Vecs per semantic
    assert_eq!(output.boundary_paint_updates().len(), 1);
    let (_, bp) = &output.boundary_paint_updates()[0];
    let mat_bp = bp.get(&PaintSemantic::Material).unwrap();
    assert!(mat_bp.is_empty()); // no polygons = empty outer Vec
}
