//! Full-origin identity in the surviving prepared-region dispatch path.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, PaintValue, Point2, Polygon, PreparedRegionData,
    RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_sdk::views::SliceRegionView;
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionIndexInput, LayerSelectionModuleInput, STAGES,
};
use slicer_wasm_host::{binding::LayerStageInput, CompiledModuleLive, WasmInstancePool};

fn chain(tool: u32) -> Vec<(String, PaintValue)> {
    vec![("material".into(), PaintValue::ToolIndex(tool))]
}

fn boundary(x: f32) -> Vec<ExPolygon> {
    vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, 0.0),
                Point2::from_mm(x + 1.0, 0.0),
                Point2::from_mm(x, 1.0),
            ],
        },
        holes: Vec::new(),
    }]
}

fn prepared_views(
    first_chain: &[(String, PaintValue)],
    second_region: u64,
    second_chain: Vec<(String, PaintValue)>,
) -> Vec<SliceRegionView> {
    let slice = SliceIR {
        global_layer_index: 5,
        z: 1.0,
        regions: vec![
            SlicedRegion {
                object_id: "obj-1".into(),
                region_id: 1,
                variant_chain: first_chain.to_vec(),
                ..Default::default()
            },
            SlicedRegion {
                object_id: "obj-1".into(),
                region_id: second_region,
                variant_chain: second_chain,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let prepared = [
        PreparedRegionData {
            prev_layer_boundary: boundary(0.0),
            ..Default::default()
        },
        PreparedRegionData {
            prev_layer_boundary: boundary(10.0),
            ..Default::default()
        },
    ];
    let module_id = "prepared-keying".to_owned();
    let mut region_map = RegionMapIR::default();
    let config = region_map.intern_config(ResolvedConfig {
        sparse_fill_holder: module_id.clone(),
        ..Default::default()
    });
    for region in &slice.regions {
        region_map.entries.insert(
            slicer_wasm_host::selection::region_map_key_for_sliced_region(5, region),
            RegionPlan {
                config,
                ..Default::default()
            },
        );
    }
    let claims = vec!["claim:sparse-fill".to_owned()];
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        Arc::new(ConfigView::from_map(HashMap::new())),
    );
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    // exhaustive: committed selection snapshot supplies every authority channel.
    let index = build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: std::slice::from_ref(&layer),
        // exhaustive: module metadata includes every selection channel.
        modules: &[LayerSelectionModuleInput {
            stage_id: "Layer::Infill",
            module_id: &module_id,
            claims: &claims,
            paint_semantics: None,
            invocation_config: &module.config_view,
        }],
        slices: std::slice::from_ref(&slice),
        region_map: Some(&region_map),
        support_plan: None,
        perimeters: &[],
    })
    .expect("selection");
    // exhaustive: LayerStageInput has no Default; all live inputs are supplied.
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        region_map: Some(Arc::new(region_map)),
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: Some(&prepared),
        prepared_perimeter_source_regions: None,
        infill: None,
        selection: Some(index.selection_for(5, &module_id).expect("selected module")),
    };
    slicer_wasm_host::marshal::native::build_native_layer_request(
        "Layer::Infill",
        5,
        &input,
        &module,
        &HashMap::new(),
    )
    .regions
}

fn lookup<'a>(
    views: &'a [SliceRegionView],
    object: &str,
    region: u64,
    chain: &[(String, PaintValue)],
) -> Option<&'a SliceRegionView> {
    views.iter().find(|view| {
        view.object_id() == object && *view.region_id() == region && view.variant_chain() == chain
    })
}

fn assert_keys(
    first_chain: Vec<(String, PaintValue)>,
    second_region: u64,
    second_chain: Vec<(String, PaintValue)>,
) {
    // Segment transport retired; production dispatch must still associate each
    // prepared payload with its exact object/region/ordered-chain identity.
    let views = prepared_views(&first_chain, second_region, second_chain.clone());
    assert_eq!(views.len(), 2, "neither full-origin bucket may collapse");
    let first = lookup(&views, "obj-1", 1, &first_chain).expect("first key");
    let second = lookup(&views, "obj-1", second_region, &second_chain).expect("second key");
    assert_eq!(first.prev_layer_boundary(), boundary(0.0));
    assert_eq!(second.prev_layer_boundary(), boundary(10.0));
    assert!(lookup(&views, "obj-1", 9, &chain(1)).is_none());
    assert!(lookup(&views, "other-object", 1, &chain(1)).is_none());
    assert!(lookup(&views, "obj-1", 1, &chain(9)).is_none());
    // test-quality: determinism pin for repeated prepared dispatch and reads.
    let repeated = prepared_views(&first_chain, second_region, second_chain.clone());
    for _ in 0..2 {
        assert_eq!(
            lookup(&repeated, "obj-1", 1, &first_chain)
                .unwrap()
                .prev_layer_boundary(),
            first.prev_layer_boundary()
        );
        assert_eq!(
            lookup(&repeated, "obj-1", second_region, &second_chain)
                .unwrap()
                .prev_layer_boundary(),
            second.prev_layer_boundary()
        );
    }
}

#[test]
fn lightning_dispatch_per_region_keying() {
    assert_keys(chain(1), 2, chain(2));
}

#[test]
fn same_number_sources_discriminate_chains() {
    assert_keys(chain(1), 1, chain(2));
}

#[test]
fn same_number_sources_discriminate_ordered_chains() {
    let mut ordered = chain(1);
    ordered.push(("finish".into(), PaintValue::Flag(true)));
    let reversed = ordered.iter().cloned().rev().collect();
    assert_keys(ordered, 1, reversed);
}
