//! Per-layer parallel executor contracts (TASK-031).
//!
//! This module defines the per-layer parallel execution contracts for running
//! all Tier-2 layer stages using rayon. Each layer gets its own `LayerArena`
//! for intermediate IR storage. Stages within each layer run sequentially,
//! but layers can be processed in parallel.

use std::fmt;

use rayon::prelude::*;
use slicer_ir::{
    GlobalLayer, InfillIR, LayerCollectionIR, ModuleId, PaintRegionIR, PaintSemantic, PerimeterIR,
    PrintEntity, RegionKey, SemVer, StageId, SupportIR,
};

use crate::layer_slice::{execute_layer_slice, LayerSliceError};
use crate::progress_events::ProgressEvent;
use crate::slice_postprocess::{
    execute_slice_postprocess_paint_annotation, paint_annotation_warning_to_progress_event,
    SlicePostProcessPaintAnnotationError, SlicePostProcessPaintAnnotationRequest,
};
use crate::{
    Blackboard, BlackboardError, CompiledModule, ExecutionPlan, LayerArena, LayerArenaError,
    ModuleAccessAudit,
};

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

/// Output produced by a single layer stage module invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerStageOutput {
    /// Module completed successfully with optional IR commits.
    Success,
    /// Module encountered non-fatal error, continue with next module.
    NonFatalError {
        /// Stable human-readable detail.
        message: String,
    },
}

/// Fatal error from a layer stage module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerStageError {
    /// Fatal error, abort entire layer.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// Arena commit failed.
    ArenaCommit {
        /// Underlying arena failure.
        source: LayerArenaError,
    },
}

impl fmt::Display for LayerStageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal layer stage module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::ArenaCommit { source } => write!(f, "arena commit failed: {source}"),
        }
    }
}

impl std::error::Error for LayerStageError {}

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
    /// fatal error (missing paint region data, stale boundary_paint
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

/// Callback surface used by tests and future runtime bindings for layer stage execution.
pub trait LayerStageRunner {
    /// Execute one compiled layer module against the current layer state.
    ///
    /// Returns both the stage output and the runtime IR read paths collected
    /// by the WIT view methods during this call. The returned `runtime_reads`
    /// is used to populate `ModuleAccessAudit.runtime_reads` for audit
    /// construction in `execute_single_layer`.
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>), LayerStageError>;
}

/// Executes the Tier-2 per-layer parallel pipeline using rayon.
///
/// Layers are processed in parallel, but stages within each layer are sequential.
/// Each layer gets its own `LayerArena` that is freed when the layer completes.
/// Results are committed to the blackboard's write-once layer output slots.
pub fn execute_per_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
) -> Result<Vec<LayerCollectionIR>, LayerExecutionError> {
    let (layers, _audits) = execute_per_layer_with_events(plan, blackboard, runner, &NoopLayerProgressSink)?;
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
                    &required_semantics,
                    layer,
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
    out.sort_by(|a, b| semantic_sort_key(a).cmp(&semantic_sort_key(b)));
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
    required_semantics: &[PaintSemantic],
    layer: &GlobalLayer,
) -> Result<(LayerCollectionIR, Vec<ModuleAccessAudit>), LayerExecutionError> {
    let mut audits = Vec::new();

    // Create an isolated LayerArena for this layer
    let mut arena = LayerArena::new();

    // Host-built-in Layer::Slice (docs/04 §Full Lifecycle): commit a
    // `SliceIR` produced from the mesh before any user Layer::Slice /
    // Layer::SlicePostProcess module runs. Skipped if a caller has already
    // pre-seeded a slice (e.g. integration tests).
    if arena.slice().is_none() {
        let slice_ir =
            execute_layer_slice(blackboard.mesh().as_ref(), layer).map_err(|source| {
                LayerExecutionError::LayerSlice {
                    layer_index: layer.index,
                    source,
                }
            })?;
        arena
            .set_slice(slice_ir)
            .map_err(|_| LayerExecutionError::FatalLayer {
                layer_index: layer.index,
                stage_id: "Layer::Slice".to_string(),
                module_id: "<host-built-in>".to_string(),
                message: "slice arena slot already occupied".to_string(),
            })?;
    }

    // Execute stages sequentially in deterministic order.
    // Immediately before `Layer::PathOptimization` runs, freeze the assembled
    // `LayerCollectionIR.ordered_entities` into the arena so the path-
    // optimization commit path (and any downstream per-layer stage) can see
    // the same entity sequence that the host emitter will consume.
    let mut paint_annotation_ran = false;
    for stage in &plan.per_layer_stages {
        if stage.stage_id == "Layer::PathOptimization" && arena.layer_collection().is_none() {
            let ordered_entities = assemble_ordered_entities(
                layer.index,
                arena.perimeter(),
                arena.infill(),
                arena.support(),
            );
            arena.set_layer_collection(LayerCollectionIR {
                schema_version: SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                global_layer_index: layer.index,
                z: layer.z,
                ordered_entities,
                tool_changes: Vec::new(),
                z_hops: Vec::new(),
                annotations: Vec::new(),
            });
        }
        // Execute modules in topological order within each stage
        for module in &stage.modules {
            let run_result = runner.run_stage(&stage.stage_id, layer, module, blackboard, &mut arena);
            let (stage_result, runtime_reads) = match run_result {
                Ok((output, reads)) => (output, reads),
                Err(e) => return Err(LayerExecutionError::FatalLayer {
                    layer_index: layer.index,
                    stage_id: stage.stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: e.to_string(),
                }),
            };

            match stage_result {
                LayerStageOutput::Success => {
                    // Record runtime write audit for this module.
                    // Host-built-in stages (Slice) are not audited.
                    if let Some(path) = ir_path_for_layer_stage(&stage.stage_id) {
                        audits.push(ModuleAccessAudit {
                            module_id: module.module_id.clone(),
                            runtime_reads,
                            runtime_writes: vec![path],
                        });
                    }
                }
                LayerStageOutput::NonFatalError { message: _ } => {
                    // Non-fatal error: log but continue with next module
                }
            }
        }

        // Host-built-in paint annotation runs once, at the end of the
        // `Layer::SlicePostProcess` stage (docs/04 §Full Lifecycle and
        // docs/10 §Paint Region Resolution). This must happen before any
        // downstream stage consumes `SlicedRegion.boundary_paint`.
        if !paint_annotation_ran && stage.stage_id == "Layer::SlicePostProcess" {
            run_paint_annotation(blackboard, required_semantics, sink, &mut arena, layer)?;
            paint_annotation_ran = true;
        }
    }

    // Fallback: if no `Layer::SlicePostProcess` stage was scheduled but paint
    // data is committed, still run the built-in annotator so boundary_paint
    // is populated before finalization.
    if !paint_annotation_ran {
        run_paint_annotation(blackboard, required_semantics, sink, &mut arena, layer)?;
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
        );
        LayerCollectionIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layer_index: layer.index,
            z: layer.z,
            ordered_entities,
            tool_changes: Vec::new(),
            z_hops: Vec::new(),
            annotations: Vec::new(),
        }
    });
    layer_output
        .tool_changes
        .extend(arena.take_deferred_tool_changes());
    layer_output
        .annotations
        .extend(arena.take_deferred_annotations());
    layer_output.z_hops.extend(arena.take_deferred_z_hops());
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
fn ir_path_for_layer_stage(stage_id: &StageId) -> Option<String> {
    match stage_id.as_str() {
        "Layer::Slice" => None, // host-built-in, not audited
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
) -> Result<(), LayerExecutionError> {
    if required_semantics.is_empty() {
        return Ok(());
    }
    let paint_regions = match blackboard.paint_regions() {
        Some(pr) => std::sync::Arc::clone(pr),
        None => return Ok(()),
    };
    let slice_ir = match arena.take_slice() {
        Some(s) => s,
        None => return Ok(()),
    };

    let request = SlicePostProcessPaintAnnotationRequest {
        slice_ir,
        paint_regions,
        required_semantics: required_semantics.to_vec(),
    };
    let result = execute_slice_postprocess_paint_annotation(request).map_err(|source| {
        LayerExecutionError::PaintAnnotation {
            layer_index: layer.index,
            source,
        }
    })?;

    // Surface deterministic, non-fatal fallback warnings through the
    // existing progress-event adapter (docs/09 §ModuleError; docs/11 §73-75).
    for (i, warning) in result.warnings.iter().enumerate() {
        let event = paint_annotation_warning_to_progress_event(
            warning,
            String::new(),
            String::from("com.host.slice-postprocess-paint-annotator"),
            i as u64,
        );
        sink.record(event);
    }

    // Put the (possibly annotated) SliceIR back so downstream per-layer
    // stages can still read it via `arena.slice()`.
    arena
        .set_slice(result.slice_ir)
        .map_err(|_| LayerExecutionError::FatalLayer {
            layer_index: layer.index,
            stage_id: "Layer::SlicePostProcess".to_string(),
            module_id: "<host-built-in-paint-annotator>".to_string(),
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
pub(crate) fn assemble_ordered_entities(
    global_layer_index: u32,
    perimeter: Option<&PerimeterIR>,
    infill: Option<&InfillIR>,
    support: Option<&SupportIR>,
) -> Vec<PrintEntity> {
    let mut out: Vec<PrintEntity> = Vec::new();
    let push = |path: slicer_ir::ExtrusionPath3D,
                role: slicer_ir::ExtrusionRole,
                key: RegionKey,
                acc: &mut Vec<PrintEntity>| {
        let topo_order = acc.len() as u32;
        acc.push(PrintEntity {
            path,
            role,
            region_key: key,
            topo_order,
        });
    };

    if let Some(perim) = perimeter {
        for region in &perim.regions {
            let key = RegionKey {
                global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
            };
            for wl in &region.walls {
                let role = wl.path.role.clone();
                push(wl.path.clone(), role, key.clone(), &mut out);
            }
        }
    }

    if let Some(inf) = infill {
        for region in &inf.regions {
            let key = RegionKey {
                global_layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
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

#[cfg(test)]
mod tests {
    use super::*;

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
