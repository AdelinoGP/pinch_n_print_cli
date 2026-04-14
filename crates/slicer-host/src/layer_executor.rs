//! Per-layer parallel executor contracts (TASK-031).
//!
//! This module defines the per-layer parallel execution contracts for running
//! all Tier-2 layer stages using rayon. Each layer gets its own `LayerArena`
//! for intermediate IR storage. Stages within each layer run sequentially,
//! but layers can be processed in parallel.

use std::fmt;

use rayon::prelude::*;
use slicer_ir::{
    GlobalLayer, InfillIR, LayerCollectionIR, ModuleId, PerimeterIR, PrintEntity, RegionKey,
    SemVer, StageId, SupportIR,
};

use crate::layer_slice::{execute_layer_slice, LayerSliceError};
use crate::{
    Blackboard, BlackboardError, CompiledModule, ExecutionPlan, LayerArena, LayerArenaError,
};

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
        }
    }
}

impl std::error::Error for LayerExecutionError {}

/// Callback surface used by tests and future runtime bindings for layer stage execution.
pub trait LayerStageRunner {
    /// Execute one compiled layer module against the current layer state.
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<LayerStageOutput, LayerStageError>;
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
    let global_layers = &plan.global_layers;

    // Process layers in parallel using rayon.
    // collect() preserves the original item order, matching global_layers index order.
    global_layers
        .par_iter()
        .map(|layer| execute_single_layer(plan, blackboard, runner, layer))
        .collect()
}

/// Execute all stages for a single layer sequentially.
fn execute_single_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    layer: &GlobalLayer,
) -> Result<LayerCollectionIR, LayerExecutionError> {
    // Create an isolated LayerArena for this layer
    let mut arena = LayerArena::new();

    // Host-built-in Layer::Slice (docs/04 §Full Lifecycle): commit a
    // `SliceIR` produced from the mesh before any user Layer::Slice /
    // Layer::SlicePostProcess module runs. Skipped if a caller has already
    // pre-seeded a slice (e.g. integration tests).
    if arena.slice().is_none() {
        let slice_ir = execute_layer_slice(blackboard.mesh().as_ref(), layer).map_err(
            |source| LayerExecutionError::LayerSlice {
                layer_index: layer.index,
                source,
            },
        )?;
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
    for stage in &plan.per_layer_stages {
        if stage.stage_id == "Layer::PathOptimization" && arena.layer_collection().is_none() {
            let ordered_entities = assemble_ordered_entities(
                layer.index,
                arena.perimeter(),
                arena.infill(),
                arena.support(),
            );
            arena.set_layer_collection(LayerCollectionIR {
                schema_version: SemVer { major: 1, minor: 0, patch: 0 },
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
            let result = runner.run_stage(&stage.stage_id, layer, module, blackboard, &mut arena);

            match result {
                Ok(LayerStageOutput::Success) => {
                    // Module completed successfully, continue
                }
                Ok(LayerStageOutput::NonFatalError { message: _ }) => {
                    // Non-fatal error: log but continue with next module
                }
                Err(LayerStageError::FatalModule {
                    stage_id,
                    module_id,
                    message,
                }) => {
                    // Fatal error: abort this layer immediately
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id,
                        module_id,
                        message,
                    });
                }
                Err(LayerStageError::ArenaCommit { source: _ }) => {
                    // Arena commit failure: treat as fatal for this layer
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id.clone(),
                        message: String::from("arena commit failed"),
                    });
                }
            }
        }
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
            schema_version: SemVer { major: 1, minor: 0, patch: 0 },
            global_layer_index: layer.index,
            z: layer.z,
            ordered_entities,
            tool_changes: Vec::new(),
            z_hops: Vec::new(),
            annotations: Vec::new(),
        }
    });
    layer_output.tool_changes.extend(arena.take_deferred_tool_changes());
    layer_output.annotations.extend(arena.take_deferred_annotations());
    layer_output.z_hops.extend(arena.take_deferred_z_hops());
    Ok(layer_output)
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
    let push = |path: slicer_ir::ExtrusionPath3D, role: slicer_ir::ExtrusionRole, key: RegionKey, acc: &mut Vec<PrintEntity>| {
        let topo_order = acc.len() as u32;
        acc.push(PrintEntity { path, role, region_key: key, topo_order });
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
