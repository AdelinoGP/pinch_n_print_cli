//! Regression: native and WASM layer projections must remain field-identical.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, ConfigValue, ConfigView, ExPolygon, GlobalLayer, ObjectSurfaceData,
    OverhangRegion, PerimeterIR, PerimeterRegion, Point2, Polygon, QuartileBand, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion, SupportPlanIR,
    SurfaceClassificationIR,
};
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
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
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

#[test]
fn native_projection_filters_and_resolves_configured_regions() {
    // This exercises native construction and the WASM region-data converter,
    // not a running guest or HostExecutionContext's config accessor. Full
    // native/WASM output equivalence needs a separate end-to-end check.
    let polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(1.0, 0.0),
                Point2::from_mm(1.0, 1.0),
                Point2::from_mm(0.0, 1.0),
            ],
        },
        holes: Vec::new(),
    };
    let selected = SlicedRegion {
        object_id: "selected".into(),
        region_id: 4,
        polygons: vec![polygon.clone()],
        ..Default::default()
    };
    let excluded = SlicedRegion {
        object_id: "excluded".into(),
        region_id: 5,
        polygons: vec![polygon.clone()],
        ..Default::default()
    };
    let slice = SliceIR {
        global_layer_index: 3,
        z: 0.4,
        regions: vec![selected.clone(), excluded],
        ..Default::default()
    };
    let perimeter = PerimeterIR {
        global_layer_index: 3,
        regions: vec![
            PerimeterRegion {
                object_id: "selected".into(),
                region_id: 4,
                ..Default::default()
            },
            PerimeterRegion {
                object_id: "perimeter-only".into(),
                region_id: 6,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let layer = GlobalLayer {
        index: 3,
        active_regions: vec![ActiveRegion {
            object_id: "selected".into(),
            region_id: 4,
            resolved_config: ResolvedConfig {
                extensions: BTreeMap::from([(
                    "support_family".to_owned(),
                    ConfigValue::String("tree".to_owned()),
                )]),
                ..Default::default()
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut map = RegionMapIR::default();
    let override_config = ResolvedConfig {
        extensions: BTreeMap::from([("infill_density".to_owned(), ConfigValue::Float(0.7))]),
        ..Default::default()
    };
    let config_id = map.intern_config(override_config);
    map.entries.insert(
        RegionKey {
            global_layer_index: 3,
            object_id: "selected".into(),
            region_id: 4,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: config_id,
            ..Default::default()
        },
    );
    map.entries.insert(
        RegionKey {
            global_layer_index: 3,
            object_id: "perimeter-only".into(),
            region_id: 6,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: config_id,
            ..Default::default()
        },
    );
    let classification = SurfaceClassificationIR {
        per_object: HashMap::from([(
            "selected".to_owned(),
            ObjectSurfaceData {
                overhang_regions: vec![OverhangRegion {
                    xy_footprint: vec![polygon.clone()],
                    ..Default::default()
                }],
                ..Default::default()
            },
        )]),
        overhang_quartile_polygons: HashMap::from([(
            "selected".to_owned(),
            HashMap::from([(
                3,
                vec![QuartileBand {
                    quartile: 1,
                    polygons: vec![polygon.clone()],
                }],
            )]),
        )]),
        prev_layer_boundaries: HashMap::from([(
            "selected".to_owned(),
            HashMap::from([(3, vec![polygon])]),
        )]),
        ..Default::default()
    };
    let claims = vec!["support-family:tree".to_owned()];
    let module_id = "view-identity".to_owned();
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        Arc::new(ConfigView::from_map(HashMap::from([(
            "infill_density".to_owned(),
            ConfigValue::Float(0.2),
        )]))),
    );
    let held = vec!["support-family:tree".to_owned()];
    let claims_map = HashMap::from([(("selected".to_owned(), "4".to_owned()), held.clone())]);
    // exhaustive: the seam test supplies the complete stage input
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: Some(Arc::new(map)),
        slice: Some(&slice),
        perimeter: Some(&perimeter),
        layer_collection: None,
        surface_classification: Some(&classification),
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let native = slicer_wasm_host::marshal::native::build_native_layer_request_for_layer(
        "Layer::Infill",
        &layer,
        &input,
        &module,
        &claims_map,
    );
    let wasm = slicer_wasm_host::host::sliced_region_to_data(
        &selected,
        slice.z,
        held.clone(),
        Some(&classification),
        slice.global_layer_index,
    );

    assert_eq!(
        native.regions.len(),
        1,
        "excluded region must not reach native"
    );
    let region = &native.regions[0];
    assert_eq!(region.object_id(), &wasm.object_id);
    assert_eq!(region.region_id().to_string(), wasm.region_id);
    assert_eq!(region.held_claims(), held.as_slice());
    assert_eq!(region.held_claims(), wasm.held_claims);
    assert!(region.needs_support(), "classification must be nonempty");
    assert_eq!(region.needs_support(), wasm.needs_support);
    assert!(
        !wasm.overhang_areas.is_empty(),
        "WASM classification must be exercised"
    );
    assert!(
        !region.overhang_areas().is_empty(),
        "native classification must be exercised"
    );
    assert_eq!(region.overhang_areas().len(), wasm.overhang_areas.len());
    assert!(!wasm.prev_layer_boundary.is_empty());
    assert!(!region.prev_layer_boundary().is_empty());
    assert_eq!(
        region.prev_layer_boundary().len(),
        wasm.prev_layer_boundary.len()
    );
    assert_eq!(
        region
            .config()
            .and_then(|cfg| cfg.get_float("infill_density")),
        Some(0.7)
    );
    assert_ne!(region.config(), Some(module.config_view.as_ref()));
    let perimeter_region = &native.perimeter_regions.as_ref().unwrap()[0];
    assert_eq!(
        perimeter_region
            .config()
            .and_then(|cfg| cfg.get_float("infill_density")),
        Some(0.7)
    );
    assert_eq!(
        native.perimeter_regions.as_ref().unwrap()[1]
            .config()
            .and_then(|cfg| cfg.get_float("infill_density")),
        Some(0.2),
        "WASM has no SliceIR-derived override for a perimeter-only identity"
    );
}

#[test]
fn prepared_region_projection_matches_fallback_projection() {
    let region = SlicedRegion {
        object_id: "prepared-object".to_owned(),
        region_id: 9,
        polygons: vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: 20, y: 0 },
                    Point2 { x: 0, y: 20 },
                ],
            },
            holes: Vec::new(),
        }],
        ..Default::default()
    };
    let slice = SliceIR {
        global_layer_index: 0,
        regions: vec![region.clone()],
        ..Default::default()
    };
    let prepared = slicer_wasm_host::marshal::prepare_slice_regions(&slice, None);
    let fallback = slicer_wasm_host::host::sliced_region_to_data(
        &region,
        slice.z,
        Vec::new(),
        None,
        slice.global_layer_index,
    );
    let reused = slicer_wasm_host::host::sliced_region_to_data_with_prepared(
        &region,
        slice.z,
        Vec::new(),
        None,
        slice.global_layer_index,
        Some(&prepared[0]),
    );
    assert_eq!(fallback.object_id, reused.object_id);
    assert_eq!(fallback.region_id, reused.region_id);
    assert_eq!(fallback.polygons.len(), reused.polygons.len());
    assert_eq!(fallback.needs_support, reused.needs_support);
    assert_eq!(fallback.overhang_areas.len(), reused.overhang_areas.len());
    assert_eq!(
        fallback.overhang_quartile_polygons.len(),
        reused.overhang_quartile_polygons.len()
    );
    assert_eq!(
        fallback.prev_layer_boundary.len(),
        reused.prev_layer_boundary.len()
    );
    assert_eq!(
        fallback.surface_group.is_some(),
        reused.surface_group.is_some()
    );
}
