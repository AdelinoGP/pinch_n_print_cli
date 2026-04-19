//! Centralized WASM runtime dispatch for stage runners.
//!
//! Maps stage IDs to WIT export function names and provides [`WasmRuntimeDispatcher`],
//! a single struct implementing all four runner traits (`PrepassStageRunner`,
//! `LayerStageRunner`, `FinalizationStageRunner`, `PostpassStageRunner`).
//!
//! All four runner families (`PrepassStageRunner`, `LayerStageRunner`,
//! `FinalizationStageRunner`, `PostpassStageRunner`) use typed component-model
//! boundaries through `HostExecutionContext` and world-specific bindings
//! (`LayerModule`, `PrepassModule`, `FinalizationModule`, `PostpassModule`).

use std::collections::HashMap;
use std::sync::Arc;

use wasmtime::component::Resource;

use slicer_ir::{GCodeIR, GlobalLayer, LayerCollectionIR, StageId};

use crate::wit_host::{self, ConfigViewData, HostExecutionContext, PaintRegionLayerData};
use crate::{
    Blackboard, CompiledModule, FinalizationError, FinalizationOutput, FinalizationStageRunner,
    LayerArena, LayerStageError, LayerStageOutput, LayerStageRunner, PostpassError,
    PostpassOutput, PostpassStageRunner, PrepassExecutionError, PrepassStageOutput,
    PrepassStageRunner, WasmEngine,
};

/// Maps a canonical stage ID to the WIT export function name the module must provide.
///
/// This is the single source of truth for stage→export routing. The mapping
/// follows the WIT world definitions in `wit/world-*.wit`.
pub fn export_name_for_stage(stage_id: &str) -> Option<&'static str> {
    match stage_id {
        "PrePass::MeshSegmentation" => Some("run-mesh-segmentation"),
        "PrePass::MeshAnalysis" => Some("run-mesh-analysis"),
        "PrePass::LayerPlanning" => Some("run-layer-planning"),
        "PrePass::PaintSegmentation" => Some("run-paint-segmentation"),
        "Layer::Slice" => Some("run-slice"),
        "Layer::SlicePostProcess" => Some("run-slice-postprocess"),
        "Layer::Perimeters" => Some("run-perimeters"),
        "Layer::PerimetersPostProcess" => Some("run-wall-postprocess"),
        "Layer::Infill" => Some("run-infill"),
        "Layer::InfillPostProcess" => Some("run-infill-postprocess"),
        "Layer::Support" => Some("run-support"),
        "Layer::SupportPostProcess" => Some("run-support-postprocess"),
        "Layer::PathOptimization" => Some("run-path-optimization"),
        "PostPass::LayerFinalization" => Some("run-finalization"),
        "PostPass::GCodePostProcess" => Some("run-gcode-postprocess"),
        "PostPass::TextPostProcess" => Some("run-text-postprocess"),
        _ => None,
    }
}

/// Structured runtime dispatch error with full diagnostic context.
#[derive(Debug, Clone)]
pub struct DispatchError {
    /// Module identifier from manifest.
    pub module_id: String,
    /// Stage being executed.
    pub stage_id: String,
    /// Export function name that was targeted.
    pub export_name: String,
    /// Phase where the error occurred.
    pub phase: DispatchPhase,
    /// Human-readable root cause.
    pub reason: String,
}

/// Phase within the dispatch lifecycle where an error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchPhase {
    /// No compiled component available for the module.
    MissingComponent,
    /// Unknown stage ID with no export mapping.
    UnknownStage,
    /// Typed host import linker setup failed.
    LinkerSetup,
    /// Per-call execution context creation failed (resource push error).
    ContextCreation,
    /// Typed component-model instantiation through bindgen failed.
    TypedInstantiation,
    /// Typed export call through the component-model boundary failed.
    TypedExportCall,
    /// Collected guest output failed validation or arena commit.
    OutputCommit,
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dispatch error for module '{}' at stage '{}' (export '{}', phase {:?}): {}",
            self.module_id, self.stage_id, self.export_name, self.phase, self.reason
        )
    }
}

impl std::error::Error for DispatchError {}

/// Convert a borrowed resource handle into an owned handle for component calls.
fn own<T: 'static>(r: Resource<T>) -> Resource<T> {
    Resource::new_own(r.rep())
}

/// Bundled static configuration for a layer dispatch call.
struct CallConfig<'a> {
    bindings: &'a wit_host::LayerModule,
    store: &'a mut wasmtime::Store<HostExecutionContext>,
    stage_id: &'a str,
    module_id: &'a str,
    export_name: &'a str,
    config_handle: Resource<ConfigViewData>,
}

/// Bundled layer-specific parameters for a dispatch call.
struct LayerParams<'a> {
    layer_index: u32,
    layer_z: f32,
    paint_ir: Option<&'a slicer_ir::PaintRegionIR>,
    arena: &'a LayerArena,
}

/// Runtime dispatcher that invokes WASM module exports through the component model.
///
/// All four runner families (layer, prepass, finalization, postpass) use typed
/// component-model boundaries through `HostExecutionContext` and world-specific
/// bindings (`LayerModule`, `PrepassModule`, `FinalizationModule`, `PostpassModule`).
///
/// Each call:
/// 1. Resolves the stage→export mapping
/// 2. Acquires a pool slot from the module's instance pool
/// 3. Creates a per-call `HostExecutionContext`
/// 4. Instantiates the compiled component through typed bindings
/// 5. Calls the resolved typed export function
/// 6. Releases the pool slot (via RAII lease drop)
pub struct WasmRuntimeDispatcher {
    engine: Arc<WasmEngine>,
    /// Accumulated runtime reads from postpass dispatch calls.
    /// Populated by `run_gcode_postprocess` and `run_text_postprocess`,
    /// consumed by `take_runtime_reads`.
    postpass_runtime_reads: std::cell::RefCell<Vec<Vec<String>>>,
}

impl WasmRuntimeDispatcher {
    /// Create a new dispatcher backed by the given WASM engine.
    pub fn new(engine: Arc<WasmEngine>) -> Self {
        Self {
            engine,
            postpass_runtime_reads: std::cell::RefCell::new(Vec::new()),
        }
    }

    // ── Typed layer-world dispatch ─────────────────────────────────────

    /// Dispatch a layer-stage call through the typed component-model boundary.
    ///
    /// Creates a fresh `HostExecutionContext`, wires host imports via
    /// `LayerModule::add_to_linker`, instantiates through typed bindings,
    /// and calls the stage-appropriate typed export. Returns the execution
    /// context so the caller can extract and commit collected outputs.
    fn dispatch_layer_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        layer_index: u32,
        layer_z: f32,
        paint_ir: Option<&slicer_ir::PaintRegionIR>,
        arena: &LayerArena,
    ) -> Result<HostExecutionContext, DispatchError> {
        let export_name = export_name_for_stage(stage_id).ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: String::new(),
            phase: DispatchPhase::UnknownStage,
            reason: format!("no export mapping for stage '{stage_id}'"),
        })?;

        let component =
            module
                .wasm_component
                .as_ref()
                .ok_or_else(|| DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::MissingComponent,
                    reason: "no compiled WASM component available".to_string(),
                })?;

        // Acquire pool slot for concurrency control (RAII — released on drop).
        let _lease = module.instance_pool.acquire();

        let engine = self.engine.wasmtime_engine();

        // Wire typed host imports into a fresh linker.
        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx).map_err(|e| {
            DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            }
        })?;

        // Create per-call execution context and store.
        let ctx = HostExecutionContext::new(module.module_id.clone(), 0.0, 0.0, None);
        let mut store = wasmtime::Store::new(engine, ctx);

        // Push config-view resource from the module's frozen config.
        let config_handle = store
            .data_mut()
            .push_config_view(wit_host::config_view_to_data(&module.config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        // Instantiate component through typed bindings.
        let bindings = wit_host::LayerModule::instantiate(
            &mut store,
            component.wasmtime_component(),
            &linker,
        )
        .map_err(|e| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedInstantiation,
            reason: e.to_string(),
        })?;

        // Call the stage-appropriate typed export.
        let call_result = self.call_layer_export(
            CallConfig {
                bindings: &bindings,
                store: &mut store,
                stage_id,
                module_id: &module.module_id,
                export_name,
                config_handle,
            },
            LayerParams {
                layer_index,
                layer_z,
                paint_ir,
                arena,
            },
        )?;

        // Handle module-returned error (inner Result).
        call_result.map_err(|module_err| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: format!(
                "module error (code={}, fatal={}): {}",
                module_err.code, module_err.fatal, module_err.message
            ),
        })?;

        // Extract the execution context with collected outputs.
        Ok(store.into_data())
    }

    /// Route to the correct typed export based on stage ID.
    ///
    /// Each layer stage has a distinct export signature. This method pushes
    /// the stage-specific resources (output builders, paint views) and
    /// invokes the matching typed call.
    fn call_layer_export(
        &self,
        config: CallConfig<'_>,
        params: LayerParams<'_>,
    ) -> Result<Result<(), wit_host::ModuleError>, DispatchError> {
        let mk_call_err = |e: wasmtime::Error| DispatchError {
            module_id: config.module_id.to_string(),
            stage_id: config.stage_id.to_string(),
            export_name: config.export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: e.to_string(),
        };
        let mk_ctx_err = |e: wasmtime::Error| DispatchError {
            module_id: config.module_id.to_string(),
            stage_id: config.stage_id.to_string(),
            export_name: config.export_name.to_string(),
            phase: DispatchPhase::ContextCreation,
            reason: e.to_string(),
        };

        match config.stage_id {
            "Layer::Infill" => {
                let region_handles =
                    push_slice_regions(config.store, params.arena, params.layer_z).map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_infill_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_infill(config.store, params.layer_index, &region_handles, own(output), own(config.config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::InfillPostProcess" => {
                let region_handles =
                    push_perimeter_regions(config.store, params.arena).map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_infill_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_infill_postprocess(config.store, params.layer_index, &region_handles, own(output), own(config.config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::SlicePostProcess" => {
                let region_handles =
                    push_slice_regions(config.store, params.arena, params.layer_z).map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(params.paint_ir, params.layer_index);
                let paint = config.store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_slice_postprocess_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_slice_postprocess(
                        config.store,
                        params.layer_index,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::Perimeters" => {
                let region_handles =
                    push_slice_regions(config.store, params.arena, params.layer_z).map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(params.paint_ir, params.layer_index);
                let paint = config.store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_perimeter_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_perimeters(
                        config.store,
                        params.layer_index,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::PerimetersPostProcess" => {
                let region_handles =
                    push_perimeter_regions(config.store, params.arena).map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_perimeter_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_wall_postprocess(config.store, params.layer_index, &region_handles, own(output), own(config.config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::Support" => {
                let region_handles =
                    push_slice_regions(config.store, params.arena, params.layer_z).map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(params.paint_ir, params.layer_index);
                let paint = config.store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_support_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_support(
                        config.store,
                        params.layer_index,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::SupportPostProcess" => {
                let region_handles =
                    push_slice_regions(config.store, params.arena, params.layer_z).map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_support_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_support_postprocess(config.store, params.layer_index, &region_handles, own(output), own(config.config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::PathOptimization" => {
                let region_handles =
                    push_perimeter_regions(config.store, params.arena).map_err(mk_ctx_err)?;
                let output = config.store
                    .data_mut()
                    .push_gcode_output_builder()
                    .map_err(mk_ctx_err)?;
                config.bindings
                    .call_run_path_optimization(config.store, params.layer_index, &region_handles, own(output), own(config.config_handle))
                    .map_err(mk_call_err)
            }
            _ => Err(DispatchError {
                module_id: config.module_id.to_string(),
                stage_id: config.stage_id.to_string(),
                export_name: config.export_name.to_string(),
                phase: DispatchPhase::UnknownStage,
                reason: format!("no typed layer export for stage '{}'", config.stage_id),
            }),
        }
    }

    // ── Typed prepass-world dispatch ──────────────────────────────────

    /// Dispatch a prepass-stage call through the typed prepass-module boundary.
    ///
    /// `object_ids` are the canonical object identifiers from the host mesh IR
    /// (docs/02 §Canonical ID Types).  They are forwarded to the guest's
    /// `run-mesh-analysis` / `run-layer-planning` exports so that the module
    /// can iterate over real objects rather than receiving an empty list.
    ///
    /// Returns the [`HostExecutionContext`] so the caller can harvest
    /// any collected output (e.g. `layer_plan_proposals` for
    /// `PrePass::LayerPlanning`).  The store is consumed and its data
    /// moved out; no further WASM access is needed after this returns.
    fn dispatch_prepass_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<wit_host::HostExecutionContext, DispatchError> {
        let export_name = export_name_for_stage(stage_id).unwrap_or("unknown");
        let component = module.wasm_component.as_ref().ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<wit_host::HostExecutionContext>::new(engine);
        wit_host::PrepassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            })?;

        let ctx = wit_host::HostExecutionContext::new(module.module_id.clone(), 0.0, 0.0, None);
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        let bindings = wit_host::PrepassModule::instantiate(&mut store, component.wasmtime_component(), &linker)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedInstantiation,
                reason: e.to_string(),
            })?;

        let mk_call_err = |e: wasmtime::Error| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
            reason: e.to_string(),
        };
        let mk_ctx_err = |e: wasmtime::Error| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
            reason: e.to_string(),
        };

        // Build the appropriate WIT view for each stage.
        // MeshAnalysis and LayerPlanning still pass object IDs (they don't need geometry).
        // MeshSegmentation and PaintSegmentation pass geometry views.
        let call_result = match stage_id.as_str() {
            "PrePass::MeshAnalysis" => {
                let object_ids: Vec<String> = blackboard.mesh().objects.iter().map(|o| o.id.clone()).collect();
                let output = store.data_mut().push_mesh_analysis_output().map_err(mk_ctx_err)?;
                bindings.call_run_mesh_analysis(&mut store, &object_ids, own(output), own(config_handle)).map_err(mk_call_err)
            }
            "PrePass::LayerPlanning" => {
                let object_ids: Vec<String> = blackboard.mesh().objects.iter().map(|o| o.id.clone()).collect();
                let output = store.data_mut().push_layer_plan_output().map_err(mk_ctx_err)?;
                bindings.call_run_layer_planning(&mut store, &object_ids, own(output), own(config_handle)).map_err(mk_call_err)
            }
            "PrePass::MeshSegmentation" => {
                let mesh_object_views: Vec<_> = blackboard.mesh().objects.iter().map(|obj| {
                    wit_host::object_mesh_to_wit_mesh_object_view(obj)
                }).collect();
                let output = store.data_mut().push_mesh_segmentation_output().map_err(mk_ctx_err)?;
                bindings.call_run_mesh_segmentation(&mut store, &mesh_object_views, own(output), own(config_handle)).map_err(mk_call_err)
            }
            "PrePass::PaintSegmentation" => {
                let layer_plan = blackboard.layer_plan();
                let paint_object_views: Vec<_> = blackboard.mesh().objects.iter().map(|obj| {
                    let participating_layers = layer_plan
                        .as_ref()
                        .and_then(|lp| lp.object_participation.get(&obj.id))
                        .map(|refs| refs.iter().map(|r| r.global_layer_index).collect::<Vec<u32>>())
                        .unwrap_or_default();
                    wit_host::object_mesh_to_wit_paint_segmentation_view(obj, &participating_layers)
                }).collect();
                let output = store.data_mut().push_paint_segmentation_output().map_err(mk_ctx_err)?;
                bindings.call_run_paint_segmentation(&mut store, &paint_object_views, own(output), own(config_handle)).map_err(mk_call_err)
            }
            _ => Err(DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::UnknownStage,
                reason: format!("no typed prepass export for stage '{stage_id}'"),
            }),
        }?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
            reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
        })?;

        // Move the execution context out of the store so the caller can harvest
        // any collected output (layer proposals, log messages, etc.).
        Ok(store.into_data())
    }

    // ── Typed finalization-world dispatch ──────────────────────────────

    /// Dispatch a finalization-stage call through the typed finalization-module boundary.
    ///
    /// Returns the captured `FinalizationBuilderPush` stream the guest
    /// emitted through `push-entity-to-layer` / `insert-synthetic-layer`.
    /// `layers` is deep-copied into one `LayerCollectionView` resource
    /// per completed IR so the guest observes real per-layer metadata.
    fn dispatch_finalization_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        layers: &[slicer_ir::LayerCollectionIR],
    ) -> Result<Vec<wit_host::FinalizationBuilderPush>, DispatchError> {
        let export_name = export_name_for_stage(stage_id).unwrap_or("unknown");
        let component = module.wasm_component.as_ref().ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::FinalizationModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            })?;

        let ctx = HostExecutionContext::new(module.module_id.clone(), 0.0, 0.0, None);
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        let output_handle = store.data_mut().push_finalization_output_builder()
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push finalization output resource: {e}"),
            })?;

        // Deep-copy each completed layer into a wit-bindgen
        // LayerCollectionView handle so the guest sees real metadata
        // rather than the previous empty-shell stub (docs/03
        // world-finalization.wit `resource layer-collection-view`).
        let mut layer_handles = Vec::with_capacity(layers.len());
        for layer in layers {
            let h = store.data_mut().push_finalization_layer_view(layer).map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push layer-collection-view resource: {e}"),
            })?;
            layer_handles.push(own(h));
        }

        let bindings = wit_host::FinalizationModule::instantiate(&mut store, component.wasmtime_component(), &linker)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedInstantiation,
                reason: e.to_string(),
            })?;

        let call_result = bindings
            .call_run_finalization(&mut store, &layer_handles, own(output_handle), own(config_handle))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
                reason: e.to_string(),
            })?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
            reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
        })?;

        // Drain the builder's captured pushes so the caller can apply
        // them to the downstream layer collection. The resource has
        // already been dropped by the guest at this point; the drop
        // handler moved its pushes onto the HostExecutionContext.
        Ok(store.data_mut().drain_finalization_output_builder())
    }

    // ── Typed postpass-world dispatch ──────────────────────────────────

    /// Dispatch a postpass gcode-postprocess call through the typed postpass-module boundary.
    /// Returns `(Result<(), DispatchError>, Vec<String>)` where the second element
    /// is the collected `runtime_reads` from the WIT view calls made during this dispatch.
    fn dispatch_postpass_gcode_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
    ) -> (Result<(), DispatchError>, Vec<String>) {
        let export_name = "run-gcode-postprocess";
        let component = match module.wasm_component.as_ref() {
            Some(c) => c,
            None => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::MissingComponent,
                    reason: "no compiled WASM component available".to_string(),
                }),
                Vec::new(),
            ),
        };

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        if let Err(e) = wit_host::PostpassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx) {
            return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::LinkerSetup,
                    reason: e.to_string(),
                }),
                Vec::new(),
            );
        }

        let ctx = HostExecutionContext::new(module.module_id.clone(), 0.0, 0.0, None);
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = match store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view)) {
            Ok(h) => h,
            Err(e) => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::ContextCreation,
                    reason: format!("failed to push config resource: {e}"),
                }),
                Vec::new(),
            ),
        };
        let output_handle = match store.data_mut().push_postpass_gcode_output_builder() {
            Ok(h) => h,
            Err(e) => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::ContextCreation,
                    reason: format!("failed to push gcode output resource: {e}"),
                }),
                Vec::new(),
            ),
        };

        let bindings = match wit_host::PostpassModule::instantiate(&mut store, component.wasmtime_component(), &linker) {
            Ok(b) => b,
            Err(e) => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedInstantiation,
                    reason: e.to_string(),
                }),
                Vec::new(),
            ),
        };

        let call_result = bindings.call_run_gcode_postprocess(&mut store, &[], own(output_handle), own(config_handle));
        let runtime_reads = store.data().runtime_reads.clone();

        match call_result {
            Ok(Ok(())) => (Ok(()), runtime_reads),
            Ok(Err(module_err)) => (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedExportCall,
                    reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
                }),
                runtime_reads,
            ),
            Err(e) => (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedExportCall,
                    reason: e.to_string(),
                }),
                runtime_reads,
            ),
        }
    }

    /// Dispatch a postpass text-postprocess call through the typed postpass-module boundary.
    /// Returns `(Result<String, DispatchError>, Vec<String>)` where the second element
    /// is the collected `runtime_reads` from the WIT view calls made during this dispatch.
    fn dispatch_postpass_text_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        text: &str,
    ) -> (Result<String, DispatchError>, Vec<String>) {
        let export_name = "run-text-postprocess";
        let component = match module.wasm_component.as_ref() {
            Some(c) => c,
            None => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::MissingComponent,
                    reason: "no compiled WASM component available".to_string(),
                }),
                Vec::new(),
            ),
        };

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        if let Err(e) = wit_host::PostpassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |ctx| ctx) {
            return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::LinkerSetup,
                    reason: e.to_string(),
                }),
                Vec::new(),
            );
        }

        let ctx = HostExecutionContext::new(module.module_id.clone(), 0.0, 0.0, None);
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = match store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view)) {
            Ok(h) => h,
            Err(e) => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::ContextCreation,
                    reason: format!("failed to push config resource: {e}"),
                }),
                Vec::new(),
            ),
        };

        let bindings = match wit_host::PostpassModule::instantiate(&mut store, component.wasmtime_component(), &linker) {
            Ok(b) => b,
            Err(e) => return (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedInstantiation,
                    reason: e.to_string(),
                }),
                Vec::new(),
            ),
        };

        let call_result = bindings.call_run_text_postprocess(&mut store, text, own(config_handle));
        let runtime_reads = store.data().runtime_reads.clone();

        match call_result {
            Ok(Ok(result_text)) => (Ok(result_text), runtime_reads),
            Ok(Err(module_err)) => (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedExportCall,
                    reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
                }),
                runtime_reads,
            ),
            Err(e) => (
                Err(DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedExportCall,
                    reason: e.to_string(),
                }),
                runtime_reads,
            ),
        }
    }
}

/// Build `PaintRegionLayerData` from an optional `PaintRegionIR`.
///
/// If the IR is `None` (no paint segmentation was run), returns empty-but-valid data.
/// This is the correct behavior per docs: empty paint input is valid when no
/// paint was applied to the scene.
fn build_paint_layer_data(
    paint_ir: Option<&slicer_ir::PaintRegionIR>,
    layer_index: u32,
) -> PaintRegionLayerData {
    match paint_ir {
        Some(ir) => wit_host::paint_region_ir_to_layer_data(ir, layer_index),
        None => PaintRegionLayerData {
            layer_index,
            regions_by_semantic: HashMap::new(),
            custom_regions: HashMap::new(),
        },
    }
}

/// Push `SliceRegionData` resources into the store from the arena's `SliceIR`.
///
/// Returns resource handles for each `SlicedRegion`. Returns an empty vec
/// if no `SliceIR` is staged in the arena (valid for stages that run before
/// `Layer::Slice`, or when the slice stage produced no output).
fn push_slice_regions(
    store: &mut wasmtime::Store<HostExecutionContext>,
    arena: &LayerArena,
    layer_z: f32,
) -> Result<Vec<Resource<wit_host::SliceRegionData>>, wasmtime::Error> {
    let slice_ir = match arena.slice() {
        Some(ir) => ir,
        None => return Ok(Vec::new()),
    };

    let mut handles = Vec::with_capacity(slice_ir.regions.len());
    for region in &slice_ir.regions {
        let data = wit_host::sliced_region_to_data(region, layer_z);
        let handle = store.data_mut().push_slice_region(data)?;
        handles.push(handle);
    }
    Ok(handles)
}

/// Push `PerimeterRegionData` resources into the store from the arena's `PerimeterIR`.
///
/// Returns resource handles for each `PerimeterRegion`. Returns an empty vec
/// if no `PerimeterIR` is staged in the arena (valid for stages run before
/// `Layer::Perimeters`, or when the perimeter stage produced no walls).
fn push_perimeter_regions(
    store: &mut wasmtime::Store<HostExecutionContext>,
    arena: &LayerArena,
) -> Result<Vec<Resource<wit_host::PerimeterRegionData>>, wasmtime::Error> {
    let perimeter_ir = match arena.perimeter() {
        Some(ir) => ir,
        None => return Ok(Vec::new()),
    };

    let mut handles = Vec::with_capacity(perimeter_ir.regions.len());
    for region in &perimeter_ir.regions {
        let data = wit_host::perimeter_region_to_data(region);
        let handle = store.data_mut().push_perimeter_region(data)?;
        handles.push(handle);
    }
    Ok(handles)
}

// ── Layer-plan harvest ────────────────────────────────────────────────────

/// Convert WIT `LayerProposal` records collected by a `PrePass::LayerPlanning`
/// call into a host-side [`slicer_ir::LayerPlanIR`].
///
/// # Region-ID canonicalization
///
/// WIT `region-id` is declared as a string (docs/02 §Canonical ID Types).
/// The canonical host form is a decimal `u64` string with no leading zeros.
/// Modules that emit non-canonical IDs (e.g. `"default"`) receive a
/// deterministic stable hash so the pipeline can proceed; the non-canonical
/// value is logged as a diagnostic warning.  This matches the "harvest what
/// the component actually returns" contract while avoiding a fatal error on
/// the known `layer-planner-default` fallback case.
///
/// # Validation
///
/// - `z` must be finite and non-negative (enforced in `push_layer`).
/// - `effective_layer_height` must be finite and positive (enforced in `push_layer`).
/// - `GlobalLayer.index` must be `< 100_000` (docs/02 §Bounds).
fn harvest_layer_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: wit_host::HostExecutionContext,
) -> Result<slicer_ir::LayerPlanIR, String> {
    use slicer_ir::{ActiveRegion, GlobalLayer, LayerPlanIR, ObjectLayerRef, ResolvedConfig, SemVer};
    use std::collections::HashMap;

    let proposals = ctx.layer_plan_proposals;

    const MAX_LAYERS: u32 = 100_000;

    let mut global_layers: Vec<GlobalLayer> = Vec::with_capacity(proposals.len());
    // object_id → Vec<ObjectLayerRef>
    let mut object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::new();

    for (idx, proposal) in proposals.into_iter().enumerate() {
        let index = idx as u32;
        if index >= MAX_LAYERS {
            return Err(format!(
                "layer-plan-output: layer count exceeded maximum budget of {MAX_LAYERS}"
            ));
        }

        let mut active_regions: Vec<ActiveRegion> = Vec::new();

        for region_prop in proposal.active_regions {
            // Parse region_id: canonical = decimal u64, no leading zeros.
            // Non-canonical strings get a stable FNV-1a hash as a fallback.
            let region_id: u64 = match region_prop.region_id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    // Stable hash for determinism across identical runs.
                    fnv1a_string_to_u64(&region_prop.region_id)
                }
            };

            active_regions.push(ActiveRegion {
                object_id: region_prop.object_id.clone(),
                region_id,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: region_prop.effective_layer_height,
                nonplanar_shell: None,
                is_catchup_layer: region_prop.is_catchup,
                catchup_z_bottom: region_prop.catchup_z_bottom,
                tool_index: 0,
            });

            // Track per-object layer participation.
            let obj_refs = object_participation
                .entry(region_prop.object_id.clone())
                .or_default();

            // Avoid duplicate entries for the same (object, global_layer) pair
            // when multiple regions from the same object appear in one layer.
            let already_referenced = obj_refs
                .iter()
                .any(|r| r.global_layer_index == index);
            if !already_referenced {
                obj_refs.push(ObjectLayerRef {
                    local_layer_index: obj_refs.len() as u32,
                    global_layer_index: index,
                    effective_layer_height: region_prop.effective_layer_height,
                });
            }
        }

        global_layers.push(GlobalLayer {
            index,
            z: proposal.z,
            active_regions,
            has_nonplanar: false,
            is_sync_layer: false,
        });
    }

    Ok(LayerPlanIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        global_layers,
        object_participation,
    })
}

/// Harvest `push-paint-region` entries collected by a prepass
/// `run-paint-segmentation` invocation into a `PaintRegionIR`.
///
/// Reshapes each flat WIT `paint-region-entry` into the blackboard's
/// structured form: entries are grouped by `(layer_index,
/// paint_semantic)` and appended to
/// `PaintRegionIR.per_layer[layer].semantic_regions[semantic]` in
/// insertion order. `paint_order` is derived from that insertion index
/// so every `SemanticRegion` carries a deterministic rank within its
/// semantic bucket (docs/02 §Paint Region IR — `paint_order` stability).
/// `PaintValue` is parsed from the WIT `value: string` field with the
/// following conventions:
///   - `"true"`/`"false"`                    → `PaintValue::Flag(bool)`
///   - parseable as `u32` (e.g. `"0"`, `"3"`) → `PaintValue::ToolIndex(u32)`
///   - anything else                          → `PaintValue::Named(String)`
///
/// Unknown semantic strings map to `PaintSemantic::Custom(name)` so
/// guests can introduce new semantics without host-side changes.
fn harvest_paint_segmentation_ir(
    ctx: wit_host::HostExecutionContext,
) -> slicer_ir::PaintRegionIR {
    use std::collections::HashMap;
    use slicer_ir::{
        ExPolygon, LayerPaintMap, PaintRegionIR, PaintSemantic, PaintValue, Point2, Polygon,
        SemanticRegion, SemVer,
    };

    let parse_semantic = |s: &str| -> PaintSemantic {
        match s {
            "material" | "Material" => PaintSemantic::Material,
            "fuzzy_skin" | "FuzzySkin" => PaintSemantic::FuzzySkin,
            "support_enforcer" | "SupportEnforcer" => PaintSemantic::SupportEnforcer,
            "support_blocker" | "SupportBlocker" => PaintSemantic::SupportBlocker,
            other => PaintSemantic::Custom(other.to_string()),
        }
    };
    // WIT value → IR PaintValue. The IR enum has `Flag(bool)`,
    // `Scalar(f32)`, `ToolIndex(u32)` — no free-form string variant.
    // Guests that need named values should emit them as ToolIndex or
    // Scalar; unrecognized strings degrade to ToolIndex(0) so the
    // channel stays observable.
    let parse_value = |s: &str| -> PaintValue {
        if s.eq_ignore_ascii_case("true") {
            PaintValue::Flag(true)
        } else if s.eq_ignore_ascii_case("false") {
            PaintValue::Flag(false)
        } else if let Ok(n) = s.parse::<u32>() {
            PaintValue::ToolIndex(n)
        } else if let Ok(f) = s.parse::<f32>() {
            PaintValue::Scalar(f)
        } else {
            PaintValue::ToolIndex(0)
        }
    };

    let mut per_layer: HashMap<u32, LayerPaintMap> = HashMap::new();
    for (idx, entry) in ctx.paint_region_entries.into_iter().enumerate() {
        let layer_index = entry.layer_index;
        let layer = per_layer
            .entry(layer_index)
            .or_insert_with(|| LayerPaintMap {
                global_layer_index: layer_index,
                semantic_regions: HashMap::new(),
            });
        let semantic = parse_semantic(&entry.semantic);
        let polygons: Vec<ExPolygon> = entry
            .polygons
            .iter()
            .map(|ep| ExPolygon {
                contour: Polygon {
                    points: ep
                        .contour
                        .points
                        .iter()
                        .map(|pt| Point2 { x: pt.x, y: pt.y })
                        .collect(),
                },
                holes: ep
                    .holes
                    .iter()
                    .map(|h| Polygon {
                        points: h
                            .points
                            .iter()
                            .map(|pt| Point2 { x: pt.x, y: pt.y })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        let value = parse_value(&entry.value);
        layer
            .semantic_regions
            .entry(semantic)
            .or_default()
            .push(SemanticRegion {
                object_id: entry.object_id,
                polygons,
                value,
                paint_order: idx as u64,
            });
    }

    PaintRegionIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        per_layer,
    }
}

/// Harvest `mark-triangle-paint` tuples collected by a prepass
/// `run-mesh-segmentation` invocation into a `MeshSegmentationIR`.
///
/// Ordering follows the guest's insertion order (deterministic for any
/// single guest implementation). Values pass through verbatim — this
/// helper only reshapes the tuples into `FacetPaintMark` records;
/// per-field validation is done at the resource-push site in
/// `HostMeshSegmentationOutput::mark_triangle_paint`.
fn harvest_mesh_segmentation_ir(
    ctx: wit_host::HostExecutionContext,
) -> slicer_ir::MeshSegmentationIR {
    use slicer_ir::{FacetPaintMark, MeshSegmentationIR, SemVer};

    let marks: Vec<FacetPaintMark> = ctx
        .mesh_segmentation_marks
        .into_iter()
        .map(|(object_id, facet_index, semantic, value)| FacetPaintMark {
            object_id,
            facet_index,
            semantic,
            value,
        })
        .collect();

    MeshSegmentationIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        marks,
    }
}

/// Stable FNV-1a 64-bit hash of a string.
///
/// Used to derive a deterministic `RegionId` from non-canonical WIT
/// `region-id` strings (e.g., `"default"` emitted by the layer-planner
/// fallback path).  Identical strings always produce identical hashes
/// across runs, satisfying the determinism contract.
fn fnv1a_string_to_u64(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    let mut hash: u64 = FNV_OFFSET;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ── Stage runner trait implementations ──────────────────────────────────

impl PrepassStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        // Extract canonical object IDs from the blackboard's mesh IR so that
        // the guest export receives the real object list (docs/02 §MeshIR).
        // An empty list is valid (no-op for the module) but must never be
        // hard-coded; the mesh is the authoritative source.
        // Note: object_ids is computed here for MeshAnalysis/LayerPlanning but
        // the actual object data flow is handled inside dispatch_prepass_call.

        let ctx = match self.dispatch_prepass_call(stage_id, module, blackboard) {
            Ok(ctx) => ctx,
            Err(e) => {
                // A `MissingComponent` error means the module's `.wasm` was a
                // placeholder or could not be compiled.  The load path already
                // emitted a structured warning for it.  Treat it as a graceful
                // skip — return `None` so the prepass continues without this
                // module's output.  Any other error is genuinely fatal.
                if e.phase == DispatchPhase::MissingComponent {
                    return Ok((PrepassStageOutput::None, Vec::new()));
                }
                return Err(PrepassExecutionError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        // Preserve runtime reads before consuming the context.
        let runtime_reads: Vec<String> = ctx.runtime_reads.clone();

        // For the LayerPlanning stage, convert collected proposals to LayerPlanIR.
        if stage_id == "PrePass::LayerPlanning" {
            let ir = harvest_layer_plan_ir(stage_id, &module.module_id, ctx)
                .map_err(|e| PrepassExecutionError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e,
                })?;
            return Ok((PrepassStageOutput::LayerPlan(Arc::new(ir)), runtime_reads));
        }

        // For the MeshSegmentation stage, convert collected triangle paint
        // marks to MeshSegmentationIR.
        if stage_id == "PrePass::MeshSegmentation" {
            let ir = harvest_mesh_segmentation_ir(ctx);
            return Ok((PrepassStageOutput::MeshSegmentation(Arc::new(ir)), runtime_reads));
        }

        // For the PaintSegmentation stage, convert collected paint-region
        // entries to PaintRegionIR.
        if stage_id == "PrePass::PaintSegmentation" {
            let ir = harvest_paint_segmentation_ir(ctx);
            return Ok((PrepassStageOutput::PaintRegions(Arc::new(ir)), runtime_reads));
        }

        // For the MeshAnalysis stage, surface any guest-emitted
        // annotations / surface groups via `MeshAnalysisAuxiliary` when
        // the drain is non-empty. A guest that pushed nothing still
        // returns `None` so the existing empty-drain contract (and its
        // regression tests) are preserved — the new variant is only
        // raised when there is real output to observe.
        if stage_id == "PrePass::MeshAnalysis" {
            let aux = harvest_mesh_analysis_auxiliary(ctx);
            if aux.facet_annotations.is_empty() && aux.surface_groups.is_empty() {
                return Ok((PrepassStageOutput::None, runtime_reads));
            }
            return Ok((PrepassStageOutput::MeshAnalysisAuxiliary(Arc::new(aux)), runtime_reads));
        }

        Ok((PrepassStageOutput::None, runtime_reads))
    }
}

/// Convert the `(object_id, FacetAnnotation)` / `(object_id, SurfaceGroupProposal)`
/// pushes collected on the `HostExecutionContext` into the host-local
/// `MeshAnalysisAuxiliary` record. Order matches guest push order.
fn harvest_mesh_analysis_auxiliary(
    ctx: wit_host::HostExecutionContext,
) -> crate::prepass::MeshAnalysisAuxiliary {
    use crate::prepass::{FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, SurfaceGroupRecord};
    use crate::wit_host::prepass as pm;

    let facet_annotations = ctx
        .mesh_analysis_annotations
        .into_iter()
        .map(|(obj, ann)| {
            let classification = match ann.classification {
                pm::FacetClass::Normal => FacetClassRecord::Normal,
                pm::FacetClass::NearHorizontal => FacetClassRecord::NearHorizontal,
                pm::FacetClass::Overhang => FacetClassRecord::Overhang,
                pm::FacetClass::Bridge => FacetClassRecord::Bridge,
                pm::FacetClass::TopSurface => FacetClassRecord::TopSurface,
                pm::FacetClass::BottomSurface => FacetClassRecord::BottomSurface,
            };
            (
                obj,
                FacetAnnotationRecord {
                    facet_index: ann.facet_index,
                    slope_angle_deg: ann.slope_angle_deg,
                    classification,
                },
            )
        })
        .collect();

    let surface_groups = ctx
        .mesh_analysis_surface_groups
        .into_iter()
        .map(|(obj, grp)| {
            (
                obj,
                SurfaceGroupRecord {
                    facet_indices: grp.facet_indices,
                    z_min: grp.z_min,
                    z_max: grp.z_max,
                    shell_count: grp.shell_count,
                },
            )
        })
        .collect();

    MeshAnalysisAuxiliary {
        facet_annotations,
        surface_groups,
    }
}

impl LayerStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>), LayerStageError> {
        // Extract paint region IR from blackboard for paint-consuming stages.
        let paint_ir = blackboard.paint_regions();
        let paint_ref = paint_ir.map(|arc| arc.as_ref());

        // Layer stages always use the typed component-model boundary.
        let ctx = match self.dispatch_layer_call(stage_id, module, layer.index, layer.z, paint_ref, arena) {
            Ok(ctx) => ctx,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                // Placeholder/uncompiled module — skip gracefully.
                return Ok((LayerStageOutput::Success, Vec::new()));
            }
            Err(e) => {
                return Err(LayerStageError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        // Preserve runtime reads before committing outputs.
        let runtime_reads: Vec<String> = ctx.runtime_reads.clone();

        // Commit collected outputs into the layer arena based on stage.
        commit_layer_outputs(stage_id, &module.module_id, layer.index, &ctx, arena)?;

        Ok((LayerStageOutput::Success, runtime_reads))
    }
}

/// Commit collected guest outputs from `HostExecutionContext` into the `LayerArena`.
///
/// Each layer stage writes to a specific arena slot:
/// - Infill/InfillPostProcess → `set_infill`
/// - Support/SupportPostProcess → `set_support`
/// - Perimeters/PerimetersPostProcess → `set_perimeter`
/// - SlicePostProcess → `set_slice`
///
/// PathOptimization produces GCode output, which is not an arena slot — it is
/// collected but deferred until the LayerCollectionIR finalization step.
///
/// Empty-but-valid output (no paths emitted) skips the commit without error.
/// Invalid output (NaN/Inf, cardinality mismatch) fails with a structured diagnostic.
/// Test-only wrapper around the private `commit_layer_outputs` so integration
/// tests can exercise the PathOptimization GCode-override rejection path
/// without compiling a bespoke WAT guest per case.
#[doc(hidden)]
pub fn commit_layer_outputs_for_test(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    ctx: &HostExecutionContext,
    arena: &mut LayerArena,
) -> Result<(), LayerStageError> {
    commit_layer_outputs(stage_id, module_id, layer_index, ctx, arena)
}

fn commit_layer_outputs(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    ctx: &HostExecutionContext,
    arena: &mut LayerArena,
) -> Result<(), LayerStageError> {
    let mk_validation_err = |what: &str, reason: String| LayerStageError::FatalModule {
        stage_id: stage_id.to_string(),
        module_id: module_id.to_string(),
        message: format!("invalid {what} output: {reason}"),
    };

    match stage_id {
        "Layer::Infill" | "Layer::InfillPostProcess" => {
            let infill = &ctx.infill_output;
            if infill.sparse_paths.is_empty()
                && infill.solid_paths.is_empty()
                && infill.ironing_paths.is_empty()
            {
                return Ok(());
            }
            let ir = wit_host::convert_infill_output(infill, layer_index)
                .map_err(|r| mk_validation_err("infill", r))?;
            if stage_id == "Layer::InfillPostProcess" {
                let _ = arena.take_infill();
            }
            arena
                .set_infill(ir)
                .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
        }
        "Layer::Support" | "Layer::SupportPostProcess" => {
            let support = &ctx.support_output;
            if support.support_paths.is_empty()
                && support.interface_paths.is_empty()
                && support.raft_paths.is_empty()
            {
                return Ok(());
            }
            let ir = wit_host::convert_support_output(support, layer_index)
                .map_err(|r| mk_validation_err("support", r))?;
            if stage_id == "Layer::SupportPostProcess" {
                let _ = arena.take_support();
            }
            arena
                .set_support(ir)
                .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
        }
        "Layer::Perimeters" | "Layer::PerimetersPostProcess" => {
            let perimeter = &ctx.perimeter_output;
            if perimeter.wall_loops.is_empty()
                && perimeter.infill_areas.is_empty()
                && perimeter.seam_candidates.is_empty()
            {
                return Ok(());
            }
            let ir = wit_host::convert_perimeter_output(perimeter, layer_index)
                .map_err(|r| mk_validation_err("perimeter", r))?;
            if stage_id == "Layer::PerimetersPostProcess" {
                let _ = arena.take_perimeter();
            }
            arena
                .set_perimeter(ir)
                .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
        }
        "Layer::SlicePostProcess" => {
            let sp = &ctx.slice_postprocess_output;
            if sp.polygon_updates.is_empty() && sp.path_z_updates.is_empty() {
                return Ok(());
            }
            // Identity-preserving merge into existing SliceIR: take the staged
            // SliceIR, apply per-region updates keyed by `(object_id, region_id)`,
            // and restage. Regions not mentioned by the guest pass through
            // unchanged so the full per-region shape survives to downstream
            // consumers (push_slice_regions on later stages sees every region).
            let existing = arena.take_slice().ok_or_else(|| LayerStageError::FatalModule {
                stage_id: stage_id.to_string(),
                module_id: module_id.to_string(),
                message: "Layer::SlicePostProcess has no staged SliceIR to merge into; \
                          Layer::Slice must commit per-region slice output first".into(),
            })?;
            let merged = wit_host::merge_slice_postprocess_into(existing, sp)
                .map_err(|r| mk_validation_err("slice postprocess", r))?;
            arena
                .set_slice(merged)
                .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
        }
        "Layer::PathOptimization" => {
            // PathOptimization runs after ordered_entities have been pre-staged
            // into arena.layer_collection. The guest observes the layer via
            // perimeter-region-view and may emit overrides through the
            // gcode-output-builder. Today we accept tool-change, comment, and
            // raw overrides; tool-changes are folded into the final
            // LayerCollectionIR.tool_changes. Move/Retract/FanSpeed/Temperature
            // have no documented LayerCollectionIR mapping and are rejected
            // with a structured diagnostic rather than being silently dropped.
            use wit_host::GcodeCommandCollected;
            // The executor pre-stages `layer_collection` when running the full
            // per-layer loop. Direct-dispatch tests or plans without an
            // executor-owned arena may call this path with no staged IR; in
            // that case `anchor` defaults to 0 and tool-changes are routed
            // into the deferred queue for whatever downstream consumer reads
            // them, instead of failing eagerly.
            let anchor = arena
                .layer_collection()
                .map(|lc| lc.ordered_entities.len().saturating_sub(1) as u32)
                .unwrap_or(0);
            let entity_count = arena
                .layer_collection()
                .map(|lc| lc.ordered_entities.len() as u32)
                .unwrap_or(0);
            let mut accepted: Vec<slicer_ir::ToolChange> = Vec::new();
            let mut accepted_z_hops: Vec<slicer_ir::ZHop> = Vec::new();
            for (i, cmd) in ctx.gcode_output.commands.iter().enumerate() {
                match cmd {
                    GcodeCommandCollected::ToolChange { from_tool, to_tool } => {
                        accepted.push(slicer_ir::ToolChange {
                            after_entity_index: anchor,
                            from_tool: *from_tool,
                            to_tool: *to_tool,
                        });
                    }
                    GcodeCommandCollected::Comment(text) => {
                        arena.push_deferred_annotation(slicer_ir::LayerAnnotation {
                            after_entity_index: anchor,
                            kind: slicer_ir::LayerAnnotationKind::Comment(text.clone()),
                        });
                    }
                    GcodeCommandCollected::Raw(text) => {
                        arena.push_deferred_annotation(slicer_ir::LayerAnnotation {
                            after_entity_index: anchor,
                            kind: slicer_ir::LayerAnnotationKind::Raw(text.clone()),
                        });
                    }
                    GcodeCommandCollected::ZHop { after_entity_index, hop_height } => {
                        // Validation per docs/03 § z-hops:
                        // - after-entity-index in bounds (or 0 for empty layers)
                        // - hop-height finite and strictly > 0
                        if entity_count == 0 {
                            if *after_entity_index != 0 {
                                return Err(LayerStageError::FatalModule {
                                    stage_id: stage_id.to_string(),
                                    module_id: module_id.to_string(),
                                    message: format!(
                                        "Layer::PathOptimization push-z-hop call {i} rejected: \
                                         after-entity-index={after_entity_index} but ordered_entities is empty (must be 0)"
                                    ),
                                });
                            }
                        } else if *after_entity_index >= entity_count {
                            return Err(LayerStageError::FatalModule {
                                stage_id: stage_id.to_string(),
                                module_id: module_id.to_string(),
                                message: format!(
                                    "Layer::PathOptimization push-z-hop call {i} rejected: \
                                     after-entity-index={after_entity_index} out of bounds for ordered_entities.len()={entity_count}"
                                ),
                            });
                        }
                        if !hop_height.is_finite() || *hop_height <= 0.0 {
                            return Err(LayerStageError::FatalModule {
                                stage_id: stage_id.to_string(),
                                module_id: module_id.to_string(),
                                message: format!(
                                    "Layer::PathOptimization push-z-hop call {i} rejected: \
                                     hop-height={hop_height} is not finite and strictly positive"
                                ),
                            });
                        }
                        accepted_z_hops.push(slicer_ir::ZHop {
                            after_entity_index: *after_entity_index,
                            hop_height: *hop_height,
                        });
                    }
                    other => {
                        return Err(LayerStageError::FatalModule {
                            stage_id: stage_id.to_string(),
                            module_id: module_id.to_string(),
                            message: format!(
                                "Layer::PathOptimization guest emitted unsupported GCode command at index {i} ({:?}); \
                                 only tool-change/comment/raw are documented overrides for the LayerCollectionIR commit path",
                                std::mem::discriminant(other)
                            ),
                        });
                    }
                }
            }
            for tc in accepted {
                arena.push_deferred_tool_change(tc);
            }
            for zh in accepted_z_hops {
                arena.push_deferred_z_hop(zh);
            }
        }
        _ => {}
    }
    Ok(())
}

impl FinalizationStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        let pushes = match self.dispatch_finalization_call(stage_id, module, layers) {
            Ok(p) => p,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                // Placeholder/uncompiled modules are gracefully skipped.
                return Ok(FinalizationOutput::Success);
            }
            Err(e) => {
                return Err(FinalizationError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        // Apply guest-emitted pushes to the downstream layer collection.
        // `push-entity-to-layer` appends ordered extrusion entities to
        // the targeted existing layer; `insert-synthetic-layer` creates
        // a new layer at the requested Z with the supplied extrusion
        // paths (docs/03 world-finalization.wit §finalization-output-builder).
        // Ordering is preserved: pushes run in guest-emission order,
        // which — for a deterministic guest — gives byte-stable output.
        for push in pushes {
            match push {
                wit_host::FinalizationBuilderPush::EntityToLayer { layer_index, path, region_key } => {
                    if let Some(target) = layers.iter_mut().find(|l| l.global_layer_index == layer_index) {
                        let role = path.role.clone();
                        let topo_order = target.ordered_entities.len() as u32;
                        target.ordered_entities.push(slicer_ir::PrintEntity {
                            path,
                            role,
                            region_key,
                            topo_order,
                        });
                    }
                }
                wit_host::FinalizationBuilderPush::SyntheticLayer { z, paths } => {
                    let new_index = layers.len() as u32;
                    let entities: Vec<_> = paths
                        .into_iter()
                        .enumerate()
                        .map(|(i, path)| {
                            let role = path.role.clone();
                            slicer_ir::PrintEntity {
                                path,
                                role,
                                region_key: slicer_ir::RegionKey {
                                    global_layer_index: new_index,
                                    object_id: String::new(),
                                    region_id: 0,
                                },
                                topo_order: i as u32,
                            }
                        })
                        .collect();
                    layers.push(LayerCollectionIR {
                        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
                        global_layer_index: new_index,
                        z,
                        ordered_entities: entities,
                        tool_changes: Vec::new(),
                        z_hops: Vec::new(),
                        annotations: Vec::new(),
                    });
                }
            }
        }

        Ok(FinalizationOutput::Success)
    }
}

impl PostpassStageRunner for WasmRuntimeDispatcher {
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        let (result, reads) = self.dispatch_postpass_gcode_call(stage_id, module);
        // Store reads for later retrieval via take_runtime_reads
        if !reads.is_empty() {
            self.postpass_runtime_reads.borrow_mut().push(reads);
        }
        match result {
            Ok(()) => {}
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                return Ok(PostpassOutput::GCodeSuccess);
            }
            Err(e) => {
                return Err(PostpassError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        }

        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        let (result, reads) = self.dispatch_postpass_text_call(stage_id, module, &text);
        // Store reads for later retrieval via take_runtime_reads
        if !reads.is_empty() {
            self.postpass_runtime_reads.borrow_mut().push(reads);
        }
        match result {
            Ok(result_text) => Ok(PostpassOutput::TextSuccess { text: result_text }),
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                // Placeholder module: pass text through unchanged.
                Ok(PostpassOutput::TextSuccess { text })
            }
            Err(e) => Err(PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            }),
        }
    }

    fn take_runtime_reads(&mut self) -> Vec<Vec<String>> {
        self.postpass_runtime_reads.borrow_mut().drain(..).collect()
    }
}

// Safety: WasmRuntimeDispatcher is Sync because WasmEngine (wrapping wasmtime::Engine)
// is Send+Sync, and all mutable state is created per-call (not shared).
unsafe impl Sync for WasmRuntimeDispatcher {}
