//! Runtime coverage for selecting the traditional support family through the
//! real module-loading and prepass aggregation path.

use std::sync::{Arc, Mutex};

use slicer_ir::{
    ConfigValue, ExtrusionRole, GlobalLayer, LayerStageCommit, SupportIR, SupportRole,
};
use slicer_runtime::{
    build_live_execution_plan, execute_per_layer_with_anchored_events, LayerStageInput,
    LayerStageRunner, NoopLayerProgressSink, WasmRuntimeDispatcher,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::SliceRegionViewBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_wasm_host::marshal::convert_native_support_output_with_plan;
use traditional_support::TraditionalSupport;

use crate::common::support_wedge;

/// The wedge must select the traditional family (via the `normal*` alias) and
/// retain family attribution through the host SupportPlanIR aggregation
/// boundary. The renderer must emit only the planner's polygons, and the
/// disabled helper exercises the matching no-output layer path.
#[test]
pub fn traditional_support_family() {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[(
            "support_type",
            ConfigValue::String("normal(auto)".to_string()),
        )],
    );
    let plan = ctx
        .blackboard
        .support_plan()
        .expect("traditional SupportPlanIR must be committed");

    assert!(!plan.entries.is_empty());
    assert!(plan
        .entries
        .iter()
        .all(|entry| entry.family_id == "traditional"));
    let mut structural = plan
        .entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none());
    assert!(structural.clone().next().is_some());
    assert!(structural.clone().all(|entry| !entry.body_ids.is_empty()));
    assert!(structural.all(|entry| { entry.roles.iter().any(|role| !role.regions.is_empty()) }));

    let renderer_commit = Arc::new(Mutex::new(None));
    let renderer_invoked = Arc::new(Mutex::new(false));
    let support_dispatches = Arc::new(Mutex::new(Vec::new()));
    let native_commit = Arc::new(Mutex::new(None));
    let structural_entry = plan
        .entries
        .iter()
        .find(|entry| {
            entry.decline_reason.is_none()
                && entry.roles.iter().any(|role| !role.regions.is_empty())
        })
        .expect("traditional fixture must contain a structural overhang entry");
    let core_modules = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules")
        .join("core-modules");
    let loaded = crate::common::wasm_cache::cached_live_modules(&[core_modules], 1);
    let mut config_source = std::collections::HashMap::new();
    config_source.insert("enable_support".to_string(), ConfigValue::Bool(true));
    config_source.insert(
        "support_type".to_string(),
        ConfigValue::String("normal(auto)".to_string()),
    );
    let target_z = structural_entry.anchor_z as f32 / 10_000.0;
    let global_layers = ctx
        .blackboard
        .layer_plan()
        .expect("traditional prepass must commit LayerPlanIR")
        .global_layers
        .iter()
        .min_by(|left, right| {
            (left.z - target_z)
                .abs()
                .partial_cmp(&(right.z - target_z).abs())
                .expect("traditional layer z values must be finite")
        })
        .cloned()
        .into_iter()
        .collect::<Vec<_>>();
    // Production promotes `global_layers` out of the committed `LayerPlanIR`
    // and backfills each region's resolved config from `RegionMapIR` first
    // (`promote_global_layers`). Without it every `ActiveRegion` still carries
    // `ResolvedConfig::default()`, so `module_claims_match_active_region`
    // resolves the traditional fallback for every region and the wrong family
    // renderer is handed this family's plan entries.
    let mut global_layers = global_layers;
    if let Some(region_map) = ctx.blackboard.region_map() {
        slicer_runtime::layer_executor::backfill_active_region_configs(
            &mut global_layers,
            region_map,
        );
    }
    let global_layers = global_layers;
    let mut layer_plan = build_live_execution_plan(
        loaded.sorted_stages.clone(),
        loaded.bindings.clone(),
        &config_source,
        Arc::new(global_layers),
        Arc::new(std::collections::HashMap::new()),
        &mut Vec::new(),
    )
    .expect("real core-module execution plan must build");
    let support_stage = layer_plan
        .per_layer_stages
        .iter_mut()
        .find(|stage| stage.stage_id == "Layer::Support")
        .expect("real Layer::Support stage must be present");
    assert!(
        support_stage
            .modules
            .iter()
            .any(|module| module.module_id() == "com.core.traditional-support"),
        "real traditional-support module must be available for support dispatch"
    );
    assert!(
        support_stage
            .modules
            .iter()
            .any(|module| module.module_id() == "com.core.tree-support"),
        "real tree-support module must be loaded for atomic-selection coverage"
    );
    let wasm_handles = loaded
        .bindings
        .iter()
        .map(|binding| {
            (
                binding.module.id().to_string(),
                (
                    Arc::clone(&binding.instance_pool),
                    binding.wasm_component.clone(),
                    binding.native_entry,
                ),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    execute_per_layer_with_anchored_events(
        &layer_plan,
        &ctx.blackboard,
        &CapturingLayerRunner {
            inner: WasmRuntimeDispatcher::new(Arc::clone(&loaded.engine)),
            commit: Arc::clone(&renderer_commit),
            invoked: Arc::clone(&renderer_invoked),
            support_dispatches: Arc::clone(&support_dispatches),
            native_commit: Arc::clone(&native_commit),
            plan: plan.clone(),
        },
        &NoopLayerProgressSink,
        &wasm_handles,
        &[],
    )
    .expect("traditional support renderer stage must execute");

    assert!(
        *renderer_invoked
            .lock()
            .expect("traditional renderer invocation lock must not be poisoned"),
        "traditional-support wasm module must be invoked"
    );
    let support_dispatches = support_dispatches
        .lock()
        .expect("support dispatch capture lock must not be poisoned")
        .clone();
    assert!(
        support_dispatches
            .iter()
            .any(|module_id| module_id == "com.core.traditional-support"),
        "traditional-support module must be dispatched for the traditional region; observed: {support_dispatches:?}"
    );
    assert!(
        !support_dispatches
            .iter()
            .any(|module_id| module_id == "com.core.tree-support"),
        "tree-support module must not be dispatched for the traditional region; observed: {support_dispatches:?}"
    );
    let live_committed = renderer_commit
        .lock()
        .expect("traditional renderer commit lock must not be poisoned")
        .clone()
        .expect("live traditional renderer must commit SupportIR");
    let live_rendered = live_committed
        .entries
        .iter()
        .find(|entry| entry.family_id == "traditional")
        .expect("live traditional renderer must retain the traditional family");
    assert_eq!(live_rendered.body_id, structural_entry.body_ids[0]);
    assert!(
        !live_rendered.paths.is_empty(),
        "live traditional body must emit at least one planned path"
    );

    let committed = native_commit
        .lock()
        .expect("native renderer commit lock must not be poisoned")
        .clone()
        .expect("native traditional renderer must commit SupportIR");
    let rendered = committed
        .entries
        .iter()
        .find(|entry| entry.family_id == "traditional")
        .expect("traditional renderer must retain the traditional family");
    assert_eq!(rendered.body_id, structural_entry.body_ids[0]);
    assert_eq!(rendered.demand_ids, structural_entry.demand_ids);
    assert_eq!(rendered.role, SupportRole::SupportBody);
    assert_eq!(rendered.object_id, structural_entry.object_id);
    assert_eq!(rendered.region_id, 0);
    assert!(
        !rendered.paths.is_empty(),
        "traditional body must emit at least one planned path"
    );
    assert!(rendered
        .paths
        .iter()
        .all(|path| { path.points.len() > 1 && path.role == ExtrusionRole::SupportMaterial }));

    crate::support_disabled_no_output::support_disabled_no_output();
}

struct CapturingLayerRunner {
    inner: WasmRuntimeDispatcher,
    commit: Arc<Mutex<Option<SupportIR>>>,
    invoked: Arc<Mutex<bool>>,
    support_dispatches: Arc<Mutex<Vec<String>>>,
    native_commit: Arc<Mutex<Option<SupportIR>>>,
    plan: Arc<slicer_ir::SupportPlanIR>,
}

impl LayerStageRunner for CapturingLayerRunner {
    fn run_stage(
        &self,
        stage_id: &slicer_ir::StageId,
        _layer: &GlobalLayer,
        module: &slicer_runtime::CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, slicer_ir::LayerStageError> {
        if stage_id == "Layer::Support" {
            self.support_dispatches
                .lock()
                .expect("support dispatch capture lock must not be poisoned")
                .push(module.module_id.to_string());
        }
        if stage_id == "Layer::Support" && module.module_id == "com.core.traditional-support" {
            *self
                .invoked
                .lock()
                .expect("traditional renderer invocation lock must not be poisoned") = true;
            let config = slicer_ir::ConfigView::from_map(std::collections::HashMap::from([
                ("enable_support".to_string(), ConfigValue::Bool(true)),
                (
                    "support_type".to_string(),
                    ConfigValue::String("normal(auto)".to_string()),
                ),
            ]));
            let native = TraditionalSupport::from_config(&config)
                .expect("traditional-support native module must construct");
            let layer_index = self
                .plan
                .entries
                .iter()
                .find(|entry| {
                    entry.decline_reason.is_none()
                        && entry.roles.iter().any(|role| !role.regions.is_empty())
                })
                .map(|entry| entry.global_layer_index as u32)
                .unwrap_or(_layer.index);
            let paint =
                PaintRegionLayerView::new(layer_index).with_support_plan(Arc::clone(&self.plan));
            let regions = input
                .slice
                .into_iter()
                .flat_map(|slice| slice.regions.iter())
                .map(|region| {
                    let mut view = SliceRegionViewBuilder::new()
                        .object_id(region.object_id.clone())
                        .region_id(region.region_id)
                        .z(_layer.z)
                        .overhang_areas(region.polygons.clone())
                        .build();
                    view.set_needs_support(true);
                    view
                })
                .collect::<Vec<_>>();
            let mut output = SupportOutputBuilder::new();
            native
                .run_support(layer_index, &regions, &paint, &mut output, &config)
                .expect("traditional-support native renderer must run");
            let support =
                convert_native_support_output_with_plan(&output, layer_index, self.plan.as_ref())
                    .expect("native host support conversion must succeed");
            *self.native_commit.lock().expect("native commit lock") = Some(support);
        }
        let result = self.inner.run_stage(stage_id, _layer, module, input)?;
        if let Some(LayerStageCommit::Support(support)) = &result {
            *self
                .commit
                .lock()
                .expect("traditional renderer commit lock must not be poisoned") =
                Some(support.clone());
        }
        Ok(result)
    }
}
