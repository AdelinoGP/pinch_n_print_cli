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
use slicer_ir::{PaintSemantic, PaintValue, WallBoundaryType};

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
        build_wall_flags(num_points, poly_idx, &annotations, false, None, None);

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

    let (flags, _) = build_wall_flags(num_points, poly_idx, &annotations, false, None, None);

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

    let (flags, boundary_type) = build_wall_flags(4, 0, &annotations, false, None, None);

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

    let (flags, boundary_type) = build_wall_flags(4, 0, &annotations, true, None, None);

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
        build_wall_flags(num_points, poly_idx, &annotations, false, None, None);

    assert_eq!(flags.len(), num_points);
    assert!(
        matches!(boundary_type, WallBoundaryType::MaterialBoundary { .. }),
        "poly_idx=2 should still find the two-tool annotations; got {boundary_type:?}"
    );
}
