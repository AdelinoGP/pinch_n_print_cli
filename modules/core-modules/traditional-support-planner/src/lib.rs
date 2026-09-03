// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Support/SupportMaterial.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
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
/// Default extrusion line width in mm, used to expand the canonical bottom
/// contact area (`support_material_flow.scaled_width()`).
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;
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
    /// Orca key `support_top_z_distance`.
    support_top_z_distance: f32,
    /// Support layer height in mm (0.0 = use model layer height).
    support_layer_height_mm: f32,
    /// Packet 239c: support rows may leave the object layer grid. When true,
    /// canonical `bottom_contact_layer` (enabled branch) plus
    /// `generate_support_layers` let intermediate support rows print between
    /// object planes; when false, `anchor_z` stays a grid-exact copy of the
    /// object plane (canonical `sync_gap_with_object_layer`).
    independent_support_layer_height: bool,
    /// XY clearance in mm held between support and the object during base-layer
    /// trimming, mirroring canonical `SupportParameters::gap_xy`.
    support_object_xy_distance: f32,
    /// Extrusion line width in mm. Canonical expands the bottom contact area by
    /// one support-flow width (`bottom_contact_layers_and_layer_support_areas`).
    line_width_mm: f32,
}

#[slicer_module]
impl PrepassModule for SupportPlanner {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        let enabled = match config.get("enable_support") {
            Some(ConfigValue::Bool(b)) => *b,
            _ => false,
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
        let support_top_z_distance = match config.get("support_top_z_distance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => DEFAULT_TOP_Z_DISTANCE_MM,
        };
        let support_layer_height_mm = match config.get("support_layer_height_mm") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => 0.0,
        };
        // Packet 239c: default true, matching the manifest declaration and
        // canonical `PrintConfig.cpp` `init_fff_params` (coBool, default
        // true). When true, `plan_candidate` derives free-floating
        // intermediate support planes; when false the plan is byte-identical
        // to the pre-239c grid-exact behavior.
        let independent_support_layer_height = config
            .get_bool("independent_support_layer_height")
            .unwrap_or(true);
        // `support_overhang_angle` is no longer read here. Contact detection
        // moved to `PrePass::SupportAnalysis`, which consumes that key from the
        // resolved config and hands this planner finished contacts.
        let support_object_xy_distance = match config.get("support_object_xy_distance") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => DEFAULT_OBJECT_XY_DISTANCE_MM,
        };
        let line_width_mm = match config.get("line_width") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => DEFAULT_LINE_WIDTH_MM,
        };
        Ok(Self {
            enabled,
            support_family,
            support_interface_top_layers,
            support_interface_bottom_layers,
            support_base_pattern,
            support_top_z_distance,
            support_layer_height_mm,
            independent_support_layer_height,
            support_object_xy_distance,
            line_width_mm,
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

        // Intermediate planes need identities distinct from their bracketing
        // grid rows. Reuse an identity when candidates declare the same plane
        // so same-family aggregation can still union their geometry.
        let mut intermediate_plane_indices = BTreeMap::<i64, i32>::new();
        let mut pending_entries = Vec::new();

        for candidate in support_analysis
            .candidates
            .iter()
            .filter(|candidate| candidate.object_id == obj.object_id)
        {
            if candidate_family(candidate, support_analysis).as_deref() != Some("traditional") {
                continue;
            }
            self.plan_candidate(
                obj,
                layer_plan,
                support_analysis,
                candidate,
                &mut pending_entries,
                output,
            )?;
        }

        if self.independent_support_layer_height && self.support_layer_height_mm > 0.0 {
            self.emit_coarse_entries(
                layer_plan,
                &mut pending_entries,
                &mut intermediate_plane_indices,
                output,
            )?;
        } else {
            for entry in pending_entries.drain(..) {
                output
                    .push_support_plan_entry(entry)
                    .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
            }
        }

        Ok(())
    }

    fn plan_candidate(
        &self,
        obj: &MeshObjectView,
        layer_plan: &LayerPlanView,
        support_analysis: &SupportAnalysisView,
        candidate: &SupportAnalysisCandidate,
        pending_entries: &mut Vec<SupportPlanEntry>,
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
        // `support_top_z_distance` below that plane.
        //
        // The gap is measured by walking actual layer Z rather than dividing by
        // `effective_layer_height`: that field is derived per global layer from
        // object participation and is not a dependable per-layer thickness in
        // the guest view. Dividing by it yielded an offset of zero here (so
        // support fused to the model) and tens of layers in the tree planner.
        let overhang_plane_z = layer_plan.layers[contact_layer.saturating_sub(1) as usize].z;
        let target_top_z = overhang_plane_z - self.support_top_z_distance;
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
        // G-18: widen the traditional top band only for a raw positive bottom count;
        // see design.md §Plan Corrections item 4.
        let top_interface_layers =
            top_layers + u32::from(self.support_interface_bottom_layers >= 1);

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
            // Ticket 19: keep the column on the side of a modifier boundary
            // this family owns. A minted sub-region keeps `carry ∩ own`; a
            // base region keeps `carry - inflate(foreign, line width)` so the
            // two families never touch. Orca has no per-region support family;
            // this has no canonical counterpart. An emptied carry falls into
            // the `NoRoute` decline below.
            if !carry.is_empty() {
                if let Some(own) =
                    support_analysis.region_territory(&obj.object_id, layer, &candidate.region_id)
                {
                    carry = host::clip_polygons(&carry, own, ClipOperation::Intersection);
                } else if let Some(partition) =
                    support_analysis.territory_partition(&obj.object_id, layer, "traditional")
                {
                    if !partition.foreign.is_empty() {
                        let bar = if self.line_width_mm > 0.0 {
                            let grown = host::offset_polygons(
                                &partition.foreign,
                                self.line_width_mm,
                                OffsetJoinType::Miter,
                                0.0,
                            );
                            if grown.is_empty() {
                                partition.foreign
                            } else {
                                grown
                            }
                        } else {
                            partition.foreign
                        };
                        carry = host::clip_polygons(&carry, &bar, ClipOperation::Difference);
                    }
                }
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
        let support_step = if self.support_layer_height_mm > 0.0 && model_layer_height > 0.0 {
            (self.support_layer_height_mm / model_layer_height)
                .round()
                .max(1.0) as u32
        } else {
            1
        };
        // With one-layer support stepping, the contact layer is the model
        // facing layer and the interface anchors one layer below it. Larger
        // support steps already land on the computed emit layer.
        // `emit_top_layer` is the first printed layer. The configured band is
        // counted from that layer; subtracting one here made every top band
        // one layer too wide (1->2, 2->3, 3->4).
        let interface_top_layer = emit_top_layer;

        // F-36. Canonical `bottom_contact_layers_and_layer_support_areas`
        // builds the floor from `intersection(top_surfaces, supports_projected)`
        // and then expands it by one support-flow width — the floor covers only
        // the part of the column that actually lands on a model top surface.
        // This planner marked the *whole* layer cross-section BottomInterface,
        // so a column landing half on the model and half on the plate printed
        // dense interface over the plate half too.
        let bottom_contact_area: Vec<ExPolygon> = match model_termination_layer {
            None => Vec::new(),
            Some(term_layer) => {
                let surfaces: Vec<ExPolygon> = support_analysis
                    .termination_surfaces
                    .iter()
                    .filter(|surface| {
                        surface.object_id == obj.object_id
                            && surface.region_id == candidate.region_id
                            && surface.global_support_layer_index == term_layer
                    })
                    .flat_map(|surface| surface.polygons.iter().cloned())
                    .collect();
                let landed = propagated_by_layer
                    .get(&term_layer)
                    .cloned()
                    .unwrap_or_default();
                let contact = host::clip_polygons(&landed, &surfaces, ClipOperation::Intersection);
                if contact.is_empty() || self.line_width_mm <= 0.0 {
                    contact
                } else {
                    let grown = host::offset_polygons(
                        &contact,
                        self.line_width_mm,
                        OffsetJoinType::Miter,
                        0.0,
                    );
                    if grown.is_empty() {
                        contact
                    } else {
                        grown
                    }
                }
            }
        };

        let is_interface_layer = |layer: u32| {
            let top = top_interface_layers > 0
                && layer >= interface_top_layer.saturating_sub(top_interface_layers - 1);
            let bottom = bottom_layers > 0
                && model_termination_layer.is_some()
                && layer < termination_layer + bottom_layers;
            top || bottom
        };
        let buffer_for_possible_coarse_order =
            self.independent_support_layer_height && self.support_layer_height_mm > 0.0;

        for layer in (termination_layer..=emit_top_layer).rev() {
            let is_interface_layer = is_interface_layer(layer);
            // The termination layer always prints: it is where the column
            // actually lands. Skipping it because it failed the support-layer-
            // height modulo left the support stopping short of the plate.
            let is_termination_layer = layer == termination_layer;
            // Enabled independent-height rows are buffered undecimated so D1a
            // can classify each bracket from the actual support-bearing run.
            // `emit_coarse_entries` then applies this same gate only outside
            // coarse brackets before deriving any intermediate planes.
            if !buffer_for_possible_coarse_order
                && !(emit_top_layer - layer).is_multiple_of(support_step)
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
            // F-49: top-interface membership depends only on the layer's
            // distance below the top contact, exactly as canonical
            // `generate_interface_layers` counts `top_interface_layers` down
            // from the contact layer. This additionally required
            // `layer != termination_layer || model_termination_layer.is_some()`,
            // which excluded the plate layer — so a column shorter than
            // `support_interface_top_layers` printed its bottom-most layer as
            // body even though it lies inside the roof band.
            let is_top_interface = top_interface_layers > 0
                && layer >= interface_top_layer.saturating_sub(top_interface_layers - 1);
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
            } else if is_bottom_interface && !bottom_contact_area.is_empty() {
                // Only the part of the cross-section standing on the model top
                // surface is floor; the remainder keeps printing as body.
                let floor = host::clip_polygons(
                    layer_geometry,
                    &bottom_contact_area,
                    ClipOperation::Intersection,
                );
                let remainder = host::clip_polygons(
                    layer_geometry,
                    &bottom_contact_area,
                    ClipOperation::Difference,
                );
                if !floor.is_empty() {
                    roles.push(slicer_ir::SupportPlanRoleRegion {
                        role: slicer_ir::SupportPlanRole::BottomInterface,
                        regions: floor,
                    });
                }
                if !remainder.is_empty() {
                    roles.push(slicer_ir::SupportPlanRoleRegion {
                        role: slicer_ir::SupportPlanRole::SupportBody,
                        regions: remainder,
                    });
                }
                if roles.is_empty() {
                    roles.push(slicer_ir::SupportPlanRoleRegion {
                        role: slicer_ir::SupportPlanRole::SupportBody,
                        regions: layer_geometry.clone(),
                    });
                }
            } else {
                roles.push(slicer_ir::SupportPlanRoleRegion {
                    role: slicer_ir::SupportPlanRole::SupportBody,
                    regions: layer_geometry.clone(),
                });
            }
            let z = layer_plan.layers[layer as usize].z;
            let entry = SupportPlanEntry {
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
            };
            if buffer_for_possible_coarse_order {
                pending_entries.push(entry);
            } else {
                output
                    .push_support_plan_entry(entry)
                    .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
            }
        }

        Ok(())
    }

    fn emit_coarse_entries(
        &self,
        layer_plan: &LayerPlanView,
        entries: &mut Vec<SupportPlanEntry>,
        intermediate_plane_indices: &mut BTreeMap<i64, i32>,
        output: &mut SupportGeometryOutput,
    ) -> Result<(), ModuleError> {
        if self.support_layer_height_mm <= 0.0 {
            return Ok(());
        }

        // Coarse support is local to one object/region run. Physical rows are
        // deduplicated only for bracket derivation; all genuine entries survive.
        let mut regions: Vec<String> = entries.iter().map(|e| e.region_id.clone()).collect();
        regions.sort();
        regions.dedup();
        let mut retained = Vec::new();
        let mut synthesized: Vec<(SupportPlanEntry, f64)> = Vec::new();
        let mut coarse_used = false;
        let mut coarse_membership = Vec::<(String, u32, u32, i64, i64)>::new();
        let mut removed = std::collections::BTreeSet::new();
        for region in regions {
            let mut indexed_rows: Vec<usize> = entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.region_id == region && e.global_layer_index >= 0)
                .map(|(i, _)| i)
                .collect();
            indexed_rows.sort_by_key(|&i| (entries[i].anchor_z, entries[i].anchor_layer_index));
            let mut rows: Vec<usize> = Vec::new();
            for index in indexed_rows {
                if let Some(last) = rows.last_mut() {
                    if entries[*last].anchor_z == entries[index].anchor_z {
                        let is_interface = |entry: &SupportPlanEntry| {
                            entry.roles.iter().any(|role| {
                                matches!(
                                    role.role,
                                    slicer_ir::SupportPlanRole::TopInterface
                                        | slicer_ir::SupportPlanRole::BaseInterface
                                        | slicer_ir::SupportPlanRole::BottomInterface
                                )
                            })
                        };
                        if !is_interface(&entries[*last]) && is_interface(&entries[index]) {
                            *last = index;
                        }
                        continue;
                    }
                }
                rows.push(index);
            }
            let mut runs: Vec<Vec<usize>> = Vec::new();
            for row in rows {
                if runs.last().is_some_and(|run| {
                    entries[*run.last().unwrap()].anchor_layer_index + 1
                        == entries[row].anchor_layer_index
                }) {
                    runs.last_mut().unwrap().push(row);
                } else {
                    runs.push(vec![row]);
                }
            }
            for run in runs {
                let is_interface = |index: usize| {
                    entries[index].roles.iter().any(|role| {
                        matches!(
                            role.role,
                            slicer_ir::SupportPlanRole::TopInterface
                                | slicer_ir::SupportPlanRole::BaseInterface
                                | slicer_ir::SupportPlanRole::BottomInterface
                        )
                    })
                };
                let mut interfaces: Vec<usize> =
                    run.iter().copied().filter(|&i| is_interface(i)).collect();
                interfaces.dedup_by_key(|&mut i| entries[i].anchor_z);
                let mut bracket_pairs: Vec<(usize, usize)> = interfaces
                    .windows(2)
                    .filter_map(|pair| {
                        let lower = pair[0];
                        let upper = pair[1];
                        run.iter()
                            .copied()
                            .any(|index| {
                                entries[index].anchor_z > entries[lower].anchor_z
                                    && entries[index].anchor_z < entries[upper].anchor_z
                                    && !is_interface(index)
                            })
                            .then_some((lower, upper))
                    })
                    .collect();
                if bracket_pairs.is_empty() {
                    let lower = *run.first().unwrap();
                    let upper = *run.last().unwrap();
                    if entries[lower].anchor_z < entries[upper].anchor_z {
                        bracket_pairs.push((lower, upper));
                    }
                }
                for (lower_index, upper_index) in bracket_pairs {
                    let lower = &entries[lower_index];
                    let upper = &entries[upper_index];
                    let covered: Vec<usize> = run
                        .iter()
                        .copied()
                        .filter(|&i| {
                            entries[i].anchor_z >= lower.anchor_z
                                && entries[i].anchor_z <= upper.anchor_z
                        })
                        .collect();
                    let local_support_gap = covered
                        .windows(2)
                        .map(|rows| entries[rows[1]].anchor_z - entries[rows[0]].anchor_z)
                        .filter(|gap| *gap > 0)
                        .max()
                        .unwrap_or(0);
                    let pitch_units = mm_to_units(self.support_layer_height_mm);
                    if local_support_gap == 0 || pitch_units < local_support_gap {
                        continue;
                    }
                    coarse_used = true;
                    let run_first_layer = entries[*run.first().unwrap()].anchor_layer_index;
                    let run_last_layer = entries[*run.last().unwrap()].anchor_layer_index;
                    coarse_membership.push((
                        region.clone(),
                        run_first_layer,
                        run_last_layer,
                        lower.anchor_z,
                        upper.anchor_z,
                    ));
                    for (i, entry) in entries.iter().enumerate() {
                        if entry.region_id == region
                            && entry.anchor_layer_index >= run_first_layer
                            && entry.anchor_layer_index <= run_last_layer
                            && entry.anchor_z > lower.anchor_z
                            && entry.anchor_z < upper.anchor_z
                            && entry
                                .roles
                                .iter()
                                .all(|r| r.role == slicer_ir::SupportPlanRole::SupportBody)
                        {
                            removed.insert(i);
                        }
                    }
                    for plane_mm in packet239d_coarse_planes(
                        lower.anchor_z,
                        upper.anchor_z,
                        self.support_layer_height_mm as f64,
                    ) {
                        let plane = (plane_mm * slicer_ir::UNITS_PER_MM).round() as i64;
                        let source_index = run
                            .iter()
                            .copied()
                            .rev()
                            .find(|&index| entries[index].anchor_z <= plane)
                            .unwrap_or(lower_index);
                        let source_z = entries[source_index].anchor_z;
                        let anchor_layer_index = layer_plan
                            .layers
                            .iter()
                            .enumerate()
                            .min_by_key(|(index, layer)| {
                                (mm_to_units(layer.z).abs_diff(plane), *index)
                            })
                            .map(|(index, _)| index as u32)
                            .unwrap_or(lower.anchor_layer_index);
                        // Prefer body geometry only within the same demanded
                        // body membership on the selected physical row.
                        for source in entries.iter().filter(|entry| {
                            entry.region_id == region
                                && entry.global_layer_index >= 0
                                && entry.anchor_layer_index >= run_first_layer
                                && entry.anchor_layer_index <= run_last_layer
                                && entry.anchor_z == source_z
                                && (entry.roles.iter().all(|role| {
                                    !matches!(
                                        role.role,
                                        slicer_ir::SupportPlanRole::TopInterface
                                            | slicer_ir::SupportPlanRole::BaseInterface
                                            | slicer_ir::SupportPlanRole::BottomInterface
                                    )
                                }) || !entries.iter().any(|candidate| {
                                    candidate.region_id == region
                                        && candidate.global_layer_index >= 0
                                        && candidate.anchor_layer_index >= run_first_layer
                                        && candidate.anchor_layer_index <= run_last_layer
                                        && candidate.anchor_z == source_z
                                        && candidate.body_ids == entry.body_ids
                                        && candidate.roles.iter().all(|role| {
                                            !matches!(
                                                role.role,
                                                slicer_ir::SupportPlanRole::TopInterface
                                                    | slicer_ir::SupportPlanRole::BaseInterface
                                                    | slicer_ir::SupportPlanRole::BottomInterface
                                            )
                                        })
                                }))
                        }) {
                            let mut clone = source.clone();
                            clone.anchor_layer_index = anchor_layer_index;
                            clone.anchor_z = plane;
                            clone.roles = clone
                                .roles
                                .into_iter()
                                .map(|role| slicer_ir::SupportPlanRoleRegion {
                                    role: slicer_ir::SupportPlanRole::SupportBody,
                                    regions: role.regions,
                                })
                                .collect();
                            synthesized.push((clone, plane_mm));
                        }
                    }
                }
            }
        }
        let mut rows_by_body = BTreeMap::<Vec<String>, Vec<usize>>::new();
        for (index, entry) in entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.global_layer_index >= 0 && entry.decline_reason.is_none())
        {
            rows_by_body
                .entry(entry.body_ids.clone())
                .or_default()
                .push(index);
        }
        for rows in rows_by_body.values() {
            let Some(emit_top_layer) = rows
                .iter()
                .map(|&index| entries[index].anchor_layer_index)
                .max()
            else {
                continue;
            };
            let termination_layer = rows
                .iter()
                .map(|&index| entries[index].anchor_layer_index)
                .min()
                .unwrap_or(emit_top_layer);
            let model_layer_height =
                layer_plan.layers[emit_top_layer as usize].effective_layer_height;
            let support_step = if model_layer_height > 0.0 {
                (self.support_layer_height_mm / model_layer_height)
                    .round()
                    .max(1.0) as u32
            } else {
                1
            };
            for &index in rows {
                let entry = &entries[index];
                let in_coarse_range = coarse_membership.iter().any(
                    |(region, run_first, run_last, lower_z, upper_z)| {
                        entry.region_id == *region
                            && entry.anchor_layer_index >= *run_first
                            && entry.anchor_layer_index <= *run_last
                            && entry.anchor_z >= *lower_z
                            && entry.anchor_z <= *upper_z
                    },
                );
                let is_interface = entry.roles.iter().any(|role| {
                    matches!(
                        role.role,
                        slicer_ir::SupportPlanRole::TopInterface
                            | slicer_ir::SupportPlanRole::BaseInterface
                            | slicer_ir::SupportPlanRole::BottomInterface
                    )
                });
                if !in_coarse_range
                    && !is_interface
                    && entry.anchor_layer_index != termination_layer
                    && !(emit_top_layer - entry.anchor_layer_index).is_multiple_of(support_step)
                {
                    removed.insert(index);
                }
            }
        }

        // The finer derivation must consume the locally decimated rows, not the
        // raw rows needed for D1a classification. This is the production D3
        // gate: rows in coarse brackets are retained as support_step=1 above;
        // every other body range keeps the pre-239d support-step modulo.
        if !coarse_used {
            let mut previous_by_body = BTreeMap::<Vec<String>, SupportPlanEntry>::new();
            for (index, entry) in entries.iter().enumerate() {
                if removed.contains(&index) {
                    continue;
                }
                if let Some(above) = previous_by_body.get(&entry.body_ids) {
                    let below_z = layer_plan.layers[entry.anchor_layer_index as usize].z;
                    let above_z = layer_plan.layers[above.anchor_layer_index as usize].z;
                    for plane in packet239c_intermediate_planes(
                        below_z,
                        above_z,
                        self.support_layer_height_mm as f64,
                    ) {
                        let mut clone = entry.clone();
                        clone.global_layer_index =
                            next_intermediate_plane_index(intermediate_plane_indices, plane)?;
                        clone.anchor_z = plane;
                        retained.push(clone);
                    }
                }
                previous_by_body.insert(entry.body_ids.clone(), entry.clone());
                retained.push(entry.clone());
            }
            for entry in retained.drain(..) {
                output
                    .push_support_plan_entry(entry)
                    .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
            }
            return Ok(());
        }

        retained.extend(
            entries
                .iter()
                .enumerate()
                .filter(|(index, _)| !removed.contains(index))
                .map(|(_, entry)| entry.clone()),
        );
        for rows in rows_by_body.values() {
            let mut surviving: Vec<usize> = rows
                .iter()
                .copied()
                .filter(|index| !removed.contains(index))
                .collect();
            surviving.sort_by_key(|&index| entries[index].anchor_z);
            for pair in surviving.windows(2) {
                let lower = &entries[pair[0]];
                let upper = &entries[pair[1]];
                let in_same_coarse_bracket = coarse_membership.iter().any(
                    |(region, run_first, run_last, lower_z, upper_z)| {
                        lower.region_id == *region
                            && lower.anchor_layer_index >= *run_first
                            && upper.anchor_layer_index <= *run_last
                            && lower.anchor_z >= *lower_z
                            && upper.anchor_z <= *upper_z
                    },
                );
                if in_same_coarse_bracket {
                    continue;
                }
                let lower_z = layer_plan.layers[lower.anchor_layer_index as usize].z;
                let upper_z = layer_plan.layers[upper.anchor_layer_index as usize].z;
                for plane in packet239c_intermediate_planes(
                    lower_z,
                    upper_z,
                    self.support_layer_height_mm as f64,
                ) {
                    let mut clone = lower.clone();
                    clone.anchor_z = plane;
                    synthesized.push((clone, plane as f64 / slicer_ir::UNITS_PER_MM));
                }
            }
        }
        // Canonical candidate grouping happens before plane identity assignment.
        const EPSILON_MM: f64 = 1e-4;
        synthesized.sort_by(|left, right| left.1.total_cmp(&right.1));
        let mut start = 0;
        while start < synthesized.len() {
            let mut end = start + 1;
            while end < synthesized.len() && synthesized[end].1 - synthesized[start].1 <= EPSILON_MM
            {
                end += 1;
            }
            let midpoint_mm = (synthesized[start].1 + synthesized[end - 1].1) / 2.0;
            let midpoint = (midpoint_mm * slicer_ir::UNITS_PER_MM).round() as i64;
            for (entry, _) in &mut synthesized[start..end] {
                entry.anchor_z = midpoint;
                entry.anchor_layer_index = layer_plan
                    .layers
                    .iter()
                    .enumerate()
                    .min_by_key(|(index, layer)| (mm_to_units(layer.z).abs_diff(midpoint), *index))
                    .map(|(index, _)| index as u32)
                    .unwrap_or(entry.anchor_layer_index);
            }
            start = end;
        }
        let mut synthesized_keys = std::collections::BTreeSet::new();
        synthesized.retain(|(entry, _)| {
            synthesized_keys.insert((
                entry.global_layer_index,
                entry.object_id.clone(),
                entry.region_id.clone(),
                entry.body_ids.clone(),
                entry.anchor_z,
            ))
        });
        for (entry, _) in &mut synthesized {
            let plane_ordinal = i32::try_from(intermediate_plane_indices.len()).map_err(|_| {
                ModuleError::fatal(1, "too many intermediate traditional-support planes")
            })?;
            entry.global_layer_index = *intermediate_plane_indices.entry(entry.anchor_z).or_insert(
                i32::MIN.checked_add(plane_ordinal).ok_or_else(|| {
                    ModuleError::fatal(1, "too many intermediate traditional-support planes")
                })?,
            );
        }
        retained.extend(synthesized.into_iter().map(|(entry, _)| entry));
        retained.sort_by_key(|entry| entry.anchor_z);
        entries.clear();
        entries.extend(retained);
        for entry in entries.drain(..) {
            output
                .push_support_plan_entry(entry)
                .map_err(|e| ModuleError::fatal(1, format!("push_support_plan failed: {e}")))?;
        }
        Ok(())
    }
}

/// Packet 239c (Step 2): canonical `generate_support_layers`
/// (`Support/SupportCommon.cpp`) intermediate-row stepping for one vertical
/// gap between two bracketing support rows.
///
/// Canonical rule (flag-independent there; gated here by
/// `independent_support_layer_height`):
/// `n_layers_extra = ceil((dist - EPSILON) / max_support_layer_height)`,
/// `step = dist / n_layers_extra`, `print_z = bottom_z + k * step`.
///
/// `below_z_mm` / `above_z_mm` are the two bracketing support planes
/// (ascending, mm). Returns the k = 1..n rows strictly between the brackets,
/// in canonical units, ascending. EPSILON is canonical's 1e-4 mm — exactly
/// one canonical unit. Insertion happens only when the configured pitch is
/// finer than the gap (`n >= 2`), so the bracketing grid planes themselves
/// never move and no plane is duplicated, deleted, or inverted.
/// Deterministic: pure function of the pair plus the pitch. Mirrors the
/// identically-named helper in `tree-support-planner/src/lib.rs`.
fn packet239c_intermediate_planes(below_z_mm: f32, above_z_mm: f32, pitch_mm: f64) -> Vec<i64> {
    const EPSILON_MM: f64 = 1e-4;
    let below_units = slicer_ir::mm_to_units(below_z_mm);
    let above_units = slicer_ir::mm_to_units(above_z_mm);
    if pitch_mm <= 0.0 || above_units <= below_units {
        return Vec::new();
    }
    let dist = (above_z_mm - below_z_mm) as f64;
    let n = ((dist - EPSILON_MM) / pitch_mm).ceil();
    if n < 2.0 {
        return Vec::new();
    }
    let step = dist / n;
    let n = n as i64;
    (1..n)
        .map(|k| slicer_ir::mm_to_units((below_z_mm as f64 + k as f64 * step) as f32))
        .filter(|plane| *plane > below_units && *plane < above_units)
        .collect()
}

fn packet239d_coarse_planes(below_units: i64, above_units: i64, pitch_mm: f64) -> Vec<f64> {
    const EPSILON_MM: f64 = 1e-4;
    if pitch_mm <= 0.0 || above_units <= below_units {
        return Vec::new();
    }
    let below_mm = below_units as f64 / slicer_ir::UNITS_PER_MM;
    let above_mm = above_units as f64 / slicer_ir::UNITS_PER_MM;
    let dist = above_mm - below_mm;
    let n = ((dist - EPSILON_MM) / pitch_mm).ceil().max(1.0) as i64;
    (1..=n)
        .map(|k| {
            if k == n {
                above_mm
            } else {
                below_mm + dist * k as f64 / n as f64
            }
        })
        // The aligned upper plane is the surviving real bracket entry, not a
        // synthesized body clone.
        .filter(|plane| *plane > below_mm && *plane < above_mm)
        .collect()
}

fn next_intermediate_plane_index(
    intermediate_plane_indices: &mut BTreeMap<i64, i32>,
    plane: i64,
) -> Result<i32, ModuleError> {
    let plane_ordinal = i32::try_from(intermediate_plane_indices.len())
        .map_err(|_| ModuleError::fatal(1, "too many intermediate traditional-support planes"))?;
    let next_index = i32::MIN
        .checked_add(plane_ordinal)
        .ok_or_else(|| ModuleError::fatal(1, "too many intermediate traditional-support planes"))?;
    Ok(*intermediate_plane_indices
        .entry(plane)
        .or_insert(next_index))
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
