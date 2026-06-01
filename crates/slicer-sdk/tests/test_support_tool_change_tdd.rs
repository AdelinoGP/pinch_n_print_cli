//! TDD tests for the `tool_change` freestanding fixture helper.

use slicer_sdk::test_prelude::*;

#[test]
fn tool_change_threads_after_entity_index_and_tool_index() {
    let tc = tool_change(5, 0, 3);
    assert_eq!(tc.after_entity_index, 5);
    assert_eq!(tc.to_tool, 3);
}

#[test]
fn tool_change_threads_from_tool_explicitly() {
    let tc = tool_change(0, 0, 1);
    assert_eq!(tc.from_tool, 0);
}

#[test]
fn tool_change_round_trip_varies_anchor() {
    let tc = tool_change(42, 0, 7);
    assert_eq!(tc.after_entity_index, 42);
    assert_eq!(tc.to_tool, 7);
    assert_eq!(tc.from_tool, 0);
}

#[test]
fn tool_change_round_trips_nonzero_from_tool() {
    let tc = tool_change(11, 3, 7);
    assert_eq!(tc.after_entity_index, 11);
    assert_eq!(tc.from_tool, 3);
    assert_eq!(tc.to_tool, 7);
}
