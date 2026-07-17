//! Per-layer parallel executor contracts (TASK-031).
//!
//! This module defines the per-layer parallel execution contracts for running
//! all Tier-2 layer stages using rayon. Each layer gets its own `LayerArena`
//! for intermediate IR storage. Stages within each layer run sequentially,
//! but layers can be processed in parallel.

use std::fmt;
use std::sync::Arc;

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use slicer_ir::{
    ConfigValue, GCodeIR, GlobalLayer, InfillIR, LayerCollectionIR, LayerEntityIdGen,
    LayerStageCommit, ModuleId, PaintSemantic, PerimeterIR, PrintEntity, RegionKey, RegionMapIR,
    SeamPlanIR, SliceIR, StageId, SupportGeometryIR, SupportIR, SupportPlanIR,
    SurfaceClassificationIR, WallFeatureFlags,
};
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageInput, LayerStageRunner, WasmComponent, WasmInstancePool,
};

use crate::instrumentation::{NoopInstrumentation, PipelineInstrumentation};
use crate::progress_events::ProgressEvent;
use crate::slice_postprocess::SlicePostProcessPaintAnnotationError;
use crate::{
    Blackboard, BlackboardError, CompiledStage, ExecutionPlan, LayerArena, ModuleAccessAudit,
    STAGE_ORDER,
};
use slicer_core::algos::prepass_slice::LayerSliceError;

/// Base extruder; tool-resolution fallback so a region IDENTITY can never
/// enter the tool slot. When all four resolvers (paint_tool, spatial_tool,
/// variant_tool, modifier_tool) return `None`, we emit tool 0 (T0) rather
/// than the synthesized paint-variant IDENTITY stored in `region.region_id`.
/// Leaking the identity caused a 9.9 GiB OOM: `region_id as u32` produced
/// 2_664_076_552, driving `vec![0.0f32; max_tool+1]` in `emit.rs`.
const DEFAULT_TOOL: u64 = 0;

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
// It takes CompiledModuleLive<'_> + LayerStageInput<'_> and returns
// Option<LayerStageCommit> (ADR-0020); the executor's `apply` performs the arena writes.

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
                    &[],
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
    hydrate_slice_arena(&mut arena, blackboard, layer)?;

    // Execute stages sequentially in deterministic order.
    // Immediately before `Layer::PathOptimization` runs, freeze the assembled
    // `LayerCollectionIR.ordered_entities` into the arena so the path-
    // optimization commit path (and any downstream per-layer stage) can see
    // the same entity sequence that the host emitter will consume.
    for stage in &plan.per_layer_stages {
        prestage_layer_collection_if_path_optimization(&mut arena, stage, layer, blackboard);
        instrumentation.on_stage_start(&stage.stage_id, Some(layer.index));
        // Execute modules in topological order within each stage
        for module in &stage.modules {
            // Per-layer host filter (packet 92): skip this module on this layer
            // if it declares [[region_split]] semantics and no region's
            // variant_chain matches any of them. The `continue` is placed
            // BEFORE on_module_start so the skipped module is truly absent
            // from the instrumentation and audit log.
            if !module_invocation_allowed_on_layer(module.region_split_semantics(), arena.slice()) {
                continue;
            }

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
                paint_regions: None,
                seam_plan: blackboard.seam_plan().cloned(),
                support_plan: blackboard.support_plan().cloned(),
                region_map: blackboard.region_map().cloned(),
                slice: arena.slice(),
                perimeter: arena.perimeter(),
                layer_collection: arena.layer_collection(),
                surface_classification: blackboard.surface_classification().map(|a| a.as_ref()),
                // Committed InfillIR is only marshalled into the
                // `prior-infill` parameter of `run-infill-postprocess`
                // (ADR-0028 Option 1b); every other stage gets `None`.
                infill: if stage.stage_id == "Layer::InfillPostProcess" {
                    arena.infill()
                } else {
                    None
                },
            };
            // Seam plan consulted by the perimeter arms of `apply` (ADR-0020).
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
                Ok(commit) => commit,
                Err(e) => {
                    let message = e.to_string();
                    instrumentation.on_module_error(
                        &stage.stage_id,
                        Some(layer.index),
                        module.module_id(),
                        &message,
                        true,
                    );
                    instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id().to_owned(),
                        message,
                    });
                }
            };

            // Apply the invocation's per-stage commit (ADR-0020). `None` means it
            // committed nothing. `apply` owns the stage's own pre/post hooks —
            // entity-order proposal, seam back-fill, fill partition, anchor
            // stamping — so there is no replayed protocol here.
            if let Some(staged) = commit {
                let ctx = StageApplyContext {
                    stage_id: &stage.stage_id,
                    module_id: module.module_id().as_str(),
                    layer_index: layer.index,
                    seam_plan: seam_plan_ir_for_commit,
                };
                if let Err(e) = apply(&mut arena, staged, &ctx) {
                    let message = e.to_string();
                    instrumentation.on_module_error(
                        &stage.stage_id,
                        Some(layer.index),
                        module.module_id(),
                        &message,
                        true,
                    );
                    instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id().to_owned(),
                        message,
                    });
                }
            }

            let writes = ir_path_for_layer_stage(&stage.stage_id)
                .map(|p| vec![p])
                .unwrap_or_default();
            let runtime_reads = runner.last_runtime_reads();
            // Drain module log messages (already forwarded to the log facade
            // inside the dispatcher; this clears the thread-local stash).
            let _log_messages = runner.last_log_messages();
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
            arena.slice(),
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

/// Hydrate the per-layer arena's `SliceIR` slot from the prepass-committed
/// `Vec<SliceIR>` on the blackboard, unless already staged. Shared by
/// [`execute_single_layer_inner`] and [`execute_captured_stages`] (packet
/// 158) so the two per-layer entry points hydrate identically.
fn hydrate_slice_arena(
    arena: &mut LayerArena,
    blackboard: &Blackboard,
    layer: &GlobalLayer,
) -> Result<(), LayerExecutionError> {
    if arena.slice().is_some() {
        return Ok(());
    }
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
        })
}

/// Immediately before `Layer::PathOptimization` runs, freeze the assembled
/// `LayerCollectionIR.ordered_entities` into the arena so the
/// path-optimization commit path (and any downstream per-layer stage) can
/// see the same entity sequence the host emitter will consume. Shared by
/// [`execute_single_layer_inner`] and [`execute_captured_stages`] (packet 158).
fn prestage_layer_collection_if_path_optimization(
    arena: &mut LayerArena,
    stage: &CompiledStage,
    layer: &GlobalLayer,
    blackboard: &Blackboard,
) {
    if stage.stage_id != "Layer::PathOptimization" || arena.layer_collection().is_some() {
        return;
    }
    let ordered_entities = assemble_ordered_entities(
        layer.index,
        arena.perimeter(),
        arena.infill(),
        arena.support(),
        blackboard.region_map().map(|arc| arc.as_ref()),
        arena.slice(),
    );
    arena.set_layer_collection(LayerCollectionIR {
        global_layer_index: layer.index,
        z: layer.z,
        ordered_entities,
        ..Default::default()
    });
}

// ── Typed tap capture (packet 158) ─────────────────────────────────────────
//
// Request-gated, post-commit IR capture at the executor boundary. Runs only
// the scheduler dependency closure required to reach the furthest selected
// tap, then stops (docs/specs/visual-pipeline-debug.md "Dependency
// Closure"; ADR-0037). Scope for this packet is the `Layer::*` stages whose
// committed output is one of the four arena-owned IR types
// (`PerimeterIR`/`InfillIR`/`SupportIR`/`LayerCollectionIR`) reachable from
// [`apply`]; PrePass and G-code taps are out of scope here.

/// Per-layer stage IDs this packet supports as typed-capture taps. Mirrors
/// the `Layer::*` rows of the Stage Tap Inventory
/// (`docs/specs/visual-pipeline-debug.md`) whose source is an [`apply`]
/// commit boundary.
pub const SUPPORTED_TAP_STAGE_IDS: &[&str] = &[
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::PathOptimization",
];

// ── Blackboard-read tap capture (packet 161, Step 3) ───────────────────────
//
// A second, distinct tap family (ADR-0040 "three tap classes"): these taps
// source from the committed, whole-print `Vec<SliceIR>` Blackboard slot
// (`Blackboard::slice_ir`) rather than a per-layer `LayerArena` commit. They
// are read directly off a `Blackboard` obtained from
// `crate::run::prepare_prepass_context` — prepass only, no per-layer
// scheduler closure, no module dispatch, no `LayerStageRunner` involved (see
// [`execute_blackboard_taps`]).

/// SliceIR-family Blackboard-read taps this packet supports
/// (`docs/specs/visual-pipeline-debug.md` Stage Tap Inventory). All four read
/// the same committed `Vec<SliceIR>` slot; the tap id is capture identity
/// only — it does not change which fields are cloned. `Layer::PaintRegionAnnotation`
/// and `Layer::SlicePostProcess` are the pre/post pair for the same post-edit
/// annotation view (mirrors the `Layer::Perimeters`/`Layer::PerimetersPostProcess`
/// pairing in [`SUPPORTED_TAP_STAGE_IDS`]).
///
/// Packet 161 Step 4 adds five more Blackboard-read taps, each sourcing a
/// different prepass composite instead of `Vec<SliceIR>` (still validated
/// against the same committed `Vec<SliceIR>` layer universe — see
/// [`execute_blackboard_taps`]):
/// - `PrePass::MeshAnalysis` and `PrePass::OverhangAnnotation` both read the
///   committed `SurfaceClassificationIR` (`CapturedIr::SurfaceClassification`).
/// - `PrePass::SeamPlanning` reads the committed `SeamPlanIR`
///   (`CapturedIr::SeamPlan`).
/// - `PrePass::SupportGeometry` reads the committed `SupportGeometryIR` +
///   `SupportPlanIR` composite (`CapturedIr::SupportGeometry`).
/// - `PrePass::RegionMapping` reads the committed `RegionMapIR` retained
///   alongside the whole-print `Vec<SliceIR>` for a render-time join
///   (`CapturedIr::RegionMapping`).
pub const BLACKBOARD_TAP_STAGE_IDS: &[&str] = &[
    "Layer::Slice",
    "PrePass::PaintSegmentation",
    "Layer::PaintRegionAnnotation",
    "Layer::SlicePostProcess",
    "PrePass::MeshAnalysis",
    "PrePass::OverhangAnnotation",
    "PrePass::SeamPlanning",
    "PrePass::SupportGeometry",
    "PrePass::RegionMapping",
];

/// PostPass whole-print taps (ADR-0040 "three tap classes", packet 161 Step
/// 5): the third tap class. Unlike [`SUPPORTED_TAP_STAGE_IDS`] (bounded
/// per-layer arena closure) and [`BLACKBOARD_TAP_STAGE_IDS`] (prepass-only,
/// already-committed reads), these two taps source from driving the *whole*
/// per-layer -> finalization -> postpass pipeline prefix — every stage,
/// every layer — via [`crate::postpass::execute_postpass_with_capture`]'s
/// optional `PostPassCapture` sink:
/// - `PostPass::LayerFinalization` reads the finalized, travel-reconciled
///   `Vec<LayerCollectionIR>` (`CapturedIr::LayerFinalization`) — the same
///   stage id the scheduler already assigns the module-based finalization
///   stage (`ExecutionPlan::layer_finalization_stage`).
/// - `PostPass::GCodeEmit` reads the initially emitted `GCodeIR`
///   (`CapturedIr::GCodeEmit`), before any `GCodePostProcess` module runs —
///   the same stage id `execute_postpass_with_capture` already uses to
///   instrument the host-built-in emission stage.
///
/// A request selecting either tap pays the cost of the whole print (not a
/// truncated per-layer closure): `pnp-cli`'s visual-debug command records
/// this as a documented whole-print closure deviation in the manifest's
/// `executed_stage_ids`/`executed_layer_indices` rather than silently
/// pretending it only ran the requested layers.
pub const POSTPASS_TAP_STAGE_IDS: &[&str] = &["PostPass::LayerFinalization", "PostPass::GCodeEmit"];

/// A renderer-owned, post-commit IR snapshot. Always an owned clone taken
/// immediately after [`apply`] returns `Ok` — never a borrow into
/// `LayerArena` (ADR-0037).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum CapturedIr {
    /// `Layer::Perimeters` / `Layer::PerimetersPostProcess` commit.
    Perimeter(PerimeterIR),
    /// `Layer::Infill` / `Layer::InfillPostProcess` commit.
    Infill(InfillIR),
    /// `Layer::Support` / `Layer::SupportPostProcess` commit.
    Support(SupportIR),
    /// `Layer::PathOptimization` commit.
    LayerCollection(LayerCollectionIR),
    /// Blackboard-read SliceIR-family tap (packet 161, Step 3): one
    /// whole-print `SliceIR` entry, selected by `global_layer_index`, from
    /// the committed `Vec<SliceIR>` Blackboard slot. Covers
    /// `Layer::Slice`, `PrePass::PaintSegmentation`,
    /// `Layer::PaintRegionAnnotation`, and `Layer::SlicePostProcess` (see
    /// [`BLACKBOARD_TAP_STAGE_IDS`]).
    Slice(SliceIR),
    /// Blackboard-read composite tap (packet 161, Step 4): the whole-print
    /// committed `SurfaceClassificationIR`, unfiltered by layer (its only
    /// per-layer-keyed field, `overhang_quartile_polygons`, stays keyed in
    /// the captured payload for the renderer to filter at render time).
    /// Covers `PrePass::MeshAnalysis` and `PrePass::OverhangAnnotation`.
    SurfaceClassification(SurfaceClassificationIR),
    /// Blackboard-read tap (packet 161, Step 4): the whole-print committed
    /// `SeamPlanIR`. Covers `PrePass::SeamPlanning`.
    SeamPlan(SeamPlanIR),
    /// Blackboard-read composite tap (packet 161, Step 4): the committed
    /// `SupportGeometryIR` (coarse outlines) paired with the committed
    /// `SupportPlanIR` (planned branch geometry) — both prepass artifacts
    /// documented as `PrePass::SupportGeometry`'s source
    /// (`docs/specs/visual-pipeline-debug.md` Stage Tap Inventory).
    SupportGeometry {
        /// Coarse support outline IR.
        geometry: SupportGeometryIR,
        /// Planned branch-geometry IR.
        plan: SupportPlanIR,
    },
    /// Blackboard-read composite tap (packet 161, Step 4): the committed
    /// `RegionMapIR` retained alongside the whole-print `Vec<SliceIR>` for
    /// the renderer to join by `RegionKey` at render time (Step 6) — this
    /// capture step performs no join. Covers `PrePass::RegionMapping`.
    RegionMapping {
        /// Region-to-dispatch mapping IR.
        region_map: RegionMapIR,
        /// Whole-print slice IR, retained for the render-time join.
        slice_ir: Vec<SliceIR>,
    },
    /// PostPass whole-print capture (packet 161, Step 5): the finalized,
    /// travel-reconciled `Vec<LayerCollectionIR>` fed into G-code emission,
    /// captured unfiltered by layer (mirrors `RegionMapping`'s
    /// whole-composite-then-render-time-filter pattern) — the renderer
    /// selects the request's chosen layers at render time (Step 6). Covers
    /// `PostPass::LayerFinalization`.
    LayerFinalization(Vec<LayerCollectionIR>),
    /// PostPass whole-print capture (packet 161, Step 5): the `GCodeIR` as
    /// initially emitted, before any `GCodePostProcess` module runs. Covers
    /// `PostPass::GCodeEmit`.
    GCodeEmit(GCodeIR),
}

impl CapturedIr {
    /// The captured IR type's own `schema_version`, formatted `MAJOR.MINOR.PATCH`.
    ///
    /// For the two composite variants (`SupportGeometry`, `RegionMapping`)
    /// this reports the primary IR's version (`SupportGeometryIR`,
    /// `RegionMapIR` respectively) — the secondary IR's own
    /// `schema_version` field is still directly inspectable on the captured
    /// struct itself. `LayerFinalization` reports its first layer's
    /// `schema_version` (falling back to
    /// `slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` for an empty
    /// print, which never occurs downstream of a real capture) since
    /// `LayerCollectionIR`'s schema version is not print-wide state.
    pub fn schema_version_string(&self) -> String {
        let v = match self {
            Self::Perimeter(ir) => ir.schema_version,
            Self::Infill(ir) => ir.schema_version,
            Self::Support(ir) => ir.schema_version,
            Self::LayerCollection(ir) => ir.schema_version,
            Self::Slice(ir) => ir.schema_version,
            Self::SurfaceClassification(ir) => ir.schema_version,
            Self::SeamPlan(ir) => ir.schema_version,
            Self::SupportGeometry { geometry, .. } => geometry.schema_version,
            Self::RegionMapping { region_map, .. } => region_map.schema_version,
            Self::LayerFinalization(layers) => layers
                .first()
                .map(|l| l.schema_version)
                .unwrap_or(slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION),
            Self::GCodeEmit(ir) => ir.schema_version,
        };
        format!("{}.{}.{}", v.major, v.minor, v.patch)
    }
}

/// One typed tap capture: the requested stage's committed IR for one layer.
#[derive(Debug, Clone, PartialEq)]
pub struct StageCapture {
    /// The tap (per-layer stage id) this capture was taken at.
    pub stage_id: StageId,
    /// Global layer index this capture belongs to.
    pub layer_index: u32,
    /// Layer Z (mm) at capture time.
    pub layer_z: f32,
    /// The renderer-owned captured IR.
    pub ir: CapturedIr,
}

/// A layer the closure executed for a genuine, real scheduler-fixed-order
/// correctness dependency even though it was not in the request's selected
/// layer set: executed, not rendered/retained (docs/specs/
/// visual-pipeline-debug.md "Dependency Closure"). No tap in
/// `SUPPORTED_TAP_STAGE_IDS` has such a dependency today (Tier 2 per-layer
/// work is cross-layer-independent — see [`execute_captured_stages`]), so
/// this never appears in practice; it exists so a future tap that does need
/// one reports a specific, real reason here instead of the closure silently
/// running every layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerExpansion {
    /// The layer that was executed but not captured.
    pub layer_index: u32,
    /// Human-readable reason it was required.
    pub reason: String,
}

/// Request for [`execute_captured_stages`]: the taps (per-layer stage ids)
/// and layers a visual-debug request selected. Both must be validated
/// (non-empty, resolvable) by the caller before matching against the plan;
/// [`execute_captured_stages`] performs the plan-relative validation itself
/// and fails closed rather than executing a partial closure.
#[derive(Debug, Clone)]
pub struct CaptureRequest {
    /// Selected taps, as per-layer stage ids (see [`SUPPORTED_TAP_STAGE_IDS`]).
    pub stage_ids: Vec<StageId>,
    /// Selected global layer indices.
    pub layer_indices: Vec<u32>,
}

/// Successful output of [`execute_captured_stages`].
#[derive(Debug, Clone, Default)]
pub struct CaptureOutput {
    /// Retained typed captures, one per requested (tap, layer) pair,
    /// deterministically ordered by (`STAGE_ORDER` position, layer_index).
    pub captures: Vec<StageCapture>,
    /// Layers executed for correctness but not captured, ordered by
    /// layer_index.
    pub expansions: Vec<LayerExpansion>,
    /// The truncated per-layer stage closure that actually ran, in fixed
    /// `STAGE_ORDER` order — the prerequisite stages through and including
    /// the furthest selected tap, and nothing after it.
    pub closure_stage_ids: Vec<StageId>,
    /// Global layer indices the closure actually ran the truncated stage
    /// sequence for, ascending (follow-up fix: layer-skip). Equal to the
    /// request's validated, plan-applicable `layer_indices` — a non-selected
    /// layer never appears here because it was never executed (see
    /// [`execute_captured_stages`]).
    pub executed_layer_indices: Vec<u32>,
}

/// Failure modes specific to typed tap capture (packet 158). Every variant
/// fails outright — never a partial success (docs/specs/
/// visual-pipeline-debug.md Success Criterion 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureExecutionError {
    /// A requested tap is not a supported, documented tap.
    UnknownTap {
        /// The offending tap name, verbatim from the request.
        tap: String,
    },
    /// No requested layer index resolves to a real layer in the plan.
    NoApplicableLayer,
    /// A requested tap's source IR was unavailable at its documented
    /// commit boundary (e.g. no module bound to that stage in this plan).
    TapSourceUnavailable {
        /// The tap whose source was unavailable.
        stage_id: StageId,
        /// The layer at which it was unavailable.
        layer_index: u32,
    },
    /// The underlying per-layer executor failed.
    Layer(LayerExecutionError),
}

impl fmt::Display for CaptureExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTap { tap } => write!(f, "unsupported visual-debug tap: '{tap}'"),
            Self::NoApplicableLayer => write!(
                f,
                "no requested layer applies to this plan; nothing to capture"
            ),
            Self::TapSourceUnavailable {
                stage_id,
                layer_index,
            } => write!(
                f,
                "tap '{stage_id}' source IR unavailable at layer {layer_index}: \
                 no module committed it at its documented boundary"
            ),
            Self::Layer(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CaptureExecutionError {}

impl From<LayerExecutionError> for CaptureExecutionError {
    fn from(e: LayerExecutionError) -> Self {
        Self::Layer(e)
    }
}

fn capture_ir_for_stage(stage_id: &str, arena: &LayerArena) -> Option<CapturedIr> {
    match stage_id {
        "Layer::Perimeters" | "Layer::PerimetersPostProcess" => {
            arena.perimeter().cloned().map(CapturedIr::Perimeter)
        }
        "Layer::Infill" | "Layer::InfillPostProcess" => {
            arena.infill().cloned().map(CapturedIr::Infill)
        }
        "Layer::Support" | "Layer::SupportPostProcess" => {
            arena.support().cloned().map(CapturedIr::Support)
        }
        "Layer::PathOptimization" => arena
            .layer_collection()
            .cloned()
            .map(CapturedIr::LayerCollection),
        _ => None,
    }
}

/// Request-gated, typed post-stage capture at the executor boundary
/// (packet 158).
///
/// Executes only the scheduler dependency closure required to reach the
/// furthest tap in `request.stage_ids`: `plan.per_layer_stages` is
/// truncated (in fixed `STAGE_ORDER` order) to the prerequisite stages
/// through and including that tap; nothing after it runs. That truncated
/// closure runs ONLY for layers in `request.layer_indices` — Tier 2
/// per-layer work has no cross-layer dependency for the `Layer::*` stages
/// this packet supports (docs/01_system_architecture.md "Tier 2 —
/// Per-Layer": "Each layer runs independently. Layers share no mutable
/// state. The Blackboard is read-only during this tier."), so a
/// non-selected layer is never required for correctness in this closure's
/// scope and is not executed at all — no arena is created, no module is
/// invoked, no `apply` call runs. [`CaptureOutput::expansions`] therefore
/// stays empty for every request today; the type is retained so a future
/// tap with a genuine, real correctness dependency can report a
/// specifically-worded [`LayerExpansion`] rather than the closure silently
/// over-running.
///
/// A capture is taken immediately after [`apply`] returns `Ok` for a
/// requested (tap, layer) pair — a post-commit, renderer-owned clone, never
/// a borrow into `LayerArena` (ADR-0037). If a requested tap's arena slot is
/// still empty once its stage has run (no module committed it), the whole
/// call fails with [`CaptureExecutionError::TapSourceUnavailable`] rather
/// than returning a partial bundle.
pub fn execute_captured_stages(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
    request: &CaptureRequest,
) -> Result<CaptureOutput, CaptureExecutionError> {
    for tap in &request.stage_ids {
        if !SUPPORTED_TAP_STAGE_IDS.contains(&tap.as_str()) {
            return Err(CaptureExecutionError::UnknownTap { tap: tap.clone() });
        }
    }
    if request.stage_ids.is_empty() {
        return Ok(CaptureOutput::default());
    }

    let real_layer_indices: HashSet<u32> = plan.global_layers.iter().map(|l| l.index).collect();
    let applicable: HashSet<u32> = request
        .layer_indices
        .iter()
        .copied()
        .filter(|i| real_layer_indices.contains(i))
        .collect();
    if applicable.is_empty() {
        return Err(CaptureExecutionError::NoApplicableLayer);
    }

    let requested: HashSet<&str> = request.stage_ids.iter().map(|s| s.as_str()).collect();
    let Some(furthest_idx) = plan
        .per_layer_stages
        .iter()
        .rposition(|s| requested.contains(s.stage_id.as_str()))
    else {
        // None of the (validated, documented) requested taps have any
        // module bound to their stage in this plan — their source can
        // never become available in this closure.
        let mut missing_taps: Vec<&String> = request.stage_ids.iter().collect();
        missing_taps.sort();
        return Err(CaptureExecutionError::TapSourceUnavailable {
            stage_id: missing_taps[0].clone(),
            layer_index: *applicable.iter().min().expect("applicable is non-empty"),
        });
    };
    let truncated_stages = &plan.per_layer_stages[..=furthest_idx];
    let closure_stage_ids: Vec<StageId> = truncated_stages
        .iter()
        .map(|s| s.stage_id.clone())
        .collect();

    // Only the requested (applicable) layers ever execute the closure — see
    // the "no cross-layer dependency" note on this function's doc comment.
    // A non-selected layer is skipped entirely, not merely un-retained.
    let mut sorted_layers: Vec<&GlobalLayer> = plan
        .global_layers
        .iter()
        .filter(|l| applicable.contains(&l.index))
        .collect();
    sorted_layers.sort_by_key(|l| l.index);
    let executed_layer_indices: Vec<u32> = sorted_layers.iter().map(|l| l.index).collect();

    let mut captures = Vec::new();
    // Stays empty: no tap in `SUPPORTED_TAP_STAGE_IDS` has a genuine
    // cross-layer correctness dependency today, so no layer is ever
    // executed-but-not-retained. See [`LayerExpansion`] for when a real one
    // would be recorded here.
    let expansions: Vec<LayerExpansion> = Vec::new();

    for layer in sorted_layers {
        let mut arena = LayerArena::new();
        hydrate_slice_arena(&mut arena, blackboard, layer)?;

        for stage in truncated_stages {
            prestage_layer_collection_if_path_optimization(&mut arena, stage, layer, blackboard);

            for module in &stage.modules {
                if !module_invocation_allowed_on_layer(
                    module.region_split_semantics(),
                    arena.slice(),
                ) {
                    continue;
                }
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
                    paint_regions: None,
                    seam_plan: blackboard.seam_plan().cloned(),
                    support_plan: blackboard.support_plan().cloned(),
                    region_map: blackboard.region_map().cloned(),
                    slice: arena.slice(),
                    perimeter: arena.perimeter(),
                    layer_collection: arena.layer_collection(),
                    surface_classification: blackboard.surface_classification().map(|a| a.as_ref()),
                    // Same gating as `execute_per_layer`: the committed
                    // InfillIR feeds only `run-infill-postprocess`.
                    infill: if stage.stage_id == "Layer::InfillPostProcess" {
                        arena.infill()
                    } else {
                        None
                    },
                };
                let seam_plan_ir_for_commit = blackboard.seam_plan().map(|arc| arc.as_ref());

                let commit = runner
                    .run_stage(&stage.stage_id, layer, &live_module, input)
                    .map_err(|e| {
                        CaptureExecutionError::Layer(LayerExecutionError::FatalLayer {
                            layer_index: layer.index,
                            stage_id: stage.stage_id.clone(),
                            module_id: module.module_id().to_owned(),
                            message: e.to_string(),
                        })
                    })?;

                if let Some(staged) = commit {
                    let ctx = StageApplyContext {
                        stage_id: &stage.stage_id,
                        module_id: module.module_id().as_str(),
                        layer_index: layer.index,
                        seam_plan: seam_plan_ir_for_commit,
                    };
                    apply(&mut arena, staged, &ctx).map_err(|e| {
                        CaptureExecutionError::Layer(LayerExecutionError::FatalLayer {
                            layer_index: layer.index,
                            stage_id: stage.stage_id.clone(),
                            module_id: module.module_id().to_owned(),
                            message: e.to_string(),
                        })
                    })?;
                }
            }

            if requested.contains(stage.stage_id.as_str()) {
                match capture_ir_for_stage(&stage.stage_id, &arena) {
                    Some(ir) => captures.push(StageCapture {
                        stage_id: stage.stage_id.clone(),
                        layer_index: layer.index,
                        layer_z: layer.z,
                        ir,
                    }),
                    None => {
                        return Err(CaptureExecutionError::TapSourceUnavailable {
                            stage_id: stage.stage_id.clone(),
                            layer_index: layer.index,
                        })
                    }
                }
            }
        }
    }

    captures.sort_by_key(|c| {
        (
            STAGE_ORDER
                .iter()
                .position(|s| *s == c.stage_id.as_str())
                .unwrap_or(usize::MAX),
            c.layer_index,
        )
    });
    // `expansions` is always empty (see the field's initializer above), so
    // there is nothing to sort.

    Ok(CaptureOutput {
        captures,
        expansions,
        closure_stage_ids,
        executed_layer_indices,
    })
}

/// Request-gated, Blackboard-read capture (packet 161, Steps 3-4): every tap
/// in [`BLACKBOARD_TAP_STAGE_IDS`].
///
/// Distinct closure from [`execute_captured_stages`] (ADR-0040 "three tap
/// classes"): this reads already-committed prepass artifacts straight off
/// `Blackboard` getters. It never constructs a [`LayerArena`], never takes a
/// `LayerStageRunner`/`WasmInstancePool`, and never invokes a module — there
/// is no per-layer scheduler closure to run because every source is already
/// fully committed by prepass (`crate::run::prepare_prepass_context` runs
/// prepass, then this function reads off its `blackboard` field). Each
/// clone happens once, immediately — never a live borrow retained past this
/// call.
///
/// The committed, whole-print `Vec<SliceIR>` (`blackboard.slice_ir()`) is the
/// layer universe every Blackboard-read tap validates `request.layer_indices`
/// against, SliceIR-family or composite alike: in the real pipeline every
/// prepass artifact this function reads (`SurfaceClassificationIR`,
/// `SeamPlanIR`, `SupportGeometryIR`/`SupportPlanIR`, `RegionMapIR`) is
/// committed for the same whole print, alongside `PrePass::Slice`, before any
/// per-layer tier runs. The four SliceIR-family taps capture the single
/// `SliceIR` entry matching the requested layer; the five composite taps
/// (`PrePass::MeshAnalysis`, `PrePass::OverhangAnnotation`,
/// `PrePass::SeamPlanning`, `PrePass::SupportGeometry`,
/// `PrePass::RegionMapping`) are not themselves layer-indexed at the top
/// level, so each applicable requested layer gets the *whole* committed
/// composite, unfiltered — the renderer performs any per-layer filtering or
/// `RegionKey` join at render time (Step 6), never this capture step.
///
/// `CaptureOutput::closure_stage_ids` is always empty here (no arena stage
/// sequence ran); `CaptureOutput::expansions` is always empty (no cross-layer
/// dependency exists for a whole-print, already-committed slot).
///
/// # Errors
///
/// - [`CaptureExecutionError::UnknownTap`] if a requested tap is not in
///   [`BLACKBOARD_TAP_STAGE_IDS`].
/// - [`CaptureExecutionError::NoApplicableLayer`] if no requested layer index
///   matches a `global_layer_index` present in the committed `Vec<SliceIR>`.
/// - [`CaptureExecutionError::TapSourceUnavailable`] if the `Vec<SliceIR>`
///   Blackboard slot itself was never committed (prepass never ran
///   `PrePass::Slice` — should not happen downstream of
///   `prepare_prepass_context`, but fails closed rather than panicking), or
///   if a requested composite tap's own Blackboard slot was never committed.
pub fn execute_blackboard_taps(
    blackboard: &Blackboard,
    request: &CaptureRequest,
) -> Result<CaptureOutput, CaptureExecutionError> {
    for tap in &request.stage_ids {
        if !BLACKBOARD_TAP_STAGE_IDS.contains(&tap.as_str()) {
            return Err(CaptureExecutionError::UnknownTap { tap: tap.clone() });
        }
    }
    if request.stage_ids.is_empty() {
        return Ok(CaptureOutput::default());
    }

    let Some(slice_ir) = blackboard.slice_ir() else {
        let mut missing_taps: Vec<&String> = request.stage_ids.iter().collect();
        missing_taps.sort();
        let layer_index = request
            .layer_indices
            .iter()
            .copied()
            .min()
            .ok_or(CaptureExecutionError::NoApplicableLayer)?;
        return Err(CaptureExecutionError::TapSourceUnavailable {
            stage_id: missing_taps[0].clone(),
            layer_index,
        });
    };

    let real_layer_indices: HashSet<u32> = slice_ir.iter().map(|s| s.global_layer_index).collect();
    let applicable: HashSet<u32> = request
        .layer_indices
        .iter()
        .copied()
        .filter(|i| real_layer_indices.contains(i))
        .collect();
    if applicable.is_empty() {
        return Err(CaptureExecutionError::NoApplicableLayer);
    }

    let mut sorted_layer_indices: Vec<u32> = applicable.into_iter().collect();
    sorted_layer_indices.sort_unstable();

    let mut captures = Vec::new();
    for &layer_index in &sorted_layer_indices {
        // `real_layer_indices` was built from `slice_ir`, so this `find`
        // always succeeds for every index in `sorted_layer_indices`.
        let ir = slice_ir
            .iter()
            .find(|s| s.global_layer_index == layer_index)
            .expect("layer_index came from real_layer_indices, derived from slice_ir");
        for tap in &request.stage_ids {
            let captured_ir = match tap.as_str() {
                "Layer::Slice"
                | "PrePass::PaintSegmentation"
                | "Layer::PaintRegionAnnotation"
                | "Layer::SlicePostProcess" => CapturedIr::Slice(ir.clone()),
                "PrePass::MeshAnalysis" | "PrePass::OverhangAnnotation" => {
                    let sc = blackboard.surface_classification().ok_or(
                        CaptureExecutionError::TapSourceUnavailable {
                            stage_id: tap.clone(),
                            layer_index,
                        },
                    )?;
                    CapturedIr::SurfaceClassification((**sc).clone())
                }
                "PrePass::SeamPlanning" => {
                    let sp = blackboard.seam_plan().ok_or(
                        CaptureExecutionError::TapSourceUnavailable {
                            stage_id: tap.clone(),
                            layer_index,
                        },
                    )?;
                    CapturedIr::SeamPlan((**sp).clone())
                }
                "PrePass::SupportGeometry" => {
                    let geometry = blackboard.support_geometry().ok_or(
                        CaptureExecutionError::TapSourceUnavailable {
                            stage_id: tap.clone(),
                            layer_index,
                        },
                    )?;
                    let plan = blackboard.support_plan().ok_or(
                        CaptureExecutionError::TapSourceUnavailable {
                            stage_id: tap.clone(),
                            layer_index,
                        },
                    )?;
                    CapturedIr::SupportGeometry {
                        geometry: (**geometry).clone(),
                        plan: (**plan).clone(),
                    }
                }
                "PrePass::RegionMapping" => {
                    let region_map = blackboard.region_map().ok_or(
                        CaptureExecutionError::TapSourceUnavailable {
                            stage_id: tap.clone(),
                            layer_index,
                        },
                    )?;
                    CapturedIr::RegionMapping {
                        region_map: (**region_map).clone(),
                        slice_ir: (**slice_ir).clone(),
                    }
                }
                _ => {
                    unreachable!("tap validated against BLACKBOARD_TAP_STAGE_IDS at function entry")
                }
            };
            captures.push(StageCapture {
                stage_id: tap.clone(),
                layer_index,
                layer_z: ir.z,
                ir: captured_ir,
            });
        }
    }

    // Deterministic ordering: tap position in `STAGE_ORDER`, then layer index
    // — mirrors `execute_captured_stages`'s sort.
    captures.sort_by_key(|c| {
        (
            STAGE_ORDER
                .iter()
                .position(|s| *s == c.stage_id.as_str())
                .unwrap_or(usize::MAX),
            c.layer_index,
        )
    });

    Ok(CaptureOutput {
        captures,
        expansions: Vec::new(),
        closure_stage_ids: Vec::new(),
        executed_layer_indices: sorted_layer_indices,
    })
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

/// No-op stub: paint annotation is now handled by `PrePass::PaintSegmentation`
/// which writes colour data directly into `SliceIR` segment annotations.
/// The `Layer::PaintRegionAnnotation` stage dispatch and safety-net paths are
/// retained for plan wiring, but the host built-in body is a no-op (AC-16).
fn run_paint_annotation(
    _blackboard: &Blackboard,
    _required_semantics: &[PaintSemantic],
    _sink: &(dyn LayerProgressSink + Sync),
    _arena: &mut LayerArena,
    _layer: &GlobalLayer,
    _event_stage: &str,
) -> Result<(), LayerExecutionError> {
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
    slice: Option<&SliceIR>,
) -> Vec<PrintEntity> {
    let mut out: Vec<PrintEntity> = Vec::new();
    let id_gen = LayerEntityIdGen::new();
    let push = |path: slicer_ir::ExtrusionPath3D,
                role: slicer_ir::ExtrusionRole,
                tool_index: u32,
                key: RegionKey,
                acc: &mut Vec<PrintEntity>| {
        let topo_order = acc.len() as u32;
        acc.push(PrintEntity {
            entity_id: id_gen.next(),
            path,
            role,
            // Tool is now an explicit SELECTOR; `region_key.region_id` stays a
            // pure region IDENTITY so a paint-variant identity can never leak
            // into the tool slot (the packet-125 OOM).
            tool_index,
            region_key: key,
            topo_order,
        });
    };

    // Fix 2 (Step 19 / Option B′): build two lookup tables from the SliceIR
    // to drive the per-wall and per-infill tool resolution.
    //
    // (a) `variant_tool_by_region` maps `(object_id, region_id) → ToolIndex`
    //     for the painted variants. Used when the host's per-region bucketing
    //     correctly attributes a `PerimeterRegion` / `InfillRegion` to a
    //     painted variant (via the synthesized `region_id` from
    //     `paint_segmentation::paint_variant_region_id`).
    //
    // (b) `painted_regions` is the list of painted SlicedRegions with their
    //     tool indices and polygons. Used as the SPATIAL fallback: when the
    //     guest's SDK-side `SliceRegionView` adapter touches all per-region
    //     polygons up front (see `slicer_macros::__slicer_adapt_slice_regions`),
    //     the host's `current_slice_region` ends up at the LAST region
    //     visited and ALL wall_loops the guest pushes get the same origin.
    //     We salvage the per-wall tool by doing a point-in-polygon test of
    //     the wall's start vertex against each painted SlicedRegion. This
    //     is the path-of-least-disturbance fix that avoids changing the WIT
    //     surface or the SDK adapter pattern (which would be a P95 + P96
    //     architectural change; tracked as a follow-up).
    let variant_tool_by_region: HashMap<(String, u64), u64> = slice
        .map(|s| {
            let mut m: HashMap<(String, u64), u64> = HashMap::new();
            for r in &s.regions {
                for (sem_name, value) in &r.variant_chain {
                    if sem_name == "material" {
                        if let slicer_ir::PaintValue::ToolIndex(n) = value {
                            m.insert((r.object_id.clone(), r.region_id), *n as u64);
                            break;
                        }
                    }
                }
            }
            m
        })
        .unwrap_or_default();
    let painted_regions: Vec<(u64, &Vec<slicer_ir::ExPolygon>)> = slice
        .map(|s| {
            let mut v: Vec<(u64, &Vec<slicer_ir::ExPolygon>)> = Vec::new();
            for r in &s.regions {
                if r.variant_chain.is_empty() {
                    continue;
                }
                for (sem_name, value) in &r.variant_chain {
                    if sem_name == "material" {
                        if let slicer_ir::PaintValue::ToolIndex(n) = value {
                            v.push((*n as u64, &r.polygons));
                            break;
                        }
                    }
                }
            }
            v
        })
        .unwrap_or_default();
    // D-112-MMU-TOPOLOGY closure: build a parallel set of (object_id,
    // region_id) keys for BASE (unpainted) SlicedRegions. The wall/infill
    // tool resolver below uses this set to *skip* the spatial fallback for
    // BASE walls — a BASE outer wall's first vertex may sit inside a
    // per-color cell (the bisector is shared), and the previous
    // `spatial_tool.or(variant_tool)` chain reattributed the BASE wall to
    // that per-color tool, producing a wall whose geometry (the full model
    // outer outline) escapes its attributed per-color cell. BASE walls
    // must stay on `DEFAULT_TOOL` and not pick up a per-color attribution
    // from the spatial point-in-polygon test.
    let base_region_keys: HashSet<(String, u64)> = slice
        .map(|s| {
            s.regions
                .iter()
                .filter(|r| r.variant_chain.is_empty())
                .map(|r| (r.object_id.clone(), r.region_id))
                .collect()
        })
        .unwrap_or_default();

    // Spatial fallback: find the painted SlicedRegion whose polygons contain
    // the given (x, y) point (mm). Returns the painted variant's ToolIndex
    // if a containing region exists. Walls and infill paths emit in mm-space
    // (`ExtrusionPath3D.points: Point3WithWidth { x: f32, y: f32, z: f32 }`).
    let lookup_tool_by_point_mm = |px_mm: f32, py_mm: f32| -> Option<u64> {
        if painted_regions.is_empty() {
            return None;
        }
        let px_mm = px_mm as f64;
        let py_mm = py_mm as f64;
        // 1 µm tolerance. Polygon-edge ties are rare in practice; first-match
        // policy is deterministic in SliceIR.regions order.
        let eps_mm: f64 = 1.0e-3;
        for (tool, polys) in &painted_regions {
            for ep in polys.iter() {
                if slicer_ir::point_in_polygon_winding(ep, px_mm, py_mm, eps_mm) {
                    return Some(*tool);
                }
            }
        }
        None
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
            // Per Step 19 Fix 2: look up the source SlicedRegion's
            // Material/ToolIndex by the region's (object_id, region_id). This
            // wins over modifier-tool but yields to a paint-pipeline-emitted
            // per-point Material tool (forward compat: paint v2 doesn't write
            // segment_annotations[Material] today).
            let variant_tool: Option<u64> = variant_tool_by_region
                .get(&(region.object_id.clone(), region.region_id))
                .copied();
            for wl in &region.walls {
                let paint_tool = dominant_tool_index(&wl.feature_flags);
                // D-112-MMU-TOPOLOGY closure: prefer `variant_tool` (the
                // per-region paint-derived tool, looked up from the source
                // `SlicedRegion`'s variant chain) over the spatial fallback.
                // The previous chain (`paint.or(spatial).or(variant)`) let
                // the spatial fallback re-attribute a per-color region's
                // walls to whichever per-color cell the wall's FIRST vertex
                // happened to land in — producing walls that escape their
                // own per-color cell when the first vertex sits on an
                // adjacent cell's bisector.
                //
                // The spatial fallback is reserved for the LIFO-touch case
                // (when a PerimeterRegion was emitted with the wrong region
                // id by the guest): when `variant_tool` is None for this
                // (object_id, region_id) — i.e. the region has no material
                // variant chain (BASE) or the SDK adapter lost the origin
                // (untagged) — fall through to spatial classification. BASE
                // regions are explicitly excluded: their walls are the
                // full-model outer outline, never a per-color cell wall,
                // and reattributing them to a per-color tool produces
                // per-color headers that escape their own per-color cells
                // (D-112-MMU-TOPOLOGY closure).
                let region_key = (region.object_id.clone(), region.region_id);
                let region_is_base = base_region_keys.contains(&region_key);
                let region_is_tagged = variant_tool_by_region.contains_key(&region_key);
                // Spatial fallback: when the host's per-region bucketing
                // collapsed all wall_loops under a single PerimeterRegion
                // (SDK adapter LIFO touch — see comment on `painted_regions`
                // above), classify each wall by its first vertex's containing
                // painted SlicedRegion. This restores per-wall tool identity
                // in the gcode without a WIT/SDK redesign.
                let spatial_tool: Option<u64> = wl
                    .path
                    .points
                    .first()
                    .and_then(|p| lookup_tool_by_point_mm(p.x, p.y));
                let resolved_tool = paint_tool
                    .or(variant_tool)
                    .or(if region_is_base || region_is_tagged {
                        None
                    } else {
                        spatial_tool
                    })
                    .or(modifier_tool)
                    .unwrap_or(DEFAULT_TOOL);
                let entity_key = RegionKey {
                    global_layer_index,
                    object_id: region.object_id.clone(),
                    // Pure region IDENTITY (restored — packet 125 had overwritten
                    // this with the tool). Postpass back-refs key on this.
                    region_id: region.region_id,
                    variant_chain: Vec::new(),
                };
                let role = wl.path.role.clone();
                push(
                    wl.path.clone(),
                    role,
                    resolved_tool as u32,
                    entity_key,
                    &mut out,
                );
            }
        }
    }

    if let Some(inf) = infill {
        for region in &inf.regions {
            // Per Step 19 Fix 2: derive tool from the SlicedRegion variant
            // chain when available; otherwise fall back to `region_id`.
            let variant_tool: Option<u64> = variant_tool_by_region
                .get(&(region.object_id.clone(), region.region_id))
                .copied();
            // Per-path spatial fallback (same reasoning as the wall loop).
            // Infill paths are likely emitted in a single guest batch and
            // need per-path tool resolution when host bucketing collapsed.
            let infill_region_key = (region.object_id.clone(), region.region_id);
            let infill_region_is_tagged = variant_tool_by_region.contains_key(&infill_region_key);
            let infill_push = |path: &slicer_ir::ExtrusionPath3D,
                               role: slicer_ir::ExtrusionRole,
                               acc: &mut Vec<PrintEntity>| {
                let spatial_tool: Option<u64> = path
                    .points
                    .first()
                    .and_then(|p| lookup_tool_by_point_mm(p.x, p.y));
                // D-112-MMU-TOPOLOGY closure: prefer `variant_tool` over the
                // spatial fallback for tagged regions, matching the wall-loop
                // resolver above.
                let resolved_tool = variant_tool
                    .or(if infill_region_is_tagged {
                        None
                    } else {
                        spatial_tool
                    })
                    .unwrap_or(DEFAULT_TOOL);
                let key = RegionKey {
                    global_layer_index,
                    object_id: region.object_id.clone(),
                    // Pure region IDENTITY (restored — see wall-loop note above).
                    region_id: region.region_id,
                    variant_chain: Vec::new(),
                };
                push(path.clone(), role, resolved_tool as u32, key, acc);
            };
            for path in &region.sparse_infill {
                infill_push(path, path.role.clone(), &mut out);
            }
            for path in &region.solid_infill {
                infill_push(path, path.role.clone(), &mut out);
            }
            for path in &region.ironing {
                infill_push(path, path.role.clone(), &mut out);
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
        // Support geometry prints with the base tool (T0); region_id=0 identity.
        for path in &sup.support_paths {
            push(path.clone(), path.role.clone(), 0, key.clone(), &mut out);
        }
        for path in &sup.interface_paths {
            push(path.clone(), path.role.clone(), 0, key.clone(), &mut out);
        }
        for path in &sup.raft_paths {
            push(path.clone(), path.role.clone(), 0, key.clone(), &mut out);
        }
        for path in &sup.ironing_paths {
            push(path.clone(), path.role.clone(), 0, key.clone(), &mut out);
        }
    }

    out
}

// ── Runtime-side per-stage commit (ADR-0020) ───────────────────────────────
//
// `apply` (above) and these helpers depend only on `LayerArena` and `slicer-ir`
// types, so they live here rather than in `slicer-wasm-host` (which must not
// depend on slicer-runtime). The producer in `slicer-wasm-host` builds the
// `LayerStageCommit`; this side performs the arena writes.

/// Test-only wrapper around the crate-private [`apply`] so integration tests can
/// exercise per-stage commit behavior (e.g. the PerimetersPostProcess field
/// preservation or the PathOptimization anchor stamping) without compiling a
/// bespoke WAT guest per case.
#[doc(hidden)]
pub fn apply_for_test(
    arena: &mut LayerArena,
    commit: LayerStageCommit,
    ctx: &StageApplyContext<'_>,
) -> Result<(), slicer_ir::LayerStageError> {
    apply(arena, commit, ctx)
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

/// Read-only context threaded into [`apply`] (ADR-0020). The output-side twin of
/// `slicer_wasm_host::LayerStageInput`: an extensible borrow-struct carrying the
/// arena-apply-time reads. New apply-time dependencies (e.g. paint regions for
/// seam work) add a field here rather than churning `apply`'s signature.
pub struct StageApplyContext<'a> {
    /// Stage id of the invocation being committed (for error messages).
    pub stage_id: &'a str,
    /// Module id of the invocation being committed (for error messages).
    pub module_id: &'a str,
    /// Global layer index this commit belongs to.
    pub layer_index: u32,
    /// Seam plan consulted by the perimeter arms to back-fill `resolved_seam`.
    pub seam_plan: Option<&'a slicer_ir::SeamPlanIR>,
}

/// Back-fill `resolved_seam` on perimeter regions that the guest left unresolved,
/// from the planning-chosen candidate in `seam_plan` (ADR-0020).
///
/// This is the single home for the seam injection that was previously written
/// twice with divergent code (`Layer::Perimeters` reconstructed `SeamPosition`
/// field-by-field; `Layer::PerimetersPostProcess` cloned `chosen_candidate`).
/// `SeamPlanEntry.chosen_candidate` is itself a `SeamPosition`, so a clone is the
/// faithful, drift-proof form.
fn backfill_resolved_seam(
    perim: &mut PerimeterIR,
    seam_plan: &slicer_ir::SeamPlanIR,
    layer_index: u32,
) {
    for region in &mut perim.regions {
        if region.resolved_seam.is_some() {
            continue;
        }
        if let Some(entry) = seam_plan.entries.iter().find(|e| {
            e.region_key.global_layer_index == layer_index
                && e.region_key.object_id == region.object_id
                && e.region_key.region_id == region.region_id
        }) {
            region.resolved_seam = Some(entry.chosen_candidate.clone());
        }
    }
}

/// Apply one module invocation's [`LayerStageCommit`] to the per-layer arena,
/// including the stage's own pre/post hooks (ADR-0020).
///
/// This is the deep replacement for the `apply_entity_order_proposal` →
/// `commit_layer_outputs` → inline-seam-injection → anchor-override protocol that
/// was previously replayed across four call sites in `execute_single_layer_inner`.
/// Each arm sequences its own hooks; the type makes a forgotten step a compile
/// error rather than a silent regression.
///
/// Anchor note: the `PathOptimization` arm stamps `ordered_entities.len()-1`
/// (per-invocation, "end of layer") onto the four deferred groups here, while
/// travel-move `entity_id`s are resolved later, at end-of-layer assembly, against
/// the final entity list. That asymmetry is inherent — the final list does not
/// exist until every stage has run.
pub(crate) fn apply(
    arena: &mut LayerArena,
    commit: LayerStageCommit,
    ctx: &StageApplyContext<'_>,
) -> Result<(), slicer_ir::LayerStageError> {
    let mk_validation_err = |what: &str, reason: String| slicer_ir::LayerStageError::FatalModule {
        stage_id: ctx.stage_id.to_string(),
        module_id: ctx.module_id.to_string(),
        message: format!("invalid {what} output: {reason}"),
    };

    match commit {
        LayerStageCommit::SeedLayerCollection(lc) => {
            let _ = arena.take_layer_collection(); // clear any auto-assembled one
            arena.set_layer_collection(lc);
        }
        LayerStageCommit::Perimeters(mut ir) => {
            let _ = arena.take_perimeter();
            // Seam back-fill and fill partition are independent (one reads/writes
            // `resolved_seam`, the other `infill_areas`), so back-filling the
            // owned IR before commit is equivalent to the original take→set→
            // partition→take→inject→set sequence, without the double round-trip.
            if let Some(seam_plan) = ctx.seam_plan {
                backfill_resolved_seam(&mut ir, seam_plan, ctx.layer_index);
            }
            arena
                .set_perimeter(ir)
                .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
            crate::region_partition::sync_perimeter_infill_areas_into_slice(
                arena,
                ctx.layer_index,
            )?;
        }
        LayerStageCommit::PerimetersPostProcess(incoming) => {
            let mut original = arena.take_perimeter();
            if let (Some(seam_plan), Some(orig_perim)) = (ctx.seam_plan, original.as_mut()) {
                backfill_resolved_seam(orig_perim, seam_plan, ctx.layer_index);
            }
            // Pair by `(object_id, region_id)`, not positional index: a post-process
            // module that drops a region must not mis-route preserved fields.
            match (incoming, original) {
                (Some(mut ir_owned), Some(orig_perim)) => {
                    for region in ir_owned.regions.iter_mut() {
                        let Some(orig_region) = orig_perim.regions.iter().find(|r| {
                            r.object_id == region.object_id && r.region_id == region.region_id
                        }) else {
                            continue;
                        };
                        if region.resolved_seam.is_none() {
                            if let Some(rs) = &orig_region.resolved_seam {
                                region.resolved_seam = Some(rs.clone());
                            }
                        }
                        if region.infill_areas.is_empty() {
                            region.infill_areas = orig_region.infill_areas.clone();
                        }
                        if region.seam_candidates.is_empty() {
                            region.seam_candidates = orig_region.seam_candidates.clone();
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
            if arena.perimeter().is_some() {
                crate::region_partition::sync_perimeter_infill_areas_into_slice(
                    arena,
                    ctx.layer_index,
                )?;
            }
        }
        LayerStageCommit::Infill(ir) => {
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
        LayerStageCommit::InfillPostProcess(ir) => {
            let _ = arena.take_infill();
            arena
                .set_infill(ir)
                .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
        }
        LayerStageCommit::Support(ir) => {
            arena
                .set_support(ir)
                .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
        }
        LayerStageCommit::SupportPostProcess(ir) => {
            let _ = arena.take_support();
            arena
                .set_support(ir)
                .map_err(|e| slicer_ir::LayerStageError::ArenaCommit { source: e })?;
        }
        LayerStageCommit::SlicePostProcess {
            polygon_updates,
            path_z_updates,
        } => {
            let mut existing =
                arena
                    .take_slice()
                    .ok_or_else(|| slicer_ir::LayerStageError::FatalModule {
                        stage_id: ctx.stage_id.to_string(),
                        module_id: ctx.module_id.to_string(),
                        message: "Layer::SlicePostProcess has no staged SliceIR to merge into; \
                          Layer::Slice must commit per-region slice output first"
                            .into(),
                    })?;

            for (i, (key, polys)) in polygon_updates.iter().enumerate() {
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

            for (i, (key, path_idx, vertex_idx, _z)) in path_z_updates.iter().enumerate() {
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
        LayerStageCommit::PathOptimization(c) => {
            // Apply the entity-order proposal first — it permutes the staged
            // `ordered_entities` the anchor and downstream emit will reference.
            if let Some(ref proposal) = c.order_proposal {
                apply_entity_order_proposal(arena, proposal).map_err(|message| {
                    slicer_ir::LayerStageError::FatalModule {
                        stage_id: ctx.stage_id.to_string(),
                        module_id: ctx.module_id.to_string(),
                        message,
                    }
                })?;
            }
            // End-of-layer anchor, computed once from arena state. The producer
            // carried no anchor for these four groups (ADR-0020), so there is no
            // placeholder to override — the lie cannot exist.
            let anchor = arena
                .layer_collection()
                .map(|lc| lc.ordered_entities.len().saturating_sub(1) as u32)
                .unwrap_or(0);

            for tc in c.tool_changes {
                arena.push_deferred_tool_change(tc);
            }
            for hop_height in c.z_hops {
                if !hop_height.is_finite() || hop_height <= 0.0 {
                    return Err(slicer_ir::LayerStageError::FatalModule {
                        stage_id: ctx.stage_id.to_string(),
                        module_id: ctx.module_id.to_string(),
                        message: format!(
                            "Layer::PathOptimization push-z-hop rejected: \
                             hop-height={hop_height} is not finite and strictly positive"
                        ),
                    });
                }
                arena.push_deferred_z_hop(slicer_ir::ZHop {
                    after_entity_index: anchor,
                    hop_height,
                });
            }
            for kind in c.annotations {
                arena.push_deferred_annotation(slicer_ir::LayerAnnotation {
                    after_entity_index: anchor,
                    kind,
                });
            }
            for r in c.retracts {
                arena.push_deferred_retract(crate::blackboard::DeferredRetract {
                    after_entity_index: anchor,
                    length: r.length,
                    speed: r.speed,
                    is_unretract: r.is_unretract,
                    mode: r.mode,
                });
            }
            for t in c.travel_moves {
                arena.push_deferred_travel_move(crate::blackboard::DeferredTravelMove {
                    after_entity_index: anchor,
                    x: t.x,
                    y: t.y,
                    z: t.z,
                    f: t.f,
                });
            }
        }
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

/// Per-layer host dispatch filter (packet 92).
///
/// Returns `true` iff the module either:
/// - declares NO `[[region_split]]` semantics (paint-transparent default), OR
/// - at least one region on the layer has a `variant_chain` entry whose
///   semantic name is in `declared`.
///
/// Filter granularity is per-(module × layer); per-region filtering is
/// module-internal. `slice` is `None` only when the layer executor bypasses
/// the PrePass::Slice builtin (unusual; conservatively allows invocation).
pub fn module_invocation_allowed_on_layer(
    declared: &std::collections::HashSet<String>,
    slice: Option<&SliceIR>,
) -> bool {
    // Paint-transparent: no region-split declarations → always invoke.
    if declared.is_empty() {
        return true;
    }
    // No SliceIR available: conservatively allow.
    let Some(slice_ir) = slice else {
        return true;
    };
    slice_ir.regions.iter().any(|region| {
        region
            .variant_chain
            .iter()
            .any(|(semantic, _value)| declared.contains(semantic))
    })
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

    /// AC-1 — P125 WI-2 / B: a region whose four tool resolvers all return
    /// `None` must produce a `RegionKey.region_id` of `DEFAULT_TOOL` (0), never
    /// the paint-variant IDENTITY (`region.region_id`).
    ///
    /// The captured OOM value was `0x3E8281949ECA9508`; when cast to `u32` that
    /// yields 2_664_076_552, driving a 9.9 GiB allocation in `emit.rs`.
    ///
    /// NOTE: `assemble_ordered_entities` is `pub(crate)`, so this test lives
    /// here rather than in `tests/integration/`. Run with:
    ///   cargo test -p slicer-runtime -- tool_fallback_never_leaks_region_identity
    #[test]
    fn tool_fallback_never_leaks_region_identity() {
        use slicer_ir::{
            ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion, LoopType, PerimeterIR,
            PerimeterRegion, Point3WithWidth, SemVer, WallBoundaryType, WallFeatureFlags, WallLoop,
            WidthProfile,
        };
        use std::collections::HashMap;

        // A paint-variant IDENTITY large enough to produce a 9.9 GiB alloc
        // when passed as u32 to vec![0.0f32; max_tool+1].
        const IDENTITY: u64 = 0x3E8281949ECA9508;

        let schema_version = SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        };

        // One wall-loop point; spatial lookup will return None (no SliceIR).
        let pt = Point3WithWidth {
            x: 1.0,
            y: 1.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
            overhang_quartile: None,
        };
        let wall_path = ExtrusionPath3D {
            points: vec![pt, pt],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        };
        // feature_flags: tool_index = None  → paint_tool resolver returns None
        let flags = WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new(),
        };
        let wall = WallLoop {
            perimeter_index: 0,
            loop_type: LoopType::Outer,
            path: wall_path.clone(),
            width_profile: WidthProfile::default(),
            feature_flags: vec![flags],
            boundary_type: WallBoundaryType::Interior,
        };

        let perimeter = PerimeterIR {
            schema_version,
            global_layer_index: 0,
            regions: vec![PerimeterRegion {
                object_id: "obj".to_string(),
                region_id: IDENTITY, // the dangerous identity
                walls: vec![wall],
                infill_areas: Vec::new(),
                seam_candidates: Vec::new(),
                resolved_seam: None,
            }],
        };

        // Infill path — also no resolvable tool.
        let infill = InfillIR {
            schema_version,
            global_layer_index: 0,
            regions: vec![InfillRegion {
                object_id: "obj".to_string(),
                region_id: IDENTITY,
                sparse_infill: vec![wall_path.clone()],
                solid_infill: Vec::new(),
                ironing: Vec::new(),
            }],
        };

        // No slice (no variant_tool), no region_map (no modifier_tool),
        // no support — all four resolvers will return None at both sites.
        let entities = super::assemble_ordered_entities(
            0,
            Some(&perimeter),
            Some(&infill),
            None, // support
            None, // region_map  → modifier_tool = None
            None, // slice       → variant_tool = None, spatial_tool = None
        );

        assert!(
            !entities.is_empty(),
            "expected at least one PrintEntity (wall + infill)"
        );
        for entity in &entities {
            // Post region_id↔tool split: the TOOL lives in the first-class
            // `tool_index` (a pure selector). When all four resolvers return
            // None it falls back to DEFAULT_TOOL=0 — the paint-variant IDENTITY
            // can never reach the tool slot (the packet-125 9.9 GiB OOM).
            assert_eq!(
                entity.tool_index, 0,
                "tool_index must be DEFAULT_TOOL=0 on fallback, \
                 not the paint-variant identity {IDENTITY:#x}; got {:#x}",
                entity.tool_index
            );
            // And the identity is correctly PRESERVED in its own slot
            // (`region_key.region_id`) — the split keeps the back-reference the
            // postpasses depend on, instead of zeroing it (packet-125 floor).
            assert_eq!(
                entity.region_key.region_id, IDENTITY,
                "region_key.region_id must preserve the region IDENTITY {IDENTITY:#x} \
                 (no longer overwritten by the tool); got {:#x}",
                entity.region_key.region_id
            );
        }
    }
}
