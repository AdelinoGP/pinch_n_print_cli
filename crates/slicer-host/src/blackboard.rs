//! Host-owned blackboard and per-layer arena contracts.
//!
//! This module defines the TASK-026 public API surface and minimal runtime
//! behavior for blackboard and layer-arena ownership.

use std::fmt;
use std::sync::Arc;

use slicer_ir::{
    InfillIR, LayerAnnotation, LayerCollectionIR, LayerPlanIR, MeshIR, MeshSegmentationIR,
    PaintRegionIR, PerimeterIR, RegionMapIR, RetractMode, SeamPlanIR, SliceIR, SupportGeometryIR,
    SupportIR, SupportPlanIR, SurfaceClassificationIR, ToolChange, ZHop,
};

/// A retract or unretract decision collected from `Layer::PathOptimization`.
///
/// Stored in the per-layer deferred queue and verified by travel-policy tests.
/// Packet 11 serialises these; packet 20 may reconcile them with finalization geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct DeferredRetract {
    /// Entity index anchor (same semantics as `ZHop.after_entity_index`).
    pub after_entity_index: u32,
    /// Retraction length in mm.
    pub length: f32,
    /// Retraction speed in mm/s.
    pub speed: f32,
    /// `true` = Unretract; `false` = Retract.
    pub is_unretract: bool,
    /// Selects whether the emitter materializes this as an inline-E `G1`
    /// move (`Gcode`) or a bare `G10`/`G11` firmware opcode (`Firmware`).
    /// Threaded from `path-optimization-default`'s `retract_mode` config
    /// through `Layer::PathOptimization` dispatch into `gcode_emit`.
    pub mode: RetractMode,
}

/// A travel move decision collected from `Layer::PathOptimization`.
///
/// Stored in the per-layer deferred queue so that packet-11 can serialize travel
/// moves and packet-20 can reconcile them with finalization geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct DeferredTravelMove {
    /// Entity index anchor (same semantics as `ZHop.after_entity_index`).
    pub after_entity_index: u32,
    /// X destination in module coordinate units (100 nm).
    pub x: Option<f32>,
    /// Y destination in module coordinate units (100 nm).
    pub y: Option<f32>,
    /// Z destination in module coordinate units (100 nm).
    pub z: Option<f32>,
    /// Feed-rate override in mm/s (`None` = keep current speed).
    pub f: Option<f32>,
}

/// Host-owned immutable global IR store plus write-once per-layer output slots.
#[derive(Debug)]
pub struct Blackboard {
    mesh_ir: Arc<MeshIR>,
    surface_classification: Option<Arc<SurfaceClassificationIR>>,
    mesh_segmentation: Option<Arc<MeshSegmentationIR>>,
    layer_plan: Option<Arc<LayerPlanIR>>,
    seam_plan: Option<Arc<SeamPlanIR>>,
    support_plan: Option<Arc<SupportPlanIR>>,
    paint_regions: Option<Arc<PaintRegionIR>>,
    region_map: Option<Arc<RegionMapIR>>,
    support_geometry: Option<Arc<SupportGeometryIR>>,
    layer_outputs: Option<Vec<Option<LayerCollectionIR>>>,
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
    /// Mesh segmentation marks produced by `PrePass::MeshSegmentation`.
    MeshSegmentation,
    /// Layer plan produced by `PrePass::LayerPlanning`.
    LayerPlan,
    /// Seam plan produced by `PrePass::SeamPlanning`.
    SeamPlan,
    /// Support plan produced by `PrePass::SupportGeometry`.
    SupportPlan,
    /// Paint regions produced by `PrePass::PaintSegmentation`.
    PaintRegions,
    /// Region map produced by `PrePass::RegionMapping`.
    RegionMap,
    /// Support geometry coarse outlines produced by `PrePass::SupportGeometry`.
    SupportGeometry,
}

impl fmt::Display for BlackboardPrepassSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::SurfaceClassification => "surface-classification",
            Self::MeshSegmentation => "mesh-segmentation",
            Self::LayerPlan => "layer-plan",
            Self::SeamPlan => "seam-plan",
            Self::SupportPlan => "support-plan",
            Self::PaintRegions => "paint-regions",
            Self::RegionMap => "region-map",
            Self::SupportGeometry => "support-geometry",
        };

        f.write_str(name)
    }
}

impl Blackboard {
    /// Create a blackboard around a host-owned mesh and fixed per-layer slot count.
    #[must_use]
    pub fn new(mesh_ir: Arc<MeshIR>, layer_count: usize) -> Self {
        Self {
            mesh_ir,
            surface_classification: None,
            mesh_segmentation: None,
            layer_plan: None,
            seam_plan: None,
            support_plan: None,
            paint_regions: None,
            region_map: None,
            support_geometry: None,
            layer_outputs: Some((0..layer_count).map(|_| None).collect()),
        }
    }

    /// Return the host-owned mesh as an `Arc`-backed shared reference.
    #[must_use]
    pub fn mesh(&self) -> &Arc<MeshIR> {
        &self.mesh_ir
    }

    /// Commit `SurfaceClassificationIR` exactly once.
    pub fn commit_surface_classification(
        &mut self,
        ir: Arc<SurfaceClassificationIR>,
    ) -> Result<(), BlackboardError> {
        commit_prepass(
            &mut self.surface_classification,
            ir,
            BlackboardPrepassSlot::SurfaceClassification,
        )
    }

    /// Return the committed surface classification, if available.
    #[must_use]
    pub fn surface_classification(&self) -> Option<&Arc<SurfaceClassificationIR>> {
        self.surface_classification.as_ref()
    }

    /// Commit `MeshSegmentationIR` exactly once.
    pub fn commit_mesh_segmentation(
        &mut self,
        ir: Arc<MeshSegmentationIR>,
    ) -> Result<(), BlackboardError> {
        commit_prepass(
            &mut self.mesh_segmentation,
            ir,
            BlackboardPrepassSlot::MeshSegmentation,
        )
    }

    /// Return the committed mesh-segmentation IR, if available.
    #[must_use]
    pub fn mesh_segmentation(&self) -> Option<&Arc<MeshSegmentationIR>> {
        self.mesh_segmentation.as_ref()
    }

    /// Commit `LayerPlanIR` exactly once.
    pub fn commit_layer_plan(&mut self, ir: Arc<LayerPlanIR>) -> Result<(), BlackboardError> {
        commit_prepass(&mut self.layer_plan, ir, BlackboardPrepassSlot::LayerPlan)
    }

    /// Return the committed layer plan, if available.
    #[must_use]
    pub fn layer_plan(&self) -> Option<&Arc<LayerPlanIR>> {
        self.layer_plan.as_ref()
    }

    /// Commit `SeamPlanIR` exactly once.
    pub fn commit_seam_plan(&mut self, ir: Arc<SeamPlanIR>) -> Result<(), BlackboardError> {
        commit_prepass(&mut self.seam_plan, ir, BlackboardPrepassSlot::SeamPlan)
    }

    /// Return the committed seam plan, if available.
    #[must_use]
    pub fn seam_plan(&self) -> Option<&Arc<SeamPlanIR>> {
        self.seam_plan.as_ref()
    }

    /// Commit `SupportPlanIR` exactly once.
    pub fn commit_support_plan(&mut self, ir: Arc<SupportPlanIR>) -> Result<(), BlackboardError> {
        commit_prepass(
            &mut self.support_plan,
            ir,
            BlackboardPrepassSlot::SupportPlan,
        )
    }

    /// Return the committed support plan, if available.
    #[must_use]
    pub fn support_plan(&self) -> Option<&Arc<SupportPlanIR>> {
        self.support_plan.as_ref()
    }

    /// Commit `PaintRegionIR` exactly once.
    pub fn commit_paint_regions(&mut self, ir: Arc<PaintRegionIR>) -> Result<(), BlackboardError> {
        commit_prepass(
            &mut self.paint_regions,
            ir,
            BlackboardPrepassSlot::PaintRegions,
        )
    }

    /// Return the committed paint regions, if available.
    #[must_use]
    pub fn paint_regions(&self) -> Option<&Arc<PaintRegionIR>> {
        self.paint_regions.as_ref()
    }

    /// Commit `RegionMapIR` exactly once.
    pub fn commit_region_map(&mut self, ir: Arc<RegionMapIR>) -> Result<(), BlackboardError> {
        commit_prepass(&mut self.region_map, ir, BlackboardPrepassSlot::RegionMap)
    }

    /// Return the committed region map, if available.
    #[must_use]
    pub fn region_map(&self) -> Option<&Arc<RegionMapIR>> {
        self.region_map.as_ref()
    }

    /// Commit `SupportGeometryIR` exactly once.
    pub fn commit_support_geometry(
        &mut self,
        ir: Arc<SupportGeometryIR>,
    ) -> Result<(), BlackboardError> {
        commit_prepass(
            &mut self.support_geometry,
            ir,
            BlackboardPrepassSlot::SupportGeometry,
        )
    }

    /// Return the committed support geometry, if available.
    #[must_use]
    pub fn support_geometry(&self) -> Option<&Arc<SupportGeometryIR>> {
        self.support_geometry.as_ref()
    }

    /// Commit one `LayerCollectionIR` into its write-once layer slot.
    pub fn commit_layer_output(
        &mut self,
        layer_index: usize,
        ir: LayerCollectionIR,
    ) -> Result<(), BlackboardError> {
        let layer_outputs = match self.layer_outputs.as_mut() {
            Some(layer_outputs) => layer_outputs,
            None => return Err(BlackboardError::LayerOutputsAlreadyDrained),
        };

        if layer_index >= layer_outputs.len() {
            return Err(BlackboardError::LayerSlotOutOfRange {
                layer_index,
                layer_count: layer_outputs.len(),
            });
        }

        let slot = &mut layer_outputs[layer_index];
        if slot.is_some() {
            return Err(BlackboardError::DuplicateLayerCommit { layer_index });
        }

        *slot = Some(ir);
        Ok(())
    }

    /// Drain all committed layer outputs exactly once after the layer loop.
    pub fn drain_layer_outputs(&mut self) -> Result<Vec<LayerCollectionIR>, BlackboardError> {
        let layer_outputs = match self.layer_outputs.take() {
            Some(layer_outputs) => layer_outputs,
            None => return Err(BlackboardError::LayerOutputsAlreadyDrained),
        };

        let missing_indices: Vec<usize> = layer_outputs
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.is_none().then_some(index))
            .collect();

        if !missing_indices.is_empty() {
            self.layer_outputs = Some(layer_outputs);
            return Err(BlackboardError::IncompleteLayerDrain { missing_indices });
        }

        let drained = layer_outputs
            .into_iter()
            .map(|slot| match slot {
                Some(ir) => ir,
                None => unreachable!("checked for missing layer outputs before draining"),
            })
            .collect();

        Ok(drained)
    }
}

/// Ephemeral per-layer intermediate IR ownership used during one layer worker run.
#[derive(Debug, Default)]
pub struct LayerArena {
    slice: Option<SliceIR>,
    perimeter: Option<PerimeterIR>,
    infill: Option<InfillIR>,
    support: Option<SupportIR>,
    /// Pre-assembled `LayerCollectionIR` staged by the executor immediately
    /// before `Layer::PathOptimization` runs. Once present, any subsequent
    /// `commit_layer_outputs` call for that stage consumes
    /// guest-emitted GCode overrides and appends them onto this staged IR.
    layer_collection: Option<LayerCollectionIR>,
    /// Tool-change entries collected from `Layer::PathOptimization` guest
    /// output and destined for the final `LayerCollectionIR.tool_changes`.
    deferred_tool_changes: Vec<ToolChange>,
    /// Comment/Raw annotations collected from `Layer::PathOptimization` guest
    /// output and destined for the final `LayerCollectionIR.annotations`.
    deferred_annotations: Vec<LayerAnnotation>,
    /// Z-hops collected from `Layer::PathOptimization` guest output
    /// destined for `LayerCollectionIR.z_hops`.
    deferred_z_hops: Vec<ZHop>,
    /// Retract/unretract decisions from `Layer::PathOptimization`.
    deferred_retracts: Vec<DeferredRetract>,
    /// Travel move destinations from `Layer::PathOptimization`.
    deferred_travel_moves: Vec<DeferredTravelMove>,
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
        Self::default()
    }

    /// Stage `SliceIR` in the arena.
    pub fn set_slice(&mut self, ir: SliceIR) -> Result<(), LayerArenaError> {
        set_arena_slot(&mut self.slice, ir, LayerArenaSlot::Slice)
    }

    /// Borrow the staged `SliceIR`, if present.
    #[must_use]
    pub fn slice(&self) -> Option<&SliceIR> {
        self.slice.as_ref()
    }

    /// Take ownership of the staged `SliceIR`, if present.
    pub fn take_slice(&mut self) -> Option<SliceIR> {
        self.slice.take()
    }

    /// Stage `PerimeterIR` in the arena.
    pub fn set_perimeter(&mut self, ir: PerimeterIR) -> Result<(), LayerArenaError> {
        set_arena_slot(&mut self.perimeter, ir, LayerArenaSlot::Perimeter)
    }

    /// Borrow the staged `PerimeterIR`, if present.
    #[must_use]
    pub fn perimeter(&self) -> Option<&PerimeterIR> {
        self.perimeter.as_ref()
    }

    /// Take ownership of the staged `PerimeterIR`, if present.
    pub fn take_perimeter(&mut self) -> Option<PerimeterIR> {
        self.perimeter.take()
    }

    /// Stage `InfillIR` in the arena.
    pub fn set_infill(&mut self, ir: InfillIR) -> Result<(), LayerArenaError> {
        set_arena_slot(&mut self.infill, ir, LayerArenaSlot::Infill)
    }

    /// Borrow the staged `InfillIR`, if present.
    #[must_use]
    pub fn infill(&self) -> Option<&InfillIR> {
        self.infill.as_ref()
    }

    /// Take ownership of the staged `InfillIR`, if present.
    pub fn take_infill(&mut self) -> Option<InfillIR> {
        self.infill.take()
    }

    /// Stage `SupportIR` in the arena.
    pub fn set_support(&mut self, ir: SupportIR) -> Result<(), LayerArenaError> {
        set_arena_slot(&mut self.support, ir, LayerArenaSlot::Support)
    }

    /// Borrow the staged `SupportIR`, if present.
    #[must_use]
    pub fn support(&self) -> Option<&SupportIR> {
        self.support.as_ref()
    }

    /// Take ownership of the staged `SupportIR`, if present.
    pub fn take_support(&mut self) -> Option<SupportIR> {
        self.support.take()
    }

    /// Stage a pre-assembled `LayerCollectionIR` (idempotent replace).
    pub fn set_layer_collection(&mut self, ir: LayerCollectionIR) {
        self.layer_collection = Some(ir);
    }

    /// Borrow the staged `LayerCollectionIR`, if present.
    #[must_use]
    pub fn layer_collection(&self) -> Option<&LayerCollectionIR> {
        self.layer_collection.as_ref()
    }

    /// Take ownership of the staged `LayerCollectionIR`, if present.
    pub fn take_layer_collection(&mut self) -> Option<LayerCollectionIR> {
        self.layer_collection.take()
    }

    /// Append a guest-emitted `ToolChange` onto the per-layer deferred queue.
    pub fn push_deferred_tool_change(&mut self, tc: ToolChange) {
        self.deferred_tool_changes.push(tc);
    }

    /// Take all accumulated deferred tool-changes.
    pub fn take_deferred_tool_changes(&mut self) -> Vec<ToolChange> {
        std::mem::take(&mut self.deferred_tool_changes)
    }

    /// Append a guest-emitted `LayerAnnotation` onto the per-layer queue.
    pub fn push_deferred_annotation(&mut self, ann: LayerAnnotation) {
        self.deferred_annotations.push(ann);
    }

    /// Take all accumulated deferred annotations.
    pub fn take_deferred_annotations(&mut self) -> Vec<LayerAnnotation> {
        std::mem::take(&mut self.deferred_annotations)
    }

    /// Append a guest-emitted `ZHop` onto the per-layer deferred queue.
    pub fn push_deferred_z_hop(&mut self, zh: ZHop) {
        self.deferred_z_hops.push(zh);
    }

    /// Take all accumulated deferred z-hops.
    pub fn take_deferred_z_hops(&mut self) -> Vec<ZHop> {
        std::mem::take(&mut self.deferred_z_hops)
    }

    /// Append a guest-emitted `DeferredRetract` onto the per-layer queue.
    pub fn push_deferred_retract(&mut self, r: DeferredRetract) {
        self.deferred_retracts.push(r);
    }

    /// Take all accumulated deferred retract/unretract decisions.
    pub fn take_deferred_retracts(&mut self) -> Vec<DeferredRetract> {
        std::mem::take(&mut self.deferred_retracts)
    }

    /// Append a guest-emitted travel move onto the per-layer queue.
    pub fn push_deferred_travel_move(&mut self, tm: DeferredTravelMove) {
        self.deferred_travel_moves.push(tm);
    }

    /// Take all accumulated deferred travel move destinations.
    pub fn take_deferred_travel_moves(&mut self) -> Vec<DeferredTravelMove> {
        std::mem::take(&mut self.deferred_travel_moves)
    }

    /// Drop all staged per-layer intermediates before finalization/postpass.
    ///
    /// `deferred_retracts` and `deferred_travel_moves` are flushed into
    /// `LayerCollectionIR` by `layer_executor` before this is called.
    pub fn reset(&mut self) {
        self.slice = None;
        self.perimeter = None;
        self.infill = None;
        self.support = None;
        self.layer_collection = None;
        self.deferred_tool_changes.clear();
        self.deferred_annotations.clear();
        self.deferred_z_hops.clear();
        self.deferred_retracts.clear();
        self.deferred_travel_moves.clear();
    }
}

fn commit_prepass<T>(
    slot: &mut Option<Arc<T>>,
    ir: Arc<T>,
    slot_name: BlackboardPrepassSlot,
) -> Result<(), BlackboardError> {
    if slot.is_some() {
        return Err(BlackboardError::DuplicatePrepassCommit { slot: slot_name });
    }

    *slot = Some(ir);
    Ok(())
}

fn set_arena_slot<T>(
    slot: &mut Option<T>,
    ir: T,
    slot_name: LayerArenaSlot,
) -> Result<(), LayerArenaError> {
    if slot.is_some() {
        return Err(LayerArenaError::SlotAlreadyOccupied { slot: slot_name });
    }

    *slot = Some(ir);
    Ok(())
}
