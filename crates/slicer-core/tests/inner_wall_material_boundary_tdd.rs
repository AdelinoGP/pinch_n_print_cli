#![allow(missing_docs)]
// inner_wall_material_boundary_tdd.rs — AC-2 TDD test.
//
// Verifies that `build_wall_flags(…, is_outer: false, …)` produces
// `WallBoundaryType::MaterialBoundary` (not `Interior`) for an inner wall polygon
// whose `segment_annotations` span a material boundary between tools 1 and 2.
// Also verifies that per-vertex `tool_index` reflects per-vertex tool membership
// for the inner wall (T-021).

use std::collections::HashMap;

use slicer_core::perimeter_utils::build_wall_flags;
use slicer_ir::{
    ExPolygon, PaintSemantic, PaintValue, Point2, Polygon, WallBoundaryType, WallFeatureFlags,
};

/// Build a `segment_annotations` map for a 4-point polygon where the first two
/// points belong to tool 1 and the last two to tool 2.
fn two_tool_annotations(poly_idx: usize) -> HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> {
    // poly_idx slots before the one we care about filled with empty vecs.
    let num_polys = poly_idx + 1;
    let mut per_poly: Vec<Vec<Option<PaintValue>>> = vec![vec![]; num_polys];
    per_poly[poly_idx] = vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
    ];
    let mut annotations = HashMap::new();
    annotations.insert(PaintSemantic::Material, per_poly);
    annotations
}

/// AC-2: inner-wall `build_wall_flags` with `is_outer=false` must produce
/// `WallBoundaryType::MaterialBoundary` (not `Interior`) when the region's
/// segment_annotations cross a tool boundary.
#[test]
fn inner_wall_material_boundary_not_interior() {
    let poly_idx = 0_usize;
    let num_points = 4;
    let annotations = two_tool_annotations(poly_idx);

    let (flags, boundary_type) =
        build_wall_flags(num_points, poly_idx, &annotations, false, None, None, false);

    assert_eq!(flags.len(), num_points, "flag count must match num_points");

    match &boundary_type {
        WallBoundaryType::MaterialBoundary { segments } => {
            assert!(
                !segments.is_empty(),
                "inner wall with tool transitions must produce at least one MaterialBoundarySegment"
            );
            // Expect exactly 2 transitions: [1→2] at index 1, [2→1] at index 3 (wrap).
            assert_eq!(
                segments.len(),
                2,
                "two-tool four-point polygon should produce 2 transitions; got {segments:?}"
            );
            let t0 = &segments[0];
            assert_eq!(
                t0.near_tool,
                Some(1),
                "first transition near_tool should be 1"
            );
            assert_eq!(
                t0.far_tool,
                Some(2),
                "first transition far_tool should be 2"
            );
        }
        other => {
            panic!("expected WallBoundaryType::MaterialBoundary for inner wall; got {other:?}")
        }
    }
}

/// AC-2 (T-021 sub-case): per-vertex `tool_index` must reflect actual tool membership
/// for the inner wall (not all-None).
#[test]
fn inner_wall_per_vertex_tool_index_reflects_membership() {
    let poly_idx = 0_usize;
    let num_points = 4;
    let annotations = two_tool_annotations(poly_idx);

    let (flags, _) = build_wall_flags(num_points, poly_idx, &annotations, false, None, None, false);

    // First two vertices → tool 1
    assert_eq!(
        flags[0].tool_index,
        Some(1),
        "vertex 0 should be tool 1; got {:?}",
        flags[0].tool_index
    );
    assert_eq!(
        flags[1].tool_index,
        Some(1),
        "vertex 1 should be tool 1; got {:?}",
        flags[1].tool_index
    );
    // Last two vertices → tool 2
    assert_eq!(
        flags[2].tool_index,
        Some(2),
        "vertex 2 should be tool 2; got {:?}",
        flags[2].tool_index
    );
    assert_eq!(
        flags[3].tool_index,
        Some(2),
        "vertex 3 should be tool 2; got {:?}",
        flags[3].tool_index
    );
}

/// AC-2: inner wall with no Material paint must return `WallBoundaryType::Interior`
/// (design invariant: empty paint → Interior for inner walls, ExteriorSurface for outer).
#[test]
fn inner_wall_no_paint_returns_interior() {
    let annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> = HashMap::new();

    let (flags, boundary_type) = build_wall_flags(4, 0, &annotations, false, None, None, false);

    assert_eq!(flags.len(), 4);
    assert_eq!(
        boundary_type,
        WallBoundaryType::Interior,
        "unpainted inner wall must return Interior (is_outer=false, no Material paint)"
    );
}

/// AC-2 (sibling): outer wall with no Material paint must return `ExteriorSurface`.
#[test]
fn outer_wall_no_paint_returns_exterior_surface() {
    let annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> = HashMap::new();

    let (flags, boundary_type) = build_wall_flags(4, 0, &annotations, true, None, None, false);

    assert_eq!(flags.len(), 4);
    assert_eq!(
        boundary_type,
        WallBoundaryType::ExteriorSurface,
        "unpainted outer wall must return ExteriorSurface (is_outer=true, no Material paint)"
    );
}

/// AC-2: multi-poly annotations — poly_idx selects the correct sub-vec.
#[test]
fn inner_wall_correct_poly_idx_selected() {
    let poly_idx = 2_usize;
    let num_points = 4;
    let annotations = two_tool_annotations(poly_idx);

    let (flags, boundary_type) =
        build_wall_flags(num_points, poly_idx, &annotations, false, None, None, false);

    assert_eq!(flags.len(), num_points);
    assert!(
        matches!(boundary_type, WallBoundaryType::MaterialBoundary { .. }),
        "poly_idx=2 should still find the two-tool annotations; got {boundary_type:?}"
    );
}

#[test]
fn ineffective_annotations_skip_reprojection_but_preserve_length_and_fallback() {
    let mut annotations = HashMap::new();
    annotations.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::Scalar(0.5))]],
    );
    annotations.insert(
        PaintSemantic::FuzzySkin,
        vec![vec![Some(PaintValue::Flag(false))]],
    );
    let original = vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100, y: 0 },
                Point2 { x: 100, y: 100 },
                Point2 { x: 0, y: 100 },
            ],
        },
        holes: vec![],
    }];
    let ring = [Point2 { x: 10, y: 10 }];

    let (flags, boundary_type) = build_wall_flags(
        5,
        0,
        &annotations,
        true,
        Some(&ring),
        Some(&original),
        false,
    );

    assert_eq!(flags.len(), 5, "including a closing slot must be preserved");
    assert!(flags.iter().all(|flag| flag.tool_index.is_none()));
    assert!(flags.iter().all(|flag| !flag.fuzzy_skin));
    assert_eq!(boundary_type, WallBoundaryType::ExteriorSurface);
}

#[test]
fn variant_fuzzy_seeds_fastpath_for_inner_and_outer_walls() {
    let annotations = HashMap::new();
    for is_outer in [false, true] {
        let (flags, boundary_type) =
            build_wall_flags(3, 0, &annotations, is_outer, None, None, true);
        assert_eq!(flags.len(), 3);
        assert!(flags.iter().all(|flag| flag.fuzzy_skin));
        assert_eq!(
            boundary_type,
            if is_outer {
                WallBoundaryType::ExteriorSurface
            } else {
                WallBoundaryType::Interior
            }
        );
    }
}

#[test]
fn annotation_guard_cases_have_exact_expected_outputs() {
    struct GuardCase {
        name: &'static str,
        annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
        poly_idx: usize,
        is_outer: bool,
        num_points: usize,
        expected_flags: Vec<WallFeatureFlags>,
        expected_boundary: WallBoundaryType,
    }

    let mut uniform_material = HashMap::new();
    uniform_material.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::ToolIndex(7)); 2]],
    );
    let mut empty = HashMap::new();
    empty.insert(PaintSemantic::Material, vec![vec![]]);
    let mut all_none = HashMap::new();
    all_none.insert(PaintSemantic::Material, vec![vec![None, None, None]]);
    let mut custom_only = HashMap::new();
    custom_only.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::Custom("ignored".to_string()))]],
    );
    let mut ineffective = HashMap::new();
    ineffective.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::Scalar(0.25))]],
    );
    ineffective.insert(
        PaintSemantic::FuzzySkin,
        vec![vec![Some(PaintValue::Flag(false))]],
    );

    let cases = [
        GuardCase {
            name: "effective material uses index mode",
            annotations: uniform_material,
            poly_idx: 0,
            is_outer: true,
            num_points: 2,
            expected_flags: vec![
                WallFeatureFlags {
                    tool_index: Some(7),
                    ..Default::default()
                },
                WallFeatureFlags {
                    tool_index: Some(7),
                    ..Default::default()
                },
            ],
            expected_boundary: WallBoundaryType::ExteriorSurface,
        },
        GuardCase {
            name: "empty annotation vector",
            annotations: empty,
            poly_idx: 0,
            is_outer: false,
            num_points: 2,
            expected_flags: vec![WallFeatureFlags::default(); 2],
            expected_boundary: WallBoundaryType::Interior,
        },
        GuardCase {
            name: "all none annotations",
            annotations: all_none,
            poly_idx: 0,
            is_outer: true,
            num_points: 3,
            expected_flags: vec![WallFeatureFlags::default(); 3],
            expected_boundary: WallBoundaryType::ExteriorSurface,
        },
        GuardCase {
            name: "custom annotation",
            annotations: custom_only,
            poly_idx: 0,
            is_outer: false,
            num_points: 1,
            expected_flags: vec![WallFeatureFlags::default()],
            expected_boundary: WallBoundaryType::Interior,
        },
        GuardCase {
            name: "ineffective typed paint",
            annotations: ineffective,
            poly_idx: 0,
            is_outer: false,
            num_points: 1,
            expected_flags: vec![WallFeatureFlags::default()],
            expected_boundary: WallBoundaryType::Interior,
        },
        GuardCase {
            name: "zero points",
            annotations: HashMap::new(),
            poly_idx: 99,
            is_outer: true,
            num_points: 0,
            expected_flags: vec![],
            expected_boundary: WallBoundaryType::ExteriorSurface,
        },
    ];

    for case in cases {
        let (flags, boundary) = build_wall_flags(
            case.num_points,
            case.poly_idx,
            &case.annotations,
            case.is_outer,
            None,
            None,
            false,
        );
        assert_eq!(flags, case.expected_flags, "flags for {}", case.name);
        assert_eq!(
            boundary, case.expected_boundary,
            "boundary for {}",
            case.name
        );
    }
}

#[test]
fn reprojection_guard_scans_all_polygons_even_with_wrong_poly_idx() {
    let mut annotations = HashMap::new();
    annotations.insert(
        PaintSemantic::Material,
        vec![vec![None, None], vec![Some(PaintValue::ToolIndex(4))]],
    );
    let original = vec![
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: 0, y: 0 }, Point2 { x: 10, y: 0 }],
            },
            holes: vec![],
        },
        ExPolygon {
            contour: Polygon {
                points: vec![Point2 { x: 1000, y: 1000 }],
            },
            holes: vec![],
        },
    ];
    let ring = [Point2 { x: 1001, y: 1001 }];

    let (flags, boundary) = build_wall_flags(
        2,
        999,
        &annotations,
        false,
        Some(&ring),
        Some(&original),
        false,
    );

    assert_eq!(
        flags,
        vec![
            WallFeatureFlags {
                tool_index: Some(4),
                ..Default::default()
            },
            WallFeatureFlags {
                tool_index: Some(4),
                ..Default::default()
            },
        ]
    );
    assert_eq!(boundary, WallBoundaryType::Interior);
}

#[test]
fn empty_reprojection_ring_is_safe_when_annotations_are_ineffective() {
    let mut annotations = HashMap::new();
    annotations.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::Custom("ignored".to_string()))]],
    );
    let original = vec![ExPolygon {
        contour: Polygon {
            points: vec![Point2 { x: 0, y: 0 }],
        },
        holes: vec![],
    }];

    let (flags, boundary) =
        build_wall_flags(2, 0, &annotations, true, Some(&[]), Some(&original), false);

    assert_eq!(flags, vec![WallFeatureFlags::default(); 2]);
    assert_eq!(boundary, WallBoundaryType::ExteriorSurface);
}
