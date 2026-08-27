//! Contract tests for canonical tree support style behavior.

use std::collections::HashMap;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use tree_support_planner::{
    hybrid_contact_is_polygon, organic_substitution_requested, resolve_tree_style, style_movement,
    style_neighbour_direction,
};

fn config(style: &str) -> ConfigView {
    let mut values = HashMap::<ConfigKey, ConfigValue>::new();
    values.insert("support_style".into(), ConfigValue::String(style.into()));
    ConfigView::from_map(values)
}

fn config_with_type(style: &str, support_type: &str) -> ConfigView {
    let mut values = HashMap::<ConfigKey, ConfigValue>::new();
    values.insert("support_style".into(), ConfigValue::String(style.into()));
    values.insert(
        "support_type".into(),
        ConfigValue::String(support_type.into()),
    );
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

/// Canonical `drop_nodes`' `is_strong` movement-composition block is DEAD
/// code: its result is unconditionally overwritten by the final chain
/// `movement = normal(direction_to_outer)` when `dist2_to_outer > 0`, else
/// `normal(move_to_neighbor_center)` (it even reads an uninitialized
/// `movement` in its dot gate). Every style therefore takes the same final
/// movement; Strong differs only in the neighbour-sum weighting.
#[test]
fn movement_is_outer_normal_for_every_style_when_outer_is_nonzero() {
    for style in ["tree_strong", "tree_slim", "default", "tree_hybrid"] {
        close(style_movement(style, (1.0, 0.0), (0.0, 1.0), 2.0), (2.0, 0.0));
        close(
            style_movement(style, (1.0, 0.0), (-1.0, 0.0), 2.0),
            (2.0, 0.0),
        );
        close(
            style_movement(style, (0.0, 0.0), (0.0, 1.0), 2.0),
            (0.0, 2.0),
        );
    }
}

#[test]
fn tree_hybrid_mints_polygon_only_for_large_flat_overhangs() {
    assert!(hybrid_contact_is_polygon("tree_hybrid", 1.01));
    assert!(!hybrid_contact_is_polygon("tree_hybrid", 1.0));
    assert!(!hybrid_contact_is_polygon("tree_slim", 2.0));
}

/// Canonical `SupportParameters.hpp` substitution chain, with this port's
/// organic-engine alias (the organic engine is not implemented; canonically-
/// organic inputs run the Strong style of the old engine):
/// grid/snug + tree type -> smsDefault; smsDefault + tree type -> organic
/// -> Strong here. Explicit tree styles keep themselves on a tree family.
#[test]
fn canonically_organic_styles_alias_to_strong_on_tree_family() {
    assert_eq!(resolve_tree_style(&config("tree_slim")), "tree_slim");
    assert_eq!(resolve_tree_style(&config("tree_strong")), "tree_strong");
    assert_eq!(resolve_tree_style(&config("tree_hybrid")), "tree_hybrid");
    for style in ["default", "grid", "snug", "organic"] {
        // No support_type: this planner's family fallback is tree.
        assert_eq!(
            resolve_tree_style(&config(style)),
            "tree_strong",
            "style={style}"
        );
        assert_eq!(
            resolve_tree_style(&config_with_type(style, "tree(auto)")),
            "tree_strong",
            "style={style}"
        );
        // The by-name style helpers are explicit surfaces and stay unaliased.
        close(
            style_neighbour_direction(style, (0.0, 0.0), &[(1.0, 0.0), (-4.0, 0.0)]),
            (0.75, 0.0),
        );
        assert!(!hybrid_contact_is_polygon(style, 2.0));
    }
}

/// Canonical degrades tree styles on a non-tree support type back to
/// smsDefault (`SupportParameters.hpp`); nothing aliases to Strong there.
#[test]
fn non_tree_family_resolves_default_for_every_style() {
    for style in ["default", "grid", "snug", "organic", "tree_strong", "tree_slim"] {
        assert_eq!(
            resolve_tree_style(&config_with_type(style, "normal(auto)")),
            "default",
            "style={style}"
        );
    }
}

/// Only an explicit `organic` request on a tree family earns the once-per-
/// slice Warn diagnostic; the plain default alias is silent (product
/// decision, documented in docs/DEVIATION_LOG.md).
#[test]
fn organic_substitution_warns_only_for_explicit_organic_on_tree_family() {
    assert!(organic_substitution_requested(&config("organic")));
    assert!(organic_substitution_requested(&config_with_type(
        "organic",
        "tree(auto)"
    )));
    assert!(!organic_substitution_requested(&config("default")));
    assert!(!organic_substitution_requested(&config("tree_strong")));
    assert!(!organic_substitution_requested(&config_with_type(
        "organic",
        "normal(auto)"
    )));
}
