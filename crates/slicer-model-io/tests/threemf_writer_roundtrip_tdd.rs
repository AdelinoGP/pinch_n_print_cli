//! AC-7 — 3MF writer round-trip: write_3mf -> load_model preserves triangle count.

use slicer_ir::{IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, Transform3d};

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, // col 0
            0.0, 1.0, 0.0, 0.0, // col 1
            0.0, 0.0, 1.0, 0.0, // col 2
            0.0, 0.0, 0.0, 1.0, // col 3
        ],
    }
}

fn unit_triangle_mesh() -> MeshIR {
    let mesh = IndexedTriangleSet {
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
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
        ],
        indices: vec![0, 1, 2],
    };
    let obj = ObjectMesh {
        id: "tri".to_string(),
        mesh,
        transform: identity_transform(),
        config: ObjectConfig::default(),
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: Some((0.0, 0.0)),
    };
    MeshIR {
        objects: vec![obj],
        ..Default::default()
    }
}

#[test]
fn threemf_writer_roundtrip_preserves_triangle_count() {
    let original = unit_triangle_mesh();
    let original_triangles: usize = original
        .objects
        .iter()
        .map(|o| o.mesh.indices.len() / 3)
        .sum();
    assert_eq!(original_triangles, 1);

    // Write to a temp file.
    let tmpdir = std::env::temp_dir();
    let path = tmpdir.join("slicer_model_io_p81_roundtrip.3mf");
    {
        let file = std::fs::File::create(&path).expect("create 3mf temp");
        slicer_model_io::write_3mf(&original, file).expect("write_3mf must succeed");
    }

    // Reload via load_model.
    let reloaded = slicer_model_io::load_model(&path).expect("3mf reload must succeed");
    let reloaded_triangles: usize = reloaded
        .objects
        .iter()
        .map(|o| o.mesh.indices.len() / 3)
        .sum();

    assert_eq!(
        reloaded_triangles, original_triangles,
        "triangle count must round-trip; got {} expected {}",
        reloaded_triangles, original_triangles
    );

    let _ = std::fs::remove_file(&path);
}
