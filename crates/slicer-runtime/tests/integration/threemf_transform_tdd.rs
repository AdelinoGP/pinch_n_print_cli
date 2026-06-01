#![allow(missing_docs)]
#![allow(unused_imports)]

use slicer_ir::{IndexedTriangleSet, MeshIR, ObjectMesh, Point3};
use slicer_model_io::{load_model, ModelLoadError};
use std::io::Write;
use zip::write::SimpleFileOptions;

fn create_3mf(xml: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new().suffix(".3mf").tempfile().unwrap();
    let mut zip = zip::ZipWriter::new(&mut file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("3D/3dmodel.model", options).unwrap();
    zip.write_all(xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    file
}

#[test]
fn translation_transform_shifts_vertices() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="10" y="0" z="0"/>
          <vertex x="0" y="10" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="1 0 0 0 1 0 0 0 1 125 105 0"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();

    assert_eq!(mesh_ir.objects.len(), 1);
    let object: &ObjectMesh = &mesh_ir.objects[0];
    assert_eq!(object.mesh.vertices.len(), 3);

    let cx =
        (object.mesh.vertices[0].x + object.mesh.vertices[1].x + object.mesh.vertices[2].x) / 3.0;
    let cy =
        (object.mesh.vertices[0].y + object.mesh.vertices[1].y + object.mesh.vertices[2].y) / 3.0;

    assert!((cx - 128.33).abs() < 0.5, "Expected x~128.33, got {}", cx);
    assert!((cy - 108.33).abs() < 0.5, "Expected y~108.33, got {}", cy);
}

#[test]
fn z_rotation_transform_rotates_mesh() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="20" y="0" z="0"/>
          <vertex x="0" y="10" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="0 -1 0 1 0 0 0 0 1 0 0 0"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();

    // 3MF row-vector convention: floats `0 -1 0 1 0 0 0 0 1 0 0 0` build
    // M_row[0]=(0,-1,0), M_row[1]=(1,0,0), M_row[2]=(0,0,1). Applying
    // v_new = v Â· M_row:
    //   (20, 0, 0) â†’ ( 0, -20, 0)
    //   (0, 10, 0) â†’ (10,   0, 0)
    //   (0,  0, 0) â†’ ( 0,   0, 0)
    // This is a Z-rotation by -90Â° (clockwise in XY viewed from +Z).
    let verts = &mesh_ir.objects[0].mesh.vertices;
    let mut have_origin = false;
    let mut have_x20 = false;
    let mut have_y10 = false;
    for v in verts {
        if v.x.abs() < 0.01 && v.y.abs() < 0.01 {
            have_origin = true;
        }
        if v.x.abs() < 0.01 && (v.y + 20.0).abs() < 0.01 {
            have_x20 = true;
        }
        if (v.x - 10.0).abs() < 0.01 && v.y.abs() < 0.01 {
            have_y10 = true;
        }
    }
    assert!(
        have_origin && have_x20 && have_y10,
        "Z-rotation by -90Â° must send (20,0,0)â†’(0,-20,0) and (0,10,0)â†’(10,0,0); got {:?}",
        verts
    );
}

#[test]
fn y_rotation_45deg_orients_correctly() {
    // benchy_4color.3mf's component-1 transform is a Y-rotation by -45Â°
    // (3MF row-vector: `0.7071 0 0.7071 0 1 0 -0.7071 0 0.7071 0 0 0`).
    // Test with a Y-rotation by +45Â° applied at <build><item> so the
    // expected mapping is unambiguous and not coupled to component
    // resolution: in row-vector convention `0.7071 0 -0.7071 0 1 0 0.7071 0 0.7071 0 0 0`
    // sends (1, 0, 0) â†’ (0.7071, 0, -0.7071) and (0, 0, 1) â†’ (0.7071, 0, 0.7071).
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="1" y="0" z="0"/>
          <vertex x="0" y="0" z="1"/>
          <vertex x="0" y="0" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="0.7071067811865476 0 -0.7071067811865476 0 1 0 0.7071067811865476 0 0.7071067811865476 0 0 0"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();
    let verts = &mesh_ir.objects[0].mesh.vertices;
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let mut have_x_image = false;
    let mut have_z_image = false;
    for v in verts {
        if (v.x - s).abs() < 0.001 && v.y.abs() < 0.001 && (v.z + s).abs() < 0.001 {
            have_x_image = true;
        }
        if (v.x - s).abs() < 0.001 && v.y.abs() < 0.001 && (v.z - s).abs() < 0.001 {
            have_z_image = true;
        }
    }
    assert!(
        have_x_image,
        "Y+45Â° must send (1,0,0) â†’ (1/âˆš2, 0, -1/âˆš2); got {:?}",
        verts
    );
    assert!(
        have_z_image,
        "Y+45Â° must send (0,0,1) â†’ (1/âˆš2, 0, 1/âˆš2); got {:?}",
        verts
    );
}

#[test]
fn benchy_painted_transform_applies_z_translation() {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../resources/benchy_painted.3mf");

    let mesh_ir: MeshIR = load_model(&path).expect("Failed to load benchy_painted.3mf");
    assert!(!mesh_ir.objects.is_empty(), "No objects loaded");

    let mut max_z = f32::MIN;
    for object in &mesh_ir.objects {
        for v in &object.mesh.vertices {
            max_z = max_z.max(v.z);
        }
    }

    assert!(
        (max_z - 48.0).abs() < 2.0,
        "Expected max_z ~48.0, got {}",
        max_z
    );
}

#[test]
fn multi_instance_build_items() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="10" y="0" z="0"/>
          <vertex x="0" y="10" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="1 0 0 0 1 0 0 0 1 10 10 0"/>
    <item objectid="1" transform="1 0 0 0 1 0 0 0 1 20 20 0"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();

    assert_eq!(
        mesh_ir.objects.len(),
        2,
        "Expected 2 objects for multi-instance build items"
    );

    let obj1: &ObjectMesh = &mesh_ir.objects[0];
    let obj2: &ObjectMesh = &mesh_ir.objects[1];

    let cx1 = (obj1.mesh.vertices[0].x + obj1.mesh.vertices[1].x + obj1.mesh.vertices[2].x) / 3.0;
    let cx2 = (obj2.mesh.vertices[0].x + obj2.mesh.vertices[1].x + obj2.mesh.vertices[2].x) / 3.0;

    assert!(
        (cx1 - cx2).abs() > 5.0,
        "Centroids should differ between instances (cx1: {}, cx2: {})",
        cx1,
        cx2
    );
}

#[test]
fn malformed_transform_returns_parse_error() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="10" y="0" z="0"/>
          <vertex x="0" y="10" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="1 0 0 0 1 0 0 0 1 5"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let result = load_model(file.path());

    assert!(
        matches!(result, Err(ModelLoadError::ThreeMfParse(_))),
        "Expected ModelLoadError::ThreeMfParse, got {:?}",
        result
    );
}

#[test]
fn mirror_transform_produces_mirrored_geometry() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="5" y="0" z="0"/>
          <vertex x="10" y="0" z="0"/>
          <vertex x="5" y="10" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="-1 0 0 0 1 0 0 0 1 0 0 0"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR =
        load_model(file.path()).expect("mirror transform must load (DEV-046 closure)");
    let object: &ObjectMesh = &mesh_ir.objects[0];
    let mut max_x = f32::MIN;
    for v in &object.mesh.vertices {
        max_x = max_x.max(v.x);
    }
    assert!(
        max_x < 0.0,
        "Geometry should have negative X values due to mirroring, max_x: {}",
        max_x
    );
}

#[test]
fn nested_component_transform_composes_with_item_transform() {
    // Object 2 wraps object 1 via <components> with its own +5/+5 translation;
    // <build><item> adds another +10/+0. Composed effect on a vertex at (0,0,0)
    // should be (15, 5, 0).
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="1" y="0" z="0"/>
          <vertex x="0" y="1" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
    <object id="2" type="model">
      <components>
        <component objectid="1" transform="1 0 0 0 1 0 0 0 1 5 5 0"/>
      </components>
    </object>
  </resources>
  <build>
    <item objectid="2" transform="1 0 0 0 1 0 0 0 1 10 0 0"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();
    assert_eq!(mesh_ir.objects.len(), 1);
    let v0 = mesh_ir.objects[0].mesh.vertices[0];
    assert!(
        (v0.x - 15.0).abs() < 0.001 && (v0.y - 5.0).abs() < 0.001 && v0.z.abs() < 0.001,
        "Composed component+item transform expected vertex (15, 5, 0), got ({}, {}, {})",
        v0.x,
        v0.y,
        v0.z
    );
}

#[test]
fn component_merge_pads_paint_layer_for_unpainted_sibling() {
    // Object 1: 2 triangles, with paint_color on both.
    // Object 2: 1 triangle, no paint at all.
    // Object 3: <components> wrapping both. Merged paint must have
    // facet_values length == 3 (with None for object 2's triangle),
    // not 2. This is the bug surfaced by benchy_4color.3mf.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="1" y="0" z="0"/>
          <vertex x="0" y="1" z="0"/>
          <vertex x="1" y="1" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2" paint_color="4"/>
          <triangle v1="1" v2="3" v3="2" paint_color="8"/>
        </triangles>
      </mesh>
    </object>
    <object id="2" type="model">
      <mesh>
        <vertices>
          <vertex x="10" y="0" z="0"/>
          <vertex x="11" y="0" z="0"/>
          <vertex x="10" y="1" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2"/>
        </triangles>
      </mesh>
    </object>
    <object id="3" type="model">
      <components>
        <component objectid="1"/>
        <component objectid="2"/>
      </components>
    </object>
  </resources>
  <build>
    <item objectid="3"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();
    let object: &ObjectMesh = &mesh_ir.objects[0];
    let tri_count = object.mesh.indices.len() / 3;
    assert_eq!(tri_count, 3, "expected 3 triangles after component merge");
    let paint = object
        .paint_data
        .as_ref()
        .expect("merged paint data present");
    for layer in &paint.layers {
        assert_eq!(
            layer.facet_values.len(),
            tri_count,
            "paint layer {:?} length {} != triangle count {}",
            layer.semantic,
            layer.facet_values.len(),
            tri_count,
        );
    }
}

#[test]
fn paint_stroke_vertices_move_with_item_transform() {
    // A subdivided paint_color stroke (hex length > 2) produces a PaintStroke
    // with explicit triangle vertices baked from the parent triangle. Those
    // vertices must be transformed alongside the mesh.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0"/>
          <vertex x="10" y="0" z="0"/>
          <vertex x="0" y="10" z="0"/>
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2" paint_color="1C04"/>
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" transform="1 0 0 0 1 0 0 0 1 100 200 50"/>
  </build>
</model>"#;

    let file = create_3mf(xml);
    let mesh_ir: MeshIR = load_model(file.path()).unwrap();
    let object: &ObjectMesh = &mesh_ir.objects[0];
    let paint = object
        .paint_data
        .as_ref()
        .expect("paint_color stroke should produce FacetPaintData");
    let material_layer = paint
        .layers
        .iter()
        .find(|l| matches!(l.semantic, slicer_ir::PaintSemantic::Material))
        .expect("Material layer present");
    assert!(
        !material_layer.strokes.is_empty(),
        "Subdivided paint_color should produce at least one stroke"
    );
    for stroke in &material_layer.strokes {
        for tri in &stroke.triangles {
            for v in tri {
                assert!(
                    v.x >= 99.0 && v.y >= 199.0 && (v.z - 50.0).abs() < 0.001,
                    "Stroke vertex not transformed: ({}, {}, {})",
                    v.x,
                    v.y,
                    v.z
                );
            }
        }
    }
}
