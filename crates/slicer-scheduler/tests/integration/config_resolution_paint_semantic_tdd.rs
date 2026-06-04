#![allow(missing_docs)]

//! TDD-RED test file for packet 51 (`paint-semantic-region-overrides`).
//! Tests assert the behaviour of the not-yet-implemented
//! `resolve_per_paint_semantic_configs` function (Step 3 will add it).
//! Both tests are RED: they reach a `panic!("RED: â€¦")` placeholder at runtime.

use std::collections::HashMap;

use slicer_ir::{ConfigValue, PaintSemantic, ResolvedConfig};
use slicer_scheduler::{
    resolve_per_paint_semantic_configs, ConfigBoundsIndex, ConfigResolutionError,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn config_value_int(v: i64) -> ConfigValue {
    ConfigValue::Int(v)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// AC-1 (packet 51): `paint_config:<semantic>:<key>` namespace entries are
/// resolved into a per-semantic `ResolvedConfig` that overrides the global
/// default.
#[test]
fn resolves_paint_config_namespace() {
    // Build a source map with a global perimeter_count=2 and an override for
    // the "fuzzy_skin" paint semantic.
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert("wall_count".to_string(), config_value_int(2));
    source.insert(
        "paint_config:fuzzy_skin:wall_count".to_string(),
        config_value_int(5),
    );

    let global = ResolvedConfig {
        wall_count: 2,
        ..ResolvedConfig::default()
    };

    let semantics = [PaintSemantic::Custom("fuzzy_skin".to_string())];

    let bounds = ConfigBoundsIndex::empty();
    let (result, _warnings) =
        resolve_per_paint_semantic_configs(&global, &source, &semantics, &bounds)
            .expect("resolution should not fail");
    assert!(result.contains_key(&PaintSemantic::Custom("fuzzy_skin".to_string())));
    assert_eq!(
        result[&PaintSemantic::Custom("fuzzy_skin".to_string())].wall_count,
        5
    );
    let _: Result<(), ConfigResolutionError> = Ok(()); // keep import used
}

/// AC-2 (packet 51): A `paint_config` entry whose semantic does not appear in
/// the known-semantics slice is silently dropped, and a warning is emitted
/// naming the unknown semantic.
#[test]
fn unknown_semantic_warns_then_ignores() {
    let mut source: HashMap<String, ConfigValue> = HashMap::new();
    source.insert(
        "paint_config:UNKNOWN_SEMANTIC:wall_count".to_string(),
        config_value_int(5),
    );

    let global = ResolvedConfig::default();

    // Known semantics list deliberately does NOT include UNKNOWN_SEMANTIC.
    let semantics: [PaintSemantic; 0] = [];

    let bounds = ConfigBoundsIndex::empty();
    let (result, warnings) =
        resolve_per_paint_semantic_configs(&global, &source, &semantics, &bounds)
            .expect("resolution should not fail");
    assert!(!result.contains_key(&PaintSemantic::Custom("UNKNOWN_SEMANTIC".to_string())));
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("UNKNOWN_SEMANTIC"));
}
