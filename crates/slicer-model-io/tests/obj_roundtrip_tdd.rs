//! AC-7 — OBJ load round-trip: 20mm_cube.obj loads to non-empty MeshIR.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn cube_obj_loads_with_nonempty_triangles() {
    let path = workspace_root().join("resources").join("20mm_cube.obj");
    assert!(
        path.exists(),
        "20mm_cube.obj must exist at {}",
        path.display()
    );
    let mesh = slicer_model_io::load_model(&path).expect("20mm_cube.obj must load");
    assert!(
        !mesh.objects.is_empty(),
        "MeshIR must have at least one object"
    );
    assert!(!mesh.objects[0].mesh.indices.is_empty());
    assert_eq!(mesh.objects[0].mesh.indices.len() % 3, 0);
}
