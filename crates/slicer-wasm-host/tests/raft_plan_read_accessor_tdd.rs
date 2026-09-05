//! AC-7 (packet 240a): a `Layer::Infill` guest can read the print-wide raft
//! plan and the current layer's raft-band flag through
//! `paint-region-layer-view`.
//!
//! The raft band is a POSITIVE OFFSET band: raft layers occupy global layer
//! indices `0..support_raft_layers-1`. Raft-ness is carried explicitly by
//! `GlobalLayer.is_raft` and is never inferred from the index, so a guest that
//! cannot call `is-raft` cannot tell a raft layer from a model layer at all.
//!
//! Own test binary (not the `contract` bucket) so the raft reads stay isolated
//! from the shared harness. `build_paint_layer_data_with_plan` is private; both
//! wasm tests reach it only through the public `LayerStageRunner::run_stage`
//! dispatch entry point.

#![allow(missing_docs)]

#[path = "common/wasm_cache.rs"]
mod wasm_cache;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, GlobalLayer, RaftPlan, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion, SupportPlanIR,
};
use slicer_wasm_host::{
    binding::LayerStageInput, CompiledModuleLive, LayerStageRunner, WasmInstancePool,
    WasmRuntimeDispatcher,
};

const OBJECT_ID: &str = "raft-fixture";
const REGION_ID: u64 = 3;

/// The raft plan the host commits; every field is distinct so a field-order
/// mix-up in the WIT mirror cannot pass.
fn raft_plan() -> RaftPlan {
    RaftPlan {
        raft_layers: 4,
        raft_first_layer_density: 0.75,
        base_raft_layers: 2,
        interface_raft_layers: 1,
    }
}

fn region_map() -> RegionMapIR {
    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(ResolvedConfig::default());
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: OBJECT_ID.to_string(),
            region_id: REGION_ID,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: config_id,
            ..RegionPlan::default()
        },
    );
    region_map
}

fn slice_ir() -> SliceIR {
    SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: OBJECT_ID.to_string(),
            region_id: REGION_ID,
            effective_layer_height: 0.2,
            ..SlicedRegion::default()
        }],
    }
}

fn config() -> Arc<ConfigView> {
    Arc::new(ConfigView::from_map(HashMap::from([(
        "infill-spacing".to_string(),
        ConfigValue::Float(2.0),
    )])))
}

/// Dispatch the shipped `layer-infill-guest` component through the public
/// runner and return the sparse paths it emitted.
fn run_wasm_guest(is_raft: bool, support_plan: Option<SupportPlanIR>) -> Vec<ExtrusionPath3D> {
    let dispatcher = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    let module_id = "com.test.raft-infill".to_string();
    let claims = vec!["claim:sparse-fill".to_string()];
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        Some(wasm_cache::compiled_guest("layer-infill-guest")),
        &claims,
        config(),
    );
    let slice = slice_ir();
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        is_raft,
        ..GlobalLayer::default()
    };
    // exhaustive: LayerStageInput has no Default and this test supplies the complete stage input
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: support_plan.map(Arc::new),
        lightning_tree_ir: None,
        region_map: Some(Arc::new(region_map())),
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        infill: None,
    };
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        input,
    )
    .expect("raft infill dispatch must succeed")
    .expect("guest must emit infill output");
    match commit {
        slicer_ir::LayerStageCommit::Infill(ir) => ir
            .regions
            .into_iter()
            .flat_map(|region| region.sparse_infill)
            .collect(),
        other => panic!("expected Layer::Infill commit, got {other:?}"),
    }
}

/// The guest tags each witness with a distinct `width`; the payload rides on
/// `x` / `y` / `flow_factor` of the single point.
fn witness(paths: &[ExtrusionPath3D], tag: f32) -> Option<&ExtrusionPath3D> {
    paths
        .iter()
        .find(|path| path.points.len() == 1 && (path.points[0].width - tag).abs() < 1e-4)
}

#[test]
fn raft_plan_reaches_layer_infill_guest() {
    let plan = SupportPlanIR {
        raft_plan: Some(raft_plan()),
        ..SupportPlanIR::default()
    };
    let paths = run_wasm_guest(false, Some(plan));

    let a = witness(&paths, 241.0).expect("guest must emit the raft-plan witness (tag 241)");
    assert_eq!(a.points[0].x, 4.0, "raft_layers");
    assert!(
        (a.points[0].y - 0.75).abs() < 1e-6,
        "raft_first_layer_density, got {}",
        a.points[0].y
    );
    assert_eq!(a.points[0].flow_factor, 2.0, "base_raft_layers");

    let b = witness(&paths, 242.0).expect("guest must emit the raft-plan witness (tag 242)");
    assert_eq!(b.points[0].x, 1.0, "interface_raft_layers");

    // `is-raft` is a property of the LAYER, not of the raft plan: this is a
    // model layer, so witness 240 must be absent even though a raft plan
    // reached the guest (witnesses 241/242 above).
    assert!(
        witness(&paths, 240.0).is_none(),
        "is-raft must stay false on a model layer even when a raft plan exists"
    );
}

#[test]
fn is_raft_reaches_layer_infill_guest() {
    let raft_paths = run_wasm_guest(true, None);
    assert!(
        witness(&raft_paths, 240.0).is_some(),
        "guest must observe is-raft == true when GlobalLayer.is_raft is set"
    );

    // Negative control: without the flag the accessor must report false, so
    // the guest emits no witness. Without this the test would pass against an
    // accessor hard-wired to `true`.
    let model_paths = run_wasm_guest(false, None);
    assert!(
        witness(&model_paths, 240.0).is_none(),
        "guest must observe is-raft == false on a model layer"
    );
}

// ---------------------------------------------------------------------------
// Native leg
// ---------------------------------------------------------------------------

/// `(is_raft, raft_plan)` observed by the native entry, captured per dispatch.
static NATIVE_OBSERVED: Mutex<Option<(bool, Option<RaftPlan>)>> = Mutex::new(None);

fn native_infill_entry(
    request: &slicer_sdk::native::NativeLayerRequest,
) -> Result<slicer_sdk::native::NativeLayerResponse, slicer_sdk::error::ModuleError> {
    let paint = request
        .paint
        .as_ref()
        .expect("native request carries paint");
    *NATIVE_OBSERVED.lock().expect("observed mutex") =
        Some((paint.is_raft(), paint.raft_plan().cloned()));
    let mut infill = slicer_sdk::builders::InfillOutputBuilder::new();
    infill.begin_region(OBJECT_ID, REGION_ID);
    infill
        // exhaustive: ExtrusionPath3D has no Default; test-only witness path
        .push_sparse_path(ExtrusionPath3D {
            points: vec![
                slicer_ir::Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    ..Default::default()
                },
                slicer_ir::Point3WithWidth {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    ..Default::default()
                },
            ],
            role: slicer_ir::ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        })
        .expect("push sparse path");
    // exhaustive: test-only native layer response; every stage slot named explicitly
    Ok(slicer_sdk::native::NativeLayerResponse {
        infill: Some(infill),
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    })
}

fn run_native_entry(
    is_raft: bool,
    support_plan: Option<SupportPlanIR>,
) -> (bool, Option<RaftPlan>) {
    *NATIVE_OBSERVED.lock().expect("observed mutex") = None;
    let dispatcher = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    let module_id = "com.test.raft-infill-native".to_string();
    let claims = vec!["claim:sparse-fill".to_string()];
    let module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        None,
        &claims,
        config(),
    )
    .with_native_entry(slicer_sdk::native::NativeStageEntry::Layer(
        native_infill_entry,
    ));
    let slice = slice_ir();
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        is_raft,
        ..GlobalLayer::default()
    };
    // exhaustive: LayerStageInput has no Default and this test supplies the complete stage input
    let input = LayerStageInput {
        mesh: Arc::new(slicer_ir::MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: support_plan.map(Arc::new),
        lightning_tree_ir: None,
        region_map: Some(Arc::new(region_map())),
        slice: Some(&slice),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        infill: None,
    };
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        input,
    )
    .expect("native raft infill dispatch must succeed");
    NATIVE_OBSERVED
        .lock()
        .expect("observed mutex")
        .clone()
        .expect("native entry must have run")
}

/// Native-leg guard. `PaintRegionLayerView::is_raft` compiles and returns
/// `false` forever unless `build_native_layer_request_with_raft` is actually
/// handed `GlobalLayer.is_raft` by `dispatch_layer_call`; the two wasm tests
/// above would still pass in that state. This test dispatches a real native
/// entry through the same public runner and reads what the module saw.
#[test]
fn is_raft_set_on_native_leg() {
    let plan = SupportPlanIR {
        raft_plan: Some(raft_plan()),
        ..SupportPlanIR::default()
    };

    let (observed_raft, observed_plan) = run_native_entry(true, Some(plan));
    assert!(
        observed_raft,
        "native module must observe is_raft == true when GlobalLayer.is_raft is set"
    );
    assert_eq!(
        observed_plan,
        Some(raft_plan()),
        "native module must observe the committed raft plan"
    );

    // Negative control: proves the flag is threaded, not hard-wired.
    let (observed_raft, _) = run_native_entry(false, None);
    assert!(
        !observed_raft,
        "native module must observe is_raft == false on a model layer"
    );
}
