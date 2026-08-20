//! Traditional support planner.
//!
//! Plans cross-layer contact, base, interface, obstacle, and termination
//! geometry for traditional support, emitting `SupportPlanIR` inside
//! `PrePass::SupportGeometry`.
//!
//! The planner consumes strategy-neutral host analysis (`SupportAnalysisView`)
//! and emits universal structural `SupportPlanIR` v2.0.0 entries: stable
//! `family_id = "traditional"`, demand/body IDs, contact-area body/interface
//! roles derived across layers, and anchored plate/model termination. It never
//! emits nozzle-width toolpaths — the `traditional-support` renderer scan-fills
//! only the planned body/interface polygons.

#![warn(missing_docs)]
#![warn(unused_imports)]

use std::collections::BTreeMap;

use slicer_ir::SupportPlanDeclineReason;
use slicer_sdk::prelude::*;

/// Default number of dense interface layers at the top of a support column.
const DEFAULT_INTERFACE_TOP_LAYERS: i32 = 2;
/// Default number of dense interface layers at the bottom of a support column.
/// `-1` means "mirror the top interface count" (OrcaSlicer convention).
const DEFAULT_INTERFACE_BOTTOM_LAYERS: i32 = -1;
/// Default base fill pattern.
const DEFAULT_BASE_PATTERN: &str = "rectilinear";
/// Default XY clearance between support and object, matching OrcaSlicer's
/// `support_object_xy_distance` default of 0.35 mm.
const DEFAULT_OBJECT_XY_DISTANCE_MM: f32 = 0.35;
/// Default vertical gap between a support contact and the model above it.
/// Matches OrcaSlicer's `support_top_z_distance` default of 0.2 mm. This was
/// `0.0`, so support was printed flush against the overhang with no gap.
const DEFAULT_TOP_Z_DISTANCE_MM: f32 = 0.2;

/// Multi-layer traditional support planner.
#[allow(dead_code)]
pub struct SupportPlanner {
    enabled: bool,
    /// Canonical support family selected for the matching renderer.
    support_family: String,
    /// Number of dense interface layers at the top of each support column.
    support_interface_top_layers: i32,
    /// Number of dense interface layers at the bottom of each support column.
    /// `-1` mirrors the top interface count.
    support_interface_bottom_layers: i32,
    /// Base fill pattern recorded on every body entry.
    support_base_pattern: String,
    /// Distance in mm from column tops to add intermediate model layers.
    support_top_z_distance_mm: f32,
    /// Support layer height in mm (0.0 = use model layer height).
    support_layer_height_mm: f32,
    /// XY clearance in mm held between support and the object during base-layer
    /// trimming, mirroring canonical `SupportParameters::gap_xy`.
    support_object_xy_distance: f32,
}

#[slicer_module]
impl PrepassModule for SupportPlanner {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("enable_support") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => true,
        };
        let support_family = canonical_support_family(config);
        let support_interface_top_layers = match config.get("support_interface_top_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => DEFAULT_INTERFACE_TOP_LAYERS,
        };
        let support_interface_bottom_layers = match config.get("support_interface_bottom_layers") {
            Some(ConfigValue::Int(n)) => *n as i32,
            Some(ConfigValue::Float(n)) => *n as i32,
            _ => DEFAULT_INTERFACE_BOTTOM_LAYERS,
        };
        let support_base_pattern = match config.get("support_base_pattern") {
            Some(ConfigValue::String(s)) => s.clone(),
            _ => DEFAULT_BASE_PATTERN.to_string(),
        };
        let support_top_z_distance_mm = match config.get("support_top_z_distance_mm") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => DEFAULT_TOP_Z_DISTANCE_MM,
        };
        let support_layer_height_mm = match config.get("support_layer_height_mm") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => 0.0,
        };
        // `support_overhang_angle` is no longer read here. Contact detection
        // moved to `PrePass::SupportAnalysis`, which consumes that key from the
        // resolved config and hands this planner finished contacts.
        let support_object_xy_distance = match config.get("support_object_xy_distance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => DEFAULT_OBJECT_XY_DISTANCE_MM,
        };
        Ok(Self {
            enabled,
            support_family,
            support_interface_top_layers,
            support_interface_bottom_layers,
            support_base_pattern,
            support_top_z_distance_mm,
            support_layer_height_mm,
            support_object_xy_distance,
        })
    }

    fn run_support_geometry(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        region_segmentation: &RegionSegmentationView,
        support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        self.run_support_geometry_with_analysis(
            objects,
            layer_plan,
            region_segmentation,
            &SupportAnalysisView::default(),
            support_geometry,
            output,
            config,
        )
    }

    fn run_support_geometry_with_analysis(
        &self,
        objects: &[MeshObjectView],
        layer_plan: &LayerPlanView,
        _region_segmentation: &RegionSegmentationView,
        support_analysis: &SupportAnalysisView,
        _support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !self.enabled {
            for obj in objects {
                for candidate in support_analysis
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.object_id == obj.object_id)
                {
                    if candidate_family(candidate, support_analysis).as_deref()
                        == Some("traditional")
                    {
                        push_policy_declined(output, obj, candidate)?;
                    }
                }
            }
            return Ok(());
        }

        if layer_plan.layers.is_empty() {
            return Err(ModuleError::fatal(1, "empty layer-plan-view"));
        }

        for obj in objects {
            self.plan_for_object(obj, layer_plan, support_analysis, output)?;
        }

        Ok(())
    }
}

impl SupportPlanner {
    fn plan_for_object(
        &self,
        obj: &MeshObjectView,
        layer_plan: &LayerPlanView,
        support_analysis: &SupportAnalysisView,
        output: &mut SupportGeometryOutput,
    ) -> Result<(), ModuleError> {
        let num_layers = layer_plan.layers.len() as u32;
        if num_layers == 0 {
            return Ok(());
        }

        for candidate in support_analysis
            .candidates
            .iter()
            .filter(|candidate| candidate.object_id == obj.object_id)
        {
            if candidate_family(candidate, support_analysis).as_deref() != Some("traditional") {
                continue;
            }
            self.plan_candidate(obj, layer_plan, support_analysis, candidate, output)?;
        }

        Ok(())
    }

    fn plan_candidate(
        &self,
        obj: &MeshObjectView,
        layer_plan: &LayerPlanView,
        support_analysis: &SupportAnalysisView,
        candidate: &SupportAnalysisCandidate,
        output: &mut SupportGeometryOutput,
    ) -> Result<(), ModuleError> {
        let num_layers = layer_plan.layers.len() as u32;
        let demand_id = format!("demand-{}", candidate.id);
        let body_id = format!("traditional-body-{}-{}", obj.object_id, candidate.id);

        if candidate.blocked {
            return push_declined(
                output,
                obj,
                candidate,
                demand_id,
                SupportPlanDeclineReason::Blocked,
            );
        }

        let candidate_geometry: Vec<ExPolygon> = candidate
            .geometry
            .iter()
            .filter(|polygon| polygon.contour.points.len() >= 3)
            .cloned()
            .collect();
        if candidate_geometry.is_empty() {
            return push_declined(
                output,
                obj,
                candidate,
                demand_id,
                SupportPlanDeclineReason::NoRoute,
            );
        }

        let contact_layer = candidate.global_layer_index.min(num_layers - 1);

        // The candidate *is* the contact. `PrePass::SupportAnalysis` derives it
        // with canonical `detect_overhangs` semantics — the angle-thresholded
        // 2D difference between this layer's slice and the grown layer below —
        // so there is nothing further to detect here.
        //
        // This planner previously re-derived contact geometry from downward-
        // facing mesh facets, filtered by whether each facet's Z span crossed
        // this layer's slab. That was wrong twice over: canonical contact
        // detection is 2D over slices rather than 3D over facets, and a step
        // overhang (whose facets are coplanar) crosses at most one slab, so
        // every other candidate was declined `NoRoute`. On the decisive
        // SupportTest fixture that rejected 150 of 150 candidates.
        //
        // Do not reintroduce a second overhang algorithm here. If contact
        // geometry looks wrong, fix `detect_support_overhangs`, which both
        // families share.
        let contact_geometry = candidate_geometry.clone();

        let model_layer_height = layer_plan.layers[contact_layer as usize].effective_layer_height;

        // The candidate's layer is the first layer that *contains* the
        // overhang, so the overhanging surface sits at the bottom of that
        // layer — i.e. at the top of the layer below it. Support must stop
        // `support_top_z_distance_mm` below that plane.
        //
        // The gap is measured by walking actual layer Z rather than dividing by
        // `effective_layer_height`: that field is derived per global layer from
        // object participation and is not a dependable per-layer thickness in
        // the guest view. Dividing by it yielded an offset of zero here (so
        // support fused to the model) and tens of layers in the tree planner.
        let overhang_plane_z = layer_plan.layers[contact_layer.saturating_sub(1) as usize].z;
        let target_top_z = overhang_plane_z - self.support_top_z_distance_mm;
        let mut emit_top_layer = contact_layer.saturating_sub(1);
        while emit_top_layer > 0 && layer_plan.layers[emit_top_layer as usize].z > target_top_z {
            emit_top_layer -= 1;
        }

        // Prefer the highest eligible model termination reached during descent.
        // An empty analysis list preserves the plate fallback contract.
        let model_termination_layer = support_analysis
            .termination_surfaces
            .iter()
            .filter(|surface| {
                surface.object_id == obj.object_id
                    && surface.region_id == candidate.region_id
                    && surface.global_support_layer_index < emit_top_layer
                    && expolygons_overlap(&contact_geometry, &surface.polygons)
            })
            .map(|surface| surface.global_support_layer_index)
            .max();
        // `None` means the column runs to the build plate. The plate is not a
        // model surface, so it carries no bottom interface: there is nothing
        // beneath to interface with. Collapsing both cases into a bare `u32`
        // put dense interface on the first layers off the plate.
        let termination_layer = model_termination_layer.unwrap_or(0);

        // Occupancy rejection is handled by the propagation carry below, which
        // subtracts the object (plus `support_object_xy_distance` clearance)
        // from the carried area layer by layer. A separate pre-pass used to
        // reject the whole body on any overlap and `return Ok(())` — a silent
        // drop that recorded a diagnostic but no declined entry, so the demand
        // vanished from `SupportPlanIR` with nothing marking it unmet. It also
        // rejected on mere overlap rather than shrinking the body around the
        // obstacle, which is what canonical's per-layer `diff` does.

        let top_layers = self.support_interface_top_layers.max(0) as u32;
        let bottom_layers = if self.support_interface_bottom_layers < 0 {
            top_layers
        } else {
            self.support_interface_bottom_layers.max(0) as u32
        };

        let support_step = if self.support_layer_height_mm > 0.0 && model_layer_height > 0.0 {
            (self.support_layer_height_mm / model_layer_height)
                .round()
                .max(1.0) as u32
        } else {
            1
        };
        // Canonical downward propagation (`generate_base_layers` /
        // `bottom_contact_layers_and_layer_support_areas`). Two properties
        // matter and both were missing before packet 224, when this loop emitted
        // the unmodified contact polygon at every layer:
        //
        // 1. **The carry does not grow.** Canonical propagates a *smaller* area
        //    than it prints (`extract_support(expansion_to_propagate)` versus
        //    `(expansion_to_slice)`) precisely so base areas do not swell with
        //    depth. Propagating the contact area unexpanded is that semantic.
        //    The printed-area expansion is deliberately not applied here — it
        //    exists to snap the zig-zag onto `SupportGridPattern`'s grid lines,
        //    and without the grid it would only fatten support by an arbitrary
        //    amount. See the needs-research deviation on the grid pattern.
        //
        // 2. **Each layer is trimmed against the object.** Canonical trims in
        //    `trim_support_layers_by_object` using `gap_xy`
        //    (`support_object_xy_distance`), holding a real XY clearance rather
        //    than merely avoiding overlap.
        //
        // The carry is stateful across layers, so it is built top-down here and
        // consumed by the emit loop below.
        let mut propagated_by_layer: BTreeMap<u32, Vec<ExPolygon>> = BTreeMap::new();
        let mut carry = contact_geometry.clone();
        // Every emitted layer is trimmed against the exact per-layer model
        // occupancy. The contact geometry is an analysis input, not a license
        // for the renderer to overlap the model at the chosen support Z.
        let trim_end = emit_top_layer + 1;
        for layer in (termination_layer..trim_end).rev() {
            let occupancy = occupancy_at(
                support_analysis,
                &obj.object_id,
                &candidate.region_id,
                layer,
            );
            if !occupancy.is_empty() {
                let trimming = if self.support_object_xy_distance > 0.0 {
                    let clearance = host::offset_polygons(
                        &occupancy,
                        self.support_object_xy_distance,
                        OffsetJoinType::Miter,
                        0.0,
                    );
                    if clearance.is_empty() {
                        occupancy
                    } else {
                        clearance
                    }
                } else {
                    occupancy
                };
                carry = host::clip_polygons(&carry, &trimming, ClipOperation::Difference);
            }
            if carry.is_empty() {
                // The object closes off every route below this layer. The demand
                // is unmet and must be recorded as such — never silently
                // dropped, and never tunnelled through the model.
                let _ = output.push_diagnostic(Diagnostic {
                    severity: DiagnosticSeverity::Warn,
                    code: 1203,
                    layer: Some(layer as i32),
                    object_id: Some(obj.object_id.clone()),
                    message: format!(
                        "traditional body rejected: complete body intersects model occupancy at layer {layer}"
                    ),
                });
                return push_declined(
                    output,
                    obj,
                    candidate,
                    demand_id,
                    SupportPlanDeclineReason::NoRoute,
                );
            }
            propagated_by_layer.insert(layer, carry.clone());
        }
        // With one-layer support stepping, the contact layer is the model
        // facing layer and the interface anchors one layer below it. Larger
        // support steps already land on the computed emit layer.
        // `emit_top_layer` is the first printed layer. The configured band is
        // counted from that layer; subtracting one here made every top band
        // one layer too wide (1->2, 2->3, 3->4).
        let interface_top_layer = emit_top_layer;
        for layer in (termination_layer..=emit_top_layer).rev() {
            let is_interface_layer = (top_layers > 0
                && layer >= interface_top_layer.saturating_sub(top_layers - 1))
                || (bottom_layers > 0
                    && model_termination_layer.is_some()
                    && layer < termination_layer + bottom_layers);
            // The termination layer always prints: it is where the column
            // actually lands. Skipping it because it failed the support-layer-
            // height modulo left the support stopping short of the plate.
            let is_termination_layer = layer == termination_layer;
            if !(emit_top_layer - layer).is_multiple_of(support_step)
                && !is_interface_layer
                && !is_termination_layer
            {
                continue;
            }
            let Some(layer_geometry) = propagated_by_layer.get(&layer) else {
                continue;
            };
            // Canonical keeps interface geometry distinct from the base and
            // subtracts it out (`SupportCommon.cpp`'s interface generation), so
            // a layer is either interface or body over any given area — never
            // both. These three roles previously carried byte-identical
            // regions, so an interface layer was extruded twice: once dense as
            // interface and again underneath as body.
            let is_top_interface = top_layers > 0
                && (layer != termination_layer || model_termination_layer.is_some())
                && layer >= interface_top_layer.saturating_sub(top_layers - 1);
            // A floor exists only where the column lands on the model.
            let is_bottom_interface = bottom_layers > 0
                && model_termination_layer.is_some()
                && layer < termination_layer + bottom_layers;
            let mut roles = Vec::new();
            if is_top_interface {
                roles.push(slicer_ir::SupportPlanRoleRegion {
                    role: slicer_ir::SupportPlanRole::TopInterface,
                    regions: layer_geometry.clone(),
                });
            } else if is_bottom_interface {
                roles.push(slicer_ir::SupportPlanRoleRegion {
                    role: slicer_ir::SupportPlanRole::BottomInterface,
                    regions: layer_geometry.clone(),
                });
            } else {
                roles.push(slicer_ir::SupportPlanRoleRegion {
                    role: slicer_ir::SupportPlanRole::SupportBody,
                    regions: layer_geometry.clone(),
                });
            }
            let z = layer_plan.layers[layer as usize].z;
            output
                .push_support_plan_entry(SupportPlanEntry {
                    global_layer_index: layer as i32,
                    object_id: obj.object_id.clone(),
                    region_id: candidate.region_id.clone(),
                    family_id: "traditional".to_string(),
                    demand_ids: vec![demand_id.clone()],
                    body_ids: vec![body_id.clone()],
                    anchor_layer_index: layer,
                    anchor_z: mm_to_units(z),
                    roles,
                    skeleton: None,
                    capabilities: vec![format!(
                        "traditional-base-pattern:{}",
                        self.support_base_pattern
                    )],
                    provenance: vec!["traditional-support-planner".to_string()],
                    decline_reason: None,
                })
                .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
        }

        Ok(())
    }
}

fn push_declined(
    output: &mut SupportGeometryOutput,
    obj: &MeshObjectView,
    candidate: &SupportAnalysisCandidate,
    demand_id: String,
    reason: SupportPlanDeclineReason,
) -> Result<(), ModuleError> {
    output
        .push_support_plan_entry(SupportPlanEntry {
            global_layer_index: candidate.global_layer_index as i32,
            object_id: obj.object_id.clone(),
            region_id: candidate.region_id.clone(),
            family_id: "traditional".to_string(),
            demand_ids: vec![demand_id],
            body_ids: Vec::new(),
            anchor_layer_index: candidate.global_layer_index,
            anchor_z: candidate.z_units,
            roles: Vec::new(),
            skeleton: None,
            capabilities: Vec::new(),
            provenance: vec!["traditional-support-planner".to_string()],
            decline_reason: Some(reason),
        })
        .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
    Ok(())
}

fn push_policy_declined(
    output: &mut SupportGeometryOutput,
    obj: &MeshObjectView,
    candidate: &SupportAnalysisCandidate,
) -> Result<(), ModuleError> {
    output
        .push_support_plan_entry(SupportPlanEntry {
            global_layer_index: candidate.global_layer_index as i32,
            object_id: obj.object_id.clone(),
            region_id: candidate.region_id.clone(),
            family_id: "traditional".to_string(),
            demand_ids: Vec::new(),
            body_ids: Vec::new(),
            anchor_layer_index: candidate.global_layer_index,
            anchor_z: candidate.z_units,
            roles: Vec::new(),
            skeleton: None,
            capabilities: Vec::new(),
            provenance: vec!["traditional-support-planner".to_string()],
            decline_reason: Some(SupportPlanDeclineReason::DeclinedPolicy),
        })
        .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
    Ok(())
}

/// Resolve the canonical support family for a candidate from the host's
/// per-region family assignments, falling back to the planner's own family.
/// Resolve the canonical support family for a candidate from the host's
/// per-region family assignments.
///
/// Returns `None` when the host made no assignment for this region, in which
/// case the planner plans nothing for it. `PrePass::SupportAnalysis` is the
/// single authority; a planner that falls back to its own family can publish
/// entries for regions region routing assigned elsewhere, and the resulting
/// disagreement is silent (see the tree planner's `candidate_family`).
fn candidate_family(
    candidate: &SupportAnalysisCandidate,
    analysis: &SupportAnalysisView,
) -> Option<String> {
    analysis
        .family_assignments
        .iter()
        .find(|assignment| {
            assignment.object_id == candidate.object_id
                && assignment.region_id == candidate.region_id
        })
        .map(|assignment| canonical_support_family_alias(Some(&assignment.family_id)))
}

/// Resolve the global support selection to the family vocabulary shared by
/// the planner and both renderers. Orca-style `support_type` aliases remain
/// accepted, with the legacy key taking precedence when both are present.
fn canonical_support_family(config: &ConfigView) -> String {
    let value = config
        .get("support_type")
        .or_else(|| config.get("support_family"))
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        });
    value
        .map(|value| canonical_support_family_alias(Some(value)))
        .unwrap_or_else(|| "traditional".to_string())
}

fn canonical_support_family_alias(value: Option<&str>) -> String {
    slicer_ir::canonical_support_family(value).to_string()
}

/// Return the model-occupancy polygons for one (object, region, layer) triple.
fn occupancy_at(
    analysis: &SupportAnalysisView,
    object_id: &str,
    region_id: &str,
    layer: u32,
) -> Vec<ExPolygon> {
    analysis
        .model_occupancy
        .iter()
        .filter(|entry| {
            entry.object_id == object_id
                && entry.region_id == region_id
                && entry.global_support_layer_index == layer
        })
        .flat_map(|entry| entry.polygons.iter().cloned())
        .collect()
}

/// Whether any polygon in `a` overlaps any polygon in `b` (positive area).
fn expolygons_overlap(a: &[ExPolygon], b: &[ExPolygon]) -> bool {
    !host::clip_polygons(a, b, ClipOperation::Intersection).is_empty()
}
