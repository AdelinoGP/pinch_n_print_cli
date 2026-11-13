// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Brim.cpp / src/libslic3r/Print.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Canonical skirt/brim geometry helpers.
//!
//! Ported from OrcaSlicer at the pinned `v2.5.0` (`40ecc66f78`):
//! `make_brim` and `tryExPolygonOffset` (`Brim.cpp`) for the brim ring, and
//! `Print::_make_skirt` (`Print.cpp`) for the convex-hull-anchored skirt.
//!
//! Everything here works in millimetres at the module boundary and delegates
//! boolean/offset work to the host geometry services, so the guest and the
//! native (integrated edition) build share one implementation.

use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::host::{self, ClipOperation, OffsetJoinType};

/// Arc tolerance OrcaSlicer uses for brim offsets (`SCALED_RESOLUTION`,
/// 0.0125 mm at the pinned revision).
pub(crate) const BRIM_ARC_TOLERANCE_MM: f32 = 0.0125;

/// Arc tolerance `Print::_make_skirt` passes to `offset(hull, ..., jtRound,
/// float(scale_(0.1)))`.
pub(crate) const SKIRT_ARC_TOLERANCE_MM: f32 = 0.1;

/// Two millimetre-space points are the same vertex within this distance.
const POINT_EPS_MM: f32 = 1e-4;

/// Converts a closed extrusion loop (as stamped on the IR) into a filled
/// [`ExPolygon`] contour. Returns `None` when the loop is degenerate.
///
/// The IR repeats the first point to close a loop; Clipper2 wants the closing
/// vertex implicit, so the duplicate is dropped here. Collinear vertices are
/// left alone — the host's clipper normalises them.
pub(crate) fn closed_loop_to_ex_polygon(points: &[(f32, f32)]) -> Option<ExPolygon> {
    let mut kept: Vec<(f32, f32)> = Vec::with_capacity(points.len());
    for &(x, y) in points {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        match kept.last() {
            Some(&(px, py)) if (px - x).abs() < POINT_EPS_MM && (py - y).abs() < POINT_EPS_MM => {}
            _ => kept.push((x, y)),
        }
    }
    if kept.len() >= 2 {
        let (fx, fy) = kept[0];
        let (lx, ly) = *kept.last().expect("non-empty");
        if (fx - lx).abs() < POINT_EPS_MM && (fy - ly).abs() < POINT_EPS_MM {
            kept.pop();
        }
    }
    if kept.len() < 3 {
        return None;
    }
    Some(ExPolygon {
        contour: Polygon {
            points: kept.iter().map(|&(x, y)| Point2::from_mm(x, y)).collect(),
        },
        holes: Vec::new(),
    })
}

/// Morphological closing of a polygon set (`offset(+r)` then `offset(-r)`).
///
/// Used to reconstruct a support *region* from its bead centerlines: canonical
/// `make_brim` subtracts the support slice contour, which the IR does not carry,
/// so parallel beads are merged into their island before the brim is clipped.
pub(crate) fn closing(
    polys: &[ExPolygon],
    radius_mm: f32,
    arc_tolerance_mm: f32,
) -> Vec<ExPolygon> {
    if polys.is_empty() || radius_mm <= 0.0 {
        return polys.to_vec();
    }
    let grown = offset(polys, radius_mm, OffsetJoinType::Round, arc_tolerance_mm);
    offset(&grown, -radius_mm, OffsetJoinType::Round, arc_tolerance_mm)
}

/// Builds the covered-footprint polygon set of a polyline with a bead of
/// `width` mm: one rectangle per segment, whose union is the area the bead
/// sweeps. Used to reconstruct a support pad's footprint from its extrusion
/// centerlines (`polygons_covered_by_spacing()` in canonical `make_brim`).
pub(crate) fn polyline_strips(points: &[(f32, f32)], width: f32) -> Vec<ExPolygon> {
    let half = (width * 0.5).max(0.0);
    let mut out = Vec::with_capacity(points.len().saturating_sub(1));
    for pair in points.windows(2) {
        let (ax, ay) = pair[0];
        let (bx, by) = pair[1];
        let (dx, dy) = (bx - ax, by - ay);
        let len = (dx * dx + dy * dy).sqrt();
        if len < POINT_EPS_MM || half <= 0.0 {
            continue;
        }
        let (nx, ny) = (-dy / len * half, dx / len * half);
        let corners = [
            (ax + nx, ay + ny),
            (bx + nx, by + ny),
            (bx - nx, by - ny),
            (ax - nx, ay - ny),
        ];
        out.push(ExPolygon {
            contour: Polygon {
                points: corners
                    .iter()
                    .map(|&(x, y)| Point2::from_mm(x, y))
                    .collect(),
            },
            holes: Vec::new(),
        });
    }
    out
}

/// Andrew's monotone-chain convex hull over millimetre-space points.
///
/// Mirrors `Slic3r::Geometry::convex_hull` (`Geometry.cpp`) as used by
/// `Print::_make_skirt`: duplicate and interior points are removed, and the
/// result is a closed-compatible ring without a repeated first vertex.
pub(crate) fn convex_hull(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut pts: Vec<(f32, f32)> = points
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .collect();
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts.dedup_by(|a, b| (a.0 - b.0).abs() < POINT_EPS_MM && (a.1 - b.1).abs() < POINT_EPS_MM);
    if pts.len() < 3 {
        return pts;
    }

    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| -> f32 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let mut lower: Vec<(f32, f32)> = Vec::with_capacity(pts.len());
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(f32, f32)> = Vec::with_capacity(pts.len());
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Unions a set of filled polygons through the host clip service.
///
/// `host::clip_polygons` with [`ClipOperation::Union`] and an empty clip set is
/// the union of the subject set (Clipper2 `Union` against nothing).
pub(crate) fn union(polys: Vec<ExPolygon>) -> Vec<ExPolygon> {
    if polys.is_empty() {
        return Vec::new();
    }
    host::clip_polygons(&polys, &[], ClipOperation::Union)
}

/// Offsets a polygon set through the host offset service. Positive `delta_mm`
/// inflates (moves outward), negative deflates.
pub(crate) fn offset(
    polys: &[ExPolygon],
    delta_mm: f32,
    join: OffsetJoinType,
    arc_tolerance_mm: f32,
) -> Vec<ExPolygon> {
    if polys.is_empty() || delta_mm.abs() < POINT_EPS_MM {
        return polys.to_vec();
    }
    host::offset_polygons(polys, delta_mm, join, arc_tolerance_mm)
}

/// Modal non-zero point width of a wall loop, in mm, with ties broken by the
/// first occurrence.
///
/// The IR stamps one width per point. Canonical `make_brim` anchors on the
/// slice contour, which is half a wall width outside the wall centerline; the
/// uniform reconstruction uses the loop's modal width, so a classic (uniform)
/// wall returns exactly its width and an Arachne wall returns its dominant
/// width. Returns `None` when every width is zero or non-finite.
pub(crate) fn modal_non_zero_width(widths: &[f32]) -> Option<f32> {
    // (bit pattern, count, first index), small enough that a linear scan beats
    // a hash map and keeps first-occurrence order explicit.
    let mut groups: Vec<(u32, usize, usize)> = Vec::new();
    for (index, &width) in widths.iter().enumerate() {
        if !width.is_finite() || width <= 0.0 {
            continue;
        }
        let bits = width.to_bits();
        match groups.iter_mut().find(|group| group.0 == bits) {
            Some(group) => group.1 += 1,
            None => groups.push((bits, 1, index)),
        }
    }
    groups
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.2.cmp(&a.2)))
        .map(|(bits, _, _)| f32::from_bits(bits))
}

/// Area of a polygon ring in mm² (shoelace). Used only to pick the outer
/// contour of an offset result, never as a geometry input.
pub(crate) fn ring_area_mm2(contour: &Polygon) -> f32 {
    let pts: Vec<(f32, f32)> = contour.points.iter().map(Point2::to_mm).collect();
    if pts.len() < 3 {
        return 0.0;
    }
    let mut twice_area = 0.0f32;
    for i in 0..pts.len() {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % pts.len()];
        twice_area += x0 * y1 - x1 * y0;
    }
    (twice_area * 0.5).abs()
}
