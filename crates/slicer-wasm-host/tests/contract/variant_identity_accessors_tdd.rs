//! Full-identity native postprocess delivery and public WIT config accessors.
//!
//! Host-trait calls cover the actual accessor boundary, not a full WASM
//! postprocess pipeline or geometry parity.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, LayerStageCommit, PaintValue, PerimeterIR,
    PerimeterRegion, Point2, Polygon, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR,
    SlicedRegion,
};
use slicer_sdk::native::{NativeLayerRequest, NativeLayerResponse, NativeStageEntry};
use slicer_wasm_host::binding::LayerStageInput;
use slicer_wasm_host::host::layer::slicer::config::config_types::HostConfigView;
use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::{
    HostPerimeterRegionView, HostSliceRegionView,
};
use slicer_wasm_host::host::{
    config_view_to_data, ConfigValueStorage, HostExecutionContext, HostExecutionContextBuilder,
};
use slicer_wasm_host::marshal::{perimeter_region_to_data, sliced_region_to_data, OriginId};
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionIndexInput, LayerSelectionModuleInput, STAGES,
};
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageRunner, WasmInstancePool, WasmRuntimeDispatcher,
};
use wasmtime::component::Resource;

fn own<T>(rep: u32) -> Resource<T> {
    Resource::new_own(rep)
}

fn chain(tool: u32) -> Vec<(String, PaintValue)> {
    vec![("material".into(), PaintValue::ToolIndex(tool))]
}

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        ..Default::default()
    }
}

struct Fixture {
    slice: SliceIR,
    perimeter: PerimeterIR,
    map: RegionMapIR,
    config: Arc<ConfigView>,
}

fn fixture() -> Fixture {
    let mut map = RegionMapIR::default();
    let mut slices = Vec::new();
    let mut perimeters = Vec::new();
    for (tool, width) in [(1, 0.4), (2, 0.6)] {
        let config = map.intern_config(ResolvedConfig {
            line_width: width,
            extensions: BTreeMap::from([("private_probe".into(), ConfigValue::Int(99))]),
            ..Default::default()
        });
        map.entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "accessor-cube".into(),
                region_id: 7,
                variant_chain: chain(tool),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
        // Deliberately collide object AND transported region ID. Only the
        // ordered material chain can distinguish these two accessor resources.
        slices.push(SlicedRegion {
            object_id: "accessor-cube".into(),
            region_id: 7_000_002,
            variant_chain: chain(tool),
            polygons: vec![square()],
            sparse_infill_area: vec![square()],
            effective_layer_height: 0.2,
            ..Default::default()
        });
        perimeters.push(PerimeterRegion {
            object_id: "accessor-cube".into(),
            region_id: 7_000_002,
            variant_chain: chain(tool),
            infill_areas: vec![square()],
            ..Default::default()
        });
    }
    Fixture {
        slice: SliceIR {
            z: 0.2,
            regions: slices,
            ..Default::default()
        },
        perimeter: PerimeterIR {
            regions: perimeters,
            ..Default::default()
        },
        map,
        config: Arc::new(ConfigView::from_declared(
            &HashMap::from([
                ("line_width".into(), ConfigValue::Float(0.8)),
                ("private_probe".into(), ConfigValue::Int(99)),
            ]),
            ["line_width"],
        )),
    }
}

thread_local! {
    static DELIVERED_CONFIGS: std::cell::RefCell<Vec<(OriginId, ConfigView)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn native_config_witness(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    assert_eq!(request.stage_export, "Layer::InfillPostProcess");
    assert_eq!(request.config.get_float("line_width"), Some(0.8));
    let regions = request
        .perimeter_regions
        .as_ref()
        .expect("postprocess perimeter views");
    DELIVERED_CONFIGS.with(|records| {
        records.borrow_mut().extend(regions.iter().map(|region| {
            (
                OriginId {
                    object_id: region.object_id().clone(),
                    region_id: *region.region_id(),
                    variant_chain: region.variant_chain().to_vec(),
                },
                region.config().expect("native per-region config").clone(),
            )
        }));
    });
    // exhaustive: postprocess observer supplies only its explicit empty infill replacement.
    Ok(NativeLayerResponse {
        infill: Some(slicer_sdk::builders::InfillOutputBuilder::new()),
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    })
}

fn run_native(fx: &Fixture) -> Vec<(OriginId, ConfigView)> {
    let id = "com.test.variant-accessors".to_string();
    let module = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        None,
        &[],
        fx.config.clone(),
    )
    .with_native_entry(NativeStageEntry::Layer(native_config_witness));
    let layer = GlobalLayer {
        z: 0.2,
        ..Default::default()
    };
    // exhaustive: snapshot has exact model/perimeter authority and no non-region products.
    let index = build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: std::slice::from_ref(&layer),
        // exhaustive: all static metadata belongs to this declared-key native module.
        modules: &[LayerSelectionModuleInput {
            stage_id: "Layer::InfillPostProcess",
            module_id: &id,
            claims: &[],
            paint_semantics: None,
            invocation_config: &fx.config,
        }],
        slices: std::slice::from_ref(&fx.slice),
        region_map: Some(&fx.map),
        support_plan: None,
        perimeters: std::slice::from_ref(&fx.perimeter),
    })
    .expect("complete full-identity postprocess selection");
    // exhaustive: production dispatch input pins all current products and indexed selection.
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: Some(Arc::new(fx.map.clone())),
        slice: Some(&fx.slice),
        perimeter: Some(&fx.perimeter),
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
        selection: Some(
            index
                .selection_for(layer.index, &id)
                .expect("bound native module"),
        ),
    };
    DELIVERED_CONFIGS.with(|records| records.borrow_mut().clear());
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".into(),
        &layer,
        &module,
        input,
    )
    .expect("production native postprocess dispatch");
    let Some(LayerStageCommit::InfillPostProcess(ir)) = commit else {
        panic!("native observer must commit its explicit replacement, got {commit:?}");
    };
    assert!(
        ir.regions.is_empty(),
        "observer is not a geometry implementation"
    );
    DELIVERED_CONFIGS.with(|records| records.borrow().clone())
}

fn assert_native_configs(records: &[(OriginId, ConfigView)], check_private: bool) {
    assert_eq!(
        records.len(),
        2,
        "native entry must observe both perimeter identities"
    );
    for (tool, width) in [(1, 0.4), (2, 0.6)] {
        let (origin, config) = records
            .iter()
            .find(|(origin, _)| origin.variant_chain == chain(tool))
            .unwrap_or_else(|| panic!("native: missing material variant {tool}"));
        assert_eq!(origin.object_id, "accessor-cube");
        assert_eq!(
            origin.region_id, 7_000_002,
            "postprocess must preserve raw identity"
        );
        assert_eq!(config.get_float("line_width"), Some(width));
        if check_private {
            assert_eq!(
                config.get("private_probe"),
                None,
                "native: undeclared extension leaked"
            );
        }
    }
}

fn boundary_context() -> HostExecutionContext {
    let mut ctx = HostExecutionContextBuilder::new("variant-accessors", 0.2, 0.2).build();
    ctx.set_default_config_fields(HashMap::from([
        ("line_width".into(), ConfigValueStorage::Float(0.8)),
        ("private_probe".into(), ConfigValueStorage::Int(99)),
    ]));
    // Independently authored boundary inputs: do not derive the host accessor
    // oracle from the native request or the selection's projected configs.
    let mut fields = HashMap::new();
    for (tool, width) in [(1, 0.4), (2, 0.6)] {
        let source = HashMap::from([
            ("line_width".into(), ConfigValue::Float(width)),
            ("private_probe".into(), ConfigValue::Int(99)),
        ]);
        let declared = ConfigView::from_declared(&source, ["line_width"]);
        fields.insert(
            OriginId {
                object_id: "accessor-cube".into(),
                region_id: 7_000_002,
                variant_chain: chain(tool),
            },
            config_view_to_data(&declared).fields,
        );
    }
    ctx.set_config_fields_per_region(fields);
    ctx
}

fn assert_public_accessors(fx: &Fixture, ctx: &mut HostExecutionContext, check_private: bool) {
    assert_eq!(fx.slice.regions.len(), 2);
    assert_eq!(fx.perimeter.regions.len(), 2);
    for (tool, width) in [(1, 0.4), (2, 0.6)] {
        let slice = fx
            .slice
            .regions
            .iter()
            .find(|region| region.variant_chain == chain(tool))
            .expect("slice variant fixture");
        let perimeter = fx
            .perimeter
            .regions
            .iter()
            .find(|region| region.variant_chain == chain(tool))
            .expect("perimeter variant fixture");
        let slice_handle = ctx
            .push_slice_region(sliced_region_to_data(slice, 0.2, vec![], None, 0))
            .expect("slice-region resource");
        let perimeter_handle = ctx
            .push_perimeter_region(perimeter_region_to_data(perimeter))
            .expect("perimeter-region resource");
        let slice_config = HostSliceRegionView::config(ctx, own(slice_handle.rep()))
            .expect("slice full-identity config accessor");
        let perimeter_config = HostPerimeterRegionView::config(ctx, own(perimeter_handle.rep()))
            .expect("perimeter full-identity config accessor");
        for (kind, config) in [("slice", slice_config), ("perimeter", perimeter_config)] {
            assert_eq!(
                HostConfigView::get_float(ctx, own(config.rep()), "line_width".into()).unwrap(),
                Some(width),
                "{kind}: material variant {tool} must not borrow sibling/default width"
            );
            if check_private {
                assert!(
                    HostConfigView::get(ctx, own(config.rep()), "private_probe".into())
                        .unwrap()
                        .is_none(),
                    "{kind}: undeclared extension leaked"
                );
            }
            HostConfigView::drop(ctx, own(config.rep())).unwrap();
        }
        HostSliceRegionView::drop(ctx, own(slice_handle.rep())).unwrap();
        HostPerimeterRegionView::drop(ctx, own(perimeter_handle.rep())).unwrap();
    }
}

#[test]
fn slice_and_perimeter_accessors_preserve_variant_identity() {
    let fx = fixture();
    assert_native_configs(&run_native(&fx), false);
    assert_public_accessors(&fx, &mut boundary_context(), false);
}

#[test]
fn declared_keys_and_canonical_ids_are_enforced() {
    let fx = fixture();
    assert_native_configs(&run_native(&fx), true);
    let mut ctx = boundary_context();
    assert_public_accessors(&fx, &mut ctx, true);

    let mut slice = sliced_region_to_data(&fx.slice.regions[0], 0.2, vec![], None, 0);
    slice.region_id = "01".into();
    let slice_handle = ctx
        .push_slice_region(slice)
        .expect("noncanonical slice resource");
    let slice_error = HostSliceRegionView::config(&mut ctx, own(slice_handle.rep()))
        .expect_err("slice config must reject malformed identity before default lookup");
    let mut perimeter = perimeter_region_to_data(&fx.perimeter.regions[0]);
    perimeter.region_id = "01".into();
    let perimeter_handle = ctx
        .push_perimeter_region(perimeter)
        .expect("noncanonical perimeter resource");
    let perimeter_error = HostPerimeterRegionView::config(&mut ctx, own(perimeter_handle.rep()))
        .expect_err("perimeter config must reject malformed identity before default lookup");
    for (kind, error) in [("slice", slice_error), ("perimeter", perimeter_error)] {
        assert_eq!(
            error.to_string(),
            "expected canonical decimal u64 string with no leading zeros, got '01'",
            "{kind}: malformed identity must not silently use nonempty default fields"
        );
    }
}
