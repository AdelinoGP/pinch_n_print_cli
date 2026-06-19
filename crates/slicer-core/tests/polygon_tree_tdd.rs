//! TDD tests for polygon_tree — AC-4: hole/contour containment tree builder.

use slicer_core::polygon_tree::build_polygon_tree;
use slicer_ir::slice_ir::{ExPolygon, Point2, Polygon};

/// Helper: build a simple rectangular ExPolygon (contour only, no holes).
/// Coordinates are in mm; internally converted to raw units (1 unit = 100 nm,
/// so 1 mm = 10_000 units).
fn rect_mm(x0: f64, y0: f64, x1: f64, y1: f64) -> ExPolygon {
    let to_units = |v: f64| -> i64 { (v * 10_000.0) as i64 };
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: to_units(x0),
                    y: to_units(y0),
                },
                Point2 {
                    x: to_units(x1),
                    y: to_units(y0),
                },
                Point2 {
                    x: to_units(x1),
                    y: to_units(y1),
                },
                Point2 {
                    x: to_units(x0),
                    y: to_units(y1),
                },
            ],
        },
        holes: vec![],
    }
}

/// AC-4: build_polygon_tree returns correct two-level forest.
///
/// Input polygons (by index):
///   0 - outer_square:  large square  (0,0)..(20,20) mm
///   1 - hole_a:        small square  (2,2)..(5,5)   mm  — inside outer_square
///   2 - hole_b:        small square  (10,10)..(13,13) mm — inside outer_square
///   3 - isolated:      square (30,30)..(35,35) mm  — completely outside outer_square
///
/// Expected tree:
///   root[0] outer_square  is_contour=true
///     child[0] hole_a     is_contour=false
///     child[1] hole_b     is_contour=false
///   root[1] isolated      is_contour=true
#[test]
fn build_polygon_tree_two_level_forest() {
    let outer_square = rect_mm(0.0, 0.0, 20.0, 20.0);
    let hole_a = rect_mm(2.0, 2.0, 5.0, 5.0);
    let hole_b = rect_mm(10.0, 10.0, 13.0, 13.0);
    let isolated = rect_mm(30.0, 30.0, 35.0, 35.0);

    let roots = build_polygon_tree(&[outer_square, hole_a, hole_b, isolated]);

    // Exactly two roots.
    assert_eq!(
        roots.len(),
        2,
        "expected exactly 2 roots, got {}",
        roots.len()
    );

    // Roots are in ascending polygon_index order.
    assert_eq!(
        roots[0].polygon_index, 0,
        "first root should be polygon_index 0 (outer_square)"
    );
    assert_eq!(
        roots[1].polygon_index, 3,
        "second root should be polygon_index 3 (isolated)"
    );

    // Both roots are contours (depth 0 → even → is_contour = true).
    assert!(roots[0].is_contour, "outer_square root must be a contour");
    assert!(roots[1].is_contour, "isolated root must be a contour");

    // outer_square has exactly two children: hole_a (index 1) and hole_b (index 2).
    let outer_children = &roots[0].children;
    assert_eq!(
        outer_children.len(),
        2,
        "outer_square should have 2 children, got {}",
        outer_children.len()
    );
    assert_eq!(
        outer_children[0].polygon_index, 1,
        "first child of outer_square should be polygon_index 1 (hole_a)"
    );
    assert_eq!(
        outer_children[1].polygon_index, 2,
        "second child of outer_square should be polygon_index 2 (hole_b)"
    );

    // Children at depth 1 are holes (odd depth → is_contour = false).
    assert!(
        !outer_children[0].is_contour,
        "hole_a at depth 1 must be a hole"
    );
    assert!(
        !outer_children[1].is_contour,
        "hole_b at depth 1 must be a hole"
    );

    // Children of holes have no grandchildren in this fixture.
    assert!(
        outer_children[0].children.is_empty(),
        "hole_a should have no children"
    );
    assert!(
        outer_children[1].children.is_empty(),
        "hole_b should have no children"
    );

    // isolated has no children.
    assert!(
        roots[1].children.is_empty(),
        "isolated root should have no children"
    );
}
