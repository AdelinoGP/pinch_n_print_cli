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
            modifier_projections: vec![],
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
            modifier_projections: vec![],
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
            modifier_projections: vec![],
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
            modifier_projections: vec![],
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
        modifier_projections: vec![],
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
            modifier_projections: vec![],
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
        is_top_surface: false,
        is_bottom_surface: false,
        is_bridge: false,
        bridge_areas: vec![],
        bridge_orientation_deg: 0.0,
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

// ── Step 15: degraded-warning determinism + propagation ─────────────────
// docs/02 §Paint Region Resolution Contract; docs/04 §770-775 (degraded
// success); docs/10 §Degraded success row; docs/11 §73-75.

#[test]
fn paint_annotation_repeated_runs_emit_byte_identical_warnings() {
    use slicer_host::execute_slice_postprocess_paint_annotation;

    let mk_request = || SlicePostProcessPaintAnnotationRequest {
        slice_ir: slice_fixture(
            42,
            vec![region_fixture(
                "ambiguous-object",
                6,
                vec![polygon(vec![
                    (0.0, 0.0),
                    (10.0, 0.0),
                    (10.0, 0.0001),
                    (0.0, 10.0),
                ])],
            )],
        ),
        paint_regions: Arc::new(partially_resolved_builtin_regions(42)),
        required_semantics: vec![
            PaintSemantic::Material,
            PaintSemantic::FuzzySkin,
            PaintSemantic::SupportEnforcer,
            PaintSemantic::SupportBlocker,
        ],
        modifier_projections: vec![],
    };

    let a = execute_slice_postprocess_paint_annotation(mk_request()).unwrap();
    let b = execute_slice_postprocess_paint_annotation(mk_request()).unwrap();
    let c = execute_slice_postprocess_paint_annotation(mk_request()).unwrap();
    assert!(a.degraded && b.degraded && c.degraded);
    assert_eq!(
        a.warnings, b.warnings,
        "warning sequence must be deterministic across repeated calls"
    );
    assert_eq!(b.warnings, c.warnings);
    assert_eq!(a.slice_ir, b.slice_ir);
}

#[test]
fn paint_annotation_warnings_ordered_region_then_semantic_then_polygon_then_point() {
    use slicer_host::execute_slice_postprocess_paint_annotation;

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                42,
                vec![region_fixture(
                    "ambiguous-object",
                    6,
                    vec![polygon(vec![
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 0.0001),
                        (0.0, 10.0),
                    ])],
                )],
            ),
            paint_regions: Arc::new(partially_resolved_builtin_regions(42)),
            required_semantics: vec![
                PaintSemantic::Material,
                PaintSemantic::FuzzySkin,
                PaintSemantic::SupportEnforcer,
                PaintSemantic::SupportBlocker,
            ],
            modifier_projections: vec![],
        })
        .unwrap();

    // Same (region, polygon, contour_point) — warnings must follow the input
    // semantic order, not arbitrary HashMap iteration.
    let semantics: Vec<&PaintSemantic> = result.warnings.iter().map(|w| &w.semantic).collect();
    assert_eq!(
        semantics,
        vec![
            &PaintSemantic::Material,
            &PaintSemantic::FuzzySkin,
            &PaintSemantic::SupportEnforcer,
            &PaintSemantic::SupportBlocker,
        ]
    );
    for w in &result.warnings {
        assert_eq!(
            w.contour_point_index, 2,
            "only the ambiguous point yields a warning"
        );
        assert_eq!(w.polygon_index, 0);
        assert_eq!(w.global_layer_index, 42);
        assert_eq!(w.code, 504);
    }
}

#[test]
fn paint_annotation_warnings_propagate_to_slice_event_collector_as_degraded() {
    use slicer_host::progress_events::SliceEventCollector;
    use slicer_host::{
        execute_slice_postprocess_paint_annotation, paint_annotation_warning_to_progress_event,
    };

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                42,
                vec![region_fixture(
                    "ambiguous-object",
                    6,
                    vec![polygon(vec![
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 0.0001),
                        (0.0, 10.0),
                    ])],
                )],
            ),
            paint_regions: Arc::new(partially_resolved_builtin_regions(42)),
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
        })
        .unwrap();

    assert!(result.degraded);
    assert_eq!(result.warnings.len(), 1);

    let mut collector = SliceEventCollector::new();
    for (i, w) in result.warnings.iter().enumerate() {
        let evt = paint_annotation_warning_to_progress_event(
            w,
            String::from("slice-test"),
            String::from("com.host.slice-postprocess-paint-annotator"),
            1_000 + i as u64,
        );
        // Bridge contract: the bridge MUST stamp fatal=false so the
        // collector marks the slice degraded rather than aborting.
        let err = evt.error.as_ref().expect("module_error must carry error");
        assert!(!err.fatal);
        assert_eq!(err.code, 504);
        assert!(err.suggestion.is_some());
        collector.record(evt);
    }

    assert!(
        collector.is_degraded(),
        "non-fatal paint fallback must propagate degraded=true to the slice-level collector"
    );
    assert_eq!(collector.non_fatal_count(), 1);
    assert_eq!(collector.fatal_count(), 0);
}

#[test]
fn paint_annotation_fallback_value_is_deterministic_for_each_semantic() {
    use slicer_host::execute_slice_postprocess_paint_annotation;
    use slicer_host::SlicePostProcessPaintAnnotationWarning as W;
    use slicer_host::SlicePostProcessPaintAnnotationWarningReason as R;

    // Build a request that yields one warning per built-in semantic, then
    // assert each fallback value matches the doc-required deterministic
    // default (Material→ToolIndex(0); flag-typed semantics→Flag(false)).
    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                7,
                vec![region_fixture(
                    "obj",
                    1,
                    vec![polygon(vec![
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 0.0001),
                        (0.0, 10.0),
                    ])],
                )],
            ),
            paint_regions: Arc::new(partially_resolved_builtin_regions(7)),
            required_semantics: vec![
                PaintSemantic::Material,
                PaintSemantic::FuzzySkin,
                PaintSemantic::SupportEnforcer,
                PaintSemantic::SupportBlocker,
            ],
            modifier_projections: vec![],
        })
        .unwrap();

    let by_semantic: HashMap<PaintSemantic, &W> = result
        .warnings
        .iter()
        .map(|w| (w.semantic.clone(), w))
        .collect();

    let m = by_semantic
        .get(&PaintSemantic::Material)
        .expect("Material warning");
    assert_eq!(m.fallback_value, PaintValue::ToolIndex(0));
    assert_eq!(m.reason, R::NumericalEdgeAmbiguity);

    for sem in [
        PaintSemantic::FuzzySkin,
        PaintSemantic::SupportEnforcer,
        PaintSemantic::SupportBlocker,
    ] {
        let w = by_semantic
            .get(&sem)
            .unwrap_or_else(|| panic!("{sem:?} warning"));
        assert_eq!(w.fallback_value, PaintValue::Flag(false));
        assert_eq!(w.reason, R::NumericalEdgeAmbiguity);
    }
}

// Regression: prior to the EPSILON_UNITS tightening (10 → 1), a contour point
// that sat several integer units (sub-µm but well above grid quantization) from
// a paint-region polygon edge was misclassified as "numerical edge ambiguity"
// and emitted a code-504 warning per point. On real slices with per-facet
// projected paint regions this produced hundreds of warnings per layer. After
// the fix, only points within 1 unit (100 nm, integer-grid noise) of an edge
// fire — points further away are silently assigned a region-level default.
#[test]
fn paint_annotation_does_not_warn_for_points_outside_grid_quantization_window() {
    use slicer_host::execute_slice_postprocess_paint_annotation;

    // Paint region triangle (0,0) - (10,0) - (0,10), value ToolIndex(3).
    // Contour point at (10.0, 0.0005) is 5 integer units (= 0.5 µm) above the
    // segment (0,0)-(10,0) extended past its right endpoint, projected onto
    // that segment. Strictly outside the triangle (Include == None).
    let regions = paint_region_ir(
        7,
        vec![(
            PaintSemantic::Material,
            vec![semantic_region(
                "five-unit-offset-object",
                vec![polygon(vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)])],
                PaintValue::ToolIndex(3),
                0,
            )],
        )],
    );

    let result =
        execute_slice_postprocess_paint_annotation(SlicePostProcessPaintAnnotationRequest {
            slice_ir: slice_fixture(
                7,
                vec![region_fixture(
                    "five-unit-offset-object",
                    0,
                    vec![polygon(vec![
                        (0.0, 0.0),
                        (10.0, 0.0),
                        (10.0, 0.0005),
                        (0.0, 10.0),
                    ])],
                )],
            ),
            paint_regions: Arc::new(regions),
            required_semantics: vec![PaintSemantic::Material],
            modifier_projections: vec![],
        })
        .expect("annotation must not error on a 5-unit-offset point");

    assert!(
        !result.degraded,
        "5-unit (0.5 µm) offset is above grid-quantization noise and must not degrade"
    );
    assert!(
        result.warnings.is_empty(),
        "5-unit (0.5 µm) offset must not fire a code-504 warning; got {} warnings: {:?}",
        result.warnings.len(),
        result.warnings,
    );
    // The point is outside all paint regions; it should receive the
    // region-level default value (the only painted region's value) silently.
    let region = &result.slice_ir.regions[0];
    let material = &region.boundary_paint[&PaintSemantic::Material][0];
    assert_eq!(material[2], Some(PaintValue::ToolIndex(3)));
}

// Aggregation contract: per-point warnings within the same
// (layer, object, region, semantic, polygon) group MUST coalesce into one
// progress event whose message reports a count and the first/last contour
// indices, so a structurally-noisy paint region never produces one event per
// contour point.
#[test]
fn paint_annotation_warnings_aggregate_into_one_event_per_polygon_semantic_group() {
    use slicer_host::paint_annotation_warnings_to_progress_events;

    let mk = |cpi: usize, sem: PaintSemantic, poly: usize| SlicePostProcessPaintAnnotationWarning {
        code: 504,
        global_layer_index: 240,
        object_id: String::from("obj-1"),
        region_id: 0,
        semantic: sem,
        polygon_index: poly,
        contour_point_index: cpi,
        fallback_value: PaintValue::Flag(false),
        reason: SlicePostProcessPaintAnnotationWarningReason::NumericalEdgeAmbiguity,
    };

    // 4 warnings on the same (layer/obj/region/semantic/polygon) group → 1 event.
    // 2 warnings on a different semantic → 1 event.
    // 1 warning on a different polygon → 1 event.
    // Expect 3 events total, in first-occurrence order.
    let warnings = vec![
        mk(255, PaintSemantic::FuzzySkin, 2),
        mk(348, PaintSemantic::FuzzySkin, 2),
        mk(662, PaintSemantic::SupportBlocker, 2),
        mk(940, PaintSemantic::FuzzySkin, 2),
        mk(5, PaintSemantic::FuzzySkin, 3),
        mk(700, PaintSemantic::SupportBlocker, 2),
        mk(1001, PaintSemantic::FuzzySkin, 2),
    ];

    let events = paint_annotation_warnings_to_progress_events(
        &warnings,
        String::new(),
        String::from("com.host.slice-postprocess-paint-annotator"),
        0,
    );

    assert_eq!(
        events.len(),
        3,
        "must emit exactly one event per (polygon, semantic) group; got {} events",
        events.len(),
    );

    // First group: FuzzySkin/polygon=2, 4 points (255, 348, 940, 1001),
    // first=255, last=1001.
    let e0_err = events[0].error.as_ref().expect("error payload");
    assert_eq!(e0_err.code, 504);
    assert!(!e0_err.fatal);
    assert!(
        e0_err.message.contains("semantic=FuzzySkin"),
        "msg={}",
        e0_err.message
    );
    assert!(
        e0_err.message.contains("polygon=2"),
        "msg={}",
        e0_err.message
    );
    assert!(
        e0_err.message.contains("on 4 contour points"),
        "msg={}",
        e0_err.message
    );
    assert!(
        e0_err.message.contains("first=255") && e0_err.message.contains("last=1001"),
        "msg={}",
        e0_err.message
    );

    // Second group: SupportBlocker/polygon=2, 2 points (662, 700).
    let e1_err = events[1].error.as_ref().expect("error payload");
    assert!(
        e1_err.message.contains("semantic=SupportBlocker")
            && e1_err.message.contains("polygon=2")
            && e1_err.message.contains("on 2 contour points")
            && e1_err.message.contains("first=662")
            && e1_err.message.contains("last=700"),
        "msg={}",
        e1_err.message,
    );

    // Third group: FuzzySkin/polygon=3, 1 point — singleton uses the original
    // per-point message format (no count/first/last suffix) to avoid noise on
    // genuine isolated ambiguity.
    let e2_err = events[2].error.as_ref().expect("error payload");
    assert!(
        e2_err.message.contains("semantic=FuzzySkin")
            && e2_err.message.contains("polygon=3")
            && e2_err.message.contains("contour_point=5")
            && !e2_err.message.contains("on 1 contour points"),
        "msg={}",
        e2_err.message,
    );

    // Timestamps are deterministic and monotonically increasing from the base.
    assert_eq!(events[0].timestamp_ms, 0);
    assert_eq!(events[1].timestamp_ms, 1);
    assert_eq!(events[2].timestamp_ms, 2);
}
