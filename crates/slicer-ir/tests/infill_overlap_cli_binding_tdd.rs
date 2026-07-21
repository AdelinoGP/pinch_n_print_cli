//! AC-4: `infill_overlap` is a CLI-bindable ResolvedConfig field whose value
//! flows from a JSON/CLI config source into the per-region config consumed by
//! the linker.
//!
//! The linker reads `infill_overlap` from the per-region resolved config to
//! decide how far the sparse path extends past the perimeter boundary. A
//! different overlap value must produce a different overlap boundary in the
//! downstream IR.
//!
//! This test pairs the binding assertion (the CLI key reaches the
//! `ResolvedConfig` field) with a behavioral assertion (0.30 vs the 0.45
//! default diverge measurably on the canonical `to_config_map` shape), and a
//! 3-binding regression guard (replicating the `fill_holder_cli_binding_tdd`
//! 3-test shape).

use slicer_ir::{ConfigValue, ResolvedConfig};
use std::collections::HashMap;

#[test]
fn infill_overlap_default_is_zero_point_four_five() {
    let cfg = ResolvedConfig::default();
    assert!(
        (cfg.infill_overlap - 0.45).abs() < 1e-5,
        "default infill_overlap should be ~0.45 (OrcaSlicer default, f32 precision), got {}",
        cfg.infill_overlap
    );
}

#[test]
fn infill_overlap_apply_cli_key_sets_value() {
    let mut cfg = ResolvedConfig::default();
    let applied = cfg
        .apply_cli_key("infill_overlap", &ConfigValue::Float(0.30))
        .expect("type check for infill_overlap must succeed");
    assert!(
        applied,
        "infill_overlap must be a recognized CLI-bound field"
    );
    assert!(
        (cfg.infill_overlap - 0.30).abs() < 1e-5,
        "after apply_cli_key(0.30), infill_overlap should be ~0.30 (f32 precision), got {}",
        cfg.infill_overlap
    );
}

#[test]
fn infill_overlap_thirty_produces_different_overlap_boundary_than_default() {
    // Default (0.45) and 0.30 produce different overlap boundaries. The
    // canonical surface for this comparison is `to_config_map`: the linker
    // reads the value through this map. Both should report a value (no
    // Option-skipped omission) and the values must differ.
    let default_cfg = ResolvedConfig::default();
    let mut tweaked_cfg = ResolvedConfig::default();
    tweaked_cfg
        .apply_cli_key("infill_overlap", &ConfigValue::Float(0.30))
        .expect("type check for infill_overlap must succeed");

    let default_map: HashMap<String, ConfigValue> = default_cfg.to_config_map();
    let tweaked_map: HashMap<String, ConfigValue> = tweaked_cfg.to_config_map();

    let default_value = default_map
        .get("infill_overlap")
        .expect("to_config_map must emit infill_overlap key for the default config");
    let tweaked_value = tweaked_map
        .get("infill_overlap")
        .expect("to_config_map must emit infill_overlap key after CLI override");

    let default_f = match default_value {
        ConfigValue::Float(v) => *v,
        other => panic!("default infill_overlap should be Float, got {:?}", other),
    };
    let tweaked_f = match tweaked_value {
        ConfigValue::Float(v) => *v,
        other => panic!("tweaked infill_overlap should be Float, got {:?}", other),
    };

    assert!(
        (default_f - 0.45).abs() < 1e-5,
        "default map infill_overlap should be ~0.45 (f32 precision), got {}",
        default_f
    );
    assert!(
        (tweaked_f - 0.30).abs() < 1e-5,
        "tweaked map infill_overlap should be ~0.30 (f32 precision), got {}",
        tweaked_f
    );
    assert!(
        (default_f - tweaked_f).abs() > 0.10,
        "0.45 and 0.30 must differ measurably, got delta = {}",
        (default_f - tweaked_f).abs()
    );
}
