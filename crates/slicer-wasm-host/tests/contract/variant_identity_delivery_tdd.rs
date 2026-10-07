//! Full variant identity must select distinct configs on the ordinary runner.
//!
//! Family A: width, fill-holder, ordering, and rejection delivery (AC-1/2/3/N1/N2).

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, InfillIR,
    LayerStageCommit, LayerStageError, PaintValue, Point2, Point3WithWidth, Polygon, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_sdk::native::{NativeLayerRequest, NativeLayerResponse, NativeStageEntry};
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelection, LayerSelectionIndexInput,
    LayerSelectionModuleInput, SelectionError, STAGES,
};
use slicer_wasm_host::{
    binding::LayerStageInput, CompiledModuleLive, LayerStageRunner, WasmEngine, WasmInstancePool,
    WasmRuntimeDispatcher,
};

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
        holes: Vec::new(),
    }
}

fn fixture() -> (SliceIR, RegionMapIR) {
    ordered_fixture(false, false, false)
}

fn ordered_fixture(
    fill_roles: bool,
    reverse_slice: bool,
    reverse_map: bool,
) -> (SliceIR, RegionMapIR) {
    ordered_fixture_with_missing_holder(fill_roles, reverse_slice, reverse_map, false)
}

fn ordered_fixture_with_missing_holder(
    fill_roles: bool,
    reverse_slice: bool,
    reverse_map: bool,
    missing_holder: bool,
) -> (SliceIR, RegionMapIR) {
    let mut map = RegionMapIR::default();
    let mut regions = Vec::new();
    let mut variants = [(1, 0.4), (2, 0.6)];
    if reverse_map {
        variants.reverse();
    }
    for (tool, width) in variants {
        let config = map.intern_config(ResolvedConfig {
            line_width: width,
            infill_density: 0.2,
            sparse_fill_holder: if missing_holder && tool == 2 {
                "com.test.missing"
            } else if missing_holder || !fill_roles || tool == 2 {
                "rectilinear-infill"
            } else {
                "com.test.other"
            }
            .into(),
            top_fill_holder: if fill_roles && tool == 1 {
                "rectilinear-infill"
            } else {
                "com.test.other"
            }
            .into(),
            bottom_fill_holder: "com.test.other".into(),
            bridge_fill_holder: "com.test.other".into(),
            ..Default::default()
        });
        map.entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "variant-cube".into(),
                region_id: 7,
                variant_chain: chain(tool),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
        let mut sparse = square();
        if fill_roles {
            for point in &mut sparse.contour.points {
                point.x += Point2::from_mm(20.0, 0.0).x;
            }
        }
        regions.push(SlicedRegion {
            object_id: "variant-cube".into(),
            region_id: 7_000_000 + u64::from(tool) + 1,
            variant_chain: chain(tool),
            polygons: if fill_roles {
                vec![square(), sparse.clone()]
            } else {
                vec![square()]
            },
            sparse_infill_area: vec![sparse],
            top_shell_index: fill_roles.then_some(0),
            top_solid_fill: if fill_roles {
                vec![square()]
            } else {
                Vec::new()
            },
            effective_layer_height: 0.2,
            ..Default::default()
        });
    }
    regions.sort_by_key(|region| {
        if region.variant_chain == chain(1) {
            1
        } else {
            2
        }
    });
    if reverse_slice {
        regions.reverse();
    }
    (
        SliceIR {
            z: 0.2,
            regions,
            ..Default::default()
        },
        map,
    )
}

fn module_config() -> Arc<ConfigView> {
    // Same required manifest reads as the painted-holder fixture. This is the
    // module baseline; exact per-variant config must override its line width.
    let mut map = HashMap::new();
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
        map.insert(key.into(), ConfigValue::Float(value));
    }
    for (key, value) in [
        ("thick_bridges", false),
        ("thick_internal_bridges", true),
        ("dont_filter_internal_bridges", false),
        ("enable_extra_bridge_layer", false),
    ] {
        map.insert(key.into(), ConfigValue::Bool(value));
    }
    Arc::new(ConfigView::from_map(map))
}

fn native_width_witness(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, slicer_sdk::error::ModuleError> {
    NATIVE_INVOCATIONS.with(|count| count.set(count.get() + 1));
    let mut output = slicer_sdk::builders::InfillOutputBuilder::new();
    for region in &request.regions {
        DELIVERED_CLAIMS.with(|records| {
            records.borrow_mut().push((
                region.variant_chain().to_vec(),
                region.held_claims().to_vec(),
            ))
        });
        if region.held_claims().is_empty() {
            continue;
        }
        let config = region.config().unwrap_or(&request.config);
        let width = match config.get("line_width") {
            Some(ConfigValue::Float(width)) => *width as f32,
            other => panic!("native delivered line_width must be numeric: {other:?}"),
        };
        output.begin_region(
            region.object_id(),
            *region.region_id(),
            region.variant_chain(),
        );
        output
            // exhaustive: witness path explicitly specifies all non-default path metadata
            .push_sparse_path(ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        width,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                    Point3WithWidth {
                        x: 1.0,
                        y: 0.0,
                        z: 0.2,
                        width,
                        flow_factor: 1.0,
                        ..Default::default()
                    },
                ],
                role: ExtrusionRole::SparseInfill,
                speed_factor: 1.0,
                tool_index: None,
                order_lock: None,
            })
            .expect("native width witness path");
    }
    // exhaustive: native infill-only response protects the runner commit contract
    Ok(NativeLayerResponse {
        infill: Some(output),
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    })
}

fn run(module: &CompiledModuleLive<'_>, slice: &SliceIR, map: &RegionMapIR) -> InfillIR {
    let dispatcher = WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine());
    let layer = GlobalLayer {
        z: 0.2,
        ..Default::default()
    };
    // exhaustive: committed snapshot pins every selection authority channel.
    let selection_index = build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: std::slice::from_ref(&layer),
        // exhaustive: binding supplies all static module metadata.
        modules: &[LayerSelectionModuleInput {
            stage_id: "Layer::Infill",
            module_id: module.module_id,
            claims: module.claims,
            paint_semantics: None,
            invocation_config: &module.config_view,
        }],
        slices: std::slice::from_ref(slice),
        region_map: Some(map),
        support_plan: None,
        perimeters: &[],
    })
    .expect("variant delivery selection index");
    // exhaustive: selection runner boundary has no Default; all inputs explicit
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: Some(Arc::new(map.clone())),
        slice: Some(slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
        selection: Some(
            selection_index
                .selection_for(layer.index, module.module_id)
                .expect("bound variant module"),
        ),
    };
    match LayerStageRunner::run_stage(&dispatcher, &"Layer::Infill".into(), &layer, module, input)
        .expect("ordinary variant dispatch must succeed")
    {
        Some(LayerStageCommit::Infill(ir)) => ir,
        other => panic!("variant fixture must produce a nonempty infill commit: {other:?}"),
    }
}

thread_local! {
    static DELIVERED_CLAIMS: std::cell::RefCell<Vec<(Vec<(String, PaintValue)>, Vec<String>)>> = const { std::cell::RefCell::new(Vec::new()) };
    static NATIVE_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn run_both(slice: &SliceIR, map: &RegionMapIR, fill_roles: bool) -> [(&'static str, InfillIR); 2] {
    let id = "com.core.rectilinear-infill".to_string();
    let claims = if fill_roles {
        vec![
            "claim:top-fill",
            "claim:bottom-fill",
            "claim:bridge-fill",
            "claim:sparse-fill",
        ]
    } else {
        vec!["claim:sparse-fill"]
    }
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/rectilinear-infill/rectilinear-infill.wasm");
    assert!(
        path.exists(),
        "missing production guest: {}",
        path.display()
    );
    let wasm = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        Some(crate::common::wasm_cache::compiled_component_at(&path)),
        &claims,
        module_config(),
    );
    let native = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        module_config(),
    )
    .with_native_entry(NativeStageEntry::Layer(native_width_witness));
    // Execute both production runner legs before checking delivered widths.
    DELIVERED_CLAIMS.with(|records| records.borrow_mut().clear());
    [
        ("native", run(&native, slice, map)),
        ("wasm", run(&wasm, slice, map)),
    ]
}

fn assert_width_delivery(outputs: [(&str, InfillIR); 2]) {
    for (leg, ir) in outputs {
        assert_eq!(
            ir.regions.len(),
            2,
            "{leg}: both variant commits must survive"
        );
        for (tool, expected) in [(1, 0.4_f32), (2, 0.6_f32)] {
            let region = ir
                .regions
                .iter()
                .find(|r| r.variant_chain == chain(tool))
                .unwrap_or_else(|| panic!("{leg}: missing exact variant {tool}"));
            assert_eq!(region.object_id, "variant-cube");
            assert_eq!(region.region_id, 7);
            assert!(
                !region.sparse_infill.is_empty(),
                "{leg}: variant {tool} must emit paths"
            );
            for path in &region.sparse_infill {
                assert!(
                    !path.points.is_empty(),
                    "{leg}: variant {tool} must emit points"
                );
                for point in &path.points {
                    assert!((point.width - expected).abs() < 1e-6,
                        "{leg}: variant {tool} must receive exact line_width {expected}; delivered {}", point.width);
                }
            }
        }
    }
}

#[test]
fn distinct_variant_configs_reach_both_legs() {
    let (slice, map) = fixture();
    assert_width_delivery(run_both(&slice, &map, false));
}

fn assert_fill_delivery(outputs: [(&str, InfillIR); 2]) {
    DELIVERED_CLAIMS.with(|records| {
        let records = records.borrow();
        assert_eq!(records.len(), 2, "native: exactly two delivered identities");
        for (tool, expected) in [(1, "claim:top-fill"), (2, "claim:sparse-fill")] {
            let (_, claims) = records
                .iter()
                .find(|(variant, _)| *variant == chain(tool))
                .unwrap_or_else(|| panic!("native: missing exact variant {tool}"));
            assert_eq!(
                claims,
                &[expected.to_string()],
                "native: variant {tool} must receive exact held claims"
            );
        }
    });
    let (_, wasm) = outputs
        .into_iter()
        .find(|(leg, _)| *leg == "wasm")
        .expect("WASM leg executed");
    assert_eq!(
        wasm.regions.len(),
        2,
        "wasm: exactly two delivered identities"
    );
    for tool in [1, 2] {
        let region = wasm
            .regions
            .iter()
            .find(|region| region.variant_chain == chain(tool))
            .unwrap_or_else(|| panic!("wasm: missing exact variant {tool}"));
        assert_eq!(region.object_id, "variant-cube");
        assert_eq!(region.region_id, 7);
        assert!(
            region.ironing.is_empty(),
            "wasm: variant {tool} must not iron"
        );
        assert!(
            region.internal_bridge_infill.is_empty(),
            "wasm: variant {tool} must not bridge"
        );
        let (paths, expected_role) = if tool == 1 {
            assert!(
                region.sparse_infill.is_empty(),
                "wasm: top holder must not emit sparse paths"
            );
            (&region.solid_infill, ExtrusionRole::TopSolidInfill)
        } else {
            assert!(
                region.solid_infill.is_empty(),
                "wasm: sparse holder must not emit solid paths"
            );
            (&region.sparse_infill, ExtrusionRole::SparseInfill)
        };
        assert!(
            !paths.is_empty(),
            "wasm: variant {tool} must emit held-role paths"
        );
        for path in paths {
            assert_eq!(
                path.role, expected_role,
                "wasm: variant {tool} must emit only its held role"
            );
        }
    }
}

#[test]
fn variant_specific_fill_holders_reach_both_legs() {
    let (slice, map) = ordered_fixture(true, false, false);
    assert_fill_delivery(run_both(&slice, &map, true));
}

#[test]
fn variant_delivery_is_order_independent() {
    // Run every fixture/permutation on both legs before checking independent
    // expectations, so a first behavioral red does not hide dispatch failures.
    let mut results = Vec::new();
    for reverse_slice in [false, true] {
        for reverse_map in [false, true] {
            for fill_roles in [false, true] {
                let (slice, map) = ordered_fixture(fill_roles, reverse_slice, reverse_map);
                let outputs = run_both(&slice, &map, fill_roles);
                let claims = DELIVERED_CLAIMS.with(|records| records.borrow().clone());
                results.push((fill_roles, outputs, claims));
            }
        }
    }
    for (fill_roles, outputs, claims) in results {
        if fill_roles {
            DELIVERED_CLAIMS.with(|records| *records.borrow_mut() = claims);
            assert_fill_delivery(outputs);
        } else {
            assert_width_delivery(outputs);
        }
    }
}

fn rejection_input<'a>(
    slice: &'a SliceIR,
    map: Option<&RegionMapIR>,
    selection: Option<&'a LayerSelection>,
) -> LayerStageInput<'a> {
    // exhaustive: rejection and positive controls explicitly pin every runner input.
    LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: map.map(|map| Arc::new(map.clone())),
        slice: Some(slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
        selection,
    }
}

#[test]
fn missing_exact_model_config_rejects_before_invocation() {
    let (slice, complete_map) = ordered_fixture(false, false, false);
    assert_eq!(slice.regions.len(), 2);
    for region in &slice.regions {
        assert!(!region.polygons.is_empty());
        assert!(!region.sparse_infill_area.is_empty());
    }
    let key = |tool| RegionKey {
        global_layer_index: 0,
        object_id: "variant-cube".into(),
        region_id: 7,
        variant_chain: chain(tool),
    };
    let mut partial_map = complete_map.clone();
    partial_map.entries.remove(&key(2));
    let layer = GlobalLayer {
        z: 0.2,
        ..Default::default()
    };
    let id = "com.core.rectilinear-infill".to_string();
    let stage = "Layer::Infill".to_string();
    let claims = vec!["claim:sparse-fill".to_string()];
    let config = module_config();
    let build = |region_map| {
        // exhaustive: both missing-authority cases and the complete control share one snapshot.
        build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: std::slice::from_ref(&layer),
            // exhaustive: the scheduled module has all static selection metadata.
            modules: &[LayerSelectionModuleInput {
                stage_id: &stage,
                module_id: &id,
                claims: &claims,
                paint_semantics: None,
                invocation_config: &config,
            }],
            slices: std::slice::from_ref(&slice),
            region_map,
            support_plan: None,
            perimeters: &[],
        })
    };
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/rectilinear-infill/rectilinear-infill.wasm");
    // Meter actual guest instructions, not merely dispatch attempts. The same
    // engine/component runs a nonempty positive control below.
    let engine = Arc::new(WasmEngine::with_profiling(true));
    let component = Arc::new(
        engine
            .compile_component(&std::fs::read(&path).expect("production rectilinear guest"))
            .expect("compile metered rectilinear guest"),
    );
    let dispatcher = WasmRuntimeDispatcher::new(engine);
    let wasm = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        Some(component),
        &claims,
        config.clone(),
    );
    let native = CompiledModuleLive::new(
        &id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        config.clone(),
    )
    .with_native_entry(NativeStageEntry::Layer(native_width_witness));
    NATIVE_INVOCATIONS.with(|count| count.set(0));
    DELIVERED_CLAIMS.with(|records| records.borrow_mut().clear());
    // Fuel samples are thread-local and draining; discard any earlier test's
    // sample before observing this test's four rejected dispatches.
    let _ = LayerStageRunner::last_call_fuel(&dispatcher);
    for (label, map, missing) in [
        ("partial map", Some(&partial_map), key(2)),
        ("absent map", None, key(1)),
    ] {
        let error = build(map).expect_err("missing exact model authority must reject the index");
        assert_eq!(error.stage_id, stage, "{label}: failing stage");
        assert_eq!(error.module_id, id, "{label}: failing module");
        assert_eq!(
            error.error,
            SelectionError::MissingModelConfig { key: missing }
        );
        for (leg, module) in [("native", &native), ("wasm", &wasm)] {
            // An Err has no commit payload: neither leg can publish output.
            let error = LayerStageRunner::run_stage(
                &dispatcher,
                &stage,
                &layer,
                module,
                rejection_input(&slice, map, None),
            )
            .expect_err("scheduled module without a selection must reject before invocation");
            let LayerStageError::FatalModule {
                stage_id,
                module_id,
                message,
            } = error
            else {
                panic!("{label}/{leg}: expected FatalModule, got {error:?}");
            };
            assert_eq!(stage_id, stage, "{label}/{leg}: scheduled stage");
            assert_eq!(module_id, id, "{label}/{leg}: scheduled module");
            assert_eq!(message, "scheduled layer module has no selection");
            assert_eq!(NATIVE_INVOCATIONS.with(std::cell::Cell::get), 0);
            assert!(DELIVERED_CLAIMS.with(|records| records.borrow().is_empty()));
            assert_eq!(
                LayerStageRunner::last_call_fuel(&dispatcher),
                0,
                "{label}/{leg}: rejected dispatch must execute zero WASM instructions"
            );
        }
    }
    let index = build(Some(&complete_map)).expect("complete exact authority");
    let selection = index
        .selection_for(layer.index, &id)
        .expect("scheduled module");
    assert!(selection.invoke);
    let mut outputs = Vec::new();
    for (leg, module) in [("native", &native), ("wasm", &wasm)] {
        let commit = LayerStageRunner::run_stage(
            &dispatcher,
            &stage,
            &layer,
            module,
            rejection_input(&slice, Some(&complete_map), Some(selection)),
        )
        .expect("complete-index positive control must invoke");
        let Some(LayerStageCommit::Infill(ir)) = commit else {
            panic!("{leg}: positive control must commit infill, got {commit:?}");
        };
        if leg == "wasm" {
            assert!(
                LayerStageRunner::last_call_fuel(&dispatcher) > 0,
                "positive control must observe actual metered WASM guest execution"
            );
        }
        outputs.push((leg, ir));
    }
    assert_eq!(NATIVE_INVOCATIONS.with(std::cell::Cell::get), 1);
    DELIVERED_CLAIMS.with(|records| assert_eq!(records.borrow().len(), 2));
    assert_width_delivery(outputs.try_into().expect("both positive legs executed"));
}

#[test]
fn unknown_variant_holder_does_not_borrow_sibling_claim() {
    let (slice, map) = ordered_fixture_with_missing_holder(false, false, false, true);
    assert_eq!(slice.regions.len(), 2);
    for region in &slice.regions {
        assert!(!region.sparse_infill_area.is_empty());
        assert!(!region.sparse_infill_area[0].contour.points.is_empty());
    }
    let outputs = run_both(&slice, &map, false);
    DELIVERED_CLAIMS.with(|records| {
        let records = records.borrow();
        assert_eq!(
            records.len(),
            2,
            "native: both full identities must be delivered"
        );
        for (tool, expected) in [(1, vec!["claim:sparse-fill".to_string()]), (2, vec![])] {
            let (_, claims) = records
                .iter()
                .find(|(variant, _)| *variant == chain(tool))
                .unwrap_or_else(|| panic!("native: missing exact variant {tool}"));
            assert_eq!(
                *claims, expected,
                "native: variant {tool} must not borrow sibling claims"
            );
        }
    });
    for (leg, ir) in outputs {
        assert_eq!(
            ir.regions.len(),
            1,
            "{leg}: unknown-holder variant must not commit"
        );
        let region = &ir.regions[0];
        assert_eq!(region.object_id, "variant-cube");
        assert_eq!(region.region_id, 7);
        assert_eq!(region.variant_chain, chain(1));
        assert!(
            !region.sparse_infill.is_empty(),
            "{leg}: held sparse fill must emit paths"
        );
        for path in &region.sparse_infill {
            assert_eq!(path.role, ExtrusionRole::SparseInfill);
            assert!(
                !path.points.is_empty(),
                "{leg}: held sparse paths must emit points"
            );
        }
    }
}
