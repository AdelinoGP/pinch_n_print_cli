//! AC-7 — STL load round-trip: regression_wedge.stl loads to non-empty MeshIR.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

#[test]
fn benchy_stl_loads_with_nonempty_triangles() {
    let path = workspace_root()
        .join("resources")
        .join("regression_wedge.stl");
    assert!(
        path.exists(),
        "regression_wedge.stl must exist at {}",
        path.display()
    );
    let mesh = slicer_model_io::load_model(&path).expect("regression_wedge.stl must load");
    assert!(
        !mesh.objects.is_empty(),
        "MeshIR must have at least one object"
    );
    assert!(
        !mesh.objects[0].mesh.indices.is_empty(),
        "indices.len() must be > 0 (got {})",
        mesh.objects[0].mesh.indices.len()
    );
    // Triangle count is indices/3; sanity check it's a positive integer multiple of 3.
    assert_eq!(mesh.objects[0].mesh.indices.len() % 3, 0);
}
