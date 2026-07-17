//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! top-surface-ironing matches its manifest's declared layer world/stage.
//! (The manifest declares Layer::Infill; the Cargo.toml description mentioning
//! LayerFinalization is stale.)

#![allow(missing_docs)]

use top_surface_ironing::TopSurfaceIroning;

#[test]
fn binding_surface_matches_manifest() {
    assert_eq!(
        TopSurfaceIroning::__slicer_world_id(),
        slicer_schema::WORLD_LAYER
    );
    assert_eq!(TopSurfaceIroning::__slicer_trait_name(), "LayerModule");
    assert_eq!(TopSurfaceIroning::__slicer_stage_name(), "Layer::Infill");
    assert_eq!(
        TopSurfaceIroning::__slicer_stage_export_name(),
        "run-infill"
    );
    let exports = TopSurfaceIroning::__slicer_wit_exports();
    assert!(exports.contains(&"on-print-start"));
    assert!(exports.contains(&"on-print-end"));
    assert!(exports.contains(&"run-infill"));
}
