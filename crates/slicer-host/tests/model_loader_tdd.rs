//! TDD tests for TASK-076: file format loaders (STL/OBJ/3MF).

use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

use slicer_host::model_loader::{detect_format, load_model, ModelFormat, ModelLoadError};
use slicer_ir::{PaintSemantic, PaintValue};

// ---------------------------------------------------------------------------
// Helper: generate a minimal binary STL cube (12 triangles, 8 unique vertices)
// ---------------------------------------------------------------------------
fn write_binary_stl_cube(w: &mut impl Write) {
    // 80-byte header
    w.write_all(&[0u8; 80]).unwrap();
    // triangle count: 12
    w.write_all(&12u32.to_le_bytes()).unwrap();

    // Unit cube vertices for 12 triangles (2 per face, 6 faces)
    let tris: [[[f32; 3]; 3]; 12] = [
        // -Z face (z=0)
        [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
        // +Z face (z=1)
        [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0]],
        [[0.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]],
        // -X face (x=0)
        [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
        [[0.0, 0.0, 0.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]],
        // +X face (x=1)
        [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]],
        [[1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]],
        // -Y face (y=0)
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0]],
        [[0.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
        // +Y face (y=1)
        [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0]],
        [[0.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
    ];

    for tri in &tris {
        // normal (unused, 3 floats)
        w.write_all(&0.0f32.to_le_bytes()).unwrap();
        w.write_all(&0.0f32.to_le_bytes()).unwrap();
        w.write_all(&0.0f32.to_le_bytes()).unwrap();
        // 3 vertices
        for v in tri {
            for c in v {
                w.write_all(&c.to_le_bytes()).unwrap();
            }
        }
        // attribute byte count
        w.write_all(&0u16.to_le_bytes()).unwrap();
    }
}

fn binary_stl_cube_file() -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".stl").tempfile().unwrap();
    write_binary_stl_cube(&mut f);
    f.flush().unwrap();
    f
}

fn ascii_stl_cube_file() -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".stl").tempfile().unwrap();
    write!(
        f,
        r#"solid cube
  facet normal 0 0 -1
    outer loop
      vertex 0 0 0
      vertex 1 1 0
      vertex 1 0 0
    endloop
  endfacet
  facet normal 0 0 -1
    outer loop
      vertex 0 0 0
      vertex 0 1 0
      vertex 1 1 0
    endloop
  endfacet
  facet normal 0 0 1
    outer loop
      vertex 0 0 1
      vertex 1 0 1
      vertex 1 1 1
    endloop
  endfacet
  facet normal 0 0 1
    outer loop
      vertex 0 0 1
      vertex 1 1 1
      vertex 0 1 1
    endloop
  endfacet
endsolid cube
"#
    )
    .unwrap();
    f.flush().unwrap();
    f
}

fn obj_cube_file() -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".obj").tempfile().unwrap();
    write!(
        f,
        r#"# unit cube
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
v 1.0 0.0 1.0
v 1.0 1.0 1.0
v 0.0 1.0 1.0
f 1 3 2
f 1 4 3
f 5 6 7
f 5 7 8
f 1 5 8
f 1 8 4
f 2 3 7
f 2 7 6
f 1 2 6
f 1 6 5
f 4 8 7
f 4 7 3
"#
    )
    .unwrap();
    f.flush().unwrap();
    f
}

fn threemf_cube_file() -> NamedTempFile {
    use std::io::Cursor;
    let model_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />
          <vertex x="0" y="0" z="1" />
          <vertex x="1" y="0" z="1" />
          <vertex x="1" y="1" z="1" />
          <vertex x="0" y="1" z="1" />
        </vertices>
        <triangles>
          <triangle v1="0" v2="2" v3="1" />
          <triangle v1="0" v2="3" v3="2" />
          <triangle v1="4" v2="5" v3="6" />
          <triangle v1="4" v2="6" v3="7" />
          <triangle v1="0" v2="4" v3="7" />
          <triangle v1="0" v2="7" v3="3" />
          <triangle v1="1" v2="2" v3="6" />
          <triangle v1="1" v2="6" v3="5" />
          <triangle v1="0" v2="1" v3="5" />
          <triangle v1="0" v2="5" v3="4" />
          <triangle v1="3" v2="7" v3="6" />
          <triangle v1="3" v2="6" v3="2" />
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" />
  </build>
</model>"#;

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip_writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip_writer.start_file("3D/3dmodel.model", options).unwrap();
        zip_writer.write_all(model_xml.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
    }

    let mut f = tempfile::Builder::new().suffix(".3mf").tempfile().unwrap();
    f.write_all(&buf).unwrap();
    f.flush().unwrap();
    f
}

fn threemf_custom_paint_file(vertices_xml: &str, triangle_xml: &str) -> NamedTempFile {
    use std::io::Cursor;
    let model_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
{vertices_xml}
        </vertices>
        <triangles>
{triangle_xml}
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" />
  </build>
</model>"#
    );

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip_writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip_writer.start_file("3D/3dmodel.model", options).unwrap();
        zip_writer.write_all(model_xml.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
    }

    let mut f = tempfile::Builder::new().suffix(".3mf").tempfile().unwrap();
    f.write_all(&buf).unwrap();
    f.flush().unwrap();
    f
}

// ---------------------------------------------------------------------------
// 3MF paint_fuzzy_skin tests
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn load_stl_binary_cube() {
    let f = binary_stl_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);
    let its = &mesh_ir.objects[0].mesh;
    assert_eq!(its.indices.len(), 36, "12 triangles * 3 indices");
    assert!(!its.vertices.is_empty());
    // vertices are deduplicated: a cube has 8 unique vertices
    assert_eq!(its.vertices.len(), 8);
}

#[test]
fn load_stl_ascii_cube() {
    let f = ascii_stl_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);
    let its = &mesh_ir.objects[0].mesh;
    // ASCII cube with 4 triangles
    assert_eq!(its.indices.len(), 12, "4 triangles * 3 indices");
    assert!(!its.vertices.is_empty());
}

#[test]
fn load_obj_cube() {
    let f = obj_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);
    let its = &mesh_ir.objects[0].mesh;
    assert_eq!(its.indices.len(), 36, "12 triangles * 3 indices");
    assert_eq!(its.vertices.len(), 8, "cube has 8 unique vertices");
}

#[test]
fn load_3mf_cube() {
    let f = threemf_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);
    let its = &mesh_ir.objects[0].mesh;
    assert_eq!(its.indices.len(), 36, "12 triangles * 3 indices");
    assert_eq!(its.vertices.len(), 8, "cube has 8 unique vertices");
}

#[test]
fn detect_format_by_extension() {
    assert_eq!(detect_format("model.stl").unwrap(), ModelFormat::Stl);
    assert_eq!(detect_format("model.STL").unwrap(), ModelFormat::Stl);
    assert_eq!(detect_format("model.obj").unwrap(), ModelFormat::Obj);
    assert_eq!(detect_format("model.OBJ").unwrap(), ModelFormat::Obj);
    assert_eq!(detect_format("model.3mf").unwrap(), ModelFormat::ThreeMf);
    assert_eq!(detect_format("model.3MF").unwrap(), ModelFormat::ThreeMf);
}

#[test]
fn unknown_extension_error() {
    let err = detect_format("model.xyz").unwrap_err();
    assert!(matches!(err, ModelLoadError::UnsupportedFormat(_)));
}

#[test]
fn nonexistent_file_error() {
    let err = load_model(&PathBuf::from("/nonexistent/model.stl")).unwrap_err();
    assert!(matches!(err, ModelLoadError::Io(_)));
}

#[test]
fn load_model_produces_mesh_ir() {
    let f = binary_stl_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.schema_version.major, 1);
    assert!(!mesh_ir.objects.is_empty());
    // Each object has an identity transform
    let transform = mesh_ir.objects[0].transform;
    assert_eq!(transform.matrix[0], 1.0, "identity diagonal");
    assert_eq!(transform.matrix[5], 1.0, "identity diagonal");
    assert_eq!(transform.matrix[10], 1.0, "identity diagonal");
    assert_eq!(transform.matrix[15], 1.0, "identity diagonal");
}

#[test]
fn bounding_box_computed() {
    let f = binary_stl_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    let bb = mesh_ir.build_volume;
    // Unit cube: min ~(0,0,0), max ~(1,1,1)
    assert!((bb.min.x - 0.0).abs() < 1e-5);
    assert!((bb.min.y - 0.0).abs() < 1e-5);
    assert!((bb.min.z - 0.0).abs() < 1e-5);
    assert!((bb.max.x - 1.0).abs() < 1e-5);
    assert!((bb.max.y - 1.0).abs() < 1e-5);
    assert!((bb.max.z - 1.0).abs() < 1e-5);
}

#[test]
fn pipeline_config_accepts_mesh_ir() {
    // Verify that PipelineConfig can accept a loaded mesh_ir
    // (this just verifies the type integration compiles)
    let f = binary_stl_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert!(!mesh_ir.objects.is_empty());
    // The MeshIR is compatible with Arc wrapping for pipeline use
    let _arc = std::sync::Arc::new(mesh_ir);
}

// ---------------------------------------------------------------------------
// 3MF paint_fuzzy_skin tests
// ---------------------------------------------------------------------------

fn threemf_paint_file(triangle_xml: &str) -> NamedTempFile {
    use std::io::Cursor;
    let model_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />
        </vertices>
        <triangles>
{triangle_xml}
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" />
  </build>
</model>"#
    );

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip_writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip_writer.start_file("3D/3dmodel.model", options).unwrap();
        zip_writer.write_all(model_xml.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
    }

    let mut f = tempfile::Builder::new().suffix(".3mf").tempfile().unwrap();
    f.write_all(&buf).unwrap();
    f.flush().unwrap();
    f
}

#[test]
fn load_3mf_extracts_fuzzy_skin_facets() {
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_fuzzy_skin="4" />
          <triangle v1="0" v2="2" v3="3" />"#;
    let f = threemf_paint_file(triangle_xml);
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);

    let paint_data = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data should be present");
    assert_eq!(paint_data.layers.len(), 1);
    assert_eq!(paint_data.layers[0].semantic, PaintSemantic::FuzzySkin);

    let its = &mesh_ir.objects[0].mesh;
    let facet_count = its.indices.len() / 3;
    assert_eq!(paint_data.layers[0].facet_values.len(), facet_count);
    assert_eq!(facet_count, 2);

    let has_painted = paint_data.layers[0]
        .facet_values
        .iter()
        .any(|v| matches!(v, Some(PaintValue::Flag(true))));
    assert!(has_painted, "at least one facet should be painted");
    assert!(paint_data.layers[0].strokes.is_empty());
}

#[test]
fn load_3mf_malformed_fuzzy_skin_rejects() {
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_fuzzy_skin="999" />"#;
    let f = threemf_paint_file(triangle_xml);
    let err = load_model(f.path()).unwrap_err();
    assert!(
        matches!(err, ModelLoadError::PaintMetadata { .. }),
        "expected PaintMetadata error, got {:?}",
        err
    );
}

#[test]
fn load_3mf_without_paint_returns_none() {
    let f = threemf_cube_file();
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);
    assert!(
        mesh_ir.objects[0].paint_data.is_none(),
        "paint_data should be None when no paint attributes are present"
    );
}

#[test]
fn load_3mf_extracts_support_facets() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />
          <vertex x="0" y="0" z="1" />
          <vertex x="1" y="0" z="1" />
          <vertex x="1" y="1" z="1" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_supports="4" />
          <triangle v1="0" v2="2" v3="3" paint_supports="8" />
          <triangle v1="4" v2="5" v3="6" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);

    let paint_data = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data should be present");
    assert_eq!(paint_data.layers.len(), 2);

    let its = &mesh_ir.objects[0].mesh;
    let facet_count = its.indices.len() / 3;
    assert_eq!(facet_count, 3);

    let enforcer_layer = paint_data
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::SupportEnforcer)
        .expect("SupportEnforcer layer should exist");
    assert_eq!(enforcer_layer.facet_values.len(), facet_count);
    assert_eq!(enforcer_layer.facet_values[0], Some(PaintValue::Flag(true)));
    assert_eq!(enforcer_layer.facet_values[1], None);
    assert_eq!(enforcer_layer.facet_values[2], None);

    let blocker_layer = paint_data
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::SupportBlocker)
        .expect("SupportBlocker layer should exist");
    assert_eq!(blocker_layer.facet_values.len(), facet_count);
    assert_eq!(blocker_layer.facet_values[0], None);
    assert_eq!(blocker_layer.facet_values[1], Some(PaintValue::Flag(true)));
    assert_eq!(blocker_layer.facet_values[2], None);
}

#[test]
fn load_3mf_extracts_seam_facets() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_seam="4" />
          <triangle v1="0" v2="2" v3="3" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);

    let paint_data = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data should be present");

    let its = &mesh_ir.objects[0].mesh;
    let facet_count = its.indices.len() / 3;
    assert_eq!(facet_count, 2);

    let seam_layer = paint_data
        .layers
        .iter()
        .find(|l| matches!(l.semantic, PaintSemantic::Custom(ref s) if s == "seam_enforcer"))
        .expect("seam_enforcer layer should exist");
    assert_eq!(seam_layer.facet_values.len(), facet_count);
    assert_eq!(seam_layer.facet_values[0], Some(PaintValue::Flag(true)));
    assert_eq!(seam_layer.facet_values[1], None);
}

#[test]
fn load_3mf_extracts_mmu_color() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />
          <vertex x="0" y="0" z="1" />
          <vertex x="1" y="0" z="1" />
          <vertex x="1" y="1" z="1" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_color="4" />
          <triangle v1="0" v2="2" v3="3" paint_color="8" />
          <triangle v1="4" v2="5" v3="6" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh_ir = load_model(f.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);

    let paint_data = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data should be present");

    let its = &mesh_ir.objects[0].mesh;
    let facet_count = its.indices.len() / 3;
    assert_eq!(facet_count, 3);

    let color_layer = paint_data
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::Material)
        .expect("Material layer should exist");
    assert_eq!(color_layer.facet_values.len(), facet_count);
    assert_eq!(color_layer.facet_values[0], Some(PaintValue::ToolIndex(0)));
    assert_eq!(color_layer.facet_values[1], Some(PaintValue::ToolIndex(1)));
    assert_eq!(color_layer.facet_values[2], None);
}

#[test]
fn load_3mf_malformed_support_value_rejects() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_supports="16" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let err = load_model(f.path()).unwrap_err();
    assert!(
        matches!(err, ModelLoadError::PaintMetadata { .. }),
        "expected PaintMetadata error, got {:?}",
        err
    );
}

#[test]
fn load_3mf_truncated_paint_tree_rejects() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_color="101" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let err = load_model(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("unexpected end"),
        "expected truncated tree error, got {:?}",
        err
    );
}

#[test]
fn load_3mf_invalid_paint_hex_rejects() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_fuzzy_skin="GG" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let err = load_model(f.path()).unwrap_err();
    assert!(
        matches!(err, ModelLoadError::PaintMetadata { .. }),
        "expected PaintMetadata error, got {:?}",
        err
    );
    assert!(
        err.to_string().contains("invalid hex digit"),
        "expected 'invalid hex digit' in error message, got {:?}",
        err
    );
}

#[test]
fn load_3mf_subdivision_dominant_state() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="1" y="1" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_color="401" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh_ir = load_model(f.path()).unwrap();
    let paint_data = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint data should exist");
    let material_layer = paint_data
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::Material)
        .expect("Material layer should exist");
    assert_eq!(material_layer.facet_values.len(), 1);
    assert!(
        matches!(
            material_layer.facet_values[0],
            Some(PaintValue::ToolIndex(0))
        ),
        "expected dominant state to map to ToolIndex(0) (0-based), got {:?}",
        material_layer.facet_values[0]
    );
}

#[test]
fn load_3mf_benchy_4color_loads() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/benchy_4color.3mf"
    );
    let result = load_model(&PathBuf::from(path));
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let objects = result.unwrap();
    // AC-2: Material layer present with at least one ToolIndex
    let has_material = objects.objects.iter().any(|obj| {
        obj.paint_data.as_ref().map_or(false, |pd| {
            pd.layers.iter().any(|l| {
                matches!(l.semantic, PaintSemantic::Material)
                    && l.facet_values
                        .iter()
                        .any(|v| matches!(v, Some(PaintValue::ToolIndex(_))))
            })
        })
    });
    assert!(
        has_material,
        "expected Material layer with ToolIndex entries"
    );
    // AC-3: SupportEnforcer layer present
    let has_support = objects.objects.iter().any(|obj| {
        obj.paint_data.as_ref().map_or(false, |pd| {
            pd.layers.iter().any(|l| {
                matches!(l.semantic, PaintSemantic::SupportEnforcer)
                    && l.facet_values
                        .iter()
                        .any(|v| matches!(v, Some(PaintValue::Flag(true))))
            })
        })
    });
    assert!(
        has_support,
        "expected SupportEnforcer layer with Flag(true) entries"
    );
}

#[test]
fn load_3mf_benchy_4color_strokes_populated() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/benchy_4color.3mf"
    );
    let mesh_ir = load_model(&PathBuf::from(path)).expect("should load without error");
    let has_strokes = mesh_ir.objects.iter().any(|obj| {
        obj.paint_data.as_ref().map_or(false, |pd| {
            pd.layers
                .iter()
                .any(|l| matches!(l.semantic, PaintSemantic::Material) && !l.strokes.is_empty())
        })
    });
    assert!(has_strokes, "expected non-empty strokes in Material layer");
    // AC-8: all stroke triangles are non-degenerate
    for obj in &mesh_ir.objects {
        if let Some(pd) = &obj.paint_data {
            for layer in &pd.layers {
                for stroke in &layer.strokes {
                    for tri in &stroke.triangles {
                        let [a, b, c] = tri;
                        assert!(
                            a != b && b != c && a != c,
                            "degenerate stroke triangle found"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn load_3mf_wholefacet_has_no_strokes() {
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_color="4" />"#;
    let f = threemf_paint_file(triangle_xml);
    let mesh_ir = load_model(f.path()).expect("should load");
    for obj in &mesh_ir.objects {
        if let Some(pd) = &obj.paint_data {
            for layer in &pd.layers {
                assert!(
                    layer.strokes.is_empty(),
                    "whole-facet paint should produce no strokes, semantic={:?}",
                    layer.semantic
                );
            }
        }
    }
}

#[test]
fn load_3mf_4color_has_mmu_and_support_layers() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/benchy_4color.3mf"
    );
    let mesh = load_model(&PathBuf::from(path)).unwrap();
    let pd = mesh.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    assert!(
        pd.layers
            .iter()
            .any(|l| l.semantic == PaintSemantic::Material),
        "expected Material layer"
    );
    assert!(
        pd.layers.iter().any(|l| matches!(
            l.semantic,
            PaintSemantic::SupportEnforcer | PaintSemantic::SupportBlocker
        )),
        "expected support layer"
    );
}

#[test]
fn load_3mf_4color_material_spans_four_tool_indices() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/benchy_4color.3mf"
    );
    let mesh = load_model(&PathBuf::from(path)).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let mat = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::Material)
        .expect("no Material layer");
    let indices: std::collections::HashSet<u32> = mat
        .facet_values
        .iter()
        .filter_map(|v| {
            if let Some(PaintValue::ToolIndex(n)) = v {
                Some(*n)
            } else {
                None
            }
        })
        .collect();
    assert!(
        indices.len() >= 4,
        "expected ≥4 distinct ToolIndex values, got {}: {:?}",
        indices.len(),
        indices
    );
    assert!(
        indices.contains(&0),
        "expected ToolIndex(0) present (0-based values), got {:?}",
        indices
    );
}

#[test]
fn load_3mf_4color_support_enforcer_has_facets() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/benchy_4color.3mf"
    );
    let mesh = load_model(&PathBuf::from(path)).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let sup = pd
        .layers
        .iter()
        .find(|l| {
            matches!(
                l.semantic,
                PaintSemantic::SupportEnforcer | PaintSemantic::SupportBlocker
            )
        })
        .expect("no support layer");
    let has_any = sup
        .facet_values
        .iter()
        .any(|v| matches!(v, Some(PaintValue::Flag(true))));
    assert!(has_any, "support layer has no painted facets");
}

#[test]
fn load_3mf_4color_layer_count_at_least_two() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/benchy_4color.3mf"
    );
    let mesh = load_model(&PathBuf::from(path)).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    assert!(
        pd.layers.len() >= 2,
        "expected ≥2 layers, got {}",
        pd.layers.len()
    );
}
