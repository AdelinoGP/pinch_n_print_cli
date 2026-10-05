//! Authored 3MF strings must pass the same typed boundary as explicit config.
use slicer_ir::ConfigValue;
use slicer_runtime::run::prepare_prepass_context;
use std::{collections::HashMap, io::Write, sync::Arc};

fn model_file() -> tempfile::NamedTempFile {
    let file = tempfile::Builder::new().suffix(".3mf").tempfile().unwrap();
    let model = r#"<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
<resources><object id="1" type="model"><mesh><vertices>
<vertex x="0" y="0" z="0"/><vertex x="2" y="0" z="0"/>
<vertex x="2" y="2" z="0"/><vertex x="0" y="2" z="0"/>
<vertex x="0" y="0" z="1"/><vertex x="2" y="0" z="1"/>
<vertex x="2" y="2" z="1"/><vertex x="0" y="2" z="1"/>
</vertices><triangles>
<triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="3" v3="2"/>
<triangle v1="4" v2="5" v3="6"/><triangle v1="4" v2="6" v3="7"/>
<triangle v1="0" v2="1" v3="5"/><triangle v1="0" v2="5" v3="4"/>
<triangle v1="1" v2="2" v3="6"/><triangle v1="1" v2="6" v3="5"/>
<triangle v1="2" v2="3" v3="7"/><triangle v1="2" v2="7" v3="6"/>
<triangle v1="3" v2="0" v3="4"/><triangle v1="3" v2="4" v3="7"/>
</triangles></mesh></object></resources><build><item objectid="1"/></build></model>"#;
    let sidecar = r#"<config><object id="1">
<metadata key="layer_height" value="0.1"/>
<metadata key="first_layer_line_width" value="0.6"/>
</object></config>"#;
    let mut writer = zip::ZipWriter::new(std::fs::File::create(file.path()).unwrap());
    for (name, contents) in [
        ("3D/3dmodel.model", model),
        ("Metadata/model_settings.config", sidecar),
    ] {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
    file
}

#[test]
fn real_3mf_object_string_reaches_production_resolution() {
    let file = model_file();
    let mesh = slicer_model_io::loader::load_model(file.path()).unwrap();
    assert_eq!(mesh.objects.len(), 1);
    assert_eq!(
        mesh.objects[0].config.data.get("layer_height"),
        Some(&ConfigValue::String("0.1".into()))
    );
    assert_eq!(
        mesh.objects[0].config.data.get("first_layer_line_width"),
        Some(&ConfigValue::String("0.6".into())),
        "model ingestion must preserve authored aliases for registry resolution"
    );
    let context =
        prepare_prepass_context(Arc::new(mesh), HashMap::new(), Vec::new(), &[], true, false)
            .expect("production prepass must type model object metadata");
    let map = context
        .blackboard
        .region_map()
        .expect("committed region map");
    assert!(!map.entries.is_empty());
    for key in map.entries.keys() {
        assert_eq!(map.config_for(key).layer_height, 0.1);
        assert_eq!(map.config_for(key).initial_layer_line_width.value, 0.6);
    }
    assert!(context.plan.global_layers.len() > 2);
    assert!((context.plan.global_layers[1].z - context.plan.global_layers[0].z - 0.1).abs() < 1e-6);
}

#[test]
fn explicit_object_override_wins_over_3mf_metadata() {
    let file = model_file();
    let mesh = slicer_model_io::loader::load_model(file.path()).unwrap();
    assert_eq!(mesh.objects.len(), 1);
    let object_id = &mesh.objects[0].id;
    let context = prepare_prepass_context(
        Arc::new(mesh.clone()),
        HashMap::from([
            (
                format!("object_config:{object_id}:layer_height"),
                ConfigValue::String("0.25".into()),
            ),
            (
                format!("object_config:{object_id}:initial_layer_line_width"),
                ConfigValue::String("0.55".into()),
            ),
        ]),
        Vec::new(),
        &[],
        true,
        false,
    )
    .expect("explicit scoped overrides must retain precedence over model metadata");
    let map = context
        .blackboard
        .region_map()
        .expect("committed region map");
    assert!(!map.entries.is_empty());
    for key in map.entries.keys() {
        assert_eq!(map.config_for(key).layer_height, 0.25);
        assert_eq!(map.config_for(key).initial_layer_line_width.value, 0.55);
    }
}
