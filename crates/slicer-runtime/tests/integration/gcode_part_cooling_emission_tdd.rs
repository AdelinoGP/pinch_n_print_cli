#![allow(missing_docs)]

//! TDD tests for packet 53 (TASK-154): part-cooling fan G-code emission config keys.

use std::collections::HashMap;
use std::sync::Arc;

use part_cooling::PartCooling;
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, IndexedTriangleSet,
    LayerCollectionIR, MeshIR, ObjectConfig, ObjectMesh, Point3, Point3WithWidth, PrintEntity,
    RegionKey, SemVer, Transform3d,
};
use slicer_runtime::{
    load_module_from_paths, Blackboard, DefaultGCodeEmitter, DefaultGCodeSerializer, GCodeEmitter,
    GCodeSerializer,
};
use slicer_sdk::traits::FinalizationModule;

// ============================================================================
// Test fixtures
// ============================================================================

fn semver_fixture() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, // column 0
            0.0, 1.0, 0.0, 0.0, // column 1
            0.0, 0.0, 1.0, 0.0, // column 2
            0.0, 0.0, 0.0, 1.0, // column 3
        ],
    }
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver_fixture(),
        objects: vec![ObjectMesh {
            id: "test-object".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 5.0,
                        y: 10.0,
                        z: 0.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 220.0,
                y: 220.0,
                z: 250.0,
            },
        },
    }
}

fn blackboard_fixture() -> Blackboard {
    let mesh = Arc::new(mesh_fixture());
    Blackboard::new(mesh, 0)
}

fn point3_with_width(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn region_key_fixture(layer_index: u32) -> RegionKey {
    RegionKey {
        global_layer_index: layer_index,
        object_id: "test-object".to_string(),
        region_id: 0,
    }
}

fn print_entity_fixture(points: Vec<Point3WithWidth>, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: region_key_fixture(0),
        topo_order: 0,
    }
}

fn layer_with_entity(index: u32, z: f32, entity: PrintEntity) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver_fixture(),
        global_layer_index: index,
        z,
        ordered_entities: vec![entity],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

fn config_view(entries: &[(&str, ConfigValue)]) -> ConfigView {
    let mut m = HashMap::new();
    for (k, v) in entries {
        m.insert(k.to_string(), v.clone());
    }
    ConfigView::from_map(m)
}

/// Run the cooling module on the given layers and return the serialized GCode text.
fn run_cooling_and_serialize(config: &ConfigView, layers: &mut Vec<LayerCollectionIR>) -> String {
    let module = PartCooling::from_config(config).expect("config must be valid");
    let sdk_layers: Vec<slicer_sdk::traits::LayerCollectionView> = layers
        .iter()
        .map(|l| slicer_sdk::traits::LayerCollectionView::new(l.clone()))
        .collect();
    let mut output = slicer_sdk::traits::FinalizationOutputBuilder::new();
    module
        .run_finalization(&sdk_layers, &mut output, config)
        .expect("run_finalization must succeed");
    output.apply_to(layers).expect("apply_to must succeed");

    let blackboard = blackboard_fixture();
    let emitter = DefaultGCodeEmitter::new("1.0.0-test".to_string());
    let gcode_ir = emitter
        .emit_gcode(layers.as_slice(), &blackboard)
        .expect("emit_gcode must succeed");
    let serializer = DefaultGCodeSerializer::new();
    serializer
        .serialize_gcode(&gcode_ir)
        .expect("serialize_gcode must succeed")
}

/// Split serialized GCode text into per-layer sections using `;LAYER_CHANGE` as the delimiter.
///
/// Drops anything before the first `;LAYER_CHANGE` (the emitter writes a
/// header preamble there), so `sections[i]` corresponds to layer `i`.
fn layer_sections(text: &str) -> Vec<&str> {
    let mut sections = Vec::new();
    let mut positions: Vec<usize> = text
        .match_indices(";LAYER_CHANGE")
        .map(|(p, _)| p)
        .collect();
    if positions.is_empty() {
        return sections;
    }
    positions.push(text.len());
    for w in positions.windows(2) {
        sections.push(&text[w[0]..w[1]]);
    }
    sections
}

// ============================================================================
// Config schema tests
// ============================================================================

/// Loads the part-cooling module manifest from `modules/core-modules/` and
/// asserts that the eight cooling-related config keys are declared with the
/// expected types and defaults. The previous version of this test asserted
/// against a host-side `FullConfigSchema::default()` registry that was a
/// hand-maintained duplicate of this TOML; the registry was removed and the
/// assertions are now TOML-backed.
#[test]
fn cooling_keys_registered() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules")
        .join("core-modules")
        .join("part-cooling");
    let manifest_path = manifest_dir.join("part-cooling.toml");
    let wasm_path = manifest_dir.join("part-cooling.wasm");
    let module = load_module_from_paths(&manifest_path, &wasm_path)
        .expect("part-cooling.toml must ingest cleanly");
    let entries = &module.config_schema().entries;

    // (key, expected field_type, expected default-as-TOML-literal)
    let expected_int_keys = [
        ("fan_speed_min", "51"),
        ("fan_speed_max", "255"),
        ("disable_fan_first_layers", "1"),
        ("overhang_fan_speed", "100"),
    ];
    for (key, default_literal) in expected_int_keys {
        let entry = entries
            .get(key)
            .unwrap_or_else(|| panic!("part-cooling.toml is missing key {key}"));
        assert_eq!(entry.field_type, "int", "expected int type for {key}");
        assert_eq!(
            entry.default.as_deref(),
            Some(default_literal),
            "incorrect default for {key}"
        );
        assert_eq!(
            entry.group.as_deref(),
            Some("Cooling"),
            "expected Cooling group for {key}"
        );
    }

    let expected_bool_keys = [
        ("enable_overhang_fan", "true"),
        ("slow_down_for_layer_cooling", "true"),
    ];
    for (key, default_literal) in expected_bool_keys {
        let entry = entries
            .get(key)
            .unwrap_or_else(|| panic!("part-cooling.toml is missing key {key}"));
        assert_eq!(entry.field_type, "bool", "expected bool type for {key}");
        assert_eq!(
            entry.default.as_deref(),
            Some(default_literal),
            "incorrect default for {key}"
        );
        assert_eq!(
            entry.group.as_deref(),
            Some("Cooling"),
            "expected Cooling group for {key}"
        );
    }

    // Floats: compare numerically because the TOML crate may serialize 10.0
    // back as "10.0" or "10" depending on version.
    let expected_float_keys = [
        ("slow_down_min_speed", 10.0_f64),
        ("slow_down_layer_time", 5.0),
    ];
    for (key, expected_default) in expected_float_keys {
        let entry = entries
            .get(key)
            .unwrap_or_else(|| panic!("part-cooling.toml is missing key {key}"));
        assert_eq!(entry.field_type, "float", "expected float type for {key}");
        let raw = entry
            .default
            .as_deref()
            .unwrap_or_else(|| panic!("missing default for {key}"));
        let parsed: f64 = raw
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric default '{raw}' for {key}"));
        assert!(
            (parsed - expected_default).abs() < f64::EPSILON,
            "incorrect default for {key}: expected {expected_default}, got {parsed}"
        );
        assert_eq!(
            entry.group.as_deref(),
            Some("Cooling"),
            "expected Cooling group for {key}"
        );
    }
}

// ============================================================================
// Positive acceptance criteria
// ============================================================================

#[test]
fn m106_present_after_layer_2() {
    let config = config_view(&[
        ("fan_speed_max", ConfigValue::Int(255)),
        ("disable_fan_first_layers", ConfigValue::Int(1)),
        ("enable_overhang_fan", ConfigValue::Bool(false)),
    ]);

    let entity = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layers = vec![
        layer_with_entity(0, 0.2, entity.clone()),
        layer_with_entity(1, 0.4, entity.clone()),
        layer_with_entity(2, 0.6, entity.clone()),
    ];

    let text = run_cooling_and_serialize(&config, &mut layers);
    let sections = layer_sections(&text);

    assert!(
        sections.len() >= 3,
        "expected at least 3 layer sections, got {}",
        sections.len()
    );
    // layer 0: M107 only
    assert!(
        sections[0].contains("M107"),
        "layer 0 should have M107 (fan off)"
    );
    // layer 2: M106 S255 present
    assert!(
        sections[2].contains("M106 S255"),
        "layer 2 should have M106 S255"
    );
}

#[test]
fn fan_off_before_end_gcode() {
    let config = config_view(&[
        ("fan_speed_max", ConfigValue::Int(255)),
        ("disable_fan_first_layers", ConfigValue::Int(1)),
        ("enable_overhang_fan", ConfigValue::Bool(false)),
    ]);

    let entity = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layers = vec![
        layer_with_entity(0, 0.2, entity.clone()),
        layer_with_entity(1, 0.4, entity.clone()),
    ];

    let text = run_cooling_and_serialize(&config, &mut layers);

    // The trailing M107 should be present after the last layer.
    assert!(
        text.contains("M107"),
        "M107 must be present to turn fan off after last layer"
    );
    // Count M107 occurrences: one for layer 0 + one trailing = 2
    let m107_count = text.matches("M107").count();
    assert_eq!(m107_count, 2, "expected exactly 2 M107 commands");
}

#[test]
fn fan_disabled_on_first_layers() {
    let config = config_view(&[
        ("fan_speed_max", ConfigValue::Int(255)),
        ("disable_fan_first_layers", ConfigValue::Int(2)),
        ("enable_overhang_fan", ConfigValue::Bool(false)),
    ]);

    let entity = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layers = vec![
        layer_with_entity(0, 0.2, entity.clone()),
        layer_with_entity(1, 0.4, entity.clone()),
        layer_with_entity(2, 0.6, entity.clone()),
        layer_with_entity(3, 0.8, entity.clone()),
    ];

    let text = run_cooling_and_serialize(&config, &mut layers);
    let sections = layer_sections(&text);

    assert!(sections.len() >= 4);
    // layers 0,1 must have no M106 S>0
    for (idx, section) in sections.iter().take(2).enumerate() {
        assert!(
            !section.contains("M106 S"),
            "layer {} must not contain any M106 command",
            idx
        );
    }
    // layers 2,3 must have M106 S255
    assert!(
        sections[2].contains("M106 S255"),
        "layer 2 should have M106 S255"
    );
    assert!(
        sections[3].contains("M106 S255"),
        "layer 3 should have M106 S255"
    );
}

#[test]
fn overhang_fan_bumped() {
    let config = config_view(&[
        ("fan_speed_max", ConfigValue::Int(255)),
        ("disable_fan_first_layers", ConfigValue::Int(0)),
        ("enable_overhang_fan", ConfigValue::Bool(true)),
        ("overhang_fan_speed", ConfigValue::Int(100)),
    ]);

    let wall = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let bridge = print_entity_fixture(
        vec![point3_with_width(1.0, 1.0, 0.2)],
        ExtrusionRole::BridgeInfill,
    );
    let mut layers = vec![LayerCollectionIR {
        schema_version: semver_fixture(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![wall.clone(), bridge.clone(), wall.clone()],
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }];

    let text = run_cooling_and_serialize(&config, &mut layers);

    // There should be an M106 S255 for the overhang bump.
    let m106_count = text.matches("M106 S255").count();
    // base layer + overhang bump + restore = 3
    assert!(
        m106_count >= 3,
        "expected at least 3 M106 S255 commands (base + bump + restore), got {}",
        m106_count
    );
}

#[test]
fn cooling_module_invoked_in_finalization() {
    let config = config_view(&[
        ("fan_speed_max", ConfigValue::Int(255)),
        ("disable_fan_first_layers", ConfigValue::Int(0)),
        ("enable_overhang_fan", ConfigValue::Bool(false)),
    ]);

    let entity = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layers = vec![layer_with_entity(0, 0.2, entity)];

    let text = run_cooling_and_serialize(&config, &mut layers);

    assert!(
        text.contains("M106 S255"),
        "cooling module must emit M106 S255 when enabled"
    );
}

// ============================================================================
// Negative cases
// ============================================================================

#[test]
fn rejects_phantom_fan_when_disabled() {
    let config = config_view(&[
        ("fan_speed_max", ConfigValue::Int(0)),
        ("disable_fan_first_layers", ConfigValue::Int(1)),
        ("enable_overhang_fan", ConfigValue::Bool(false)),
    ]);

    let entity = print_entity_fixture(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
    );
    let mut layers = vec![
        layer_with_entity(0, 0.2, entity.clone()),
        layer_with_entity(1, 0.4, entity.clone()),
    ];

    let text = run_cooling_and_serialize(&config, &mut layers);

    // Zero M106 S>0
    assert!(
        !text.contains("M106 S"),
        "fan disabled: must contain no M106 commands"
    );
    // Exactly one M107
    let m107_count = text.matches("M107").count();
    assert_eq!(m107_count, 1, "fan disabled: expected exactly one M107");
}

// Note: `rejects_cooling_missing_when_required` was moved to
// `tests/e2e/benchy_end_to_end_tdd.rs` (it runs the real pnp_cli binary
// end-to-end and was structurally an e2e test). The cache machinery for
// the no-part-cooling module directory lives in
// `tests/common/slicer_cache.rs` as `ModuleDirKind::PartCoolingFiltered`.
