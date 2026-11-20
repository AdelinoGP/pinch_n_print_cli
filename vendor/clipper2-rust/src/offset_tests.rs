/*******************************************************************************
* Comprehensive tests for the offset module                                   *
* Direct port of C++ test cases from TestOffsets.cpp and                       *
* TestOffsetOrientation.cpp                                                   *
* Date: 2025                                                                  *
*******************************************************************************/

use crate::core::{area, constants, Path64, Paths64, Point64};
use crate::offset::{ClipperOffset, EndType, JoinType};

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Create a simple square path centered at origin
fn make_square(size: i64) -> Path64 {
    vec![
        Point64::new(-size, -size),
        Point64::new(size, -size),
        Point64::new(size, size),
        Point64::new(-size, size),
    ]
}

/// Create a diamond (rotated square) path
fn make_diamond(size: i64) -> Path64 {
    vec![
        Point64::new(0, -size),
        Point64::new(size, 0),
        Point64::new(0, size),
        Point64::new(-size, 0),
    ]
}

/// Create a triangle path
fn make_triangle(size: i64) -> Path64 {
    vec![
        Point64::new(0, -size),
        Point64::new(size, size),
        Point64::new(-size, size),
    ]
}

/// Create a simple open path (horizontal line)
fn make_open_line(length: i64) -> Path64 {
    vec![Point64::new(0, 0), Point64::new(length, 0)]
}

/// Create an L-shaped open path
fn make_l_path(size: i64) -> Path64 {
    vec![
        Point64::new(0, 0),
        Point64::new(size, 0),
        Point64::new(size, size),
    ]
}

// ---------------------------------------------------------------------------
// Basic enum tests
// ---------------------------------------------------------------------------

#[test]
fn test_join_type_values() {
    // Verify all JoinType variants exist and are distinct
    let types = [
        JoinType::Square,
        JoinType::Bevel,
        JoinType::Round,
        JoinType::Miter,
    ];
    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            assert_ne!(types[i], types[j]);
        }
    }
}

#[test]
fn test_end_type_values() {
    // Verify all EndType variants exist and are distinct
    let types = [
        EndType::Polygon,
        EndType::Joined,
        EndType::Butt,
        EndType::Square,
        EndType::Round,
    ];
    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            assert_ne!(types[i], types[j]);
        }
    }
}

// ---------------------------------------------------------------------------
// Construction and configuration tests
// ---------------------------------------------------------------------------

#[test]
fn test_clipper_offset_default_construction() {
    let co = ClipperOffset::new_default();
    assert_eq!(co.error_code(), 0);
    assert_eq!(co.miter_limit(), 2.0);
    assert_eq!(co.arc_tolerance(), 0.0);
    assert!(!co.preserve_collinear());
    assert!(!co.reverse_solution());
}

#[test]
fn test_clipper_offset_custom_construction() {
    let co = ClipperOffset::new(3.0, 0.5, true, true);
    assert_eq!(co.miter_limit(), 3.0);
    assert_eq!(co.arc_tolerance(), 0.5);
    assert!(co.preserve_collinear());
    assert!(co.reverse_solution());
}

#[test]
fn test_clipper_offset_setters() {
    let mut co = ClipperOffset::new_default();

    co.set_miter_limit(5.0);
    assert_eq!(co.miter_limit(), 5.0);

    co.set_arc_tolerance(1.0);
    assert_eq!(co.arc_tolerance(), 1.0);

    co.set_preserve_collinear(true);
    assert!(co.preserve_collinear());

    co.set_reverse_solution(true);
    assert!(co.reverse_solution());
}

#[test]
fn test_clipper_offset_clear() {
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);
    co.clear();

    // After clearing, executing should produce no output
    let mut result = Paths64::new();
    co.execute(10.0, &mut result);
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Polygon offset tests (closed paths)
// ---------------------------------------------------------------------------

#[test]
fn test_offset_square_inflate_miter() {
    // Inflating a square with miter joins should produce a larger square-like shape
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty(), "Result should not be empty");
    // The inflated area should be larger than the original
    let original_area = area(&square).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(
        result_area > original_area,
        "Inflated area {} should be > original area {}",
        result_area,
        original_area
    );
}

#[test]
fn test_offset_square_shrink_miter() {
    // Shrinking a square should produce a smaller square-like shape.
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(-10.0, &mut result);

    assert!(
        !result.is_empty(),
        "Shrinking a square by 10 should produce a non-empty result"
    );
    let original_area = area(&square).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(
        result_area < original_area,
        "Shrunk area {} should be < original area {}",
        result_area,
        original_area
    );
}

#[test]
fn test_offset_square_inflate_round() {
    // Inflating with round joins should produce a rounded shape
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Round, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty(), "Result should not be empty");
    let original_area = area(&square).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // Round joins produce slightly less area than miter
    assert!(result_area > original_area);
}

#[test]
fn test_offset_square_inflate_bevel() {
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Bevel, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty(), "Result should not be empty");
    let original_area = area(&square).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(result_area > original_area);
}

#[test]
fn test_offset_square_inflate_square_join() {
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Square, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty(), "Result should not be empty");
    let original_area = area(&square).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(result_area > original_area);
}

#[test]
fn test_offset_triangle_inflate() {
    let mut co = ClipperOffset::new_default();
    let triangle = make_triangle(100);
    co.add_path(&triangle, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
    let original_area = area(&triangle).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(result_area > original_area);
}

#[test]
fn test_offset_diamond_inflate() {
    let mut co = ClipperOffset::new_default();
    let diamond = make_diamond(100);
    co.add_path(&diamond, JoinType::Round, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(20.0, &mut result);

    assert!(!result.is_empty());
    let original_area = area(&diamond).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(result_area > original_area);
}

// ---------------------------------------------------------------------------
// Shrinking to nothing
// ---------------------------------------------------------------------------

#[test]
fn test_offset_shrink_to_nothing() {
    // Shrinking a small square by more than its half-width should result in nothing
    let mut co = ClipperOffset::new_default();
    let square = make_square(10);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(-20.0, &mut result);

    // The square is 20x20, shrinking by 20 should eliminate it
    assert!(
        result.is_empty(),
        "Over-shrunk square should produce empty result"
    );
}

// ---------------------------------------------------------------------------
// Insignificant delta
// ---------------------------------------------------------------------------

#[test]
fn test_offset_insignificant_delta() {
    // Delta < 0.5 should just return the original paths
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(0.1, &mut result);

    assert_eq!(result.len(), 1, "Should return the original path");
    assert_eq!(result[0].len(), square.len());
}

// ---------------------------------------------------------------------------
// Open path tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_open_path_butt_end() {
    let mut co = ClipperOffset::new_default();
    let line = make_open_line(200);
    co.add_path(&line, JoinType::Miter, EndType::Butt);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty(), "Open path offset should produce result");
    // The result should be a closed polygon surrounding the line
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // Expected: approximately 200 * 20 = 4000 (line length * 2*delta)
    assert!(
        result_area > 3000.0,
        "Area {} should be approximately 4000",
        result_area
    );
}

#[test]
fn test_offset_open_path_square_end() {
    let mut co = ClipperOffset::new_default();
    let line = make_open_line(200);
    co.add_path(&line, JoinType::Miter, EndType::Square);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // Square ends extend beyond the line, so area should be > butt
    assert!(result_area > 4000.0);
}

#[test]
fn test_offset_open_path_round_end() {
    let mut co = ClipperOffset::new_default();
    let line = make_open_line(200);
    co.add_path(&line, JoinType::Round, EndType::Round);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // Round ends add semicircles, area should be > butt but < square
    assert!(result_area > 3000.0);
}

#[test]
fn test_offset_open_path_joined_end() {
    let mut co = ClipperOffset::new_default();
    let line = make_open_line(200);
    co.add_path(&line, JoinType::Miter, EndType::Joined);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
}

// ---------------------------------------------------------------------------
// L-shaped path tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_l_shape_miter() {
    let mut co = ClipperOffset::new_default();
    let l_path = make_l_path(100);
    co.add_path(&l_path, JoinType::Miter, EndType::Butt);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
}

#[test]
fn test_offset_l_shape_round() {
    let mut co = ClipperOffset::new_default();
    let l_path = make_l_path(100);
    co.add_path(&l_path, JoinType::Round, EndType::Round);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
}

// ---------------------------------------------------------------------------
// Single point tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_single_point_round() {
    let mut co = ClipperOffset::new_default();
    let point = vec![Point64::new(100, 100)];
    co.add_path(&point, JoinType::Round, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(20.0, &mut result);

    assert!(
        !result.is_empty(),
        "Single point offset should produce a circle"
    );
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // Expected: approximately PI * 20^2 = ~1257
    let expected_area = constants::PI * 20.0 * 20.0;
    assert!(
        (result_area - expected_area).abs() < expected_area * 0.15,
        "Circle area {} should be approximately {}",
        result_area,
        expected_area
    );
}

#[test]
fn test_offset_single_point_square() {
    let mut co = ClipperOffset::new_default();
    let point = vec![Point64::new(100, 100)];
    co.add_path(&point, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(20.0, &mut result);

    assert!(
        !result.is_empty(),
        "Single point offset should produce a square"
    );
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // Expected: approximately (2*20)^2 = 1600
    // The rect is (x-d, y-d) to (x+d, y+d) where d = ceil(20) = 20
    let expected_area = (2.0 * 20.0) * (2.0 * 20.0);
    assert!(
        (result_area - expected_area).abs() < expected_area * 0.15,
        "Square area {} should be approximately {}",
        result_area,
        expected_area
    );
}

#[test]
fn test_offset_single_point_small_delta() {
    // Delta < 1 for single point should produce nothing
    let mut co = ClipperOffset::new_default();
    let point = vec![Point64::new(100, 100)];
    co.add_path(&point, JoinType::Round, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(0.5, &mut result);

    // group_delta < 1 means the single-point path is skipped
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Multiple paths tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_multiple_paths() {
    let mut co = ClipperOffset::new_default();

    let square1 = vec![
        Point64::new(0, 0),
        Point64::new(100, 0),
        Point64::new(100, 100),
        Point64::new(0, 100),
    ];
    let square2 = vec![
        Point64::new(200, 0),
        Point64::new(300, 0),
        Point64::new(300, 100),
        Point64::new(200, 100),
    ];

    co.add_paths(
        &vec![square1.clone(), square2.clone()],
        JoinType::Miter,
        EndType::Polygon,
    );

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(
        result.len() >= 2,
        "Should produce at least 2 output paths for 2 non-overlapping inputs"
    );
}

#[test]
fn test_offset_add_paths_empty() {
    let mut co = ClipperOffset::new_default();
    let empty: Paths64 = vec![];
    co.add_paths(&empty, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Miter limit tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_miter_limit() {
    // With a low miter limit, sharp corners should be clipped
    let mut co_low = ClipperOffset::new(1.0, 0.0, false, false);
    let mut co_high = ClipperOffset::new(10.0, 0.0, false, false);

    let triangle = make_triangle(100);
    co_low.add_path(&triangle, JoinType::Miter, EndType::Polygon);
    co_high.add_path(&triangle, JoinType::Miter, EndType::Polygon);

    let mut result_low = Paths64::new();
    let mut result_high = Paths64::new();
    co_low.execute(10.0, &mut result_low);
    co_high.execute(10.0, &mut result_high);

    assert!(!result_low.is_empty());
    assert!(!result_high.is_empty());
    // Higher miter limit allows sharper corners = larger area
    let area_low: f64 = result_low.iter().map(|p| area(p).abs()).sum();
    let area_high: f64 = result_high.iter().map(|p| area(p).abs()).sum();
    assert!(
        area_high >= area_low,
        "Higher miter limit should produce >= area: high={}, low={}",
        area_high,
        area_low
    );
}

// ---------------------------------------------------------------------------
// Orientation tests (matching TestOffsetOrientation.cpp)
// ---------------------------------------------------------------------------

#[test]
fn test_offset_orientation_positive() {
    // Counter-clockwise polygon (positive area) should stay counter-clockwise after inflate
    let mut co = ClipperOffset::new_default();
    let ccw_square = vec![
        Point64::new(0, 0),
        Point64::new(100, 0),
        Point64::new(100, 100),
        Point64::new(0, 100),
    ];
    let original_area = area(&ccw_square);
    co.add_path(&ccw_square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
    // The result orientation should match the input orientation
    let result_area = area(&result[0]);
    assert!(
        (original_area > 0.0) == (result_area > 0.0),
        "Orientation should be preserved: original={}, result={}",
        original_area,
        result_area
    );
}

#[test]
fn test_offset_orientation_negative() {
    // Clockwise polygon (negative area) should stay clockwise after inflate
    let mut co = ClipperOffset::new_default();
    let cw_square = vec![
        Point64::new(0, 100),
        Point64::new(100, 100),
        Point64::new(100, 0),
        Point64::new(0, 0),
    ];
    let original_area = area(&cw_square);
    co.add_path(&cw_square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
    let result_area = area(&result[0]);
    assert!(
        (original_area > 0.0) == (result_area > 0.0),
        "Orientation should be preserved: original={}, result={}",
        original_area,
        result_area
    );
}

// ---------------------------------------------------------------------------
// Two-point path tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_two_point_polygon() {
    // A two-point "polygon" should be treated as a line with 180-degree joins
    let mut co = ClipperOffset::new_default();
    let line = vec![Point64::new(0, 0), Point64::new(100, 0)];
    co.add_path(&line, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    // Should produce a shape around the line
    assert!(!result.is_empty());
}

#[test]
fn test_offset_two_point_joined() {
    // Two-point path with Joined end type
    let mut co = ClipperOffset::new_default();
    let line = vec![Point64::new(0, 0), Point64::new(100, 0)];
    co.add_path(&line, JoinType::Miter, EndType::Joined);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
}

// ---------------------------------------------------------------------------
// Arc tolerance tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_arc_tolerance_small() {
    // Small arc tolerance should produce more vertices (smoother curves)
    let mut co_small = ClipperOffset::new(2.0, 0.5, false, false);
    let mut co_large = ClipperOffset::new(2.0, 5.0, false, false);

    let square = make_square(100);
    co_small.add_path(&square, JoinType::Round, EndType::Polygon);
    co_large.add_path(&square, JoinType::Round, EndType::Polygon);

    let mut result_small = Paths64::new();
    let mut result_large = Paths64::new();
    co_small.execute(20.0, &mut result_small);
    co_large.execute(20.0, &mut result_large);

    assert!(!result_small.is_empty());
    assert!(!result_large.is_empty());

    // Smaller arc tolerance -> more vertices (smoother curves)
    let verts_small: usize = result_small.iter().map(|p| p.len()).sum();
    let verts_large: usize = result_large.iter().map(|p| p.len()).sum();
    assert!(
        verts_small >= verts_large,
        "Small arc tolerance should produce more vertices: {} vs {}",
        verts_small,
        verts_large
    );
}

// ---------------------------------------------------------------------------
// Reverse solution tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_reverse_solution() {
    let mut co_normal = ClipperOffset::new_default();
    let mut co_reversed = ClipperOffset::new(2.0, 0.0, false, true);

    let square = make_square(100);
    co_normal.add_path(&square, JoinType::Miter, EndType::Polygon);
    co_reversed.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result_normal = Paths64::new();
    let mut result_reversed = Paths64::new();
    co_normal.execute(10.0, &mut result_normal);
    co_reversed.execute(10.0, &mut result_reversed);

    assert!(!result_normal.is_empty());
    assert!(!result_reversed.is_empty());

    // Reversed solution should have opposite orientation
    let area_normal = area(&result_normal[0]);
    let area_reversed = area(&result_reversed[0]);
    assert!(
        area_normal * area_reversed < 0.0,
        "Reversed solution should have opposite orientation: normal={}, reversed={}",
        area_normal,
        area_reversed
    );
}

// ---------------------------------------------------------------------------
// Delta callback (variable offset) tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_with_delta_callback() {
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    // Constant callback - should behave the same as fixed delta
    type DeltaCallback = Box<dyn Fn(&Path64, &crate::core::PathD, usize, usize) -> f64>;
    let cb: DeltaCallback = Box::new(|_path, _norms, _j, _k| 10.0);

    let mut result = Paths64::new();
    co.execute_with_callback(cb, &mut result);

    assert!(!result.is_empty());
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    let original_area = area(&square).abs();
    assert!(result_area > original_area);
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_empty_path() {
    let mut co = ClipperOffset::new_default();
    let empty: Path64 = vec![];
    co.add_path(&empty, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    // Empty path after stripping duplicates should produce nothing meaningful
    assert!(result.is_empty());
}

#[test]
fn test_offset_no_groups() {
    let mut co = ClipperOffset::new_default();
    let mut result = Paths64::new();
    co.execute(10.0, &mut result);
    assert!(result.is_empty());
}

#[test]
fn test_offset_zero_delta() {
    // Zero delta should return original paths
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(0.0, &mut result);

    assert_eq!(result.len(), 1);
}

// ---------------------------------------------------------------------------
// Large offset tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_large_inflate() {
    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(500.0, &mut result);

    assert!(!result.is_empty());
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    // The result should be much larger
    assert!(result_area > 1_000_000.0);
}

// ---------------------------------------------------------------------------
// Concave polygon tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_concave_polygon() {
    // An L-shaped polygon (concave)
    let mut co = ClipperOffset::new_default();
    let l_shape = vec![
        Point64::new(0, 0),
        Point64::new(200, 0),
        Point64::new(200, 100),
        Point64::new(100, 100),
        Point64::new(100, 200),
        Point64::new(0, 200),
    ];
    co.add_path(&l_shape, JoinType::Miter, EndType::Polygon);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    assert!(!result.is_empty());
    let original_area = area(&l_shape).abs();
    let result_area: f64 = result.iter().map(|p| area(p).abs()).sum();
    assert!(result_area > original_area);
}

// ---------------------------------------------------------------------------
// PolyTree output test
// ---------------------------------------------------------------------------

#[test]
fn test_offset_execute_tree() {
    use crate::engine_public::PolyTree64;

    let mut co = ClipperOffset::new_default();
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    let mut tree = PolyTree64::new();
    co.execute_tree(10.0, &mut tree);

    // The tree should contain the result
    assert!(
        !tree.nodes.is_empty(),
        "PolyTree should have at least one node"
    );
}

// ---------------------------------------------------------------------------
// Preserve collinear test
// ---------------------------------------------------------------------------

#[test]
fn test_offset_preserve_collinear() {
    let mut co_preserve = ClipperOffset::new(2.0, 0.0, true, false);
    let mut co_no_preserve = ClipperOffset::new(2.0, 0.0, false, false);

    // Square with an extra collinear point on one edge
    let square_with_collinear = vec![
        Point64::new(0, 0),
        Point64::new(50, 0), // collinear with 0,0 -> 100,0
        Point64::new(100, 0),
        Point64::new(100, 100),
        Point64::new(0, 100),
    ];

    co_preserve.add_path(&square_with_collinear, JoinType::Miter, EndType::Polygon);
    co_no_preserve.add_path(&square_with_collinear, JoinType::Miter, EndType::Polygon);

    let mut result_preserve = Paths64::new();
    let mut result_no_preserve = Paths64::new();
    co_preserve.execute(10.0, &mut result_preserve);
    co_no_preserve.execute(10.0, &mut result_no_preserve);

    assert!(!result_preserve.is_empty());
    assert!(!result_no_preserve.is_empty());
}

// ---------------------------------------------------------------------------
// Mixed join/end type tests
// ---------------------------------------------------------------------------

#[test]
fn test_offset_mixed_groups() {
    let mut co = ClipperOffset::new_default();

    // Add a closed polygon
    let square = make_square(100);
    co.add_path(&square, JoinType::Miter, EndType::Polygon);

    // Add an open path
    let line = make_open_line(200);
    co.add_path(&line, JoinType::Round, EndType::Round);

    let mut result = Paths64::new();
    co.execute(10.0, &mut result);

    // Should produce non-empty result (the union step may merge paths)
    assert!(
        !result.is_empty(),
        "Mixed groups should produce output paths"
    );
}

// ---------------------------------------------------------------------------
// OrcaSlicer OffsetConcaveCrossing tests (t172)
// ---------------------------------------------------------------------------

#[test]
fn test_offset_concave_crossing_eliminates_three_point_loops_and_rounds() {
    // Dense 32-gon (radius 100_000, edge length ~19_603) eroded inward by 2_000.
    // Every vertex is concave under inward offset (`sin_a * delta < 0`) and
    // satisfies `4 * x^2 <= edge_len^2`, so canonical `OffsetConcaveCrossing`
    // emits exactly 1 rounded crossing point per vertex instead of a 3-point
    // concave loop (96 raw points -> 32 raw points).
    let n = 32usize;
    let radius = 100_000.0_f64;
    let poly: Path64 = (0..n)
        .map(|i| {
            let theta = 2.0 * constants::PI * (i as f64) / (n as f64);
            Point64::new((radius * theta.cos()).round() as i64, (radius * theta.sin()).round() as i64)
        })
        .collect();

    let delta = -2_000.0_f64;
    let mut co = ClipperOffset::new(3.0, 0.0, false, false);
    co.add_path(&poly, JoinType::Miter, EndType::Polygon);
    co.temp_lim = 2.0 / (co.miter_limit * co.miter_limit);
    co.delta = delta;
    co.do_group_offset(0);

    assert_eq!(co.solution.len(), 1);
    assert_eq!(
        co.solution[0].len(),
        n,
        "OffsetConcaveCrossing must emit 1 crossing point per concave vertex (got {}, unpatched emits {})",
        co.solution[0].len(),
        3 * n
    );

    // Verify exact canonical rounding at every vertex (not truncation).
    let raw = &co.solution[0];
    let path_in = &co.groups[0].paths_in[0];
    let mut k = path_in.len() - 1;
    for j in 0..path_in.len() {
        let cos_a = crate::core::dot_product_two_vectors(co.norms[j], co.norms[k]);
        let q = co.group_delta / (1.0 + cos_a);
        let expected = Point64::new(
            (path_in[j].x as f64 + (co.norms[k].x + co.norms[j].x) * q).round() as i64,
            (path_in[j].y as f64 + (co.norms[k].y + co.norms[j].y) * q).round() as i64,
        );
        assert_eq!(
            raw[j], expected,
            "vertex {j} must match canonical rounded crossing"
        );
        k = j;
    }

    // Public API check: execute() preserves the exact 32-vertex eroded polygon.
    let mut pub_co = ClipperOffset::new(3.0, 0.0, false, false);
    pub_co.add_path(&poly, JoinType::Miter, EndType::Polygon);
    let mut out = Paths64::new();
    pub_co.execute(delta, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].len(), n);
}

#[test]
fn test_offset_concave_crossing_respects_short_edge_and_callback_guards() {
    // 1. Short-edge guard: 4 * x^2 > dist_sqr must fall back to the 3-point join.
    // Right-angle square of side 1_000 eroded by 600 => x = 600, 4*x^2 = 1_440_000 > 1_000_000.
    let small_square = vec![
        Point64::new(0, 0),
        Point64::new(1_000, 0),
        Point64::new(1_000, 1_000),
        Point64::new(0, 1_000),
    ];
    let mut co_short = ClipperOffset::new(3.0, 0.0, false, false);
    co_short.add_path(&small_square, JoinType::Miter, EndType::Polygon);
    co_short.temp_lim = 2.0 / (co_short.miter_limit * co_short.miter_limit);
    co_short.delta = -600.0;
    co_short.do_group_offset(0);
    assert_eq!(
        co_short.solution[0].len(),
        12,
        "short edges (4x^2 > dist^2) must fall back to 3-point concave joins"
    );

    // 2. Delta-callback guard: variable-delta offset must bypass OffsetConcaveCrossing.
    let big_square = vec![
        Point64::new(0, 0),
        Point64::new(100_000, 0),
        Point64::new(100_000, 100_000),
        Point64::new(0, 100_000),
    ];
    let mut co_cb = ClipperOffset::new(3.0, 0.0, false, false);
    co_cb.add_path(&big_square, JoinType::Miter, EndType::Polygon);
    co_cb.delta_callback = Some(Box::new(|_, _, _, _| -1_000.0));
    co_cb.temp_lim = 2.0 / (co_cb.miter_limit * co_cb.miter_limit);
    co_cb.delta = 1.0;
    co_cb.do_group_offset(0);
    assert_eq!(
        co_cb.solution[0].len(),
        12,
        "delta_callback must bypass OffsetConcaveCrossing"
    );
}
