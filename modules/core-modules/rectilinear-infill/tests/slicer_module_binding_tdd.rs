//! TASK-111 regression guard: rectilinear-infill has adopted
//! `#[slicer_module]` and the macro-emitted binding surface matches
//! the manifest's declared stage/world.

use rectilinear_infill::RectilinearInfill;

#[test]
fn binding_surface_matches_documented_layer_infill_stage() {
    assert_eq!(
        RectilinearInfill::__slicer_world_id(),
        "slicer:world-layer@1.0.0"
    );
    assert_eq!(RectilinearInfill::__slicer_trait_name(), "LayerModule");
    assert_eq!(
        RectilinearInfill::__slicer_stage_name(),
        "Layer::Infill"
    );
    assert_eq!(
        RectilinearInfill::__slicer_stage_export_name(),
        "run-infill"
    );
    let exports = RectilinearInfill::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-infill"));
}

#[test]
fn binding_schema_json_includes_stage_and_world() {
    let json = RectilinearInfill::__slicer_binding_schema_json();
    assert!(json.contains(r#""world":"slicer:world-layer@1.0.0""#));
    assert!(json.contains(r#""stage_id":"Layer::Infill""#));
    assert!(json.contains(r#""trait":"LayerModule""#));
}
