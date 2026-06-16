//! Stage I/O types shared between slicer-runtime executors and slicer-wasm-host.
//!
//! These types are the contract surface for per-layer, finalization, and
//! postpass stage outputs and errors. They live here (slicer-ir) so that
//! slicer-wasm-host can reference them without a back-edge dependency on
//! slicer-runtime.

use std::fmt;

use crate::{ModuleId, StageId};

// ============================================================================
// Layer arena error types
// ============================================================================

/// Named intermediate slots in the per-layer arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerArenaSlot {
    /// Temporary `SliceIR` output.
    Slice,
    /// Temporary `PerimeterIR` output.
    Perimeter,
    /// Temporary `InfillIR` output.
    Infill,
    /// Temporary `SupportIR` output.
    Support,
}

impl fmt::Display for LayerArenaSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Slice => "slice",
            Self::Perimeter => "perimeter",
            Self::Infill => "infill",
            Self::Support => "support",
        };

        f.write_str(name)
    }
}

/// Structured layer-arena contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerArenaError {
    /// A staged IR was inserted into an occupied arena slot.
    SlotAlreadyOccupied {
        /// The occupied arena slot.
        slot: LayerArenaSlot,
    },
}

impl fmt::Display for LayerArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SlotAlreadyOccupied { slot } => {
                write!(f, "layer arena slot already occupied: {slot}")
            }
        }
    }
}

impl std::error::Error for LayerArenaError {}

// ============================================================================
// Layer stage I/O types
// ============================================================================

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

// ============================================================================
// Finalization stage I/O types
// ============================================================================

/// Output produced by a single layer finalization module invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum FinalizationOutput {
    /// Module completed successfully.
    Success,
    /// Module encountered a non-fatal error, continue with next module.
    NonFatalError {
        /// Stable human-readable detail.
        message: String,
    },
}

/// Fatal error from a layer finalization module or executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizationError {
    /// Fatal error, abort finalization.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// Validation error (e.g. non-monotonic layer indices).
    Validation {
        /// Stable human-readable detail.
        message: String,
    },
}

impl fmt::Display for FinalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal finalization module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::Validation { message } => write!(f, "finalization validation failed: {message}"),
        }
    }
}

impl std::error::Error for FinalizationError {}

// ============================================================================
// Postpass stage I/O types
// ============================================================================

/// Output produced by a single postpass module invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum PostpassOutput {
    /// GCodePostProcess module completed successfully.
    GCodeSuccess,
    /// TextPostProcess module completed successfully, returning the final text.
    TextSuccess {
        /// The final G-code text produced by the module.
        text: String,
    },
    /// Module encountered a non-fatal error, continue with next module.
    NonFatalError {
        /// Stable human-readable detail.
        message: String,
    },
}

/// Fatal error from a postpass module or executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostpassError {
    /// Fatal error from a module, abort postpass.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// GCode emit failed.
    GCodeEmit {
        /// Stable human-readable detail.
        message: String,
    },
    /// GCode serialization failed.
    GCodeSerialization {
        /// Stable human-readable detail.
        message: String,
    },
    /// A ToolChange was emitted without surrounding retract/prime entities while
    /// `wipe_tower_enabled` is true.
    MissingToolchangePurge {
        /// Layer index (global) where the bare ToolChange was detected.
        layer_index: u32,
        /// Index of the ToolChange within `layer.tool_changes` (0-based).
        tool_change_index: u32,
    },
}

impl fmt::Display for PostpassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal postpass module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::GCodeEmit { message } => write!(f, "gcode emit failed: {message}"),
            Self::GCodeSerialization { message } => {
                write!(f, "gcode serialization failed: {message}")
            }
            Self::MissingToolchangePurge {
                layer_index,
                tool_change_index,
            } => write!(
                f,
                "missing toolchange purge: layer {layer_index} tool_change[{tool_change_index}] \
                 has no ExtrusionRole::WipeTower entity after the tool change; \
                 ensure wipe-tower module runs before gcode emit"
            ),
        }
    }
}

impl std::error::Error for PostpassError {}

// ============================================================================
// Blackboard error types
// ============================================================================

/// Named prepass storage locations inside the blackboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackboardPrepassSlot {
    /// Surface classification produced by `PrePass::MeshAnalysis`.
    SurfaceClassification,
    /// Layer plan produced by `PrePass::LayerPlanning`.
    LayerPlan,
    /// Seam plan produced by `PrePass::SeamPlanning`.
    SeamPlan,
    /// Support plan produced by `PrePass::SupportGeometry`.
    SupportPlan,
    /// Region map produced by `PrePass::RegionMapping`.
    RegionMap,
    /// Per-global-layer `SliceIR` produced by `PrePass::Slice` and refined by
    /// `PrePass::ShellClassification`.
    SliceIR,
    /// Support geometry coarse outlines produced by `PrePass::SupportGeometry`.
    SupportGeometry,
}

impl fmt::Display for BlackboardPrepassSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::SurfaceClassification => "surface-classification",
            Self::LayerPlan => "layer-plan",
            Self::SeamPlan => "seam-plan",
            Self::SupportPlan => "support-plan",
            Self::RegionMap => "region-map",
            Self::SliceIR => "slice-ir",
            Self::SupportGeometry => "support-geometry",
        };

        f.write_str(name)
    }
}

/// Structured blackboard contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlackboardError {
    /// A prepass output was committed more than once.
    DuplicatePrepassCommit {
        /// The duplicated prepass slot.
        slot: BlackboardPrepassSlot,
    },
    /// A requested prepass output has not been committed yet.
    MissingRequiredPrepass {
        /// The missing prepass slot.
        slot: BlackboardPrepassSlot,
    },
    /// A per-layer output was committed more than once.
    DuplicateLayerCommit {
        /// The duplicated layer slot index.
        layer_index: usize,
    },
    /// A per-layer output index was outside the configured slot range.
    LayerSlotOutOfRange {
        /// The out-of-range layer index.
        layer_index: usize,
        /// The configured slot count.
        layer_count: usize,
    },
    /// Draining was attempted before every layer slot was populated.
    IncompleteLayerDrain {
        /// Slot indexes that are still empty.
        missing_indices: Vec<usize>,
    },
    /// Layer outputs were already drained once.
    LayerOutputsAlreadyDrained,
}

impl fmt::Display for BlackboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePrepassCommit { slot } => {
                write!(f, "prepass output already committed for {slot}")
            }
            Self::MissingRequiredPrepass { slot } => {
                write!(f, "required prepass output missing for {slot}")
            }
            Self::DuplicateLayerCommit { layer_index } => {
                write!(f, "layer output already committed for slot {layer_index}")
            }
            Self::LayerSlotOutOfRange {
                layer_index,
                layer_count,
            } => write!(
                f,
                "layer output slot {layer_index} is out of range for {layer_count} slots"
            ),
            Self::IncompleteLayerDrain { missing_indices } => write!(
                f,
                "cannot drain layer outputs while slots are missing: {missing_indices:?}"
            ),
            Self::LayerOutputsAlreadyDrained => {
                write!(f, "layer outputs have already been drained")
            }
        }
    }
}

impl std::error::Error for BlackboardError {}

// ============================================================================
// Prepass runner error types
// ============================================================================

/// Narrow runner-side error returned by the WASM-host `PrepassStageRunner` trait impl.
///
/// Mirrors the P86 `GCodeEmitError → PostpassError` idiom: the wasm dispatcher constructs
/// only the variants here, and `slicer-runtime` provides a `From<PrepassRunnerError> for
/// PrepassExecutionError` impl so the orchestrator's `?` lifts the narrow error into the
/// broader `PrepassExecutionError` (which retains its 7 built-in-producer variants).
/// See P83 design.md "Narrow runner errors".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepassRunnerError {
    /// A module returned a fatal error during prepass execution.
    FatalModule {
        /// Stage that was executing when the error occurred.
        stage_id: StageId,
        /// Module that produced the fatal error.
        module_id: ModuleId,
        /// Human-readable error detail.
        message: String,
    },
    /// A blackboard commit contract failed during prepass execution.
    Blackboard {
        /// Stage that was executing when the commit failed.
        stage_id: StageId,
        /// Module whose commit failed.
        module_id: ModuleId,
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
}

impl fmt::Display for PrepassRunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(f, "fatal module error in {stage_id}/{module_id}: {message}"),
            Self::Blackboard {
                stage_id,
                module_id,
                source,
            } => write!(f, "blackboard error in {stage_id}/{module_id}: {source}"),
        }
    }
}

impl std::error::Error for PrepassRunnerError {}

// ============================================================================
// Layer stage commit data (P83 symmetric IR-typed trait boundary)
// ============================================================================

/// IR-typed commit data returned by `LayerStageRunner::run_stage` in `slicer-wasm-host`.
///
/// The wasm-host runner impl deconstructs its internal `HostExecutionContext`
/// into this struct before returning, so the runtime-side `commit_layer_outputs`
/// (which moves into `crates/slicer-runtime/src/layer_executor.rs` in P83 Step 4d)
/// consumes only plain IR values and never sees the wasm-host-internal
/// `HostExecutionContext`. See packet 83 design.md "Symmetric IR-typed trait boundary".
///
/// All fields default to empty / `None` — stages that do not produce a given output
/// leave the corresponding field at its default. `commit_layer_outputs` in
/// `layer_executor.rs` interprets empty collections as "no output" for that output class,
/// exactly as the original `ctx.*` emptiness checks did.
///
/// # Travel-move staging note
///
/// `deferred_travel_moves` stores moves as `(anchor_entity_index, x, y, z, feed_rate)` where
/// `anchor_entity_index` is a `u32` index into `LayerCollectionIR::ordered_entities` at the
/// time the move is resolved. This is the pre-resolved form; `layer_executor.rs` converts
/// each entry to `slicer_ir::TravelMove` (keyed by `entity_id: u64`) during the arena-commit
/// pass. There is no `slicer-ir` type with an `after_entity_index: u32` key today — a
/// dedicated `DeferredTravelMove` IR type may be introduced in a later sub-step if the
/// tuple representation becomes unwieldy.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerStageCommitData {
    /// Converted `InfillIR` from the guest's infill-output-builder.
    ///
    /// `None` means the guest produced no infill output (all path lists empty).
    /// Corresponds to `Layer::Infill` and `Layer::InfillPostProcess`.
    pub infill_output: Option<crate::InfillIR>,

    /// Converted `SupportIR` from the guest's support-output-builder.
    ///
    /// `None` means the guest produced no support output.
    /// Corresponds to `Layer::Support` and `Layer::SupportPostProcess`.
    pub support_output: Option<crate::SupportIR>,

    /// Converted `PerimeterIR` from the guest's perimeter-output-builder.
    ///
    /// `None` means the guest produced no perimeter output.
    /// Corresponds to `Layer::Perimeters` and `Layer::PerimetersPostProcess`.
    pub perimeter_output: Option<crate::PerimeterIR>,

    /// Per-region polygon updates from a `Layer::SlicePostProcess` module.
    ///
    /// Each entry is `(region_key, replacement_polygons)`. Empty means no
    /// polygon updates. Consumed by `merge_slice_postprocess_into` in `layer_executor.rs`.
    pub slice_polygon_updates: Vec<(crate::RegionKey, Vec<crate::ExPolygon>)>,

    /// Per-region path-Z updates from a `Layer::SlicePostProcess` module.
    ///
    /// Each entry is `(region_key, path_idx, vertex_idx, new_z)`. Empty means
    /// no Z updates. Consumed by `merge_slice_postprocess_into` in `layer_executor.rs`.
    pub slice_path_z_updates: Vec<(crate::RegionKey, u32, u32, f32)>,

    /// Tool-change commands emitted by a `Layer::PathOptimization` module.
    pub tool_changes: Vec<crate::ToolChange>,

    /// Z-hop requests emitted by a `Layer::PathOptimization` module.
    pub z_hops: Vec<crate::ZHop>,

    /// Comment / raw G-code annotations emitted by a `Layer::PathOptimization` module.
    ///
    /// Each `LayerAnnotation` carries its own `after_entity_index` anchor (set to the
    /// `anchor` computed from `LayerCollectionIR::ordered_entities` at dispatch time).
    pub annotations: Vec<crate::LayerAnnotation>,

    /// Retract / unretract decisions emitted by a `Layer::PathOptimization` module.
    ///
    /// Uses `slicer_ir::TravelRetract`, which is field-for-field isomorphic with
    /// `slicer_runtime::blackboard::DeferredRetract`. `layer_executor.rs` pushes
    /// entries directly onto the per-layer arena's deferred-retract queue.
    pub retracts: Vec<crate::TravelRetract>,

    /// Pre-resolved travel-move requests emitted by a `Layer::PathOptimization` module.
    ///
    /// Stored as `(anchor_entity_index, x, y, z, feed_rate)` tuples. The `u32`
    /// anchor is an index into `LayerCollectionIR::ordered_entities`; `layer_executor.rs`
    /// resolves it to `entity_id: u64` when converting to `slicer_ir::TravelMove`.
    /// Using a tuple here avoids introducing a new `slicer-ir` sub-type before the
    /// planner has decided whether to promote `DeferredTravelMove` into `slicer-ir`
    /// (see packet 83 design.md staging notes).
    pub deferred_travel_moves: Vec<(u32, Option<f32>, Option<f32>, Option<f32>, Option<f32>)>,

    /// Pre-seeded `LayerCollectionIR` to place in the arena before the next stage runs.
    ///
    /// This field exists primarily as a test escape hatch: mock `LayerStageRunner`
    /// impls that need to inject a specific `LayerCollectionIR` (e.g. with custom
    /// `tool_index` per entity) can populate this field so the executor commits it
    /// to the arena, bypassing the automatic `assemble_ordered_entities` fallback.
    ///
    /// Production WASM runners leave this `None`.
    pub layer_collection_output: Option<crate::LayerCollectionIR>,

    /// Entity-order proposal from a `Layer::PathOptimization` guest's `set-entity-order` call.
    ///
    /// Each entry is `(entity_index: u32, reverse: bool)`. `None` means the guest did not
    /// call `set-entity-order`. When `Some`, `layer_executor.rs` applies this via
    /// `apply_entity_order_proposal` BEFORE committing the PathOptimization GCode outputs.
    ///
    /// Corresponds to `HostExecutionContext::layer_collection_proposal` in slicer-wasm-host.
    pub entity_order_proposal: Option<Vec<(u32, bool)>>,

    /// Whether this commit data carries a post-commit seam injection for `Layer::Perimeters`.
    ///
    /// When `true`, `layer_executor.rs` must inject seam from the `SeamPlanIR` into the
    /// committed `PerimeterIR` in the arena. This mirrors the post-`commit_layer_outputs`
    /// seam injection in the original `dispatch.rs` `LayerStageRunner::run_stage` body.
    ///
    /// Always `false` for stages other than `Layer::Perimeters`. WASM runner sets this
    /// to `true` whenever a perimeter was committed for the `Layer::Perimeters` stage.
    pub needs_seam_injection: bool,
}

// ============================================================================
// Per-stage layer commit (ADR-0020)
// ============================================================================
//
// `LayerStageCommit` is the deep replacement for the passive `LayerStageCommitData`
// value-bag: a flat per-stage enum mirroring `slicer-schema::STAGES`. The runtime's
// `apply` consumes exactly one variant per module invocation, making illegal
// `(stage, output)` pairings unrepresentable and the per-stage commit protocol a
// compiler-checked exhaustive match. See ADR-0020 for the full rationale.
//
// During the staged migration (ADR-0020 Steps A/B), `from_legacy` bridges the
// still-struct-returning runner into the enum so the consumer can route through
// `apply` without churning every mock. The bridge is deleted in Step C when the
// producer (`deconstruct_layer_ctx`) builds the enum directly.

/// Anchor-less retract / unretract spec emitted by a `Layer::PathOptimization`
/// module. The entity anchor is resolved by the runtime's `apply` from arena
/// state — never carried here — so no placeholder index can leak (ADR-0020).
#[derive(Debug, Clone, PartialEq)]
pub struct RetractSpec {
    /// Retraction length in mm.
    pub length: f32,
    /// Retraction speed in mm/s.
    pub speed: f32,
    /// `true` = Unretract; `false` = Retract.
    pub is_unretract: bool,
    /// Emit-mode (`Gcode` inline-E vs `Firmware` `G10`/`G11`).
    pub mode: crate::RetractMode,
}

/// Anchor-less travel-move destination emitted by a `Layer::PathOptimization`
/// module. The anchor is resolved by `apply`; only the destination is carried.
#[derive(Debug, Clone, PartialEq)]
pub struct TravelMoveDest {
    /// X destination (module coordinate units, 100 nm).
    pub x: Option<f32>,
    /// Y destination (module coordinate units, 100 nm).
    pub y: Option<f32>,
    /// Z destination (module coordinate units, 100 nm).
    pub z: Option<f32>,
    /// Feed-rate override in mm/s (`None` = keep current speed).
    pub f: Option<f32>,
}

/// G-code side-effects emitted by a `Layer::PathOptimization` module.
///
/// `tool_changes` carry their own guest-provided `after_entity_index` (genuine
/// per-command anchoring). The four end-of-layer groups carry **no** anchor —
/// `apply` stamps `ordered_entities.len()-1` from arena state. This is the
/// structural fix for the placeholder-`0` anchor bug (ADR-0020): there is no
/// field to hold a lie.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PathOptimizationCommit {
    /// Tool-change commands; each keeps its guest `after_entity_index`.
    pub tool_changes: Vec<crate::ToolChange>,
    /// Z-hop heights (mm); anchored at end-of-layer by `apply`.
    pub z_hops: Vec<f32>,
    /// Comment / raw annotations; anchored at end-of-layer by `apply`.
    pub annotations: Vec<crate::LayerAnnotationKind>,
    /// Retract / unretract decisions; anchored at end-of-layer by `apply`.
    pub retracts: Vec<RetractSpec>,
    /// Travel-move destinations; anchored at end-of-layer by `apply`.
    pub travel_moves: Vec<TravelMoveDest>,
    /// `set-entity-order` proposal `(index, reverse)`; applied before the
    /// g-code groups are anchored. `None` = guest did not reorder.
    pub order_proposal: Option<Vec<(u32, bool)>>,
}

/// One module invocation's committed output, keyed by stage (ADR-0020).
///
/// Exactly one variant per per-layer stage in `slicer-schema::STAGES`, plus the
/// documented test-only `SeedLayerCollection`. `run_stage` returns
/// `Option<LayerStageCommit>`; `None` means the invocation committed nothing
/// (empty guest output or a missing component).
#[derive(Debug, Clone, PartialEq)]
pub enum LayerStageCommit {
    /// `Layer::Perimeters`: replace the arena perimeter slot, partition fill
    /// polygons, then back-fill `resolved_seam` from the seam plan.
    Perimeters(crate::PerimeterIR),
    /// `Layer::PerimetersPostProcess`: reconcile against the existing perimeter
    /// (preserve `infill_areas`/`seam_candidates`/`resolved_seam` by region key),
    /// then re-partition. Carries `None` when the post-process emitted no
    /// perimeter of its own (the existing perimeter is re-partitioned in place).
    PerimetersPostProcess(Option<crate::PerimeterIR>),
    /// `Layer::Infill`: merge per-region paths into the arena infill slot.
    Infill(crate::InfillIR),
    /// `Layer::InfillPostProcess`: replace the arena infill slot.
    InfillPostProcess(crate::InfillIR),
    /// `Layer::Support`: set the arena support slot.
    Support(crate::SupportIR),
    /// `Layer::SupportPostProcess`: replace the arena support slot.
    SupportPostProcess(crate::SupportIR),
    /// `Layer::SlicePostProcess`: mutate the existing arena `SliceIR` in place.
    SlicePostProcess {
        /// Per-region polygon replacements `(region_key, replacement_polygons)`.
        polygon_updates: Vec<(crate::RegionKey, Vec<crate::ExPolygon>)>,
        /// Per-region path-Z updates `(region_key, path_idx, vertex_idx, new_z)`.
        path_z_updates: Vec<(crate::RegionKey, u32, u32, f32)>,
    },
    /// `Layer::PathOptimization`: apply the entity-order proposal, then
    /// accumulate the g-code side-effects onto the deferred queues.
    PathOptimization(PathOptimizationCommit),
    /// Test-only escape hatch: pre-seed a `LayerCollectionIR` into the arena so a
    /// downstream stage consumes a known entity list. Named for its arena effect,
    /// not its caller; never produced by a production runner. See ADR-0020.
    SeedLayerCollection(crate::LayerCollectionIR),
}

impl LayerStageCommit {
    /// Bridge a legacy `LayerStageCommitData` + `stage_id` into the per-stage
    /// enum (ADR-0020 Steps A/B). Returns `None` when the invocation committed
    /// nothing, preserving the early-return semantics of the original
    /// `commit_layer_outputs`. Deleted in Step C once the producer builds the
    /// enum directly.
    ///
    /// `SeedLayerCollection` takes priority: the sole caller sets
    /// `layer_collection_output` *instead of* real stage output, never alongside
    /// it, so prioritising it here loses nothing.
    pub fn from_legacy(stage_id: &str, data: LayerStageCommitData) -> Option<Self> {
        if let Some(lc) = data.layer_collection_output {
            return Some(Self::SeedLayerCollection(lc));
        }
        match stage_id {
            "Layer::Perimeters" => data.perimeter_output.map(Self::Perimeters),
            // Post-process always commits for its stage: even an empty output
            // re-partitions the existing perimeter (original commit_layer_outputs
            // ran the arm unconditionally for the stage's modules).
            "Layer::PerimetersPostProcess" => {
                Some(Self::PerimetersPostProcess(data.perimeter_output))
            }
            "Layer::Infill" => data.infill_output.map(Self::Infill),
            "Layer::InfillPostProcess" => data.infill_output.map(Self::InfillPostProcess),
            "Layer::Support" => data.support_output.map(Self::Support),
            "Layer::SupportPostProcess" => data.support_output.map(Self::SupportPostProcess),
            "Layer::SlicePostProcess" => {
                if data.slice_polygon_updates.is_empty() && data.slice_path_z_updates.is_empty() {
                    None
                } else {
                    Some(Self::SlicePostProcess {
                        polygon_updates: data.slice_polygon_updates,
                        path_z_updates: data.slice_path_z_updates,
                    })
                }
            }
            "Layer::PathOptimization" => {
                let commit = PathOptimizationCommit {
                    tool_changes: data.tool_changes,
                    z_hops: data.z_hops.into_iter().map(|z| z.hop_height).collect(),
                    annotations: data.annotations.into_iter().map(|a| a.kind).collect(),
                    retracts: data
                        .retracts
                        .into_iter()
                        .map(|r| RetractSpec {
                            length: r.length,
                            speed: r.speed,
                            is_unretract: r.is_unretract,
                            mode: r.mode,
                        })
                        .collect(),
                    travel_moves: data
                        .deferred_travel_moves
                        .into_iter()
                        .map(|(_anchor, x, y, z, f)| TravelMoveDest { x, y, z, f })
                        .collect(),
                    order_proposal: data.entity_order_proposal,
                };
                Some(Self::PathOptimization(commit))
            }
            _ => None,
        }
    }
}
