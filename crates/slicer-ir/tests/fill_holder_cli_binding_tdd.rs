//! Regression: fill-role holder keys must be CLI-bound so user configs
//! (JSON, CLI overrides) actually reach `ResolvedConfig`.
//!
//! Pre-fix: `top_fill_holder`, `bottom_fill_holder`, `bridge_fill_holder`,
//! and `sparse_fill_holder` were declared `plain` in `ResolvedConfig`, so
//! `resolve_global_config` ignored them and they always stayed at the
//! default `rectilinear-infill`. User multi-infill configs were silently
//! ignored, and painted regions fell back to the default holder.

use slicer_ir::{ConfigValue, ResolvedConfig};
use std::collections::HashMap;

#[test]
fn fill_holders_apply_from_cli_string_values() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "top_fill_holder".into(),
        ConfigValue::String("rectilinear-infill".into()),
    );
    source.insert(
        "bottom_fill_holder".into(),
        ConfigValue::String("rectilinear-infill".into()),
    );
    source.insert(
        "bridge_fill_holder".into(),
        ConfigValue::String("rectilinear-infill".into()),
    );
    source.insert(
        "sparse_fill_holder".into(),
        ConfigValue::String("gyroid-infill".into()),
    );

    let mut cfg = ResolvedConfig::default();
    for (key, value) in &source {
        let applied = cfg.apply_cli_key(key, value).expect("type check");
        assert!(applied, "{key} must be a recognized CLI-bound field");
    }

    assert_eq!(cfg.top_fill_holder, "rectilinear-infill");
    assert_eq!(cfg.bottom_fill_holder, "rectilinear-infill");
    assert_eq!(cfg.bridge_fill_holder, "rectilinear-infill");
    assert_eq!(cfg.sparse_fill_holder, "gyroid-infill");
}

#[test]
fn fill_holders_default_to_rectilinear() {
    let cfg = ResolvedConfig::default();
    assert_eq!(cfg.top_fill_holder, "rectilinear-infill");
    assert_eq!(cfg.bottom_fill_holder, "rectilinear-infill");
    assert_eq!(cfg.bridge_fill_holder, "rectilinear-infill");
    assert_eq!(cfg.sparse_fill_holder, "rectilinear-infill");
}

#[test]
fn fill_holder_short_name_is_accepted() {
    // The matching logic (module_id_matches_holder) supports both full and
    // short module IDs for com.core modules. Config storage is just a string,
    // so short names are valid values.
    let mut cfg = ResolvedConfig::default();
    cfg.apply_cli_key(
        "sparse_fill_holder",
        &ConfigValue::String("lightning-infill".into()),
    )
    .unwrap();
    assert_eq!(cfg.sparse_fill_holder, "lightning-infill");
}
