//! Smoke test: cube_cilindrical_modifier.3mf carries per-object AND per-modifier
//! `sparse_infill_density` overrides, and the loader plumbs them into the IR.
//!
//! This is the M3 fixture's contract: a base cube (object id=3) with infill
//! density 0.15 plus a centered cylinder modifier (part id=2) with density 0.40.
//! The two-density delta must reach both `ObjectMesh.config.data` (base) and
//! `ModifierVolume.config_delta.fields` (modifier) so that the per-region config
//! delivery (packet 131) can resolve the two regions to their distinct densities.

use slicer_ir::ConfigValue;
use slicer_model_io::loader::load_model;

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/cube_cilindrical_modifier.3mf")
        .to_path_buf()
}

#[test]
fn cube_cilindrical_modifier_loads_object_and_modifier_with_density_overrides() {
    let path = fixture_path();
    if !path.exists() {
        eprintln!(
            "Skipping: cube_cilindrical_modifier.3mf not found at {:?}",
            path
        );
        return;
    }

    let model = load_model(&path).expect("load cube_cilindrical_modifier.3mf");

    // One object (the cube body).
    assert_eq!(
        model.objects.len(),
        1,
        "expected one object, got {}",
        model.objects.len()
    );
    let obj = &model.objects[0];

    // One modifier volume (the cylinder).
    assert_eq!(
        obj.modifier_volumes.len(),
        1,
        "expected one modifier volume, got {}",
        obj.modifier_volumes.len()
    );
    let modifier = &obj.modifier_volumes[0];

    // Modifier's per-volume density override lands in config_delta.fields.
    let modifier_density = modifier
        .config_delta
        .fields
        .get("sparse_infill_density")
        .unwrap_or_else(|| {
            panic!(
                "modifier missing sparse_infill_density in config_delta.fields; have keys: {:?}",
                modifier.config_delta.fields.keys().collect::<Vec<_>>()
            )
        });
    match modifier_density {
        ConfigValue::Float(v) => {
            assert!(
                (v - 0.40).abs() < 1e-9,
                "modifier sparse_infill_density should be 0.40, got {}",
                v
            );
        }
        other => panic!(
            "modifier sparse_infill_density should be ConfigValue::Float(0.40), got {:?}",
            other
        ),
    }
}

#[test]
fn cube_cilindrical_modifier_object_carries_base_density_override() {
    let path = fixture_path();
    if !path.exists() {
        eprintln!(
            "Skipping: cube_cilindrical_modifier.3mf not found at {:?}",
            path
        );
        return;
    }

    let model = load_model(&path).expect("load cube_cilindrical_modifier.3mf");
    let obj = &model.objects[0];

    // The base object's per-object density override lives in ObjectConfig.data.
    let object_density = obj
        .config
        .data
        .get("sparse_infill_density")
        .unwrap_or_else(|| {
            panic!(
                "base object missing sparse_infill_density in config.data; have keys: {:?}",
                obj.config.data.keys().collect::<Vec<_>>()
            )
        });
    match object_density {
        ConfigValue::Float(v) => {
            assert!(
                (v - 0.15).abs() < 1e-9,
                "base object sparse_infill_density should be 0.15, got {}",
                v
            );
        }
        other => panic!(
            "base object sparse_infill_density should be ConfigValue::Float(0.15), got {:?}",
            other
        ),
    }
}
