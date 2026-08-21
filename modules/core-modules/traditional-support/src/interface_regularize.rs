// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Support/SupportCommon.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Interface regularization (F-37, wiring half).
//!
//! Port of the `regularize` lambda inside canonical `generate_interface_layers`
//! (`SupportCommon.cpp`) and of the way its result is folded back into the
//! layer:
//!
//! ```text
//! regularize(polys, r) = smooth_supports
//!     ? smooth_outward(closing(polys, closing_distance + r, closing_distance,
//!                              jtSquare, 0.), smoothing_distance)
//!     : union_safety_offset(polys)
//! bottom = intersection(regularize(bottom, r_bottom) + regularize(top, r_top),
//!                       intermediate_layer.polygons)
//! intermediate_layer.polygons = diff(intermediate_layer.polygons, bottom)
//! ```
//!
//! Distances, all from canonical `SupportParameters`:
//! * `smoothing_distance = support_material_interface_flow.scaled_spacing() * 1.5`
//! * `closing_distance   = smoothing_distance`
//! * `minimum_island_radius_top    = interface_flow.scaled_spacing() / top_interface_density`
//! * `minimum_island_radius_bottom = interface_flow.scaled_spacing() / bottom_interface_density`
//!
//! with `top_interface_density = min(1, spacing / top_interface_spacing)`, so
//! the radius reduces to `max(flow_spacing, interface_pitch)`. Every distance
//! here is derived in **millimetres** from the in-tree flow helpers and is only
//! converted with `slicer_ir::mm_to_units` at the `smooth_outward` boundary --
//! no canonical `coord_t` (1 nm) literal is carried across, so there is no
//! 100x conversion hazard.
//!
//! # Coverage safety
//!
//! `smooth_outward` is **not** a strict superset of its input: canonical
//! `clear()`s a ring that degenerates while clipping, so an interface island
//! can vanish outright. That is handled the same way canonical handles it --
//! the base area is computed as `layer_area - interface`, never as "whatever
//! role the planner originally labelled body". A deleted interface ring
//! therefore falls back to printing as support **body**, exactly as canonical's
//! `intermediate_layer.polygons` keeps whatever the interface did not claim.
//! Support is never silently dropped.

use slicer_core::polygon_ops::{difference_ex, intersection_ex, offset, union_ex, OffsetJoinType};
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
    slicer_core::smooth_outward(&closed, slicer_ir::mm_to_units(smoothing_distance_mm))
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
pub(crate) fn regularize_entry_roles(
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
    // Canonical: `closing_distance = smoothing_distance`.
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

    // Canonical clips the regularized union back to the intermediate layer, so
    // the net island-radius expansion can never print outside the column.
    let iface_top = intersection_ex(&reg_top, &layer_area);
    // Canonical merges both sides into one interface layer; in-tree they stay
    // two roles with two fill pitches, so the overlap is resolved in favour of
    // the roof to keep the area extruded exactly once.
    let iface_bottom = difference_ex(&intersection_ex(&reg_bottom, &layer_area), &iface_top);

    let mut iface_all = iface_top.clone();
    iface_all.extend(iface_bottom.iter().cloned());
    let iface_all = union_ex(&iface_all);

    // Canonical `intermediate_layer.polygons = diff(intermediate_layer.polygons, interface)`.
    let body = difference_ex(&layer_area, &iface_all);

    let mut out: Vec<(SupportPlanRole, Vec<ExPolygon>)> = Vec::new();
    for role in &order {
        let regions = match role {
            SupportPlanRole::TopInterface => &iface_top,
            SupportPlanRole::BottomInterface => &iface_bottom,
            SupportPlanRole::SupportBody => &body,
            SupportPlanRole::RaftRelated => &raft,
        };
        if !regions.is_empty() {
            out.push((*role, regions.clone()));
        }
    }
    if !order.contains(&SupportPlanRole::SupportBody) && !body.is_empty() {
        // The planner labelled the whole cross-section interface, but
        // regularization did not claim all of it (canonical `smooth_outward`
        // `clear()`s a degenerating ring). The remainder prints as base,
        // matching canonical's `intermediate_layer.polygons`.
        out.push((SupportPlanRole::SupportBody, body));
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{mm_to_units, Point2, Polygon};

    fn square(min_mm: f32, max_mm: f32) -> ExPolygon {
        let lo = mm_to_units(min_mm);
        let hi = mm_to_units(max_mm);
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: lo, y: lo },
                    Point2 { x: hi, y: lo },
                    Point2 { x: hi, y: hi },
                    Point2 { x: lo, y: hi },
                ],
            },
            holes: Vec::new(),
        }
    }

    fn ring_area(pts: &[Point2]) -> f64 {
        let mut acc = 0.0_f64;
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            acc += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
        }
        acc / 2.0
    }

    fn area_mm2(polys: &[ExPolygon]) -> f64 {
        polys
            .iter()
            .map(|e| {
                let mut a = ring_area(&e.contour.points).abs();
                for h in &e.holes {
                    a -= ring_area(&h.points).abs();
                }
                a
            })
            .sum::<f64>()
            / (slicer_ir::UNITS_PER_MM * slicer_ir::UNITS_PER_MM)
    }

    fn role_region(role: SupportPlanRole, regions: Vec<ExPolygon>) -> SupportPlanRoleRegion {
        SupportPlanRoleRegion { role, regions }
    }

    /// The regularized interface is clipped back to the layer cross-section, so
    /// it never grows outside the column despite the net island-radius
    /// expansion inside `closing`.
    #[test]
    fn regularized_interface_never_exceeds_layer_area() {
        let src = vec![role_region(
            SupportPlanRole::TopInterface,
            vec![square(0.0, 10.0)],
        )];
        let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).expect("regularized");
        let iface: Vec<ExPolygon> = out
            .iter()
            .filter(|(r, _)| *r == SupportPlanRole::TopInterface)
            .flat_map(|(_, p)| p.iter().cloned())
            .collect();
        assert!(!iface.is_empty(), "roof must survive regularization");
        assert!(
            area_mm2(&iface) <= 100.0 + 1e-3,
            "interface area {} exceeded the 10x10 mm layer area",
            area_mm2(&iface)
        );
    }

    /// Consumer-warning coverage: `smooth_outward` can DELETE a ring, and
    /// `closing`'s erosion leg can dissolve a feature smaller than the closing
    /// distance. Whatever the interface does not claim must reappear as support
    /// body -- total printed coverage is conserved, support is never lost.
    #[test]
    fn small_feature_interface_never_loses_coverage() {
        // A 0.15 mm island against a 0.35 mm flow spacing: the closing /
        // smoothing distance (0.525 mm) is several times the feature size.
        let src = vec![role_region(
            SupportPlanRole::TopInterface,
            vec![square(0.0, 0.15)],
        )];
        let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).expect("regularized");
        assert!(
            !out.is_empty(),
            "a degenerating interface ring must not delete the support entirely"
        );
        let covered: Vec<ExPolygon> = out.iter().flat_map(|(_, p)| p.iter().cloned()).collect();
        let covered_area = area_mm2(&covered);
        assert!(
            (covered_area - 0.0225).abs() < 1e-4,
            "total coverage must equal the 0.15x0.15 mm input area, got {covered_area}"
        );
    }

    /// Grid style takes canonical's `union_safety_offset` branch: no smoothing,
    /// no island radius, so the partition the planner produced is preserved.
    #[test]
    fn grid_style_does_not_smooth() {
        let src = vec![role_region(
            SupportPlanRole::TopInterface,
            vec![square(0.0, 0.15)],
        )];
        let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, false).expect("regularized");
        let iface: Vec<ExPolygon> = out
            .iter()
            .filter(|(r, _)| *r == SupportPlanRole::TopInterface)
            .flat_map(|(_, p)| p.iter().cloned())
            .collect();
        assert!(
            (area_mm2(&iface) - 0.0225).abs() < 1e-4,
            "grid style must leave the interface area untouched"
        );
    }

    /// No interface role means no regularization pass at all.
    #[test]
    fn body_only_entry_is_untouched() {
        let src = vec![role_region(
            SupportPlanRole::SupportBody,
            vec![square(0.0, 10.0)],
        )];
        assert!(regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).is_none());
    }

    /// Roof and floor may not both claim the same area: an area extruded twice
    /// is a double-extrusion bug, not a coverage improvement.
    #[test]
    fn roof_and_floor_do_not_overlap() {
        let src = vec![
            role_region(SupportPlanRole::TopInterface, vec![square(0.0, 10.0)]),
            role_region(SupportPlanRole::BottomInterface, vec![square(0.0, 10.0)]),
        ];
        let out = regularize_entry_roles(&src, 0.35, 0.75, 0.75, true).expect("regularized");
        let total: f64 = out.iter().map(|(_, p)| area_mm2(p)).sum();
        assert!(
            total <= 100.0 + 1e-3,
            "summed role areas {total} exceed the 10x10 mm layer area (double extrusion)"
        );
    }
}

