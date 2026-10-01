//! End-to-end visual-debug bundle check for a synthetic modifier-bearing 3MF.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use pnp_cli::visual_debug::{
    run_visual_debug, FrameMode, LayerSelector, TapSelector, VisualDebugRequest, VisualDebugSource,
    VisualizationSpec,
};
use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
}

fn target_path(name: &str) -> PathBuf {
    workspace_root().join("target").join(name)
}

fn box_mesh_xml(min: [f64; 3], max: [f64; 3]) -> String {
    let vertices = [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], max[1], min[2]],
        [min[0], max[1], min[2]],
        [min[0], min[1], max[2]],
        [max[0], min[1], max[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    let triangles = [
        [0, 2, 1],
        [0, 3, 2], // bottom
        [4, 5, 6],
        [4, 6, 7], // top
        [0, 1, 5],
        [0, 5, 4], // front
        [1, 2, 6],
        [1, 6, 5], // right
        [2, 3, 7],
        [2, 7, 6], // back
        [3, 0, 4],
        [3, 4, 7], // left
    ];

    let mut xml = String::from("<mesh><vertices>");
    for [x, y, z] in vertices {
        xml.push_str(&format!("<vertex x=\"{x}\" y=\"{y}\" z=\"{z}\"/>"));
    }
    xml.push_str("</vertices><triangles>");
    for [v1, v2, v3] in triangles {
        xml.push_str(&format!("<triangle v1=\"{v1}\" v2=\"{v2}\" v3=\"{v3}\"/>"));
    }
    xml.push_str("</triangles></mesh>");
    xml
}

fn write_modifier_3mf(path: &Path) {
    fs::create_dir_all(path.parent().expect("3MF path has a parent"))
        .expect("create synthetic 3MF directory");
    let file = fs::File::create(path).expect("create synthetic modifier 3MF");
    let mut archive = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let entries = [
        (
            "[Content_Types].xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
             <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
             <Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/>\
             </Types>",
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
             <Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/>\
             </Relationships>",
        ),
    ];
    for (name, contents) in entries {
        archive
            .start_file(name, options)
            .expect("start synthetic 3MF archive entry");
        archive
            .write_all(contents.as_bytes())
            .expect("write synthetic 3MF archive entry");
    }

    let model = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\
           <resources>\
             <object id=\"1\" type=\"model\" name=\"printable body volume\">\
               <metadata name=\"object_type\">model_part</metadata>{}\
             </object>\
             <object id=\"2\" type=\"model\" name=\"typed modifier volume\">\
               <metadata name=\"object_type\">modifier</metadata>{}\
             </object>\
             <object id=\"3\" type=\"model\" name=\"synthetic modifier-bearing object\">\
               <metadata name=\"object_type\">model_part</metadata>\
               <components><component objectid=\"1\"/><component objectid=\"2\"/></components>\
             </object>\
           </resources>\
           <build><item objectid=\"3\"/></build>\
         </model>",
        box_mesh_xml([0.0, 0.0, 0.0], [20.0, 20.0, 0.4]),
        box_mesh_xml([5.0, 5.0, 0.0], [15.0, 15.0, 0.4]),
    );
    archive
        .start_file("3D/3dmodel.model", options)
        .expect("start synthetic 3MF model entry");
    archive
        .write_all(model.as_bytes())
        .expect("write synthetic 3MF model entry");
    let sidecar = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <config>\
          <object id=\"3\">\
            <part id=\"1\" subtype=\"normal_part\">\
              <metadata key=\"name\" value=\"printable body volume\"/>\
            </part>\
            <part id=\"2\" subtype=\"modifier_part\">\
              <metadata key=\"name\" value=\"typed modifier volume\"/>\
            </part>\
          </object>\
        </config>";
    archive
        .start_file("Metadata/model_settings.config", options)
        .expect("start synthetic 3MF sidecar entry");
    archive
        .write_all(sidecar.as_bytes())
        .expect("write synthetic 3MF sidecar entry");
    archive.finish().expect("finish synthetic modifier 3MF");
}

fn write_modifier_config() -> PathBuf {
    let path = target_path("typed_modifier_kind_visual_debug/project_settings.config.json");
    fs::create_dir_all(path.parent().expect("config path has a parent"))
        .expect("create synthetic modifier config directory");
    fs::write(&path, "{}").expect("write synthetic modifier config");
    path
}

fn model_request(model: PathBuf, config: PathBuf) -> VisualDebugRequest {
    // exhaustive: synthetic modifier visual-debug request boundary fixture
    VisualDebugRequest {
        schema_version: "1.2.0".to_string(),
        source: VisualDebugSource::Model {
            model: Some(model),
            config: Some(config),
            module_dirs: vec![workspace_root().join("modules").join("core-modules")],
            path: None,
        },
        layers: vec![LayerSelector::Index(0)],
        taps: vec![TapSelector::Name("PrePass::RegionMapping".to_string())],
        visualizations: vec![VisualizationSpec::Name("silhouette".to_string())],
        resolution_scale: 1,
        gcode_line_width_mm: None,
        frame: FrameMode::Model,
    }
}

fn has_layer_zero(value: &Value) -> bool {
    value
        .get("layer_index")
        .and_then(Value::as_i64)
        .map_or(false, |index| index == 0)
        || value
            .get("layers_rendered")
            .and_then(Value::as_array)
            .map_or(false, |layers| {
                layers.iter().any(|layer| {
                    layer.as_i64() == Some(0)
                        || (layer
                            .get("start")
                            .and_then(Value::as_i64)
                            .map_or(false, |start| start <= 0)
                            && layer
                                .get("end")
                                .and_then(Value::as_i64)
                                .map_or(false, |end| end >= 0))
                })
            })
}

fn find_region_mapping_silhouette(value: &Value) -> Option<&Value> {
    if value.get("tap").and_then(Value::as_str) == Some("PrePass::RegionMapping")
        && value.get("visualization").and_then(Value::as_str) == Some("silhouette")
        && has_layer_zero(value)
        && value.get("png_path").and_then(Value::as_str).is_some()
    {
        return Some(value);
    }

    match value {
        Value::Array(values) => values.iter().find_map(find_region_mapping_silhouette),
        Value::Object(values) => values.values().find_map(find_region_mapping_silhouette),
        _ => None,
    }
}

#[test]
fn typed_modifier_kind_renders_region_mapping_bundle() {
    let model = target_path("typed_modifier_kind_visual_debug/model.3mf");
    let output = target_path("typed_modifier_kind_visual_debug/bundle");
    write_modifier_3mf(&model);
    let config = write_modifier_config();

    let loaded =
        slicer_model_io::load_model(&model).expect("synthetic modifier 3MF should load as MeshIR");
    assert!(
        loaded.objects.iter().any(|object| {
            object
                .modifier_volumes
                .iter()
                .any(|modifier| modifier.kind == slicer_ir::ModifierKind::ParameterModifier)
        }),
        "synthetic 3MF must load a parameter-modifier volume before visual rendering"
    );

    let manifest_path = run_visual_debug(model_request(model, config), &output, true)
        .expect("synthetic modifier model should render a RegionMapping silhouette");
    assert!(manifest_path.is_file(), "manifest.json must be written");
    let manifest: Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("manifest.json must be readable"))
            .expect("manifest.json must contain valid JSON");

    let image_entry = find_region_mapping_silhouette(&manifest)
        .expect("manifest must index the layer-0 PrePass::RegionMapping silhouette image");
    let png_path = image_entry["png_path"]
        .as_str()
        .expect("RegionMapping silhouette entry must include a PNG path");
    assert!(!png_path.is_empty(), "PNG path must not be empty");
    let png_file = output.join(png_path);
    assert!(
        png_file.is_file(),
        "manifest image path must exist on disk: {}",
        png_file.display()
    );
}
