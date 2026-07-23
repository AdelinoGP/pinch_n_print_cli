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
    /// Lightning tree-edge segments produced by `PrePass::LightningTreeGen`.
    /// Packet 137 lands the seam; the algorithm ships in 138/139. Skipped
    /// (slot stays `None`) when no region's `sparse_fill_holder` is
    /// `lightning-infill` — see ADR-0029.
    LightningTreeIR,
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
            Self::LightningTreeIR => "lightning-tree-ir",
        };

        f.write_str(name)
    }
}

/// Structured blackboard contract failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlackboardError {
    /// Lightning tree generation failed before its output could be committed.
    LightningTreeGeneration {
        /// Human-readable generation failure detail.
        message: String,
    },
    /// A prepass output was committed more than once.
    DuplicatePrepassCommit {
        /// The duplicated prepass slot.
        slot: BlackboardPrepassSlot,
    },
    /// A `SeamPlanIR` was committed with two entries sharing the same
    /// full `RegionKey` (including `variant_chain`). Rejected at commit
    /// time (packet 178 AC-N1) so the host never silently shadows a
    /// region's plan during per-layer injection.
    DuplicateSeamPlanEntry {
        /// The first duplicate `RegionKey` encountered, in entry order.
        region_key: crate::slice_ir::RegionKey,
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
            Self::LightningTreeGeneration { message } => {
                write!(f, "lightning tree generation failed: {message}")
            }
            Self::DuplicatePrepassCommit { slot } => {
                write!(f, "prepass output already committed for {slot}")
            }
            Self::DuplicateSeamPlanEntry { region_key } => {
                write!(
                    f,
                    "SeamPlanIR contains duplicate entries for RegionKey (layer={}, object='{}', region={}, variant_chain={:?})",
                    region_key.global_layer_index, region_key.object_id, region_key.region_id, region_key.variant_chain
                )
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
// Diagnostic types (prepass diagnostic channel, ADR-0010)
// ============================================================================

/// Severity level for prepass diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Trace-level diagnostic, most verbose.
    Trace,
    /// Debug-level diagnostic, verbose.
    Debug,
    /// Informational diagnostic.
    Info,
    /// Warning diagnostic; recoverable, does not abort the slice.
    Warn,
    /// Error diagnostic; recoverable, does not abort the slice.
    Error,
}

/// A typed diagnostic record emitted by a prepass module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Numeric code per diagnostic class; module-allocated (e.g. support-planner 1000-1999).
    pub code: u32,
    /// Global layer index when the diagnostic is layer-scoped; `None` for prepass-global diagnostics. Signed to allow negative raft prefix layer indices.
    pub layer: Option<i32>,
    /// Object identifier when the diagnostic is object-scoped; `None` for object-agnostic diagnostics.
    pub object_id: Option<String>,
    /// Human-readable description; includes parameters that don't fit the fixed fields.
    pub message: String,
}

// ============================================================================
// Per-stage layer commit (ADR-0020)
// ============================================================================
//
// `LayerStageCommit` is the deep replacement for the passive `LayerStageCommitData`
// value-bag: a flat per-stage enum mirroring `slicer-schema::STAGES`. The runtime's
// `apply` consumes exactly one variant per module invocation, making illegal
// `(stage, output)` pairings unrepresentable and the per-stage commit protocol a
// compiler-checked exhaustive match. The producer (`deconstruct_layer_ctx` in
// `slicer-wasm-host`) builds the enum directly. See ADR-0020 for the rationale.

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
    /// The canonical per-layer `StageId` this commit belongs to, matching the
    /// corresponding row in `slicer-schema::STAGES`. `None` for the test-only
    /// `SeedLayerCollection`, which has no production stage.
    ///
    /// The non-`None` set is exactly the eight `world-layer` stages — a property
    /// pinned by a meta-test so the enum and `STAGES` cannot drift (ADR-0020).
    pub fn stage_id(&self) -> Option<&'static str> {
        Some(match self {
            Self::Perimeters(_) => "Layer::Perimeters",
            Self::PerimetersPostProcess(_) => "Layer::PerimetersPostProcess",
            Self::Infill(_) => "Layer::Infill",
            Self::InfillPostProcess(_) => "Layer::InfillPostProcess",
            Self::Support(_) => "Layer::Support",
            Self::SupportPostProcess(_) => "Layer::SupportPostProcess",
            Self::SlicePostProcess { .. } => "Layer::SlicePostProcess",
            Self::PathOptimization(_) => "Layer::PathOptimization",
            Self::SeedLayerCollection(_) => return None,
        })
    }
}
