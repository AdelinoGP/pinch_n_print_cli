use slicer_ir::{IndexedTriangleSet, ObjectConfig, ObjectMesh, Transform3d};

pub fn object_mesh_base() -> ObjectMesh {
    // exhaustive: file-shared FRU base — a new ObjectMesh field must be routed here deliberately
    ObjectMesh {
        id: "base-object".to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        },
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, // col 0
                0.0, 1.0, 0.0, 0.0, // col 1
                0.0, 0.0, 1.0, 0.0, // col 2
                0.0, 0.0, 0.0, 1.0, // col 3
            ],
        },
        config: ObjectConfig::default(),
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: None,
    }
}
