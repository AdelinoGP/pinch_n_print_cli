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

use slicer_ir::SupportPlanDeclineReason;
use slicer_sdk::prelude::*;

/// Default number of dense interface layers at the top of a support column.
const DEFAULT_INTERFACE_TOP_LAYERS: i32 = 2;
/// Default number of dense interface layers at the bottom of a support column.
/// `-1` means "mirror the top interface count" (OrcaSlicer convention).
const DEFAULT_INTERFACE_BOTTOM_LAYERS: i32 = -1;
/// Default base fill pattern.
const DEFAULT_BASE_PATTERN: &str = "rectilinear";
/// Default mesh-facet overhang threshold, matching OrcaSlicer.
const DEFAULT_OVERHANG_ANGLE_DEG: f32 = 45.0;

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
    /// Downward normal threshold for mesh-facet overhang detection.
    support_overhang_angle: f32,
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
            _ => 0.0,
        };
        let support_layer_height_mm = match config.get("support_layer_height_mm") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => 0.0,
        };
        let support_overhang_angle = match config.get("support_overhang_angle") {
            Some(ConfigValue::Float(v)) => *v as f32,
            Some(ConfigValue::Int(v)) => *v as f32,
            _ => DEFAULT_OVERHANG_ANGLE_DEG,
        };
        Ok(Self {
            enabled,
            support_family,
            support_interface_top_layers,
            support_interface_bottom_layers,
            support_base_pattern,
            support_top_z_distance_mm,
            support_layer_height_mm,
            support_overhang_angle,
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
                    if candidate_family(candidate, support_analysis, &self.support_family)
                        == "traditional"
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
            let family = candidate_family(candidate, support_analysis, &self.support_family);
            if family != "traditional" {
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

        // Candidate geometry is the full region cross-section. Derive contact
        // from downward-facing mesh facets whose Z span crosses this layer.
        let layer = &layer_plan.layers[contact_layer as usize];
        let slab_bottom = layer.z - layer.effective_layer_height;
        let contact_facets = overhang_facets(obj, self.support_overhang_angle)
            .into_iter()
            .filter(|(vertices, _)| {
                let min_z = vertices
                    .iter()
                    .map(|vertex| vertex[2])
                    .fold(f32::INFINITY, f32::min);
                let max_z = vertices
                    .iter()
                    .map(|vertex| vertex[2])
                    .fold(f32::NEG_INFINITY, f32::max);
                max_z >= slab_bottom && min_z <= layer.z
            })
            .filter_map(|(_vertices, polygon)| {
                let facet = ExPolygon {
                    contour: polygon,
                    holes: vec![],
                };
                let clipped =
                    host::clip_polygons(&candidate_geometry, &[facet], ClipOperation::Intersection);
                if clipped.is_empty() {
                    None
                } else {
                    Some(clipped)
                }
            })
            .flatten()
            .collect::<Vec<_>>();
        let contact_geometry = contact_facets;
        if contact_geometry.is_empty() {
            return push_declined(
                output,
                obj,
                candidate,
                demand_id,
                SupportPlanDeclineReason::NoRoute,
            );
        }

        let model_layer_height = layer_plan.layers[contact_layer as usize].effective_layer_height;
        let top_offset = if model_layer_height > 0.0 {
            (self.support_top_z_distance_mm / model_layer_height).ceil() as u32
        } else {
            0
        };
        let emit_top_layer = contact_layer.saturating_sub(top_offset);

        // Prefer the highest eligible model termination reached during descent.
        // An empty analysis list preserves the plate fallback contract.
        let termination_layer = support_analysis
            .termination_surfaces
            .iter()
            .filter(|surface| {
                surface.object_id == obj.object_id
                    && surface.region_id == candidate.region_id
                    && surface.global_support_layer_index < emit_top_layer
                    && expolygons_overlap(&contact_geometry, &surface.polygons)
            })
            .map(|surface| surface.global_support_layer_index)
            .max()
            .unwrap_or(0);

        // Non-termination occupancy still rejects the complete body. The
        // selected termination layer is intentionally permitted to touch its
        // model surface.
        for layer in termination_layer..contact_layer {
            if layer == termination_layer {
                continue;
            }
            let occupancy = occupancy_at(
                support_analysis,
                &obj.object_id,
                &candidate.region_id,
                layer,
            );
            if !occupancy.is_empty() && expolygons_overlap(&contact_geometry, &occupancy) {
                let _ = output.push_diagnostic(Diagnostic {
                    severity: DiagnosticSeverity::Warn,
                    code: 1203,
                    layer: Some(layer as i32),
                    object_id: Some(obj.object_id.clone()),
                    message: format!(
                        "traditional body rejected: complete body intersects model occupancy at layer {layer}"
                    ),
                });
                return Ok(());
            }
        }

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
        for layer in (termination_layer..=emit_top_layer).rev() {
            let is_interface_layer = (top_layers > 0
                && layer >= emit_top_layer.saturating_sub(top_layers - 1))
                || (bottom_layers > 0 && layer < termination_layer + bottom_layers);
            if (emit_top_layer - layer) % support_step != 0 && !is_interface_layer {
                continue;
            }
            let mut roles = vec![slicer_ir::SupportPlanRoleRegion {
                role: slicer_ir::SupportPlanRole::SupportBody,
                regions: contact_geometry.clone(),
            }];
            if top_layers > 0 && layer >= emit_top_layer.saturating_sub(top_layers - 1) {
                roles.push(slicer_ir::SupportPlanRoleRegion {
                    role: slicer_ir::SupportPlanRole::TopInterface,
                    regions: contact_geometry.clone(),
                });
            }
            if bottom_layers > 0 && layer < termination_layer + bottom_layers {
                roles.push(slicer_ir::SupportPlanRoleRegion {
                    role: slicer_ir::SupportPlanRole::BottomInterface,
                    regions: contact_geometry.clone(),
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

fn overhang_facets(obj: &MeshObjectView, threshold_deg: f32) -> Vec<([[f32; 3]; 3], Polygon)> {
    let threshold_nz = -(threshold_deg.to_radians().sin());
    let mut result = Vec::new();
    for triangle in &obj.triangles {
        let vertices = [
            obj.vertices[triangle[0] as usize],
            obj.vertices[triangle[1] as usize],
            obj.vertices[triangle[2] as usize],
        ];
        let e1 = [
            vertices[1][0] - vertices[0][0],
            vertices[1][1] - vertices[0][1],
            vertices[1][2] - vertices[0][2],
        ];
        let e2 = [
            vertices[2][0] - vertices[0][0],
            vertices[2][1] - vertices[0][1],
            vertices[2][2] - vertices[0][2],
        ];
        let nz = e1[0] * e2[1] - e1[1] * e2[0];
        let nx = e1[1] * e2[2] - e1[2] * e2[1];
        let ny = e1[2] * e2[0] - e1[0] * e2[2];
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len < 1e-8 || nz / len > threshold_nz {
            continue;
        }
        result.push((
            vertices,
            Polygon {
                points: vertices
                    .iter()
                    .map(|vertex| Point2::from_mm(vertex[0], vertex[1]))
                    .collect(),
            },
        ));
    }
    result
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
fn candidate_family(
    candidate: &SupportAnalysisCandidate,
    analysis: &SupportAnalysisView,
    default_family: &str,
) -> String {
    analysis
        .family_assignments
        .iter()
        .find(|assignment| {
            assignment.object_id == candidate.object_id
                && assignment.region_id == candidate.region_id
        })
        .map(|assignment| canonical_support_family_alias(Some(&assignment.family_id)))
        .unwrap_or_else(|| default_family.to_string())
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
    let value = value.unwrap_or("traditional");
    if value.starts_with("tree") || value.starts_with("hybrid") {
        "tree".to_string()
    } else {
        "traditional".to_string()
    }
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
