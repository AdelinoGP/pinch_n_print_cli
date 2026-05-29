//! TDD tests for real host-service implementations.
//!
//! These tests prove that the geometry ops (clip, offset, simplify) produce
//! non-placeholder results via real Clipper2 dispatch, that unsupported mesh
//! operations fail diagnostically, and that timing is monotonic.

use slicer_runtime::wit_host::{
    ir_clip_polygons, ir_offset_polygons, ir_simplify_polygon, HostExecutionContextBuilder,
};

fn make_square_ir(x: i64, y: i64, size: i64) -> slicer_ir::ExPolygon {
    slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 { x, y },
                slicer_ir::Point2 { x: x + size, y },
                slicer_ir::Point2 {
                    x: x + size,
                    y: y + size,
                },
                slicer_ir::Point2 { x, y: y + size },
            ],
        },
        holes: Vec::new(),
    }
}

// â”€â”€ A. Clip polygons â€” real Clipper2 results â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn clip_union_merges_overlapping_squares() {
    // Two overlapping squares: union should produce a single merged polygon
    // that is larger than either input.
    let a = make_square_ir(0, 0, 100_000); // 10mm square
    let b = make_square_ir(50_000, 0, 100_000); // offset by 5mm

    let result = ir_clip_polygons(&[a], &[b], slicer_core::polygon_ops::ClipOperation::Union);

    // Union of two overlapping squares produces at least one polygon
    assert!(
        !result.is_empty(),
        "union should produce at least one polygon"
    );
    // The union polygon should have more than 4 points (it's not just one square)
    let total_points: usize = result.iter().map(|p| p.contour.points.len()).sum();
    assert!(
        total_points >= 6,
        "union of overlapping squares should have >=6 vertices, got {total_points}"
    );
}

#[test]
fn clip_intersection_produces_overlap_region() {
    let a = make_square_ir(0, 0, 100_000);
    let b = make_square_ir(50_000, 0, 100_000);

    let result = ir_clip_polygons(
        &[a],
        &[b],
        slicer_core::polygon_ops::ClipOperation::Intersection,
    );

    assert!(
        !result.is_empty(),
        "intersection of overlapping squares should produce output"
    );
    // Intersection should be smaller than either input
    let pts = &result[0].contour.points;
    assert_eq!(
        pts.len(),
        4,
        "intersection of two overlapping squares is a rectangle"
    );
}

#[test]
fn clip_difference_removes_overlap() {
    let a = make_square_ir(0, 0, 100_000);
    let b = make_square_ir(50_000, 0, 100_000);

    let result = ir_clip_polygons(
        &[a],
        &[b],
        slicer_core::polygon_ops::ClipOperation::Difference,
    );

    assert!(!result.is_empty(), "difference should produce output");
}

#[test]
fn clip_with_empty_clip_set_returns_subject() {
    let a = make_square_ir(0, 0, 100_000);
    let result = ir_clip_polygons(
        &[a.clone()],
        &[],
        slicer_core::polygon_ops::ClipOperation::Union,
    );

    assert!(
        !result.is_empty(),
        "union with empty clip should return subject"
    );
}

// â”€â”€ B. Offset polygons â€” real Clipper2 results â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn offset_positive_expands_polygon() {
    let square = make_square_ir(0, 0, 100_000); // 10mm
    let result = ir_offset_polygons(
        &[square],
        1.0, // 1mm outward
        slicer_core::polygon_ops::OffsetJoinType::Miter,
    );

    assert!(!result.is_empty(), "offset should produce output");
    // Expanded polygon should have larger extents
    let max_x = result
        .iter()
        .flat_map(|p| p.contour.points.iter().map(|pt| pt.x))
        .max()
        .unwrap();
    assert!(
        max_x > 100_000,
        "offset +1mm should expand beyond 10mm, got max_x={max_x}"
    );
}

#[test]
fn offset_negative_shrinks_polygon() {
    let square = make_square_ir(0, 0, 100_000); // 10mm
    let result = ir_offset_polygons(
        &[square],
        -1.0, // 1mm inward
        slicer_core::polygon_ops::OffsetJoinType::Miter,
    );

    assert!(
        !result.is_empty(),
        "inward offset of large polygon should still produce output"
    );
    let max_x = result
        .iter()
        .flat_map(|p| p.contour.points.iter().map(|pt| pt.x))
        .max()
        .unwrap();
    assert!(
        max_x < 100_000,
        "offset -1mm should shrink below 10mm, got max_x={max_x}"
    );
}

// â”€â”€ C. Simplify polygon â€” collinearity removal â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn simplify_removes_collinear_points() {
    // A square with an extra collinear point on one edge
    let pts = vec![
        slicer_ir::Point2 { x: 0, y: 0 },
        slicer_ir::Point2 { x: 50, y: 0 }, // collinear with neighbors
        slicer_ir::Point2 { x: 100, y: 0 },
        slicer_ir::Point2 { x: 100, y: 100 },
        slicer_ir::Point2 { x: 0, y: 100 },
    ];

    let result = ir_simplify_polygon(pts);
    assert_eq!(
        result.len(),
        4,
        "collinear point should be removed, got {}",
        result.len()
    );
}

#[test]
fn simplify_preserves_non_collinear_polygon() {
    let pts = vec![
        slicer_ir::Point2 { x: 0, y: 0 },
        slicer_ir::Point2 { x: 100, y: 0 },
        slicer_ir::Point2 { x: 100, y: 100 },
        slicer_ir::Point2 { x: 0, y: 100 },
    ];

    let result = ir_simplify_polygon(pts);
    assert_eq!(result.len(), 4, "no collinear points to remove");
}

#[test]
fn simplify_handles_degenerate_input() {
    let pts = vec![
        slicer_ir::Point2 { x: 0, y: 0 },
        slicer_ir::Point2 { x: 100, y: 0 },
    ];
    let result = ir_simplify_polygon(pts);
    assert_eq!(result.len(), 2, "degenerate input should be returned as-is");
}

// â”€â”€ D. Mesh queries â€” explicit diagnostic failures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn object_bounds_fails_with_diagnostic_when_mesh_not_wired() {
    use slicer_runtime::wit_host::layer::slicer::world_layer::host_services as hs;

    let mut ctx = HostExecutionContextBuilder::new("test-mod", 0.0, 0.0).build();

    let result = hs::Host::object_bounds(&mut ctx, "obj-1".to_string());
    assert!(
        result.is_err(),
        "object_bounds should fail when mesh is not wired"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("OBJECT_NOT_FOUND") || msg.contains("object-bounds"),
        "error should identify the missing object: {msg}"
    );
    assert!(msg.contains("obj-1"), "error should name the object: {msg}");
}

#[test]
fn raycast_returns_none_when_mesh_not_wired() {
    use slicer_runtime::wit_host::layer::slicer::world_layer::host_services as hs;

    let mut ctx = HostExecutionContextBuilder::new("test-mod", 0.0, 0.0).build();

    let result = hs::Host::raycast_z_down(&mut ctx, "obj-1".to_string(), 5.0, 5.0, 10.0);
    assert!(result.is_ok(), "raycast should succeed (returning None)");
    assert_eq!(
        result.unwrap(),
        None,
        "raycast should return None when mesh not wired"
    );
}

// â”€â”€ E. Timing semantics â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn now_us_is_monotonic_within_call() {
    use slicer_runtime::wit_host::layer::slicer::world_layer::host_services as hs;

    let mut ctx = HostExecutionContextBuilder::new("test-mod", 0.0, 0.0).build();

    let t1 = hs::Host::now_us(&mut ctx).unwrap();
    // Do a small amount of work to ensure time advances
    let _ = (0..1000).sum::<u64>();
    let t2 = hs::Host::now_us(&mut ctx).unwrap();
    assert!(t2 >= t1, "now_us should be monotonic: t1={t1}, t2={t2}");
}

// â”€â”€ F. Guest-visible behavior changes with different inputs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn clip_result_changes_when_clip_polygon_moves() {
    let subject = make_square_ir(0, 0, 100_000);
    let clip_near = make_square_ir(50_000, 0, 100_000);
    let clip_far = make_square_ir(200_000, 0, 100_000); // no overlap

    let result_near = ir_clip_polygons(
        &[subject.clone()],
        &[clip_near],
        slicer_core::polygon_ops::ClipOperation::Intersection,
    );
    let result_far = ir_clip_polygons(
        &[subject],
        &[clip_far],
        slicer_core::polygon_ops::ClipOperation::Intersection,
    );

    assert!(
        !result_near.is_empty(),
        "near clip should produce intersection"
    );
    assert!(
        result_far.is_empty(),
        "far clip should produce empty intersection"
    );
}
