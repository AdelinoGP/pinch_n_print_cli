//! TDD tests for TASK-076: file format loaders (STL/OBJ/3MF).

use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

use slicer_ir::{PaintSemantic, PaintValue};
use slicer_model_io::loader::{detect_format, load_model, ModelFormat, ModelLoadError};

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

// NOTE (packet 89, Step 6): two retired-benchy load tests that previously sat
// here (Material+SupportEnforcer presence; strokes populated) were deleted
// because they are fully duplicated by the `cube_4color_*` block below —
// Material/ToolIndex/strokes coverage now comes from
// `load_3mf_cube_4color_loads`, `load_3mf_cube_4color_strokes_populated`, and
// `load_3mf_cube_4color_material_spans_4_tool_indices`. The retired fixture
// additionally carried `paint_supports` painting, which `cube_4color.3mf`
// intentionally does not — that signal moves to the executor-side support
// suites and is not a model_loader concern.

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

// NOTE (packet 89, Step 6): the four `load_3mf_4color_*` tests that previously
// sat here (mmu_and_support_layers, material_spans_four_tool_indices,
// support_enforcer_has_facets, layer_count_at_least_two) were deleted as
// duplicates of the cube_4color block below. The Material-layer/ToolIndex
// coverage is preserved by `load_3mf_cube_4color_material_spans_4_tool_indices`
// (which asserts EXACTLY 4 distinct indices — a strengthening over the prior
// `>= 4` retired-fixture bound). The support-layer assertions tested a fixture
// property (`paint_supports` in the retired multi-color 3MF) that
// `cube_4color.3mf` intentionally does not carry; that signal is covered by
// the executor-side support suites and is not a model_loader concern.

// ---------------------------------------------------------------------------
// cube_4color.3mf loader tests
// ---------------------------------------------------------------------------

fn cube_4color_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_4color.3mf"
    ))
}

fn cube_fuzzy_painted_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/cube_fuzzyPainted.3mf"
    ))
}

#[test]
fn load_3mf_cube_4color_loads() {
    let path = cube_4color_path();
    let result = load_model(&path);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let mesh_ir = result.unwrap();
    let pd = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    let has_material = pd
        .layers
        .iter()
        .any(|l| matches!(l.semantic, PaintSemantic::Material));
    assert!(has_material, "expected Material layer in cube_4color");
}

#[test]
fn load_3mf_cube_4color_material_spans_4_tool_indices() {
    let path = cube_4color_path();
    let mesh = load_model(&path).unwrap();
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
    assert_eq!(
        indices.len(),
        4,
        "expected exactly 4 distinct ToolIndex values (0=orange, 1=green, 2=blue, 3=red), got {}: {:?}",
        indices.len(),
        indices
    );
    for expected in [0, 1, 2, 3] {
        assert!(
            indices.contains(&expected),
            "expected ToolIndex({expected}) present"
        );
    }
}

#[test]
fn load_3mf_cube_4color_strokes_populated() {
    let path = cube_4color_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let mat = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::Material)
        .expect("no Material layer");
    assert!(
        !mat.strokes.is_empty(),
        "Material strokes must be non-empty; cube_4color has hex subdivision (circles, banding)"
    );
    for stroke in &mat.strokes {
        for tri in &stroke.triangles {
            let [a, b, c] = tri;
            assert!(
                a != b && b != c && a != c,
                "degenerate stroke triangle found: a={a:?} b={b:?} c={c:?}"
            );
        }
    }
}

#[test]
fn load_3mf_cube_4color_no_fuzzy_layer() {
    let path = cube_4color_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let has_fuzzy = pd
        .layers
        .iter()
        .any(|l| l.semantic == PaintSemantic::FuzzySkin);
    assert!(
        !has_fuzzy,
        "cube_4color has no fuzzy skin painting; FuzzySkin layer must be absent"
    );
}

#[test]
fn load_3mf_cube_4color_facet_coverage() {
    let path = cube_4color_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let mat = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::Material)
        .expect("no Material layer");
    let facet_count = mat.facet_values.len();
    assert_eq!(facet_count, 12, "cube has 12 triangles");
    // Paint has two mutually-exclusive per-facet representations:
    //   * SOLID-painted facets (a single colour over the whole triangle, encoded
    //     in 1-2 hex chars) carry a per-facet Material value in `facet_values`.
    //   * SUBDIVIDED facets (the front circles and left stripes) carry their paint
    //     as per-leaf `strokes`; their `facet_values` entry is None so phase3 does
    //     not also project a coarse half-face line that would conflict with the
    //     strokes (spurious colour boundary on the triangle diagonal).
    // The cube_4color fixture has 7 solid-painted facets (top/bottom/back/right),
    // 4 subdivided facets (front + left), and 1 genuinely unpainted facet.
    let facet_painted: usize = mat
        .facet_values
        .iter()
        .filter(|v| matches!(v, Some(PaintValue::ToolIndex(_))))
        .count();
    let none: usize = mat.facet_values.iter().filter(|v| v.is_none()).count();
    assert_eq!(facet_painted + none, 12, "all facets accounted for");
    assert!(
        facet_painted >= 6,
        "solid-painted facets must carry per-facet Material values, got {facet_painted}"
    );
    assert!(
        !mat.strokes.is_empty(),
        "subdivided facets (circles/stripes) must carry Material strokes"
    );
}

/// Documented coverage gap from packet 89 (Benchy 3MF Retirement).
///
/// Two deleted benchy-fixture tests covered properties that NO cube fixture currently
/// carries:
///
/// 1. SupportEnforcer / SupportBlocker layer PARSING against a real OrcaSlicer-exported
///    3MF (was: `load_3mf_4color_support_enforcer_has_facets`,
///    `load_3mf_4color_has_mmu_and_support_layers` SupportEnforcer arm, plus
///    the SupportEnforcer arm of the deleted real-3MF multi-color loader test).
///    The parser logic itself remains covered by `load_3mf_extracts_support_facets`
///    against a SYNTHETIC fixture; what is lost is the "does it also work on a real
///    OrcaSlicer-exported file" regression check.
/// 2. Multi-layer paint_data (>= 2 layers) assertion (was: `load_3mf_4color_layer_count_at_least_two`).
///    cube_4color.3mf carries only a Material layer by design; cube_cilindrical_modifier.3mf
///    has no paint at all. Neither can carry a multi-layer assertion.
///
/// Restoration path: author `resources/cube_with_paint_supports.3mf` (a small cube +
/// paint_supports attribute on 1+ faces) and replace this stub with concrete assertions.
/// Tracked under packet 89 §Closure Log AC-N1.
#[test]
#[ignore = "Awaiting cube fixture with paint_supports + multi-layer paint_data; see packet 89 §Closure Log AC-N1"]
fn support_enforcer_and_multi_layer_paint_from_real_3mf_fixture_documented_gap() {
    // Intentionally empty. This stub exists to make the deleted coverage visible in
    // `cargo test --list` output and prevent the gap from disappearing into git history.
}

// ---------------------------------------------------------------------------
// cube_fuzzyPainted.3mf loader tests
// ---------------------------------------------------------------------------

#[test]
fn load_3mf_cube_fuzzy_painted_loads() {
    let path = cube_fuzzy_painted_path();
    let result = load_model(&path);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let mesh_ir = result.unwrap();
    let pd = mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    let has_fuzzy = pd
        .layers
        .iter()
        .any(|l| l.semantic == PaintSemantic::FuzzySkin);
    assert!(has_fuzzy, "expected FuzzySkin layer in cube_fuzzyPainted");
}

#[test]
fn load_3mf_cube_fuzzy_painted_partial_coverage() {
    let path = cube_fuzzy_painted_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let fuzzy = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::FuzzySkin)
        .expect("no FuzzySkin layer");
    let painted: usize = fuzzy
        .facet_values
        .iter()
        .filter(|v| matches!(v, Some(PaintValue::Flag(true))))
        .count();
    let unpainted: usize = fuzzy.facet_values.iter().filter(|v| v.is_none()).count();
    assert_eq!(
        painted, 7,
        "expected 7 facets FuzzySkin Flag(true) (front fully+fuzzy circle+ back half), got {painted}"
    );
    assert_eq!(
        unpainted, 5,
        "expected 5 facets unpainted FuzzySkin (left+back half+bottom), got {unpainted}"
    );
    assert_eq!(painted + unpainted, 12, "all 12 facets accounted for");
}

#[test]
fn load_3mf_cube_fuzzy_painted_facet_count_matches_mesh() {
    let path = cube_fuzzy_painted_path();
    let mesh = load_model(&path).unwrap();
    let tri_count = mesh.objects[0].mesh.indices.len() / 3;
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let fuzzy = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::FuzzySkin)
        .expect("no FuzzySkin layer");
    assert_eq!(
        fuzzy.facet_values.len(),
        tri_count,
        "facet_values length {} must match triangle count {}",
        fuzzy.facet_values.len(),
        tri_count
    );
}

#[test]
fn load_3mf_cube_fuzzy_painted_no_material_layer() {
    let path = cube_fuzzy_painted_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let has_material = pd
        .layers
        .iter()
        .any(|l| l.semantic == PaintSemantic::Material);
    assert!(
        !has_material,
        "cube_fuzzyPainted has no paint_color attributes; Material layer must be absent"
    );
}

#[test]
fn load_3mf_cube_fuzzy_painted_fuzzy_strokes_populated() {
    // Pre-P98: fuzzy_skin sub-facet hex strokes were dropped (hardcoded Vec::new() returned an
    // empty strokes vec). Post-P98 (Step 2): decode_strokes_for_channel is wired for
    // paint_fuzzy_skin, so hex subdivision (circles) now produces PaintStrokes and strokes is
    // non-empty at the loader level. cube_fuzzyPainted.3mf has hex subdivision on some facets.
    let path = cube_fuzzy_painted_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0].paint_data.as_ref().unwrap();
    let fuzzy = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::FuzzySkin)
        .expect("no FuzzySkin layer");
    assert!(
        !fuzzy.strokes.is_empty(),
        "FuzzySkin strokes must be non-empty: Step 2 decodes hex subdivision in cube_fuzzyPainted.3mf"
    );
}

// ---------------------------------------------------------------------------
// Packet 98: paint-channel symmetry — sub-facet stroke tests (Step 3)
//
// Hex "401" encoding (OrcaSlicer nibble-reversed storage):
//   raw hex chars: '4','0','1'
//   parse_nibbles reverses → bytes ['1','0','4'] → nibbles [1, 0, 4]
//   walk:  nibble[0]=1, split_type=1 (non-leaf, 2 children), special_side=0
//          children iterated in REVERSE:
//            nibble[1]=0, split_type=0 (leaf), state_bits=0, state=0 → skip
//            nibble[2]=4, split_type=0 (leaf), state_bits=1, state=1 → push stroke
//   Result: 1 sub-facet stroke with state=1; dominant whole-face state also=1 (≠0).
//   Both conditions for stroke decode pass: hex.len()=3>2, *_state=Some(1)≠0.
// ---------------------------------------------------------------------------

/// AC: paint_color sub-facet strokes are decoded and exposed as PaintStroke{Material, ToolIndex}.
///
/// Uses cube_4color.3mf which has hex subdivision (paint circles) on the Material channel.
#[test]
fn paint_color_subfacet_strokes_decoded() {
    let path = cube_4color_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    let mat = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::Material)
        .expect("Material layer must exist in cube_4color");
    assert!(
        !mat.strokes.is_empty(),
        "Material strokes must be non-empty: cube_4color has hex subdivision on paint_color"
    );
    let has_tool_index_stroke = mat.strokes.iter().any(|s| {
        s.semantic == PaintSemantic::Material && matches!(s.value, PaintValue::ToolIndex(_))
    });
    assert!(
        has_tool_index_stroke,
        "at least one Material stroke must have value=ToolIndex(n); got strokes: {:?}",
        mat.strokes
            .iter()
            .map(|s| (&s.semantic, &s.value))
            .collect::<Vec<_>>()
    );
}

/// AC: paint_supports sub-facet strokes are decoded into SupportEnforcer/SupportBlocker PaintStrokes.
///
/// Hex "401" on paint_supports → state=1 → SupportEnforcer stroke with Flag(true).
/// Triangle vertices chosen to be non-degenerate; midpoint(v1,v2)=(0.5,0.5,0) is also distinct.
#[test]
fn paint_supports_subfacet_strokes_decoded() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    // paint_supports="401": dominant state=1 → SupportEnforcer; sub-facet stroke state=1 → SupportEnforcer+Flag(true)
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_supports="401" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh = load_model(f.path()).expect("should load paint_supports subfacet 3MF");
    let pd = mesh.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    let enforcer = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::SupportEnforcer)
        .expect("SupportEnforcer layer must exist for paint_supports state=1");
    assert!(
        !enforcer.strokes.is_empty(),
        "SupportEnforcer strokes must be non-empty for hex '401'"
    );
    let has_enforcer_stroke = enforcer
        .strokes
        .iter()
        .any(|s| s.semantic == PaintSemantic::SupportEnforcer && s.value == PaintValue::Flag(true));
    assert!(
        has_enforcer_stroke,
        "at least one stroke must have semantic=SupportEnforcer, value=Flag(true)"
    );
}

/// AC: paint_seam sub-facet strokes are decoded into Custom("seam_enforcer")/Custom("seam_blocker") PaintStrokes.
///
/// Hex "401" on paint_seam → state=1 → seam_enforcer stroke with Flag(true).
#[test]
fn paint_seam_subfacet_strokes_decoded() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    // paint_seam="401": dominant state=1 → seam_enforcer; sub-facet stroke state=1 → Custom("seam_enforcer")+Flag(true)
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_seam="401" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh = load_model(f.path()).expect("should load paint_seam subfacet 3MF");
    let pd = mesh.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    let seam = pd
        .layers
        .iter()
        .find(|l| matches!(&l.semantic, PaintSemantic::Custom(s) if s == "seam_enforcer"))
        .expect("seam_enforcer layer must exist for paint_seam state=1");
    assert!(
        !seam.strokes.is_empty(),
        "seam_enforcer strokes must be non-empty for hex '401'"
    );
    let has_seam_stroke = seam.strokes.iter().any(|s| {
        matches!(&s.semantic, PaintSemantic::Custom(n) if n == "seam_enforcer")
            && s.value == PaintValue::Flag(true)
    });
    assert!(
        has_seam_stroke,
        "at least one stroke must have semantic=Custom('seam_enforcer'), value=Flag(true)"
    );
}

/// AC: paint_fuzzy_skin sub-facet strokes are decoded and exposed as PaintStroke{FuzzySkin, Flag(true)}.
///
/// Uses cube_fuzzyPainted.3mf which has hex subdivision (paint circles) on the FuzzySkin channel.
/// Step 2 wired decode_strokes_for_channel for this channel; strokes are no longer hardcoded empty.
#[test]
fn paint_fuzzy_skin_subfacet_strokes_decoded() {
    let path = cube_fuzzy_painted_path();
    let mesh = load_model(&path).unwrap();
    let pd = mesh.objects[0]
        .paint_data
        .as_ref()
        .expect("paint_data must be Some");
    let fuzzy = pd
        .layers
        .iter()
        .find(|l| l.semantic == PaintSemantic::FuzzySkin)
        .expect("FuzzySkin layer must exist in cube_fuzzyPainted");
    assert!(
        !fuzzy.strokes.is_empty(),
        "FuzzySkin strokes must be non-empty: cube_fuzzyPainted has hex subdivision (circles)"
    );
    let has_fuzzy_stroke = fuzzy
        .strokes
        .iter()
        .any(|s| s.semantic == PaintSemantic::FuzzySkin && s.value == PaintValue::Flag(true));
    assert!(
        has_fuzzy_stroke,
        "at least one FuzzySkin stroke must have value=Flag(true)"
    );
}

/// AC: a malformed paint_seam hex returns ModelLoadError (not a panic).
///
/// "ZZZZ" contains invalid hex digits; parse_nibbles propagates PaintMetadata immediately
/// when decode_paint_hex_state is called during attribute parsing.
#[test]
fn paint_seam_malformed_hex_rejected() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_seam="ZZZZ" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let err = load_model(f.path()).unwrap_err();
    assert!(
        matches!(err, ModelLoadError::PaintMetadata { .. }),
        "expected PaintMetadata error for malformed seam hex 'ZZZZ', got: {:?}",
        err
    );
    // Confirm no panic occurred — the assert above already proves it (panics don't return Err).
}

/// AC: an empty paint channel hex produces zero strokes and no error.
///
/// Empty string → decode_paint_hex_state returns Ok(0) → state=0 → no layer created,
/// no stroke decode triggered (hex.len()=0 ≤ 2), paint_data=None.
#[test]
fn paint_channel_empty_hex_noop() {
    let vertices_xml = r#"          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
          <vertex x="0" y="1" z="0" />"#;
    // Empty paint_seam attribute: state=0, no layer, no strokes.
    let triangle_xml = r#"          <triangle v1="0" v2="1" v3="2" paint_seam="" />"#;
    let f = threemf_custom_paint_file(vertices_xml, triangle_xml);
    let mesh = load_model(f.path()).expect("empty paint_seam hex must not cause an error");
    // state=0 → has_any_paint stays false → paint_data=None
    let stroke_count: usize = mesh
        .objects
        .iter()
        .filter_map(|o| o.paint_data.as_ref())
        .flat_map(|pd| pd.layers.iter())
        .map(|l| l.strokes.len())
        .sum();
    assert_eq!(
        stroke_count, 0,
        "empty hex must produce zero strokes across all layers"
    );
}

/// AC: a 3MF with none of the four paint attributes loads OK and has zero paint strokes.
///
/// Uses the synthetic threemf_cube_file() which has no paint_color / paint_supports /
/// paint_seam / paint_fuzzy_skin attributes on any triangle.
#[test]
fn threemf_no_paint_channels_no_strokes() {
    let f = threemf_cube_file();
    let mesh = load_model(f.path()).expect("unpainted 3MF must load without error");
    for obj in &mesh.objects {
        assert!(
            obj.paint_data.is_none(),
            "unpainted 3MF object must have paint_data=None; got: {:?}",
            obj.paint_data.as_ref().map(|pd| pd
                .layers
                .iter()
                .map(|l| &l.semantic)
                .collect::<Vec<_>>())
        );
    }
}
