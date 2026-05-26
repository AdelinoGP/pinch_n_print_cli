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

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

use wasmtime::component::Resource;

thread_local! {
    /// Per-worker-thread slot holding the wasm linear-memory sample
    /// `(initial_bytes, peak_bytes)` from the most recent
    /// [`WasmRuntimeDispatcher::dispatch_layer_call`] on this thread.
    /// Read and cleared by `LayerStageRunner::last_wasm_mem_sample`.
    /// Rayon workers are stable threads, so a thread-local is safe for the
    /// per-layer parallel executor's `run_stage → on_module_end` sequence.
    static LAST_WASM_MEM_SAMPLE: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}

use slicer_ir::{
    GCodeCommand, GCodeIR, GlobalLayer, LayerCollectionIR, RetractMode, SeamPosition, StageId,
};
use slicer_sdk::traits::{EntityMutation, SortKey, SyntheticLayerData};

use crate::wit_host::{
    self, ConfigViewData, HostExecutionContext, HostExecutionContextBuilder, PaintRegionLayerData,
};
use crate::{
    Blackboard, CompiledModule, FinalizationError, FinalizationOutput, FinalizationStageRunner,
    LayerArena, LayerStageError, LayerStageOutput, LayerStageRunner, PostpassError, PostpassOutput,
    PostpassStageRunner, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner, WasmEngine,
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
        "PrePass::SeamPlanning" => Some("run-seam-planning"),
        "PrePass::SupportGeometry" => Some("run-support-geometry"),
        "PrePass::PaintSegmentation" => Some("run-paint-segmentation"),
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

fn convert_postpass_role_to_wit(
    role: &slicer_ir::ExtrusionRole,
) -> wit_host::postpass::slicer::world_postpass::geometry::ExtrusionRole {
    use wit_host::postpass::slicer::world_postpass::geometry::ExtrusionRole as WitExtrusionRole;

    match role {
        slicer_ir::ExtrusionRole::OuterWall => WitExtrusionRole::OuterWall,
        slicer_ir::ExtrusionRole::InnerWall => WitExtrusionRole::InnerWall,
        slicer_ir::ExtrusionRole::ThinWall => WitExtrusionRole::ThinWall,
        slicer_ir::ExtrusionRole::TopSolidInfill => WitExtrusionRole::TopSolidInfill,
        slicer_ir::ExtrusionRole::BottomSolidInfill => WitExtrusionRole::BottomSolidInfill,
        slicer_ir::ExtrusionRole::SparseInfill => WitExtrusionRole::SparseInfill,
        slicer_ir::ExtrusionRole::SupportMaterial => WitExtrusionRole::SupportMaterial,
        slicer_ir::ExtrusionRole::SupportInterface => WitExtrusionRole::SupportInterface,
        slicer_ir::ExtrusionRole::Ironing => WitExtrusionRole::Ironing,
        slicer_ir::ExtrusionRole::BridgeInfill => WitExtrusionRole::BridgeInfill,
        slicer_ir::ExtrusionRole::WipeTower => WitExtrusionRole::WipeTower,
        slicer_ir::ExtrusionRole::Custom(tag) => WitExtrusionRole::Custom(tag.clone()),
        slicer_ir::ExtrusionRole::PrimeTower => {
            WitExtrusionRole::Custom(wit_host::BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::Skirt => {
            WitExtrusionRole::Custom(wit_host::BUILTIN_EXTRUSION_ROLE_SKIRT_TAG.to_string())
        }
    }
}

/// Convert host-side `slicer_ir::RetractMode` to the WIT enum used by the
/// postpass-module bindings (host→guest direction).
fn retract_mode_to_postpass_wit(mode: RetractMode) -> wit_host::postpass::RetractMode {
    use wit_host::postpass::RetractMode as PostpassRetractMode;
    match mode {
        RetractMode::Gcode => PostpassRetractMode::Gcode,
        RetractMode::Firmware => PostpassRetractMode::Firmware,
    }
}

fn convert_gcode_command_to_postpass_wit(
    command: &GCodeCommand,
) -> wit_host::postpass::GcodeCommand {
    match command {
        GCodeCommand::Move {
            x,
            y,
            z,
            e,
            f,
            role,
        } => wit_host::postpass::GcodeCommand::Move(wit_host::postpass::GcodeMoveCmd {
            x: *x,
            y: *y,
            z: *z,
            e: *e,
            f: *f,
            role: convert_postpass_role_to_wit(role),
        }),
        GCodeCommand::Retract {
            length,
            speed,
            mode,
        } => wit_host::postpass::GcodeCommand::Retract(wit_host::postpass::GcodeRetractCmd {
            length: *length,
            speed: *speed,
            mode: retract_mode_to_postpass_wit(*mode),
        }),
        GCodeCommand::Unretract {
            length,
            speed,
            mode,
        } => wit_host::postpass::GcodeCommand::Unretract(wit_host::postpass::GcodeRetractCmd {
            length: *length,
            speed: *speed,
            mode: retract_mode_to_postpass_wit(*mode),
        }),
        GCodeCommand::FanSpeed { value } => {
            wit_host::postpass::GcodeCommand::FanSpeed(wit_host::postpass::GcodeFanSpeedCmd {
                value: *value,
            })
        }
        GCodeCommand::Temperature {
            tool,
            celsius,
            wait,
        } => {
            wit_host::postpass::GcodeCommand::Temperature(wit_host::postpass::GcodeTemperatureCmd {
                tool: *tool,
                celsius: *celsius,
                wait: *wait,
            })
        }
        GCodeCommand::ToolChange {
            after_entity_index,
            from,
            to,
        } => wit_host::postpass::GcodeCommand::ToolChange(wit_host::postpass::GcodeToolChangeCmd {
            after_entity_index: *after_entity_index,
            from_tool: *from,
            to_tool: *to,
        }),
        GCodeCommand::Comment { text } => wit_host::postpass::GcodeCommand::Comment(text.clone()),
        GCodeCommand::Raw { text } => wit_host::postpass::GcodeCommand::Raw(text.clone()),
        // ExtrusionMode is not yet a WIT variant; pass through as Raw so postpass
        // modules see the correct M82/M83 line.
        GCodeCommand::ExtrusionMode { absolute } => {
            wit_host::postpass::GcodeCommand::Raw(if *absolute {
                "M82".to_string()
            } else {
                "M83".to_string()
            })
        }
    }
}

fn collect_postpass_output(
    commands: &[wit_host::GcodeCommandCollected],
) -> Result<Option<Vec<GCodeCommand>>, String> {
    if commands.is_empty() {
        return Ok(None);
    }

    let mut collected = Vec::with_capacity(commands.len());
    for (index, command) in commands.iter().enumerate() {
        let converted = match command {
            wit_host::GcodeCommandCollected::Move(cmd) => GCodeCommand::Move {
                x: cmd.x,
                y: cmd.y,
                z: cmd.z,
                e: cmd.e,
                f: cmd.f,
                role: wit_host::convert_extrusion_role(&cmd.role),
            },
            wit_host::GcodeCommandCollected::Retract {
                length,
                speed,
                mode,
            } => GCodeCommand::Retract {
                length: *length,
                speed: *speed,
                mode: *mode,
            },
            wit_host::GcodeCommandCollected::Unretract {
                length,
                speed,
                mode,
            } => GCodeCommand::Unretract {
                length: *length,
                speed: *speed,
                mode: *mode,
            },
            wit_host::GcodeCommandCollected::FanSpeed(value) => {
                GCodeCommand::FanSpeed { value: *value }
            }
            wit_host::GcodeCommandCollected::Temperature {
                tool,
                celsius,
                wait,
            } => GCodeCommand::Temperature {
                tool: *tool,
                celsius: *celsius,
                wait: *wait,
            },
            wit_host::GcodeCommandCollected::ToolChange {
                after_entity_index,
                from_tool,
                to_tool,
            } => GCodeCommand::ToolChange {
                after_entity_index: *after_entity_index,
                from: *from_tool,
                to: *to_tool,
            },
            wit_host::GcodeCommandCollected::Comment(text) => {
                GCodeCommand::Comment { text: text.clone() }
            }
            wit_host::GcodeCommandCollected::Raw(text) => GCodeCommand::Raw { text: text.clone() },
            wit_host::GcodeCommandCollected::ZHop { .. } => {
                return Err(format!(
                    "postpass gcode output command {index} used push-z-hop, but GCodeIR has no z-hop command variant"
                ));
            }
        };
        collected.push(converted);
    }

    Ok(Some(collected))
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
    seam_plan_ir: Option<&'a slicer_ir::SeamPlanIR>,
    support_plan_ir: Option<&'a slicer_ir::SupportPlanIR>,
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
    #[allow(clippy::too_many_arguments)]
    fn dispatch_layer_call(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
        layer_index: u32,
        layer_z: f32,
        envelope_floor: f32,
        envelope_height: f32,
        paint_ir: Option<&slicer_ir::PaintRegionIR>,
        _seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
        support_plan_ir: Option<&slicer_ir::SupportPlanIR>,
        arena: &LayerArena,
    ) -> Result<HostExecutionContext, DispatchError> {
        let export_name = export_name_for_stage(stage_id).ok_or_else(|| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: String::new(),
            phase: DispatchPhase::UnknownStage,
            reason: format!("no export mapping for stage '{stage_id}'"),
        })?;

        let component = module
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
        wit_host::LayerModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        )
        .map_err(|e| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::LinkerSetup,
            reason: e.to_string(),
        })?;

        // Create per-call execution context and store.
        let ctx = HostExecutionContextBuilder::new(
            module.module_id.clone(),
            envelope_floor,
            envelope_height,
        )
        .mesh_ir(Some(blackboard.mesh().clone()))
        .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        // Wire the per-call MemTracker as the store's ResourceLimiter so
        // wasmtime notifies it on memory.grow (and on initial instantiation
        // sizing). Sampled after the typed export returns.
        store.limiter(|ctx| &mut ctx.mem_tracker);

        // Resolve fill-role held claims per region for this module call. The
        // resolver intersects the module's manifest `[claims].holds` with the
        // per-region `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder`
        // selection so each region carries the authoritative effective set
        // (packet 37). Stages without a staged `SliceIR` get an empty map.
        let held_claims_map: std::collections::HashMap<(String, String), Vec<String>> =
            if let Some(slice_ir) = arena.slice() {
                slice_ir
                    .regions
                    .iter()
                    .map(|region| {
                        let key = slicer_ir::RegionKey {
                            global_layer_index: layer_index,
                            object_id: region.object_id.clone(),
                            region_id: region.region_id,
                        };
                        let config = blackboard
                            .region_map()
                            .and_then(|map| map.entries.get(&key))
                            .map(|plan| plan.config.clone())
                            .unwrap_or_default();
                        let holders = crate::validation::FillHolders {
                            top: &config.top_fill_holder,
                            bottom: &config.bottom_fill_holder,
                            bridge: &config.bridge_fill_holder,
                            sparse: &config.sparse_fill_holder,
                        };
                        let held = crate::validation::resolve_held_claims(
                            &module.module_id,
                            &module.claims,
                            &holders,
                        );
                        (
                            (region.object_id.clone(), region.region_id.to_string()),
                            held,
                        )
                    })
                    .collect()
            } else {
                std::collections::HashMap::new()
            };
        store.data_mut().set_held_claims_per_region(held_claims_map);

        // Push config-view resource derived from the per-region overlay config.
        //
        // Packet 51: the ConfigView handed to each layer-tier module must come
        // from the already-overlaid `RegionPlan.config` stored in the
        // `RegionMapIR`, NOT from the module's frozen base `config_view`.  We
        // pick the first region on this layer as the representative config
        // (single-object models have exactly one; multi-object models share the
        // same paint semantic coverage, so any region is equivalent for the
        // per-call single config handle).  Fall back to the module's frozen
        // config when no region map or no entry is found.
        let effective_config_view: slicer_ir::ConfigView = blackboard
            .region_map()
            .and_then(|map| {
                map.entries
                    .iter()
                    .find(|(key, _)| key.global_layer_index == layer_index)
                    .map(|(_, plan)| {
                        let region_map = resolved_config_to_map(&plan.config);
                        let declared_keys = module.config_view.keys();
                        slicer_ir::ConfigView::from_declared(
                            &region_map,
                            declared_keys.iter().map(String::as_str),
                        )
                    })
            })
            .unwrap_or_else(|| module.config_view.as_ref().clone());
        let config_handle = store
            .data_mut()
            .push_config_view(wit_host::config_view_to_data(&effective_config_view))
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push config resource: {e}"),
            })?;

        // Instantiate component through typed bindings.
        let bindings =
            wit_host::LayerModule::instantiate(&mut store, component.wasmtime_component(), &linker)
                .map_err(|e| DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::TypedInstantiation,
                    reason: e.to_string(),
                })?;

        // Snapshot the post-instantiation memory size — captures the module's
        // baseline linear-memory allocation before the export call runs.
        let mem_initial_bytes = store.data().mem_tracker.current_bytes;

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
                seam_plan_ir: blackboard.seam_plan().map(|arc| arc.as_ref()),
                support_plan_ir,
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

        // Sample the wasm linear-memory tracker before consuming the store.
        // `mem_initial_bytes` captured the post-instantiation baseline above;
        // `peak_bytes` is the highwater observed across both instantiation
        // and the export call (always >= initial because wasm memory only
        // grows). The pair (initial, peak) lets the report distinguish
        // module static cost from per-call dynamic growth.
        let mem_peak_bytes = store.data().mem_tracker.peak_bytes;
        LAST_WASM_MEM_SAMPLE.with(|c| c.set((mem_initial_bytes, mem_peak_bytes)));

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
                let region_handles = push_slice_regions(config.store, params.arena, params.layer_z)
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
                    push_perimeter_regions(config.store, params.arena, None, params.layer_index)
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
                let region_handles = push_slice_regions(config.store, params.arena, params.layer_z)
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
                let region_handles = push_slice_regions(config.store, params.arena, params.layer_z)
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
                    params.arena,
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
                let region_handles = push_slice_regions(config.store, params.arena, params.layer_z)
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
                let region_handles = push_slice_regions(config.store, params.arena, params.layer_z)
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
                    push_perimeter_regions(config.store, params.arena, None, params.layer_index)
                        .map_err(mk_ctx_err)?;
                let output = config
                    .store
                    .data_mut()
                    .push_gcode_output_builder()
                    .map_err(mk_ctx_err)?;
                let snapshot = project_ordered_entities(params.arena);
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
        let component = module
            .wasm_component
            .as_ref()
            .ok_or_else(|| DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::MissingComponent,
                reason: "no compiled WASM component available".to_string(),
            })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<wit_host::HostExecutionContext>::new(engine);
        wit_host::PrepassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        )
        .map_err(|e| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::LinkerSetup,
            reason: e.to_string(),
        })?;

        let ctx = wit_host::HostExecutionContextBuilder::new(module.module_id.clone(), 0.0, 0.0)
            .mesh_ir(Some(blackboard.mesh().clone()))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

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

        let bindings = wit_host::PrepassModule::instantiate(
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

        let mk_call_err = |e: wasmtime::Error| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::TypedExportCall,
            reason: e.to_string(),
        };
        let mk_ctx_err = |e: wasmtime::Error| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::ContextCreation,
            reason: e.to_string(),
        };

        // Build the appropriate WIT view for each stage.
        // MeshAnalysis and LayerPlanning still pass object IDs (they don't need geometry).
        // MeshSegmentation and PaintSegmentation pass geometry views.
        let call_result = match stage_id.as_str() {
            "PrePass::MeshAnalysis" => {
                let object_ids: Vec<String> = blackboard
                    .mesh()
                    .objects
                    .iter()
                    .map(|o| o.id.clone())
                    .collect();
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
                let object_ids: Vec<String> = blackboard
                    .mesh()
                    .objects
                    .iter()
                    .map(|o| o.id.clone())
                    .collect();
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
            "PrePass::MeshSegmentation" => {
                let mesh_object_views: Vec<_> = blackboard
                    .mesh()
                    .objects
                    .iter()
                    .map(wit_host::object_mesh_to_wit_mesh_object_view)
                    .collect();
                let output = store
                    .data_mut()
                    .push_mesh_segmentation_output()
                    .map_err(mk_ctx_err)?;
                bindings
                    .call_run_mesh_segmentation(
                        &mut store,
                        &mesh_object_views,
                        own(output),
                        own(config_handle),
                    )
                    .map_err(mk_call_err)
            }
            "PrePass::SeamPlanning" => {
                let mesh_object_views: Vec<_> = blackboard
                    .mesh()
                    .objects
                    .iter()
                    .map(wit_host::object_mesh_to_wit_mesh_object_view)
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
                let mesh_object_views: Vec<_> = blackboard
                    .mesh()
                    .objects
                    .iter()
                    .map(wit_host::object_mesh_to_wit_mesh_object_view)
                    .collect();
                let layer_plan_view = blackboard
                    .layer_plan()
                    .map(|lp| wit_host::project_layer_plan_view(lp))
                    .unwrap_or_else(|| wit_host::prepass::LayerPlanView { layers: Vec::new() });
                let region_segmentation_view = blackboard
                    .region_map()
                    .map(|rm| wit_host::project_region_segmentation_view(rm))
                    .unwrap_or_else(|| wit_host::prepass::RegionSegmentationView {
                        entries: Vec::new(),
                    });
                let support_geometry_view = blackboard
                    .support_geometry()
                    .map(|sg| wit_host::project_support_geometry_view(sg))
                    .unwrap_or_else(|| wit_host::prepass::SupportGeometryView {
                        entries: Vec::new(),
                    });
                // run-support-geometry returns support-geometry-output directly
                // (no output resource, no config param; returns record not result).
                // The returned support-plan-entries are stashed on the context
                // by push_support_geometry_result so harvest_support_plan_ir
                // can drain them after the call returns.
                let sg_output = bindings
                    .call_run_support_geometry(
                        &mut store,
                        &mesh_object_views,
                        &layer_plan_view,
                        &region_segmentation_view,
                        &support_geometry_view,
                    )
                    .map_err(mk_call_err)?;
                store
                    .data_mut()
                    .push_support_geometry_result(sg_output)
                    .map_err(mk_ctx_err)?;
                // Synthesise a success result matching the outer call_result type
                // (other arms return Result<Result<(), ModuleError>, DispatchError>).
                Ok::<Result<(), wit_host::prepass::ModuleError>, DispatchError>(Ok(()))
            }
            _ => Err(DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::UnknownStage,
                reason: format!("no typed prepass export for stage '{stage_id}'"),
            }),
        }?;

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
        blackboard: &Blackboard,
        layers: &[slicer_ir::LayerCollectionIR],
    ) -> Result<Vec<wit_host::FinalizationBuilderPush>, DispatchError> {
        let export_name = export_name_for_stage(stage_id).unwrap_or("unknown");
        let component = module
            .wasm_component
            .as_ref()
            .ok_or_else(|| DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::MissingComponent,
                reason: "no compiled WASM component available".to_string(),
            })?;

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        wit_host::FinalizationModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        )
        .map_err(|e| DispatchError {
            module_id: module.module_id.clone(),
            stage_id: stage_id.clone(),
            export_name: export_name.to_string(),
            phase: DispatchPhase::LinkerSetup,
            reason: e.to_string(),
        })?;

        let ctx = HostExecutionContextBuilder::new(module.module_id.clone(), 0.0, 0.0)
            .mesh_ir(Some(blackboard.mesh().clone()))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

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

        let output_handle = store
            .data_mut()
            .push_finalization_output_builder()
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::ContextCreation,
                reason: format!("failed to push finalization output resource: {e}"),
            })?;

        // Deep-copy each completed layer into a wit-bindgen
        // LayerCollectionView handle so the guest sees real metadata
        // rather than the previous empty-shell stub (docs/03
        // world-finalization.wit `resource layer-collection-view`).
        let mut layer_handles = Vec::with_capacity(layers.len());
        for layer in layers {
            let h = store
                .data_mut()
                .push_finalization_layer_view(layer)
                .map_err(|e| DispatchError {
                    module_id: module.module_id.clone(),
                    stage_id: stage_id.clone(),
                    export_name: export_name.to_string(),
                    phase: DispatchPhase::ContextCreation,
                    reason: format!("failed to push layer-collection-view resource: {e}"),
                })?;
            layer_handles.push(own(h));
        }

        let bindings = wit_host::FinalizationModule::instantiate(
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

        let call_result = bindings
            .call_run_finalization(
                &mut store,
                &layer_handles,
                own(output_handle),
                own(config_handle),
            )
            .map_err(|e| DispatchError {
                module_id: module.module_id.clone(),
                stage_id: stage_id.clone(),
                export_name: export_name.to_string(),
                phase: DispatchPhase::TypedExportCall,
                reason: e.to_string(),
            })?;

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
        blackboard: &Blackboard,
        commands: &[GCodeCommand],
    ) -> (
        Result<Option<Vec<GCodeCommand>>, DispatchError>,
        Vec<String>,
    ) {
        let export_name = "run-gcode-postprocess";
        let component = match module.wasm_component.as_ref() {
            Some(c) => c,
            None => {
                return (
                    Err(DispatchError {
                        module_id: module.module_id.clone(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::MissingComponent,
                        reason: "no compiled WASM component available".to_string(),
                    }),
                    Vec::new(),
                )
            }
        };

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        if let Err(e) = wit_host::PostpassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        ) {
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

        let ctx = HostExecutionContextBuilder::new(module.module_id.clone(), 0.0, 0.0)
            .mesh_ir(Some(blackboard.mesh().clone()))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        let config_handle = match store
            .data_mut()
            .push_config_view(wit_host::config_view_to_data(&module.config_view))
        {
            Ok(h) => h,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module.module_id.clone(),
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
                        module_id: module.module_id.clone(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::ContextCreation,
                        reason: format!("failed to push gcode output resource: {e}"),
                    }),
                    Vec::new(),
                )
            }
        };

        let bindings = match wit_host::PostpassModule::instantiate(
            &mut store,
            component.wasmtime_component(),
            &linker,
        ) {
            Ok(b) => b,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module.module_id.clone(),
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
                let output = match collect_postpass_output(&store.data().gcode_output.commands) {
                    Ok(output) => output,
                    Err(reason) => {
                        return (
                            Err(DispatchError {
                                module_id: module.module_id.clone(),
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
                    module_id: module.module_id.clone(),
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
        blackboard: &Blackboard,
        text: &str,
    ) -> (Result<String, DispatchError>, Vec<String>) {
        let export_name = "run-text-postprocess";
        let component = match module.wasm_component.as_ref() {
            Some(c) => c,
            None => {
                return (
                    Err(DispatchError {
                        module_id: module.module_id.clone(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::MissingComponent,
                        reason: "no compiled WASM component available".to_string(),
                    }),
                    Vec::new(),
                )
            }
        };

        let _lease = module.instance_pool.acquire();
        let engine = self.engine.wasmtime_engine();

        let mut linker = wasmtime::component::Linker::<HostExecutionContext>::new(engine);
        if let Err(e) = wit_host::PostpassModule::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |ctx| ctx,
        ) {
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

        let ctx = HostExecutionContextBuilder::new(module.module_id.clone(), 0.0, 0.0)
            .mesh_ir(Some(blackboard.mesh().clone()))
            .build();
        let mut store = wasmtime::Store::new(engine, ctx);
        store.limiter(|ctx| &mut ctx.mem_tracker);

        let config_handle = match store
            .data_mut()
            .push_config_view(wit_host::config_view_to_data(&module.config_view))
        {
            Ok(h) => h,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module.module_id.clone(),
                        stage_id: stage_id.clone(),
                        export_name: export_name.to_string(),
                        phase: DispatchPhase::ContextCreation,
                        reason: format!("failed to push config resource: {e}"),
                    }),
                    Vec::new(),
                )
            }
        };

        let bindings = match wit_host::PostpassModule::instantiate(
            &mut store,
            component.wasmtime_component(),
            &linker,
        ) {
            Ok(b) => b,
            Err(e) => {
                return (
                    Err(DispatchError {
                        module_id: module.module_id.clone(),
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
                    module_id: module.module_id.clone(),
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
    build_paint_layer_data_with_plan(paint_ir, layer_index, None)
}

/// Variant of [`build_paint_layer_data`] that also indexes a committed
/// `SupportPlanIR` for this layer so the WIT
/// `paint-region-layer-view::support-plan-segments` accessor can serve it
/// to Layer::Support modules that declare `SupportPlanIR` as a manifest
/// read (e.g. `tree-support`).
fn build_paint_layer_data_with_plan(
    paint_ir: Option<&slicer_ir::PaintRegionIR>,
    layer_index: u32,
    support_plan_ir: Option<&slicer_ir::SupportPlanIR>,
) -> PaintRegionLayerData {
    let mut data = match paint_ir {
        Some(ir) => wit_host::paint_region_ir_to_layer_data(ir, layer_index),
        None => PaintRegionLayerData {
            layer_index,
            regions_by_semantic: HashMap::new(),
            custom_regions: HashMap::new(),
            support_plan_segments: HashMap::new(),
        },
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
                    .map(
                        |p| wit_host::layer::slicer::world_layer::geometry::Point3WithWidth {
                            x: p.x,
                            y: p.y,
                            z: p.z,
                            width: p.width,
                            flow_factor: p.flow_factor,
                            overhang_quartile: p.overhang_quartile,
                        },
                    )
                    .collect();
                bucket.push(pts);
            }
        }
    }
    data
}

fn derive_layer_output_envelope(layer: &GlobalLayer, arena: &LayerArena) -> (f32, f32) {
    let fallback_height = arena
        .slice()
        .and_then(|slice_ir| slice_ir.regions.first())
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
        log::warn!(
            "derive_layer_output_envelope: invalid envelope (floor={}, ceiling={}) for layer z={}, using fallback height={}",
            floor, ceiling, layer.z, fallback_height
        );
        return (layer.z, fallback_height);
    }

    (floor, ceiling - floor)
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
        let held_claims = store
            .data()
            .held_claims_for(&region.object_id, &region.region_id.to_string())
            .to_vec();
        let data = wit_host::sliced_region_to_data(region, layer_z, held_claims);
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
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
    layer_index: u32,
) -> Result<Vec<Resource<wit_host::PerimeterRegionData>>, wasmtime::Error> {
    let perimeter_ir = match arena.perimeter() {
        Some(ir) => ir,
        None => return Ok(Vec::new()),
    };

    if let Some(seam_ir) = seam_plan_ir {
        eprintln!(
            "seam_plan_ir has {} entries, looking for layer={}",
            seam_ir.entries.len(),
            layer_index
        );
    }

    let mut handles = Vec::with_capacity(perimeter_ir.regions.len());
    for region in &perimeter_ir.regions {
        let mut data = wit_host::perimeter_region_to_data(region);
        // Inject resolved seam from SeamPlanIR if available for this region.
        if let Some(seam_ir) = seam_plan_ir {
            if let Some(entry) = seam_ir.entries.iter().find(|e| {
                e.region_key.global_layer_index == layer_index
                    && e.region_key.object_id == region.object_id
                    && e.region_key.region_id == region.region_id
            }) {
                data.resolved_seam = Some((
                    wit_host::Point3 {
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
///
/// # Region-ID canonicalization
///
/// WIT `region-id` is declared as a string (docs/02 §Canonical ID Types).
/// The canonical host form is a decimal `u64` string with no leading zeros.
/// Any non-canonical value is rejected as a fatal contract error.
///
/// # Validation
///
/// - `z` must be finite and non-negative (enforced in `push_layer`).
/// - `effective_layer_height` must be finite and positive (enforced in `push_layer`).
/// - `GlobalLayer.index` must be `< 100_000` (docs/02 §Bounds).
fn parse_canonical_region_id(raw: &str) -> Result<u64, String> {
    let parsed = raw.parse::<u64>().map_err(|_| {
        format!("expected canonical decimal u64 string with no leading zeros, got '{raw}'")
    })?;

    if parsed.to_string() != raw {
        return Err(format!(
            "expected canonical decimal u64 string with no leading zeros, got '{raw}'"
        ));
    }

    Ok(parsed)
}

fn harvest_layer_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: wit_host::HostExecutionContext,
) -> Result<slicer_ir::LayerPlanIR, String> {
    use slicer_ir::{ActiveRegion, GlobalLayer, LayerPlanIR, ObjectLayerRef, ResolvedConfig};
    use std::collections::HashMap;

    let proposals = ctx.layer_plan_proposals;

    const MAX_LAYERS: u32 = 100_000;

    let mut global_layers: Vec<GlobalLayer> = Vec::with_capacity(proposals.len());
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
            let region_id =
                parse_canonical_region_id(&region_prop.region_id).map_err(|reason| {
                    format!(
                        "layer-plan-output: region '{}'/'{}' has invalid region-id: {reason}",
                        region_prop.object_id, region_prop.region_id
                    )
                })?;

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

            let obj_refs = object_participation
                .entry(region_prop.object_id.clone())
                .or_default();
            let already_referenced = obj_refs.iter().any(|r| r.global_layer_index == index);
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
        global_layers,
        object_participation,
        ..Default::default()
    })
}

// ── Seam-plan harvest ──────────────────────────────────────────────────────

/// Convert WIT `SeamPlanEntry` records collected by a `PrePass::SeamPlanning`
/// call into a host-side [`slicer_ir::SeamPlanIR`].
///
/// Entries are keyed by `(global_layer_index, object_id, region_id)` and
/// deduplicated — if two entries share the same key the second wins.
fn harvest_seam_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: wit_host::HostExecutionContext,
) -> Result<slicer_ir::SeamPlanIR, String> {
    use slicer_ir::{RegionKey, ScoredSeamCandidate, SeamPlanEntry, SeamPlanIR, SeamPosition};
    use std::collections::HashMap;

    let mut seen: HashMap<RegionKey, ()> = HashMap::new();
    let mut entries: Vec<SeamPlanEntry> = Vec::with_capacity(ctx.seam_plan_entries.len());

    for entry in ctx.seam_plan_entries.into_iter() {
        let region_id = parse_canonical_region_id(&entry.region_id).map_err(|reason| {
            format!(
                "seam-planning-output: region '{}'/'{}' has invalid region-id: {reason}",
                entry.object_id, entry.region_id
            )
        })?;

        let region_key = RegionKey {
            global_layer_index: entry.global_layer_index,
            object_id: entry.object_id.clone(),
            region_id,
        };

        // Deduplicate: later entry wins for same key
        let is_duplicate = seen.contains_key(&region_key);
        seen.insert(region_key.clone(), ());
        if is_duplicate {
            continue;
        }

        let scored_candidates: Vec<ScoredSeamCandidate> = entry
            .scored_candidates
            .iter()
            .map(|sc| ScoredSeamCandidate {
                position: slicer_ir::Point3WithWidth {
                    x: sc.position.x,
                    y: sc.position.y,
                    z: sc.position.z,
                    width: sc.position.width,
                    flow_factor: sc.position.flow_factor,
                    overhang_quartile: sc.position.overhang_quartile,
                },
                score: sc.score,
                reason: match sc.reason.tag.as_str() {
                    "concave" => slicer_ir::SeamReason::Concave,
                    "sharp" => slicer_ir::SeamReason::Sharp,
                    "user_forced" => slicer_ir::SeamReason::UserForced,
                    _ => slicer_ir::SeamReason::Aligned,
                },
            })
            .collect();

        let chosen_candidate = SeamPosition {
            point: slicer_ir::Point3WithWidth {
                x: entry.chosen_position.x,
                y: entry.chosen_position.y,
                z: entry.chosen_position.z,
                width: entry.chosen_position.width,
                flow_factor: entry.chosen_position.flow_factor,
                overhang_quartile: entry.chosen_position.overhang_quartile,
            },
            wall_index: entry.chosen_wall_index,
        };

        entries.push(SeamPlanEntry {
            region_key,
            chosen_candidate,
            scored_candidates,
        });
    }

    Ok(SeamPlanIR {
        entries,
        ..Default::default()
    })
}

// ── Support-plan harvest ───────────────────────────────────────────────────

/// Convert WIT `SupportPlanEntry` records collected by a
/// `PrePass::SupportGeometry` call into a host-side
/// [`slicer_ir::SupportPlanIR`].
///
/// Entries are preserved in push order (the harvester does not deduplicate —
/// multiple `(layer, object, region)` entries with distinct branch segment
/// sets are legal because different parts of an object may share a
/// `region_id` bucket across layers).
fn harvest_support_plan_ir(
    _stage_id: &str,
    _module_id: &str,
    ctx: wit_host::HostExecutionContext,
) -> Result<slicer_ir::SupportPlanIR, String> {
    use slicer_ir::{
        ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SupportPlanEntry, SupportPlanIR,
    };

    let mut entries: Vec<SupportPlanEntry> = Vec::with_capacity(ctx.support_plan_entries.len());

    for entry in ctx.support_plan_entries.into_iter() {
        let region_id = parse_canonical_region_id(&entry.region_id).map_err(|reason| {
            format!(
                "support-generation-output: region '{}'/'{}' has invalid region-id: {reason}",
                entry.object_id, entry.region_id
            )
        })?;

        let mut branch_segments: Vec<ExtrusionPath3D> =
            Vec::with_capacity(entry.branch_segments.len());
        for segment in entry.branch_segments.into_iter() {
            let points: Vec<Point3WithWidth> = segment
                .into_iter()
                .map(|p| Point3WithWidth {
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    width: p.width,
                    flow_factor: p.flow_factor,
                    overhang_quartile: p.overhang_quartile,
                })
                .collect();
            branch_segments.push(ExtrusionPath3D {
                points,
                role: ExtrusionRole::SupportMaterial,
                speed_factor: 1.0,
            });
        }

        entries.push(SupportPlanEntry {
            global_layer_index: entry.global_layer_index,
            object_id: entry.object_id,
            region_id,
            branch_segments,
        });
    }

    Ok(SupportPlanIR {
        entries,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::harvest_layer_plan_ir;
    use crate::wit_host::{self, HostExecutionContextBuilder};

    #[test]
    fn harvest_layer_plan_ir_rejects_noncanonical_region_id_strings() {
        let mut ctx = HostExecutionContextBuilder::new(
            "com.test.layer-plan-bad-region-id".to_string(),
            0.0,
            0.2,
        )
        .build();
        ctx.layer_plan_proposals
            .push(wit_host::prepass::LayerProposal {
                z: 0.2,
                active_regions: vec![wit_host::prepass::RegionLayerProposal {
                    object_id: "obj-1".to_string(),
                    region_id: "01".to_string(),
                    effective_layer_height: 0.2,
                    is_catchup: false,
                    catchup_z_bottom: 0.0,
                }],
            });

        let err = harvest_layer_plan_ir(
            "PrePass::LayerPlanning",
            "com.test.layer-plan-bad-region-id",
            ctx,
        )
        .expect_err("non-canonical region-id must be rejected");

        assert!(
            err.contains("region-id") && err.contains("01"),
            "diagnostic must explain the rejected non-canonical region-id: {err}"
        );
    }
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
///
/// # WIT PaintValueInput → IR PaintValue mapping
///
/// | WIT `paint-value-input` variant | IR `PaintValue` variant       |
/// |----------------------------------|-------------------------------|
/// | `flag(bool)`                     | `PaintValue::Flag(bool)`      |
/// | `scalar(f32)`                    | `PaintValue::Scalar(f32)`     |
/// | `tool-index(u32)`                | `PaintValue::ToolIndex(u32)`  |
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
    use slicer_ir::{FacetPaintMark, MeshSegmentationIR};

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
        marks,
        ..Default::default()
    }
}

// ── Stage runner trait implementations ──────────────────────────────────
impl PrepassStageRunner for WasmRuntimeDispatcher {
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        let ctx = match self.dispatch_prepass_call(stage_id, module, blackboard) {
            Ok(ctx) => ctx,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                return Ok((PrepassStageOutput::None, Vec::new()));
            }
            Err(e) => {
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
            let ir = harvest_layer_plan_ir(stage_id, &module.module_id, ctx).map_err(|e| {
                PrepassExecutionError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e,
                }
            })?;
            return Ok((PrepassStageOutput::LayerPlan(Arc::new(ir)), runtime_reads));
        }

        // For the MeshSegmentation stage, convert collected triangle paint
        // marks to MeshSegmentationIR.
        if stage_id == "PrePass::MeshSegmentation" {
            let ir = harvest_mesh_segmentation_ir(ctx);
            return Ok((
                PrepassStageOutput::MeshSegmentation(Arc::new(ir)),
                runtime_reads,
            ));
        }

        // For the SeamPlanning stage, convert collected seam-plan entries
        // to SeamPlanIR.
        if stage_id == "PrePass::SeamPlanning" {
            let ir = harvest_seam_plan_ir(stage_id, &module.module_id, ctx).map_err(|e| {
                PrepassExecutionError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e,
                }
            })?;
            return Ok((PrepassStageOutput::SeamPlan(Arc::new(ir)), runtime_reads));
        }

        // For the SupportGeometry stage, convert collected support-plan
        // entries to SupportPlanIR.
        if stage_id == "PrePass::SupportGeometry" {
            let ir = harvest_support_plan_ir(stage_id, &module.module_id, ctx).map_err(|e| {
                PrepassExecutionError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e,
                }
            })?;
            return Ok((PrepassStageOutput::SupportPlan(Arc::new(ir)), runtime_reads));
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
            return Ok((
                PrepassStageOutput::MeshAnalysisAuxiliary(Arc::new(aux)),
                runtime_reads,
            ));
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
    use crate::prepass::{
        FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, SurfaceGroupRecord,
    };
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
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        // Extract paint region IR from blackboard for paint-consuming stages.
        let paint_ir = blackboard.paint_regions();
        let paint_ref = paint_ir.map(|arc| arc.as_ref());
        let seam_plan_ir = blackboard.seam_plan();
        let seam_plan_ref = seam_plan_ir.map(|arc| arc.as_ref());
        let support_plan_ir = blackboard.support_plan();
        let support_plan_ref = support_plan_ir.map(|arc| arc.as_ref());
        let (envelope_floor, envelope_height) = derive_layer_output_envelope(layer, arena);

        // Layer stages always use the typed component-model boundary.
        let mut ctx = match self.dispatch_layer_call(
            stage_id,
            module,
            blackboard,
            layer.index,
            layer.z,
            envelope_floor,
            envelope_height,
            paint_ref,
            seam_plan_ref,
            support_plan_ref,
            arena,
        ) {
            Ok(ctx) => ctx,
            Err(e) if e.phase == DispatchPhase::MissingComponent => {
                // Placeholder/uncompiled module — skip gracefully.
                return Ok((LayerStageOutput::Success, Vec::new(), Vec::new()));
            }
            Err(e) => {
                return Err(LayerStageError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                });
            }
        };

        // Preserve runtime reads and writes before committing outputs.
        let runtime_reads: Vec<String> = ctx.runtime_reads.clone();
        let runtime_writes: Vec<String> = ctx.runtime_writes.clone();

        // Layer::PathOptimization may have emitted a `set-entity-order`
        // proposal via `layer-collection-builder`. Validate and apply it
        // against the staged `LayerCollectionIR.ordered_entities` before
        // commit_layer_outputs runs (so any downstream commit sees the
        // post-permutation order). When no proposal was emitted, the
        // host fallback ordering pre-staged by the executor remains in
        // effect (packet-18 behavior preserved until packet 33).
        if stage_id == "Layer::PathOptimization" {
            if let Some(proposal) = ctx.layer_collection_proposal.take() {
                apply_entity_order_proposal(arena, &proposal).map_err(|message| {
                    LayerStageError::FatalModule {
                        stage_id: stage_id.clone(),
                        module_id: module.module_id.clone(),
                        message,
                    }
                })?;
            }
        }

        // Commit collected outputs into the layer arena based on stage.
        commit_layer_outputs(
            stage_id,
            &module.module_id,
            layer.index,
            &ctx,
            arena,
            seam_plan_ref,
        )?;

        // For Layer::Perimeters: inject seam from SeamPlanIR into arena.perimeter()
        // so PerimetersPostProcess can merge it into the guest output.
        // The seam was sent to the WASM store via PerimeterRegionData but was NOT
        // baked into the PerimeterIR committed above (WASM output doesn't carry it).
        if stage_id == "Layer::Perimeters" {
            if let Some(seam_ir) = seam_plan_ir {
                if let Some(mut perimeter) = arena.take_perimeter() {
                    for region in &mut perimeter.regions {
                        if region.resolved_seam.is_none() {
                            if let Some(entry) = seam_ir.entries.iter().find(|e| {
                                e.region_key.global_layer_index == layer.index
                                    && e.region_key.object_id == region.object_id
                                    && e.region_key.region_id == region.region_id
                            }) {
                                region.resolved_seam = Some(SeamPosition {
                                    point: entry.chosen_candidate.point,
                                    wall_index: entry.chosen_candidate.wall_index,
                                });
                            }
                        }
                    }
                    let _ = arena.set_perimeter(perimeter);
                }
            }
        }

        Ok((LayerStageOutput::Success, runtime_reads, runtime_writes))
    }

    fn last_wasm_mem_sample(&self) -> (u64, u64) {
        LAST_WASM_MEM_SAMPLE.with(|c| c.replace((0, 0)))
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
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
) -> Result<(), LayerStageError> {
    commit_layer_outputs(stage_id, module_id, layer_index, ctx, arena, seam_plan_ir)
}

/// Host-local projection of a single staged
/// `LayerCollectionIR.ordered_entities[i]` entry, mirroring the WIT
/// `ordered-entity-view` record. Built once per `Layer::PathOptimization`
/// invocation by [`project_ordered_entities`] and stashed on
/// [`crate::wit_host::LayerCollectionBuilderData`] so the host-side
/// `HostLayerCollectionBuilder::get_ordered_entities` impl can serve
/// repeated reads from a snapshot rather than the live arena.
#[derive(Debug, Clone)]
pub struct OrderedEntityView {
    /// Index into the host-staged `LayerCollectionIR.ordered_entities`
    /// at the time this snapshot was projected.
    pub original_index: u32,
    /// Region key of the entity at `original_index`.
    pub region_key: slicer_ir::RegionKey,
    /// Extrusion role of the entity's path.
    pub role: slicer_ir::ExtrusionRole,
    /// First point of `path.points`. PrintEntity invariant requires
    /// `path.points` to be non-empty.
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
/// Total: when no `LayerCollectionIR` is staged on the arena, returns
/// an empty `Vec` (no error). The caller is expected to stash this
/// snapshot on the builder resource so subsequent guest reads through
/// `layer-collection-builder.get-ordered-entities` are served from
/// the snapshot.
pub fn project_ordered_entities(arena: &LayerArena) -> Vec<OrderedEntityView> {
    let Some(lc) = arena.layer_collection() else {
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

/// Validate a `set-entity-order` proposal from a `Layer::PathOptimization`
/// module and apply it to the arena's staged `LayerCollectionIR.ordered_entities`.
///
/// Validation order — first failure short-circuits with the corresponding
/// diagnostic; on `Err` the arena's `ordered_entities` is left in its pre-call
/// state (no partial mutation):
/// 1. `proposal.len() == ordered_entities.len()` else
///    `"set-entity-order: expected N indices, got M"`
/// 2. each index in `[0, N)` else
///    `"set-entity-order: index N out of range [0, M)"`
/// 3. no duplicate indices else
///    `"set-entity-order: duplicate index N"`
///
/// On `Ok`, the entities are permuted into the proposed order; entries whose
/// reversal flag is `true` have `path.points` reversed in place; each entity's
/// `topo_order` is reassigned to its new 0-based slot.
pub fn apply_entity_order_proposal(
    arena: &mut LayerArena,
    proposal: &[(u32, bool)],
) -> Result<(), String> {
    let n = arena
        .layer_collection()
        .ok_or_else(|| "set-entity-order: no LayerCollectionIR staged on arena".to_string())?
        .ordered_entities
        .len();
    if proposal.len() != n {
        return Err(format!(
            "set-entity-order: expected {} indices, got {}",
            n,
            proposal.len()
        ));
    }
    for (idx, _reverse) in proposal {
        if (*idx as usize) >= n {
            return Err(format!(
                "set-entity-order: index {} out of range [0, {})",
                idx, n
            ));
        }
    }
    let mut seen = vec![false; n];
    for (idx, _reverse) in proposal {
        let slot = *idx as usize;
        if seen[slot] {
            return Err(format!("set-entity-order: duplicate index {}", idx));
        }
        seen[slot] = true;
    }

    // Validation passed — apply permutation, per-entity reversal, and
    // topo_order reassignment. Take the staged IR, mutate in place, replace.
    let mut lc = arena
        .take_layer_collection()
        .expect("layer_collection presence verified above");
    let original = std::mem::take(&mut lc.ordered_entities);
    let mut buckets: Vec<Option<slicer_ir::PrintEntity>> = original.into_iter().map(Some).collect();
    let mut new_entities: Vec<slicer_ir::PrintEntity> = Vec::with_capacity(n);
    for (new_slot, (orig_idx, reverse)) in proposal.iter().enumerate() {
        let mut entity = buckets[*orig_idx as usize]
            .take()
            .expect("uniqueness validated above");
        if *reverse {
            entity.path.points.reverse();
        }
        entity.topo_order = new_slot as u32;
        new_entities.push(entity);
    }
    lc.ordered_entities = new_entities;
    arena.set_layer_collection(lc);
    Ok(())
}

fn commit_layer_outputs(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    ctx: &HostExecutionContext,
    arena: &mut LayerArena,
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
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
            // For PerimetersPostProcess: perimeter may have wall_loops (before rotation
            // from Layer::Perimeters) OR rotated_wall_loops (after rotation from seam-placer).
            // Skip only if BOTH are empty (genuinely no perimeter output).
            let has_any_output = if stage_id == "Layer::PerimetersPostProcess" {
                !perimeter.wall_loops.is_empty()
                    || !perimeter.rotated_wall_loops.is_empty()
                    || !perimeter.infill_areas.is_empty()
                    || !perimeter.seam_candidates.is_empty()
            } else {
                !perimeter.wall_loops.is_empty()
                    || !perimeter.infill_areas.is_empty()
                    || !perimeter.seam_candidates.is_empty()
            };
            if !has_any_output {
                return Ok(());
            }
            let ir = wit_host::convert_perimeter_output(perimeter, layer_index)
                .map_err(|r| mk_validation_err("perimeter", r))?;
            // For PerimetersPostProcess: preserve the original perimeter's
            // resolved_seam if it was pre-seeded from SeamPlanIR. The guest
            // only rotates wall loops; it does not re-emit a resolved_seam
            // through the WIT boundary, so we must not take/overwrite the
            // original perimeter slot which holds the injected seam.
            if stage_id == "Layer::PerimetersPostProcess" {
                // Take ownership of original perimeter so we can inject seam if needed.
                let mut original = arena.take_perimeter();
                // If we have seam_plan_ir and the original has no seam in any region,
                // inject from seam_plan_ir. This handles the case where perimeter was
                // pre-staged (e.g., by a test) without going through Layer::Perimeters.
                if let (Some(seam_ir), Some(ref mut orig_perim)) = (seam_plan_ir, &mut original) {
                    for region in &mut orig_perim.regions {
                        if region.resolved_seam.is_none() {
                            if let Some(entry) = seam_ir.entries.iter().find(|e| {
                                e.region_key.global_layer_index == layer_index
                                    && e.region_key.object_id == region.object_id
                                    && e.region_key.region_id == region.region_id
                            }) {
                                region.resolved_seam = Some(entry.chosen_candidate.clone());
                            }
                        }
                    }
                }
                if let Some(orig_perim) = original {
                    let mut ir_owned = ir;
                    for (idx, region) in ir_owned.regions.iter_mut().enumerate() {
                        if region.resolved_seam.is_none() {
                            if let Some(orig_region) = orig_perim.regions.get(idx) {
                                if let Some(rs) = &orig_region.resolved_seam {
                                    region.resolved_seam = Some(rs.clone());
                                }
                            }
                        }
                    }
                    // ir_owned now has seam (either from guest or copied from original).
                    // Take is already done above; set it back.
                    arena
                        .set_perimeter(ir_owned)
                        .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
                } else {
                    arena
                        .set_perimeter(ir)
                        .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
                }
            } else {
                let _ = arena.take_perimeter();
                arena
                    .set_perimeter(ir)
                    .map_err(|e| LayerStageError::ArenaCommit { source: e })?;
            }
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
            let existing = arena
                .take_slice()
                .ok_or_else(|| LayerStageError::FatalModule {
                    stage_id: stage_id.to_string(),
                    module_id: module_id.to_string(),
                    message: "Layer::SlicePostProcess has no staged SliceIR to merge into; \
                          Layer::Slice must commit per-region slice output first"
                        .into(),
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
            // gcode-output-builder. Accepted overrides:
            //   ToolChange        -> deferred_tool_changes -> LayerCollectionIR.tool_changes
            //   Comment/Raw       -> deferred_annotations  -> LayerCollectionIR.annotations
            //   ZHop              -> deferred_z_hops       -> LayerCollectionIR.z_hops
            //   Retract/Unretract -> deferred_retracts     -> LayerCollectionIR.retracts
            //   Move              -> deferred_travel_moves  -> LayerCollectionIR.travel_moves
            // FanSpeed/Temperature have no LayerCollectionIR mapping and are rejected.
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
            let mut accepted: Vec<slicer_ir::ToolChange> = Vec::new();
            let mut accepted_z_hops: Vec<slicer_ir::ZHop> = Vec::new();
            let mut accepted_retracts: Vec<crate::blackboard::DeferredRetract> = Vec::new();
            let mut accepted_travel_moves: Vec<crate::blackboard::DeferredTravelMove> = Vec::new();
            for (i, cmd) in ctx.gcode_output.commands.iter().enumerate() {
                match cmd {
                    GcodeCommandCollected::ToolChange {
                        after_entity_index,
                        from_tool,
                        to_tool,
                    } => {
                        accepted.push(slicer_ir::ToolChange {
                            after_entity_index: *after_entity_index,
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
                    GcodeCommandCollected::Move(cmd) => {
                        accepted_travel_moves.push(crate::blackboard::DeferredTravelMove {
                            after_entity_index: anchor,
                            x: cmd.x,
                            y: cmd.y,
                            z: cmd.z,
                            f: cmd.f,
                        });
                    }
                    GcodeCommandCollected::ZHop {
                        after_entity_index: _,
                        hop_height,
                    } => {
                        // Normalize the module-supplied after_entity_index to the same global
                        // `anchor` used for Retract/Move/Unretract so that gcode_emit.rs
                        // emits the canonical sequence (Retract→ZHop→Travel→Unretract) anchored
                        // at the same entity position. The module-supplied index reflects a
                        // per-region local count and is always ignored at this stage.
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
                            after_entity_index: anchor,
                            hop_height: *hop_height,
                        });
                    }
                    GcodeCommandCollected::Retract {
                        length,
                        speed,
                        mode,
                    } => {
                        // Packet 34: `mode` is now threaded onto `DeferredRetract`
                        // so the gcode_emit stage can materialize the correct
                        // opcode (`G1 E-...` for `Gcode`, bare `G10` for
                        // `Firmware`). The value originates from the
                        // path-optimization-default module's `retract_mode`
                        // ConfigView field.
                        accepted_retracts.push(crate::blackboard::DeferredRetract {
                            after_entity_index: anchor,
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
                        accepted_retracts.push(crate::blackboard::DeferredRetract {
                            after_entity_index: anchor,
                            length: *length,
                            speed: *speed,
                            is_unretract: true,
                            mode: *mode,
                        });
                    }
                    other => {
                        return Err(LayerStageError::FatalModule {
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
            for tc in accepted {
                arena.push_deferred_tool_change(tc);
            }
            for zh in accepted_z_hops {
                arena.push_deferred_z_hop(zh);
            }
            for r in accepted_retracts {
                arena.push_deferred_retract(r);
            }
            for tm in accepted_travel_moves {
                arena.push_deferred_travel_move(tm);
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
        let pushes = match self.dispatch_finalization_call(stage_id, module, _blackboard, layers) {
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
        //
        // All variants (EntityToLayer, EntityToLayerWithPriority, ModifyEntity,
        // SortLayerBy, InsertSyntheticLayerAfter, SyntheticLayer) are forwarded
        // to the SDK FinalizationOutputBuilder and applied via apply_to(), which
        // runs the full 5-phase merge sequence:
        //   1. Append pushed entities + stamp entity_ids.
        //   2. Stable-sort each modified layer by (effective_priority, original_index).
        //   3. Apply ModifyEntity ops in record order.
        //   4. Apply SortLayer ops in record order.
        //   5. Apply InsertSynthLayer ops in record order.
        //
        // Legacy EntityToLayer pushes (priority = 0) preserve skirt/brim prepend
        // semantics: after the priority-based sort they land before producer-emitted
        // entities whose role.default_priority() > 0.
        //
        // Legacy SyntheticLayer (append-to-end) pushes are collected separately
        // because the SDK apply_to() does not process the synthetic_layers backcompat
        // field; they are appended to `layers` after apply_to() completes.
        let mut sdk_builder = slicer_sdk::traits::FinalizationOutputBuilder::new();
        let mut legacy_synthetic_layers: Vec<(f32, Vec<slicer_ir::ExtrusionPath3D>)> = Vec::new();

        for push in pushes {
            match push {
                wit_host::FinalizationBuilderPush::EntityToLayer {
                    layer_index,
                    path,
                    region_key,
                } => {
                    // Legacy alias: priority = 0 (sorts before all producer entities).
                    sdk_builder
                        .push_entity_to_layer(layer_index, path, region_key)
                        .unwrap_or_else(|e| {
                            log::warn!("finalization: push_entity_to_layer rejected: {e}")
                        });
                }
                wit_host::FinalizationBuilderPush::EntityToLayerWithPriority {
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
                wit_host::FinalizationBuilderPush::ModifyEntity {
                    layer_index,
                    entity_id,
                    mutation,
                } => {
                    let sdk_mutation = match mutation {
                        wit_host::WitEntityMutation::SetSpeedFactor(v) => {
                            EntityMutation::SetSpeedFactor(v)
                        }
                        wit_host::WitEntityMutation::SetFlowFactor(v) => {
                            EntityMutation::SetFlowFactor(v)
                        }
                    };
                    sdk_builder
                        .modify_entity(layer_index, entity_id, sdk_mutation)
                        .unwrap_or_else(|e| {
                            log::warn!("finalization: modify_entity rejected: {e}")
                        });
                }
                wit_host::FinalizationBuilderPush::SortLayerBy { layer_index, key } => match key {
                    wit_host::WitSortKey::ByPriorityAndEntityId => {
                        sdk_builder
                            .sort_layer_by(layer_index, SortKey::ByPriorityAndEntityId)
                            .unwrap_or_else(|err| {
                                log::warn!(
                                    "finalization: sort_layer_by(ByPriorityAndEntityId) rejected: {err}"
                                )
                            });
                    }
                    wit_host::WitSortKey::ByEntityId => {
                        sdk_builder
                            .sort_layer_by(layer_index, SortKey::ByEntityId)
                            .unwrap_or_else(|err| {
                                log::warn!(
                                    "finalization: sort_layer_by(ByEntityId) rejected: {err}"
                                )
                            });
                    }
                    wit_host::WitSortKey::ByObjectIdThenPriority => {
                        sdk_builder
                            .sort_layer_by(layer_index, SortKey::ByObjectIdThenPriority)
                            .unwrap_or_else(|err| {
                                log::warn!(
                                    "finalization: sort_layer_by(ByObjectIdThenPriority) rejected: {err}"
                                )
                            });
                    }
                },
                wit_host::FinalizationBuilderPush::InsertSyntheticLayerAfter { idx, z, paths } => {
                    sdk_builder
                        .insert_synthetic_layer_after(idx, SyntheticLayerData { z, paths })
                        .unwrap_or_else(|e| {
                            log::warn!("finalization: insert_synthetic_layer_after rejected: {e}")
                        });
                }
                wit_host::FinalizationBuilderPush::SyntheticLayer { z, paths } => {
                    // Legacy append-to-end: collected here and appended after apply_to().
                    legacy_synthetic_layers.push((z, paths));
                }
                wit_host::FinalizationBuilderPush::InsertEntityAt {
                    layer_index,
                    position,
                    path,
                    region_key,
                } => {
                    sdk_builder
                        .insert_entity_at(layer_index, position, path, region_key)
                        .unwrap_or_else(|e| {
                            log::warn!("finalization: insert_entity_at rejected: {e}")
                        });
                }
                wit_host::FinalizationBuilderPush::SetEntityOrder { layer_index, items } => {
                    sdk_builder
                        .set_entity_order(layer_index, items)
                        .unwrap_or_else(|e| {
                            log::warn!("finalization: set_entity_order rejected: {e}")
                        });
                }
            }
        }

        // Run the full merge sequence (all 5 phases) once across the whole layer vec.
        sdk_builder
            .apply_to(layers)
            .map_err(|msg| FinalizationError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: format!("finalization merge failed: {msg}"),
            })?;

        // Append legacy SyntheticLayer pushes to the end of the layer collection.
        // These use the old WIT `insert-synthetic-layer(z, paths)` call which
        // semantically appends; they run after apply_to() so InsertSyntheticLayerAfter
        // ops (which can affect layers.len()) have already settled.
        for (z, paths) in legacy_synthetic_layers {
            let new_index = layers.len() as u32;
            let id_gen = slicer_ir::LayerEntityIdGen::new();
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

        Ok(FinalizationOutput::Success)
    }
}

impl PostpassStageRunner for WasmRuntimeDispatcher {
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        let (result, reads) =
            self.dispatch_postpass_gcode_call(stage_id, module, _blackboard, &gcode_ir.commands);
        // Store reads for later retrieval via take_runtime_reads
        if !reads.is_empty() {
            self.postpass_runtime_reads.borrow_mut().push(reads);
        }
        match result {
            Ok(Some(commands)) => {
                gcode_ir.commands = commands;
            }
            Ok(None) => {}
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
        let (result, reads) =
            self.dispatch_postpass_text_call(stage_id, module, _blackboard, &text);
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

/// Convert a [`ResolvedConfig`] struct into a flat `HashMap<ConfigKey, ConfigValue>`.
///
/// Used by the layer dispatch path (packet 51) to build a per-region `ConfigView`
/// from the already-overlaid `RegionPlan.config` rather than from the module's
/// frozen base config.  Only the fields that map to canonical config keys are
/// emitted; extension keys are passed through unchanged.
fn resolved_config_to_map(
    cfg: &slicer_ir::ResolvedConfig,
) -> std::collections::HashMap<String, slicer_ir::ConfigValue> {
    use slicer_ir::ConfigValue;
    let mut m = std::collections::HashMap::new();
    m.insert(
        "layer_height".to_string(),
        ConfigValue::Float(cfg.layer_height as f64),
    );
    m.insert(
        "line_width".to_string(),
        ConfigValue::Float(cfg.line_width as f64),
    );
    m.insert(
        "first_layer_height".to_string(),
        ConfigValue::Float(cfg.first_layer_height as f64),
    );
    m.insert(
        "first_layer_line_width".to_string(),
        ConfigValue::Float(cfg.first_layer_line_width as f64),
    );
    m.insert(
        "wall_count".to_string(),
        ConfigValue::Int(cfg.wall_count as i64),
    );
    m.insert(
        "outer_wall_speed".to_string(),
        ConfigValue::Float(cfg.outer_wall_speed as f64),
    );
    m.insert(
        "inner_wall_speed".to_string(),
        ConfigValue::Float(cfg.inner_wall_speed as f64),
    );
    if let Some(v) = cfg.arachne_min_feature_size {
        m.insert(
            "arachne_min_feature_size".to_string(),
            ConfigValue::Float(v as f64),
        );
    }
    m.insert(
        "infill_density".to_string(),
        ConfigValue::Float(cfg.infill_density as f64),
    );
    m.insert(
        "infill_angle".to_string(),
        ConfigValue::Float(cfg.infill_angle as f64),
    );
    m.insert(
        "infill_speed".to_string(),
        ConfigValue::Float(cfg.infill_speed as f64),
    );
    m.insert(
        "solid_infill_speed".to_string(),
        ConfigValue::Float(cfg.solid_infill_speed as f64),
    );
    m.insert(
        "top_shell_layers".to_string(),
        ConfigValue::Int(cfg.top_shell_layers as i64),
    );
    m.insert(
        "bottom_shell_layers".to_string(),
        ConfigValue::Int(cfg.bottom_shell_layers as i64),
    );
    m.insert(
        "support_enabled".to_string(),
        ConfigValue::Bool(cfg.support_enabled),
    );
    m.insert(
        "support_overhang_angle".to_string(),
        ConfigValue::Float(cfg.support_overhang_angle as f64),
    );
    if let Some(v) = cfg.nonplanar_max_angle_deg {
        m.insert(
            "nonplanar_max_angle_deg".to_string(),
            ConfigValue::Float(v as f64),
        );
    }
    if let Some(v) = cfg.nonplanar_shell_count {
        m.insert(
            "nonplanar_shell_count".to_string(),
            ConfigValue::Int(v as i64),
        );
    }
    if let Some(v) = cfg.nonplanar_amplitude {
        m.insert(
            "nonplanar_amplitude".to_string(),
            ConfigValue::Float(v as f64),
        );
    }
    if let Some(v) = cfg.smoothificator_target_height {
        m.insert(
            "smoothificator_target_height".to_string(),
            ConfigValue::Float(v as f64),
        );
    }
    if let Some(v) = cfg.smoothificator_adaptive {
        m.insert("smoothificator_adaptive".to_string(), ConfigValue::Bool(v));
    }
    // Pass extension keys through unchanged.
    for (k, v) in &cfg.extensions {
        m.insert(k.clone(), v.clone());
    }
    m
}
