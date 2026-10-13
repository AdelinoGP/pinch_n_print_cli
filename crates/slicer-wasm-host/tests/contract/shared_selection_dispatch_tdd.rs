//! Family-B witnesses through the public selection index and ordinary runner.
//! Geometry assertions protect source isolation, not a captured path baseline.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks, AnchoredGeometryContract,
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer,
    LayerStageCommit, LoopType, OrderedEventCollection, PaintValue, Point2, Point3WithWidth,
    Polygon, RaftPlan, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
    SupportPlanEntry, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion, WallBoundaryType,
    WallLoop, WidthProfile,
};
use slicer_sdk::builders::{InfillOutputBuilder, PerimeterOutputBuilder, SupportOutputBuilder};
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::native::{
    NativeLayerRequest, NativeLayerResponse, NativeStageEntry, NativeSupportOutput,
};
use slicer_sdk::PaintRegionLayerView;
use slicer_wasm_host::host::layer::slicer::config::config_types::HostConfigView;
use slicer_wasm_host::host::layer::slicer::ir_handles::ir_handles::HostSliceRegionView;
use slicer_wasm_host::host::{config_view_to_data, HostExecutionContextBuilder};
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelection, LayerSelectionIndex, LayerSelectionIndexInput,
    LayerSelectionModuleInput, LayerTargetSource, STAGES,
};
use slicer_wasm_host::{
    binding::LayerStageInput, CompiledModuleLive, LayerStageRunner, WasmInstancePool,
    WasmRuntimeDispatcher,
};
use wasmtime::component::Resource;

#[derive(Debug, Clone)]
struct Delivered {
    object: String,
    region: u64,
    chain: Vec<(String, PaintValue)>,
    width: f64,
    polygons: Vec<ExPolygon>,
    claims: Vec<String>,
}

thread_local! {
    static DELIVERED: RefCell<Vec<Delivered>> = const { RefCell::new(Vec::new()) };
    static ANCHORED_CALLS: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
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

// Required classic-perimeters / traditional-support reads, grounded in their
// LayerModule::from_config and run_perimeters/run_support implementations.
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
    ] {
        extensions.insert(key.into(), ConfigValue::Bool(false));
    }
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

fn config(width: f64) -> Arc<ConfigView> {
    Arc::new(ConfigView::from_map(resolved(width).to_config_map()))
}

fn production_guest(name: &str) -> Arc<slicer_wasm_host::WasmComponent> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../modules/core-modules/{name}/{name}.wasm"));
    assert!(
        path.is_file(),
        "missing production guest: {}",
        path.display()
    );
    crate::common::wasm_cache::compiled_component_at(&path)
}

fn selection_index(
    stage: &str,
    layer: &GlobalLayer,
    module: &CompiledModuleLive<'_>,
    slice: Option<&SliceIR>,
    map: Option<&RegionMapIR>,
    plan: Option<&SupportPlanIR>,
) -> LayerSelectionIndex {
    // exhaustive: borrowed binding explicitly supplies every selection authority.
    let binding = LayerSelectionModuleInput {
        stage_id: stage,
        module_id: module.module_id,
        claims: module.claims,
        paint_semantics: None,
        invocation_config: &module.config_view,
    };
    // exhaustive: whole-print fixture pins all committed product channels.
    build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: std::slice::from_ref(layer),
        modules: &[binding],
        slices: slice.map(std::slice::from_ref).unwrap_or(&[]),
        region_map: map,
        support_plan: plan,
        perimeters: &[],
    })
    .expect("public whole-print selection construction")
}

fn run(
    stage: &str,
    layer: &GlobalLayer,
    module: &CompiledModuleLive<'_>,
    selection: &LayerSelection,
    slice: Option<&SliceIR>,
    map: Option<&RegionMapIR>,
    plan: Option<&SupportPlanIR>,
) -> Option<LayerStageCommit> {
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    // Clear thread-local evidence so no earlier WASM call can satisfy this one.
    LayerStageRunner::last_wasm_mem_sample(&dispatcher);
    // exhaustive: runner borrow envelope has no Default; no hidden fixture products.
    let input = LayerStageInput {
        selection: Some(selection),
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: plan.cloned().map(Arc::new),
        lightning_tree_ir: None,
        region_map: map.cloned().map(Arc::new),
        slice,
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let commit = LayerStageRunner::run_stage(&dispatcher, &stage.into(), layer, module, input)
        .expect("ordinary selection-backed production dispatch");
    if module.native_entry.is_none() {
        let (initial, _) = LayerStageRunner::last_wasm_mem_sample(&dispatcher);
        assert!(
            initial > 0,
            "real guest invoked, including no-output controls"
        );
        assert_eq!(LayerStageRunner::last_wasm_mem_sample(&dispatcher), (0, 0));
    }
    commit
}

fn record(request: &NativeLayerRequest) {
    DELIVERED.with(|records| {
        *records.borrow_mut() = request
            .regions
            .iter()
            .map(|region| Delivered {
                object: region.object_id().clone(),
                region: *region.region_id(),
                chain: region.variant_chain().to_vec(),
                width: region
                    .config()
                    .expect("delivered region config")
                    .get_float("line_width")
                    .expect("numeric delivered width"),
                polygons: region.polygons().to_vec(),
                claims: region.held_claims().to_vec(),
            })
            .collect();
    });
}

fn path(points: Vec<Point3WithWidth>, role: ExtrusionRole) -> ExtrusionPath3D {
    assert!(
        !points.is_empty(),
        "native emitting entry requires real input points"
    );
    // exhaustive: no Default for extrusion paths; witness metadata is explicit.
    ExtrusionPath3D {
        points,
        role,
        speed_factor: 1.0,
        tool_index: None,
        order_lock: None,
    }
}

fn points(polygon: &ExPolygon, z: f32, width: f32) -> Vec<Point3WithWidth> {
    polygon
        .contour
        .points
        .iter()
        .map(|p| Point3WithWidth {
            x: slicer_ir::units_to_mm(p.x),
            y: slicer_ir::units_to_mm(p.y),
            z,
            width,
            flow_factor: 1.0,
            ..Point3WithWidth::default()
        })
        .collect()
}

fn empty_response() -> NativeLayerResponse {
    // exhaustive: native recording entries explicitly choose their stage output slot.
    NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    }
}

fn native_perimeters(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    record(request);
    let mut output = PerimeterOutputBuilder::new();
    for region in &request.regions {
        output.begin_region(
            region.object_id(),
            *region.region_id(),
            region.variant_chain(),
        );
        let width = region.config().unwrap().get_float("line_width").unwrap() as f32;
        for polygon in region.polygons() {
            let path = path(points(polygon, region.z(), width), ExtrusionRole::OuterWall);
            output
                // exhaustive: native witness wall explicitly defines boundary/width metadata.
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
                .expect("emit native perimeter witness");
        }
    }
    Ok(NativeLayerResponse {
        perimeters: Some(output),
        ..empty_response()
    })
}

fn native_roles(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    record(request);
    let mut output = InfillOutputBuilder::new();
    for region in &request.regions {
        output.begin_region(
            region.object_id(),
            *region.region_id(),
            region.variant_chain(),
        );
        let width = region.config().unwrap().get_float("line_width").unwrap() as f32;
        for polygon in region.polygons() {
            if region.held_claims().iter().any(|c| c == "claim:top-fill") {
                output
                    .push_solid_path(path(
                        points(polygon, region.z(), width),
                        ExtrusionRole::TopSolidInfill,
                    ))
                    .expect("emit held top role");
            }
            if region
                .held_claims()
                .iter()
                .any(|c| c == "claim:sparse-fill")
            {
                output
                    .push_sparse_path(path(
                        points(polygon, region.z(), width),
                        ExtrusionRole::SparseInfill,
                    ))
                    .expect("emit held sparse role");
            }
        }
    }
    Ok(NativeLayerResponse {
        infill: Some(output),
        ..empty_response()
    })
}

#[test]
fn merged_sources_and_multiple_roles() {
    let layer = layer();
    let id = "com.core.classic-perimeters".to_string();
    let claims = vec!["claim:perimeters".into()];
    let wasm = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        Some(production_guest("classic-perimeters")),
        &claims,
        config(0.8),
    );
    let native = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        config(0.8),
    )
    .with_native_entry(NativeStageEntry::Layer(native_perimeters));
    let keys = [1, 2].map(|tool| RegionKey {
        global_layer_index: 7,
        object_id: "merged-model".into(),
        region_id: 7,
        variant_chain: chain(tool),
    });
    let raw_ids = [7_000_002, 7_000_003];
    // Repaired stored selection identity is SOURCE space; `keys` stay the
    // exact derived map-space config addresses seeded into the map below.
    let source_keys = [
        RegionKey {
            region_id: raw_ids[0],
            ..keys[0].clone()
        },
        RegionKey {
            region_id: raw_ids[1],
            ..keys[1].clone()
        },
    ];
    let child_id = slicer_ir::modifier_sub_region_id(raw_ids[0], "modifier", &[]);
    let child_key = RegionKey {
        region_id: child_id,
        ..keys[0].clone()
    };
    let slice = SliceIR {
        global_layer_index: 7,
        z: 0.3,
        regions: vec![
            SlicedRegion {
                object_id: "merged-model".into(),
                region_id: raw_ids[0],
                variant_chain: chain(1),
                polygons: vec![square(0.0)],
                effective_layer_height: 0.2,
                ..SlicedRegion::default()
            },
            SlicedRegion {
                object_id: "merged-model".into(),
                region_id: raw_ids[1],
                variant_chain: chain(2),
                polygons: vec![square(30.0)],
                effective_layer_height: 0.2,
                ..SlicedRegion::default()
            },
            SlicedRegion {
                object_id: "merged-model".into(),
                region_id: child_id,
                variant_chain: chain(1),
                polygons: vec![square(15.0)],
                effective_layer_height: 0.2,
                ..SlicedRegion::default()
            },
        ],
        ..SliceIR::default()
    };
    let mut map = RegionMapIR::default();
    for (key, width) in [(&keys[0], 0.4), (&keys[1], 0.6), (&child_key, 1.2)] {
        let mut cfg = resolved(width);
        cfg.top_fill_holder = "com.test.multiple-roles".into();
        cfg.sparse_fill_holder = "com.test.multiple-roles".into();
        let config = map.intern_config(cfg);
        map.entries.insert(
            key.clone(),
            RegionPlan {
                config,
                ..RegionPlan::default()
            },
        );
    }
    let index = selection_index(
        "Layer::Perimeters",
        &layer,
        &wasm,
        Some(&slice),
        Some(&map),
        None,
    );
    let selection = index.selection_for(7, &id).expect("bound perimeters");
    assert_eq!(
        selection.targets.len(),
        2,
        "exactly two model source targets"
    );
    for (key, members, width) in [
        (
            &source_keys[0],
            vec![source_keys[0].clone(), child_key.clone()],
            0.4,
        ),
        (&source_keys[1], vec![source_keys[1].clone()], 0.6),
    ] {
        let target = selection
            .targets
            .iter()
            .find(|t| {
                matches!(&t.source,
            LayerTargetSource::Model { key: actual, .. } if actual == key)
            })
            .expect("exact source target");
        assert_eq!(
            target.source,
            LayerTargetSource::Model {
                key: key.clone(),
                members
            }
        );
        assert_eq!(
            target.config.get_float("line_width"),
            Some(width),
            "base, never modifier config"
        );
    }
    for (leg, module) in [("native", &native), ("wasm", &wasm)] {
        let Some(LayerStageCommit::Perimeters(ir)) = run(
            "Layer::Perimeters",
            &layer,
            module,
            selection,
            Some(&slice),
            Some(&map),
            None,
        ) else {
            panic!("{leg}: perimeter commit required")
        };
        assert_eq!(
            ir.regions.len(),
            2,
            "{leg}: both full source identities survive"
        );
        for tool in [1, 2] {
            let region = ir
                .regions
                .iter()
                .find(|r| r.variant_chain == chain(tool))
                .expect("committed full chain");
            assert_eq!(region.object_id, "merged-model");
            assert!(!region.walls.is_empty(), "{leg}: nonempty walls");
            for wall in &region.walls {
                assert!(!wall.path.points.is_empty(), "{leg}: nonempty wall points");
            }
            let xs: Vec<_> = region
                .walls
                .iter()
                .flat_map(|w| &w.path.points)
                .map(|p| p.x)
                .collect();
            if tool == 1 {
                assert!(
                    xs.iter().any(|x| (15.0..=25.0).contains(x)),
                    "{leg}: modifier geometry reaches variant 1"
                );
                assert!(
                    xs.iter().all(|x| *x < 26.0),
                    "{leg}: sibling geometry stays isolated"
                );
            } else {
                assert!(
                    xs.iter().all(|x| *x > 29.0),
                    "{leg}: modifier cannot enter variant 2"
                );
            }
        }
    }
    DELIVERED.with(|records| {
        let records = records.borrow();
        assert_eq!(
            records.len(),
            2,
            "native receives merged sources, not child as third source"
        );
        for (tool, width, raw_id) in [(1, 0.4, raw_ids[0]), (2, 0.6, raw_ids[1])] {
            let r = records
                .iter()
                .find(|r| r.chain == chain(tool))
                .expect("delivered full chain");
            assert_eq!(
                (r.object.as_str(), r.region, r.width),
                ("merged-model", raw_id, width)
            );
            assert!(!r.polygons.is_empty());
            let xs: Vec<_> = r
                .polygons
                .iter()
                .flat_map(|p| &p.contour.points)
                .map(|p| slicer_ir::units_to_mm(p.x))
                .collect();
            if tool == 1 {
                assert!(xs.contains(&15.0), "native receives child geometry");
            } else {
                assert!(xs.iter().all(|x| *x >= 30.0));
            }
        }
    });

    let fill_id = "com.test.multiple-roles".to_string();
    let fill_claims = vec!["claim:top-fill".into(), "claim:sparse-fill".into()];
    let fill = CompiledModuleLive::new(
        &fill_id,
        WasmInstancePool::placeholder(),
        None,
        &fill_claims,
        config(0.8),
    )
    .with_native_entry(NativeStageEntry::Layer(native_roles));
    let fill_index = selection_index(
        "Layer::Infill",
        &layer,
        &fill,
        Some(&slice),
        Some(&map),
        None,
    );
    let fill_selection = fill_index.selection_for(7, &fill_id).unwrap();
    let target = fill_selection
        .targets
        .iter()
        .find(|t| {
            matches!(&t.source,
        LayerTargetSource::Model { key, .. } if key == &source_keys[0])
        })
        .unwrap();
    assert_eq!(
        target.held_claims, fill_claims,
        "all held roles survive projection"
    );
    let Some(LayerStageCommit::Infill(ir)) = run(
        "Layer::Infill",
        &layer,
        &fill,
        fill_selection,
        Some(&slice),
        Some(&map),
        None,
    ) else {
        panic!("separate infill call must emit both roles")
    };
    DELIVERED.with(|records| {
        let records = records.borrow();
        let base = records
            .iter()
            .find(|r| r.region == 7 && r.chain == chain(1))
            .unwrap();
        assert_eq!(
            base.claims, fill_claims,
            "native receives both held roles, not just first"
        );
    });
    let base = ir
        .regions
        .iter()
        .find(|r| r.region_id == 7 && r.variant_chain == chain(1))
        .unwrap();
    assert!(!base.solid_infill.is_empty());
    assert!(!base.sparse_infill.is_empty());
    for p in base.solid_infill.iter().chain(&base.sparse_infill) {
        assert!(!p.points.is_empty());
    }
}

fn support_entry(index: usize, global: i32, anchor: u32) -> SupportPlanEntry {
    // exhaustive: support provenance fixture has no Default; preserve every plan field.
    SupportPlanEntry {
        global_layer_index: global,
        object_id: format!("carrier-{index}"),
        region_id: 70 + index as u64,
        family_id: "traditional".into(),
        demand_ids: vec![format!("demand-{index}")],
        body_ids: vec![format!("body-{index}")],
        anchor_layer_index: anchor,
        anchor_z: slicer_ir::mm_to_units(0.3),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square(0.0)],
        }],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["planner-fixture".into()],
        decline_reason: None,
        variant_chain: vec![], // genuinely chain-empty plan source (AC-B2 transport handle)
    }
}

fn native_support(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    record(request);
    assert_eq!(request.config.get_float("line_width"), Some(0.8));
    let paint = request.paint.as_ref().expect("support paint plan view");
    let mut output = SupportOutputBuilder::new();
    for region in &request.regions {
        for entry in paint.support_plan_entries_for(
            region.object_id(),
            *region.region_id(),
            region.variant_chain(),
        ) {
            if entry.anchor_layer_index != request.layer_index || entry.decline_reason.is_some() {
                continue;
            }
            output.begin_region(
                region.object_id(),
                *region.region_id(),
                region.variant_chain(),
            );
            for role in &entry.roles {
                for polygon in &role.regions {
                    output
                        .push_support_path(path(
                            points(polygon, region.z(), 0.8),
                            ExtrusionRole::SupportMaterial,
                        ))
                        .expect("native support positive control emits paths");
                }
            }
        }
    }
    Ok(NativeLayerResponse {
        support: Some(NativeSupportOutput {
            output,
            collection: LayerCollectionBuilder::new(),
        }),
        ..empty_response()
    })
}

fn support_modules<'a>(id: &'a String, claims: &'a [String]) -> [CompiledModuleLive<'a>; 2] {
    [
        CompiledModuleLive::new(
            id,
            WasmInstancePool::placeholder(),
            None,
            claims,
            config(0.8),
        )
        .with_native_entry(NativeStageEntry::Layer(native_support)),
        CompiledModuleLive::new(
            id,
            WasmInstancePool::placeholder(),
            Some(production_guest("traditional-support")),
            claims,
            config(0.8),
        ),
    ]
}

fn carrier_selection_delivery_fixture() {
    let layer = layer();
    let id = "com.core.traditional-support".to_string();
    let claims = vec!["support-family:traditional".into()];
    let modules = support_modules(&id, &claims);
    let mut wrong_family = support_entry(2, 7, 7);
    wrong_family.family_id = "tree".into();
    let mut declined = support_entry(3, 7, 7);
    declined.decline_reason = Some(slicer_ir::SupportPlanDeclineReason::DeclinedPolicy);
    let plan = SupportPlanIR {
        entries: vec![
            support_entry(0, 7, 3),
            support_entry(1, 3, 7),
            wrong_family,
            declined,
        ],
        ..SupportPlanIR::default()
    };
    let index = selection_index(
        "Layer::Support",
        &layer,
        &modules[0],
        None,
        None,
        Some(&plan),
    );
    let selected = index.selection_for(7, &id).unwrap();
    assert!(selected.invoke);
    assert_eq!(
        selected.targets.len(),
        1,
        "only global-7/anchor-3 entry 0 is eligible"
    );
    let target = &selected.targets[0];
    assert_eq!(
        target.source,
        LayerTargetSource::SupportCarrier {
            entry_index: 0,
            object_id: "carrier-0".into(),
            region_id: 70,
            variant_chain: vec![],
            family: "traditional".into(),
            demand_ids: vec!["demand-0".into()],
            body_ids: vec!["body-0".into()],
            provenance: vec!["planner-fixture".into()],
            anchor_layer_index: 3,
        }
    );
    assert_eq!(target.config.get_float("line_width"), Some(0.8));
    assert!(selected.slice_regions.is_empty());
    assert!(
        selected.perimeter_configs.is_empty(),
        "no model authority invented for carrier"
    );
    for module in &modules {
        assert!(
            run(
                "Layer::Support",
                &layer,
                module,
                selected,
                None,
                None,
                Some(&plan)
            )
            .is_none(),
            "mismatched anchor is delivery/config evidence, not a support-rendering control"
        );
    }
    DELIVERED.with(|records| {
        let records = records.borrow();
        assert_eq!(
            records.len(),
            1,
            "native slice=None still delivers only eligible carrier"
        );
        assert_eq!(
            (
                records[0].object.as_str(),
                records[0].region,
                records[0].width
            ),
            ("carrier-0", 70, 0.8)
        );
        assert!(records[0].chain.is_empty());
        assert!(
            records[0].polygons.is_empty(),
            "carrier is a transport handle, not model geometry"
        );
    });

    // Exercise the public WASM host config accessor on the selected carrier's
    // identity handle. No model map entry is present; authority is invocation.
    let mut ctx = HostExecutionContextBuilder::new(&id, 0.3, 0.2).build();
    ctx.set_default_config_fields(config_view_to_data(&selected.invocation_config).fields);
    let carrier = SlicedRegion {
        object_id: "carrier-0".into(),
        region_id: 70,
        ..SlicedRegion::default()
    };
    let handle = ctx
        .push_slice_region(slicer_wasm_host::marshal::sliced_region_to_data(
            &carrier,
            0.3,
            target.held_claims.clone(),
            None,
            7,
        ))
        .unwrap();
    let config_handle =
        HostSliceRegionView::config(&mut ctx, Resource::new_own(handle.rep())).unwrap();
    assert_eq!(
        HostConfigView::get_float(
            &mut ctx,
            Resource::new_own(config_handle.rep()),
            "line_width".into()
        )
        .unwrap(),
        Some(0.8)
    );
    // The module-facing support view independently retains anchor-layer
    // filtering: the eligible carrier-0 plan entry is anchored at layer 3 and
    // must not surface through a layer-7 view, while the anchor-7 carrier-1
    // entry does.
    let view = PaintRegionLayerView::new(7).with_support_plan(Arc::new(plan));
    assert!(
        view.support_plan_entries_for("carrier-0", 70, &[])
            .is_empty(),
        "support view independently retains anchor-layer filtering"
    );
    assert!(!view
        .support_plan_entries_for("carrier-1", 71, &[])
        .is_empty());
}

fn support_rendering_positive_control() {
    let layer = layer();
    let id = "com.core.traditional-support".to_string();
    let claims = vec!["support-family:traditional".into()];
    let modules = support_modules(&id, &claims);
    let slice = SliceIR {
        global_layer_index: 7,
        z: 0.3,
        ..SliceIR::default()
    };
    let plan = SupportPlanIR {
        entries: vec![support_entry(0, 7, 7)],
        ..SupportPlanIR::default()
    };
    let index = selection_index(
        "Layer::Support",
        &layer,
        &modules[0],
        Some(&slice),
        None,
        Some(&plan),
    );
    let selected = index.selection_for(7, &id).unwrap();
    assert_eq!(selected.targets.len(), 1);
    assert!(matches!(
        selected.targets[0].source,
        LayerTargetSource::SupportCarrier {
            entry_index: 0,
            anchor_layer_index: 7,
            ..
        }
    ));
    for (leg, module) in ["native", "wasm"].into_iter().zip(&modules) {
        let Some(LayerStageCommit::Support(ir)) = run(
            "Layer::Support",
            &layer,
            module,
            selected,
            Some(&slice),
            None,
            Some(&plan),
        ) else {
            panic!("{leg}: on-grid support paths required")
        };
        assert!(
            !ir.entries.is_empty(),
            "{leg}: support output must have entries"
        );
        for entry in &ir.entries {
            assert_eq!(entry.family_id, "traditional");
            assert_eq!(entry.body_id, "body-0");
            assert!(!entry.paths.is_empty(), "{leg}: actual support paths");
            for path in &entry.paths {
                assert!(!path.points.is_empty(), "{leg}: actual support points");
            }
        }
    }
}

fn native_raft(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    let paint = request.paint.as_ref().unwrap();
    assert!(paint.is_raft(), "native observes explicit raft flag");
    assert_eq!(paint.raft_plan(), Some(&raft_plan()));
    assert_eq!(request.config.get_float("line_width"), Some(0.8));
    let mut output = InfillOutputBuilder::new();
    let region = request
        .regions
        .first()
        .expect("existing raft witness region");
    output.begin_region(
        region.object_id(),
        *region.region_id(),
        region.variant_chain(),
    );
    output
        .push_sparse_path(path(
            points(&square(0.0), region.z(), 0.8),
            ExtrusionRole::SparseInfill,
        ))
        .unwrap();
    Ok(NativeLayerResponse {
        infill: Some(output),
        ..empty_response()
    })
}

fn raft_plan() -> RaftPlan {
    RaftPlan {
        raft_layers: 4,
        raft_first_layer_density: 0.75,
        base_raft_layers: 2,
        interface_raft_layers: 1,
    }
}

fn raft_positive_control() {
    // Same layer-infill-guest / empty-polygon model handle as the existing
    // raft_plan_read_accessor_tdd::run_wasm_guest witness, with invocation 0.8.
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        is_raft: true,
        ..GlobalLayer::default()
    };
    let id = "com.test.raft-infill".to_string();
    let claims = vec!["claim:sparse-fill".into()];
    let modules = [
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            None,
            &claims,
            config(0.8),
        )
        .with_native_entry(NativeStageEntry::Layer(native_raft)),
        CompiledModuleLive::new(
            &id,
            WasmInstancePool::placeholder(),
            Some(crate::common::wasm_cache::compiled_guest(
                "layer-infill-guest",
            )),
            &claims,
            config(0.8),
        ),
    ];
    let slice = SliceIR {
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "raft-fixture".into(),
            region_id: 3,
            effective_layer_height: 0.2,
            ..SlicedRegion::default()
        }],
        ..SliceIR::default()
    };
    let mut map = RegionMapIR::default();
    let config = map.intern_config(ResolvedConfig {
        sparse_fill_holder: id.clone(),
        ..resolved(0.8)
    });
    map.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "raft-fixture".into(),
            region_id: 3,
            variant_chain: vec![],
        },
        RegionPlan {
            config,
            ..RegionPlan::default()
        },
    );
    let plan = SupportPlanIR {
        raft_plan: Some(raft_plan()),
        ..SupportPlanIR::default()
    };
    let index = selection_index(
        "Layer::Infill",
        &layer,
        &modules[0],
        Some(&slice),
        Some(&map),
        Some(&plan),
    );
    let selected = index.selection_for(0, &id).unwrap();
    let raft = selected
        .targets
        .iter()
        .find(|t| {
            t.source
                == LayerTargetSource::Raft {
                    global_layer_index: 0,
                }
        })
        .expect("explicit non-region raft context");
    assert_eq!(raft.config.get_float("line_width"), Some(0.8));
    for (leg, module) in ["native", "wasm"].into_iter().zip(&modules) {
        let Some(LayerStageCommit::Infill(ir)) = run(
            "Layer::Infill",
            &layer,
            module,
            selected,
            Some(&slice),
            Some(&map),
            Some(&plan),
        ) else {
            panic!("{leg}: raft witness output required")
        };
        let paths: Vec<_> = ir.regions.iter().flat_map(|r| &r.sparse_infill).collect();
        assert!(!paths.is_empty(), "{leg}: committed raft witness paths");
        for path in &paths {
            assert!(
                !path.points.is_empty(),
                "{leg}: committed raft witness points"
            );
        }
        if leg == "wasm" {
            for tag in [240.0, 241.0, 242.0] {
                assert!(
                    paths
                        .iter()
                        .any(|p| p.points.iter().any(|point| point.width == tag)),
                    "guest observed explicit raft flag and print-wide raft plan: tag {tag}"
                );
            }
        }
    }
}

#[test]
fn non_region_contexts_use_invocation_config() {
    carrier_selection_delivery_fixture();
    support_rendering_positive_control();
    raft_positive_control();
}

fn native_anchored(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    assert_eq!(request.layer_index, 7);
    assert_eq!(request.stage_export, "Layer::AnchoredEvents");
    assert!(request.regions.is_empty());
    assert!(request.config.get("private_probe").is_none());
    let enabled = request
        .config
        .get_int("anchored_event_count")
        .expect("actual module switch");
    ANCHORED_CALLS.with(|calls| calls.borrow_mut().push(enabled));
    let mut builder = LayerCollectionBuilder::new();
    if enabled != 0 {
        let events = [
            AnchoredGeometryContract::Planar { z: 3000 },
            AnchoredGeometryContract::ZSpanning {
                min_z: 3000,
                max_z: 5000,
            },
        ]
        .into_iter()
        .enumerate()
        .map(|(id, geometry)| {
            // exhaustive: mirrors real anchored guest's two enabled proposals.
            AnchoredEntity {
                local_id: id as u64,
                anchor_global_layer_index: 7,
                geometry,
                input_capabilities: vec!["support.plan".into()],
                output_capabilities: vec!["extrusion.paths".into(), "cooling.account".into()],
                provenance: AnchoredEntityProvenance {
                    requesting_feature: "same-z-support".into(),
                    source_plan_entry: "plan-entry-4".into(),
                },
                path_points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 1.0,
                        z: 0.3,
                        width: 0.45,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 2.0,
                        y: 2.0,
                        z: 0.3,
                        width: 0.45,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                ],
                role: ExtrusionRole::SupportMaterial,
            }
        })
        .collect();
        builder
            .set_anchored_event_collection(OrderedEventCollection {
                anchor_global_layer_index: 7,
                events,
                runtime_hooks: AnchoredEventRuntimeHooks {
                    optimize_paths: false,
                    account_cooling: true,
                    account_time: false,
                },
            })
            .unwrap();
    }
    Ok(NativeLayerResponse {
        anchored_events: Some(builder),
        ..empty_response()
    })
}

#[test]
fn anchored_stage_uses_actual_module_config() {
    let layer = layer();
    let id = "com.test.anchored-events-roundtrip".to_string();
    let claims = vec![];
    ANCHORED_CALLS.with(|calls| calls.borrow_mut().clear());
    for enabled in [1, 0] {
        let resolved = ResolvedConfig {
            extensions: BTreeMap::from([
                ("anchored_event_count".into(), ConfigValue::Int(enabled)),
                ("private_probe".into(), ConfigValue::Int(99)),
            ]),
            ..ResolvedConfig::default()
        };
        let config = Arc::new(
            slicer_scheduler::execution_plan::project_declared_config_view(
                &resolved,
                ["anchored_event_count"],
            ),
        );
        let modules = [
            CompiledModuleLive::new(
                &id,
                WasmInstancePool::placeholder(),
                None,
                &claims,
                config.clone(),
            )
            .with_native_entry(NativeStageEntry::Layer(native_anchored)),
            CompiledModuleLive::new(
                &id,
                WasmInstancePool::placeholder(),
                Some(crate::common::wasm_cache::compiled_guest(
                    "anchored-events-roundtrip-guest",
                )),
                &claims,
                config,
            ),
        ];
        let index = selection_index(
            "Layer::AnchoredEvents",
            &layer,
            &modules[0],
            None,
            None,
            None,
        );
        let selected = index.selection_for(7, &id).unwrap();
        assert!(selected.invoke, "switch controls output, not invocation");
        assert_eq!(selected.targets.len(), 1);
        assert_eq!(
            selected.targets[0].source,
            LayerTargetSource::AnchoredStage {
                anchor_layer_index: 7
            }
        );
        assert_eq!(
            selected.targets[0].config.get_int("anchored_event_count"),
            Some(enabled)
        );
        assert!(selected.targets[0].config.get("private_probe").is_none());
        assert!(selected.invocation_config.get("private_probe").is_none());
        assert!(selected.slice_regions.is_empty());
        assert!(
            selected.perimeter_configs.is_empty(),
            "no model key for anchored stage"
        );
        for (leg, module) in ["native", "wasm"].into_iter().zip(&modules) {
            let commit = run(
                "Layer::AnchoredEvents",
                &layer,
                module,
                selected,
                None,
                None,
                None,
            );
            if enabled == 0 {
                assert!(
                    commit.is_none(),
                    "{leg}: disabled module still invoked but emits no collection"
                );
            } else {
                let Some(LayerStageCommit::AnchoredEvents(collections)) = commit else {
                    panic!("{leg}: actual module anchored collection required")
                };
                assert_eq!(collections.len(), 1);
                assert_eq!(collections[0].anchor_global_layer_index, 7);
                assert_eq!(
                    collections[0].events.len(),
                    2,
                    "enable switch 1 means two fixed entities"
                );
                for entity in &collections[0].events {
                    assert_eq!(entity.anchor_global_layer_index, 7);
                    assert!(
                        !entity.path_points.is_empty(),
                        "{leg}: nonempty anchored entity"
                    );
                }
            }
        }
        ANCHORED_CALLS.with(|calls| {
            assert_eq!(
                &*calls.borrow(),
                if enabled == 1 { &[1][..] } else { &[1, 0][..] },
                "native entry invoked exactly once per enabled/disabled execution"
            )
        });
    }
}
