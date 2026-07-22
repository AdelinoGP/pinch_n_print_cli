//! Host-owned blackboard and per-layer arena contracts.
//!
//! This module defines the TASK-026 public API surface and minimal runtime
//! behavior for blackboard and layer-arena ownership.

use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BlackboardError, BlackboardPrepassSlot, ExPolygon, InfillIR, LayerAnnotation,
    LayerArenaError, LayerArenaSlot, LayerCollectionIR, LayerPlanIR, MeshIR, ObjectMesh,
    PerimeterIR, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, RetractMode,
    SeamPlanIR, SliceIR, SlicedRegion, SupportGeometryIR, SupportGeometryKey, SupportIR,
    SupportPlanIR, SurfaceClassificationIR, ToolChange, ZHop,
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
    layer_plan: Option<Arc<LayerPlanIR>>,
    seam_plan: Option<Arc<SeamPlanIR>>,
    support_plan: Option<Arc<SupportPlanIR>>,
    region_map: Option<Arc<RegionMapIR>>,
    slice_ir: Option<Arc<Vec<SliceIR>>>,
    support_geometry: Option<Arc<SupportGeometryIR>>,
    layer_outputs: Option<Vec<Option<LayerCollectionIR>>>,
}

impl Blackboard {
    /// Create a blackboard around a host-owned mesh and fixed per-layer slot count.
    #[must_use]
    pub fn new(mesh_ir: Arc<MeshIR>, layer_count: usize) -> Self {
        Self {
            mesh_ir,
            surface_classification: None,
            layer_plan: None,
            seam_plan: None,
            support_plan: None,
            region_map: None,
            slice_ir: None,
            support_geometry: None,
            layer_outputs: Some((0..layer_count).map(|_| None).collect()),
        }
    }

    /// Return the host-owned mesh as an `Arc`-backed shared reference.
    #[must_use]
    pub fn mesh(&self) -> &Arc<MeshIR> {
        &self.mesh_ir
    }

    /// Rough heap-aware byte footprint estimate across every committed IR.
    ///
    /// Walks Vec-heavy slots (mesh vertices/indices, slice polygons, layer
    /// plan layers, support entries) and adds their heap allocations to a
    /// `std::mem::size_of` baseline for each Arc-wrapped slot. Returns 0 for
    /// any uncommitted slot. The result is intentionally approximate — fine
    /// enough for monotonic per-built-in byte-delta attribution in the
    /// slicer-report, not a substitute for a true allocator-level measurement.
    #[must_use]
    pub fn estimated_size(&self) -> u64 {
        let mut total: u64 = estimated_mesh_ir_bytes(&self.mesh_ir);
        if let Some(arc) = self.surface_classification.as_ref() {
            total = total.saturating_add(estimated_surface_classification_bytes(arc));
        }
        if let Some(arc) = self.layer_plan.as_ref() {
            total = total.saturating_add(estimated_layer_plan_bytes(arc));
        }
        if let Some(arc) = self.seam_plan.as_ref() {
            total = total.saturating_add(std::mem::size_of_val(arc.as_ref()) as u64);
        }
        if let Some(arc) = self.support_plan.as_ref() {
            total = total.saturating_add(std::mem::size_of_val(arc.as_ref()) as u64);
        }
        if let Some(arc) = self.region_map.as_ref() {
            total = total.saturating_add(estimated_region_map_bytes(arc));
        }
        if let Some(arc) = self.slice_ir.as_ref() {
            total = total.saturating_add(estimated_slice_ir_bytes(arc));
        }
        if let Some(arc) = self.support_geometry.as_ref() {
            total = total.saturating_add(estimated_support_geometry_bytes(arc));
        }
        total
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

    /// Atomically replace the committed `SurfaceClassificationIR`.
    ///
    /// Legal only when [`commit_surface_classification`] has already run
    /// (i.e. `PrePass::MeshAnalysis` committed the base classification).
    /// Used by `PrePass::OverhangAnnotation` to install a copy carrying a
    /// populated `overhang_quartile_polygons` map without re-running mesh
    /// analysis or requiring a second, dedicated blackboard slot.
    ///
    /// [`commit_surface_classification`]: Self::commit_surface_classification
    pub fn replace_surface_classification(
        &mut self,
        ir: Arc<SurfaceClassificationIR>,
    ) -> Result<(), BlackboardError> {
        if self.surface_classification.is_none() {
            return Err(BlackboardError::MissingRequiredPrepass {
                slot: BlackboardPrepassSlot::SurfaceClassification,
            });
        }
        self.surface_classification = Some(ir);
        Ok(())
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
    ///
    /// Rejects any IR that contains duplicate full `RegionKey` entries
    /// (i.e. two entries with the same `(global_layer_index, object_id,
    /// region_id, variant_chain)`). Such duplicates would silently shadow
    /// one plan during harvest/lookup; the error preserves the offending
    /// key. See packet 178 AC-N1.
    pub fn commit_seam_plan(&mut self, ir: Arc<SeamPlanIR>) -> Result<(), BlackboardError> {
        if let Some(duplicate) = ir.duplicate_region_key() {
            return Err(BlackboardError::DuplicateSeamPlanEntry {
                region_key: duplicate,
            });
        }
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

    /// Commit `RegionMapIR` exactly once.
    pub fn commit_region_map(&mut self, ir: Arc<RegionMapIR>) -> Result<(), BlackboardError> {
        commit_prepass(&mut self.region_map, ir, BlackboardPrepassSlot::RegionMap)
    }

    /// Return the committed region map, if available.
    #[must_use]
    pub fn region_map(&self) -> Option<&Arc<RegionMapIR>> {
        self.region_map.as_ref()
    }

    /// Commit the per-global-layer `Vec<SliceIR>` exactly once.
    ///
    /// Called by `PrePass::Slice` host built-in. Subsequent calls return
    /// `BlackboardError::DuplicatePrepassCommit` — use [`replace_slice_ir`] from
    /// `PrePass::ShellClassification` to atomically swap in the shell-stamped
    /// version.
    ///
    /// [`replace_slice_ir`]: Self::replace_slice_ir
    pub fn commit_slice_ir(&mut self, ir: Arc<Vec<SliceIR>>) -> Result<(), BlackboardError> {
        commit_prepass(&mut self.slice_ir, ir, BlackboardPrepassSlot::SliceIR)
    }

    /// Atomically replace the committed `Vec<SliceIR>`.
    ///
    /// Legal only when [`commit_slice_ir`] has run and Tier 2 has not yet
    /// written any per-layer slot. Used by `PrePass::ShellClassification` to
    /// install the version annotated with `top_shell_index` / `bottom_shell_index`
    /// / `top_solid_fill` / `bottom_solid_fill`.
    ///
    /// [`commit_slice_ir`]: Self::commit_slice_ir
    pub fn replace_slice_ir(&mut self, ir: Arc<Vec<SliceIR>>) -> Result<(), BlackboardError> {
        if self.slice_ir.is_none() {
            return Err(BlackboardError::MissingRequiredPrepass {
                slot: BlackboardPrepassSlot::SliceIR,
            });
        }
        debug_assert!(
            self.layer_outputs
                .as_ref()
                .is_some_and(|v| v.iter().all(Option::is_none)),
            "replace_slice_ir called after Tier 2 wrote a layer slot"
        );
        self.slice_ir = Some(ir);
        Ok(())
    }

    /// Return the committed `Vec<SliceIR>`, if available.
    #[must_use]
    pub fn slice_ir(&self) -> Option<&Arc<Vec<SliceIR>>> {
        self.slice_ir.as_ref()
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

// ============================================================================
// estimated_size helpers
//
// Walks the heap allocations of each Arc-wrapped IR to produce a rough byte
// footprint. Used by StageInstrumentationGuard to attribute per-built-in
// blackboard deltas to the slicer-report. Intentionally approximate.
// ============================================================================

fn estimated_polygon_bytes(p: &Polygon) -> u64 {
    std::mem::size_of::<Polygon>() as u64 + (p.points.len() * std::mem::size_of::<Point2>()) as u64
}

fn estimated_expolygon_bytes(p: &ExPolygon) -> u64 {
    let mut total = std::mem::size_of::<ExPolygon>() as u64;
    total += estimated_polygon_bytes(&p.contour);
    for h in &p.holes {
        total += estimated_polygon_bytes(h);
    }
    total
}

fn estimated_mesh_ir_bytes(mesh: &MeshIR) -> u64 {
    let mut total = std::mem::size_of::<MeshIR>() as u64;
    for obj in &mesh.objects {
        total += std::mem::size_of::<ObjectMesh>() as u64;
        total += (obj.mesh.vertices.len() * std::mem::size_of::<Point3>()) as u64;
        total += (obj.mesh.indices.len() * std::mem::size_of::<u32>()) as u64;
    }
    total
}

fn estimated_surface_classification_bytes(ir: &SurfaceClassificationIR) -> u64 {
    let mut total = std::mem::size_of::<SurfaceClassificationIR>() as u64;
    for data in ir.per_object.values() {
        total += data.facet_classes.len() as u64;
        for region in &data.bridge_regions {
            total += (region.facet_indices.len() * std::mem::size_of::<u32>()) as u64;
        }
    }
    total
}

fn estimated_layer_plan_bytes(plan: &LayerPlanIR) -> u64 {
    let mut total = std::mem::size_of::<LayerPlanIR>() as u64;
    for gl in &plan.global_layers {
        total += std::mem::size_of_val(gl) as u64;
        total += (gl.active_regions.len() * std::mem::size_of::<ActiveRegion>()) as u64;
    }
    total
}

fn estimated_region_map_bytes(rm: &RegionMapIR) -> u64 {
    let mut total = std::mem::size_of::<RegionMapIR>() as u64;
    total += (rm.entries.len() * std::mem::size_of::<(RegionKey, RegionPlan)>()) as u64;
    total
}

fn estimated_slice_ir_bytes(slices: &[SliceIR]) -> u64 {
    let mut total = std::mem::size_of_val(slices) as u64;
    for s in slices {
        for r in &s.regions {
            total += std::mem::size_of::<SlicedRegion>() as u64;
            for p in r.polygons.iter().chain(r.infill_areas.iter()) {
                total += estimated_expolygon_bytes(p);
            }
            for p in r.top_solid_fill.iter().chain(r.bottom_solid_fill.iter()) {
                total += estimated_expolygon_bytes(p);
            }
            for p in &r.bridge_areas {
                total += estimated_expolygon_bytes(p);
            }
        }
    }
    total
}

fn estimated_support_geometry_bytes(ir: &SupportGeometryIR) -> u64 {
    let mut total = std::mem::size_of::<SupportGeometryIR>() as u64;
    for polys in ir.entries.values() {
        total += std::mem::size_of::<(SupportGeometryKey, Vec<ExPolygon>)>() as u64;
        for p in polys {
            total += estimated_expolygon_bytes(p);
        }
    }
    total
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
