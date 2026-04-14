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
}

impl WasmRuntimeDispatcher {
    /// Create a new dispatcher backed by the given WASM engine.
    pub fn new(engine: Arc<WasmEngine>) -> Self {
        Self { engine }
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
        wit_host::LayerModule::add_to_linker(&mut linker, |ctx| ctx).map_err(|e| {
            DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            }
        })?;

        // Create per-call execution context and store.
        let ctx = HostExecutionContext::new(module.module_id.clone());
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
        let (bindings, _) = wit_host::LayerModule::instantiate(
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
            &bindings,
            &mut store,
            stage_id,
            &module.module_id,
            export_name,
            config_handle,
            layer_index,
            layer_z,
            paint_ir,
            arena,
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
        bindings: &wit_host::LayerModule,
        store: &mut wasmtime::Store<HostExecutionContext>,
        stage_id: &str,
        module_id: &str,
        export_name: &str,
        config_handle: Resource<ConfigViewData>,
        layer_index: u32,
        layer_z: f32,
        paint_ir: Option<&slicer_ir::PaintRegionIR>,
        arena: &LayerArena,
    ) -> Result<Result<(), wit_host::ModuleError>, DispatchError> {
        let mk_call_err = |e: wasmtime::Error| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.to_string(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: e.to_string(),
        };
        let mk_ctx_err = |e: wasmtime::Error| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.to_string(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::ContextCreation,
            reason: e.to_string(),
        };

        match stage_id {
            "Layer::Infill" => {
                let region_handles =
                    push_slice_regions(store, arena, layer_z).map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_infill_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_infill(store, layer_index, &region_handles, own(output), own(config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::InfillPostProcess" => {
                let region_handles =
                    push_perimeter_regions(store, arena).map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_infill_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_infill_postprocess(store, layer_index, &region_handles, own(output), own(config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::SlicePostProcess" => {
                let region_handles =
                    push_slice_regions(store, arena, layer_z).map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(paint_ir, layer_index);
                let paint = store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_slice_postprocess_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_slice_postprocess(
                        store,
                        layer_index,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::Perimeters" => {
                let region_handles =
                    push_slice_regions(store, arena, layer_z).map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(paint_ir, layer_index);
                let paint = store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_perimeter_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_perimeters(
                        store,
                        layer_index,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::PerimetersPostProcess" => {
                let region_handles =
                    push_perimeter_regions(store, arena).map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_perimeter_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_wall_postprocess(store, layer_index, &region_handles, own(output), own(config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::Support" => {
                let region_handles =
                    push_slice_regions(store, arena, layer_z).map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(paint_ir, layer_index);
                let paint = store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_support_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_support(
                        store,
                        layer_index,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::SupportPostProcess" => {
                let region_handles =
                    push_slice_regions(store, arena, layer_z).map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_support_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_support_postprocess(store, layer_index, &region_handles, own(output), own(config_handle))
                    .map_err(mk_call_err)
            }
            "Layer::PathOptimization" => {
                let region_handles =
                    push_perimeter_regions(store, arena).map_err(mk_ctx_err)?;
                let output = store
                    .data_mut()
                    .push_gcode_output_builder()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_path_optimization(store, layer_index, &region_handles, own(output), own(config_handle))
                    .map_err(mk_call_err)
            }
            _ => Err(DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.to_string(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::UnknownStage,
                reason: format!("no typed layer export for stage '{stage_id}'"),
            }),
        }
    }

    // ── Typed prepass-world dispatch ──────────────────────────────────

    /// Dispatch a prepass-stage call through the typed prepass-module boundary.
    fn dispatch_prepass_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
    ) -> Result<(), DispatchError> {
        let export_name = export_name_for_stage(stage_id).unwrap_or("unknown");
        let component = module.wasm_component.as_ref().ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::PrepassModule::add_to_linker(&mut linker, |ctx| ctx)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            })?;

        let ctx = HostExecutionContext::new(module.module_id.clone());
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        let (bindings, _) = wit_host::PrepassModule::instantiate(&mut store, component.wasmtime_component(), &linker)
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

        let call_result = match stage_id.as_str() {
            "PrePass::MeshAnalysis" => {
                let output = store.data_mut().push_mesh_analysis_output().map_err(mk_ctx_err)?;
                bindings.call_run_mesh_analysis(&mut store, &[], own(output), own(config_handle)).map_err(mk_call_err)
            }
            "PrePass::LayerPlanning" => {
                let output = store.data_mut().push_layer_plan_output().map_err(mk_ctx_err)?;
                bindings.call_run_layer_planning(&mut store, &[], own(output), own(config_handle)).map_err(mk_call_err)
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
        })
    }

    // ── Typed finalization-world dispatch ──────────────────────────────

    /// Dispatch a finalization-stage call through the typed finalization-module boundary.
    fn dispatch_finalization_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
    ) -> Result<(), DispatchError> {
        let export_name = export_name_for_stage(stage_id).unwrap_or("unknown");
        let component = module.wasm_component.as_ref().ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::FinalizationModule::add_to_linker(&mut linker, |ctx| ctx)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            })?;

        let ctx = HostExecutionContext::new(module.module_id.clone());
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

        let (bindings, _) = wit_host::FinalizationModule::instantiate(&mut store, component.wasmtime_component(), &linker)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedInstantiation,
                reason: e.to_string(),
            })?;

        let call_result = bindings.call_run_finalization(&mut store, &[], own(output_handle), own(config_handle))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
                reason: e.to_string(),
            })?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
            reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
        })
    }

    // ── Typed postpass-world dispatch ──────────────────────────────────

    /// Dispatch a postpass gcode-postprocess call through the typed postpass-module boundary.
    fn dispatch_postpass_gcode_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
    ) -> Result<(), DispatchError> {
        let export_name = "run-gcode-postprocess";
        let component = module.wasm_component.as_ref().ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::PostpassModule::add_to_linker(&mut linker, |ctx| ctx)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            })?;

        let ctx = HostExecutionContext::new(module.module_id.clone());
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;
        let output_handle = store.data_mut().push_postpass_gcode_output_builder()
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push gcode output resource: {e}"),
            })?;

        let (bindings, _) = wit_host::PostpassModule::instantiate(&mut store, component.wasmtime_component(), &linker)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedInstantiation,
                reason: e.to_string(),
            })?;

        let call_result = bindings.call_run_gcode_postprocess(&mut store, &[], own(output_handle), own(config_handle))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
                reason: e.to_string(),
            })?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
            reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
        })
    }

    /// Dispatch a postpass text-postprocess call through the typed postpass-module boundary.
    fn dispatch_postpass_text_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        text: &str,
    ) -> Result<String, DispatchError> {
        let export_name = "run-text-postprocess";
        let component = module.wasm_component.as_ref().ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::PostpassModule::add_to_linker(&mut linker, |ctx| ctx)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::LinkerSetup,
                reason: e.to_string(),
            })?;

        let ctx = HostExecutionContext::new(module.module_id.clone());
        let mut store = wasmtime::Store::new(engine, ctx);

        let config_handle = store.data_mut().push_config_view(wit_host::config_view_to_data(&module.config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        let (bindings, _) = wit_host::PostpassModule::instantiate(&mut store, component.wasmtime_component(), &linker)
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedInstantiation,
                reason: e.to_string(),
            })?;

        let call_result = bindings.call_run_text_postprocess(&mut store, text, own(config_handle))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(), stage_id: stage_id.clone(),
                export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
                reason: e.to_string(),
            })?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module.module_id.clone(), stage_id: stage_id.clone(),
            export_name: export_name.to_string(), phase: DispatchPhase::TypedExportCall,
            reason: format!("module error (code={}, fatal={}): {}", module_err.code, module_err.fatal, module_err.message),
        })
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

// ── Stage runner trait implementations ──────────────────────────────────

impl PrepassStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<PrepassStageOutput, PrepassExecutionError> {
        self.dispatch_prepass_call(stage_id, module)
            .map_err(|e| PrepassExecutionError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            })?;

        Ok(PrepassStageOutput::None)
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
    ) -> Result<LayerStageOutput, LayerStageError> {
        // Extract paint region IR from blackboard for paint-consuming stages.
        let paint_ir = blackboard.paint_regions();
        let paint_ref = paint_ir.map(|arc| arc.as_ref());

        // Layer stages always use the typed component-model boundary.
        let ctx = self
            .dispatch_layer_call(stage_id, module, layer.index, layer.z, paint_ref, arena)
            .map_err(|e| LayerStageError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            })?;

        // Commit collected outputs into the layer arena based on stage.
        commit_layer_outputs(stage_id, &module.module_id, layer.index, &ctx, arena)?;

        Ok(LayerStageOutput::Success)
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
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        self.dispatch_finalization_call(stage_id, module)
            .map_err(|e| FinalizationError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            })?;

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
        self.dispatch_postpass_gcode_call(stage_id, module)
            .map_err(|e| PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            })?;

        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        match self.dispatch_postpass_text_call(stage_id, module, &text) {
            Ok(result_text) => Ok(PostpassOutput::TextSuccess { text: result_text }),
            Err(e) => Err(PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            }),
        }
    }
}

// Safety: WasmRuntimeDispatcher is Sync because WasmEngine (wrapping wasmtime::Engine)
// is Send+Sync, and all mutable state is created per-call (not shared).
unsafe impl Sync for WasmRuntimeDispatcher {}
