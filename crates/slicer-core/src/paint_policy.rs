//! Per-region support paint policy resolution.
//!
//! Computes whether a slice region should be treated as `Blocked`,
//! `Enforced`, or `DefaultEligible` for the `Layer::Support` paint
//! precedence rules (see `docs/01_system_architecture.md` §"Support Stage
//! Paint Precedence"). The evaluation is a presence check on the region's
//! `segment_annotations` map combined with a region-area floor — the
//! `SlicedRegion` IR stores paint as per-polygon per-vertex `Some(_)` flags
//! (see `crates/slicer-ir/src/slice_ir.rs` §SlicedRegion), NOT as separate
//! annotation polygons, so a polygon-intersection area between region and
//! "painted polygons" is not the mechanism. The "non-trivial region area"
//! floor is used purely to suppress degenerate / empty regions from being
//! classified as enforcer or blocker. Region-ownership (which cell of the
//! `SliceIR` does the caller's expoly belong to) is a separate check that
//! `PaintRegionLayerView::paint_policy_for` performs at the call site; this
//! helper only classifies the per-region result.
//!
//! This module is the geometric replacement for the centroid-probe in
//! `PaintRegionLayerView::paint_policy_for`
//! (`crates/slicer-sdk/src/traits.rs`). Step 3 of packet 120 wires the host
//! to call `support_eligibility` per region and aggregate the per-region
//! results.
//!
//! # Crate layering
//!
//! `slicer-core` does not depend on `slicer-sdk` (the dependency arrow
//! points the other way: `slicer-sdk` depends on `slicer-core`, so a
//! reverse dep would form a Cargo cycle). The packet's prose asked for
//! `support_eligibility(region: &SliceRegionView)` where
//! `SliceRegionView` lives in `slicer-sdk`; that signature is not
//! compilable here without breaking the cycle, so the function instead
//! takes the two fields the helper needs as direct arguments. The
//! caller-side adapter (Step 3) extracts `view.polygons()` and
//! `view.segment_annotations()` from the `SliceRegionView` and passes
//! them in. This is a one-line difference at the call site and keeps the
//! helper free of the cycle.

use std::collections::HashMap;

use slicer_ir::{ExPolygon, PaintSemantic, PaintValue};

// Canonical `SupportPaintPolicy` lives in `slicer_ir::paint_policy` (no
// `slicer-core` ↔ `slicer-sdk` cycle). Re-export so the
// `slicer_core::paint_policy::SupportPaintPolicy` path continues to
// resolve for downstream callers and tests.
pub use slicer_ir::paint_policy::SupportPaintPolicy;

/// Area threshold (workspace scaled units²) below which a paint annotation
/// is considered to not meaningfully cover the region polygon.
///
/// 1 workspace unit = 100 nm = 10⁻⁴ mm, so 1 unit² = 10⁻⁸ mm². The
/// `docs/01_system_architecture.md` §"Support Stage Paint Precedence" rule
/// uses 1 µm² as the "non-trivial" floor; in workspace units that is
/// `1e-6 mm² / 1e-8 mm²·unit⁻² = 100 unit²`. We round up to 200 unit²
/// (≈ 2 µm²) so numeric noise from clipper2 set-ops on small polygons does
/// not flip a borderline case to `Enforced`/`Blocked`.
const NON_TRIVIAL_AREA_UNITS_SQ: i128 = 200;

/// Compute the support-paint policy for a single region via a presence
/// check on `segment_annotations` and a region-area floor.
///
/// The IR shape (`SlicedRegion.segment_annotations: HashMap<PaintSemantic,
/// Vec<Vec<Option<PaintValue>>>>`) stores paint as per-polygon per-vertex
/// `Some(_)` flags — there is no separate annotation-polygon representation
/// to intersect against. The "painted area" is therefore implicitly the
/// region polygon itself; the area floor is used to suppress degenerate
/// / empty regions from being classified as enforcer or blocker, not to
/// measure overlap with an external annotation polygon. This is the same
/// boolean-presence check the original `paint_policy_for` used under
/// the hood; what changed is that the pre-packet `paint_policy_for` also
/// performed a centroid probe (which was the bug the packet fixes), and
/// the new helper delegates the region-ownership check (which cell of
/// the SliceIR does the caller's expoly belong to) to the caller.
///
/// Precedence (per `docs/01_system_architecture.md` §"Support Stage Paint
/// Precedence"):
///
/// 1. `segment_annotations[SupportBlocker]` has a `Some(_)` entry AND
///    region area ≥ `NON_TRIVIAL_AREA_UNITS_SQ` → `SupportPaintPolicy::Blocked`.
/// 2. Else `segment_annotations[SupportEnforcer]` has a `Some(_)` entry AND
///    region area ≥ `NON_TRIVIAL_AREA_UNITS_SQ` → `SupportPaintPolicy::Enforced`.
/// 3. Otherwise → `SupportPaintPolicy::DefaultEligible`.
///
/// # Arguments
///
/// * `region_polygons` — the slice region's polygons (the same data that
///   `SliceRegionView::polygons()` would return).
/// * `segment_annotations` — the region's paint map (the same data that
///   `SliceRegionView::segment_annotations()` would return).
pub fn support_eligibility(
    region_polygons: &[ExPolygon],
    segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
) -> SupportPaintPolicy {
    if annotation_covers_region(
        region_polygons,
        segment_annotations,
        PaintSemantic::SupportBlocker,
    ) {
        return SupportPaintPolicy::Blocked;
    }
    if annotation_covers_region(
        region_polygons,
        segment_annotations,
        PaintSemantic::SupportEnforcer,
    ) {
        return SupportPaintPolicy::Enforced;
    }
    SupportPaintPolicy::DefaultEligible
}

/// True if `region`'s `segment_annotations[semantic]` map entry contains at
/// least one `Some(_)` paint value AND the region polygon has non-trivial
/// area.
///
/// The "non-trivial area" check is a region-area floor, not a measure of
/// overlap between region and a separate annotation polygon. The IR's
/// `segment_annotations` map does not carry a per-semantic annotation
/// polygon — paint is stored as per-polygon per-vertex `Some(_)` flags —
/// so the painted area is implicitly the region polygon itself (see the
/// module-level docs).
fn annotation_covers_region(
    region_polygons: &[ExPolygon],
    segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    semantic: PaintSemantic,
) -> bool {
    let entries = segment_annotations.get(&semantic);
    if !entries_has_some_paint(entries) {
        return false;
    }
    // Region-ownership has already been verified by the caller; here we
    // just check that the region is non-degenerate. The area threshold is
    // a region-area floor — it gates enforcer/blocker classification to
    // regions that actually have shape, suppressing empty or near-empty
    // regions from being classified. Compute area via the same shoelace
    // helper the rest of the workspace uses.
    region_area_units_sq(region_polygons) >= NON_TRIVIAL_AREA_UNITS_SQ
}

/// True if `entries` (the per-polygon per-vertex paint values for a given
/// semantic) contains at least one `Some(_)` paint value.
fn entries_has_some_paint(entries: Option<&Vec<Vec<Option<PaintValue>>>>) -> bool {
    match entries {
        None => false,
        Some(per_polygon) => per_polygon
            .iter()
            .any(|per_vertex| per_vertex.iter().any(Option::is_some)),
    }
}

/// Total signed area (absolute value) of the contours of `polys` in
/// workspace scaled units². Holes subtract. Uses the same shoelace formula
/// as `crate::polygon_tree::contour_area_abs` /
/// `crate::top_surface_split::contour_area_abs` so the threshold is
/// consistent with the rest of the workspace.
fn region_area_units_sq(polys: &[ExPolygon]) -> i128 {
    let mut total: i128 = 0;
    for ep in polys {
        total += shoelace_area(&ep.contour.points);
        for hole in &ep.holes {
            total -= shoelace_area(&hole.points);
        }
    }
    total.unsigned_abs() as i128
}

/// Unsigned shoelace area of a closed polygon ring in workspace scaled
/// units².
fn shoelace_area(pts: &[slicer_ir::Point2]) -> i128 {
    if pts.len() < 3 {
        return 0;
    }
    let mut twice: i128 = 0;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        twice += (a.x as i128) * (b.y as i128) - (b.x as i128) * (a.y as i128);
    }
    (twice.unsigned_abs() as i128) / 2
}
