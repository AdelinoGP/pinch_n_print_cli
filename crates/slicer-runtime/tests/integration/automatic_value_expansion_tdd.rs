//! Runtime-boundary coverage for automatic config-value expansion.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::flow::{resolve_role_width, RoleWidthContext};
use slicer_ir::{
    BoundingBox3, ConfigValue, ExtrusionRole, FacetPaintData, IndexedTriangleSet, MeshIR,
    ModifierScope, ModifierVolume, ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic, PaintValue,
    Point3, ResolvedConfig, Transform3d,
};
use slicer_runtime::run::{prepare_prepass_context, run_slice_with_collector, SliceRunOptions};
use tempfile::TempDir;

const OBJECT_ID: &str = "automatic-expansion-cube";
const WIDTH_KEYS: &[&str] = &[
    "outer_wall_line_width",
    "inner_wall_line_width",
    "sparse_infill_line_width",
    "internal_solid_infill_line_width",
    "top_surface_line_width",
    "support_line_width",
];
const COVERED_PERCENT_KEYS: &[&str] = &[
    "initial_layer_line_width",
    "bridge_line_width",
    "outer_wall_line_width",
    "inner_wall_line_width",
    "sparse_infill_line_width",
    "internal_solid_infill_line_width",
    "top_surface_line_width",
    "support_line_width",
    "overhang_1_4_speed",
    "overhang_2_4_speed",
    "overhang_3_4_speed",
    "overhang_4_4_speed",
];
const OVERHANG_EXPECTED: &[(&str, f64)] = &[
    ("overhang_1_4_speed", 15.0),
    ("overhang_2_4_speed", 30.0),
    ("overhang_3_4_speed", 45.0),
    ("overhang_4_4_speed", 60.0),
];

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

fn material_region_split_module() -> TempDir {
    let directory = tempfile::tempdir().expect("material region-split fixture directory");
    let source_dir = core_modules_dir().join("classic-perimeters");
    let manifest = fs::read_to_string(source_dir.join("classic-perimeters.toml"))
        .expect("classic-perimeters manifest must be readable");
    let manifest = format!(
        "{manifest}\n\n[[region_split]]\nsemantic = \"material\"\npriority = 100\nvalue_type = \"tool_index\"\n"
    );
    fs::write(directory.path().join("classic-perimeters.toml"), manifest)
        .expect("material region-split manifest must be written");
    fs::copy(
        source_dir.join("classic-perimeters.wasm"),
        directory.path().join("classic-perimeters.wasm"),
    )
    .expect("classic-perimeters WASM must be copied for the fixture");
    directory
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
    let object_mesh = cube(0.0, 10.0);
    let facet_count = object_mesh.indices.len() / 3;
    // exhaustive: the modifier identity and applicability are part of this interning fixture.
    let mut modifier = ModifierVolume {
        id: "automatic-expansion-modifier".to_string(),
        mesh: cube(2.0, 6.0),
        config_delta: Default::default(),
        priority: 0,
        applies_to: ModifierScope::AllFeatures,
    };
    modifier
        .config_delta
        .fields
        .insert("wall_count".to_string(), ConfigValue::Int(7));

    Arc::new(MeshIR {
        // exhaustive: this fixture intentionally pins every ObjectMesh field.
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
            mesh: object_mesh,
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: ObjectConfig::default(),
            modifier_volumes: vec![modifier],
            paint_data: Some(FacetPaintData {
                layers: vec![PaintLayer {
                    semantic: PaintSemantic::Material,
                    facet_values: vec![Some(PaintValue::ToolIndex(1)); facet_count],
                    strokes: vec![],
                }],
            }),
            world_z_extent: Some((0.0, 10.0)),
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

fn scoped_placeholders(prefix: &str, line_width: f64) -> HashMap<String, ConfigValue> {
    let key = |name: &str| {
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}:{name}")
        }
    };
    let mut values = HashMap::new();
    values.insert(key("nozzle_diameter"), ConfigValue::Float(0.4));
    values.insert(key("line_width"), ConfigValue::Float(line_width));
    for (index, width_key) in WIDTH_KEYS.iter().enumerate() {
        let value = if *width_key == "inner_wall_line_width" {
            ConfigValue::FloatOrPercent {
                value: 0.0,
                is_percent: false,
            }
        } else if index % 2 == 0 {
            ConfigValue::FloatOrPercent {
                value: 100.0,
                is_percent: true,
            }
        } else {
            ConfigValue::FloatOrPercent {
                value: 0.0,
                is_percent: false,
            }
        };
        values.insert(key(width_key), value);
    }
    values.insert(key("support_interface_top_layers"), ConfigValue::Int(3));
    values.insert(key("support_interface_bottom_layers"), ConfigValue::Int(-1));
    values.insert(key("support_interface_spacing"), ConfigValue::Float(0.35));
    values.insert(
        key("support_bottom_interface_spacing"),
        ConfigValue::Float(-1.0),
    );
    values.insert(key("outer_wall_speed"), ConfigValue::Float(60.0));
    for (index, (speed_key, _)) in OVERHANG_EXPECTED.iter().enumerate() {
        values.insert(
            key(speed_key),
            ConfigValue::FloatOrPercent {
                value: 25.0 * (index + 1) as f64,
                is_percent: true,
            },
        );
    }
    values
}

fn config_source() -> HashMap<String, ConfigValue> {
    let mut source = scoped_placeholders("", 0.45);
    source.extend(scoped_placeholders(
        &format!("object_config:{OBJECT_ID}"),
        0.50,
    ));
    source.extend(scoped_placeholders("tool_config:1", 0.60));
    source.extend(scoped_placeholders("paint_config:material", 0.55));
    source.insert(
        "wall_generator".to_string(),
        ConfigValue::String("classic".to_string()),
    );
    source
}

fn literal_float(map: &HashMap<String, ConfigValue>, key: &str, expected: f64, label: &str) {
    match map.get(key) {
        Some(ConfigValue::Float(actual))
        | Some(ConfigValue::FloatOrPercent {
            value: actual,
            is_percent: false,
        }) => assert!(
            (actual - expected).abs() < 1.0e-6,
            "{label}.{key} expected literal {expected}, got {actual}"
        ),
        other => panic!("{label}.{key} expected absolute literal {expected}, got {other:?}"),
    }
}

fn is_absolute_zero(value: &ConfigValue) -> bool {
    match value {
        ConfigValue::Float(value) => *value == 0.0,
        ConfigValue::Int(value) => *value == 0,
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => *value == 0.0,
        _ => false,
    }
}

fn is_absolute_minus_one(value: &ConfigValue) -> bool {
    match value {
        ConfigValue::Float(value) => *value == -1.0,
        ConfigValue::Int(value) => *value == -1,
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => *value == -1.0,
        _ => false,
    }
}

fn assert_no_covered_placeholders(map: &HashMap<String, ConfigValue>, label: &str) {
    for key in ["line_width", "support_line_width"] {
        if let Some(value) = map.get(key) {
            assert!(
                !is_absolute_zero(value),
                "{label}.{key} retained zero auto placeholder: {value:?}"
            );
        }
    }
    for key in COVERED_PERCENT_KEYS {
        if let Some(value) = map.get(*key) {
            assert!(
                !matches!(
                    value,
                    ConfigValue::Percent(_)
                        | ConfigValue::FloatOrPercent {
                            is_percent: true,
                            ..
                        }
                ),
                "{label}.{key} retained covered percent placeholder: {value:?}"
            );
        }
    }
    for key in [
        "support_interface_bottom_layers",
        "support_bottom_interface_spacing",
    ] {
        if let Some(value) = map.get(key) {
            assert!(
                !is_absolute_minus_one(value),
                "{label}.{key} retained -1 mirror placeholder: {value:?}"
            );
        }
    }
    // `support_threshold_overlap` is geometry-dependent and remains packet-10 owned.
}

fn assert_expanded(map: &HashMap<String, ConfigValue>, expected_width: f64, label: &str) {
    literal_float(map, "line_width", expected_width, label);
    for key in WIDTH_KEYS {
        let expected = match *key {
            "inner_wall_line_width" | "internal_solid_infill_line_width" => {
                0.0 // Role-zero dispatch signal, not Phase-B expansion.
            }
            "outer_wall_line_width"
            | "sparse_infill_line_width"
            | "top_surface_line_width"
            | "support_line_width" => 0.4,
            _ => expected_width,
        };
        literal_float(map, key, expected, label);
    }
    for (key, expected) in OVERHANG_EXPECTED {
        literal_float(map, key, *expected, label);
    }
    assert_no_covered_placeholders(map, label);
}

fn assert_expanded_view(view: &slicer_ir::ConfigView, expected_width: f64, label: &str) {
    let keys = std::iter::once("line_width")
        .chain(WIDTH_KEYS.iter().copied())
        .chain(OVERHANG_EXPECTED.iter().map(|(key, _)| *key));
    for key in keys {
        let expected = OVERHANG_EXPECTED
            .iter()
            .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
            .unwrap_or(match key {
                "inner_wall_line_width" | "internal_solid_infill_line_width" => {
                    0.0 // Role-zero dispatch signal, not Phase-B expansion.
                }
                "outer_wall_line_width"
                | "sparse_infill_line_width"
                | "top_surface_line_width"
                | "support_line_width" => 0.4,
                _ => expected_width,
            });
        match view.get(key) {
            Some(ConfigValue::Float(actual))
            | Some(ConfigValue::FloatOrPercent {
                value: actual,
                is_percent: false,
            }) => assert!(
                (actual - expected).abs() <= f64::from(f32::EPSILON),
                "{label}.{key} expected literal {expected}, got {actual}"
            ),
            Some(other) => {
                panic!("{label}.{key} expected absolute literal {expected}, got {other:?}")
            }
            None => {}
        }
    }
}

#[test]
fn runtime_expands_global_object_tool_and_paint_before_delivery() {
    let mesh = fixture_mesh();
    let source = config_source();
    let material_module = material_region_split_module();
    let module_dirs = vec![material_module.path().to_path_buf(), core_modules_dir()];

    let outcome = run_slice_with_collector(
        SliceRunOptions {
            mesh: Arc::clone(&mesh),
            model_label: "automatic-value-expansion-fixture".to_string(),
            module_dirs: module_dirs.clone(),
            no_default_module_paths: true,
            config_overrides: source.clone(),
            ..SliceRunOptions::default()
        },
        None,
    )
    .expect("run_slice_with_collector must accept fully scoped placeholder config");

    // Full-slice-path observables. The expanded global support width
    // (`0` auto resolves to the 0.4 nozzle) must reach the serializer width
    // header; an unexpanded placeholder would print `0` or `100` instead.
    assert!(
        outcome.layer_count > 0,
        "the full slice must execute at least one layer"
    );
    assert!(
        outcome
            .gcode_text
            .lines()
            .any(|line| line.trim() == "; support_line_width = 0.4"),
        "full-slice gcode must carry the expanded support width header"
    );
    // Every facet is painted tool 1, so the emitter — wired with the run-level
    // tool map — must route extrusion through a T1 change. This proves tool
    // delivery on the full-slice path; the map's expansion is pinned by the
    // `expand_scope_maps` unit test, which fails if the per-tool loop is cut.
    assert!(
        outcome.gcode_text.lines().any(|line| line.trim() == "T1"),
        "full-slice gcode must route the painted region through tool 1"
    );
    assert!(
        outcome.gcode_text.contains(";TYPE:Outer wall"),
        "full-slice gcode must contain executed outer-wall geometry"
    );

    let context = prepare_prepass_context(mesh, source, &module_dirs, true, false)
        .expect("prepare_prepass_context must accept fully scoped placeholder config");

    let mut bound_module_count = 0usize;
    for stage in context
        .plan
        .prepass_stages
        .iter()
        .chain(context.plan.per_layer_stages.iter())
        .chain(context.plan.postpass_stages.iter())
    {
        for module in &stage.modules {
            bound_module_count += 1;
            assert_expanded_view(
                module.config_view(),
                0.45,
                &format!("bound module {}", module.module_id()),
            );
        }
    }
    assert!(
        bound_module_count > 0,
        "fixture must bind at least one module"
    );

    let role_width_context = RoleWidthContext {
        line_width: 0.45,
        nozzle_diameter: 0.4,
        inner_wall_line_width: 0.0,
        ..RoleWidthContext::default()
    };
    assert_eq!(
        resolve_role_width(ExtrusionRole::InnerWall, false, false, &role_width_context),
        0.45,
        "zero inner-wall override must dispatch to the expanded line_width in Phase C"
    );

    let region_map = context
        .blackboard
        .region_map()
        .expect("region mapping prepass must commit RegionMapIR");
    assert!(
        !region_map.configs.is_empty(),
        "region mapping must intern effective configs"
    );
    assert_eq!(
        region_map.configs[0],
        ResolvedConfig::default(),
        "RegionMapIR.configs[0] must remain the default seed"
    );
    for (index, config) in region_map.configs.iter().enumerate().skip(1) {
        let map = config.to_config_map();
        let width = match map.get("line_width") {
            Some(ConfigValue::Float(width)) => *width,
            other => panic!("RegionMapIR.configs[{index}].line_width is not literal: {other:?}"),
        };
        assert!(
            [0.45, 0.50, 0.55, 0.60].contains(&width),
            "RegionMapIR.configs[{index}] has unexpected scoped line_width {width}"
        );
        assert_expanded(&map, width, &format!("RegionMapIR.configs[{index}]"));
    }

    let mut saw_base_parent = false;
    let mut saw_modifier_child = false;
    let mut saw_painted_parent = false;
    let mut saw_painted_modifier_child = false;
    for key in region_map.entries.keys() {
        let map = region_map.config_for(key).to_config_map();
        let painted = key
            .variant_chain
            .iter()
            .any(|(semantic, value)| semantic == "material" && value == &PaintValue::ToolIndex(1));
        let modified = matches!(map.get("wall_count"), Some(ConfigValue::Int(7)));
        match (painted, modified) {
            (false, false) => saw_base_parent = true,
            (false, true) => saw_modifier_child = true,
            (true, false) => saw_painted_parent = true,
            (true, true) => saw_painted_modifier_child = true,
        }
    }
    assert!(
        saw_base_parent,
        "base parent interning branch must be covered"
    );
    assert!(
        saw_modifier_child,
        "unpainted modifier-child interning branch must be covered"
    );
    assert!(
        saw_painted_parent,
        "painted parent interning branch must be covered"
    );
    assert!(
        saw_painted_modifier_child,
        "painted modifier-child interning branch must be covered"
    );
}
