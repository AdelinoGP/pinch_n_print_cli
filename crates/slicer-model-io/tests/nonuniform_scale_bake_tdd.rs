use std::io::{Cursor, Write};

use slicer_model_io::load_model;
use tempfile::NamedTempFile;

fn threemf_scale_file(scale: (f32, f32, f32), paint: bool) -> NamedTempFile {
    let paint_attribute = if paint { r#" paint_color="401""# } else { "" };
    let model_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="1" y="1" z="1" />
          <vertex x="0" y="0" z="0" />
          <vertex x="1" y="0" z="0" />
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"{paint_attribute} />
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="{} 0 0 0 {} 0 0 0 {} 0 0 0" />
  </build>
</model>"#,
        scale.0, scale.1, scale.2
    );

    let mut zip_bytes = Vec::new();
    let cursor = Cursor::new(&mut zip_bytes);
    let mut zip_writer = zip::ZipWriter::new(cursor);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip_writer.start_file("3D/3dmodel.model", options).unwrap();
    zip_writer.write_all(model_xml.as_bytes()).unwrap();
    zip_writer.finish().unwrap();

    let mut file = tempfile::Builder::new().suffix(".3mf").tempfile().unwrap();
    file.write_all(&zip_bytes).unwrap();
    file.flush().unwrap();
    file
}

fn assert_baked_vertex(vertices: &[slicer_ir::Point3], expected: (f32, f32, f32)) {
    assert!(
        vertices.iter().any(|vertex| {
            (vertex.x - expected.0).abs() <= 1e-4
                && (vertex.y - expected.1).abs() <= 1e-4
                && (vertex.z - expected.2).abs() <= 1e-4
        }),
        "baked vertex {:?} not found in {:?}",
        expected,
        vertices
    );
}

fn assert_identity(matrix: &[f64; 16]) {
    let expected = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    for (actual, expected) in matrix.iter().zip(expected) {
        assert!((actual - expected).abs() <= 1e-4);
    }
}

#[test]
fn nonuniform_scale_bakes_vertices_per_axis() {
    let file = threemf_scale_file((1.0, 2.0, 3.0), false);
    let mesh_ir = load_model(file.path());
    assert!(mesh_ir.is_ok());
    let mesh_ir = mesh_ir.unwrap();
    let object = &mesh_ir.objects[0];

    assert_baked_vertex(&object.mesh.vertices, (1.0, 2.0, 3.0));
    assert_identity(&object.transform.matrix);
}

#[test]
fn nonuniform_scale_bakes_paint_triangles() {
    let unscaled_file = threemf_scale_file((1.0, 1.0, 1.0), true);
    let unscaled_mesh_ir = load_model(unscaled_file.path()).unwrap();
    let unscaled_paint_data = unscaled_mesh_ir.objects[0]
        .paint_data
        .as_ref()
        .expect("unscaled paint data");

    let scaled_file = threemf_scale_file((1.0, 2.0, 3.0), true);
    let scaled_mesh_ir = load_model(scaled_file.path()).unwrap();
    let scaled_object = &scaled_mesh_ir.objects[0];
    let scaled_paint_data = scaled_object
        .paint_data
        .as_ref()
        .expect("scaled paint data");

    let unscaled_triangles: Vec<_> = unscaled_paint_data
        .layers
        .iter()
        .flat_map(|layer| layer.strokes.iter())
        .flat_map(|stroke| stroke.triangles.iter())
        .collect();
    let scaled_triangles: Vec<_> = scaled_paint_data
        .layers
        .iter()
        .flat_map(|layer| layer.strokes.iter())
        .flat_map(|stroke| stroke.triangles.iter())
        .collect();
    assert_eq!(unscaled_triangles.len(), scaled_triangles.len());
    assert!(unscaled_triangles
        .iter()
        .flat_map(|triangle| triangle.iter())
        .any(|vertex| {
            (vertex.x - 1.0).abs() <= 1e-4
                && (vertex.y - 1.0).abs() <= 1e-4
                && (vertex.z - 1.0).abs() <= 1e-4
        }));

    for (unscaled, scaled) in unscaled_triangles.iter().zip(&scaled_triangles) {
        for (unscaled_vertex, scaled_vertex) in unscaled.iter().zip(scaled.iter()) {
            assert!(
                (scaled_vertex.x - unscaled_vertex.x).abs() <= 1e-4
                    && (scaled_vertex.y - 2.0 * unscaled_vertex.y).abs() <= 1e-4
                    && (scaled_vertex.z - 3.0 * unscaled_vertex.z).abs() <= 1e-4,
                "paint triangle vertex {scaled_vertex:?} was not baked from {unscaled_vertex:?}"
            );
        }
    }
    assert_identity(&scaled_object.transform.matrix);
}

#[test]
fn uniform_scale_baking_unchanged() {
    let file = threemf_scale_file((2.0, 2.0, 2.0), false);
    let mesh_ir = load_model(file.path()).unwrap();
    let object = &mesh_ir.objects[0];

    assert_baked_vertex(&object.mesh.vertices, (2.0, 2.0, 2.0));
    assert_identity(&object.transform.matrix);
}
