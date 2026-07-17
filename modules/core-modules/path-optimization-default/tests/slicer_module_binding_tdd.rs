//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! path-optimization-default matches its manifest's declared layer world/stage.

#![allow(missing_docs)]

use path_optimization_default::PathOptimizationDefault;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        PathOptimizationDefault::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(
        PathOptimizationDefault::__slicer_trait_name(),
        "LayerModule"
    );
    assert_eq!(
        PathOptimizationDefault::__slicer_stage_name(),
        "Layer::PathOptimization"
    );
    assert_eq!(
        PathOptimizationDefault::__slicer_stage_export_name(),
        "run-path-optimization"
    );
    let exports = PathOptimizationDefault::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-path-optimization"));
}
