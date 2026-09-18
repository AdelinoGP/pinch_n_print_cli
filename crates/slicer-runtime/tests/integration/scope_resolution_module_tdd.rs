//! Integration coverage for the shared typed scope resolver at both runtime entry points.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_config::ResolvedObjectLayerConfig;
use slicer_ir::{
    BoundingBox3, ConfigValue, IndexedTriangleSet, MeshIR, ModifierScope, ModifierVolume,
    ObjectConfig, ObjectMesh, Point3, Transform3d,
};
use slicer_runtime::run::{prepare_prepass_context, run_slice_with_collector, SliceRunOptions};

const OBJECT_ID: &str = "scope-resolution-cube";
const SECOND_OBJECT_ID: &str = "scope-resolution-cube-b";
const MODIFIER_ID: &str = "scope-resolution-modifier";
const EXPECTED_ZS: [f32; 4] = [0.3, 0.5, 0.7, 0.9];
const EXPECTED_MODEL_LAYER_COUNT: usize = 4;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn cube(origin: f32, extent: f32) -> IndexedTriangleSet {
    let lo = origin;
    let hi = origin + extent;
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: lo,
                y: lo,
                z: lo,
            },
            Point3 {
                x: hi,
                y: lo,
                z: lo,
            },
            Point3 {
                x: hi,
                y: hi,
                z: lo,
            },
            Point3 {
                x: lo,
                y: hi,
                z: lo,
            },
            Point3 {
                x: lo,
                y: lo,
                z: hi,
            },
            Point3 {
                x: hi,
                y: lo,
                z: hi,
            },
            Point3 {
                x: hi,
                y: hi,
                z: hi,
            },
            Point3 {
                x: lo,
                y: hi,
                z: hi,
            },
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
    }
}

fn fixture_mesh() -> Arc<MeshIR> {
    // exhaustive: modifier identity, geometry, scoped delta, priority, and applicability define the fixture.
    let mut modifier = ModifierVolume {
        id: MODIFIER_ID.to_string(),
        mesh: cube(0.2, 0.6),
        config_delta: Default::default(),
        priority: 10,
        applies_to: ModifierScope::AllFeatures,
    };
    modifier
        .config_delta
        .fields
        .insert("wall_count".to_string(), ConfigValue::Int(5));

    Arc::new(MeshIR {
        // exhaustive: object geometry and all scope-bearing fields define the fixture.
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
            mesh: cube(0.0, 1.0),
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig::default(),
            modifier_volumes: vec![modifier],
            paint_data: None,
            world_z_extent: Some((0.0, 1.0)),
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    })
}

fn config_source() -> HashMap<String, ConfigValue> {
    let mut source = HashMap::from([
        ("layer_height".to_string(), ConfigValue::Float(0.45)),
        ("first_layer_height".to_string(), ConfigValue::Float(0.45)),
        ("support_raft_layers".to_string(), ConfigValue::Int(2)),
        ("wall_count".to_string(), ConfigValue::Int(1)),
        ("nozzle_diameter".to_string(), ConfigValue::Float(0.4)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        (
            "wall_generator".to_string(),
            ConfigValue::String("classic".to_string()),
        ),
        ("enable_support".to_string(), ConfigValue::Bool(false)),
    ]);
    let object_key = |key: &str| format!("object_config:{OBJECT_ID}:{key}");
    source.insert(object_key("layer_height"), ConfigValue::Float(0.2));
    source.insert(object_key("first_layer_height"), ConfigValue::Float(0.3));
    source.insert(object_key("support_raft_layers"), ConfigValue::Int(0));
    source.insert(object_key("wall_count"), ConfigValue::Int(3));
    source
}

fn expected_planning() -> ResolvedObjectLayerConfig {
    // exhaustive: the five planning fields are the contract under test.
    ResolvedObjectLayerConfig {
        object_id: OBJECT_ID.to_string(),
        object_height: 1.0,
        layer_height: 0.2,
        first_layer_height: 0.3,
        support_raft_layers: 0,
    }
}

fn assert_literal_zs(actual: &[f32], path: &str) {
    assert_eq!(
        actual.len(),
        EXPECTED_ZS.len(),
        "{path}: object_height=1.0 and support_raft_layers=0 must yield four model layers; got {actual:?}"
    );
    for (index, (actual, expected)) in actual.iter().zip(EXPECTED_ZS).enumerate() {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "{path}: layer {index} must reflect first_layer_height=0.3 and layer_height=0.2; expected {expected}, got {actual}"
        );
    }
}

fn gcode_zs(gcode: &str) -> Vec<f32> {
    gcode
        .lines()
        .filter_map(|line| line.strip_prefix(";Z:"))
        .map(|value| value.parse::<f32>().expect(";Z value must be numeric"))
        .collect()
}

fn planning_evidence(mesh: &MeshIR, zs: &[f32]) -> ResolvedObjectLayerConfig {
    let object = mesh
        .objects
        .first()
        .expect("fixture must contain its one object");
    assert_eq!(mesh.objects.len(), 1, "fixture must contain one object");
    assert!(
        zs.len() >= 2,
        "planning output must contain multiple layers"
    );
    let (z_min, z_max) = object
        .world_z_extent
        .expect("fixture object must carry its world Z extent");
    let raft_layer_count = zs
        .len()
        .checked_sub(EXPECTED_MODEL_LAYER_COUNT)
        .expect("planning output omitted hand-counted model layers");

    // exhaustive: every field is independently observed from typed input or emitted layer-grid evidence.
    ResolvedObjectLayerConfig {
        object_id: object.id.clone(),
        object_height: f64::from(z_max - z_min),
        layer_height: f64::from(zs[1] - zs[0]),
        first_layer_height: f64::from(zs[0]),
        support_raft_layers: u32::try_from(raft_layer_count)
            .expect("raft layer count must fit u32"),
    }
}

fn assert_planning_eq(
    actual: &ResolvedObjectLayerConfig,
    expected: &ResolvedObjectLayerConfig,
    path: &str,
) {
    assert_eq!(actual.object_id, expected.object_id, "{path}: object_id");
    assert!(
        (actual.object_height - expected.object_height).abs() < 1.0e-6,
        "{path}: object_height expected {}, got {}",
        expected.object_height,
        actual.object_height
    );
    assert!(
        (actual.layer_height - expected.layer_height).abs() < 1.0e-6,
        "{path}: layer_height expected {}, got {}",
        expected.layer_height,
        actual.layer_height
    );
    assert!(
        (actual.first_layer_height - expected.first_layer_height).abs() < 1.0e-6,
        "{path}: first_layer_height expected {}, got {}",
        expected.first_layer_height,
        actual.first_layer_height
    );
    assert_eq!(
        actual.support_raft_layers, expected.support_raft_layers,
        "{path}: support_raft_layers"
    );
}

fn assert_region_values(context: &slicer_runtime::run::PrepassContext) {
    let region_map = context
        .blackboard
        .region_map()
        .expect("region mapping must commit resolved region configs");
    assert!(
        !region_map.entries.is_empty(),
        "fixture must produce regions"
    );

    let mut saw_base = false;
    let mut saw_modifier = false;
    for key in region_map
        .entries
        .keys()
        .filter(|key| key.object_id == OBJECT_ID)
    {
        let config = region_map.config_for(key).to_config_map();
        assert_eq!(
            config.get("layer_height"),
            Some(&ConfigValue::Float(0.2)),
            "object regions must retain the literal object layer_height"
        );
        match config.get("wall_count") {
            Some(ConfigValue::Int(3)) => saw_base = true,
            Some(ConfigValue::Int(5)) => saw_modifier = true,
            other => panic!("unexpected resolved region wall_count: {other:?}"),
        }
    }
    assert!(saw_base, "base object region must retain wall_count=3");
    assert!(
        saw_modifier,
        "modifier region must override the object with wall_count=5"
    );
}

#[test]
fn both_entry_points_share_resolution_results() {
    let mesh = fixture_mesh();
    let source = config_source();
    let module_dirs = vec![core_modules_dir()];
    let expected = expected_planning();

    let outcome = run_slice_with_collector(
        SliceRunOptions {
            mesh: Arc::clone(&mesh),
            model_label: "scope-resolution-module-fixture".to_string(),
            module_dirs: module_dirs.clone(),
            no_default_module_paths: true,
            config_overrides: source.clone(),
            ..SliceRunOptions::default()
        },
        None,
    )
    .expect("run_slice_with_collector must resolve the typed object scope");
    let full_slice_zs = gcode_zs(&outcome.gcode_text);
    assert_literal_zs(&full_slice_zs, "run_slice_with_collector");
    let full_slice_planning = planning_evidence(mesh.as_ref(), &full_slice_zs);
    assert_planning_eq(&full_slice_planning, &expected, "run_slice_with_collector");
    assert_eq!(
        full_slice_zs.len(),
        EXPECTED_MODEL_LAYER_COUNT,
        "full slice emitted Z layers must pin object_height and support_raft_layers"
    );
    let context = prepare_prepass_context(mesh, source, &module_dirs, true, false)
        .expect("prepare_prepass_context must resolve the same typed object scope");
    let prepass_zs: Vec<f32> = context
        .plan
        .global_layers
        .iter()
        .map(|layer| layer.z)
        .collect();
    assert_literal_zs(&prepass_zs, "prepare_prepass_context");
    let prepass_planning = planning_evidence(context.blackboard.mesh().as_ref(), &prepass_zs);
    assert_planning_eq(&prepass_planning, &expected, "prepare_prepass_context");
    assert_eq!(
        full_slice_zs, prepass_zs,
        "entry-point layer grids diverged"
    );

    for layer in context.plan.global_layers.iter() {
        assert!(
            layer
                .active_regions
                .iter()
                .all(|region| region.object_id == expected.object_id),
            "resolved planning object_id must be {}",
            expected.object_id
        );
    }
    assert_region_values(&context);
}

#[test]
fn multi_object_planning_carries_divergent_first_layer_and_raft_fields() {
    let mut mesh = fixture_mesh().as_ref().clone();
    let mut second = mesh.objects[0].clone();
    second.id = SECOND_OBJECT_ID.to_string();
    second.modifier_volumes.clear();
    mesh.objects.push(second);

    let mut source = config_source();
    source.insert("support_raft_layers".to_string(), ConfigValue::Int(1));
    source.insert(
        format!("object_config:{OBJECT_ID}:first_layer_height"),
        ConfigValue::Float(0.2),
    );
    source.insert(
        format!("object_config:{OBJECT_ID}:support_raft_layers"),
        ConfigValue::Int(0),
    );
    source.insert(
        format!("object_config:{SECOND_OBJECT_ID}:layer_height"),
        ConfigValue::Float(0.2),
    );
    source.insert(
        format!("object_config:{SECOND_OBJECT_ID}:first_layer_height"),
        ConfigValue::Float(0.3),
    );
    source.insert(
        format!("object_config:{SECOND_OBJECT_ID}:support_raft_layers"),
        ConfigValue::Int(2),
    );

    let context =
        prepare_prepass_context(Arc::new(mesh), source, &[core_modules_dir()], true, false)
            .expect("divergent per-object planning fields must cross the typed boundary");
    let prefix: Vec<(i32, bool)> = context
        .plan
        .global_layers
        .iter()
        .take(4)
        .map(|layer| ((layer.z * 10.0).round() as i32, layer.is_raft))
        .collect();

    assert_eq!(
        prefix,
        vec![(3, true), (5, true), (7, false), (8, false)],
        "object B's two-layer raft and 0.3 first layer must coexist with object A's zero raft and 0.2 first layer"
    );
}
