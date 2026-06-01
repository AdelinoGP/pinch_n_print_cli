//! TDD tests for the 7 new `SliceRegionViewBuilder` setter methods that
//! collapse rectilinear-infill's `make_test_region` / `make_bridge_region`
//! post-build `set_*` chains into single-expression builder chains.
//!
//! Invariant G: the default-built `SliceRegionView` (no setter calls) must
//! match its prior baseline — locked by `default_builder_matches_baseline`
//! BEFORE exercising any new setter.

use slicer_sdk::test_support::fixtures::{rect_polygon, SliceRegionViewBuilder};

#[test]
fn default_builder_matches_baseline() {
    // Invariant G: shapes added by these 7 setters MUST default to today's
    // SliceRegionView default values when the corresponding setter is not
    // called. If this assertion ever changes, packet 78 callers using
    // `SliceRegionViewBuilder::new().build()` may silently shift behavior.
    let view = SliceRegionViewBuilder::new().build();
    assert_eq!(view.top_shell_index(), None);
    assert!(view.top_solid_fill().is_empty());
    assert_eq!(view.bottom_shell_index(), None);
    assert!(view.bottom_solid_fill().is_empty());
    assert!(!view.is_bridge());
    assert!(view.bridge_areas().is_empty());
    assert!((view.bridge_orientation_deg() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn top_shell_index_round_trip() {
    let view = SliceRegionViewBuilder::new()
        .top_shell_index(Some(2))
        .build();
    assert_eq!(view.top_shell_index(), Some(2));
}

#[test]
fn top_solid_fill_round_trip() {
    let fills = vec![rect_polygon(0.0, 0.0, 4.0, 6.0)];
    let view = SliceRegionViewBuilder::new()
        .top_solid_fill(fills.clone())
        .build();
    assert_eq!(view.top_solid_fill().len(), fills.len());
}

#[test]
fn bottom_shell_index_round_trip() {
    let view = SliceRegionViewBuilder::new()
        .bottom_shell_index(Some(3))
        .build();
    assert_eq!(view.bottom_shell_index(), Some(3));
}

#[test]
fn bottom_solid_fill_round_trip() {
    let fills = vec![
        rect_polygon(0.0, 0.0, 4.0, 6.0),
        rect_polygon(5.0, 5.0, 2.0, 3.0),
    ];
    let view = SliceRegionViewBuilder::new()
        .bottom_solid_fill(fills.clone())
        .build();
    assert_eq!(view.bottom_solid_fill().len(), fills.len());
}

#[test]
fn is_bridge_round_trip() {
    let view = SliceRegionViewBuilder::new().is_bridge(true).build();
    assert!(view.is_bridge());
}

#[test]
fn bridge_areas_round_trip() {
    let areas = vec![rect_polygon(0.0, 0.0, 10.0, 2.0)];
    let view = SliceRegionViewBuilder::new()
        .bridge_areas(areas.clone())
        .build();
    assert_eq!(view.bridge_areas().len(), areas.len());
}

#[test]
fn bridge_orientation_deg_round_trip() {
    let view = SliceRegionViewBuilder::new()
        .bridge_orientation_deg(45.0)
        .build();
    assert!((view.bridge_orientation_deg() - 45.0).abs() < f32::EPSILON);
}

#[test]
fn is_bridge_setter_is_idempotent() {
    // Idempotency on a representative setter: calling twice with the same
    // value leaves the same state as calling once.
    let view_once = SliceRegionViewBuilder::new().is_bridge(true).build();
    let view_twice = SliceRegionViewBuilder::new()
        .is_bridge(true)
        .is_bridge(true)
        .build();
    assert_eq!(view_once.is_bridge(), view_twice.is_bridge());
    assert!(view_twice.is_bridge());
}

#[test]
fn bridge_orientation_deg_last_write_wins() {
    // Last-write-wins on a representative setter: v1 then v2 → final state v2.
    let view = SliceRegionViewBuilder::new()
        .bridge_orientation_deg(15.0)
        .bridge_orientation_deg(90.0)
        .build();
    assert!((view.bridge_orientation_deg() - 90.0).abs() < f32::EPSILON);
}

#[test]
fn unset_setters_leave_other_fields_at_default() {
    // Setting only one new field MUST NOT perturb any of the other six.
    let view = SliceRegionViewBuilder::new().is_bridge(true).build();
    assert!(view.is_bridge());
    // The other six remain at default.
    assert_eq!(view.top_shell_index(), None);
    assert!(view.top_solid_fill().is_empty());
    assert_eq!(view.bottom_shell_index(), None);
    assert!(view.bottom_solid_fill().is_empty());
    assert!(view.bridge_areas().is_empty());
    assert!((view.bridge_orientation_deg() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn full_setter_chain_threads_all_seven_fields() {
    let top_fill = vec![rect_polygon(0.0, 0.0, 4.0, 6.0)];
    let bot_fill = vec![rect_polygon(1.0, 1.0, 2.0, 2.0)];
    let bridge = vec![rect_polygon(0.0, 0.0, 10.0, 2.0)];
    let view = SliceRegionViewBuilder::new()
        .top_shell_index(Some(0))
        .top_solid_fill(top_fill.clone())
        .bottom_shell_index(Some(1))
        .bottom_solid_fill(bot_fill.clone())
        .is_bridge(true)
        .bridge_areas(bridge.clone())
        .bridge_orientation_deg(30.0)
        .build();
    assert_eq!(view.top_shell_index(), Some(0));
    assert_eq!(view.top_solid_fill().len(), top_fill.len());
    assert_eq!(view.bottom_shell_index(), Some(1));
    assert_eq!(view.bottom_solid_fill().len(), bot_fill.len());
    assert!(view.is_bridge());
    assert_eq!(view.bridge_areas().len(), bridge.len());
    assert!((view.bridge_orientation_deg() - 30.0).abs() < f32::EPSILON);
}
