#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_core::{point_in_paint_region, BoundaryInclusion, PaintRegionQueryError};
use slicer_ir::{
    ExPolygon, LayerPaintMap, PaintRegionIR, PaintSemantic, PaintValue, Point2, Polygon,
    SemVer, SemanticRegion,
};

fn schema_version() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn square(min_x_mm: f32, min_y_mm: f32, max_x_mm: f32, max_y_mm: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x_mm, min_y_mm),
                Point2::from_mm(max_x_mm, min_y_mm),
                Point2::from_mm(max_x_mm, max_y_mm),
                Point2::from_mm(min_x_mm, max_y_mm),
            ],
        },
        holes: Vec::new(),
    }
}

fn square_with_hole(
    min_x_mm: f32,
    min_y_mm: f32,
    max_x_mm: f32,
    max_y_mm: f32,
    hole_min_x_mm: f32,
    hole_min_y_mm: f32,
    hole_max_x_mm: f32,
    hole_max_y_mm: f32,
) -> ExPolygon {
    ExPolygon {
        contour: square(min_x_mm, min_y_mm, max_x_mm, max_y_mm).contour,
        holes: vec![Polygon {
            points: vec![
                Point2::from_mm(hole_min_x_mm, hole_min_y_mm),
                Point2::from_mm(hole_min_x_mm, hole_max_y_mm),
                Point2::from_mm(hole_max_x_mm, hole_max_y_mm),
                Point2::from_mm(hole_max_x_mm, hole_min_y_mm),
            ],
        }],
    }
}

fn semantic_region(
    polygons: Vec<ExPolygon>,
    value: PaintValue,
    paint_order: u64,
) -> SemanticRegion {
    SemanticRegion {
        object_id: "object-1".to_string(),
        polygons,
        value,
        paint_order,
    }
}

fn paint_region_ir(layer_index: u32, semantic: PaintSemantic, regions: Vec<SemanticRegion>) -> PaintRegionIR {
    PaintRegionIR {
        schema_version: schema_version(),
        per_layer: HashMap::from([(
            layer_index,
            LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: HashMap::from([(semantic, regions)]),
            },
        )]),
    }
}

fn query(
    paint_regions: &PaintRegionIR,
    layer_index: u32,
    semantic: &PaintSemantic,
    x_mm: f32,
    y_mm: f32,
    boundary_inclusion: BoundaryInclusion,
) -> Result<Option<PaintValue>, PaintRegionQueryError> {
    point_in_paint_region(
        paint_regions,
        layer_index,
        semantic,
        Point2::from_mm(x_mm, y_mm),
        boundary_inclusion,
    )
}

#[test]
fn empty_layer_or_semantic_returns_none() {
    let empty = PaintRegionIR {
        schema_version: schema_version(),
        per_layer: HashMap::new(),
    };

    assert_eq!(
        query(
            &empty,
            7,
            &PaintSemantic::Material,
            5.0,
            5.0,
            BoundaryInclusion::Include,
        ),
        Ok(None)
    );
}

#[test]
fn point_inside_contour_returns_region_value() {
    let paint_regions = paint_region_ir(
        3,
        PaintSemantic::Material,
        vec![semantic_region(
            vec![square(0.0, 0.0, 10.0, 10.0)],
            PaintValue::ToolIndex(2),
            1,
        )],
    );

    assert_eq!(
        query(
            &paint_regions,
            3,
            &PaintSemantic::Material,
            4.0,
            6.0,
            BoundaryInclusion::Include,
        ),
        Ok(Some(PaintValue::ToolIndex(2)))
    );
}

#[test]
fn point_outside_all_regions_returns_none() {
    let paint_regions = paint_region_ir(
        3,
        PaintSemantic::Material,
        vec![semantic_region(
            vec![square(0.0, 0.0, 10.0, 10.0)],
            PaintValue::ToolIndex(2),
            1,
        )],
    );

    assert_eq!(
        query(
            &paint_regions,
            3,
            &PaintSemantic::Material,
            12.0,
            6.0,
            BoundaryInclusion::Include,
        ),
        Ok(None)
    );
}

#[test]
fn hole_interior_is_excluded_from_containment() {
    let paint_regions = paint_region_ir(
        0,
        PaintSemantic::FuzzySkin,
        vec![semantic_region(
            vec![square_with_hole(0.0, 0.0, 10.0, 10.0, 3.0, 3.0, 7.0, 7.0)],
            PaintValue::Flag(true),
            4,
        )],
    );

    assert_eq!(
        query(
            &paint_regions,
            0,
            &PaintSemantic::FuzzySkin,
            5.0,
            5.0,
            BoundaryInclusion::Include,
        ),
        Ok(None)
    );
}

#[test]
fn outer_contour_boundary_counts_as_inside_when_boundary_inclusion_is_enabled() {
    let paint_regions = paint_region_ir(
        0,
        PaintSemantic::Material,
        vec![semantic_region(
            vec![square(0.0, 0.0, 10.0, 10.0)],
            PaintValue::ToolIndex(7),
            9,
        )],
    );

    assert_eq!(
        query(
            &paint_regions,
            0,
            &PaintSemantic::Material,
            0.0,
            5.0,
            BoundaryInclusion::Include,
        ),
        Ok(Some(PaintValue::ToolIndex(7)))
    );
}

#[test]
fn hole_boundary_remains_contained_when_boundary_inclusion_is_enabled() {
    let paint_regions = paint_region_ir(
        0,
        PaintSemantic::FuzzySkin,
        vec![semantic_region(
            vec![square_with_hole(0.0, 0.0, 10.0, 10.0, 3.0, 3.0, 7.0, 7.0)],
            PaintValue::Flag(true),
            4,
        )],
    );

    assert_eq!(
        query(
            &paint_regions,
            0,
            &PaintSemantic::FuzzySkin,
            3.0,
            5.0,
            BoundaryInclusion::Include,
        ),
        Ok(Some(PaintValue::Flag(true)))
    );
}

#[test]
fn overlapping_regions_of_same_semantic_choose_highest_paint_order() {
    let paint_regions = paint_region_ir(
        1,
        PaintSemantic::Material,
        vec![
            semantic_region(vec![square(0.0, 0.0, 10.0, 10.0)], PaintValue::ToolIndex(1), 3),
            semantic_region(vec![square(2.0, 2.0, 8.0, 8.0)], PaintValue::ToolIndex(4), 8),
        ],
    );

    assert_eq!(
        query(
            &paint_regions,
            1,
            &PaintSemantic::Material,
            5.0,
            5.0,
            BoundaryInclusion::Include,
        ),
        Ok(Some(PaintValue::ToolIndex(4)))
    );
}

#[test]
fn equal_paint_order_conflicting_custom_values_report_deterministic_conflict() {
    let semantic = PaintSemantic::Custom("com.example.texture/roughness@1".to_string());
    let paint_regions = paint_region_ir(
        2,
        semantic.clone(),
        vec![
            semantic_region(vec![square(0.0, 0.0, 10.0, 10.0)], PaintValue::Scalar(0.2), 11),
            semantic_region(vec![square(2.0, 2.0, 8.0, 8.0)], PaintValue::Scalar(0.8), 11),
        ],
    );

    assert_eq!(
        query(
            &paint_regions,
            2,
            &semantic,
            4.0,
            4.0,
            BoundaryInclusion::Include,
        ),
        Err(PaintRegionQueryError::DeterministicConflict)
    );
}

#[test]
fn support_semantics_query_independently_without_cross_semantic_override_leakage() {
    let layer_index = 5;
    let point = Point2::from_mm(5.0, 5.0);
    let paint_regions = PaintRegionIR {
        schema_version: schema_version(),
        per_layer: HashMap::from([(
            layer_index,
            LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: HashMap::from([
                    (
                        PaintSemantic::SupportEnforcer,
                        vec![semantic_region(
                            vec![square(0.0, 0.0, 10.0, 10.0)],
                            PaintValue::Flag(true),
                            2,
                        )],
                    ),
                    (
                        PaintSemantic::SupportBlocker,
                        vec![semantic_region(
                            vec![square(0.0, 0.0, 10.0, 10.0)],
                            PaintValue::Flag(true),
                            6,
                        )],
                    ),
                ]),
            },
        )]),
    };

    assert_eq!(
        point_in_paint_region(
            &paint_regions,
            layer_index,
            &PaintSemantic::SupportEnforcer,
            point,
            BoundaryInclusion::Include,
        ),
        Ok(Some(PaintValue::Flag(true)))
    );
    assert_eq!(
        point_in_paint_region(
            &paint_regions,
            layer_index,
            &PaintSemantic::SupportBlocker,
            point,
            BoundaryInclusion::Include,
        ),
        Ok(Some(PaintValue::Flag(true)))
    );
}
