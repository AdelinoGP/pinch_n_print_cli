//! Per-layer parallel executor contracts (TASK-031).
//!
//! This module defines the per-layer parallel execution contracts for running
//! all Tier-2 layer stages using rayon. Each layer gets its own `LayerArena`
//! for intermediate IR storage. Stages within each layer run sequentially,
//! but layers can be processed in parallel.

use std::fmt;
use std::sync::Arc;

use rayon::prelude::*;
use std::collections::HashMap;

use slicer_core::slice_mesh_ex;
use slicer_ir::{
    ConfigValue, ExPolygon, GlobalLayer, InfillIR, LayerCollectionIR, LayerEntityIdGen,
    LayerStageCommitData, ModuleId, PaintRegionIR, PaintSemantic, PerimeterIR, PrintEntity,
    RegionKey, RegionMapIR, StageId, SupportIR, WallFeatureFlags,
};
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageInput, LayerStageRunner, WasmComponent, WasmInstancePool,
};

use crate::instrumentation::{NoopInstrumentation, PipelineInstrumentation};
use crate::progress_events::ProgressEvent;
use crate::slice_postprocess::{
    execute_slice_postprocess_paint_annotation, paint_annotation_warnings_to_progress_events,
    SlicePostProcessPaintAnnotationError, SlicePostProcessPaintAnnotationRequest,
};
use crate::{Blackboard, BlackboardError, ExecutionPlan, LayerArena, ModuleAccessAudit};
use slicer_core::algos::prepass_slice::LayerSliceError;

/// Sink for per-layer progress events (e.g. host-built-in paint-annotation
/// fallback warnings). Must be `Sync` because the per-layer executor fans out
/// across rayon worker threads.
pub trait LayerProgressSink {
    /// Record one progress event. Implementations must be thread-safe.
    fn record(&self, event: ProgressEvent);
}

/// A no-op `LayerProgressSink` used when callers don't want events.
pub struct NoopLayerProgressSink;

impl LayerProgressSink for NoopLayerProgressSink {
    fn record(&self, _event: ProgressEvent) {}
}

/// Top-level execution failure for the per-layer parallel executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerExecutionError {
    /// Fatal error in one layer (layer index included).
    FatalLayer {
        /// Layer that failed.
        layer_index: u32,
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// Blackboard commit failed.
    BlackboardCommit {
        /// Layer that failed to commit.
        layer_index: u32,
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
    /// Rayon join failed (should never happen).
    ParallelJoin {
        /// Stable human-readable detail.
        message: String,
    },
    /// The host-built-in `Layer::Slice` stage failed.
    LayerSlice {
        /// Layer that failed.
        layer_index: u32,
        /// Underlying layer-slice failure.
        source: LayerSliceError,
    },
    /// The host-built-in paint-annotation step failed with a structured
    /// fatal error (missing paint region data, stale segment_annotations
    /// cardinality, or a deterministic custom-semantic conflict).
    PaintAnnotation {
        /// Layer that failed.
        layer_index: u32,
        /// Underlying paint-annotation failure.
        source: SlicePostProcessPaintAnnotationError,
    },
}

impl fmt::Display for LayerExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalLayer {
                layer_index,
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal layer execution failure at layer {layer_index} in {stage_id} for {module_id}: {message}"
            ),
            Self::BlackboardCommit {
                layer_index,
                source,
            } => write!(
                f,
                "blackboard commit failed for layer {layer_index}: {source}"
            ),
            Self::ParallelJoin { message } => {
                write!(f, "rayon parallel join failed: {message}")
            }
            Self::LayerSlice { layer_index, source } => write!(
                f,
                "built-in Layer::Slice failed at layer {layer_index}: {source}"
            ),
            Self::PaintAnnotation { layer_index, source } => write!(
                f,
                "built-in paint-annotation failed at layer {layer_index}: {source:?}"
            ),
        }
    }
}

impl std::error::Error for LayerExecutionError {}

// LayerStageRunner trait is now defined in slicer-wasm-host::traits and re-exported
// from slicer_runtime via the transitional re-exports block in lib.rs (P83 Step 4c+4d).
// The trait signature changed to take CompiledModuleLive<'_> + LayerStageInput<'_> and
// return LayerStageCommitData — the executor call site builds these from CompiledModule +
// Blackboard + LayerArena before invoking the runner.

/// Executes the Tier-2 per-layer parallel pipeline using rayon.
///
/// Layers are processed in parallel, but stages within each layer are sequential.
/// Each layer gets its own `LayerArena` that is freed when the layer completes.
/// Results are committed to the blackboard's write-once layer output slots.
pub fn execute_per_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<Vec<LayerCollectionIR>, LayerExecutionError> {
    let (layers, _audits) = execute_per_layer_with_events(
        plan,
        blackboard,
        runner,
        &NoopLayerProgressSink,
        wasm_handles,
    )?;
    Ok(layers)
}

/// Like [`execute_per_layer`] but additionally routes per-layer progress
/// events (including host-built-in paint-annotation fallback warnings)
/// to `sink`.
///
/// Returns both the collected layer IRs and the runtime access audits from
/// all per-layer module executions (TASK-123b).
pub fn execute_per_layer_with_events(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    sink: &(dyn LayerProgressSink + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(Vec<LayerCollectionIR>, Vec<ModuleAccessAudit>), LayerExecutionError> {
    execute_per_layer_with_instrumentation(
        plan,
        blackboard,
        runner,
        sink,
        &NoopInstrumentation,
        wasm_handles,
    )
}

/// Like [`execute_per_layer_with_events`] but additionally records timing,
/// memory, and DAG bracket calls into `instrumentation`. Pass
/// `&NoopInstrumentation` for zero-overhead behavior identical to the
/// non-instrumented variant.
pub fn execute_per_layer_with_instrumentation(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(Vec<LayerCollectionIR>, Vec<ModuleAccessAudit>), LayerExecutionError> {
    let global_layers = &plan.global_layers;
    let required_semantics = blackboard
        .paint_regions()
        .map(|pr| collect_required_semantics(pr))
        .unwrap_or_default();

    use rayon::iter::ParallelIterator;
    let results: Result<Vec<(LayerCollectionIR, Vec<ModuleAccessAudit>)>, LayerExecutionError> =
        global_layers
            .par_iter()
            .map(|layer| {
                execute_single_layer(
                    plan,
                    blackboard,
                    runner,
                    sink,
                    instrumentation,
                    &required_semantics,
                    layer,
                    wasm_handles,
                )
            })
            .collect();

    match results {
        Ok(layer_results) => {
            let mut layer_irs = Vec::with_capacity(layer_results.len());
            let mut all_audits = Vec::new();
            for (layer_ir, audits) in layer_results {
                all_audits.extend(audits);
                layer_irs.push(layer_ir);
            }
            Ok((layer_irs, all_audits))
        }
        Err(e) => Err(e),
    }
}

/// Deterministically collect the union of all paint semantics present in
/// `paint_regions` across all layers, ordered: Material, FuzzySkin,
/// SupportEnforcer, SupportBlocker, then `Custom` entries sorted by name.
fn collect_required_semantics(paint_regions: &PaintRegionIR) -> Vec<PaintSemantic> {
    let mut out: Vec<PaintSemantic> = Vec::new();
    for layer_map in paint_regions.per_layer.values() {
        for sem in layer_map.semantic_regions.keys() {
            if !out.contains(sem) {
                out.push(sem.clone());
            }
        }
    }
    out.sort_by_key(semantic_sort_key);
    out
}

fn semantic_sort_key(s: &PaintSemantic) -> (u8, String) {
    match s {
        PaintSemantic::Material => (0, String::new()),
        PaintSemantic::FuzzySkin => (1, String::new()),
        PaintSemantic::SupportEnforcer => (2, String::new()),
        PaintSemantic::SupportBlocker => (3, String::new()),
        PaintSemantic::Custom(n) => (4, n.clone()),
    }
}

/// Execute all stages for a single layer sequentially, collecting runtime
/// access audits for each user module that produces output.
///
/// Returns both the finalized `LayerCollectionIR` and the `ModuleAccessAudit`
/// entries for all modules that committed output during this layer's execution.
fn execute_single_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    required_semantics: &[PaintSemantic],
    layer: &GlobalLayer,
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(LayerCollectionIR, Vec<ModuleAccessAudit>), LayerExecutionError> {
    instrumentation.on_layer_start(layer.index, layer.z);
    let result = execute_single_layer_inner(
        plan,
        blackboard,
        runner,
        sink,
        instrumentation,
        required_semantics,
        layer,
        wasm_handles,
    );
    instrumentation.on_layer_end(layer.index);
    result
}

fn execute_single_layer_inner(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    required_semantics: &[PaintSemantic],
    layer: &GlobalLayer,
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(LayerCollectionIR, Vec<ModuleAccessAudit>), LayerExecutionError> {
    let mut audits = Vec::new();

    // Create an isolated LayerArena for this layer
    let mut arena = LayerArena::new();

    // Hydrate the per-layer arena's SliceIR slot from the prepass-committed
    // `Vec<SliceIR>` on the blackboard. Production paths run PrePass::Slice
    // first (via `commit_slice_builtin`), which commits the Vec<SliceIR>.
    // Test fixtures that bypass the prepass executor must call
    // `common::seed::seed_slice_ir` before `execute_per_layer`; a missing
    // commit surfaces as a hard `FatalLayer` error (spec §Commit 2).
    if arena.slice().is_none() {
        let slice_vec = blackboard
            .slice_ir()
            .ok_or_else(|| LayerExecutionError::FatalLayer {
                layer_index: layer.index,
                stage_id: "PrePass::Slice".to_string(),
                module_id: "host:slice".to_string(),
                message: "blackboard slice_ir empty when Tier 2 started".to_string(),
            })?;
        let slice = slice_vec
            .get(layer.index as usize)
            .cloned()
            .ok_or_else(|| LayerExecutionError::FatalLayer {
                layer_index: layer.index,
                stage_id: "PrePass::Slice".to_string(),
                module_id: "host:slice".to_string(),
                message: format!("slice_ir Vec missing entry for layer index {}", layer.index),
            })?;
        arena
            .set_slice(slice)
            .map_err(|_| LayerExecutionError::FatalLayer {
                layer_index: layer.index,
                stage_id: "PrePass::Slice".to_string(),
                module_id: "host:slice".to_string(),
                message: "slice arena slot already occupied".to_string(),
            })?;
    }

    // Execute stages sequentially in deterministic order.
    // Immediately before `Layer::PathOptimization` runs, freeze the assembled
    // `LayerCollectionIR.ordered_entities` into the arena so the path-
    // optimization commit path (and any downstream per-layer stage) can see
    // the same entity sequence that the host emitter will consume.
    for stage in &plan.per_layer_stages {
        if stage.stage_id == "Layer::PathOptimization" && arena.layer_collection().is_none() {
            let ordered_entities = assemble_ordered_entities(
                layer.index,
                arena.perimeter(),
                arena.infill(),
                arena.support(),
                blackboard.region_map().map(|arc| arc.as_ref()),
            );
            arena.set_layer_collection(LayerCollectionIR {
                global_layer_index: layer.index,
                z: layer.z,
                ordered_entities,
                ..Default::default()
            });
        }
        instrumentation.on_stage_start(&stage.stage_id, Some(layer.index));
        // Execute modules in topological order within each stage
        for module in &stage.modules {
            instrumentation.on_module_start(&stage.stage_id, Some(layer.index), module.module_id());

            // Build the IR-typed borrow structs for the new slicer-wasm-host trait boundary.
            // CompiledModuleLive borrows from CompiledModule for the duration of this call.
            let (instance_pool, wasm_component) = wasm_handles
                .get(module.module_id().as_str())
                .map(|(p, c)| (Arc::clone(p), c.clone()))
                .unwrap_or_else(|| (WasmInstancePool::placeholder(), None));
            let live_module = CompiledModuleLive::new(
                module.module_id(),
                instance_pool,
                wasm_component,
                module.claims(),
                Arc::clone(module.config_view()),
            );
            let input = LayerStageInput {
                mesh: Arc::clone(blackboard.mesh()),
                paint_regions: blackboard.paint_regions().cloned(),
                seam_plan: blackboard.seam_plan().cloned(),
                support_plan: blackboard.support_plan().cloned(),
                region_map: blackboard.region_map().cloned(),
                slice: arena.slice(),
                perimeter: arena.perimeter(),
                layer_collection: arena.layer_collection(),
            };
            // Capture seam_plan for commit_layer_outputs (Layer::PerimetersPostProcess path).
            let seam_plan_ir_for_commit = blackboard.seam_plan().map(|arc| arc.as_ref());

            let run_result = runner.run_stage(&stage.stage_id, layer, &live_module, input);
            // Pull the wasm linear-memory sample for the just-completed call.
            // Returns (0, 0) for non-wasm runners (test mocks, host built-ins).
            let (wasm_before, wasm_after) = runner.last_wasm_mem_sample();
            instrumentation.on_module_end(
                &stage.stage_id,
                Some(layer.index),
                module.module_id(),
                wasm_before,
                wasm_after,
            );
            let commit = match run_result {
                Ok(commit_data) => commit_data,
                Err(e) => {
                    instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id().to_owned(),
                        message: e.to_string(),
                    });
                }
            };

            // Extract cross-boundary flags before commit consumes the struct.
            let entity_order_proposal = commit.entity_order_proposal.clone();
            let needs_seam_injection = commit.needs_seam_injection;

            // Layer::PathOptimization may have emitted a `set-entity-order` proposal
            // via `layer-collection-builder`. Apply it against the staged
            // `LayerCollectionIR.ordered_entities` BEFORE commit_layer_outputs runs,
            // matching the original dispatch.rs::run_stage ordering.
            if stage.stage_id == "Layer::PathOptimization" {
                if let Some(ref proposal) = entity_order_proposal {
                    if let Err(message) = apply_entity_order_proposal(&mut arena, proposal) {
                        instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                        return Err(LayerExecutionError::FatalLayer {
                            layer_index: layer.index,
                            stage_id: stage.stage_id.clone(),
                            module_id: module.module_id().to_owned(),
                            message,
                        });
                    }
                }
            }

            // Commit the IR-typed output data to the arena.
            if let Err(e) = commit_layer_outputs(
                &stage.stage_id,
                module.module_id(),
                layer.index,
                commit,
                &mut arena,
                seam_plan_ir_for_commit,
            ) {
                instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                return Err(LayerExecutionError::FatalLayer {
                    layer_index: layer.index,
                    stage_id: stage.stage_id.clone(),
                    module_id: module.module_id().to_owned(),
                    message: e.to_string(),
                });
            }

            // For Layer::Perimeters: inject seam from SeamPlanIR into arena.perimeter()
            // so PerimetersPostProcess can merge it into the guest output.
            // The seam was sent to the WASM store via PerimeterRegionData but is NOT
            // baked into the PerimeterIR the guest emits, so we inject it here
            // post-commit. Mirrors the post-commit seam injection in the original
            // dispatch.rs::LayerStageRunner::run_stage body.
            if needs_seam_injection {
                if let Some(seam_ir) = seam_plan_ir_for_commit {
                    if let Some(mut perimeter) = arena.take_perimeter() {
                        for region in &mut perimeter.regions {
                            if region.resolved_seam.is_none() {
                                if let Some(entry) = seam_ir.entries.iter().find(|e| {
                                    e.region_key.global_layer_index == layer.index
                                        && e.region_key.object_id == region.object_id
                                        && e.region_key.region_id == region.region_id
                                }) {
                                    region.resolved_seam = Some(slicer_ir::SeamPosition {
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

            let writes = ir_path_for_layer_stage(&stage.stage_id)
                .map(|p| vec![p])
                .unwrap_or_default();
            let runtime_reads = runner.last_runtime_reads();
            if !writes.is_empty() || !runtime_reads.is_empty() {
                audits.push(ModuleAccessAudit {
                    module_id: module.module_id().to_owned(),
                    runtime_reads,
                    runtime_writes: writes,
                });
            }
        }

        // Host-built-in paint-annotation runs at the `Layer::PaintRegionAnnotation`
        // stage (docs/04 §Full Lifecycle and docs/10 §Paint Region Resolution).
        // If a WASM module is registered for this stage, it handles the annotation
        // and the host handler is skipped.
        if stage.stage_id == "Layer::PaintRegionAnnotation" && stage.modules.is_empty() {
            let pa_module_id = "host:paint_annotator".to_string();
            instrumentation.on_module_start(&stage.stage_id, Some(layer.index), &pa_module_id);
            run_paint_annotation(
                blackboard,
                required_semantics,
                sink,
                &mut arena,
                layer,
                &stage.stage_id,
            )?;
            instrumentation.on_module_end(&stage.stage_id, Some(layer.index), &pa_module_id, 0, 0);
        }
        instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
    }

    // Safety-net: if no `Layer::PaintRegionAnnotation` stage was in the
    // execution plan (e.g. tests that construct plans by hand), run the host
    // annotator. Production plans always include this stage via
    // `build_execution_plan`, so this path is only exercised when the plan
    // is built outside the normal pipeline.
    let paint_annotation_ran = plan
        .per_layer_stages
        .iter()
        .any(|s| s.stage_id == "Layer::PaintRegionAnnotation");
    if !paint_annotation_ran {
        run_paint_annotation(
            blackboard,
            required_semantics,
            sink,
            &mut arena,
            layer,
            "Layer::PaintRegionAnnotation",
        )?;
    }

    // If `Layer::PathOptimization` pre-staged a LayerCollectionIR, take it and
    // append any guest-emitted tool changes accumulated during that stage.
    // Otherwise fall back to direct assembly from arena slots (stages without
    // a PathOptimization module, or tests that omit it).
    let mut layer_output = arena.take_layer_collection().unwrap_or_else(|| {
        let ordered_entities = assemble_ordered_entities(
            layer.index,
            arena.perimeter(),
            arena.infill(),
            arena.support(),
            blackboard.region_map().map(|arc| arc.as_ref()),
        );
        LayerCollectionIR {
            global_layer_index: layer.index,
            z: layer.z,
            ordered_entities,
            ..Default::default()
        }
    });
    layer_output
        .tool_changes
        .extend(arena.take_deferred_tool_changes());
    layer_output
        .annotations
        .extend(arena.take_deferred_annotations());
    layer_output.z_hops.extend(arena.take_deferred_z_hops());
    layer_output
        .retracts
        .extend(
            arena
                .take_deferred_retracts()
                .into_iter()
                .map(|r| slicer_ir::TravelRetract {
                    after_entity_index: r.after_entity_index,
                    length: r.length,
                    speed: r.speed,
                    is_unretract: r.is_unretract,
                    mode: r.mode,
                }),
        );
    {
        let raw_travels = arena.take_deferred_travel_moves();
        let mapped: Vec<slicer_ir::TravelMove> = raw_travels
            .into_iter()
            .map(|m| slicer_ir::TravelMove {
                entity_id: layer_output
                    .ordered_entities
                    .get(m.after_entity_index as usize)
                    .map(|e| e.entity_id)
                    .unwrap_or(0),
                x: m.x,
                y: m.y,
                z: m.z,
                f: m.f,
            })
            .collect();
        layer_output.travel_moves.extend(mapped);
    }
    Ok((layer_output, audits))
}

/// Maps a per-layer stage ID to the canonical IR field path it writes.
///
/// This mirrors the `ir_path_for_prepass_output` pattern: each stage
/// produces exactly one primary IR type, and this mapping records that
/// write in the runtime audit. `Layer::Slice` is a host-built-in and
/// is excluded (no audit). `Layer::SlicePostProcess` only merges into the
/// existing `SliceIR` without creating a primary IR object; it is also
/// excluded.
/// Maps a layer stage ID to the coarse IR write path used as fallback
/// when the stage has no narrow runtime_writes instrumentation.
///
/// This mapping is used in `execute_single_layer` when `runtime_writes` is
/// empty (non-instrumented stage). Each stage in the per-layer pipeline
/// produces exactly one primary IR type, and this mapping records that
/// write in the runtime audit. `Layer::Slice` is a host-built-in and
/// is excluded (no audit). `Layer::SlicePostProcess` only merges into the
/// existing `SliceIR` without creating a primary IR object; it is also
/// excluded.
pub fn ir_path_for_layer_stage(stage_id: &StageId) -> Option<String> {
    match stage_id.as_str() {
        "Layer::Slice" => None,            // host-built-in, not audited
        "Layer::SlicePostProcess" => None, // merges into existing SliceIR, not a primary commit
        "Layer::Perimeters" | "Layer::PerimetersPostProcess" => Some(String::from("PerimeterIR")),
        "Layer::Infill" | "Layer::InfillPostProcess" => Some(String::from("InfillIR")),
        "Layer::Support" | "Layer::SupportPostProcess" => Some(String::from("SupportIR")),
        "Layer::PathOptimization" => Some(String::from("LayerCollectionIR")),
        _ => None,
    }
}

/// Run the host-built-in paint-annotation step on the layer's staged
/// `SliceIR`. Returns early with no work if the blackboard has no paint
/// regions committed or if no required semantics were derived. Warnings are
/// converted through `paint_annotation_warning_to_progress_event` and pushed
/// to `sink`. Fatal annotation errors become `LayerExecutionError::PaintAnnotation`.
fn run_paint_annotation(
    blackboard: &Blackboard,
    required_semantics: &[PaintSemantic],
    sink: &(dyn LayerProgressSink + Sync),
    arena: &mut LayerArena,
    layer: &GlobalLayer,
    event_stage: &str,
) -> Result<(), LayerExecutionError> {
    if required_semantics.is_empty() {
        return Ok(());
    }
    let paint_regions = match blackboard.paint_regions() {
        Some(pr) => std::sync::Arc::clone(pr),
        None => return Ok(()),
    };
    let mut slice_ir = match arena.take_slice() {
        Some(s) => s,
        None => return Ok(()),
    };

    // Apply negative-part subtract before paint annotation sees the polygons (Packet 56c).
    for obj in blackboard.mesh().objects.iter() {
        crate::negative_part_subtract::apply_negative_part_subtract(
            &mut slice_ir,
            &obj.modifier_volumes,
        );
    }

    // Compute per-layer modifier projections for fuzzy-skin annotation (packet 56b).
    // For each modifier_part volume, slice its world-space mesh at the current
    // layer Z to get the intersecting ExPolygon set.
    let modifier_projections: Vec<ExPolygon> = {
        let mesh = blackboard.mesh();
        let mut projections = Vec::new();
        for obj in &mesh.objects {
            for mv in &obj.modifier_volumes {
                let is_modifier_part = mv.config_delta.fields.get("subtype").map_or(false, |v| {
                    v == &slicer_ir::ConfigValue::String("modifier_part".to_string())
                });
                if !is_modifier_part || mv.mesh.vertices.is_empty() {
                    continue;
                }
                // slice_mesh_ex returns one Vec<ExPolygon> per Z; we only need layer.z
                let slices = slice_mesh_ex(&mv.mesh, &[layer.z]);
                if let Some(layer_slice) = slices.into_iter().next() {
                    projections.extend(layer_slice);
                }
            }
        }
        projections
    };

    let paint_region_rtree = blackboard.paint_region_rtree().cloned();
    let request = SlicePostProcessPaintAnnotationRequest {
        slice_ir,
        paint_regions,
        required_semantics: required_semantics.to_vec(),
        modifier_projections,
        paint_region_rtree,
    };
    let result = execute_slice_postprocess_paint_annotation(request).map_err(|source| {
        LayerExecutionError::PaintAnnotation {
            layer_index: layer.index,
            source,
        }
    })?;

    // Surface deterministic, non-fatal fallback warnings through the
    // existing progress-event adapter (docs/09 §ModuleError; docs/11 §73-75).
    // Per-point warnings are coalesced into one event per
    // (object, region, semantic, polygon) group so structurally-noisy paint
    // regions don't drown the log in identical lines.
    let mut events = paint_annotation_warnings_to_progress_events(
        &result.warnings,
        String::new(),
        String::from("com.host.slice-postprocess-paint-annotator"),
        0,
    );
    for event in &mut events {
        event.stage = Some(event_stage.to_string());
    }
    for event in events {
        sink.record(event);
    }

    // Put the (possibly annotated) SliceIR back so downstream per-layer
    // stages can still read it via `arena.slice()`.
    arena
        .set_slice(result.slice_ir)
        .map_err(|_| LayerExecutionError::FatalLayer {
            layer_index: layer.index,
            stage_id: "Layer::SlicePostProcess".to_string(),
            module_id: "host:paint_annotator".to_string(),
            message: "slice arena slot unexpectedly occupied after take_slice".to_string(),
        })?;
    Ok(())
}

/// Thin identity-preserving drain from committed arena IR into `PrintEntity`s.
///
/// Ordering is deterministic and documented: for each `PerimeterRegion` in
/// committed order, emit one `PrintEntity` per wall loop (ordered by the
/// region's own `walls` slice, whose order is guest-preserved); then for each
/// `InfillRegion` in committed order, emit sparse / solid / ironing paths in
/// that order; finally emit `SupportIR` paths (support / interface / raft /
/// ironing). `region_key` carries `(global_layer_index, object_id, region_id)`
/// for perimeter and infill entities. `SupportIR` is flat in the current IR
/// model and has no per-region identity, so support entities use an empty
/// `object_id` and `region_id = 0` rather than inventing synthetic identity.
/// `topo_order` is the entity's 0-based position in the emitted sequence.
fn dominant_tool_index(flags: &[WallFeatureFlags]) -> Option<u64> {
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for f in flags {
        if let Some(ti) = f.tool_index {
            *counts.entry(ti as u64).or_default() += 1;
        }
    }
    counts.iter().max_by_key(|(_, c)| **c).map(|(ti, _)| *ti)
}

pub(crate) fn assemble_ordered_entities(
    global_layer_index: u32,
    perimeter: Option<&PerimeterIR>,
    infill: Option<&InfillIR>,
    support: Option<&SupportIR>,
    region_map: Option<&RegionMapIR>,
) -> Vec<PrintEntity> {
    let mut out: Vec<PrintEntity> = Vec::new();
    let id_gen = LayerEntityIdGen::new();
    let push = |path: slicer_ir::ExtrusionPath3D,
                role: slicer_ir::ExtrusionRole,
                key: RegionKey,
                acc: &mut Vec<PrintEntity>| {
        let topo_order = acc.len() as u32;
        acc.push(PrintEntity {
            entity_id: id_gen.next(),
            path,
            role,
            region_key: key,
            topo_order,
        });
    };

    if let Some(perim) = perimeter {
        for region in &perim.regions {
            // Pre-compute the per-region config-extensions "extruder" fallback
            // (packet 68 / AC-2): when no paint-derived tool exists for a wall,
            // a modifier-volume config delta stamped into
            // `RegionPlan.config.extensions["extruder"]` selects the tool.
            // Paint-derived tools (`dominant_tool_index`) still win.
            let base_key = RegionKey {
                global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
                variant_chain: Vec::new(),
            };
            let modifier_tool: Option<u64> = region_map.and_then(|rm| {
                if rm.entries.contains_key(&base_key) {
                    rm.config_for(&base_key)
                        .extensions
                        .get("extruder")
                        .and_then(|v| match v {
                            ConfigValue::Int(n) if *n >= 0 => Some(*n as u64),
                            _ => None,
                        })
                } else {
                    None
                }
            });
            for wl in &region.walls {
                let paint_tool = dominant_tool_index(&wl.feature_flags);
                let resolved_tool = paint_tool.or(modifier_tool).unwrap_or(region.region_id);
                let entity_key = RegionKey {
                    global_layer_index,
                    object_id: region.object_id.clone(),
                    region_id: resolved_tool,
                    variant_chain: Vec::new(),
                };
                let role = wl.path.role.clone();
                push(wl.path.clone(), role, entity_key, &mut out);
            }
        }
    }

    if let Some(inf) = infill {
        for region in &inf.regions {
            let key = RegionKey {
                global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
                variant_chain: Vec::new(),
            };
            for path in &region.sparse_infill {
                push(path.clone(), path.role.clone(), key.clone(), &mut out);
            }
            for path in &region.solid_infill {
                push(path.clone(), path.role.clone(), key.clone(), &mut out);
            }
            for path in &region.ironing {
                push(path.clone(), path.role.clone(), key.clone(), &mut out);
            }
        }
    }

    if let Some(sup) = support {
        // SupportIR is flat in the current schema — no per-region identity
        // available. Emit with an empty object_id and region_id=0 rather than
        // inventing synthetic structure.
        let key = RegionKey {
            global_layer_index,
            object_id: String::new(),
            region_id: 0,
            variant_chain: Vec::new(),
        };
        for path in &sup.support_paths {
            push(path.clone(), path.role.clone(), key.clone(), &mut out);
        }
        for path in &sup.interface_paths {
            push(path.clone(), path.role.clone(), key.clone(), &mut out);
        }
        for path in &sup.raft_paths {
            push(path.clone(), path.role.clone(), key.clone(), &mut out);
        }
        for path in &sup.ironing_paths {
            push(path.clone(), path.role.clone(), key.clone(), &mut out);
        }
    }

    out
}

// ── 7 RUNTIME items moved from dispatch.rs (P83 Step 4c+4d) ─────────────────
//
// These items lived in `slicer-runtime/src/dispatch.rs` in the pre-P83 split.
// They depend only on `LayerArena` and `slicer-ir` types, so they move here
// rather than into `slicer-wasm-host` (which must not depend on slicer-runtime).
// `commit_layer_outputs` signature changed: `ctx: &HostExecutionContext` →
// `commit: LayerStageCommitData` (P83 Step 4d — symmetric IR-typed boundary).

/// Test-only wrapper around the private `commit_layer_outputs` so integration
/// tests can exercise the PathOptimization GCode-override rejection path
/// without compiling a bespoke WAT guest per case.
#[doc(hidden)]
pub fn commit_layer_outputs_for_test(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    commit: LayerStageCommitData,
    arena: &mut LayerArena,
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
) -> Result<(), slicer_ir::LayerStageError> {
    commit_layer_outputs(
        stage_id,
        module_id,
        layer_index,
        commit,
        arena,
        seam_plan_ir,
    )
}

/// Host-local projection of a single staged
/// `LayerCollectionIR.ordered_entities[i]` entry, mirroring the WIT
/// `ordered-entity-view` record. Built once per `Layer::PathOptimization`
/// invocation by [`project_ordered_entities`] and stashed on
/// `LayerCollectionBuilderData` so the host-side
/// `HostLayerCollectionBuilder::get_ordered_entities` impl can serve
/// repeated reads from a snapshot rather than the live arena.
#[derive(Debug, Clone)]
pub struct OrderedEntityView {
    /// Index into the host-staged `LayerCollectionIR.ordered_entities`
    /// at the time this snapshot was projected.
    pub original_index: u32,
    /// Region key of the entity at `original_index`.
    pub region_key: RegionKey,
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
/// When no `LayerCollectionIR` is staged on the arena, returns
/// an empty `Vec` (no error).
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
    // topo_order reassignment.
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

/// Commit IR-typed layer stage output data into the per-layer arena.
///
/// Called after `LayerStageRunner::run_stage` returns a `LayerStageCommitData`.
/// The wasm-host's `deconstruct_layer_ctx` already converted the WIT output types
/// into IR types before this point, so this function only performs arena writes
/// and validation — no WIT type conversion happens here.
///
/// `seam_plan_ir` is only consulted by the `Layer::PerimetersPostProcess` path to
/// back-fill `resolved_seam` on regions that weren't resolved by the guest.
fn commit_layer_outputs(
    stage_id: &str,
    module_id: &str,
    layer_index: u32,
    commit: LayerStageCommitData,
    arena: &mut LayerArena,
    seam_plan_ir: Option<&slicer_ir::SeamPlanIR>,
) -> Result<(), slicer_ir::LayerStageError> {
    let mk_validation_err = |what: &str, reason: String| slicer_ir::LayerStageError::FatalModule {
        stage_id: stage_id.to_string(),
        module_id: module_id.to_string(),
        message: format!("invalid {what} output: {reason}"),
    };

    // Test-escape-hatch: if a mock runner injects a pre-built LayerCollectionIR, commit it
    // to the arena so the next stage (e.g. Layer::PathOptimization) sees it in input.layer_collection.
    if let Some(lc) = commit.layer_collection_output {
        let _ = arena.take_layer_collection(); // clear any auto-assembled one
        arena.set_layer_collection(lc);
    }

    match stage_id {
        "Layer::Infill" | "Layer::InfillPostProcess" => {
            let Some(ir) = commit.infill_output else {
                return Ok(());
            };
            if stage_id == "Layer::InfillPostProcess" {
                let _ = arena.take_infill();
            }
            if let Some(mut existing) = arena.take_infill() {
                merge_infill_ir(&mut existing, ir);
                arena
                    .set_infill(existing)
                    .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
            } else {
                arena
                    .set_infill(ir)
                    .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
            }
        }
        "Layer::Support" | "Layer::SupportPostProcess" => {
            let Some(ir) = commit.support_output else {
                return Ok(());
            };
            if stage_id == "Layer::SupportPostProcess" {
                let _ = arena.take_support();
            }
            arena
                .set_support(ir)
                .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
        }
        "Layer::Perimeters" | "Layer::PerimetersPostProcess" => {
            if stage_id == "Layer::PerimetersPostProcess" {
                let mut original = arena.take_perimeter();
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
                match (commit.perimeter_output, original) {
                    (Some(mut ir_owned), Some(orig_perim)) => {
                        for (idx, region) in ir_owned.regions.iter_mut().enumerate() {
                            if region.resolved_seam.is_none() {
                                if let Some(orig_region) = orig_perim.regions.get(idx) {
                                    if let Some(rs) = &orig_region.resolved_seam {
                                        region.resolved_seam = Some(rs.clone());
                                    }
                                }
                            }
                        }
                        arena
                            .set_perimeter(ir_owned)
                            .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
                    }
                    (Some(ir_owned), None) => {
                        arena
                            .set_perimeter(ir_owned)
                            .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
                    }
                    (None, Some(orig_perim)) => {
                        arena
                            .set_perimeter(orig_perim)
                            .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
                    }
                    (None, None) => {}
                }
            } else {
                let Some(ir) = commit.perimeter_output else {
                    return Ok(());
                };
                let _ = arena.take_perimeter();
                arena
                    .set_perimeter(ir)
                    .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
            }
        }
        "Layer::SlicePostProcess" => {
            if commit.slice_polygon_updates.is_empty() && commit.slice_path_z_updates.is_empty() {
                return Ok(());
            }
            let mut existing =
                arena
                    .take_slice()
                    .ok_or_else(|| slicer_ir::LayerStageError::FatalModule {
                        stage_id: stage_id.to_string(),
                        module_id: module_id.to_string(),
                        message: "Layer::SlicePostProcess has no staged SliceIR to merge into; \
                          Layer::Slice must commit per-region slice output first"
                            .into(),
                    })?;

            // Apply polygon updates: replace region.polygons by (object_id, region_id) match.
            for (i, (key, polys)) in commit.slice_polygon_updates.iter().enumerate() {
                let ridx = existing
                    .regions
                    .iter()
                    .position(|r| r.object_id == key.object_id && r.region_id == key.region_id)
                    .ok_or_else(|| {
                        mk_validation_err(
                            "slice postprocess",
                            format!(
                                "polygon_update[{i}] targets unknown region \
                             (object_id='{}', region_id='{}')",
                                key.object_id, key.region_id,
                            ),
                        )
                    })?;
                existing.regions[ridx].polygons = polys.clone();
            }

            // Apply path-Z updates: validate bounds (the actual Z mutation is a no-op
            // because slicer_ir::ExPolygon has no per-point Z field — same as the WIT path).
            for (i, (key, path_idx, vertex_idx, _z)) in
                commit.slice_path_z_updates.iter().enumerate()
            {
                let ridx = existing
                    .regions
                    .iter()
                    .position(|r| r.object_id == key.object_id && r.region_id == key.region_id)
                    .ok_or_else(|| {
                        mk_validation_err(
                            "slice postprocess",
                            format!(
                                "path_z_update[{i}] targets unknown region \
                             (object_id='{}', region_id='{}')",
                                key.object_id, key.region_id,
                            ),
                        )
                    })?;
                let region = &existing.regions[ridx];
                let poly_count = region.polygons.len();
                let poly = region.polygons.get(*path_idx as usize).ok_or_else(|| {
                    mk_validation_err(
                        "slice postprocess",
                        format!(
                            "path_z_update[{i}]: polygon index {path_idx} out of range \
                             for region ({}, {}) with {poly_count} polygons",
                            key.object_id, key.region_id,
                        ),
                    )
                })?;
                if (*vertex_idx as usize) >= poly.contour.points.len() {
                    return Err(mk_validation_err(
                        "slice postprocess",
                        format!(
                            "path_z_update[{i}]: vertex index {vertex_idx} out of range \
                             for contour with {} points",
                            poly.contour.points.len(),
                        ),
                    ));
                }
            }

            arena
                .set_slice(existing)
                .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
        }
        "Layer::PathOptimization" => {
            let anchor = arena
                .layer_collection()
                .map(|lc| lc.ordered_entities.len().saturating_sub(1) as u32)
                .unwrap_or(0);

            // tool_changes, z_hops, annotations, retracts, deferred_travel_moves are
            // pre-categorised by the wasm-host's deconstruct_layer_ctx.
            for tc in commit.tool_changes {
                arena.push_deferred_tool_change(tc);
            }
            for mut zh in commit.z_hops {
                // Validate hop_height before accepting (matches old per-command validation).
                if !zh.hop_height.is_finite() || zh.hop_height <= 0.0 {
                    return Err(slicer_ir::LayerStageError::FatalModule {
                        stage_id: stage_id.to_string(),
                        module_id: module_id.to_string(),
                        message: format!(
                            "Layer::PathOptimization push-z-hop rejected: \
                             hop-height={} is not finite and strictly positive",
                            zh.hop_height
                        ),
                    });
                }
                // Override placeholder anchor (0 from deconstruct) with real anchor,
                // matching original dispatch.rs behavior where z_hops were created
                // with `after_entity_index: anchor`.
                zh.after_entity_index = anchor;
                arena.push_deferred_z_hop(zh);
            }
            for mut ann in commit.annotations {
                // Override the placeholder anchor (0) set by deconstruct_layer_ctx
                // with the real entity-count-based anchor, matching the original
                // dispatch.rs::commit_layer_outputs behavior.
                ann.after_entity_index = anchor;
                arena.push_deferred_annotation(ann);
            }
            for r in commit.retracts {
                arena.push_deferred_retract(crate::blackboard::DeferredRetract {
                    // Override placeholder anchor to match original behavior.
                    after_entity_index: anchor,
                    length: r.length,
                    speed: r.speed,
                    is_unretract: r.is_unretract,
                    mode: r.mode,
                });
            }
            for (_after_idx, x, y, z, f) in commit.deferred_travel_moves {
                arena.push_deferred_travel_move(crate::blackboard::DeferredTravelMove {
                    // Placeholder anchor (0) from deconstruct is overridden here with
                    // the real entity-count-based anchor, matching original dispatch.rs behavior.
                    after_entity_index: anchor,
                    x,
                    y,
                    z,
                    f,
                });
            }
        }
        _ => {}
    }
    Ok(())
}

/// Merge `incoming`'s regions into `existing` in place.
///
/// `Layer::Infill` runs multiple modules per layer that each write
/// disjoint fields of `InfillRegion` (rectilinear/gyroid populate
/// `sparse_infill` / `solid_infill`; top-surface-ironing populates
/// `ironing`). When the layer-arena `infill` slot is already occupied,
/// the dispatch layer calls this helper to append per-region paths
/// instead of failing with `SlotAlreadyOccupied`.
///
/// Regions match on `(object_id, region_id)`. A region present in
/// `incoming` but not `existing` is pushed as-is. The schema_version /
/// global_layer_index of `existing` win on conflict.
fn merge_infill_ir(existing: &mut InfillIR, incoming: InfillIR) {
    for new_region in incoming.regions {
        match existing
            .regions
            .iter_mut()
            .find(|r| r.object_id == new_region.object_id && r.region_id == new_region.region_id)
        {
            Some(target) => {
                target.sparse_infill.extend(new_region.sparse_infill);
                target.solid_infill.extend(new_region.solid_infill);
                target.ironing.extend(new_region.ironing);
            }
            None => existing.regions.push(new_region),
        }
    }
}

#[cfg(test)]
mod tests {
    use slicer_ir::LayerStageOutput;

    #[test]
    fn test_layer_stage_output_equality() {
        assert_eq!(LayerStageOutput::Success, LayerStageOutput::Success);
        assert_eq!(
            LayerStageOutput::NonFatalError {
                message: "test".into()
            },
            LayerStageOutput::NonFatalError {
                message: "test".into()
            }
        );
    }
}
