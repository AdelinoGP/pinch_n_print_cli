#![allow(missing_docs)]

//! TDD-RED test file for packet 51 (`paint-semantic-region-overrides`).
//! Tests assert the behaviour of the not-yet-implemented
//! `resolve_per_paint_semantic_configs` function (Step 3 will add it).
//! Both tests are RED: they reach a `panic!("RED: â€¦")` placeholder at runtime.

use std::collections::HashMap;

use slicer_config::{ConfigScope, ExpansionContext, ResolutionTarget};
use slicer_ir::{ConfigValue, PaintSemantic};
use slicer_scheduler::{
    config_resolution::{
        ingest_resolution_config, resolve_config, unknown_paint_semantic_warnings,
    },
    ConfigBoundsIndex,
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

    let bounds = ConfigBoundsIndex::empty();
    let resolved = resolve_config(
        &source,
        &bounds,
        &ResolutionTarget {
            paint_semantics: vec!["fuzzy_skin".to_owned()],
            ..ResolutionTarget::default()
        },
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("resolution should not fail");
    assert_eq!(resolved.wall_count, 5);
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

    // Known semantics list deliberately does NOT include UNKNOWN_SEMANTIC.
    let semantics: [PaintSemantic; 0] = [];

    let bounds = ConfigBoundsIndex::empty();
    let scoped = ingest_resolution_config(&source, &bounds).expect("typed ingestion should pass");
    let warnings = unknown_paint_semantic_warnings(&scoped, &semantics);
    assert!(scoped
        .delta(&ConfigScope::PaintSemantic("UNKNOWN_SEMANTIC".to_owned()))
        .is_some());
    let resolved = resolve_config(
        &source,
        &bounds,
        &ResolutionTarget::default(),
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("unselected paint semantic must not apply");
    assert_eq!(resolved.wall_count, 2);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("UNKNOWN_SEMANTIC"));
}
