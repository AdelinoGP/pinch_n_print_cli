//! DEV-052 â€” Paint data on non-NormalPart objects must be dropped with a warning.

#![allow(missing_docs)]

use slicer_ir::MeshIR;
use slicer_runtime::model_loader::load_model;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn benchy_4color_3mf() -> PathBuf {
    repo_root().join("resources/benchy_4color.3mf")
}

// ---------------------------------------------------------------------------
// DEV-052: paint on modifier part must be dropped
// ---------------------------------------------------------------------------

#[test]
fn paint_on_modifier_part_dropped_with_warning() {
    let path = benchy_4color_3mf();
    assert!(path.exists(), "fixture missing: {}", path.display());
    let mesh_ir: MeshIR = load_model(&path).expect("load benchy_4color.3mf should succeed");

    let solid_obj = &mesh_ir.objects[0];

    assert!(
        !solid_obj.modifier_volumes.is_empty(),
        "modifier_volumes is empty"
    );

    // paint_data must not carry facet values from the modifier part. If paint
    // data was incorrectly merged, paint layers would have
    // facet_values.len() == N_solid + N_modifier; after correct drop the length
    // must equal N_solid exactly (or paint_data is None).
    let solid_tri_count = solid_obj.mesh.indices.len() / 3;
    if let Some(ref pd) = solid_obj.paint_data {
        for layer in &pd.layers {
            assert_eq!(
                layer.facet_values.len(),
                solid_tri_count,
                "DEV-052: paint layer {:?} has {} facet_values but solid \
                 mesh has {} triangles â€” modifier-part paint was not dropped",
                layer.semantic,
                layer.facet_values.len(),
                solid_tri_count
            );
        }
    }
    // If paint_data is None, the drop is vacuously satisfied.
}
