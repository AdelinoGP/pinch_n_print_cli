//! Contract tests for canonical tree support style behavior.

use std::collections::HashMap;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use tree_support_planner::{
    hybrid_contact_is_polygon, resolve_tree_style, style_movement, style_neighbour_direction,
};

fn config(style: &str) -> ConfigView {
    let mut values = HashMap::<ConfigKey, ConfigValue>::new();
    values.insert("support_style".into(), ConfigValue::String(style.into()));
    ConfigView::from_map(values)
}

fn close(a: (f32, f32), b: (f32, f32)) {
    assert!(
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6,
        "{a:?} != {b:?}"
    );
}

#[test]
fn tree_strong_uses_unweighted_neighbour_sum() {
    let neighbours = [(1.0, 0.0), (-4.0, 0.0)];
    let strong = style_neighbour_direction("tree_strong", (0.0, 0.0), &neighbours);
    let slim = style_neighbour_direction("tree_slim", (0.0, 0.0), &neighbours);

    close(strong, (-3.0, 0.0));
    assert!(
        slim.0 > 0.0,
        "weighted slim direction must favour the near neighbour: {slim:?}"
    );
}

#[test]
fn tree_strong_sums_outer_and_neighbour_only_when_dot_gate_passes() {
    close(
        style_movement("tree_strong", (1.0, 0.0), (0.0, 1.0), 2.0),
        (2.0_f32.sqrt(), 2.0_f32.sqrt()),
    );
    close(
        style_movement("tree_strong", (1.0, 0.0), (-1.0, 0.0), 2.0),
        (2.0, 0.0),
    );
    close(
        style_movement("tree_slim", (1.0, 0.0), (0.0, 1.0), 2.0),
        (2.0, 0.0),
    );
}

#[test]
fn tree_hybrid_mints_polygon_only_for_large_flat_overhangs() {
    assert!(hybrid_contact_is_polygon("tree_hybrid", 1.01));
    assert!(!hybrid_contact_is_polygon("tree_hybrid", 1.0));
    assert!(!hybrid_contact_is_polygon("tree_slim", 2.0));
}

#[test]
fn slim_and_non_tree_styles_resolve_without_new_tree_side_effects() {
    assert_eq!(resolve_tree_style(&config("tree_slim")), "tree_slim");
    for style in ["default", "grid", "snug", "organic"] {
        assert_eq!(
            resolve_tree_style(&config(style)),
            "default",
            "style={style}"
        );
        close(
            style_neighbour_direction(style, (0.0, 0.0), &[(1.0, 0.0), (-4.0, 0.0)]),
            (0.75, 0.0),
        );
        assert!(!hybrid_contact_is_polygon(style, 2.0));
    }
}
