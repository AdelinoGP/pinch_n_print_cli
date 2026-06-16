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

use slicer_ir::{
    ConfigValue, GlobalLayer, InfillIR, LayerCollectionIR, LayerEntityIdGen, LayerStageCommit,
    ModuleId, PaintSemantic, PerimeterIR, PrintEntity, RegionKey, RegionMapIR, SliceIR, StageId,
    SupportIR, WallFeatureFlags,
};
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageInput, LayerStageRunner, WasmComponent, WasmInstancePool,
};

use crate::instrumentation::{NoopInstrumentation, PipelineInstrumentation};
use crate::progress_events::ProgressEvent;
use crate::slice_postprocess::SlicePostProcessPaintAnnotationError;
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
                arena.slice(),
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
                    instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id().to_owned(),
                        message: e.to_string(),
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
                    instrumentation.on_stage_end(&stage.stage_id, Some(layer.index));
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id().to_owned(),
                        message: e.to_string(),
                    });
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
                    .or(spatial_tool)
                    .or(variant_tool)
                    .or(modifier_tool)
                    .unwrap_or(region.region_id);
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
            // Per Step 19 Fix 2: derive tool from the SlicedRegion variant
            // chain when available; otherwise fall back to `region_id`.
            let variant_tool: Option<u64> = variant_tool_by_region
                .get(&(region.object_id.clone(), region.region_id))
                .copied();
            // Per-path spatial fallback (same reasoning as the wall loop).
            // Infill paths are likely emitted in a single guest batch and
            // need per-path tool resolution when host bucketing collapsed.
            let infill_push = |path: &slicer_ir::ExtrusionPath3D,
                               role: slicer_ir::ExtrusionRole,
                               acc: &mut Vec<PrintEntity>| {
                let spatial_tool: Option<u64> = path
                    .points
                    .first()
                    .and_then(|p| lookup_tool_by_point_mm(p.x, p.y));
                let resolved_tool = spatial_tool.or(variant_tool).unwrap_or(region.region_id);
                let key = RegionKey {
                    global_layer_index,
                    object_id: region.object_id.clone(),
                    region_id: resolved_tool,
                    variant_chain: Vec::new(),
                };
                push(path.clone(), role, key, acc);
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
}
