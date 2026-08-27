// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, which is licensed under
// the GNU Affero General Public License, version 3 (AGPLv3).
// -----------------------------------------------------------------------------
//! Canonical support-flow density and interface-ratio semantics.
//!
//! These formulas follow the `SupportParameters` constructor in
//! `SupportParameters.hpp`: configured spacing is combined with the resolved
//! flow spacing, and density is the flow-spacing fraction of that pitch.

use crate::flow::{line_width_to_spacing, NegativeSpacingError};

fn density(flow_spacing: f32, configured_spacing: f32) -> f32 {
    (flow_spacing / (configured_spacing + flow_spacing)).min(1.0)
}

/// Derive support-body density from the resolved line width and layer height.
pub fn body_density(
    width: f32,
    layer_height: f32,
    base_pattern_spacing: f32,
) -> Result<f32, NegativeSpacingError> {
    let flow_spacing = line_width_to_spacing(width, layer_height)?;
    Ok(density(flow_spacing, base_pattern_spacing))
}

/// Derive top-interface density from the resolved line width and layer height.
pub fn interface_density(
    width: f32,
    layer_height: f32,
    interface_spacing: f32,
) -> Result<f32, NegativeSpacingError> {
    let flow_spacing = line_width_to_spacing(width, layer_height)?;
    Ok(density(flow_spacing, interface_spacing))
}

/// Derive bottom-interface density from the resolved line width and layer height.
pub fn bottom_interface_density(
    width: f32,
    layer_height: f32,
    bottom_interface_spacing: f32,
) -> Result<f32, NegativeSpacingError> {
    let flow_spacing = line_width_to_spacing(width, layer_height)?;
    Ok(density(flow_spacing, bottom_interface_spacing))
}

/// Resolve an interface flow percentage, falling back to canonical 100 percent.
pub fn resolved_interface_flow_ratio(percent: f32) -> f32 {
    if percent <= 0.0 {
        100.0
    } else {
        percent
    }
}

use crate::polygon_ops::{difference_ex, intersection_ex, offset, union_ex, OffsetJoinType};
use slicer_ir::{ExPolygon, SupportPlanRole, SupportPlanRoleRegion};

/// Canonical `SUPPORT_SURFACES_OFFSET_PARAMETERS` is `ClipperLib::jtSquare, 0.`
/// -- square joins, and a miter limit that square joins never consult.
const SUPPORT_SURFACES_JOIN: OffsetJoinType = OffsetJoinType::Square;

/// Canonical `smoothing_distance` is `interface_flow.scaled_spacing() * 1.5`.
const SMOOTHING_SPACING_FACTOR: f32 = 1.5;

/// Canonical `regularize`: an asymmetric `closing` that nets out to a
/// `minimum_island_radius` expansion, then an outward smoothing pass.
///
/// `closing(polys, delta1, delta2)` is `shrink(expand(polys, delta1), delta2)`
/// (canonical `ClipperUtils.cpp::closing`), so the expansion leg carries the
/// island radius and the erosion leg does not.
fn regularize(
    polys: &[ExPolygon],
    closing_distance_mm: f32,
    minimum_island_radius_mm: f32,
    smoothing_distance_mm: f32,
    smooth_supports: bool,
) -> Vec<ExPolygon> {
    if polys.is_empty() {
        return Vec::new();
    }
    if !smooth_supports {
        // Canonical's grid-style branch: `union_safety_offset`, i.e. a plain
        // union. No smoothing, no island radius.
        return union_ex(polys);
    }
    let expanded = offset(
        polys,
        closing_distance_mm + minimum_island_radius_mm,
        SUPPORT_SURFACES_JOIN,
        0.0,
    );
    let closed = offset(&expanded, -closing_distance_mm, SUPPORT_SURFACES_JOIN, 0.0);
    crate::smooth_outward(&closed, slicer_ir::mm_to_units(smoothing_distance_mm))
}

/// Canonical `minimum_island_radius` for one interface side:
/// `interface_flow.scaled_spacing() / interface_density`, where
/// `interface_density = min(1, spacing / interface_pitch)`.
fn minimum_island_radius_mm(flow_spacing_mm: f32, interface_pitch_mm: f32) -> f32 {
    if interface_pitch_mm <= 0.0 {
        return flow_spacing_mm;
    }
    let density = (flow_spacing_mm / interface_pitch_mm).min(1.0);
    if density <= 0.0 {
        flow_spacing_mm
    } else {
        flow_spacing_mm / density
    }
}

/// Regularized role regions for one support plan entry.
///
/// Returns `None` when there is nothing to do (no interface role, or a
/// degenerate flow spacing), in which case the caller must render
/// `entry.roles` unchanged.
///
/// `smooth_supports` mirrors canonical `support_params.support_style != smsGrid`.
pub fn regularize_entry_roles(
    roles: &[SupportPlanRoleRegion],
    flow_spacing_mm: f32,
    top_interface_pitch_mm: f32,
    bottom_interface_pitch_mm: f32,
    smooth_supports: bool,
) -> Option<Vec<(SupportPlanRole, Vec<ExPolygon>)>> {
    if flow_spacing_mm <= 0.0 {
        return None;
    }

    let mut top: Vec<ExPolygon> = Vec::new();
    let mut bottom: Vec<ExPolygon> = Vec::new();
    let mut raft: Vec<ExPolygon> = Vec::new();
    // Canonical `intermediate_layer.polygons` before the interface is carved
    // out: the whole printed cross-section of this entry at this layer. Raft
    // geometry is not part of a support layer's intermediate area, so it is
    // excluded and passed through untouched.
    let mut layer_area_src: Vec<ExPolygon> = Vec::new();
    let mut order: Vec<SupportPlanRole> = Vec::new();
    for role_region in roles {
        if !order.contains(&role_region.role) {
            order.push(role_region.role);
        }
        match role_region.role {
            SupportPlanRole::TopInterface => {
                top.extend(role_region.regions.iter().cloned());
                layer_area_src.extend(role_region.regions.iter().cloned());
            }
            // Base interfaces belong to the interface family for band subtraction.
            SupportPlanRole::BaseInterface => {
                top.extend(role_region.regions.iter().cloned());
                layer_area_src.extend(role_region.regions.iter().cloned());
            }
            SupportPlanRole::BottomInterface => {
                bottom.extend(role_region.regions.iter().cloned());
                layer_area_src.extend(role_region.regions.iter().cloned());
            }
            SupportPlanRole::SupportBody => {
                layer_area_src.extend(role_region.regions.iter().cloned());
            }
            SupportPlanRole::RaftRelated => raft.extend(role_region.regions.iter().cloned()),
        }
    }

    if top.is_empty() && bottom.is_empty() {
        return None;
    }
    let layer_area = union_ex(&layer_area_src);
    if layer_area.is_empty() {
        return None;
    }

    let smoothing_distance_mm = flow_spacing_mm * SMOOTHING_SPACING_FACTOR;
    let closing_distance_mm = smoothing_distance_mm;
    let reg_top = regularize(
        &top,
        closing_distance_mm,
        minimum_island_radius_mm(flow_spacing_mm, top_interface_pitch_mm),
        smoothing_distance_mm,
        smooth_supports,
    );
    let reg_bottom = regularize(
        &bottom,
        closing_distance_mm,
        minimum_island_radius_mm(flow_spacing_mm, bottom_interface_pitch_mm),
        smoothing_distance_mm,
        smooth_supports,
    );

    // Canonical clips the regularized union back to the intermediate layer.
    let iface_top = intersection_ex(&reg_top, &layer_area);
    let iface_bottom = difference_ex(&intersection_ex(&reg_bottom, &layer_area), &iface_top);
    let mut iface_all = iface_top.clone();
    iface_all.extend(iface_bottom.iter().cloned());
    let iface_all = union_ex(&iface_all);
    let body = difference_ex(&layer_area, &iface_all);

    let mut out: Vec<(SupportPlanRole, Vec<ExPolygon>)> = Vec::new();
    for role in &order {
        let regions = match role {
            SupportPlanRole::TopInterface => &iface_top,
            SupportPlanRole::BaseInterface => &iface_top,
            SupportPlanRole::BottomInterface => &iface_bottom,
            SupportPlanRole::SupportBody => &body,
            SupportPlanRole::RaftRelated => &raft,
        };
        if !regions.is_empty() {
            out.push((*role, regions.clone()));
        }
    }
    if !order.contains(&SupportPlanRole::SupportBody) && !body.is_empty() {
        out.push((SupportPlanRole::SupportBody, body));
    }
    Some(out)
}
