#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    execute_slice_postprocess_paint_annotation, SlicePostProcessPaintAnnotationError,
    SlicePostProcessPaintAnnotationRequest, SlicePostProcessPaintAnnotationResult,
    SlicePostProcessPaintAnnotationWarning, SlicePostProcessPaintAnnotationWarningReason,
};
use slicer_ir::{
    ExPolygon, LayerPaintMap, PaintRegionIR, PaintSemantic, PaintValue, Point2, Polygon, SemVer,
    SemanticRegion, SliceIR, SlicedRegion,
};

#[test]
fn slice_postprocess_paint_annotation_keeps_boundary_paint_present_but_empty_when_no_paint_applies()
{
    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                7,
                vec![region_fixture(
                    "plain-object",
                    0,
                    vec![square(0.0, 0.0, 10.0, 10.0)],
                )],
            ),
            paint_regions: Arc::new(empty_layer_paint_regions(7)),
            required_semantics: Vec::new(),
        })
        .expect(
            "regions/layers without paint must still keep boundary_paint present as an empty map",
        );

    assert_eq!(
        result,
        SlicePostProcessPaintAnnotationResult {
            slice_ir: slice_fixture(
                7,
                vec![region_fixture(
                    "plain-object",
                    0,
                    vec![square(0.0, 0.0, 10.0, 10.0)],
                )],
            ),
            degraded: false,
            warnings: Vec::new(),
        }
    );
}

#[test]
fn slice_postprocess_paint_annotation_writes_semantic_entries_parallel_to_each_final_contour() {
    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                3,
                vec![region_fixture(
                    "painted-object",
                    9,
                    vec![square(0.0, 0.0, 10.0, 10.0), square(20.0, 0.0, 30.0, 10.0)],
                )],
            ),
            paint_regions: Arc::new(material_and_fuzzy_regions(3)),
            required_semantics: vec![PaintSemantic::Material, PaintSemantic::FuzzySkin],
        })
        .expect("annotation should write one contour-parallel entry per polygon for each semantic");

    let region = &result.slice_ir.regions[0];
    assert_eq!(region.boundary_paint.len(), 2);
    assert_eq!(
        region.boundary_paint[&PaintSemantic::Material],
        vec![
            vec![
                Some(PaintValue::ToolIndex(2)),
                Some(PaintValue::ToolIndex(2)),
                Some(PaintValue::ToolIndex(2)),
                Some(PaintValue::ToolIndex(2)),
            ],
            vec![
                Some(PaintValue::ToolIndex(2)),
                Some(PaintValue::ToolIndex(2)),
                Some(PaintValue::ToolIndex(2)),
                Some(PaintValue::ToolIndex(2)),
            ],
        ]
    );
    assert_eq!(
        region.boundary_paint[&PaintSemantic::FuzzySkin],
        vec![
            vec![
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
            ],
            vec![
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
                Some(PaintValue::Flag(true)),
            ],
        ]
    );
}

#[test]
fn slice_postprocess_paint_annotation_rejects_stale_boundary_paint_after_polygon_point_edits() {
    let mut stale_region = region_fixture(
        "edited-object",
        4,
        vec![polygon(vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (12.0, 6.0),
            (5.0, 10.0),
            (0.0, 10.0),
        ])],
    );
    stale_region.boundary_paint.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::ToolIndex(1)); 4]],
    );

    assert_eq!(
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(5, vec![stale_region]),
            paint_regions: Arc::new(material_regions(5, PaintValue::ToolIndex(1))),
            required_semantics: vec![PaintSemantic::Material],
        }),
        Err(
            SlicePostProcessPaintAnnotationError::BoundaryPaintCardinalityMismatch {
                code: 502,
                global_layer_index: 5,
                object_id: String::from("edited-object"),
                region_id: 4,
                semantic: PaintSemantic::Material,
                polygon_index: 0,
                expected_points: 5,
                actual_points: 4,
            },
        )
    );
}

#[test]
fn slice_postprocess_paint_annotation_propagates_equal_precedence_custom_conflicts_fatally() {
    let semantic = PaintSemantic::Custom(String::from("com.example.texture/roughness@1"));

    assert_eq!(
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                2,
                vec![region_fixture(
                    "conflict-object",
                    1,
                    vec![square(0.0, 0.0, 10.0, 10.0)],
                )],
            ),
            paint_regions: Arc::new(conflicting_custom_regions(2, semantic.clone())),
            required_semantics: vec![semantic.clone()],
        }),
        Err(
            SlicePostProcessPaintAnnotationError::DeterministicConflict {
                code: 503,
                global_layer_index: 2,
                object_id: String::from("conflict-object"),
                region_id: 1,
                semantic,
                polygon_index: 0,
                contour_point_index: 0,
            }
        )
    );
}

#[test]
fn slice_postprocess_paint_annotation_defaults_unresolved_points_and_marks_degraded() {
    let result = execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
        slice_ir: slice_fixture(
            42,
            vec![region_fixture(
                "ambiguous-object",
                6,
                vec![polygon(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 0.0001), (0.0, 10.0)])],
            )],
        ),
        paint_regions: Arc::new(partially_resolved_builtin_regions(42)),
        required_semantics: vec![
            PaintSemantic::Material,
            PaintSemantic::FuzzySkin,
            PaintSemantic::SupportEnforcer,
            PaintSemantic::SupportBlocker,
        ],
    })
    .expect(
        "numerically unresolved points should degrade with deterministic defaults instead of aborting",
    );

    let region = &result.slice_ir.regions[0];
    assert!(result.degraded);
    assert_eq!(result.warnings.len(), 4);
    assert_eq!(
        result.warnings,
        vec![
            SlicePostProcessPaintAnnotationWarning {
                code: 504,
                global_layer_index: 42,
                object_id: String::from("ambiguous-object"),
                region_id: 6,
                semantic: PaintSemantic::Material,
                polygon_index: 0,
                contour_point_index: 2,
                fallback_value: PaintValue::ToolIndex(0),
                reason: SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity,
            },
            SlicePostProcessPaintAnnotationWarning {
                code: 504,
                global_layer_index: 42,
                object_id: String::from("ambiguous-object"),
                region_id: 6,
                semantic: PaintSemantic::FuzzySkin,
                polygon_index: 0,
                contour_point_index: 2,
                fallback_value: PaintValue::Flag(false),
                reason: SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity,
            },
            SlicePostProcessPaintAnnotationWarning {
                code: 504,
                global_layer_index: 42,
                object_id: String::from("ambiguous-object"),
                region_id: 6,
                semantic: PaintSemantic::SupportEnforcer,
                polygon_index: 0,
                contour_point_index: 2,
                fallback_value: PaintValue::Flag(false),
                reason: SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity,
            },
            SlicePostProcessPaintAnnotationWarning {
                code: 504,
                global_layer_index: 42,
                object_id: String::from("ambiguous-object"),
                region_id: 6,
                semantic: PaintSemantic::SupportBlocker,
                polygon_index: 0,
                contour_point_index: 2,
                fallback_value: PaintValue::Flag(false),
                reason: SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity,
            },
        ]
    );
    assert_eq!(region.boundary_paint[&PaintSemantic::Material][0].len(), 4);
    assert_eq!(
        region.boundary_paint[&PaintSemantic::Material][0][2],
        Some(PaintValue::ToolIndex(0))
    );
    assert_eq!(
        region.boundary_paint[&PaintSemantic::FuzzySkin][0][2],
        Some(PaintValue::Flag(false))
    );
    assert_eq!(
        region.boundary_paint[&PaintSemantic::SupportEnforcer][0][2],
        Some(PaintValue::Flag(false))
    );
    assert_eq!(
        region.boundary_paint[&PaintSemantic::SupportBlocker][0][2],
        Some(PaintValue::Flag(false))
    );
}

#[test]
fn slice_postprocess_paint_annotation_requires_paint_region_data_for_required_semantics() {
    assert_eq!(
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                11,
                vec![region_fixture(
                    "required-object",
                    2,
                    vec![square(0.0, 0.0, 10.0, 10.0)],
                )],
            ),
            paint_regions: Arc::new(empty_layer_paint_regions(11)),
            required_semantics: vec![PaintSemantic::Material],
        }),
        Err(
            SlicePostProcessPaintAnnotationError::MissingPaintRegionSemantic {
                code: 501,
                global_layer_index: 11,
                semantic: PaintSemantic::Material,
            }
        )
    );
}

fn slice_fixture(global_layer_index: u32, regions: Vec<SlicedRegion>) -> SliceIR {
    SliceIR {
        schema_version: schema_version(),
        global_layer_index,
        z: 0.2 * (global_layer_index as f32 + 1.0),
        regions,
    }
}

fn region_fixture(object_id: &str, region_id: u64, polygons: Vec<ExPolygon>) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_owned(),
        region_id,
        polygons,
        infill_areas: Vec::new(),
        nonplanar_surface: None,
        effective_layer_height: 0.2,
        boundary_paint: HashMap::new(),
    }
}

fn empty_layer_paint_regions(layer_index: u32) -> PaintRegionIR {
    PaintRegionIR {
        schema_version: schema_version(),
        per_layer: HashMap::from([(
            layer_index,
            LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: HashMap::new(),
            },
        )]),
    }
}

fn material_regions(layer_index: u32, value: PaintValue) -> PaintRegionIR {
    paint_region_ir(
        layer_index,
        vec![(
            PaintSemantic::Material,
            vec![semantic_region(
                "edited-object",
                vec![square(0.0, 0.0, 12.0, 10.0)],
                value,
                0,
            )],
        )],
    )
}

fn material_and_fuzzy_regions(layer_index: u32) -> PaintRegionIR {
    paint_region_ir(
        layer_index,
        vec![
            (
                PaintSemantic::Material,
                vec![semantic_region(
                    "painted-object",
                    vec![square(0.0, 0.0, 30.0, 10.0)],
                    PaintValue::ToolIndex(2),
                    0,
                )],
            ),
            (
                PaintSemantic::FuzzySkin,
                vec![semantic_region(
                    "painted-object",
                    vec![square(0.0, 0.0, 30.0, 10.0)],
                    PaintValue::Flag(true),
                    1,
                )],
            ),
        ],
    )
}

fn conflicting_custom_regions(layer_index: u32, semantic: PaintSemantic) -> PaintRegionIR {
    paint_region_ir(
        layer_index,
        vec![(
            semantic,
            vec![
                semantic_region(
                    "conflict-object",
                    vec![square(0.0, 0.0, 10.0, 10.0)],
                    PaintValue::Scalar(0.2),
                    7,
                ),
                semantic_region(
                    "conflict-object",
                    vec![square(0.0, 0.0, 10.0, 10.0)],
                    PaintValue::Scalar(0.8),
                    7,
                ),
            ],
        )],
    )
}

fn partially_resolved_builtin_regions(layer_index: u32) -> PaintRegionIR {
    paint_region_ir(
        layer_index,
        vec![
            (
                PaintSemantic::Material,
                vec![semantic_region(
                    "ambiguous-object",
                    vec![polygon(vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)])],
                    PaintValue::ToolIndex(3),
                    0,
                )],
            ),
            (
                PaintSemantic::FuzzySkin,
                vec![semantic_region(
                    "ambiguous-object",
                    vec![polygon(vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)])],
                    PaintValue::Flag(true),
                    0,
                )],
            ),
            (
                PaintSemantic::SupportEnforcer,
                vec![semantic_region(
                    "ambiguous-object",
                    vec![polygon(vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)])],
                    PaintValue::Flag(true),
                    0,
                )],
            ),
            (
                PaintSemantic::SupportBlocker,
                vec![semantic_region(
                    "ambiguous-object",
                    vec![polygon(vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)])],
                    PaintValue::Flag(true),
                    0,
                )],
            ),
        ],
    )
}

fn paint_region_ir(
    layer_index: u32,
    semantics: Vec<(PaintSemantic, Vec<SemanticRegion>)>,
) -> PaintRegionIR {
    PaintRegionIR {
        schema_version: schema_version(),
        per_layer: HashMap::from([(
            layer_index,
            LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: semantics.into_iter().collect(),
            },
        )]),
    }
}

fn semantic_region(
    object_id: &str,
    polygons: Vec<ExPolygon>,
    value: PaintValue,
    paint_order: u64,
) -> SemanticRegion {
    SemanticRegion {
        object_id: object_id.to_owned(),
        polygons,
        value,
        paint_order,
    }
}

fn square(min_x_mm: f32, min_y_mm: f32, max_x_mm: f32, max_y_mm: f32) -> ExPolygon {
    polygon(vec![
        (min_x_mm, min_y_mm),
        (max_x_mm, min_y_mm),
        (max_x_mm, max_y_mm),
        (min_x_mm, max_y_mm),
    ])
}

fn polygon(points_mm: Vec<(f32, f32)>) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: points_mm
                .into_iter()
                .map(|(x, y)| Point2::from_mm(x, y))
                .collect(),
        },
        holes: Vec::new(),
    }
}

fn schema_version() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}
