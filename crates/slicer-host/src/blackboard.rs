//! Host-owned blackboard and per-layer arena contracts.
//!
//! This module defines the TASK-026 public API surface only. The runtime
//! behavior is intentionally left as a red-contract stub for later
//! implementation.

use std::fmt;
use std::sync::Arc;

use slicer_ir::{
    InfillIR, LayerCollectionIR, LayerPlanIR, MeshIR, PaintRegionIR, PerimeterIR, RegionMapIR,
    SliceIR, SupportIR, SurfaceClassificationIR,
};

/// Host-owned immutable global IR store plus write-once per-layer output slots.
#[derive(Debug, Default)]
pub struct Blackboard {
    _private: (),
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

/// Named prepass storage locations inside the blackboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlackboardPrepassSlot {
    /// Surface classification produced by `PrePass::MeshAnalysis`.
    SurfaceClassification,
    /// Layer plan produced by `PrePass::LayerPlanning`.
    LayerPlan,
    /// Paint regions produced by `PrePass::PaintSegmentation`.
    PaintRegions,
    /// Region map produced by `PrePass::RegionMapping`.
    RegionMap,
}

impl fmt::Display for BlackboardPrepassSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::SurfaceClassification => "surface-classification",
            Self::LayerPlan => "layer-plan",
            Self::PaintRegions => "paint-regions",
            Self::RegionMap => "region-map",
        };

        f.write_str(name)
    }
}

impl Blackboard {
    /// Create a blackboard around a host-owned mesh and fixed per-layer slot count.
    #[must_use]
    pub fn new(_mesh_ir: Arc<MeshIR>, _layer_count: usize) -> Self {
        task_026_todo()
    }

    /// Return the host-owned mesh as an `Arc`-backed shared reference.
    #[must_use]
    pub fn mesh(&self) -> &Arc<MeshIR> {
        task_026_todo()
    }

    /// Commit `SurfaceClassificationIR` exactly once.
    pub fn commit_surface_classification(
        &mut self,
        _ir: Arc<SurfaceClassificationIR>,
    ) -> Result<(), BlackboardError> {
        task_026_todo()
    }

    /// Return the committed surface classification, if available.
    #[must_use]
    pub fn surface_classification(&self) -> Option<&Arc<SurfaceClassificationIR>> {
        task_026_todo()
    }

    /// Commit `LayerPlanIR` exactly once.
    pub fn commit_layer_plan(&mut self, _ir: Arc<LayerPlanIR>) -> Result<(), BlackboardError> {
        task_026_todo()
    }

    /// Return the committed layer plan, if available.
    #[must_use]
    pub fn layer_plan(&self) -> Option<&Arc<LayerPlanIR>> {
        task_026_todo()
    }

    /// Commit `PaintRegionIR` exactly once.
    pub fn commit_paint_regions(&mut self, _ir: Arc<PaintRegionIR>) -> Result<(), BlackboardError> {
        task_026_todo()
    }

    /// Return the committed paint regions, if available.
    #[must_use]
    pub fn paint_regions(&self) -> Option<&Arc<PaintRegionIR>> {
        task_026_todo()
    }

    /// Commit `RegionMapIR` exactly once.
    pub fn commit_region_map(&mut self, _ir: Arc<RegionMapIR>) -> Result<(), BlackboardError> {
        task_026_todo()
    }

    /// Return the committed region map, if available.
    #[must_use]
    pub fn region_map(&self) -> Option<&Arc<RegionMapIR>> {
        task_026_todo()
    }

    /// Commit one `LayerCollectionIR` into its write-once layer slot.
    pub fn commit_layer_output(
        &mut self,
        _layer_index: usize,
        _ir: LayerCollectionIR,
    ) -> Result<(), BlackboardError> {
        task_026_todo()
    }

    /// Drain all committed layer outputs exactly once after the layer loop.
    pub fn drain_layer_outputs(&mut self) -> Result<Vec<LayerCollectionIR>, BlackboardError> {
        task_026_todo()
    }
}

/// Ephemeral per-layer intermediate IR ownership used during one layer worker run.
#[derive(Debug, Default)]
pub struct LayerArena {
    _private: (),
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

impl LayerArena {
    /// Create a fresh empty per-layer arena.
    #[must_use]
    pub fn new() -> Self {
        task_026_todo()
    }

    /// Stage `SliceIR` in the arena.
    pub fn set_slice(&mut self, _ir: SliceIR) -> Result<(), LayerArenaError> {
        task_026_todo()
    }

    /// Borrow the staged `SliceIR`, if present.
    #[must_use]
    pub fn slice(&self) -> Option<&SliceIR> {
        task_026_todo()
    }

    /// Take ownership of the staged `SliceIR`, if present.
    pub fn take_slice(&mut self) -> Option<SliceIR> {
        task_026_todo()
    }

    /// Stage `PerimeterIR` in the arena.
    pub fn set_perimeter(&mut self, _ir: PerimeterIR) -> Result<(), LayerArenaError> {
        task_026_todo()
    }

    /// Borrow the staged `PerimeterIR`, if present.
    #[must_use]
    pub fn perimeter(&self) -> Option<&PerimeterIR> {
        task_026_todo()
    }

    /// Take ownership of the staged `PerimeterIR`, if present.
    pub fn take_perimeter(&mut self) -> Option<PerimeterIR> {
        task_026_todo()
    }

    /// Stage `InfillIR` in the arena.
    pub fn set_infill(&mut self, _ir: InfillIR) -> Result<(), LayerArenaError> {
        task_026_todo()
    }

    /// Borrow the staged `InfillIR`, if present.
    #[must_use]
    pub fn infill(&self) -> Option<&InfillIR> {
        task_026_todo()
    }

    /// Take ownership of the staged `InfillIR`, if present.
    pub fn take_infill(&mut self) -> Option<InfillIR> {
        task_026_todo()
    }

    /// Stage `SupportIR` in the arena.
    pub fn set_support(&mut self, _ir: SupportIR) -> Result<(), LayerArenaError> {
        task_026_todo()
    }

    /// Borrow the staged `SupportIR`, if present.
    #[must_use]
    pub fn support(&self) -> Option<&SupportIR> {
        task_026_todo()
    }

    /// Take ownership of the staged `SupportIR`, if present.
    pub fn take_support(&mut self) -> Option<SupportIR> {
        task_026_todo()
    }

    /// Drop all staged per-layer intermediates before finalization/postpass.
    pub fn reset(&mut self) {
        task_026_todo()
    }
}

fn task_026_todo<T>() -> T {
    todo!("TASK-026: implement Blackboard + LayerArena")
}
