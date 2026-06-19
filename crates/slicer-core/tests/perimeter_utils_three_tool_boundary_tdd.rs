//! TDD tests for `find_all_transitions` with 3-tool polygon (packet 102 step 2).
//!
//! Verifies that a polygon painted with 3 different tools produces the correct
//! set of `MaterialBoundarySegment` entries.

use slicer_core::perimeter_utils::find_all_transitions;
use slicer_ir::PaintValue;

#[test]
fn three_tool_polygon_produces_three_transitions() {
    // Polygon: 6 points with tools [1, 1, 2, 2, 3, 3]
    // Transitions at: 1→2 (index 1), 3→4 (index 3), 5→0 (index 5 wrap)
    let mat_vals: Vec<Option<PaintValue>> = vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(3)),
        Some(PaintValue::ToolIndex(3)),
    ];

    let segments = find_all_transitions(&mat_vals);

    assert_eq!(
        segments.len(),
        3,
        "3-tool polygon should produce 3 transitions, got {}: {:?}",
        segments.len(),
        segments
    );

    // Transition at index 1: tool 1 → tool 2
    assert_eq!(segments[0].point_range, 1..2);
    assert_eq!(segments[0].near_tool, Some(1));
    assert_eq!(segments[0].far_tool, Some(2));

    // Transition at index 3: tool 2 → tool 3
    assert_eq!(segments[1].point_range, 3..4);
    assert_eq!(segments[1].near_tool, Some(2));
    assert_eq!(segments[1].far_tool, Some(3));

    // Transition at index 5 (wrap): tool 3 → tool 1
    assert_eq!(segments[2].point_range, 5..6);
    assert_eq!(segments[2].near_tool, Some(3));
    assert_eq!(segments[2].far_tool, Some(1));
}

#[test]
fn single_tool_polygon_produces_no_transitions() {
    let mat_vals: Vec<Option<PaintValue>> = vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
    ];

    let segments = find_all_transitions(&mat_vals);
    assert!(segments.is_empty());
}

#[test]
fn two_tool_polygon_produces_two_transitions() {
    // [1, 1, 2, 2] → transitions at 1→2 (index 1) and 3→0 wrap (index 3)
    let mat_vals: Vec<Option<PaintValue>> = vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
    ];

    let segments = find_all_transitions(&mat_vals);

    assert_eq!(segments.len(), 2);

    assert_eq!(segments[0].point_range, 1..2);
    assert_eq!(segments[0].near_tool, Some(1));
    assert_eq!(segments[0].far_tool, Some(2));

    assert_eq!(segments[1].point_range, 3..4);
    assert_eq!(segments[1].near_tool, Some(2));
    assert_eq!(segments[1].far_tool, Some(1));
}

#[test]
fn none_values_are_skipped() {
    // [None, 1, 1, None] → transitions at None→1 (index 0) and 1→None (index 2)
    let mat_vals: Vec<Option<PaintValue>> = vec![
        None,
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
        None,
    ];

    let segments = find_all_transitions(&mat_vals);
    assert_eq!(segments.len(), 2);

    assert_eq!(segments[0].point_range, 0..1);
    assert_eq!(segments[0].near_tool, None);
    assert_eq!(segments[0].far_tool, Some(1));

    assert_eq!(segments[1].point_range, 2..3);
    assert_eq!(segments[1].near_tool, Some(1));
    assert_eq!(segments[1].far_tool, None);
}

#[test]
fn contiguous_run_of_same_transition_merged() {
    // [1, 2, 2, 2, 1, 1] → transitions at 0→1 and 3→4
    let mat_vals: Vec<Option<PaintValue>> = vec![
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(2)),
        Some(PaintValue::ToolIndex(1)),
        Some(PaintValue::ToolIndex(1)),
    ];

    let segments = find_all_transitions(&mat_vals);

    assert_eq!(segments.len(), 2);

    assert_eq!(segments[0].point_range, 0..1);
    assert_eq!(segments[0].near_tool, Some(1));
    assert_eq!(segments[0].far_tool, Some(2));

    assert_eq!(segments[1].point_range, 3..4);
    assert_eq!(segments[1].near_tool, Some(2));
    assert_eq!(segments[1].far_tool, Some(1));
}
