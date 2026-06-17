//! WASM dispatch layer for the slicer-wasm-host crate.
//!
//! Contains all WIT/component-model dispatch machinery moved from
//! `slicer-runtime/src/dispatch.rs` in packet 83 Step 4a-iii.
//!
//! The four stage-runner trait implementations (`PrepassStageRunner`,
//! `LayerStageRunner`, `FinalizationStageRunner`, `PostpassStageRunner`)
//! are fully implemented in P83 Step 4b. No `&Blackboard`, `&LayerArena`,
//! or `&CompiledModule` references anywhere in this file.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

use wasmtime::component::Resource;

use slicer_ir::{GCodeCommand, GlobalLayer, LayerCollectionIR, RetractMode, StageId};
use slicer_sdk::traits::{EntityMutation, SortKey};

use crate::binding::{
    CompiledModuleLive, FinalizationStageInput, LayerStageInput, PostpassStageInput,
    PrepassStageInput,
};
use crate::host::{
    self, ConfigViewData, HostExecutionContext, HostExecutionContextBuilder, PaintRegionLayerData,
};
use crate::instance::WasmEngine;
use crate::traits::{
    FinalizationStageRunner, LayerStageRunner, PostpassStageRunner, PrepassStageRunner,
};

thread_local! {
    /// Per-worker-thread slot holding the wasm linear-memory sample
    /// `(initial_bytes, peak_bytes)` from the most recent
    /// [`WasmRuntimeDispatcher::dispatch_layer_call`] on this thread.
    /// Read and cleared by `LayerStageRunner::last_wasm_mem_sample`.
    /// Rayon workers are stable threads, so a thread-local is safe for the
    /// per-layer parallel executor's `run_stage → on_module_end` sequence.
    static LAST_WASM_MEM_SAMPLE: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
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

/// Convert host-side `slicer_ir::RetractMode` to the WIT enum used by the
/// postpass-module bindings (host→guest direction).
fn retract_mode_to_postpass_wit(mode: RetractMode) -> host::postpass::RetractMode {
    use host::postpass::RetractMode as PostpassRetractMode;
    match mode {
        RetractMode::Gcode => PostpassRetractMode::Gcode,
        RetractMode::Firmware => PostpassRetractMode::Firmware,
    }
}

fn convert_gcode_command_to_postpass_wit(command: &GCodeCommand) -> host::postpass::GcodeCommand {
    match command {
        GCodeCommand::Move {
            x,
            y,
            z,
            e,
            f,
            role,
        } => host::postpass::GcodeCommand::Move(host::postpass::GcodeMoveCmd {
            x: *x,
            y: *y,
            z: *z,
            e: *e,
            f: *f,
            role: host::ir_to_wit_extrusion_role(role),
        }),
        GCodeCommand::Retract {
            length,
            speed,
            mode,
        } => host::postpass::GcodeCommand::Retract(host::postpass::GcodeRetractCmd {
            length: *length,
            speed: *speed,
            mode: retract_mode_to_postpass_wit(*mode),
        }),
        GCodeCommand::Unretract {
            length,
            speed,
            mode,
        } => host::postpass::GcodeCommand::Unretract(host::postpass::GcodeRetractCmd {
            length: *length,
            speed: *speed,
            mode: retract_mode_to_postpass_wit(*mode),
        }),
        GCodeCommand::FanSpeed { value } => {
            host::postpass::GcodeCommand::FanSpeed(host::postpass::GcodeFanSpeedCmd {
                value: *value,
            })
        }
        GCodeCommand::Temperature {
            tool,
            celsius,
            wait,
        } => host::postpass::GcodeCommand::Temperature(host::postpass::GcodeTemperatureCmd {
            tool: *tool,
            celsius: *celsius,
            wait: *wait,
        }),
        GCodeCommand::ToolChange {
            after_entity_index,
            from,
            to,
        } => host::postpass::GcodeCommand::ToolChange(host::postpass::GcodeToolChangeCmd {
            after_entity_index: *after_entity_index,
            from_tool: *from,
            to_tool: *to,
        }),
        GCodeCommand::Comment { text } => host::postpass::GcodeCommand::Comment(text.clone()),
        GCodeCommand::Raw { text } => host::postpass::GcodeCommand::Raw(text.clone()),
        // ExtrusionMode is not yet a WIT variant; pass through as Raw so postpass
        // modules see the correct M82/M83 line.
        GCodeCommand::ExtrusionMode { absolute } => {
            host::postpass::GcodeCommand::Raw(if *absolute {
                "M82".to_string()
            } else {
                "M83".to_string()
            })
        }
    }
}

// collect_postpass_output moved to crate::marshal::out (packet 113, ADR-0021).
// Used below via crate::marshal::collect_postpass_output.

/// Bundled static configuration for a layer dispatch call.
struct CallConfig<'a> {
    bindings: &'a host::LayerModule,
    store: &'a mut wasmtime::Store<HostExecutionContext>,
    stage_id: &'a str,
    module_id: &'a str,
    export_name: &'a str,
    config_handle: Resource<ConfigViewData>,
}

/// Bundled layer-specific parameters for a `call_layer_export` invocation.
///
/// Holds IR-typed refs — no `&LayerArena`. `slice_ir`, `perimeter_ir`, and
/// `layer_collection` are passed separately to `call_layer_export` rather than
/// stored here because they are only needed by specific stage branches.
struct LayerParams<'a> {
    layer_index: u32,
    layer_z: f32,
    /// Reserved: paint annotations now live in SliceIR segment_annotations (AC-16).
    paint_ir: Option<&'a ()>,
    seam_plan_ir: Option<&'a slicer_ir::SeamPlanIR>,
    support_plan_ir: Option<&'a slicer_ir::SupportPlanIR>,
    _arena_placeholder: std::marker::PhantomData<&'a ()>,
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
    /// Called by `LayerStageRunner::run_stage` with fields pre-projected from
    /// `LayerStageInput<'_>` and `CompiledModuleLive<'_>` (P83 Step 4b).
    #[allow(clippy::too_many_arguments)]
    fn dispatch_layer_call(
        &self,
        stage_id: &StageId,
        module_id: &str,
        _module_id_str: &str,
        wasm_component: Option<&Arc<crate::instance::WasmComponent>>,
        instance_pool: &Arc<crate::pool::WasmInstancePool>,
        _config_view: &slicer_ir::ConfigView,
        envelope_floor: f32,
        envelope_height: f32,
        mesh_ir: Arc<slicer_ir::MeshIR>,
        held_claims_map: std::collections::HashMap<(String, String), Vec<String>>,
        effective_config_view: slicer_ir::ConfigView,
        layer_index: u32,
        layer_z: f32,
        _paint_ir: Option<&()>,
        seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
        support_plan_ir: Option<&slicer_ir::SupportPlanIR>,
        slice_ir: Option<&slicer_ir::SliceIR>,
        perimeter_ir: Option<&slicer_ir::PerimeterIR>,
        layer_collection: Option<&slicer_ir::LayerCollectionIR>,
    ) -> Result<HostExecutionContext, DispatchError> {
        use slicer_schema::export_for_stage_id;
        let export_name = export_for_stage_id(stage_id).ok_or_else(|| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: String::new(),
            phase: DispatchPhase::UnknownStage,
            reason: format!("no export mapping for stage '{stage_id}'"),
        })?;

        let component = wasm_component.ok_or_else(|| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        // Acquire pool slot for concurrency control (RAII — released on drop).
        let _lease = instance_pool.acquire();

        let engine = self.engine.wasmtime_engine();

        // Wire typed host imports into a fresh linker.
        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        host::LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        )
        .map_err(|e| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::LinkerSetup,
            reason: e.to_string(),
        })?;

        // Create per-call execution context and store.
        let ctx = HostExecutionContextBuilder::new(
            module_id.to_string(),
            envelope_floor,
            envelope_height,
        )
        .mesh_ir(Some(mesh_ir))
        .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        store.data_mut().set_held_claims_per_region(held_claims_map);

        let config_handle = store
            .data_mut()
            .push_config_view(host::config_view_to_data(&effective_config_view))
            .map_err(|e| DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        // Instantiate component through typed bindings.
        let bindings =
            host::LayerModule::instantiate(&mut store, component.wasmtime_component(), &linker)
                .map_err(|e| DispatchError {
                    module_id: module_id.to_string(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedInstantiation,
                    reason: e.to_string(),
                })?;

        // Snapshot the post-instantiation memory size.
        let mem_initial_bytes = store.data().mem_tracker.current_bytes;

        // Call the stage-appropriate typed export.
        let call_result = self.call_layer_export(
            CallConfig {
                bindings: &bindings,
                store: &mut store,
                stage_id,
                module_id,
                export_name,
                config_handle,
            },
            LayerParams {
                layer_index,
                layer_z,
                paint_ir: None,
                seam_plan_ir,
                support_plan_ir,
                _arena_placeholder: std::marker::PhantomData,
            },
            slice_ir,
            perimeter_ir,
            layer_collection,
        )?;

        // Handle module-returned error (inner Result).
        call_result.map_err(|module_err| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: format!(
                "module error (code={}, fatal={}): {}",
                module_err.code, module_err.fatal, module_err.message
            ),
        })?;

        let mem_peak_bytes = store.data().mem_tracker.peak_bytes;
        LAST_WASM_MEM_SAMPLE.with(|c| c.set((mem_initial_bytes, mem_peak_bytes)));

        Ok(store.into_data())
    }

    /// Route to the correct typed export based on stage ID.
    fn call_layer_export(
        &self,
        config: CallConfig<'_>,
        params: LayerParams<'_>,
        slice_ir: Option<&slicer_ir::SliceIR>,
        perimeter_ir: Option<&slicer_ir::PerimeterIR>,
        layer_collection: Option<&slicer_ir::LayerCollectionIR>,
    ) -> Result<Result<(), host::ModuleError>, DispatchError> {
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
                let region_handles = push_slice_regions(config.store, slice_ir, params.layer_z)
                    .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_infill_output_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_infill(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::InfillPostProcess" => {
                let region_handles =
                    push_perimeter_regions(config.store, perimeter_ir, None, params.layer_index)
                        .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_infill_output_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_infill_postprocess(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::SlicePostProcess" => {
                let region_handles = push_slice_regions(config.store, slice_ir, params.layer_z)
                    .map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(params.paint_ir, params.layer_index);
                let paint = config
                    .store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_slice_postprocess_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_slice_postprocess(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::Perimeters" => {
                let region_handles = push_slice_regions(config.store, slice_ir, params.layer_z)
                    .map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data(params.paint_ir, params.layer_index);
                let paint = config
                    .store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_perimeter_output_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_perimeters(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::PerimetersPostProcess" => {
                let region_handles = push_perimeter_regions(
                    config.store,
                    perimeter_ir,
                    params.seam_plan_ir,
                    params.layer_index,
                )
                .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_perimeter_output_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_wall_postprocess(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::Support" => {
                let region_handles = push_slice_regions(config.store, slice_ir, params.layer_z)
                    .map_err(mk_ctx_err)?;
                let paint_data = build_paint_layer_data_with_plan(
                    params.paint_ir,
                    params.layer_index,
                    params.support_plan_ir,
                );
                let paint = config
                    .store
                    .data_mut()
                    .push_paint_region_layer_view(paint_data)
                    .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_support_output_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_support(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(paint),
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::SupportPostProcess" => {
                let region_handles = push_slice_regions(config.store, slice_ir, params.layer_z)
                    .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_support_output_builder()
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_support_postprocess(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(output),
                        own(config.config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "Layer::PathOptimization" => {
                let region_handles =
                    push_perimeter_regions(config.store, perimeter_ir, None, params.layer_index)
                        .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_gcode_output_builder()
                    .map_err(mk_ctx_err)?;
                let snapshot = project_ordered_entities_from(layer_collection);
                let collection = config
                    .store
                    .data_mut()
                    .push_layer_collection_builder(snapshot)
                    .map_err(mk_ctx_err)?;
                config
                    .bindings
                    .call_run_path_optimization(
                        config.store,
                        params.layer_index as i32,
                        &region_handles,
                        own(output),
                        own(collection),
                        own(config.config_handle),
                    )
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
    /// Called by `PrepassStageRunner::run_stage` with fields pre-projected from
    /// `PrepassStageInput<'_>` and `CompiledModuleLive<'_>` (P83 Step 4b).
    fn dispatch_prepass_call(
        &self,
        stage_id: &StageId,
        module_id: &str,
        wasm_component: Option<&Arc<crate::instance::WasmComponent>>,
        instance_pool: &Arc<crate::pool::WasmInstancePool>,
        config_view: &slicer_ir::ConfigView,
        mesh_ir: Arc<slicer_ir::MeshIR>,
        layer_plan: Option<Arc<slicer_ir::LayerPlanIR>>,
        region_map: Option<Arc<slicer_ir::RegionMapIR>>,
        support_geometry: Option<Arc<slicer_ir::SupportGeometryIR>>,
    ) -> Result<host::HostExecutionContext, DispatchError> {
        use slicer_schema::export_for_stage_id;
        let export_name = export_for_stage_id(stage_id).unwrap_or("unknown");
        let component = wasm_component.ok_or_else(|| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<host::HostExecutionContext>::new(engine);
        host::PrepassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        )
        .map_err(|e| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::LinkerSetup,
            reason: e.to_string(),
        })?;

        let ctx = host::HostExecutionContextBuilder::new(module_id.to_string(), 0.0, 0.0)
            .mesh_ir(Some(mesh_ir.clone()))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        let config_handle = store
            .data_mut()
            .push_config_view(host::config_view_to_data(config_view))
            .map_err(|e| DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        let bindings =
            host::PrepassModule::instantiate(&mut store, component.wasmtime_component(), &linker)
                .map_err(|e| DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::TypedInstantiation,
                reason: e.to_string(),
            })?;

        let mk_call_err = |e: wasmtime::Error| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: e.to_string(),
        };
        let mk_ctx_err = |e: wasmtime::Error| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::ContextCreation,
            reason: e.to_string(),
        };

        let call_result = match stage_id.as_str() {
            "PrePass::MeshAnalysis" => {
                let object_ids: Vec<String> =
                    mesh_ir.objects.iter().map(|o| o.id.clone()).collect();
                let output = store
                    .data_mut()
                    .push_mesh_analysis_output()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_mesh_analysis(
                        &mut store,
                        &object_ids,
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "PrePass::LayerPlanning" => {
                let object_ids: Vec<String> =
                    mesh_ir.objects.iter().map(|o| o.id.clone()).collect();
                let output = store
                    .data_mut()
                    .push_layer_plan_output()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_layer_planning(
                        &mut store,
                        &object_ids,
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "PrePass::SeamPlanning" => {
                let mesh_object_views: Vec<_> = mesh_ir
                    .objects
                    .iter()
                    .map(host::object_mesh_to_wit_mesh_object_view)
                    .collect();
                let output = store
                    .data_mut()
                    .push_seam_planning_output()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_seam_planning(
                        &mut store,
                        &mesh_object_views,
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "PrePass::SupportGeometry" => {
                let mesh_object_views: Vec<_> = mesh_ir
                    .objects
                    .iter()
                    .map(host::object_mesh_to_wit_mesh_object_view)
                    .collect();
                let layer_plan_view = layer_plan
                    .as_deref()
                    .map(|lp| host::project_layer_plan_view(lp))
                    .unwrap_or_else(|| host::prepass::LayerPlanView { layers: Vec::new() });
                let region_segmentation_view = region_map
                    .as_deref()
                    .map(|rm| host::project_region_segmentation_view(rm))
                    .unwrap_or_else(|| host::prepass::RegionSegmentationView {
                        entries: Vec::new(),
                    });
                let support_geometry_view = support_geometry
                    .as_deref()
                    .map(|sg| host::project_support_geometry_view(sg))
                    .unwrap_or_else(|| host::prepass::SupportGeometryView {
                        entries: Vec::new(),
                    });
                let output = store
                    .data_mut()
                    .push_support_geometry_output()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_support_geometry(
                        &mut store,
                        &mesh_object_views,
                        &layer_plan_view,
                        &region_segmentation_view,
                        &support_geometry_view,
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            _ => Err(DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::UnknownStage,
                reason: format!("no typed prepass export for stage '{stage_id}'"),
            }),
        }?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: format!(
                "module error (code={}, fatal={}): {}",
                module_err.code, module_err.fatal, module_err.message
            ),
        })?;

        Ok(store.into_data())
    }

    // ── Typed finalization-world dispatch ──────────────────────────────

    /// Dispatch a finalization-stage call through the typed finalization-module boundary.
    ///
    /// Called by `FinalizationStageRunner::run_stage` with fields pre-projected from
    /// `FinalizationStageInput<'_>` and `CompiledModuleLive<'_>` (P83 Step 4b).
    fn dispatch_finalization_call(
        &self,
        stage_id: &StageId,
        module_id: &str,
        wasm_component: Option<&Arc<crate::instance::WasmComponent>>,
        instance_pool: &Arc<crate::pool::WasmInstancePool>,
        config_view: &slicer_ir::ConfigView,
        mesh_ir: Arc<slicer_ir::MeshIR>,
        layers: &[slicer_ir::LayerCollectionIR],
    ) -> Result<Vec<host::FinalizationBuilderPush>, DispatchError> {
        use slicer_schema::export_for_stage_id;
        let export_name = export_for_stage_id(stage_id).unwrap_or("unknown");
        let component = wasm_component.ok_or_else(|| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::MissingComponent,
            reason: "no compiled WASM component available".to_string(),
        })?;

        let _lease = instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        host::FinalizationModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        )
        .map_err(|e| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::LinkerSetup,
            reason: e.to_string(),
        })?;

        let ctx = HostExecutionContextBuilder::new(module_id.to_string(), 0.0, 0.0)
            .mesh_ir(Some(mesh_ir))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        let config_handle = store
            .data_mut()
            .push_config_view(host::config_view_to_data(config_view))
            .map_err(|e| DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        let output_handle = store
            .data_mut()
            .push_finalization_output_builder()
            .map_err(|e| DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push finalization output resource: {e}"),
            })?;

        let mut layer_handles = Vec::with_capacity(layers.len());
        for layer in layers {
            let h = store
                .data_mut()
                .push_finalization_layer_view(layer)
                .map_err(|e| DispatchError {
                    module_id: module_id.to_string(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::ContextCreation,
                    reason: format!("failed to push layer-collection-view resource: {e}"),
                })?;
            layer_handles.push(own(h));
        }

        let bindings = host::FinalizationModule::instantiate(
            &mut store,
            component.wasmtime_component(),
            &linker,
        )
        .map_err(|e| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedInstantiation,
            reason: e.to_string(),
        })?;

        let call_result = bindings
            .call_run_finalization(
                &mut store,
                &layer_handles,
                own(output_handle),
                own(config_handle),
            )
            .map_err(|e| DispatchError {
                module_id: module_id.to_string(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::TypedExportCall,
                reason: e.to_string(),
            })?;

        call_result.map_err(|module_err| DispatchError {
            module_id: module_id.to_string(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: format!(
                "module error (code={}, fatal={}): {}",
                module_err.code, module_err.fatal, module_err.message
            ),
        })?;

        Ok(store.data_mut().drain_finalization_output_builder())
    }

    // ── Typed postpass-world dispatch ──────────────────────────────────

    /// Dispatch a postpass gcode-postprocess call through the typed postpass-module boundary.
    ///
    /// Called by `PostpassStageRunner::run_gcode_postprocess` with fields pre-projected from
    /// `PostpassStageInput<'_>` and `CompiledModuleLive<'_>` (P83 Step 4b).
    fn dispatch_postpass_gcode_call(
        &self,
        stage_id: &StageId,
        module_id: &str,
        wasm_component: Option<&Arc<crate::instance::WasmComponent>>,
        instance_pool: &Arc<crate::pool::WasmInstancePool>,
        config_view: &slicer_ir::ConfigView,
        mesh_ir: Arc<slicer_ir::MeshIR>,
        commands: &[GCodeCommand],
    ) -> (
        Result<Option<Vec<GCodeCommand>>, DispatchError>,
        Vec<String>,
    ) {
        let export_name = "run-gcode-postprocess";
        let component = match wasm_component {
            Some(c) => c,
            None => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::MissingComponent,
                        reason: "no compiled WASM component available".to_string(),
                    }),
                    Vec::new(),
                )
            }
        };

        let _lease = instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        if let Err(e) = host::PostpassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        ) {
            return (
                Err(DispatchError {
                    module_id: module_id.to_string(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::LinkerSetup,
                    reason: e.to_string(),
                }),
                Vec::new(),
            );
        }

        let ctx = HostExecutionContextBuilder::new(module_id.to_string(), 0.0, 0.0)
            .mesh_ir(Some(mesh_ir))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        let config_handle = match store
            .data_mut()
            .push_config_view(host::config_view_to_data(config_view))
        {
            Ok(h) => h,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::ContextCreation,
                        reason: format!("failed to push config resource: {e}"),
                    }),
                    Vec::new(),
                )
            }
        };
        let output_handle = match store.data_mut().push_postpass_gcode_output_builder() {
            Ok(h) => h,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::ContextCreation,
                        reason: format!("failed to push gcode output resource: {e}"),
                    }),
                    Vec::new(),
                )
            }
        };

        let bindings = match host::PostpassModule::instantiate(
            &mut store,
            component.wasmtime_component(),
            &linker,
        ) {
            Ok(b) => b,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::TypedInstantiation,
                        reason: e.to_string(),
                    }),
                    Vec::new(),
                )
            }
        };

        let postpass_commands: Vec<_> = commands
            .iter()
            .map(convert_gcode_command_to_postpass_wit)
            .collect();

        let call_result = bindings.call_run_gcode_postprocess(
            &mut store,
            &postpass_commands,
            own(output_handle),
            own(config_handle),
        );
        let runtime_reads = store.data().runtime_reads.clone();

        match call_result {
            Ok(Ok(())) => {
                let output = match crate::marshal::collect_postpass_output(
                    &store.data().gcode_output.commands,
                ) {
                    Ok(output) => output,
                    Err(reason) => {
                        return (
                            Err(DispatchError {
                                module_id: module_id.to_string(),
                                stage_id: stage_id.clone(),
                                export_name: export_name.to_string(),
                                phase: DispatchPhase::OutputCommit,
                                reason,
                            }),
                            runtime_reads,
                        );
                    }
                };
                (Ok(output), runtime_reads)
            }
            Ok(Err(module_err)) => (
                Err(DispatchError {
                    module_id: module_id.to_string(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedExportCall,
                    reason: format!(
                        "module error (code={}, fatal={}): {}",
                        module_err.code, module_err.fatal, module_err.message
                    ),
                }),
                runtime_reads,
            ),
            Err(e) => (
                Err(DispatchError {
                    module_id: module_id.to_string(),
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
    ///
    /// Called by `PostpassStageRunner::run_text_postprocess` with fields pre-projected from
    /// `PostpassStageInput<'_>` and `CompiledModuleLive<'_>` (P83 Step 4b).
    fn dispatch_postpass_text_call(
        &self,
        stage_id: &StageId,
        module_id: &str,
        wasm_component: Option<&Arc<crate::instance::WasmComponent>>,
        instance_pool: &Arc<crate::pool::WasmInstancePool>,
        config_view: &slicer_ir::ConfigView,
        mesh_ir: Arc<slicer_ir::MeshIR>,
        text: &str,
    ) -> (Result<String, DispatchError>, Vec<String>) {
        let export_name = "run-text-postprocess";
        let component = match wasm_component {
            Some(c) => c,
            None => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::MissingComponent,
                        reason: "no compiled WASM component available".to_string(),
                    }),
                    Vec::new(),
                )
            }
        };

        let _lease = instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        if let Err(e) = host::PostpassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        ) {
            return (
                Err(DispatchError {
                    module_id: module_id.to_string(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::LinkerSetup,
                    reason: e.to_string(),
                }),
                Vec::new(),
            );
        }

        let ctx = HostExecutionContextBuilder::new(module_id.to_string(), 0.0, 0.0)
            .mesh_ir(Some(mesh_ir))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        let config_handle = match store
            .data_mut()
            .push_config_view(host::config_view_to_data(config_view))
        {
            Ok(h) => h,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::ContextCreation,
                        reason: format!("failed to push config resource: {e}"),
                    }),
                    Vec::new(),
                )
            }
        };

        let bindings = match host::PostpassModule::instantiate(
            &mut store,
            component.wasmtime_component(),
            &linker,
        ) {
            Ok(b) => b,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module_id.to_string(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::TypedInstantiation,
                        reason: e.to_string(),
                    }),
                    Vec::new(),
                )
            }
        };

        let call_result = bindings.call_run_text_postprocess(&mut store, text, own(config_handle));
        let runtime_reads = store.data().runtime_reads.clone();

        match call_result {
            Ok(Ok(result_text)) => (Ok(result_text), runtime_reads),
            Ok(Err(module_err)) => (
                Err(DispatchError {
                    module_id: module_id.to_string(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedExportCall,
                    reason: format!(
                        "module error (code={}, fatal={}): {}",
                        module_err.code, module_err.fatal, module_err.message
                    ),
                }),
                runtime_reads,
            ),
            Err(e) => (
                Err(DispatchError {
                    module_id: module_id.to_string(),
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

/// Build `PaintRegionLayerData` from an optional paint source.
/// Paint annotations now live in SliceIR segment_annotations (AC-16);
/// this always returns empty-but-valid data.
fn build_paint_layer_data(_paint_ir: Option<&()>, layer_index: u32) -> PaintRegionLayerData {
    build_paint_layer_data_with_plan(_paint_ir, layer_index, None)
}

/// Variant of [`build_paint_layer_data`] that also indexes a committed
/// `SupportPlanIR` for this layer.
fn build_paint_layer_data_with_plan(
    _paint_ir: Option<&()>,
    layer_index: u32,
    support_plan_ir: Option<&slicer_ir::SupportPlanIR>,
) -> PaintRegionLayerData {
    let mut data = PaintRegionLayerData {
        layer_index,
        regions_by_semantic: HashMap::new(),
        custom_regions: HashMap::new(),
        support_plan_segments: HashMap::new(),
    };
    if let Some(plan) = support_plan_ir {
        for entry in &plan.entries {
            if entry.global_layer_index != layer_index as i32 {
                continue;
            }
            let key = (entry.object_id.clone(), entry.region_id.to_string());
            let bucket = data.support_plan_segments.entry(key).or_default();
            for segment in &entry.branch_segments {
                let pts: Vec<_> = segment
                    .points
                    .iter()
                    .map(|p| host::layer::slicer::types::geometry::Point3WithWidth {
                        x: p.x,
                        y: p.y,
                        z: p.z,
                        width: p.width,
                        flow_factor: p.flow_factor,
                        overhang_quartile: p.overhang_quartile,
                    })
                    .collect();
                bucket.push(pts);
            }
        }
    }
    data
}

/// Push `SliceRegionData` resources into the store from the provided `SliceIR`.
///
/// Returns resource handles for each `SlicedRegion`. Returns an empty vec
/// if no `SliceIR` is provided.
fn push_slice_regions(
    store: &mut wasmtime::Store<HostExecutionContext>,
    slice_ir: Option<&slicer_ir::SliceIR>,
    layer_z: f32,
) -> Result<Vec<Resource<host::SliceRegionData>>, wasmtime::Error> {
    let slice_ir = match slice_ir {
        Some(ir) => ir,
        None => return Ok(Vec::new()),
    };

    let mut handles = Vec::with_capacity(slice_ir.regions.len());
    for region in &slice_ir.regions {
        let held_claims = store
            .data()
            .held_claims_for(&region.object_id, &region.region_id.to_string())
            .to_vec();
        let data = host::sliced_region_to_data(region, layer_z, held_claims);
        let handle = store.data_mut().push_slice_region(data)?;
        handles.push(handle);
    }
    Ok(handles)
}

/// Push `PerimeterRegionData` resources into the store from the provided `PerimeterIR`.
///
/// Returns resource handles for each `PerimeterRegion`. Returns an empty vec
/// if no `PerimeterIR` is provided.
fn push_perimeter_regions(
    store: &mut wasmtime::Store<HostExecutionContext>,
    perimeter_ir: Option<&slicer_ir::PerimeterIR>,
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
    layer_index: u32,
) -> Result<Vec<Resource<host::PerimeterRegionData>>, wasmtime::Error> {
    let perimeter_ir = match perimeter_ir {
        Some(ir) => ir,
        None => return Ok(Vec::new()),
    };

    let mut handles = Vec::with_capacity(perimeter_ir.regions.len());
    for region in &perimeter_ir.regions {
        let mut data = host::perimeter_region_to_data(region);
        if let Some(seam_ir) = seam_plan_ir {
            if let Some(entry) = seam_ir.entries.iter().find(|e| {
                e.region_key.global_layer_index == layer_index
                    && e.region_key.object_id == region.object_id
                    && e.region_key.region_id == region.region_id
            }) {
                data.resolved_seam = Some((
                    host::Point3 {
                        x: entry.chosen_candidate.point.x,
                        y: entry.chosen_candidate.point.y,
                        z: entry.chosen_candidate.point.z,
                    },
                    entry.chosen_candidate.wall_index,
                ));
            }
        }
        let handle = store.data_mut().push_perimeter_region(data)?;
        handles.push(handle);
    }
    Ok(handles)
}

// ── Layer-plan harvest ────────────────────────────────────────────────────

/// Convert WIT `LayerProposal` records collected by a `PrePass::LayerPlanning`
/// call into a host-side [`slicer_ir::LayerPlanIR`].
fn harvest_layer_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: host::HostExecutionContext,
) -> Result<slicer_ir::LayerPlanIR, String> {
    harvest_layer_plan_ir_from(ctx.layer_plan_proposals)
}

// Pure core of harvest_layer_plan_ir moved to marshal/in_.rs (packet 113, Step 7 / ADR-0021).
use crate::marshal::in_::harvest_layer_plan_ir_from;

// ── Seam-plan harvest ──────────────────────────────────────────────────────

/// Convert WIT `SeamPlanEntry` records into a host-side [`slicer_ir::SeamPlanIR`].
fn harvest_seam_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: host::HostExecutionContext,
) -> Result<slicer_ir::SeamPlanIR, String> {
    harvest_seam_plan_ir_from(ctx.seam_plan_entries)
}

// Pure core of harvest_seam_plan_ir moved to marshal/in_.rs (packet 113, Step 7 / ADR-0021).
use crate::marshal::in_::harvest_seam_plan_ir_from;

// ── Support-plan harvest ───────────────────────────────────────────────────

/// Convert WIT `SupportPlanEntry` records into a host-side [`slicer_ir::SupportPlanIR`].
fn harvest_support_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: host::HostExecutionContext,
) -> Result<slicer_ir::SupportPlanIR, String> {
    harvest_support_plan_ir_from(ctx.support_plan_entries)
}

// Pure core of harvest_support_plan_ir moved to marshal/in_.rs (packet 113, Step 7 / ADR-0021).
use crate::marshal::in_::harvest_support_plan_ir_from;

/// Convert the `(object_id, FacetAnnotation)` / `(object_id, SurfaceGroupProposal)`
/// pushes into a `MeshAnalysisAuxiliary` record.
fn harvest_mesh_analysis_auxiliary(
    ctx: host::HostExecutionContext,
) -> slicer_core::MeshAnalysisAuxiliary {
    harvest_mesh_analysis_auxiliary_from(
        ctx.mesh_analysis_annotations,
        ctx.mesh_analysis_surface_groups,
    )
}

// Pure core of harvest_mesh_analysis_auxiliary moved to marshal/in_.rs (packet 113, Step 7 / ADR-0021).
use crate::marshal::in_::harvest_mesh_analysis_auxiliary_from;

// ── Host-local OrderedEntityView projection (used by Layer::PathOptimization) ──

/// Host-local projection of a single staged
/// `LayerCollectionIR.ordered_entities[i]` entry.
#[derive(Debug, Clone)]
pub struct OrderedEntityView {
    /// Index into the host-staged `LayerCollectionIR.ordered_entities`
    /// at the time this snapshot was projected.
    pub original_index: u32,
    /// Region key of the entity at `original_index`.
    pub region_key: slicer_ir::RegionKey,
    /// Extrusion role of the entity's path.
    pub role: slicer_ir::ExtrusionRole,
    /// First point of `path.points`.
    pub start_point: slicer_ir::Point3WithWidth,
    /// Last point of `path.points`.
    pub end_point: slicer_ir::Point3WithWidth,
    /// Number of points in `path.points`.
    pub point_count: u32,
}

/// Project the host-staged `LayerCollectionIR.ordered_entities` into
/// a snapshot list of [`OrderedEntityView`] for one
/// `Layer::PathOptimization` invocation.
///
/// This version takes an `Option<&LayerCollectionIR>` directly (IR-typed,
/// no LayerArena reference needed).
pub fn project_ordered_entities_from(
    layer_collection: Option<&slicer_ir::LayerCollectionIR>,
) -> Vec<OrderedEntityView> {
    let Some(lc) = layer_collection else {
        return Vec::new();
    };
    lc.ordered_entities
        .iter()
        .enumerate()
        .map(|(i, entity)| {
            let start_point = *entity
                .path
                .points
                .first()
                .expect("PrintEntity invariant: path.points non-empty");
            let end_point = *entity
                .path
                .points
                .last()
                .expect("PrintEntity invariant: path.points non-empty");
            OrderedEntityView {
                original_index: i as u32,
                region_key: entity.region_key.clone(),
                role: entity.path.role.clone(),
                start_point,
                end_point,
                point_count: entity.path.points.len() as u32,
            }
        })
        .collect()
}

// ── Stage runner trait implementations ───────────────────────────────────────
//
// These implement the IR-typed trait signatures from `crate::traits`, using
// the wasm-host-side dispatch helpers and deconstruction helpers below.
// Filled in P83 Step 4b: no `&Blackboard`, `&LayerArena`, or `&CompiledModule`
// references — only `*StageInput<'_>` + `&CompiledModuleLive<'_>`.

impl PrepassStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: PrepassStageInput<'_>,
    ) -> Result<slicer_core::PrepassStageOutput, slicer_ir::PrepassRunnerError> {
        let module_id_str = module.module_id.as_str();

        let ctx = match self.dispatch_prepass_call(
            stage_id,
            module_id_str,
            module.wasm_component.as_ref(),
            &module.instance_pool,
            &module.config_view,
            input.mesh.clone(),
            input.layer_plan.clone(),
            input.region_map.clone(),
            input.support_geometry.clone(),
        ) {
            Ok(ctx) => ctx,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                return Ok(slicer_core::PrepassStageOutput::None);
            }
            Err(e) => {
                return Err(slicer_ir::PrepassRunnerError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        // Deconstruct HostExecutionContext → PrepassStageOutput based on stage.
        if stage_id == "PrePass::LayerPlanning" {
            let ir = harvest_layer_plan_ir(stage_id, module_id_str, ctx).map_err(|msg| {
                slicer_ir::PrepassRunnerError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: msg,
                }
            })?;
            return Ok(slicer_core::PrepassStageOutput::LayerPlan(
                std::sync::Arc::new(ir),
            ));
        }

        if stage_id == "PrePass::SeamPlanning" {
            let ir = harvest_seam_plan_ir(stage_id, module_id_str, ctx).map_err(|msg| {
                slicer_ir::PrepassRunnerError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: msg,
                }
            })?;
            return Ok(slicer_core::PrepassStageOutput::SeamPlan(
                std::sync::Arc::new(ir),
            ));
        }

        if stage_id == "PrePass::SupportGeometry" {
            let ir = harvest_support_plan_ir(stage_id, module_id_str, ctx).map_err(|msg| {
                slicer_ir::PrepassRunnerError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: msg,
                }
            })?;
            return Ok(slicer_core::PrepassStageOutput::SupportPlan(
                std::sync::Arc::new(ir),
            ));
        }

        if stage_id == "PrePass::MeshAnalysis" {
            let aux = harvest_mesh_analysis_auxiliary(ctx);
            if aux.facet_annotations.is_empty() && aux.surface_groups.is_empty() {
                return Ok(slicer_core::PrepassStageOutput::None);
            }
            return Ok(slicer_core::PrepassStageOutput::MeshAnalysisAuxiliary(
                std::sync::Arc::new(aux),
            ));
        }

        Ok(slicer_core::PrepassStageOutput::None)
    }
}

impl LayerStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<slicer_ir::LayerStageCommit>, slicer_ir::LayerStageError> {
        let module_id_str = module.module_id.as_str();
        let (envelope_floor, envelope_height) =
            derive_layer_output_envelope_from_input(layer, input.slice);

        // Build the effective config from the region-map overlay (mirrors the original
        // dispatch.rs `blackboard.region_map()` overlay logic).
        let effective_config_view: slicer_ir::ConfigView = input
            .region_map
            .as_deref()
            .and_then(|map| {
                map.entries
                    .keys()
                    .find(|key| key.global_layer_index == layer.index)
                    .map(|key| {
                        let region_map = resolved_config_to_map(map.config_for(key));
                        let declared_keys = module.config_view.keys();
                        slicer_ir::ConfigView::from_declared(
                            &region_map,
                            declared_keys.iter().map(String::as_str),
                        )
                    })
            })
            .unwrap_or_else(|| module.config_view.as_ref().clone());

        // Build the held-claims map from the slice IR + region-map config.
        // Inlines the `resolve_held_claims` logic from slicer-runtime::validation
        // so that slicer-wasm-host has no back-edge dependency on slicer-runtime.
        const FILL_CLAIM_IDS: &[&str] = &[
            "claim:top-fill",
            "claim:bottom-fill",
            "claim:bridge-fill",
            "claim:sparse-fill",
        ];
        let held_claims_map: HashMap<(String, String), Vec<String>> =
            if let Some(slice_ir) = input.slice {
                slice_ir
                    .regions
                    .iter()
                    .map(|region| {
                        let region_key = slicer_ir::RegionKey {
                            global_layer_index: layer.index,
                            object_id: region.object_id.clone(),
                            region_id: region.region_id,
                            variant_chain: Vec::new(),
                        };
                        let config = input
                            .region_map
                            .as_deref()
                            .and_then(|map| {
                                if map.entries.contains_key(&region_key) {
                                    Some(map.config_for(&region_key).clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                        let top = config.top_fill_holder.as_str();
                        let bottom = config.bottom_fill_holder.as_str();
                        let bridge = config.bridge_fill_holder.as_str();
                        let sparse = config.sparse_fill_holder.as_str();
                        let held: Vec<String> = module
                            .claims
                            .iter()
                            .filter(|claim| FILL_CLAIM_IDS.contains(&claim.as_str()))
                            .filter(|claim| {
                                let holder = match claim.as_str() {
                                    "claim:top-fill" => top,
                                    "claim:bottom-fill" => bottom,
                                    "claim:bridge-fill" => bridge,
                                    "claim:sparse-fill" => sparse,
                                    _ => "",
                                };
                                // Mirrors slicer_scheduler::validation::module_id_matches_holder
                                // (inlined to avoid the back-edge dep). Accepts either full
                                // ID (com.core.rectilinear-infill) or short name (rectilinear-infill)
                                // for built-in modules. See docs/03_wit_and_manifest.md §"Holder
                                // identifier matching".
                                holder == module_id_str
                                    || module_id_str
                                        .strip_prefix("com.core.")
                                        .is_some_and(|short| short == holder)
                            })
                            .cloned()
                            .collect();
                        (
                            (region.object_id.clone(), region.region_id.to_string()),
                            held,
                        )
                    })
                    .collect()
            } else {
                HashMap::new()
            };

        let ctx = match self.dispatch_layer_call(
            stage_id,
            module_id_str,
            module_id_str,
            module.wasm_component.as_ref(),
            &module.instance_pool,
            &module.config_view,
            envelope_floor,
            envelope_height,
            input.mesh.clone(),
            held_claims_map,
            effective_config_view,
            layer.index,
            layer.z,
            None,
            input.seam_plan.as_deref(),
            input.support_plan.as_deref(),
            input.slice,
            input.perimeter,
            input.layer_collection,
        ) {
            Ok(ctx) => ctx,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                return Ok(None);
            }
            Err(e) => {
                return Err(slicer_ir::LayerStageError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        // Deconstruct HostExecutionContext → Option<LayerStageCommit>.
        deconstruct_layer_ctx(stage_id, module_id_str, layer.index, ctx)
    }

    fn last_wasm_mem_sample(&self) -> (u64, u64) {
        LAST_WASM_MEM_SAMPLE.with(|c| c.replace((0, 0)))
    }
}

impl FinalizationStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: FinalizationStageInput<'_>,
        layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<slicer_ir::FinalizationOutput, slicer_ir::FinalizationError> {
        let module_id_str = module.module_id.as_str();

        let pushes = match self.dispatch_finalization_call(
            stage_id,
            module_id_str,
            module.wasm_component.as_ref(),
            &module.instance_pool,
            &module.config_view,
            input.mesh.clone(),
            layers,
        ) {
            Ok(p) => p,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                return Ok(slicer_ir::FinalizationOutput::Success);
            }
            Err(e) => {
                return Err(slicer_ir::FinalizationError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        apply_finalization_pushes(stage_id, module_id_str, pushes, layers)
    }
}

impl PostpassStageRunner for WasmRuntimeDispatcher {
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: PostpassStageInput<'_>,
        commands: &mut Vec<GCodeCommand>,
    ) -> Result<slicer_ir::PostpassOutput, slicer_ir::PostpassError> {
        let module_id_str = module.module_id.as_str();
        let (result, reads) = self.dispatch_postpass_gcode_call(
            stage_id,
            module_id_str,
            module.wasm_component.as_ref(),
            &module.instance_pool,
            &module.config_view,
            input.mesh.clone(),
            commands,
        );
        if !reads.is_empty() {
            self.postpass_runtime_reads.borrow_mut().push(reads);
        }
        match result {
            Ok(Some(new_commands)) => {
                *commands = new_commands;
                Ok(slicer_ir::PostpassOutput::GCodeSuccess)
            }
            Ok(None) => Ok(slicer_ir::PostpassOutput::GCodeSuccess),
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                Ok(slicer_ir::PostpassOutput::GCodeSuccess)
            }
            Err(e) => Err(slicer_ir::PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            }),
        }
    }

    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: PostpassStageInput<'_>,
        text: String,
    ) -> Result<slicer_ir::PostpassOutput, slicer_ir::PostpassError> {
        let module_id_str = module.module_id.as_str();
        let (result, reads) = self.dispatch_postpass_text_call(
            stage_id,
            module_id_str,
            module.wasm_component.as_ref(),
            &module.instance_pool,
            &module.config_view,
            input.mesh.clone(),
            &text,
        );
        if !reads.is_empty() {
            self.postpass_runtime_reads.borrow_mut().push(reads);
        }
        match result {
            Ok(result_text) => Ok(slicer_ir::PostpassOutput::TextSuccess { text: result_text }),
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                Ok(slicer_ir::PostpassOutput::TextSuccess { text })
            }
            Err(e) => Err(slicer_ir::PostpassError::FatalModule {
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

/// Convert a [`ResolvedConfig`] struct into a flat `HashMap<ConfigKey, ConfigValue>`.
fn resolved_config_to_map(
    cfg: &slicer_ir::ResolvedConfig,
) -> std::collections::HashMap<String, slicer_ir::ConfigValue> {
    cfg.to_config_map()
}

// ── Layer-envelope helper (no LayerArena) ─────────────────────────────────────

/// Derive the Z-envelope `(floor, height)` for a layer dispatch call using
/// only the `GlobalLayer` and an optional `SliceIR` reference from
/// `LayerStageInput`. Mirrors the original `derive_layer_output_envelope`
/// from `slicer-runtime::dispatch` but takes `SliceIR` directly instead of
/// reading it from the arena.
fn derive_layer_output_envelope_from_input(
    layer: &GlobalLayer,
    slice_ir: Option<&slicer_ir::SliceIR>,
) -> (f32, f32) {
    let fallback_height = slice_ir
        .and_then(|s| s.regions.first())
        .map(|region| region.effective_layer_height)
        .unwrap_or(0.2);

    if layer.active_regions.is_empty() {
        return (layer.z, fallback_height);
    }

    let mut floor = f32::INFINITY;
    let mut ceiling = f32::NEG_INFINITY;

    for region in &layer.active_regions {
        let region_floor = if region.is_catchup_layer {
            region.catchup_z_bottom
        } else {
            layer.z
        };
        floor = floor.min(region_floor);
        ceiling = ceiling.max(region_floor + region.effective_layer_height);
    }

    if !floor.is_finite() || !ceiling.is_finite() || ceiling <= floor {
        return (layer.z, fallback_height);
    }

    (floor, ceiling - floor)
}

// ── Layer-context deconstruction (HostExecutionContext → LayerStageCommit) ─────

/// Deconstruct a `HostExecutionContext` returned from `dispatch_layer_call` into
/// the per-stage [`slicer_ir::LayerStageCommit`] the runtime's `apply` consumes
/// (ADR-0020). Produces only plain IR values — no arena mutations. Returns
/// `Ok(None)` when the invocation committed nothing (empty guest output).
///
/// The deferred g-code groups carry **no** anchor: the producer has no arena and
/// cannot compute `ordered_entities.len()-1`, so `apply` stamps it. There is no
/// placeholder field to leak (the structural fix for the anchor bug). The
/// `Layer::Perimeters` seam injection is no longer flagged — it is implied by the
/// `Perimeters` variant and performed inside `apply`.
fn deconstruct_layer_ctx(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    ctx: HostExecutionContext,
) -> Result<Option<slicer_ir::LayerStageCommit>, slicer_ir::LayerStageError> {
    use slicer_ir::LayerStageCommit;
    let mk_fatal = |what: &str, reason: String| slicer_ir::LayerStageError::FatalModule {
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
                return Ok(None);
            }
            let ir = crate::marshal::convert_infill_output(infill, layer_index)
                .map_err(|r| mk_fatal("infill", r))?;
            Ok(Some(if stage_id == "Layer::InfillPostProcess" {
                LayerStageCommit::InfillPostProcess(ir)
            } else {
                LayerStageCommit::Infill(ir)
            }))
        }
        "Layer::Support" | "Layer::SupportPostProcess" => {
            let support = &ctx.support_output;
            if support.support_paths.is_empty()
                && support.interface_paths.is_empty()
                && support.raft_paths.is_empty()
            {
                return Ok(None);
            }
            let ir = crate::marshal::convert_support_output(support, layer_index)
                .map_err(|r| mk_fatal("support", r))?;
            Ok(Some(if stage_id == "Layer::SupportPostProcess" {
                LayerStageCommit::SupportPostProcess(ir)
            } else {
                LayerStageCommit::Support(ir)
            }))
        }
        "Layer::Perimeters" => {
            let perimeter = &ctx.perimeter_output;
            let has_any_output = !perimeter.wall_loops.is_empty()
                || !perimeter.infill_areas.is_empty()
                || !perimeter.seam_candidates.is_empty();
            if !has_any_output {
                return Ok(None);
            }
            let ir = crate::marshal::convert_perimeter_output(perimeter, layer_index)
                .map_err(|r| mk_fatal("perimeter", r))?;
            Ok(Some(LayerStageCommit::Perimeters(ir)))
        }
        "Layer::PerimetersPostProcess" => {
            // Post-process always commits for its stage: even an empty output
            // re-partitions the existing perimeter (apply's `(None, Some)` arm).
            let perimeter = &ctx.perimeter_output;
            let has_any_output = !perimeter.wall_loops.is_empty()
                || !perimeter.rotated_wall_loops.is_empty()
                || !perimeter.infill_areas.is_empty()
                || !perimeter.seam_candidates.is_empty();
            let ir = if has_any_output {
                Some(
                    crate::marshal::convert_perimeter_output(perimeter, layer_index)
                        .map_err(|r| mk_fatal("perimeter", r))?,
                )
            } else {
                None
            };
            Ok(Some(LayerStageCommit::PerimetersPostProcess(ir)))
        }
        "Layer::SlicePostProcess" => {
            let sp = &ctx.slice_postprocess_output;
            if sp.polygon_updates.is_empty() && sp.path_z_updates.is_empty() {
                return Ok(None);
            }
            // Flatten WIT RegionKey → slicer_ir::RegionKey for polygon updates.
            let polygon_updates: Vec<(slicer_ir::RegionKey, Vec<slicer_ir::ExPolygon>)> = sp
                .polygon_updates
                .iter()
                .filter_map(|(wit_key, polys)| {
                    let region_id = wit_key.region_id.parse::<u64>().ok()?;
                    let ir_key = slicer_ir::RegionKey {
                        global_layer_index: layer_index,
                        object_id: wit_key.object_id.clone(),
                        region_id,
                        variant_chain: Vec::new(),
                    };
                    let ir_polys: Vec<slicer_ir::ExPolygon> = polys
                        .iter()
                        .map(|ep| slicer_ir::ExPolygon {
                            contour: slicer_ir::Polygon {
                                points: ep
                                    .contour
                                    .points
                                    .iter()
                                    .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                                    .collect(),
                            },
                            holes: ep
                                .holes
                                .iter()
                                .map(|h| slicer_ir::Polygon {
                                    points: h
                                        .points
                                        .iter()
                                        .map(|p| slicer_ir::Point2 { x: p.x, y: p.y })
                                        .collect(),
                                })
                                .collect(),
                        })
                        .collect();
                    Some((ir_key, ir_polys))
                })
                .collect();
            // Flatten WIT RegionKey → slicer_ir::RegionKey for path-Z updates.
            let path_z_updates: Vec<(slicer_ir::RegionKey, u32, u32, f32)> = sp
                .path_z_updates
                .iter()
                .filter_map(|(wit_key, path_idx, vertex_idx, z)| {
                    let region_id = wit_key.region_id.parse::<u64>().ok()?;
                    let ir_key = slicer_ir::RegionKey {
                        global_layer_index: layer_index,
                        object_id: wit_key.object_id.clone(),
                        region_id,
                        variant_chain: Vec::new(),
                    };
                    Some((ir_key, *path_idx, *vertex_idx, *z))
                })
                .collect();
            Ok(Some(LayerStageCommit::SlicePostProcess {
                polygon_updates,
                path_z_updates,
            }))
        }
        "Layer::PathOptimization" => {
            // The deferred groups carry no anchor — `apply` stamps the real
            // end-of-layer value from arena state (ADR-0020). `tool_changes`
            // keep their guest-provided `after_entity_index`.
            let mut commit = slicer_ir::PathOptimizationCommit::default();
            use host::GcodeCommandCollected;
            for (i, cmd) in ctx.gcode_output.commands.iter().enumerate() {
                match cmd {
                    GcodeCommandCollected::ToolChange {
                        after_entity_index,
                        from_tool,
                        to_tool,
                    } => {
                        commit.tool_changes.push(slicer_ir::ToolChange {
                            after_entity_index: *after_entity_index,
                            from_tool: *from_tool,
                            to_tool: *to_tool,
                        });
                    }
                    GcodeCommandCollected::Comment(text) => {
                        commit
                            .annotations
                            .push(slicer_ir::LayerAnnotationKind::Comment(text.clone()));
                    }
                    GcodeCommandCollected::Raw(text) => {
                        commit
                            .annotations
                            .push(slicer_ir::LayerAnnotationKind::Raw(text.clone()));
                    }
                    GcodeCommandCollected::Move(cmd) => {
                        commit.travel_moves.push(slicer_ir::TravelMoveDest {
                            x: cmd.x,
                            y: cmd.y,
                            z: cmd.z,
                            f: cmd.f,
                        });
                    }
                    GcodeCommandCollected::ZHop { hop_height, .. } => {
                        if !hop_height.is_finite() || *hop_height <= 0.0 {
                            return Err(slicer_ir::LayerStageError::FatalModule {
                                stage_id: stage_id.to_string(),
                                module_id: module_id.to_string(),
                                message: format!(
                                    "Layer::PathOptimization push-z-hop call {i} rejected: \
                                     hop-height={hop_height} is not finite and strictly positive"
                                ),
                            });
                        }
                        commit.z_hops.push(*hop_height);
                    }
                    GcodeCommandCollected::Retract {
                        length,
                        speed,
                        mode,
                    } => {
                        commit.retracts.push(slicer_ir::RetractSpec {
                            length: *length,
                            speed: *speed,
                            is_unretract: false,
                            mode: *mode,
                        });
                    }
                    GcodeCommandCollected::Unretract {
                        length,
                        speed,
                        mode,
                    } => {
                        commit.retracts.push(slicer_ir::RetractSpec {
                            length: *length,
                            speed: *speed,
                            is_unretract: true,
                            mode: *mode,
                        });
                    }
                    other => {
                        return Err(slicer_ir::LayerStageError::FatalModule {
                            stage_id: stage_id.to_string(),
                            module_id: module_id.to_string(),
                            message: format!(
                                "Layer::PathOptimization guest emitted unsupported GCode command at index {i} ({:?}); \
                                 accepted overrides: tool-change/comment/raw/z-hop/retract/unretract/move",
                                std::mem::discriminant(other)
                            ),
                        });
                    }
                }
            }
            commit.order_proposal = ctx.layer_collection_proposal().cloned();
            Ok(Some(LayerStageCommit::PathOptimization(commit)))
        }
        _ => Ok(None),
    }
}

// ── Finalization pushes applier ────────────────────────────────────────────────

/// Apply a `FinalizationBuilderPush` stream to the mutable `layers` collection.
///
/// Mirrors the loop in the original `FinalizationStageRunner::run_stage` from
/// `slicer-runtime::dispatch`. Now lives in `slicer-wasm-host` because
/// `FinalizationBuilderPush` is a wasm-host-internal type. The runtime-side
/// `FinalizationStageRunner` impl calls this and applies the result to `layers`.
fn apply_finalization_pushes(
    stage_id: &StageId,
    module_id: &str,
    pushes: Vec<host::FinalizationBuilderPush>,
    layers: &mut Vec<LayerCollectionIR>,
) -> Result<slicer_ir::FinalizationOutput, slicer_ir::FinalizationError> {
    use slicer_ir::LayerEntityIdGen;
    use slicer_sdk::traits::{FinalizationOutputBuilder, SyntheticLayerData};

    let mut sdk_builder = FinalizationOutputBuilder::new();
    let mut legacy_synthetic_layers: Vec<(f32, Vec<slicer_ir::ExtrusionPath3D>)> = Vec::new();

    for push in pushes {
        match push {
            host::FinalizationBuilderPush::EntityToLayer {
                layer_index,
                path,
                region_key,
            } => {
                sdk_builder
                    .push_entity_to_layer(layer_index, path, region_key)
                    .unwrap_or_else(|e| {
                        log::warn!("finalization: push_entity_to_layer rejected: {e}")
                    });
            }
            host::FinalizationBuilderPush::EntityToLayerWithPriority {
                layer_index,
                path,
                region_key,
                priority,
            } => {
                sdk_builder
                    .push_entity_with_priority(layer_index, path, region_key, priority)
                    .unwrap_or_else(|e| {
                        log::warn!("finalization: push_entity_with_priority rejected: {e}")
                    });
            }
            host::FinalizationBuilderPush::ModifyEntity {
                layer_index,
                entity_id,
                mutation,
            } => {
                let sdk_mutation = match mutation {
                    host::WitEntityMutation::SetSpeedFactor(v) => EntityMutation::SetSpeedFactor(v),
                    host::WitEntityMutation::SetFlowFactor(v) => EntityMutation::SetFlowFactor(v),
                };
                sdk_builder
                    .modify_entity(layer_index, entity_id, sdk_mutation)
                    .unwrap_or_else(|e| log::warn!("finalization: modify_entity rejected: {e}"));
            }
            host::FinalizationBuilderPush::SortLayerBy { layer_index, key } => {
                let sdk_key = match key {
                    host::WitSortKey::ByPriorityAndEntityId => SortKey::ByPriorityAndEntityId,
                    host::WitSortKey::ByEntityId => SortKey::ByEntityId,
                    host::WitSortKey::ByObjectIdThenPriority => SortKey::ByObjectIdThenPriority,
                };
                sdk_builder
                    .sort_layer_by(layer_index, sdk_key)
                    .unwrap_or_else(|e| log::warn!("finalization: sort_layer_by rejected: {e}"));
            }
            host::FinalizationBuilderPush::InsertSyntheticLayerAfter { idx, z, paths } => {
                sdk_builder
                    .insert_synthetic_layer_after(idx, SyntheticLayerData { z, paths })
                    .unwrap_or_else(|e| {
                        log::warn!("finalization: insert_synthetic_layer_after rejected: {e}")
                    });
            }
            host::FinalizationBuilderPush::SyntheticLayer { z, paths } => {
                legacy_synthetic_layers.push((z, paths));
            }
            host::FinalizationBuilderPush::InsertEntityAt {
                layer_index,
                position,
                path,
                region_key,
            } => {
                sdk_builder
                    .insert_entity_at(layer_index, position, path, region_key)
                    .unwrap_or_else(|e| log::warn!("finalization: insert_entity_at rejected: {e}"));
            }
            host::FinalizationBuilderPush::SetEntityOrder { layer_index, items } => {
                sdk_builder
                    .set_entity_order(layer_index, items)
                    .unwrap_or_else(|e| log::warn!("finalization: set_entity_order rejected: {e}"));
            }
        }
    }

    sdk_builder
        .apply_to(layers)
        .map_err(|msg| slicer_ir::FinalizationError::FatalModule {
            stage_id: stage_id.clone(),
            module_id: module_id.to_string(),
            message: format!("finalization merge failed: {msg}"),
        })?;

    for (z, paths) in legacy_synthetic_layers {
        let new_index = layers.len() as u32;
        let id_gen = LayerEntityIdGen::new();
        let entities: Vec<_> = paths
            .into_iter()
            .enumerate()
            .map(|(i, path)| {
                let role = path.role.clone();
                slicer_ir::PrintEntity {
                    entity_id: id_gen.next(),
                    path,
                    role,
                    region_key: slicer_ir::RegionKey {
                        global_layer_index: new_index,
                        object_id: String::new(),
                        region_id: 0,
                        variant_chain: Vec::new(),
                    },
                    topo_order: i as u32,
                }
            })
            .collect();
        layers.push(LayerCollectionIR {
            global_layer_index: new_index,
            z,
            ordered_entities: entities,
            ..Default::default()
        });
    }

    Ok(slicer_ir::FinalizationOutput::Success)
}
