//! Independent literal-oracle tests for typed scope-stack resolution.

use std::collections::BTreeMap;

use slicer_config::{
    assemble_registry, query_z_grid, resolve_scope_stack, ConfigSchemaRegistry, ConfigScope,
    ExpansionContext, HostChannels, ModuleDeclaration, ResolutionError, ResolutionTarget,
    ResolvedObjectLayerConfig, ScopeDelta, ScopedConfig,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::{ConfigResolutionError, ConfigValue};

fn registry() -> ConfigSchemaRegistry {
    let schema = ConfigSchema {
        entries: BTreeMap::from([
            (
                "custom_extension".to_owned(),
                ConfigFieldEntry {
                    field_type: "string".to_owned(),
                    ..ConfigFieldEntry::default()
                },
            ),
            (
                "support_raft_layers".to_owned(),
                ConfigFieldEntry {
                    field_type: "int".to_owned(),
                    ..ConfigFieldEntry::default()
                },
            ),
        ]),
    };

    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.scope-resolution".to_owned(),
            schema,
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_live(),
    )
    .expect("scope-resolution fixture registry must assemble")
    .registry
}

fn delta(entries: impl IntoIterator<Item = (&'static str, ConfigValue)>) -> ScopeDelta {
    ScopeDelta {
        values: entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

fn expansion() -> ExpansionContext {
    ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    }
}

#[test]
fn scope_stack_matches_hand_authored_precedence_table() {
    let registry = registry();
    let scoped = ScopedConfig {
        deltas: BTreeMap::from([
            (
                ConfigScope::Global,
                delta([("infill_density", ConfigValue::Float(0.10))]),
            ),
            (
                ConfigScope::Object("obj-a".to_owned()),
                delta([("infill_density", ConfigValue::Float(0.20))]),
            ),
            (
                ConfigScope::Modifier {
                    object_id: "obj-a".to_owned(),
                    modifier_id: "priority-10".to_owned(),
                },
                delta([("infill_density", ConfigValue::Float(0.30))]),
            ),
            (
                ConfigScope::Modifier {
                    object_id: "obj-a".to_owned(),
                    modifier_id: "priority-20".to_owned(),
                },
                delta([("infill_density", ConfigValue::Float(0.40))]),
            ),
            (
                ConfigScope::PaintSemantic("material".to_owned()),
                delta([("infill_density", ConfigValue::Float(0.50))]),
            ),
            (
                ConfigScope::PaintSemantic("support_enforcer".to_owned()),
                delta([("infill_density", ConfigValue::Float(0.60))]),
            ),
            (
                ConfigScope::Tool(1),
                delta([("infill_density", ConfigValue::Float(0.70))]),
            ),
        ]),
    };

    let rows = [
        (
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                modifier_ids: vec!["priority-10".to_owned(), "priority-20".to_owned()],
                paint_semantics: vec!["support_enforcer".to_owned(), "material".to_owned()],
                tool_index: Some(1),
            },
            0.70_f32,
        ),
        (
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                modifier_ids: vec!["priority-10".to_owned(), "priority-20".to_owned()],
                paint_semantics: vec!["support_enforcer".to_owned(), "material".to_owned()],
                tool_index: None,
            },
            0.60_f32,
        ),
        (
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                modifier_ids: vec!["priority-10".to_owned(), "priority-20".to_owned()],
                paint_semantics: vec!["material".to_owned()],
                tool_index: None,
            },
            0.50_f32,
        ),
        (
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                modifier_ids: vec!["priority-10".to_owned(), "priority-20".to_owned()],
                ..ResolutionTarget::default()
            },
            0.40_f32,
        ),
        (
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                modifier_ids: vec!["priority-10".to_owned()],
                ..ResolutionTarget::default()
            },
            0.30_f32,
        ),
        (
            ResolutionTarget {
                object_id: "obj-a".to_owned(),
                ..ResolutionTarget::default()
            },
            0.20_f32,
        ),
        (
            ResolutionTarget {
                object_id: "obj-b".to_owned(),
                ..ResolutionTarget::default()
            },
            0.10_f32,
        ),
    ];

    for (target, expected_density) in rows {
        let resolved = resolve_scope_stack(&registry, &scoped, &target, &expansion())
            .expect("literal precedence row must resolve");
        assert_eq!(resolved.infill_density, expected_density);
        assert_eq!(
            resolved.line_width, 0.45_f64,
            "Phase-B expansion must run after the complete scope merge"
        );
    }
}

#[test]
fn explicit_default_is_a_real_override() {
    let registry = registry();
    let scoped = ScopedConfig {
        deltas: BTreeMap::from([
            (
                ConfigScope::Global,
                delta([
                    ("wall_count", ConfigValue::Int(3)),
                    ("custom_extension", ConfigValue::String("global".to_owned())),
                ]),
            ),
            (
                ConfigScope::Object("obj-a".to_owned()),
                delta([
                    ("wall_count", ConfigValue::Int(2)),
                    ("custom_extension", ConfigValue::String("object".to_owned())),
                ]),
            ),
        ]),
    };

    let resolved = resolve_scope_stack(
        &registry,
        &scoped,
        &ResolutionTarget {
            object_id: "obj-a".to_owned(),
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect("an explicitly authored default is a valid override");

    assert_eq!(resolved.wall_count, 2);
    assert_eq!(
        resolved.extensions.get("custom_extension"),
        Some(&ConfigValue::String("object".to_owned()))
    );

    let paint_scoped = ScopedConfig {
        deltas: BTreeMap::from([
            (
                ConfigScope::Global,
                delta([("custom_extension", ConfigValue::String("global".to_owned()))]),
            ),
            (
                ConfigScope::PaintSemantic("material".to_owned()),
                delta([("custom_extension", ConfigValue::String(String::new()))]),
            ),
        ]),
    };

    let paint_resolved = resolve_scope_stack(
        &registry,
        &paint_scoped,
        &ResolutionTarget {
            object_id: "obj-a".to_owned(),
            paint_semantics: vec!["material".to_owned()],
            ..ResolutionTarget::default()
        },
        &expansion(),
    )
    .expect("a paint-scoped extension set to its default must be retained");

    assert_eq!(
        paint_resolved.extensions.get("custom_extension"),
        Some(&ConfigValue::String(String::new()))
    );
}

#[test]
fn z_grid_query_returns_hand_computed_object_record() {
    let registry = registry();
    let scoped = ScopedConfig {
        deltas: BTreeMap::from([
            (
                ConfigScope::Global,
                delta([
                    ("layer_height", ConfigValue::Float(0.20)),
                    ("first_layer_height", ConfigValue::Float(0.30)),
                ]),
            ),
            (
                ConfigScope::Object("obj-a".to_owned()),
                delta([
                    ("layer_height", ConfigValue::Float(0.25)),
                    ("support_raft_layers", ConfigValue::Int(2)),
                ]),
            ),
        ]),
    };

    let actual = query_z_grid(
        &registry,
        &scoped,
        &BTreeMap::from([("obj-a".to_owned(), 2.0)]),
        &expansion(),
    )
    .expect("valid object planning values must resolve");

    assert_eq!(
        actual,
        vec![
            // exhaustive: every field in the exact public Z-grid record is asserted.
            ResolvedObjectLayerConfig {
                object_id: "obj-a".to_owned(),
                object_height: 2.0,
                layer_height: 0.25,
                first_layer_height: 0.30,
                support_raft_layers: 2,
            }
        ]
    );
}

#[test]
fn invalid_object_height_is_rejected_atomically() {
    let registry = registry();
    let error = query_z_grid(
        &registry,
        &ScopedConfig::default(),
        &BTreeMap::from([("obj-a".to_owned(), 2.0), ("obj-b".to_owned(), -0.5)]),
        &expansion(),
    )
    .expect_err("one invalid height must reject the atomic query");

    assert_eq!(
        error,
        ResolutionError::InvalidObjectHeight {
            object_id: "obj-b".to_owned(),
            value: Some(-0.5),
        }
    );
}

#[test]
fn invalid_support_raft_layer_counts_are_rejected_instead_of_wrapped() {
    let registry = registry();

    for invalid_count in [-1_i64, 4_294_967_296_i64] {
        let scoped = ScopedConfig {
            deltas: BTreeMap::from([(
                ConfigScope::Object("obj-a".to_owned()),
                delta([("support_raft_layers", ConfigValue::Int(invalid_count))]),
            )]),
        };

        let error = query_z_grid(
            &registry,
            &scoped,
            &BTreeMap::from([("obj-a".to_owned(), 2.0)]),
            &expansion(),
        )
        .expect_err("an invalid raft count must not be cast into a u32");

        assert!(matches!(
            error,
            ResolutionError::Application(ConfigResolutionError::OutOfRange {
                key,
                value,
                min: Some(0.0),
                max: Some(4_294_967_295.0),
                index: None,
            }) if key == "support_raft_layers" && value == invalid_count as f64
        ));
    }
}
