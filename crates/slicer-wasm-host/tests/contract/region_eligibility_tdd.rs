//! Contract coverage for native and WASM support-eligibility marshalling.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, ObjectSurfaceData, OverhangRegion, Point2, Polygon, RegionMapIR,
    RegionPlan, ResolvedConfig, SliceIR, SlicedRegion, SurfaceClassificationIR,
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
        region_map: None,
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: Some(&classification),
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
        selection: None,
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

#[test]
fn preparation_requires_invocation_and_nonempty_selected_targets() {
    use slicer_wasm_host::selection::{
        build_layer_selection_index, LayerSelectionIndexInput, LayerSelectionModuleInput, STAGES,
    };

    let layer = slicer_ir::GlobalLayer {
        index: 3,
        z: 0.4,
        ..Default::default()
    };
    let config = ConfigView::from_map(HashMap::new());
    for (regions, eligible) in [
        (Vec::new(), false),
        (
            vec![SlicedRegion {
                object_id: "model".into(),
                region_id: 1,
                ..Default::default()
            }],
            true,
        ),
    ] {
        let slice = SliceIR {
            global_layer_index: 3,
            z: 0.4,
            regions,
            ..Default::default()
        };
        let mut region_map = RegionMapIR::default();
        let config_id = region_map.intern_config(ResolvedConfig::default());
        for region in &slice.regions {
            region_map.entries.insert(
                slicer_wasm_host::selection::region_map_key_for_sliced_region(3, region),
                RegionPlan {
                    config: config_id,
                    ..Default::default()
                },
            );
        }
        // exhaustive: selection fixture supplies every committed authority channel.
        let index = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: std::slice::from_ref(&layer),
            // exhaustive: selection fixture supplies all static module metadata.
            modules: &[LayerSelectionModuleInput {
                stage_id: "Layer::Infill",
                module_id: "eligibility",
                claims: &[],
                paint_semantics: None,
                invocation_config: &config,
            }],
            slices: std::slice::from_ref(&slice),
            region_map: Some(&region_map),
            support_plan: None,
            perimeters: &[],
        })
        .expect("committed selection");
        let selection = index
            .selection_for(3, "eligibility")
            .expect("registered module");
        assert_eq!(selection.invoke && !selection.targets.is_empty(), eligible);
    }
}
