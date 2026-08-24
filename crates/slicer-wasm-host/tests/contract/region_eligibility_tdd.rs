//! Contract coverage for native and WASM support-eligibility marshalling.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, ObjectSurfaceData, OverhangRegion, Point2, Polygon, SliceIR,
    SlicedRegion, SurfaceClassificationIR,
};
use slicer_wasm_host::{binding::LayerStageInput, CompiledModuleLive, WasmInstancePool};

fn square(x: f32, y: f32, size: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, y),
                Point2::from_mm(x + size, y),
                Point2::from_mm(x + size, y + size),
                Point2::from_mm(x, y + size),
            ],
        },
        holes: Vec::new(),
    }
}

fn classification(footprint: ExPolygon) -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        per_object: HashMap::from([(
            "support-object".to_owned(),
            ObjectSurfaceData {
                overhang_regions: vec![OverhangRegion {
                    xy_footprint: vec![footprint],
                    ..Default::default()
                }],
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn marshal_needs_support(overhang: ExPolygon) -> (bool, bool) {
    let region = SlicedRegion {
        object_id: "support-object".to_owned(),
        region_id: 1,
        polygons: vec![square(0.0, 0.0, 1.0)],
        ..Default::default()
    };
    let classification = classification(overhang);
    let slice = SliceIR {
        global_layer_index: 3,
        z: 0.4,
        regions: vec![region.clone()],
        ..Default::default()
    };
    let module_id = "region-eligibility".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &[],
        Arc::new(ConfigView::from_map(HashMap::new())),
    );
    // exhaustive: LayerStageInput has no Default and this test supplies the complete stage input
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: None,
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: Some(&classification),
        infill: None,
    };
    let native = slicer_wasm_host::marshal::native::build_native_layer_request(
        "Layer::Infill",
        3,
        &input,
        &module,
        &HashMap::new(),
    );
    let wasm = slicer_wasm_host::host::sliced_region_to_data(
        &region,
        slice.z,
        Vec::new(),
        Some(&classification),
        slice.global_layer_index,
    );
    (native.regions[0].needs_support(), wasm.needs_support)
}

#[test]
fn disjoint_overhang_footprint_is_ineligible_on_native_and_wasm_legs() {
    let (native, wasm) = marshal_needs_support(square(2.0, 0.0, 1.0));
    assert!(
        !native,
        "native leg must derive disjoint support eligibility"
    );
    assert!(!wasm, "WASM leg must derive disjoint support eligibility");
}

#[test]
fn overlapping_overhang_footprint_is_eligible_on_native_and_wasm_legs() {
    let (native, wasm) = marshal_needs_support(square(0.5, 0.0, 1.0));
    assert!(
        native,
        "native leg must derive overlapping support eligibility"
    );
    assert!(wasm, "WASM leg must derive overlapping support eligibility");
}
