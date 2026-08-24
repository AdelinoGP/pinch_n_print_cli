//! Regression: native and WASM layer projections must remain field-identical.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon, SliceIR, SlicedRegion, SupportPlanIR};
use slicer_sdk::traits::PaintRegionLayerView;
use slicer_sdk::views::{PerimeterRegionView, SliceRegionView};
use slicer_wasm_host::{binding::LayerStageInput, CompiledModuleLive, WasmInstancePool};

#[test]
fn native_and_wasm_layer_views_are_field_identical() {
    let region = SlicedRegion {
        object_id: "identity-object".to_owned(),
        region_id: 4,
        polygons: vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: 100, y: 0 },
                    Point2 { x: 100, y: 100 },
                ],
            },
            holes: Vec::new(),
        }],
        ..Default::default()
    };
    let slice = SliceIR {
        global_layer_index: 3,
        z: 0.4,
        regions: vec![region.clone()],
        ..Default::default()
    };
    let config = Arc::new(ConfigView::from_map(HashMap::new()));
    let claims = Vec::<String>::new();
    let module_id = "view-identity".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        config,
    );
    // exhaustive: projection identity test pins every input field
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
        surface_classification: None,
        infill: None,
    };
    let native = slicer_wasm_host::marshal::native::build_native_layer_request(
        "Layer::Infill",
        3,
        &input,
        &module,
        &HashMap::new(),
    );

    // This is the projection performed by the WASM dispatch leg before the
    // guest call. Keep comparisons separate so a skew identifies its field.
    let mut wasm_regions = vec![SliceRegionView::from_ir(&region, slice.z, Vec::new())];
    for view in &mut wasm_regions {
        view.set_config((*module.config_view).clone());
    }
    let wasm_perimeter = Some(Vec::<PerimeterRegionView>::new());
    let wasm_paint =
        PaintRegionLayerView::new(3).with_support_plan(Arc::new(SupportPlanIR::default()));

    assert_eq!(native.layer_index, 3, "layer_index");
    assert_eq!(native.regions.len(), wasm_regions.len(), "regions.len");
    for (native, wasm) in native.regions.iter().zip(&wasm_regions) {
        assert_eq!(native.object_id(), wasm.object_id(), "region.object_id");
        assert_eq!(native.region_id(), wasm.region_id(), "region.region_id");
        assert_eq!(native.polygons(), wasm.polygons(), "region.polygons");
        assert_eq!(
            native.infill_areas(),
            wasm.infill_areas(),
            "region.infill_areas"
        );
        assert_eq!(
            native.effective_layer_height(),
            wasm.effective_layer_height(),
            "region.effective_layer_height"
        );
        assert_eq!(native.z(), wasm.z(), "region.z");
        assert_eq!(
            native.has_nonplanar(),
            wasm.has_nonplanar(),
            "region.has_nonplanar"
        );
        assert_eq!(
            native.segment_annotations(),
            wasm.segment_annotations(),
            "region.segment_annotations"
        );
        assert_eq!(
            native.variant_chain(),
            wasm.variant_chain(),
            "region.variant_chain"
        );
        assert_eq!(
            native.needs_support(),
            wasm.needs_support(),
            "region.needs_support"
        );
        assert_eq!(
            native.top_shell_index(),
            wasm.top_shell_index(),
            "region.top_shell_index"
        );
        assert_eq!(
            native.bottom_shell_index(),
            wasm.bottom_shell_index(),
            "region.bottom_shell_index"
        );
        assert_eq!(
            native.top_solid_fill(),
            wasm.top_solid_fill(),
            "region.top_solid_fill"
        );
        assert_eq!(
            native.bottom_solid_fill(),
            wasm.bottom_solid_fill(),
            "region.bottom_solid_fill"
        );
        assert_eq!(native.is_bridge(), wasm.is_bridge(), "region.is_bridge");
        assert_eq!(
            native.bridge_areas(),
            wasm.bridge_areas(),
            "region.bridge_areas"
        );
        assert_eq!(
            native.bridge_orientation_deg(),
            wasm.bridge_orientation_deg(),
            "region.bridge_orientation_deg"
        );
        assert_eq!(
            native.sparse_infill_area(),
            wasm.sparse_infill_area(),
            "region.sparse_infill_area"
        );
        assert_eq!(
            native.held_claims(),
            wasm.held_claims(),
            "region.held_claims"
        );
        assert_eq!(
            native.overhang_areas(),
            wasm.overhang_areas(),
            "region.overhang_areas"
        );
        assert_eq!(
            native.overhang_quartile_polygons(),
            wasm.overhang_quartile_polygons(),
            "region.overhang_quartile_polygons"
        );
        assert_eq!(
            native.prev_layer_boundary(),
            wasm.prev_layer_boundary(),
            "region.prev_layer_boundary"
        );
        assert_eq!(
            native.surface_group(),
            wasm.surface_group(),
            "region.surface_group"
        );
        assert_eq!(native.config(), wasm.config(), "region.config");
    }
    assert_eq!(
        native.perimeter_regions.as_ref().map(Vec::len),
        wasm_perimeter.as_ref().map(Vec::len),
        "perimeter_regions.len"
    );
    assert_eq!(
        native.paint.as_ref().map(PaintRegionLayerView::layer_index),
        Some(wasm_paint.layer_index()),
        "paint.layer_index"
    );
    assert_eq!(
        native
            .paint
            .as_ref()
            .and_then(PaintRegionLayerView::slice_ir),
        wasm_paint.slice_ir(),
        "paint.slice_ir"
    );
    assert_eq!(
        native
            .paint
            .as_ref()
            .and_then(PaintRegionLayerView::support_plan),
        wasm_paint.support_plan(),
        "paint.support_plan"
    );
    assert_eq!(
        native
            .paint
            .as_ref()
            .and_then(PaintRegionLayerView::lightning_tree_ir),
        wasm_paint.lightning_tree_ir(),
        "paint.lightning_tree_ir"
    );
    assert_eq!(native.prior_infill, None, "prior_infill");
    assert_eq!(native.config, *module.config_view, "config");
    assert_eq!(native.stage_export, "Layer::Infill", "stage_export");
}
