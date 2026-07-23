//! Regression: per-region fill-holder resolution works for painted regions.
//!
//! Pre-fix, `slicer-wasm-host/src/dispatch.rs` built a `RegionKey` with an
//! empty `variant_chain` when looking up the per-region config for held-claim
//! resolution. Painted regions are keyed in `RegionMapIR` with a non-empty
//! `variant_chain` (e.g. `("material", ToolIndex(n))`), so the lookup missed and
//! the resolver fell back to `ResolvedConfig::default()`. Every painted region
//! therefore defaulted to `rectilinear-infill` for all four fill roles, causing
//! multi-infill user configs to be ignored for any painted geometry.
//!
//! This test drives the real `LayerStageRunner` boundary with the production
//! `rectilinear-infill` and `gyroid-infill` core modules against a painted
//! region whose config names `gyroid-infill` as the sparse holder. It asserts
//! that only gyroid emits sparse paths and rectilinear is gated out.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, PaintValue, Point2, Polygon, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
};
use slicer_wasm_host::{
    binding::LayerStageInput, CompiledModuleLive, LayerStageRunner, WasmInstancePool,
};

fn square(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min_x, y: min_y },
                Point2 { x: max_x, y: min_y },
                Point2 { x: max_x, y: max_y },
                Point2 { x: min_x, y: max_y },
            ],
        },
        holes: Vec::new(),
    }
}

fn core_module_wasm(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules")
        .join("core-modules")
        .join(name)
        .join(format!("{name}.wasm"))
}

fn load_component(name: &str) -> Arc<slicer_wasm_host::WasmComponent> {
    let path = core_module_wasm(name);
    assert!(
        path.exists(),
        "core module wasm missing: {} — run `cargo xtask build-guests`",
        path.display()
    );
    crate::common::wasm_cache::compiled_component_at(&path)
}

fn build_painted_region_map(sparse_holder: &str) -> RegionMapIR {
    let cfg = ResolvedConfig {
        sparse_fill_holder: sparse_holder.to_string(),
        top_fill_holder: "rectilinear-infill".to_string(),
        bottom_fill_holder: "rectilinear-infill".to_string(),
        bridge_fill_holder: "rectilinear-infill".to_string(),
        ..Default::default()
    };

    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(cfg);
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "painted-cube".into(),
        region_id: 1_000_001,
        variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
    };
    region_map.entries.insert(
        key,
        RegionPlan {
            config: config_id,
            stage_modules: HashMap::new(),
            paint_overrides: BTreeMap::new(),
        },
    );
    region_map
}

fn build_slice_ir() -> SliceIR {
    let region = SlicedRegion {
        object_id: "painted-cube".into(),
        region_id: 1_000_001,
        polygons: vec![square(0, 0, 100_000, 100_000)],
        effective_layer_height: 0.2,
        sparse_infill_area: vec![square(10_000, 10_000, 90_000, 90_000)],
        ..Default::default()
    };
    SliceIR {
        schema_version: SemVer {
            major: 4,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        regions: vec![region],
    }
}

fn build_module_config() -> Arc<ConfigView> {
    let mut map = HashMap::new();
    map.insert("infill_density".into(), ConfigValue::Float(0.2));
    map.insert("line_width".into(), ConfigValue::Float(0.4));
    Arc::new(ConfigView::from_map(map))
}

fn run_infill_stage(
    dispatcher: &slicer_wasm_host::WasmRuntimeDispatcher,
    module: &CompiledModuleLive<'_>,
    slice_ir: &SliceIR,
    region_map: &RegionMapIR,
) -> Option<slicer_ir::LayerStageCommit> {
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mesh = Arc::new(slicer_ir::MeshIR::default());
    let input = LayerStageInput {
        mesh,
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: Some(Arc::new(region_map.clone())),
        slice: Some(slice_ir),
        perimeter: None,
        layer_collection: None,
        surface_classification: None,
        infill: None,
    };
    let stage_id = "Layer::Infill".to_string();
    LayerStageRunner::run_stage(dispatcher, &stage_id, &layer, module, input)
        .expect("dispatch must succeed")
}

#[test]
fn painted_region_sparse_holder_gates_non_holder_module() {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = slicer_wasm_host::WasmRuntimeDispatcher::new(engine);

    let module_id = "com.core.rectilinear-infill".to_string();
    let rectilinear_claims = vec![
        "claim:top-fill".to_string(),
        "claim:bottom-fill".to_string(),
        "claim:bridge-fill".to_string(),
        "claim:sparse-fill".to_string(),
    ];
    let rectilinear = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        Some(load_component("rectilinear-infill")),
        &rectilinear_claims,
        build_module_config(),
    );

    let region_map = build_painted_region_map("gyroid-infill");
    let slice_ir = build_slice_ir();

    // Rectilinear holds top/bottom/bridge but NOT sparse for this region,
    // and the region has no top/bottom/bridge polygons, so it must emit nothing.
    let commit = run_infill_stage(&dispatcher, &rectilinear, &slice_ir, &region_map);
    assert!(
        commit.is_none(),
        "rectilinear must be gated out when gyroid holds sparse-fill; got {commit:?}"
    );
}

#[test]
fn painted_region_sparse_holder_allows_holder_module() {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = slicer_wasm_host::WasmRuntimeDispatcher::new(engine);

    let module_id = "com.core.gyroid-infill".to_string();
    let gyroid_claims = vec!["claim:sparse-fill".to_string()];
    let gyroid = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        Some(load_component("gyroid-infill")),
        &gyroid_claims,
        build_module_config(),
    );

    let region_map = build_painted_region_map("gyroid-infill");
    let slice_ir = build_slice_ir();

    let commit = run_infill_stage(&dispatcher, &gyroid, &slice_ir, &region_map);
    let ir = match commit {
        Some(slicer_ir::LayerStageCommit::Infill(ir)) => ir,
        other => panic!("gyroid must emit InfillIR when it holds sparse-fill; got {other:?}"),
    };
    assert_eq!(
        ir.regions.len(),
        1,
        "gyroid should produce exactly one infill region"
    );
    assert!(
        !ir.regions[0].sparse_infill.is_empty(),
        "gyroid must emit at least one sparse infill path over the populated sparse polygon"
    );
}

#[test]
fn painted_region_sparse_holder_rectilinear_when_configured() {
    // Inverse: when rectilinear is configured as the sparse holder, it emits
    // and gyroid is gated out. This proves the resolution is driven by the
    // per-region config, not by alphabetical defaults.
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = slicer_wasm_host::WasmRuntimeDispatcher::new(engine);

    let rectilinear_id = "com.core.rectilinear-infill".to_string();
    let rectilinear_claims = vec![
        "claim:top-fill".to_string(),
        "claim:bottom-fill".to_string(),
        "claim:bridge-fill".to_string(),
        "claim:sparse-fill".to_string(),
    ];
    let rectilinear = CompiledModuleLive::new(
        &rectilinear_id,
        WasmInstancePool::placeholder(),
        Some(load_component("rectilinear-infill")),
        &rectilinear_claims,
        build_module_config(),
    );

    let gyroid_id = "com.core.gyroid-infill".to_string();
    let gyroid_claims = vec!["claim:sparse-fill".to_string()];
    let gyroid = CompiledModuleLive::new(
        &gyroid_id,
        WasmInstancePool::placeholder(),
        Some(load_component("gyroid-infill")),
        &gyroid_claims,
        build_module_config(),
    );

    let mut region_map = build_painted_region_map("rectilinear-infill");
    // Re-intern the same config so the lookup still resolves.
    let cfg = region_map
        .config_for_raw(RegionPlan::default().config)
        .clone();
    let _ = region_map.intern_config(cfg);
    let slice_ir = build_slice_ir();

    let rect_commit = run_infill_stage(&dispatcher, &rectilinear, &slice_ir, &region_map);
    let gyroid_commit = run_infill_stage(&dispatcher, &gyroid, &slice_ir, &region_map);

    let rect_ir = match rect_commit {
        Some(slicer_ir::LayerStageCommit::Infill(ir)) => ir,
        other => {
            panic!("rectilinear must emit InfillIR when configured as sparse holder; got {other:?}")
        }
    };
    assert!(
        !rect_ir.regions[0].sparse_infill.is_empty(),
        "rectilinear must emit sparse paths when configured as sparse holder"
    );
    assert!(
        gyroid_commit.is_none(),
        "gyroid must be gated out when rectilinear holds sparse-fill; got {gyroid_commit:?}"
    );
}
