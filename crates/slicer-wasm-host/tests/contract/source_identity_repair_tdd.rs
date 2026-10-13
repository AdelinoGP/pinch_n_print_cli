//! Repair witnesses use independently specified identities and geometry.
//! Existing SDK postprocess echoes and committed support geometry observe the
//! WASM leg; native recording retains controls not exposed by those guests.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer,
    LayerStageCommit, LightningTreeEntry, LightningTreeIR, LoopType, PaintValue, PerimeterIR,
    PerimeterRegion, Point2, Point3WithWidth, Polygon, PreparedRegionData, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SliceIR, SlicedRegion, SupportPlanEntry, SupportPlanIR,
    SupportPlanRole, SupportPlanRoleRegion, WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_sdk::builders::{InfillOutputBuilder, PerimeterOutputBuilder};
use slicer_sdk::native::{NativeLayerRequest, NativeLayerResponse, NativeStageEntry};
use slicer_wasm_host::binding::LayerStageInput;
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelection, LayerSelectionIndex, LayerSelectionIndexInput,
    LayerSelectionModuleInput, LayerTargetSource, STAGES,
};
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageRunner, WasmInstancePool, WasmRuntimeDispatcher,
};

#[derive(Clone, Debug)]
struct Delivery {
    object: String,
    region: u64,
    chain: Vec<(String, PaintValue)>,
    polygons: Vec<ExPolygon>,
    width: f64,
    claims: Vec<String>,
    needs_support: bool,
    private: Option<ConfigValue>,
    extruder: Option<ConfigValue>,
}

thread_local! {
    static DELIVERED: RefCell<Vec<Delivery>> = const { RefCell::new(Vec::new()) };
}

fn chain(tool: u32) -> Vec<(String, PaintValue)> {
    vec![("material".into(), PaintValue::ToolIndex(tool))]
}

fn square(x: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, 0.0),
                Point2::from_mm(x + 10.0, 0.0),
                Point2::from_mm(x + 10.0, 10.0),
                Point2::from_mm(x, 10.0),
            ],
        },
        ..ExPolygon::default()
    }
}

fn layer() -> GlobalLayer {
    GlobalLayer {
        index: 7,
        z: 0.3,
        ..GlobalLayer::default()
    }
}

fn resolved(width: f64) -> ResolvedConfig {
    let mut extensions = BTreeMap::new();
    for (key, value) in [
        ("nozzle_diameter", 0.4),
        ("layer_height", 0.2),
        ("outer_wall_speed", 30.0),
        ("inner_wall_speed", 45.0),
        ("perimeter_arc_tolerance", 0.0125),
        ("bridge_line_width", 0.0),
        ("outer_wall_line_width", 0.0),
        ("inner_wall_line_width", 0.0),
        ("seam_candidate_angle_threshold_deg", 30.0),
        ("gap_infill_speed", 30.0),
        ("filter_out_gap_fill", 0.5),
        ("sparse_infill_density", 20.0),
        ("bridge_flow", 1.0),
        ("smaller_perimeter_line_width", 0.25),
        ("smaller_perimeter_threshold_mm", 0.8),
        ("narrow_loop_length_threshold_mm", 10.0),
        ("min_width_top_surface", 0.0),
        ("support_angle", 0.0),
        ("support_speed", 60.0),
        ("support_base_pattern_spacing", 2.0),
        ("support_interface_spacing", 0.2),
        ("support_bottom_interface_spacing", 0.2),
        ("infill_density", 0.2),
        ("initial_layer_line_width", 0.0),
        ("top_surface_line_width", 0.0),
        ("internal_solid_infill_line_width", 0.0),
        ("sparse_infill_line_width", 0.0),
        ("bridge_density", 1.0),
        ("bridge_speed", 25.0),
        ("internal_bridge_density", 1.0),
        ("internal_bridge_speed", 37.5),
        ("internal_bridge_flow", 1.0),
        ("top_surface_speed", 60.0),
        ("internal_solid_infill_speed", 60.0),
        ("sparse_infill_speed", 60.0),
        ("internal_bridge_angle", 0.0),
        ("infill_shift_step", 0.0),
        ("infill_angle", 45.0),
        ("infill_speed", 60.0),
    ] {
        extensions.insert(key.into(), ConfigValue::Float(value));
    }
    for key in [
        "detect_thin_wall",
        "gap_fill_medial_axis_on_painted",
        "slice_has_paint",
        "precise_outer_wall",
        "alternate_extra_wall",
        "spiral_vase",
        "extra_perimeters_on_overhangs",
        "only_one_wall_top",
        "only_one_wall_first_layer",
        "thick_bridges",
        "dont_filter_internal_bridges",
        "enable_extra_bridge_layer",
    ] {
        extensions.insert(key.into(), ConfigValue::Bool(false));
    }
    extensions.insert("thick_internal_bridges".into(), ConfigValue::Bool(true));
    extensions.insert("wall_count".into(), ConfigValue::Int(2));
    extensions.insert("extra_perimeters".into(), ConfigValue::Int(0));
    extensions.insert("support_raft_layers".into(), ConfigValue::Int(0));
    extensions.insert(
        "wall_sequence".into(),
        ConfigValue::String("InnerOuter".into()),
    );
    extensions.insert("support_style".into(), ConfigValue::String("grid".into()));
    extensions.insert("support_interface_flow".into(), ConfigValue::Percent(100.0));
    extensions.insert("infill_wall_overlap".into(), ConfigValue::Percent(15.0));
    extensions.insert(
        "top_bottom_infill_wall_overlap".into(),
        ConfigValue::Percent(25.0),
    );
    ResolvedConfig {
        line_width: width,
        support_enabled: true,
        extensions,
        ..ResolvedConfig::default()
    }
}

fn config() -> Arc<ConfigView> {
    Arc::new(ConfigView::from_map(resolved(0.8).to_config_map()))
}

fn historical_invocation_config() -> Arc<ConfigView> {
    // Same invocation baseline as AC-1, independent of the model configs.
    let mut values = std::collections::HashMap::new();
    for (key, value) in [
        ("infill_density", 0.2),
        ("line_width", 0.8),
        ("bridge_line_width", 0.0),
        ("initial_layer_line_width", 0.0),
        ("top_surface_line_width", 0.0),
        ("internal_solid_infill_line_width", 0.0),
        ("sparse_infill_line_width", 0.0),
        ("bridge_density", 1.0),
        ("bridge_speed", 25.0),
        ("bridge_flow", 1.0),
        ("internal_bridge_density", 1.0),
        ("internal_bridge_speed", 37.5),
        ("internal_bridge_flow", 1.0),
        ("top_surface_speed", 60.0),
        ("internal_solid_infill_speed", 60.0),
        ("sparse_infill_speed", 60.0),
        ("internal_bridge_angle", 0.0),
        ("infill_shift_step", 0.0),
        ("infill_angle", 45.0),
        ("infill_speed", 60.0),
    ] {
        values.insert(key.into(), ConfigValue::Float(value));
    }
    for (key, value) in [
        ("thick_bridges", false),
        ("thick_internal_bridges", true),
        ("dont_filter_internal_bridges", false),
        ("enable_extra_bridge_layer", false),
    ] {
        values.insert(key.into(), ConfigValue::Bool(value));
    }
    Arc::new(ConfigView::from_map(values))
}

fn physical_layer(slice: Option<&SliceIR>) -> GlobalLayer {
    slice.map_or_else(layer, |s| GlobalLayer {
        index: s.global_layer_index,
        z: s.z,
        ..GlobalLayer::default()
    })
}

fn guest(name: &str) -> Arc<slicer_wasm_host::WasmComponent> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../modules/core-modules/{name}/{name}.wasm"));
    assert!(path.is_file(), "missing guest: {}", path.display());
    crate::common::wasm_cache::compiled_component_at(&path)
}

fn key(id: u64, paint: Vec<(String, PaintValue)>) -> RegionKey {
    RegionKey {
        global_layer_index: 7,
        object_id: "repair-model".into(),
        region_id: id,
        variant_chain: paint,
    }
}

fn region(id: u64, paint: Vec<(String, PaintValue)>, x: f32) -> SlicedRegion {
    SlicedRegion {
        object_id: "repair-model".into(),
        region_id: id,
        variant_chain: paint,
        polygons: vec![square(x)],
        sparse_infill_area: vec![square(x)],
        effective_layer_height: 0.2,
        ..SlicedRegion::default()
    }
}

fn slice(regions: Vec<SlicedRegion>) -> SliceIR {
    SliceIR {
        global_layer_index: 7,
        z: 0.3,
        regions,
        ..SliceIR::default()
    }
}

fn insert(map: &mut RegionMapIR, key: RegionKey, cfg: ResolvedConfig) {
    let config = map.intern_config(cfg);
    map.entries.insert(
        key,
        RegionPlan {
            config,
            ..RegionPlan::default()
        },
    );
}

fn index(
    stage: &str,
    module: &CompiledModuleLive<'_>,
    slice: Option<&SliceIR>,
    map: Option<&RegionMapIR>,
    plan: Option<&SupportPlanIR>,
) -> LayerSelectionIndex {
    // exhaustive: borrowed static binding has no default and fixes all authorities.
    let binding = LayerSelectionModuleInput {
        stage_id: stage,
        module_id: module.module_id,
        claims: module.claims,
        paint_semantics: None,
        invocation_config: &module.config_view,
    };
    // exhaustive: index fixture explicitly chooses every committed product channel.
    build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &[physical_layer(slice)],
        modules: &[binding],
        slices: slice.map(std::slice::from_ref).unwrap_or(&[]),
        region_map: map,
        support_plan: plan,
        perimeters: &[],
    })
    .expect("public repair selection index")
}

// A borrowed runner envelope is passed intact to both production transports.
fn run(
    stage: &str,
    module: &CompiledModuleLive<'_>,
    selection: &LayerSelection,
    slice: Option<&SliceIR>,
    map: Option<&RegionMapIR>,
    plan: Option<&SupportPlanIR>,
    perimeter: Option<&PerimeterIR>,
    prepared: Option<&[PreparedRegionData]>,
) -> Option<LayerStageCommit> {
    run_result(
        stage, module, selection, slice, map, plan, perimeter, prepared,
    )
    .expect("real indexed stage invocation")
}

fn run_result(
    stage: &str,
    module: &CompiledModuleLive<'_>,
    selection: &LayerSelection,
    slice: Option<&SliceIR>,
    map: Option<&RegionMapIR>,
    plan: Option<&SupportPlanIR>,
    perimeter: Option<&PerimeterIR>,
    prepared: Option<&[PreparedRegionData]>,
) -> Result<Option<LayerStageCommit>, slicer_ir::LayerStageError> {
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    DELIVERED.with(|records| records.borrow_mut().clear());
    LayerStageRunner::last_wasm_mem_sample(&dispatcher);
    // exhaustive: runner envelope has no Default; current products are never inferred.
    let input = LayerStageInput {
        selection: Some(selection),
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: plan.cloned().map(Arc::new),
        lightning_tree_ir: None,
        region_map: map.cloned().map(Arc::new),
        slice,
        perimeter,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: prepared,
        infill: None,
    };
    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &stage.into(),
        &physical_layer(slice),
        module,
        input,
    );
    if module.native_entry.is_none() && result.is_ok() {
        let (initial, _) = LayerStageRunner::last_wasm_mem_sample(&dispatcher);
        assert!(initial > 0, "WASM leg must really invoke");
    }
    result
}

fn empty_response() -> NativeLayerResponse {
    // exhaustive: recording entry explicitly selects the response stage slot.
    NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    }
}

fn record(request: &NativeLayerRequest) {
    DELIVERED.with(|records| {
        *records.borrow_mut() = request
            .regions
            .iter()
            .map(|r| {
                let cfg = r.config().expect("indexed region config");
                Delivery {
                    object: r.object_id().to_string(),
                    region: *r.region_id(),
                    chain: r.variant_chain().to_vec(),
                    polygons: r.polygons().to_vec(),
                    width: cfg.get_float("line_width").unwrap(),
                    claims: r.held_claims().to_vec(),
                    needs_support: r.needs_support(),
                    private: cfg.get("private_probe").cloned(),
                    extruder: cfg.get("extruder").cloned(),
                }
            })
            .collect();
    });
}

fn path(poly: &ExPolygon, width: f32, role: ExtrusionRole) -> ExtrusionPath3D {
    assert!(
        !poly.contour.points.is_empty(),
        "witness requires nonempty geometry"
    );
    // exhaustive: no path Default; author no tool override or order lock.
    ExtrusionPath3D {
        points: poly
            .contour
            .points
            .iter()
            .map(|p| Point3WithWidth {
                x: slicer_ir::units_to_mm(p.x),
                y: slicer_ir::units_to_mm(p.y),
                z: 0.3,
                width,
                flow_factor: 1.0,
                ..Point3WithWidth::default()
            })
            .collect(),
        role,
        speed_factor: 1.0,
        tool_index: None,
        order_lock: None,
    }
}

fn native_perimeters(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    record(request);
    let mut output = PerimeterOutputBuilder::new();
    for r in &request.regions {
        output.begin_region(r.object_id(), *r.region_id(), r.variant_chain());
        let width = r.config().unwrap().get_float("line_width").unwrap() as f32;
        for polygon in r.polygons() {
            let path = path(polygon, width, ExtrusionRole::OuterWall);
            output
                // exhaustive: a wall has no Default; metadata is independent of host projection.
                .push_wall_loop(WallLoop {
                    perimeter_index: 0,
                    loop_type: LoopType::Outer,
                    width_profile: WidthProfile {
                        widths: vec![width; path.points.len()],
                    },
                    feature_flags: vec![Default::default(); path.points.len()],
                    path,
                    boundary_type: WallBoundaryType::ExteriorSurface,
                })
                .unwrap();
        }
    }
    Ok(NativeLayerResponse {
        perimeters: Some(output),
        ..empty_response()
    })
}

fn native_infill(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    record(request);
    let mut output = InfillOutputBuilder::new();
    for r in &request.regions {
        output.begin_region(r.object_id(), *r.region_id(), r.variant_chain());
        let width = r.config().unwrap().get_float("line_width").unwrap() as f32;
        for polygon in r.polygons() {
            if r.held_claims().iter().any(|c| c == "claim:sparse-fill") {
                output
                    .push_sparse_path(path(polygon, width, ExtrusionRole::SparseInfill))
                    .unwrap();
            }
            if r.held_claims().iter().any(|c| c == "claim:top-fill") {
                output
                    .push_solid_path(path(polygon, width, ExtrusionRole::TopSolidInfill))
                    .unwrap();
            }
        }
    }
    Ok(NativeLayerResponse {
        infill: Some(output),
        ..empty_response()
    })
}

fn native_carriers(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    record(request);
    Ok(empty_response())
}

fn native_postprocess(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    DELIVERED.with(|records| {
        *records.borrow_mut() = request
            .perimeter_regions
            .as_ref()
            .expect("perimeter views")
            .iter()
            .map(|r| {
                let cfg = r.config().expect("indexed postprocess config");
                Delivery {
                    object: r.object_id().to_string(),
                    region: *r.region_id(),
                    chain: r.variant_chain().to_vec(),
                    polygons: r.infill_areas().to_vec(),
                    width: cfg.get_float("line_width").unwrap(),
                    claims: vec![],
                    needs_support: false,
                    private: cfg.get("private_probe").cloned(),
                    extruder: cfg.get("extruder").cloned(),
                }
            })
            .collect();
    });
    Ok(NativeLayerResponse {
        infill: Some(InfillOutputBuilder::new()),
        ..empty_response()
    })
}

fn deliveries() -> Vec<Delivery> {
    DELIVERED.with(|records| records.borrow().clone())
}

// These are delivery ordinals and geometry counts, not source IDs, config, or
// prepared annotations. Expected counts come from each independent fixture.
fn with_wasm_perimeter_delivery(
    contour_counts: &[usize],
    invoke: impl FnOnce() -> Option<LayerStageCommit>,
) -> Option<LayerStageCommit> {
    #[cfg(feature = "perimeter-spatial-test-support")]
    {
        let (commit, _, records) =
            slicer_core::perimeter_spatial::diagnostics::with_capture(false, false, invoke);
        assert_eq!(
            records.first().map(|r| (r.layer_index, r.region_ordinal)),
            Some((7, 0)),
            "positive WASM delivery identity: first indexed source at layer 7"
        );
        let actual: Vec<_> = records
            .iter()
            .map(|r| {
                (
                    r.layer_index,
                    r.region_ordinal,
                    r.contour_count,
                    r.hole_count,
                    r.bridge_count,
                )
            })
            .collect();
        let expected: Vec<_> = contour_counts
            .iter()
            .enumerate()
            .map(|(ordinal, count)| (7, ordinal, *count, 0, 0))
            .collect();
        assert_eq!(
            actual, expected,
            "WASM delivery must include every layer-7 ordinal, even empty contours"
        );
        commit
    }
    #[cfg(not(feature = "perimeter-spatial-test-support"))]
    {
        let _ = contour_counts;
        invoke()
    }
}

fn assert_empty_postprocess_commit(commit: Option<LayerStageCommit>) {
    let Some(LayerStageCommit::InfillPostProcess(ir)) = commit else {
        panic!("a ran native postprocess must commit its explicit empty replacement")
    };
    assert_eq!(ir.global_layer_index, 7);
    assert!(ir.regions.is_empty() && ir.raft_regions.is_empty());
}

// This single-stage guest already echoes every delivered perimeter view,
// including views with no authorized donor. It does not echo line_width.
fn wasm_donor_echo(
    module: &CompiledModuleLive<'_>,
    selection: &LayerSelection,
    slice: &SliceIR,
    map: &RegionMapIR,
    perimeter: &PerimeterIR,
) -> Vec<String> {
    let stage = "Layer::InfillPostProcess";
    let echo = CompiledModuleLive::new(
        module.module_id,
        WasmInstancePool::placeholder(),
        Some(crate::common::wasm_cache::compiled_guest(
            "sdk-infill-postprocess-view-guest",
        )),
        module.claims,
        config(),
    );
    let error = run_result(
        stage,
        &echo,
        selection,
        Some(slice),
        Some(map),
        None,
        Some(perimeter),
        None,
    )
    .expect_err("SDK echo must execute its trait body");
    let slicer_ir::LayerStageError::FatalModule {
        stage_id,
        module_id,
        message,
    } = error
    else {
        panic!("SDK echo must return its typed fatal payload: {error:?}")
    };
    assert_eq!(stage_id, stage);
    assert_eq!(module_id, *module.module_id);
    let payload = message
        .split_once("SDK-VIEW")
        .expect("not an instantiation error: the guest must echo delivered SDK views")
        .1;
    let mut records = payload.split(" region=");
    assert_eq!(records.next(), Some(""));
    records.map(str::to_string).collect()
}

fn assert_donor_echo(actual: Vec<String>, expected: &[(u64, Vec<ExPolygon>)]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "SDK delivery cardinality includes donorless selected views"
    );
    for (region, polygons) in expected {
        let identity = format!("repair-model:{region} chain={:?} ", chain(1));
        let matching: Vec<_> = actual
            .iter()
            .filter(|record| record.starts_with(&identity))
            .collect();
        assert_eq!(matching.len(), 1, "exact full-source SDK view: {actual:?}");
        let infill = matching[0]
            .split_once(" infill=")
            .and_then(|(_, rest)| rest.split_once(" sparse="))
            .expect("the committed SDK guest echoes infill_areas")
            .0;
        assert_eq!(infill, format!("{polygons:?}"), "SDK donor for {region}");
    }
}

#[test]
fn stored_members_only_before_union() {
    let id = "com.core.classic-perimeters".to_string();
    let claims = vec!["claim:perimeters".into()];
    let modules = [
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            None,
            &claims,
            config(),
        )
        .with_native_entry(NativeStageEntry::Layer(native_perimeters)),
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            Some(guest("classic-perimeters")),
            &claims,
            config(),
        ),
    ];
    let child = slicer_ir::modifier_sub_region_id(7_000_002, "stored", &[]);
    let unindexed = slicer_ir::modifier_sub_region_id(7_000_002, "late", &[]);
    assert_ne!(child, unindexed, "discriminating modifier identities");
    let original = slice(vec![
        region(7_000_002, chain(1), 0.0),
        region(child, chain(1), 15.0),
        region(7_000_003, chain(2), 30.0),
    ]);
    let mut map = RegionMapIR::default();
    insert(&mut map, key(7, chain(1)), resolved(0.4));
    insert(&mut map, key(child, chain(1)), resolved(1.2));
    insert(&mut map, key(7, chain(2)), resolved(0.6));
    let index = index(
        "Layer::Perimeters",
        &modules[0],
        Some(&original),
        Some(&map),
        None,
    );
    let selected = index.selection_for(7, &id).unwrap();
    assert_eq!(selected.targets.len(), 2);
    assert!(selected.targets.iter().any(|t| matches!(&t.source,
        LayerTargetSource::Model { key: k, members } if k == &key(7_000_002, chain(1)) && members == &vec![key(7_000_002, chain(1)), key(child, chain(1))])));
    let mut current = original.clone();
    current.regions.push(region(unindexed, chain(1), 60.0));
    let mut failures = Vec::new();
    for reverse in [false, true] {
        if reverse {
            current.regions.reverse();
        }
        // Prepared perimeter data follows the current source projection order.
        let markers = if reverse {
            [true, false]
        } else {
            [false, true]
        };
        let prepared: Vec<_> = markers
            .into_iter()
            .map(|needs_support| PreparedRegionData {
                needs_support,
                ..PreparedRegionData::default()
            })
            .collect();
        for (leg, module) in [("native", &modules[0]), ("wasm", &modules[1])] {
            let invoke = || {
                run(
                    "Layer::Perimeters",
                    module,
                    selected,
                    Some(&current),
                    Some(&map),
                    None,
                    None,
                    Some(&prepared),
                )
            };
            let commit = if leg == "wasm" {
                // The first source unions two disjoint stored contours; the
                // unindexed child would add a third. Reversing current input
                // must not reorder these indexed source deliveries.
                with_wasm_perimeter_delivery(&[2, 1], invoke)
            } else {
                invoke()
            };
            let Some(LayerStageCommit::Perimeters(ir)) = commit else {
                panic!("{leg}: nonempty perimeter commit")
            };
            assert_eq!(
                ir.regions.len(),
                2,
                "both indexed source identities delivered"
            );
            for r in &ir.regions {
                assert!(!r.walls.is_empty(), "{leg}: nonempty output guard");
                let xs: Vec<_> = r
                    .walls
                    .iter()
                    .flat_map(|w| &w.path.points)
                    .map(|p| p.x)
                    .collect();
                assert!(!xs.is_empty());
                if r.variant_chain == chain(1) {
                    assert!(
                        xs.iter().any(|x| (14.0..26.0).contains(x)),
                        "stored member remains"
                    );
                    if xs.iter().any(|x| *x >= 59.0) {
                        failures.push(format!(
                            "{leg}/reverse={reverse}: unindexed child entered union"
                        ));
                    }
                } else {
                    assert_eq!(r.variant_chain, chain(2));
                    assert!(xs.iter().all(|x| *x >= 29.0));
                }
            }
            if leg == "native" {
                let values = deliveries();
                assert_eq!(values.len(), 2);
                for (paint, raw_id, width, expected) in [
                    (chain(1), 7_000_002, 0.4, false),
                    (chain(2), 7_000_003, 0.6, true),
                ] {
                    let delivered = values.iter().find(|d| d.chain == paint).unwrap();
                    assert_eq!((delivered.region, delivered.width), (raw_id, width));
                    if delivered.needs_support != expected {
                        failures.push(format!(
                            "native/reverse={reverse}: annotation changed source"
                        ));
                    }
                }
            }
        }
        // Capture adds delivery order/contour counts, not needs_support or
        // source IDs. Prepared-annotation identity remains native-controlled;
        // committed perimeter geometry independently proves WASM membership.
    }
    assert!(
        failures.is_empty(),
        "AC-R1 stored-members-only union / source annotations: {failures:?}"
    );
}

#[test]
fn exact_own_and_explicit_parent_donors() {
    let id = "com.test.repair-donors".to_string();
    let module = CompiledModuleLive::new(&id, WasmInstancePool::placeholder(), None, &[], config())
        .with_native_entry(NativeStageEntry::Layer(native_postprocess));
    let child = slicer_ir::modifier_sub_region_id(7_000_002, "modifier", &[]);
    let missing = slicer_ir::modifier_sub_region_id(9_000_002, "missing", &[]);
    let current = slice(vec![
        region(7_000_002, chain(1), 0.0),
        region(child, chain(1), 20.0),
        region(missing, chain(1), 40.0),
    ]);
    let mut map = RegionMapIR::default();
    for (k, width) in [
        (key(7, chain(1)), 0.4),
        (key(child, chain(1)), 0.6),
        (key(missing, chain(1)), 1.2),
    ] {
        insert(&mut map, k, resolved(width));
    }
    let index = index(
        "Layer::InfillPostProcess",
        &module,
        Some(&current),
        Some(&map),
        None,
    );
    let selected = index.selection_for(7, &id).unwrap();
    assert_eq!(selected.targets.len(), 3);
    let donor = |id, paint, x| PerimeterRegion {
        object_id: "repair-model".into(),
        region_id: id,
        variant_chain: paint,
        infill_areas: vec![square(x)],
        ..PerimeterRegion::default()
    };
    let mut failures = Vec::new();
    for own in [true, false] {
        let mut donors = vec![
            donor(7, vec![], 70.0),
            donor(7_000_002, chain(1), 80.0),
            donor(7_000_003, chain(2), 90.0),
        ];
        if own {
            donors.push(donor(child, chain(1), 100.0));
        }
        let perimeter = PerimeterIR {
            global_layer_index: 7,
            regions: donors,
            ..PerimeterIR::default()
        };
        assert_empty_postprocess_commit(run(
            "Layer::InfillPostProcess",
            &module,
            selected,
            Some(&current),
            Some(&map),
            None,
            Some(&perimeter),
            None,
        ));
        let values = deliveries();
        assert_eq!(values.len(), 3, "selected source views must not disappear");
        let painted = values.iter().find(|d| d.region == 7_000_002).unwrap();
        if painted.polygons != vec![square(80.0)] {
            failures.push(format!("painted exact own: {:?}", painted.polygons));
        }
        let modifier = values.iter().find(|d| d.region == child).unwrap();
        if modifier.polygons != vec![square(if own { 100.0 } else { 80.0 })] {
            failures.push(format!("modifier own={own}: wrong donor"));
        }
        assert_eq!(modifier.width, 0.6, "donor cannot replace target config");
        assert!(
            values
                .iter()
                .find(|d| d.region == missing)
                .unwrap()
                .polygons
                .is_empty(),
            "no authorized source means empty geometry"
        );
        assert_donor_echo(
            wasm_donor_echo(&module, selected, &current, &map, &perimeter),
            &[
                (7_000_002, vec![square(80.0)]),
                (child, vec![square(if own { 100.0 } else { 80.0 })]),
                (missing, vec![]),
            ],
        );
    }
    let perimeter = PerimeterIR {
        global_layer_index: 7,
        regions: vec![donor(7, vec![], 70.0), donor(7_000_003, chain(2), 90.0)],
        ..PerimeterIR::default()
    };
    assert_empty_postprocess_commit(run(
        "Layer::InfillPostProcess",
        &module,
        selected,
        Some(&current),
        Some(&map),
        None,
        Some(&perimeter),
        None,
    ));
    let values = deliveries();
    assert_eq!(values.len(), 3);
    if values
        .iter()
        .find(|d| d.region == 7_000_002)
        .unwrap()
        .polygons
        != vec![square(70.0)]
    {
        failures.push("painted absent own must use base source with EMPTY chain".into());
    }
    assert_donor_echo(
        wasm_donor_echo(&module, selected, &current, &map, &perimeter),
        &[
            (7_000_002, vec![square(70.0)]),
            (child, vec![]),
            (missing, vec![]),
        ],
    );
    // Postprocess uses push_infill_postprocess_regions, which does not emit
    // RegionCaptureRecord. The SDK echo above observes exact source identities
    // and donor geometry; target config remains the native control.
    assert!(
        failures.is_empty(),
        "AC-R2 exact source-space own and explicit parent donors: {failures:?}"
    );
}

fn support_entry(n: usize) -> SupportPlanEntry {
    // exhaustive: support entry has no Default; distinct plan provenance is essential.
    SupportPlanEntry {
        global_layer_index: 7,
        object_id: "carrier".into(),
        region_id: 70,
        family_id: "traditional".into(),
        demand_ids: vec![format!("demand-{n}")],
        body_ids: vec![format!("body-{n}")],
        anchor_layer_index: 3,
        anchor_z: slicer_ir::mm_to_units(0.3),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square(n as f32 * 20.0)],
        }],
        skeleton: None,
        capabilities: vec![],
        provenance: vec![format!("producer-{n}")],
        decline_reason: None,
        variant_chain: vec![],
    }
}

fn carrier_witness(chained_control: bool) -> Vec<usize> {
    let id = "com.core.traditional-support".to_string();
    let claims = vec!["support-family:traditional".into()];
    let modules = [
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            None,
            &claims,
            config(),
        )
        .with_native_entry(NativeStageEntry::Layer(native_carriers)),
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            Some(guest("traditional-support")),
            &claims,
            config(),
        ),
    ];
    let mut entries = (0..7).map(support_entry).collect::<Vec<_>>();
    entries[2].global_layer_index = 3;
    entries[3].family_id = "tree".into();
    entries[4].decline_reason = Some(slicer_ir::SupportPlanDeclineReason::DeclinedPolicy);
    entries[5].roles[0].regions.clear();
    entries[6].object_id = "present".into();
    entries[6].region_id = 71;
    let ordered_chain = vec![
        ("material".into(), PaintValue::ToolIndex(2)),
        ("fuzzy_skin".into(), PaintValue::Flag(true)),
    ];
    if chained_control {
        let mut chained = support_entry(7);
        chained.variant_chain = ordered_chain.clone();
        entries.push(chained);
    }
    let plan = SupportPlanIR {
        entries,
        ..SupportPlanIR::default()
    };
    let current = slice(vec![SlicedRegion {
        object_id: "present".into(),
        region_id: 71,
        ..SlicedRegion::default()
    }]);
    let mut map = RegionMapIR::default();
    insert(
        &mut map,
        RegionKey {
            object_id: "present".into(),
            region_id: 71,
            ..key(71, vec![])
        },
        resolved(0.4),
    );
    let selection_index = index(
        "Layer::Support",
        &modules[0],
        Some(&current),
        Some(&map),
        Some(&plan),
    );
    let selected = selection_index.selection_for(7, &id).unwrap();
    let carriers: Vec<_> = selected
        .targets
        .iter()
        .filter(|t| matches!(t.source, LayerTargetSource::SupportCarrier { .. }))
        .collect();
    assert_eq!(
        carriers.len(),
        if chained_control { 3 } else { 2 },
        "eligible plan contexts remain distinct, every exclusion retained"
    );
    for (n, t) in carriers.iter().enumerate() {
        let n = if n == 2 { 7 } else { n };
        assert_eq!(
            t.source,
            LayerTargetSource::SupportCarrier {
                entry_index: n,
                object_id: "carrier".into(),
                region_id: 70,
                variant_chain: if n == 7 {
                    ordered_chain.clone()
                } else {
                    vec![]
                },
                family: "traditional".into(),
                demand_ids: vec![format!("demand-{n}")],
                body_ids: vec![format!("body-{n}")],
                provenance: vec![format!("producer-{n}")],
                anchor_layer_index: 3
            }
        );
        assert!(
            !t.authored_coloring,
            "carrier cannot receive model coloring authority"
        );
        assert_eq!(t.config.get_float("line_width"), Some(0.8));
    }
    assert!(!selected
        .perimeter_configs
        .keys()
        .any(|k| k.object_id == "carrier"));
    let mut counts = Vec::new();
    for module in &modules {
        assert!(
            run(
                "Layer::Support",
                module,
                selected,
                Some(&current),
                Some(&map),
                Some(&plan),
                None,
                None
            )
            .is_none(),
            "anchor mismatch is a delivery-only control"
        );
        if module.native_entry.is_some() {
            let values = deliveries();
            assert!(!values.is_empty(), "native delivery nonempty guard");
            assert_eq!(values.len(), if chained_control { 3 } else { 2 });
            let wire: Vec<_> = values
                .iter()
                .filter(|d| d.object == "carrier" && d.region == 70)
                .collect();
            counts.push(wire.len());
            assert_eq!(wire.len(), if chained_control { 2 } else { 1 });
            let expected_chains = if chained_control {
                vec![vec![], ordered_chain.clone()]
            } else {
                vec![vec![]]
            };
            for paint in expected_chains {
                let d = wire
                    .iter()
                    .find(|d| d.chain == paint)
                    .expect("each full carrier origin has its own wire handle");
                assert!(d.polygons.is_empty());
                assert_eq!(d.width, 0.8);
                assert_eq!(d.claims, claims);
                assert_eq!(d.private, None);
                assert_eq!(d.extruder, None);
            }
        } else {
            // Keep the anchor-mismatch/exclusion matrix above unchanged. A
            // second invocation makes only its eligible contexts renderable:
            // the real support guest queries each delivered full origin. A
            // lost/reordered chain loses its polygon; duplicate wire handles
            // render the same paths twice. Observe that at the commit boundary,
            // rather than inventing an echo export on the SDK infill guest.
            let mut render_plan = plan.clone();
            for n in [0, 1, 6] {
                render_plan.entries[n].anchor_layer_index = 7;
            }
            if chained_control {
                render_plan.entries[7].anchor_layer_index = 7;
            }
            let Some(LayerStageCommit::Support(ir)) = run(
                "Layer::Support",
                module,
                selected,
                Some(&current),
                Some(&map),
                Some(&render_plan),
                None,
                None,
            ) else {
                panic!("WASM carrier positive control must render support")
            };
            let wire_chains: BTreeSet<_> = ir
                .entries
                .iter()
                .filter(|entry| entry.object_id == "carrier" && entry.region_id == 70)
                .map(|entry| entry.variant_chain.clone())
                .collect();
            counts.push(wire_chains.len());
            let mut expected = vec![("body-0", vec![], vec![0.0, 20.0])];
            if chained_control {
                expected.push(("body-7", ordered_chain.clone(), vec![140.0]));
            }
            assert_eq!(
                wire_chains,
                expected.iter().map(|(_, paint, _)| paint.clone()).collect(),
                "WASM exact ordered carrier origins survive delivery and commit"
            );
            for (body, paint, footprints) in expected {
                let entry = ir
                    .entries
                    .iter()
                    .find(|entry| entry.body_id == body && entry.variant_chain == paint)
                    .expect("each carrier's exact chain must render its planned geometry");
                assert!(!entry.paths.is_empty(), "WASM nonempty carrier guard");
                let unique_paths: BTreeSet<_> = entry
                    .paths
                    .iter()
                    .map(|path| format!("{:?}", path.points))
                    .collect();
                assert_eq!(
                    unique_paths.len(),
                    entry.paths.len(),
                    "a duplicate wire handle would render the same plan twice"
                );
                for x in &footprints {
                    assert!(entry.paths.iter().any(|path| path
                        .points
                        .iter()
                        .any(|point| (*x..=*x + 10.0).contains(&point.x))));
                }
                for path in &entry.paths {
                    assert!(!path.points.is_empty());
                    assert!(path.points.iter().all(|point| footprints
                        .iter()
                        .any(|x| (*x..=*x + 10.0).contains(&point.x))));
                }
            }
        }
    }
    // An existing *different-chain* slice identity must not exclude an
    // empty-chain carrier, even though object and raw source ID coincide.
    let mut other_chain = current.clone();
    other_chain.regions.push(SlicedRegion {
        object_id: "carrier".into(),
        region_id: 70,
        variant_chain: chain(1),
        ..SlicedRegion::default()
    });
    insert(
        &mut map,
        RegionKey {
            object_id: "carrier".into(),
            region_id: 0,
            ..key(0, chain(1))
        },
        resolved(0.4),
    );
    let different_chain_index = index(
        "Layer::Support",
        &modules[0],
        Some(&other_chain),
        Some(&map),
        Some(&plan),
    );
    let other_selection = different_chain_index.selection_for(7, &id).unwrap();
    counts.push(
        other_selection
            .targets
            .iter()
            .filter(|t| matches!(t.source, LayerTargetSource::SupportCarrier { .. }))
            .count(),
    );
    counts
}

#[test]
fn deduplicated_carrier_preserves_plan_provenance() {
    assert_eq!(
        carrier_witness(false),
        vec![1, 1, 2],
        "AC-R3 deduplicate wire carriers without collapsing plan provenance"
    );
    assert_eq!(
        carrier_witness(true),
        vec![2, 2, 3],
        "AC-R3 three contexts / two exact-chain wire handles on BOTH legs"
    );
}

#[test]
fn missing_selected_perimeter_is_empty_on_both_legs() {
    let id = "com.core.classic-perimeters".to_string();
    let claims = vec!["claim:perimeters".into()];
    let modules = [
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            None,
            &claims,
            config(),
        )
        .with_native_entry(NativeStageEntry::Layer(native_perimeters)),
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            Some(guest("classic-perimeters")),
            &claims,
            config(),
        ),
    ];
    let original = slice(vec![
        region(7_000_002, chain(1), 0.0),
        region(7_000_003, chain(2), 30.0),
    ]);
    let mut map = RegionMapIR::default();
    insert(&mut map, key(7, chain(1)), resolved(0.4));
    insert(&mut map, key(7, chain(2)), resolved(0.6));
    let index = index(
        "Layer::Perimeters",
        &modules[0],
        Some(&original),
        Some(&map),
        None,
    );
    let selected = index.selection_for(7, &id).unwrap();
    assert_eq!(selected.targets.len(), 2);
    let current = slice(vec![region(7_000_003, chain(2), 50.0)]);
    assert!(matches!(&selected.targets[0].source,
        LayerTargetSource::Model { key: k, .. } if k == &key(7_000_002, chain(1))));
    assert!(matches!(&selected.targets[1].source,
        LayerTargetSource::Model { key: k, .. } if k == &key(7_000_003, chain(2))));
    for (leg, module) in [("native", &modules[0]), ("wasm", &modules[1])] {
        let invoke = || {
            run(
                "Layer::Perimeters",
                module,
                selected,
                Some(&current),
                Some(&map),
                None,
                None,
                None,
            )
        };
        let commit = if leg == "wasm" {
            // The independently pinned first selected source is absent now:
            // a captured (layer 7, ordinal 0, contours 0) MUST exist, followed
            // by the current nonempty source. An omitted empty view cannot pass.
            with_wasm_perimeter_delivery(&[0, 1], invoke)
        } else {
            invoke()
        };
        let Some(LayerStageCommit::Perimeters(ir)) = commit else {
            panic!("{leg}: current perimeter geometry required")
        };
        assert_eq!(
            ir.regions.len(),
            1,
            "absent source may not contribute snapshot geometry"
        );
        assert_eq!(ir.global_layer_index, 7);
        assert_eq!(ir.regions[0].object_id, "repair-model");
        assert_eq!(ir.regions[0].region_id, 7_000_003);
        assert_eq!(ir.regions[0].variant_chain, chain(2));
        assert!(!ir.regions[0].walls.is_empty());
        let points: Vec<_> = ir.regions[0]
            .walls
            .iter()
            .flat_map(|w| &w.path.points)
            .collect();
        assert!(!points.is_empty());
        assert!(
            points.iter().all(|p| (49.0..61.0).contains(&p.x)),
            "{leg}: present view must use current, not indexed snapshot geometry"
        );
        if leg == "native" {
            let values = deliveries();
            assert!(
                !values.is_empty(),
                "current geometry delivered before absence check"
            );
            assert_eq!(
                values.len(),
                2,
                "AC-R4 absent selected source must receive an empty view, not omission"
            );
            let absent = values.iter().find(|d| d.chain == chain(1)).unwrap();
            assert_eq!(absent.region, 7_000_002);
            assert_eq!(absent.width, 0.4);
            // Perimeter eligibility is a stage claim, not a held fill role.
            assert_eq!(absent.claims, Vec::<String>::new());
            assert!(absent.polygons.is_empty());
            let present = values.iter().find(|d| d.chain == chain(2)).unwrap();
            assert_eq!(present.region, 7_000_003);
            assert_eq!(present.width, 0.6);
            assert_eq!(present.claims, Vec::<String>::new());
            assert_eq!(present.polygons, vec![square(50.0)]);
        }
    }
    // The classic guest filters empty output buckets; the commit above contains
    // only the present source's current geometry, not an empty postprocess IR.
    // Only the mirrored feature run adds the WASM empty-delivery witness;
    // config/claims stay native-controlled.
}

#[test]
fn versioned_query_resources_load_and_old_components_reject() {
    // W11 OLD-rejection witness: the pre-split fixture (embedded world exports
    // `slicer:layer-infill/infill@1.0.0`, imports UNVERSIONED
    // `slicer:ir-handles/ir-handles`, and carries lightning accessors on the
    // paint view — provenance receipt from the wasm-tools decode) must fail
    // typed instantiation against the NEW `@2.0.0` bindgen root BEFORE run,
    // callback, or output commit. The rejection is the stage's typed interface
    // identity as a whole — neither an export-miss claim alone nor a
    // dual-version adapter is involved.
    let old = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pre-split-layer-infill.component.wasm");
    assert!(
        old.is_file(),
        "incompatible historical component must exist"
    );
    let component = crate::common::wasm_cache::compiled_component_at(&old);
    let id = "com.test.old-query-component".to_string();
    let claims = vec!["claim:sparse-fill".into()];
    let module = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        Some(component),
        &claims,
        config(),
    );
    let current = slice(vec![region(7, vec![], 0.0)]);
    let mut map = RegionMapIR::default();
    let mut cfg = resolved(0.4);
    cfg.sparse_fill_holder = id.clone();
    insert(&mut map, key(7, vec![]), cfg);
    let index = index("Layer::Infill", &module, Some(&current), Some(&map), None);
    let selection = index.selection_for(7, &id).unwrap();
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    // Clear any earlier successful invocation's thread-local evidence.
    LayerStageRunner::last_wasm_mem_sample(&dispatcher);
    // exhaustive: old-component rejection must run through the typed production stage.
    let input = LayerStageInput {
        selection: Some(selection),
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: Some(Arc::new(map)),
        slice: Some(&current),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let error = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".into(),
        &layer(),
        &module,
        input,
    )
    .expect_err("W11 old unversioned component must reject before invocation");
    let slicer_ir::LayerStageError::FatalModule {
        stage_id,
        module_id,
        message,
    } = error
    else {
        panic!("W11 incompatible typed world must be fatal: {error:?}");
    };
    assert_eq!(stage_id, "Layer::Infill");
    assert_eq!(module_id, id);
    eprintln!("W11 typed rejection: {message}");
    // TypedInstantiation precedes resource delivery and call_run; a module
    // error/trap AFTER run would instead carry phase TypedExportCall.
    assert!(
        message.contains("phase TypedInstantiation")
            && message.contains("component imports instance `slicer:ir-handles/ir-handles`")
            && message.contains("a matching implementation was not found in the linker")
            && message
                .contains("module does not export the interface required by stage Layer::Infill")
            && message.contains("slicer:layer-infill/infill@2.0.0#run"),
        "W11 must reject the incompatible world, not fail during run: {message}"
    );
    assert_eq!(
        LayerStageRunner::last_wasm_mem_sample(&dispatcher),
        (0, 0),
        "W11 pre-invocation typed rejection must leave no WASM invocation sample"
    );
}

/// Plan entry for the W14 positive query-resource witness: anchored at the
/// fixture's layer 7, object/region/chain match the one sliced source, so the
/// layer-7 support query must surface every entry.
fn support_query_plan_entry(n: usize) -> SupportPlanEntry {
    // exhaustive: query-view fixture pins every plan field; anchor matches layer 7.
    SupportPlanEntry {
        global_layer_index: 7,
        object_id: "repair-model".into(),
        region_id: 7,
        family_id: "traditional".into(),
        demand_ids: vec![format!("demand-{n}")],
        body_ids: vec![format!("body-{n}")],
        anchor_layer_index: 7,
        anchor_z: slicer_ir::mm_to_units(0.3),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square(n as f32 * 20.0)],
        }],
        skeleton: None,
        capabilities: vec![],
        provenance: vec![format!("producer-{n}")],
        decline_reason: None,
        variant_chain: vec![],
    }
}

#[test]
fn versioned_query_resources_new_guest_runs_support_and_lightning_queries() {
    // W11 positive counterpart to the OLD-rejection witness above: the REBUILT
    // NEW-contract `layer-infill-guest` must typed-instantiate and run through
    // the production dispatch with the W7a `support-query-view` and
    // `lightning-query-view` resources sharing the one invocation's store and
    // config identity. The on-disk test-guest artifact still embeds the OLD
    // pre-split world, so this executes fully only after `cargo xtask
    // build-guests` rebuilds it (W14); the authored body below is the W14
    // assertion set — it is gated, not silently absent.
    let id = "lightning-infill".to_string();
    let claims = vec!["claim:sparse-fill".into()];
    let config = Arc::new(ConfigView::from_map(std::collections::HashMap::from([(
        "infill-spacing".into(),
        ConfigValue::Float(2.0),
    )])));
    let module = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        Some(crate::common::wasm_cache::compiled_guest(
            "layer-infill-guest",
        )),
        &claims,
        config,
    );
    let current = slice(vec![region(7, vec![], 0.0)]);
    let mut map = RegionMapIR::default();
    let mut cfg = resolved(0.8);
    cfg.sparse_fill_holder = id.clone();
    insert(&mut map, key(7, vec![]), cfg);
    let plan = SupportPlanIR {
        entries: vec![support_query_plan_entry(0), support_query_plan_entry(1)],
        ..SupportPlanIR::default()
    };
    let lightning = LightningTreeIR {
        entries: vec![
            // exhaustive: lightning fixture pins every field; chain-empty origin matches the one source.
            LightningTreeEntry {
                object_id: "repair-model".into(),
                global_layer_index: 7,
                region_id: 7,
                tree_edge_segments: vec![
                    [Point2::from_mm(0.0, 0.0), Point2::from_mm(1.0, 0.0)],
                    [Point2::from_mm(2.0, 0.0), Point2::from_mm(3.0, 0.0)],
                ],
                variant_chain: vec![],
            },
        ],
        ..LightningTreeIR::default()
    };
    let selection_index = index("Layer::Infill", &module, Some(&current), Some(&map), None);
    let selection = selection_index.selection_for(7, &id).unwrap();
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    LayerStageRunner::last_wasm_mem_sample(&dispatcher);
    // exhaustive: W14 invocation travels the typed stage with plan+lightning on one call.
    let input = LayerStageInput {
        selection: Some(selection),
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: Some(Arc::new(plan)),
        lightning_tree_ir: Some(Arc::new(lightning)),
        region_map: Some(Arc::new(map)),
        slice: Some(&current),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".into(),
        &layer(),
        &module,
        input,
    )
    .expect("W14: NEW-contract guest must typed-instantiate and run")
    .expect("W14: guest must emit infill output");
    let (initial, _) = LayerStageRunner::last_wasm_mem_sample(&dispatcher);
    assert!(
        initial > 0,
        "W14: guest must really invoke through the NEW typed interface"
    );
    let slicer_ir::LayerStageCommit::Infill(ir) = commit else {
        panic!("W14: Layer::Infill commit required")
    };
    // The lightning query call succeeded through the NEW resource: the guest
    // emits a single witness point whose x is the segment count it read.
    let witness = ir
        .regions
        .iter()
        .flat_map(|r| &r.sparse_infill)
        .flat_map(|p| &p.points)
        .find(|p| (p.width - 137.0).abs() < 1e-6)
        .expect("W14: guest must emit the lightning-tree witness point");
    assert_eq!(
        witness.x, 2.0,
        "W14: both committed tree edges must reach the guest"
    );
    // The support query call succeeded through the NEW resource: the guest's
    // own log carries the per-region support-plan entry count it read.
    let logs = slicer_wasm_host::dispatch::last_log_messages_for_test();
    assert!(
        logs.iter()
            .any(|(_, msg)| msg.contains("support_entries=2")),
        "W14: guest must observe both support-plan entries: {logs:?}"
    );
}

#[test]
fn dragon_curve_migrated_component_typed_load_and_run() {
    let artifact = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/community-modules/dragon-curve/dragon-curve.wasm");
    assert!(
        artifact.is_file(),
        "missing migrated dragon example: {}",
        artifact.display()
    );
    let id = "com.example.dragon-curve".to_string();
    let claims = vec!["claim:sparse-fill".into()];
    let mut values = resolved(0.8).to_config_map();
    values.insert("tiling_depth".into(), ConfigValue::Int(6));
    let module = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        Some(crate::common::wasm_cache::compiled_component_at(&artifact)),
        &claims,
        Arc::new(ConfigView::from_map(values)),
    );
    let mut first = region(7_000_002, chain(1), 0.0);
    first.top_solid_fill = vec![square(40.0)];
    first.top_shell_index = Some(0);
    let current = slice(vec![first, region(7_000_003, chain(2), 20.0)]);
    let mut map = RegionMapIR::default();
    for (tool, width) in [(1, 0.4), (2, 0.6)] {
        let mut cfg = resolved(width);
        cfg.sparse_fill_holder = id.clone();
        cfg.top_fill_holder = "com.test.other".into();
        insert(&mut map, key(7, chain(tool)), cfg);
    }
    let selection_index = index("Layer::Infill", &module, Some(&current), Some(&map), None);
    let selected = selection_index.selection_for(7, &id).unwrap();
    // One production typed-stage invocation; `run` also requires a real WASM
    // memory sample, so compilation or manifest loading alone cannot pass.
    let Some(LayerStageCommit::Infill(ir)) = run(
        "Layer::Infill",
        &module,
        selected,
        Some(&current),
        Some(&map),
        None,
        None,
        None,
    ) else {
        panic!("migrated dragon example must commit infill")
    };
    assert_eq!(ir.regions.len(), 2, "both full painted sources must commit");
    for (tool, width, left) in [(1, 0.4_f32, 0.0_f32), (2, 0.6, 20.0)] {
        let output = ir
            .regions
            .iter()
            .find(|r| r.variant_chain == chain(tool))
            .expect("dragon output must preserve the full painted chain");
        assert_eq!(output.object_id, "repair-model");
        // Raw delivery law: the committed ID is the selected painted source, not its config address.
        assert_eq!(
            output.region_id,
            if tool == 1 { 7_000_002 } else { 7_000_003 }
        );
        assert!(
            output.solid_infill.is_empty(),
            "the sparse-only example must not emit the unheld top-fill role"
        );
        assert!(!output.sparse_infill.is_empty(), "dragon must emit paths");
        for path in &output.sparse_infill {
            assert_eq!(path.role, ExtrusionRole::SparseInfill);
            assert_eq!(path.points.len(), 2, "dragon emits two-point segments");
            for point in &path.points {
                assert!(
                    (point.width - width).abs() < 1e-6,
                    "painted source {tool} must use its own width, not invocation width 0.8"
                );
                assert!((point.z - 0.3).abs() < 1e-6);
                assert!(point.x >= left && point.x <= left + 10.0);
                assert!(point.y >= 0.0 && point.y <= 10.0);
            }
        }
    }
}

#[test]
fn held_roles_host_tool_and_carrier_authority() {
    let id = "com.core.rectilinear-infill".to_string();
    let claims = vec!["claim:top-fill".into(), "claim:sparse-fill".into()];
    let source = resolved(0.8).to_config_map();
    let invocation = Arc::new(ConfigView::from_declared(
        &source,
        source.keys().map(String::as_str),
    ));
    let modules = [
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            None,
            &claims,
            invocation.clone(),
        )
        .with_native_entry(NativeStageEntry::Layer(native_infill)),
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            Some(guest("rectilinear-infill")),
            &claims,
            invocation,
        ),
    ];
    let mut cfg = resolved(0.4);
    cfg.sparse_fill_holder = id.clone();
    cfg.top_fill_holder = id.clone();
    cfg.extensions
        .insert("extruder".into(), ConfigValue::Int(2));
    cfg.extensions
        .insert("private_probe".into(), ConfigValue::Int(99));
    let mut map = RegionMapIR::default();
    insert(&mut map, key(7, vec![]), cfg);
    let mut r = region(7, vec![], 0.0);
    r.top_solid_fill = vec![square(20.0)];
    r.top_shell_index = Some(0);
    let current = slice(vec![r]);
    let selection_index = index(
        "Layer::Infill",
        &modules[0],
        Some(&current),
        Some(&map),
        None,
    );
    let selected = selection_index.selection_for(7, &id).unwrap();
    assert_eq!(selected.targets.len(), 1);
    assert_eq!(selected.targets[0].held_claims, claims);
    assert_eq!(selected.targets[0].resolved_role_tool, Some(2));
    let mut failures = Vec::new();
    for (leg, module) in [("native", &modules[0]), ("wasm", &modules[1])] {
        let Some(LayerStageCommit::Infill(ir)) = run(
            "Layer::Infill",
            module,
            selected,
            Some(&current),
            Some(&map),
            None,
            None,
            None,
        ) else {
            panic!("{leg}: role output required")
        };
        assert_eq!(ir.regions.len(), 1);
        let r = &ir.regions[0];
        assert!(!r.solid_infill.is_empty());
        assert!(!r.sparse_infill.is_empty());
        for p in r.solid_infill.iter().chain(&r.sparse_infill) {
            assert!(!p.points.is_empty());
            if p.tool_index != Some(2) {
                failures.push(format!("{leg}: host tool attribution {:?}", p.tool_index));
            }
        }
        if leg == "native" {
            let values = deliveries();
            assert_eq!(values.len(), 1);
            assert_eq!(values[0].claims, claims);
            assert_eq!(values[0].private, None);
            assert_eq!(values[0].extruder, None);
        } else {
            // Retain the real rectilinear role/tool controls above. A second
            // real single-stage SDK guest echoes its actual config reads and
            // delivered held roles, not a host-side inference from output.
            let mut echo_source = source.clone();
            echo_source.insert("emit_config_witness".into(), ConfigValue::Int(1));
            let echo = CompiledModuleLive::new(
                &id,
                WasmInstancePool::placeholder(),
                Some(crate::common::wasm_cache::compiled_guest(
                    "sdk-layer-infill-guest",
                )),
                &claims,
                Arc::new(ConfigView::from_declared(
                    &echo_source,
                    echo_source.keys().map(String::as_str),
                )),
            );
            // Invocation config is frozen in the index, not re-read from the
            // live module. Build this echo's index with its declared gate.
            let echo_index = index("Layer::Infill", &echo, Some(&current), Some(&map), None);
            let echo_selection = echo_index.selection_for(7, &id).unwrap();
            let Some(LayerStageCommit::Infill(echo_ir)) = run(
                "Layer::Infill",
                &echo,
                echo_selection,
                Some(&current),
                Some(&map),
                None,
                None,
                None,
            ) else {
                panic!("SDK authority echo must commit real guest output")
            };
            assert_eq!(echo_ir.regions.len(), 1);
            let echo_region = &echo_ir.regions[0];
            assert_eq!(echo_region.object_id, "repair-model");
            assert_eq!(echo_region.region_id, 7);
            assert!(echo_region.variant_chain.is_empty());
            let echoes: Vec<_> = echo_region
                .sparse_infill
                .iter()
                .filter(|p| p.points.first().is_some_and(|p| p.flow_factor == 9.0))
                .collect();
            assert_eq!(echoes.len(), 1, "one decoded authority witness required");
            let echo_path = echoes[0];
            assert_eq!(echo_path.points.len(), 3);
            let config_reads = &echo_path.points[1];
            let authority_reads = &echo_path.points[2];
            assert_eq!(config_reads.flow_factor, 1.0, "region config reached SDK");
            assert_eq!(config_reads.x, 0.4, "declared region width is readable");
            assert_eq!(config_reads.y, 0.8, "invocation width stays separate");
            // The raw `get` presence bits independently prove None, rather
            // than only a typed accessor's possible wrong-type fallback.
            assert_eq!(config_reads.z, 0.0, "guest private_probe get is None");
            assert_eq!(authority_reads.y, 0.0, "guest extruder get is None");
            assert_eq!(authority_reads.z, -1.0, "extruder get_int denied sentinel");
            assert_eq!(
                authority_reads.width, -1.0,
                "private get_int denied sentinel"
            );
            assert_eq!(authority_reads.x, 7.0, "echo keeps the source identity");
            assert_eq!(
                authority_reads.flow_factor, 3.0,
                "both held roles reached SDK"
            );
            assert_eq!(
                echo_path.tool_index,
                Some(2),
                "host assigns tool despite denied read"
            );
            // ConfigView deliberately conflates undeclared and absent keys.
            // Both were supplied as resolved Int values (2 and 99) above;
            // the witness claims default-deny visibility, not a denial reason.
        }
    }
    // Run the same carrier exclusion/authority matrix before asserting tool loss.
    let carrier_counts = carrier_witness(false);
    if carrier_counts != vec![1, 1, 2] {
        failures.push(format!(
            "carrier wire/different-chain selection counts: {carrier_counts:?}, expected [1, 1, 2]"
        ));
    }
    assert!(
        failures.is_empty(),
        "AC-R11 host-only tool authority must reach output attribution: {failures:?}"
    );
}

#[test]
fn historical_width_witness() {
    // AC-1's canonical painted source IDs and independent widths, not the
    // invocation width. Keep both transport outputs recorded before assertions.
    let id = "com.core.rectilinear-infill".to_string();
    let claims = vec!["claim:sparse-fill".into()];
    let modules = [
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            None,
            &claims,
            historical_invocation_config(),
        )
        .with_native_entry(NativeStageEntry::Layer(native_infill)),
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            Some(guest("rectilinear-infill")),
            &claims,
            historical_invocation_config(),
        ),
    ];
    let current = SliceIR {
        z: 0.2,
        regions: [1, 2]
            .into_iter()
            .map(|tool| SlicedRegion {
                object_id: "variant-cube".into(),
                region_id: 7_000_000 + u64::from(tool) + 1,
                variant_chain: chain(tool),
                polygons: vec![square(0.0)],
                sparse_infill_area: vec![square(0.0)],
                effective_layer_height: 0.2,
                ..SlicedRegion::default()
            })
            .collect(),
        ..SliceIR::default()
    };
    let mut map = RegionMapIR::default();
    for (tool, width) in [(1, 0.4), (2, 0.6)] {
        let cfg = ResolvedConfig {
            line_width: width,
            infill_density: 0.2,
            sparse_fill_holder: "rectilinear-infill".into(),
            top_fill_holder: "com.test.other".into(),
            bottom_fill_holder: "com.test.other".into(),
            bridge_fill_holder: "com.test.other".into(),
            ..ResolvedConfig::default()
        };
        insert(
            &mut map,
            RegionKey {
                global_layer_index: 0,
                object_id: "variant-cube".into(),
                region_id: 7,
                variant_chain: chain(tool),
            },
            cfg,
        );
    }
    let index = index(
        "Layer::Infill",
        &modules[0],
        Some(&current),
        Some(&map),
        None,
    );
    let selected = index.selection_for(0, &id).unwrap();
    let mut observed = Vec::new();
    for (leg, module) in [("native", &modules[0]), ("wasm", &modules[1])] {
        let Some(LayerStageCommit::Infill(ir)) = run(
            "Layer::Infill",
            module,
            selected,
            Some(&current),
            Some(&map),
            None,
            None,
            None,
        ) else {
            panic!("{leg}: nonempty width witness commit")
        };
        assert_eq!(ir.regions.len(), 2);
        for tool in [1, 2] {
            let r = ir
                .regions
                .iter()
                .find(|r| r.variant_chain == chain(tool))
                .unwrap();
            // Raw delivery law: the committed ID is the selected painted source, not its config address.
            assert_eq!(r.region_id, if tool == 1 { 7_000_002 } else { 7_000_003 });
            assert!(!r.sparse_infill.is_empty());
            let widths: Vec<_> = r
                .sparse_infill
                .iter()
                .flat_map(|p| &p.points)
                .map(|p| p.width)
                .collect();
            assert!(!widths.is_empty());
            observed.push((leg, tool, widths));
        }
    }
    eprintln!("H1 actual delivered widths (invocation=0.8): {observed:?}");
    for (leg, tool, widths) in observed {
        let expected = if tool == 1 { 0.4_f32 } else { 0.6_f32 };
        assert!(
            widths.iter().all(|width| (*width - expected).abs() < 1e-6),
            "H1 {leg}: exact painted source {tool} width {expected}, actual {widths:?}"
        );
    }
}
