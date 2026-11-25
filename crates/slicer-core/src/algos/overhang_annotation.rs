//! Per-layer overhang annotation (Step 4, O-T021/O-T022).
//!
//! Deterministic pure function: given a per-layer sequence of already-computed
//! cross-section footprints, derives the *overhanging* portion of each layer's
//! cross-section — the part of layer `n`'s footprint that is NOT supported by
//! layer `n - 1`'s footprint — and the previous layer's footprint the consumer
//! needs to locate it. Overhang is derived from the slices, never a second
//! mesh-slicing pass, matching OrcaSlicer's `detect_overhangs_for_lift`
//! (`PrintObject.cpp:880-908`), which diffs consecutive `lslices`. No
//! host-services, scheduler, or runtime dependency: this is pure geometry over
//! [`ExPolygon`], reusing existing [`crate::polygon_ops`] boolean primitives
//! (no new polygon boolean code is implemented here). The caller
//! (`PrePass::OverhangAnnotation`) supplies each object's per-layer footprints
//! from the committed `SliceIR`.
//!
//! # Why no band polygons are produced here (ticket 142)
//!
//! The 4-band concentric partition used to be built here as polygon sets
//! (`intersection(current, offset(previous, lw × {0.5, 1.0, 1.5})) - previous`,
//! three round offsets plus six whole-layer booleans per overhang layer). The
//! band **value** has no production reader: `overhang-classifier-default`
//! gates on `overhang_quartile.is_some()` and computes its speed from
//! `overhang_distance_mm`, and the perimeter annotation already computes
//! `signed_distance_to_boundary(x, y, prev_layer_boundary)` against the very
//! same previous-layer polygon set. A vertex of `current` is in some band iff
//! it is outside `previous`, which is exactly the sign of that signed
//! distance, so the quartile is now **derived from the distance** at the
//! consumer ([`crate::perimeter_utils::overhang_quartile_from_distance`]) and
//! no band polygons are built on the slice path.
//!
//! Band semantics (distance measured outward from the previous layer's
//! cross-section boundary, i.e. how far a point in the overhang region sits
//! from the last supported edge):
//!
//! | quartile | distance range              | meaning                       |
//! |----------|-----------------------------|-------------------------------|
//! | 1        | `(0, 0.5 × lw]`             | least overhanging (nearest support) |
//! | 2        | `(0.5 × lw, 1.0 × lw]`      | moderate                      |
//! | 3        | `(1.0 × lw, 1.5 × lw]`      | severe                        |
//! | 4        | `> 1.5 × lw`                | most overhanging (capped by the region's own extent) |
//!
//! # Empty-layer semantics
//!
//! A layer with **no** overhang (including layer 0, which has no previous
//! layer and is therefore never overhanging) has its key **absent** from the
//! returned maps — callers must treat a missing key as "no overhang", not
//! distinguish it from an explicit empty `Vec`. Keeping the key absent is what
//! preserves `overhang_distance_mm = None` on non-overhanging layers, which is
//! the pre-existing IR contract.
//!
//! # Config wiring note
//!
//! The band width behind the quartile thresholds is a plain `f32` on
//! `SurfaceClassificationIR` (`overhang_band_width_mm`), resolved by the host
//! stage from config key `outer_wall_line_width`, falling back to `line_width`
//! (both snake_case per repo convention) and finally to `0.4` mm. It is
//! transported to the layer modules per region so the producer's width — not
//! the consumer's per-point extrusion width — is the threshold source.

use std::collections::HashMap;

use rayon::prelude::*;
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{ExPolygon, Point2};

use crate::polygon_ops::{
    difference_ex, intersection_ex, offset, union_ex, OffsetJoinType, OFFSET_SHORTEST_EDGE_FACTOR,
};

/// Arc tolerance (mm) passed to the underlying `clipper2` offset calls.
/// Small relative to expected line-width-scale thresholds (0.2-0.8mm range);
/// matches the fine-tolerance convention used by other round-join offsets in
/// this crate (see `polygon_ops::opening`/`closing_ex`, which use `0.05`).
const OFFSET_ARC_TOLERANCE_MM: f32 = 0.01;

/// Multipliers (of the band width) defining the 3 interior band boundaries of
/// the 4-quartile distance classification. Re-exported from `perimeter_utils`
/// because the consumer side is compiled without `host-algos`.
pub use crate::perimeter_utils::OVERHANG_BAND_BOUNDARY_MULTIPLIERS as BAND_BOUNDARY_MULTIPLIERS;

/// Per-object overhang annotation derived from committed slice footprints.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverhangAnnotation {
    /// Unsupported area `current \ previous` per layer index. Absent on
    /// layers with no overhang.
    pub overhang_areas: HashMap<u32, Vec<ExPolygon>>,
    /// The previous layer's footprint per layer index, present exactly on the
    /// layers in [`Self::overhang_areas`]. The consumer measures signed
    /// distance against it to locate a vertex relative to the overhang.
    pub prev_layer_boundaries: HashMap<u32, Vec<ExPolygon>>,
}

/// Derives the overhanging cross-section area at every layer.
///
/// # Parameters
/// - `layer_footprints`: one entry per layer, ordered by increasing Z, each
///   `(layer_index, footprint)` pairing the global layer index (used as the
///   returned map's key) with that layer's cross-section polygons in
///   millimetres. Consecutive entries must be physically adjacent layers so
///   that `diff(current, previous)` is the true unsupported area. For a single
///   object these are its per-layer `SliceIR` polygons; the first entry has no
///   predecessor and is therefore never overhanging.
///
/// # Returns
///
/// [`OverhangAnnotation`] with both maps keyed by global layer index. **Layers
/// with no overhang have their key absent** in both maps — see the module
/// doc-comment's "Empty-layer semantics" section.
pub fn annotate_overhangs(layer_footprints: &[(u32, Vec<ExPolygon>)]) -> OverhangAnnotation {
    if layer_footprints.len() < 2 {
        return OverhangAnnotation::default();
    }

    // One O(layers) sweep over already-computed cross-sections — the object's
    // slice footprints, supplied by the caller. Overhang is derived from the
    // slices (not a second mesh pass), matching OrcaSlicer's
    // `detect_overhangs_for_lift`, which diffs consecutive `lslices`.
    // Consecutive entries must be adjacent layers in increasing-Z order; each
    // entry's `u32` is the layer index used to key the returned map.
    //
    // The sweep is parallel: iteration `i` reads only `layer_footprints[i - 1]`
    // and `[i]` and contributes exactly one entry, so there is no
    // cross-iteration state. Because the result is a map keyed by
    // `layer_index` — and each index is produced by exactly one iteration —
    // the collected contents are identical regardless of completion order,
    // which is what keeps the stage's output byte-stable. `difference_ex` is a
    // pure Clipper2 wrapper holding no shared mutable state, so it is safe to
    // call concurrently.
    let classified: Vec<(u32, Vec<ExPolygon>, Vec<ExPolygon>)> = (1..layer_footprints.len())
        .into_par_iter()
        .filter_map(|i| {
            let (_, previous) = &layer_footprints[i - 1];
            let (layer_index, current) = &layer_footprints[i];

            if current.is_empty() {
                return None;
            }

            let overhang_area = difference_ex(current, previous);
            if overhang_area.is_empty() {
                return None;
            }

            Some((*layer_index, overhang_area, previous.clone()))
        })
        .collect();

    // Keep both outputs keyed by the same global layer index.
    let mut overhang_areas: HashMap<u32, Vec<ExPolygon>> = HashMap::new();
    let mut prev_layer_boundaries: HashMap<u32, Vec<ExPolygon>> = HashMap::new();
    for (layer_index, area, prev) in classified {
        overhang_areas.insert(layer_index, area);
        prev_layer_boundaries.insert(layer_index, prev);
    }
    OverhangAnnotation {
        overhang_areas,
        prev_layer_boundaries,
    }
}

/// Parameters for [`detect_support_contacts`], mirroring the config inputs
/// canonical `detect_overhangs` (`SupportMaterial.cpp`) reads per layer.
///
/// **All lengths are millimetres.** `slicer_core::polygon_ops::offset` scales
/// internally, so canonical's `scale_()` calls are deliberately not ported.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportContactParams {
    /// `support_threshold_angle` in degrees, as configured (un-bumped,
    /// un-clamped -- this function applies canonical's `+1` inclusivity bump
    /// and 89-degree clamp itself). `0` selects canonical's *overlap* branch,
    /// **not** "support everything".
    pub threshold_angle_deg: f32,
    /// Printed height of the **lower** layer (canonical scales the offset by
    /// `lower_layer.height`).
    pub lower_layer_height_mm: f32,
    /// External-perimeter extrusion width (`fw` in canonical). Drives both the
    /// zero-angle overlap offset and the tiny-spot filter.
    pub external_perimeter_width_mm: f32,
    /// `support_threshold_overlap` already resolved against `fw`
    /// (`ConfigOptionFloatOrPercent(50., true)` by default, i.e. `fw / 2`).
    /// Only consulted on the zero-angle branch.
    pub threshold_overlap_mm: f32,
    /// `support_expansion` (`coFloat`, default `0`): XY growth applied to the
    /// finished contact region.
    pub xy_expansion_mm: f32,
    /// Canonical `bridge_no_support`: remove host-supplied bridge areas.
    pub bridge_no_support: bool,
    /// Bridge polygons identified by the host for this layer.
    pub bridge_polygons: Vec<ExPolygon>,
    /// Transitional opt-in for canonical sharp-tail contacts.
    pub support_sharp_tails: bool,
    /// Force plain differences for the first support layers.
    pub enforce_support_layers: u32,
    /// Zero-based current **global** layer index.
    pub layer_id: u32,
    /// Global index of the object's bottom layer, i.e. `support_raft_layers`.
    ///
    /// Under the positive-offset raft band, raft layers occupy global indices
    /// `0..raft_layers-1` and the object's own first layer is
    /// `raft_layers`. Object-bottom-geometry predicates must compare
    /// `layer_id` against this value, never against `0` (canonical's
    /// `this->layer_id == object_config->raft_layers`). `0` when no raft is
    /// configured, which reproduces the pre-raft behaviour exactly.
    pub raft_layers: u32,
}

/// Contact geometry plus the post-union cantilever annotation pass.
#[derive(Debug, Clone, PartialEq)]
pub struct SupportContactAnnotations {
    /// Unioned support-contact polygons.
    pub contacts: Vec<ExPolygon>,
    /// Complete contact unions whose span exceeds the cantilever threshold.
    pub cantilever_surfaces: Vec<ExPolygon>,
}

impl Default for SupportContactParams {
    /// Canonical defaults for a 0.4mm external perimeter: threshold angle 30
    /// (`PrintConfig.cpp` `support_threshold_angle`), overlap 50% of `fw`,
    /// no XY expansion. `lower_layer_height_mm` has no canonical default and
    /// is 0 here -- callers that care must set it.
    fn default() -> Self {
        Self {
            threshold_angle_deg: 30.0,
            lower_layer_height_mm: 0.0,
            external_perimeter_width_mm: 0.4,
            threshold_overlap_mm: 0.2,
            xy_expansion_mm: 0.0,
            bridge_no_support: false,
            bridge_polygons: Vec::new(),
            support_sharp_tails: false,
            enforce_support_layers: 0,
            layer_id: 0,
            raft_layers: 0,
        }
    }
}

/// Canonical `SUPPORT_SURFACES_OFFSET_PARAMETERS` is
/// `ClipperLib::jtSquare, 0.` -- every offset in the support-contact pipeline
/// uses a square join, never a miter or round one.
const SUPPORT_SURFACES_JOIN: OffsetJoinType = OffsetJoinType::Square;

/// The angle-derived `lower_layer_offset` (mm) canonical grows the lower layer
/// by before differencing.
///
/// Canonical, in order:
///
/// ```text
/// thresh_angle = support_threshold_angle > 0 ? support_threshold_angle + 1 : 0;
/// thresh_angle = min(thresh_angle, 89.);
/// threshold_rad = deg2rad(thresh_angle);
/// lower_layer_offset = threshold_rad > 0 ? lower_layer.height / tan(threshold_rad)
///                                        : fw - support_threshold_overlap;
/// ```
///
/// The `+1` is an inclusivity bump (an overhang exactly at the configured angle
/// must still be caught); the 89-degree clamp keeps `tan` finite. A configured
/// angle of `0` takes the **overlap** branch -- it does not mean "support
/// everything", and it is not a plain difference.
///
/// `enforce_support_layers` (canonical forces the offset to `0` below that
fn lower_layer_offset_mm(params: &SupportContactParams) -> f32 {
    let thresh_angle = if params.threshold_angle_deg > 0.0 {
        (params.threshold_angle_deg + 1.0).min(89.0)
    } else {
        0.0
    };
    let threshold_rad = thresh_angle.to_radians();
    if threshold_rad > 0.0 {
        params.lower_layer_height_mm / threshold_rad.tan()
    } else {
        params.external_perimeter_width_mm - params.threshold_overlap_mm
    }
}

/// Angle-thresholded support-contact detection for **one region of one layer**,
/// the support-generation sibling of [`annotate_overhangs`].
///
/// The two functions diff the same consecutive slices but answer different
/// questions and mirror **different** canonical functions, so neither may be
/// substituted for the other:
///
/// - [`annotate_overhangs`] mirrors `detect_overhangs_for_lift`
///   (`PrintObject.cpp`) and partitions *all* unsupported area into fixed
///   `line_width`-multiple distance bands, for lift and speed classification.
/// - This function mirrors `detect_overhangs` (`SupportMaterial.cpp`) and
///   returns only the area steep enough to *require support*.
///
/// # Layer-major, whole-lower-layer union (the caller's job)
///
/// Canonical reads `lower_layer_polygons` from
/// `object.layers()[layer_id - 1]->lslices` -- an **object-level** slice set,
/// the union of *all* regions of the layer below -- then loops the current
/// layer's regions, each contributing its own `layerm->slices.surfaces`.
/// This entry point is therefore per-`(layer, region)`: the caller computes the
/// lower-layer union **once per layer** and passes it in for every region.
///
/// Keying the lower layer per-region instead (the pre-parity in-tree shape)
/// makes a region that first appears at layer `k` while sitting squarely on a
/// *different* region below emit its entire cross-section as a support contact
/// -- spurious full-area support on every multi-region / multi-material object.
///
/// # Pipeline (canonical order)
///
/// 1. Sharp-tail handling may seed first-layer pointed profiles.
/// 2. `lower_layer_offset == 0` -> plain `diff(region, lower)`.
/// 3. otherwise -> `diff(region, expand(lower, offset))`, then, when non-empty,
///    the **expand-back**:
///    `diff(intersection(expand(diff, offset), region), lower)`. This grows the
///    contact back out to the full overhang so downstream support columns are
///    wide enough; without it contacts are systematically under-sized.
/// 4. subtract `blockers`, then bridge areas when `bridge_no_support` is set.
/// 5. tiny-spot filter: drop the result entirely if it vanishes under a
///    `-0.1 * fw` erosion.
/// 6. XY expansion by `support_expansion`, when non-zero.
/// 7. `union_ex`.
///
/// Every offset uses [`SUPPORT_SURFACES_JOIN`] (canonical
/// `SUPPORT_SURFACES_OFFSET_PARAMETERS` = `jtSquare, 0.`).
///
/// # Not modelled (canonical features needing inputs the host stage lacks)
///
/// `buildplate_covered`.
///
/// # Returns
///
/// The contact polygons for this region, or an empty `Vec` when the region is
/// self-supporting (callers keying a map should omit the key entirely, matching
/// this module's "Empty-layer semantics").
#[must_use]
pub fn detect_support_contacts(
    region_polygons: &[ExPolygon],
    lower_layer_polygons: &[ExPolygon],
    blockers: &[ExPolygon],
    params: &SupportContactParams,
) -> Vec<ExPolygon> {
    if region_polygons.is_empty() {
        return Vec::new();
    }

    // The enforce window is object-relative: canonical counts
    // `enforce_support_layers` from the object's own first layer, which under
    // the positive-offset raft band is global index `raft_layers`, not `0`.
    // Shifted window: `raft_layers .. raft_layers + enforce_support_layers`.
    let force_support = params.layer_id >= params.raft_layers
        && params.layer_id
            < params
                .raft_layers
                .saturating_add(params.enforce_support_layers);
    let lower_layer_offset = if force_support {
        0.0
    } else {
        lower_layer_offset_mm(params)
    };

    // Steps 1-2: the diff, and (on the offset branch) the expand-back.
    let mut diff_polygons = if lower_layer_offset == 0.0 {
        difference_ex(region_polygons, lower_layer_polygons)
    } else {
        let grown = offset(
            lower_layer_polygons,
            lower_layer_offset,
            SUPPORT_SURFACES_JOIN,
            OFFSET_ARC_TOLERANCE_MM,
        );
        let trimmed = difference_ex(region_polygons, &grown);
        if trimmed.is_empty() {
            trimmed
        } else {
            let expanded_back = offset(
                &trimmed,
                lower_layer_offset,
                SUPPORT_SURFACES_JOIN,
                OFFSET_ARC_TOLERANCE_MM,
            );
            difference_ex(
                &intersection_ex(&expanded_back, region_polygons),
                lower_layer_polygons,
            )
        }
    };

    // Sharp tails are an object-bottom exception: preserve pointed profiles
    // even when the ordinary lower-layer offset consumes their tiny footprint.
    // The object's bottom layer is global index `raft_layers` (0 without a
    // raft); under a raft this must not fire on the raft itself.
    let sharp_tail_enabled = params.support_sharp_tails && params.layer_id == params.raft_layers;
    if sharp_tail_enabled {
        let has_sharp_tail = region_polygons
            .iter()
            .any(|polygon| polygon.contour.points.len() == 3);
        if has_sharp_tail {
            diff_polygons = region_polygons.to_vec();
        }
    }

    // Step 3: support blockers.
    if !blockers.is_empty() {
        diff_polygons = difference_ex(&diff_polygons, blockers);
    }

    // Step 4: bridge removal. The margin mirrors canonical's safety offset
    // around host-supplied bridge regions.
    if params.bridge_no_support && !params.bridge_polygons.is_empty() {
        const SUPPORT_MATERIAL_MARGIN_MM: f32 = 1.2;
        let bridge_margin = offset(
            &params.bridge_polygons,
            SUPPORT_MATERIAL_MARGIN_MM,
            SUPPORT_SURFACES_JOIN,
            OFFSET_ARC_TOLERANCE_MM,
        );
        diff_polygons = difference_ex(&diff_polygons, &bridge_margin);
    }

    // Step 5: tiny-spot filter -- a contact that erodes away under a tenth of a
    // line width cannot be printed on, so canonical drops it wholesale.
    if diff_polygons.is_empty() {
        return Vec::new();
    }
    let erosion = -0.1 * params.external_perimeter_width_mm;
    if !sharp_tail_enabled
        && erosion != 0.0
        && offset(
            &diff_polygons,
            erosion,
            SUPPORT_SURFACES_JOIN,
            OFFSET_ARC_TOLERANCE_MM,
        )
        .is_empty()
    {
        return Vec::new();
    }

    // Step 6: XY expansion (`support_expansion`, default 0).
    if params.xy_expansion_mm != 0.0 {
        diff_polygons = offset(
            &diff_polygons,
            params.xy_expansion_mm,
            SUPPORT_SURFACES_JOIN,
            OFFSET_ARC_TOLERANCE_MM,
        );
    }

    // Step 7.
    union_ex(&diff_polygons)
}

/// Detect contacts and retain the complete union when any part spans more than
/// the canonical 3 mm cantilever threshold from the lower layer.
#[must_use]
pub fn detect_support_contacts_with_annotations(
    region_polygons: &[ExPolygon],
    lower_layer_polygons: &[ExPolygon],
    blockers: &[ExPolygon],
    params: &SupportContactParams,
) -> SupportContactAnnotations {
    let contacts = detect_support_contacts(region_polygons, lower_layer_polygons, blockers, params);
    if contacts.is_empty() {
        return SupportContactAnnotations {
            contacts,
            cantilever_surfaces: Vec::new(),
        };
    }

    // Keep the threshold in the canonical scaled-unit domain before passing it
    // to the millimetre-based polygon helpers.
    let threshold_mm = slicer_ir::units_to_mm(slicer_ir::mm_to_units(3.0));
    if certify_covered_by_square_offset(&contacts, lower_layer_polygons, threshold_mm) {
        return SupportContactAnnotations {
            contacts,
            cantilever_surfaces: Vec::new(),
        };
    }

    let supported_extent = offset(
        lower_layer_polygons,
        threshold_mm,
        SUPPORT_SURFACES_JOIN,
        OFFSET_ARC_TOLERANCE_MM,
    );
    let beyond_threshold = difference_ex(&contacts, &supported_extent);
    let cantilever_surfaces = if beyond_threshold.is_empty() {
        Vec::new()
    } else {
        contacts.clone()
    };

    SupportContactAnnotations {
        contacts,
        cantilever_surfaces,
    }
}

/// Conservative sufficient geometric certificate proving that `contacts` is
/// completely covered by `offset(lower_layer_polygons, threshold_mm, Square, 0.01)`,
/// so `difference_ex(contacts, &supported_extent)` is guaranteed empty.
///
/// Soundness invariants:
/// - Applies the exact [`OFFSET_SHORTEST_EDGE_FACTOR`] (`0.005 * delta_units`)
///   closed-ring decimation and zero-area / winding rules used by
///   `polygon_ops::offset`, dropping degenerate zero-area contours (strictly
///   conservative: a smaller lower set can only shrink its offset).
/// - Uses a conservative `1,000`-unit (`0.10 mm`) interior safety margin
///   `r_in = delta_units - 1_000.0` against the Euclidean disk `B_2(r_in)`,
///   which sits strictly inside `JoinType::Square`'s `delta_units` dilation
///   everywhere (including convex bevels and shrunk holes), absorbing ±1-unit
///   integer rounding with three orders of magnitude to spare.
/// - Returns `true` ONLY when every point of every contact contour is proven
///   either within `r_in` of a kept decimated boundary segment of `lower` or
///   strictly inside a kept decimated solid region of `lower`. Any uncertain,
///   near-threshold, malformed, or huge-domain case returns `false` so the
///   caller falls back to the exact `ClipperOffset` + `difference_ex` path.
#[must_use]
pub fn certify_covered_by_square_offset(
    contacts: &[ExPolygon],
    lower_layer_polygons: &[ExPolygon],
    threshold_mm: f32,
) -> bool {
    if contacts.is_empty() {
        return true;
    }
    if lower_layer_polygons.is_empty() || !threshold_mm.is_finite() || threshold_mm <= 0.0 {
        return false;
    }

    let delta_units = f64::from(threshold_mm) * slicer_ir::UNITS_PER_MM;
    const SAFETY_MARGIN_UNITS: f64 = 1_000.0; // 0.10 mm
    const MAX_ABS_COORD: f64 = 1.0e12;
    let r_in = delta_units - SAFETY_MARGIN_UNITS;
    if r_in <= 0.0 {
        return false;
    }
    let r_in_sq = r_in * r_in;

    // Compute contacts union AABB and validate finiteness/non-degeneracy.
    let mut c_min_x = f64::INFINITY;
    let mut c_min_y = f64::INFINITY;
    let mut c_max_x = f64::NEG_INFINITY;
    let mut c_max_y = f64::NEG_INFINITY;
    for ep in contacts {
        if ep.contour.points.len() < 3 {
            return false;
        }
        for p in &ep.contour.points {
            let x = p.x as f64;
            let y = p.y as f64;
            if x.abs() > MAX_ABS_COORD || y.abs() > MAX_ABS_COORD {
                return false;
            }
            c_min_x = c_min_x.min(x);
            c_min_y = c_min_y.min(y);
            c_max_x = c_max_x.max(x);
            c_max_y = c_max_y.max(y);
        }
    }
    if !c_min_x.is_finite() {
        return false;
    }

    // Step 1: Decimate `lower_layer_polygons` with the exact same rule as
    // `polygon_ops::offset_paths`, keeping positive-area contours and valid
    // non-degenerate holes.
    let shortest = delta_units.abs() * OFFSET_SHORTEST_EDGE_FACTOR;
    let win_min_x = c_min_x - r_in;
    let win_min_y = c_min_y - r_in;
    let win_max_x = c_max_x + r_in;
    let win_max_y = c_max_y + r_in;

    let mut decimated_lower: Vec<DecimatedExPoly> = Vec::with_capacity(lower_layer_polygons.len());
    let mut grid_segs: Vec<Seg2> = Vec::new();

    for exp in lower_layer_polygons {
        let Some(mut contour) = decimate_ring_for_offset(&exp.contour.points, shortest) else {
            continue;
        };
        let contour_area = ring_double_area(&contour);
        if contour_area == 0 {
            continue;
        }
        if contour_area < 0 {
            contour.reverse();
        }
        let Some(c_bbox) = ring_bbox(&contour, MAX_ABS_COORD) else {
            return false;
        };

        let mut kept_holes: Vec<(Vec<Point2>, [f64; 4])> = Vec::with_capacity(exp.holes.len());
        for hole in &exp.holes {
            let Some(mut dec_hole) = decimate_ring_for_offset(&hole.points, shortest) else {
                continue;
            };
            let h_area = ring_double_area(&dec_hole);
            if h_area == 0 {
                continue;
            }
            if h_area > 0 {
                dec_hole.reverse();
            }
            let Some(h_bbox) = ring_bbox(&dec_hole, MAX_ABS_COORD) else {
                return false;
            };
            // Defensive soundness check: a valid ExPolygon hole sits inside its
            // parent contour. If a caller passes a malformed hole outside its
            // contour, fall back to Clipper2 rather than trusting hole edges.
            if h_bbox[0] < c_bbox[0]
                || h_bbox[1] < c_bbox[1]
                || h_bbox[2] > c_bbox[2]
                || h_bbox[3] > c_bbox[3]
                || !point_in_ring(dec_hole[0].x as f64, dec_hole[0].y as f64, &contour)
            {
                return false;
            }
            collect_windowed_segments(
                &dec_hole,
                win_min_x,
                win_min_y,
                win_max_x,
                win_max_y,
                &mut grid_segs,
            );
            kept_holes.push((dec_hole, h_bbox));
        }

        collect_windowed_segments(
            &contour,
            win_min_x,
            win_min_y,
            win_max_x,
            win_max_y,
            &mut grid_segs,
        );
        decimated_lower.push(DecimatedExPoly {
            contour,
            contour_bbox: c_bbox,
            holes: kept_holes,
        });
    }

    if decimated_lower.is_empty() {
        return false;
    }

    let grid = LowerSegGrid::build(grid_segs, win_min_x, win_min_y, win_max_x, win_max_y, r_in);

    // Step 2: Certify every contact contour.
    for contact in contacts {
        let ring = &contact.contour.points;
        let Some([b_min_x, b_min_y, b_max_x, b_max_y]) = ring_bbox(ring, MAX_ABS_COORD) else {
            return false;
        };
        let span_x = b_max_x - b_min_x;
        let span_y = b_max_y - b_min_y;
        let mx = 0.5 * (b_min_x + b_max_x);
        let my = 0.5 * (b_min_y + b_max_y);
        let half_diag = 0.5 * span_x.hypot(span_y);

        // Fast path 1: single-disk or interior box covers entire contact AABB.
        if box_covered(mx, my, half_diag, r_in, &grid, &decimated_lower) {
            continue;
        }

        // Fast path 2: vertex screen — fail fast on any uncovered vertex
        // (immediately rejects true cantilevers and near-threshold edges).
        for p in ring {
            let px = p.x as f64;
            let py = p.y as f64;
            if !grid.has_seg_within(px, py, r_in_sq)
                && !point_in_decimated_lower(px, py, &decimated_lower)
            {
                return false;
            }
        }

        // Degenerate zero-span contact after passing vertex screen.
        if span_x <= 0.0 || span_y <= 0.0 {
            return false;
        }

        // Step 3: 2-level coarse-to-fine (+ narrow level-3) box cover.
        let coarse_target = r_in * 0.70;
        let nx = ((span_x / coarse_target).ceil() as usize).max(1);
        let ny = ((span_y / coarse_target).ceil() as usize).max(1);
        if nx.saturating_mul(ny) > 32_768 {
            return false;
        }
        let dx = span_x / (nx as f64);
        let dy = span_y / (ny as f64);
        let coarse_h = 0.5 * dx.hypot(dy);

        const TILE_OUTSIDE: u8 = 0;
        const TILE_INTERIOR: u8 = 1;
        const TILE_BOUNDARY: u8 = 2;
        let mut tiles = vec![TILE_OUTSIDE; nx * ny];

        // Mark every coarse tile whose AABB overlaps a padded contact edge.
        let inv_dx = 1.0 / dx;
        let inv_dy = 1.0 / dy;
        let n_pts = ring.len();
        for i in 0..n_pts {
            let a = ring[i];
            let b = ring[(i + 1) % n_pts];
            let ex0 = (a.x.min(b.x) as f64 - 1.0 - b_min_x) * inv_dx;
            let ex1 = (a.x.max(b.x) as f64 + 1.0 - b_min_x) * inv_dx;
            let ey0 = (a.y.min(b.y) as f64 - 1.0 - b_min_y) * inv_dy;
            let ey1 = (a.y.max(b.y) as f64 + 1.0 - b_min_y) * inv_dy;
            let tx0 = (ex0.floor() as isize).clamp(0, (nx - 1) as isize) as usize;
            let tx1 = (ex1.floor() as isize).clamp(0, (nx - 1) as isize) as usize;
            let ty0 = (ey0.floor() as isize).clamp(0, (ny - 1) as isize) as usize;
            let ty1 = (ey1.floor() as isize).clamp(0, (ny - 1) as isize) as usize;
            for ty in ty0..=ty1 {
                let row_base = ty * nx;
                for tx in tx0..=tx1 {
                    tiles[row_base + tx] = TILE_BOUNDARY;
                }
            }
        }

        // Classify non-boundary tiles per row via one horizontal scanline pass.
        let mut row_xs: Vec<f64> = Vec::new();
        for ty in 0..ny {
            let row_base = ty * nx;
            let y_mid = b_min_y + (ty as f64 + 0.5) * dy;
            row_xs.clear();
            let mut j = n_pts - 1;
            for i in 0..n_pts {
                let yi = ring[i].y as f64;
                let yj = ring[j].y as f64;
                if (yi > y_mid) != (yj > y_mid) {
                    let xi = ring[i].x as f64;
                    let xj = ring[j].x as f64;
                    row_xs.push(xi + (y_mid - yi) * (xj - xi) / (yj - yi));
                }
                j = i;
            }
            if row_xs.is_empty() {
                continue;
            }
            row_xs.sort_unstable_by(f64::total_cmp);
            let mut cross_idx = 0usize;
            for tx in 0..nx {
                if tiles[row_base + tx] != TILE_OUTSIDE {
                    continue;
                }
                let x_mid = b_min_x + (tx as f64 + 0.5) * dx;
                while cross_idx < row_xs.len() && row_xs[cross_idx] <= x_mid {
                    cross_idx += 1;
                }
                if (row_xs.len() - cross_idx) % 2 == 1 {
                    tiles[row_base + tx] = TILE_INTERIOR;
                }
            }
        }

        // Verify all active coarse tiles, refining uncovered tiles 4x4 (and a
        // final 4x4 pass on any surviving intersecting sub-tile).
        for ty in 0..ny {
            let y0 = b_min_y + (ty as f64) * dy;
            let row_base = ty * nx;
            for tx in 0..nx {
                let kind = tiles[row_base + tx];
                if kind == TILE_OUTSIDE {
                    continue;
                }
                let x0 = b_min_x + (tx as f64) * dx;
                if box_covered(
                    x0 + 0.5 * dx,
                    y0 + 0.5 * dy,
                    coarse_h,
                    r_in,
                    &grid,
                    &decimated_lower,
                ) {
                    continue;
                }
                let is_boundary = kind == TILE_BOUNDARY;
                if !certify_subdivided_tile(
                    x0,
                    y0,
                    dx,
                    dy,
                    is_boundary,
                    2,
                    r_in,
                    ring,
                    &grid,
                    &decimated_lower,
                ) {
                    return false;
                }
            }
        }
    }

    true
}

struct DecimatedExPoly {
    contour: Vec<Point2>,
    contour_bbox: [f64; 4],
    holes: Vec<(Vec<Point2>, [f64; 4])>,
}

#[derive(Clone, Copy)]
struct Seg2 {
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

struct LowerSegGrid {
    segs: Vec<Seg2>,
    min_x: f64,
    min_y: f64,
    cell_size: f64,
    inv_cell: f64,
    nx: usize,
    ny: usize,
    offsets: Vec<u32>,
    indices: Vec<u32>,
}

impl LowerSegGrid {
    fn build(segs: Vec<Seg2>, min_x: f64, min_y: f64, max_x: f64, max_y: f64, r_in: f64) -> Self {
        if segs.is_empty() {
            return Self {
                segs,
                min_x,
                min_y,
                cell_size: 1.0,
                inv_cell: 1.0,
                nx: 0,
                ny: 0,
                offsets: Vec::new(),
                indices: Vec::new(),
            };
        }
        let span_x = (max_x - min_x).max(1.0);
        let span_y = (max_y - min_y).max(1.0);
        let mut cell_size = (r_in * 0.5).max(5_000.0);
        cell_size = cell_size.max(span_x / 256.0).max(span_y / 256.0);
        let inv_cell = 1.0 / cell_size;
        let nx = ((span_x * inv_cell).ceil() as usize).clamp(1, 256);
        let ny = ((span_y * inv_cell).ceil() as usize).clamp(1, 256);
        let total_cells = nx * ny;

        let mut counts = vec![0_u32; total_cells];
        for s in &segs {
            let cx0 = (((s.min_x - min_x) * inv_cell).floor() as isize).clamp(0, (nx - 1) as isize)
                as usize;
            let cx1 = (((s.max_x - min_x) * inv_cell).floor() as isize).clamp(0, (nx - 1) as isize)
                as usize;
            let cy0 = (((s.min_y - min_y) * inv_cell).floor() as isize).clamp(0, (ny - 1) as isize)
                as usize;
            let cy1 = (((s.max_y - min_y) * inv_cell).floor() as isize).clamp(0, (ny - 1) as isize)
                as usize;
            for cy in cy0..=cy1 {
                let row = cy * nx;
                for cx in cx0..=cx1 {
                    counts[row + cx] += 1;
                }
            }
        }

        let mut offsets = vec![0_u32; total_cells + 1];
        for i in 0..total_cells {
            offsets[i + 1] = offsets[i] + counts[i];
        }
        let mut indices = vec![0_u32; offsets[total_cells] as usize];
        let mut cursor = offsets[..total_cells].to_vec();
        for (seg_idx, s) in segs.iter().enumerate() {
            let cx0 = (((s.min_x - min_x) * inv_cell).floor() as isize).clamp(0, (nx - 1) as isize)
                as usize;
            let cx1 = (((s.max_x - min_x) * inv_cell).floor() as isize).clamp(0, (nx - 1) as isize)
                as usize;
            let cy0 = (((s.min_y - min_y) * inv_cell).floor() as isize).clamp(0, (ny - 1) as isize)
                as usize;
            let cy1 = (((s.max_y - min_y) * inv_cell).floor() as isize).clamp(0, (ny - 1) as isize)
                as usize;
            for cy in cy0..=cy1 {
                let row = cy * nx;
                for cx in cx0..=cx1 {
                    let pos = cursor[row + cx] as usize;
                    indices[pos] = seg_idx as u32;
                    cursor[row + cx] += 1;
                }
            }
        }

        Self {
            segs,
            min_x,
            min_y,
            cell_size,
            inv_cell,
            nx,
            ny,
            offsets,
            indices,
        }
    }

    fn has_seg_within(&self, px: f64, py: f64, limit_sq: f64) -> bool {
        if self.segs.is_empty() || limit_sq < 0.0 {
            return false;
        }
        let limit = limit_sq.sqrt();
        let cx_c =
            (((px - self.min_x) * self.inv_cell).floor() as isize).clamp(0, (self.nx - 1) as isize);
        let cy_c =
            (((py - self.min_y) * self.inv_cell).floor() as isize).clamp(0, (self.ny - 1) as isize);
        let max_ring = (limit * self.inv_cell).ceil() as isize + 1;

        let check_cell = |cx: isize, cy: isize| -> bool {
            if cx < 0 || cy < 0 || cx >= self.nx as isize || cy >= self.ny as isize {
                return false;
            }
            let cell = (cy as usize) * self.nx + (cx as usize);
            let start = self.offsets[cell] as usize;
            let end = self.offsets[cell + 1] as usize;
            for &idx in &self.indices[start..end] {
                let s = &self.segs[idx as usize];
                let dx_box = if px < s.min_x {
                    s.min_x - px
                } else if px > s.max_x {
                    px - s.max_x
                } else {
                    0.0
                };
                let dy_box = if py < s.min_y {
                    s.min_y - py
                } else if py > s.max_y {
                    py - s.max_y
                } else {
                    0.0
                };
                if dx_box * dx_box + dy_box * dy_box > limit_sq {
                    continue;
                }
                if point_to_seg_dist_sq(px, py, s.ax, s.ay, s.bx, s.by) <= limit_sq {
                    return true;
                }
            }
            false
        };

        if check_cell(cx_c, cy_c) {
            return true;
        }
        for ring in 1..=max_ring {
            if ring > 1 {
                let min_ring_dist = (ring - 1) as f64 * self.cell_size;
                if min_ring_dist * min_ring_dist > limit_sq {
                    break;
                }
            }
            for dx in -ring..=ring {
                if check_cell(cx_c + dx, cy_c - ring) || check_cell(cx_c + dx, cy_c + ring) {
                    return true;
                }
            }
            for dy in (-ring + 1)..=(ring - 1) {
                if check_cell(cx_c - ring, cy_c + dy) || check_cell(cx_c + ring, cy_c + dy) {
                    return true;
                }
            }
        }
        false
    }
}

fn box_covered(
    mx: f64,
    my: f64,
    half_diag: f64,
    r_in: f64,
    grid: &LowerSegGrid,
    decimated_lower: &[DecimatedExPoly],
) -> bool {
    if half_diag < r_in {
        let margin = r_in - half_diag;
        if grid.has_seg_within(mx, my, margin * margin) {
            return true;
        }
    }
    if half_diag <= r_in
        && !grid.has_seg_within(mx, my, half_diag * half_diag)
        && point_in_decimated_lower(mx, my, decimated_lower)
    {
        return true;
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn certify_subdivided_tile(
    x0: f64,
    y0: f64,
    dx: f64,
    dy: f64,
    is_boundary: bool,
    levels_remaining: u8,
    r_in: f64,
    ring: &[Point2],
    grid: &LowerSegGrid,
    decimated_lower: &[DecimatedExPoly],
) -> bool {
    let sub_dx = dx * 0.25;
    let sub_dy = dy * 0.25;
    let sub_h = 0.5 * sub_dx.hypot(sub_dy);
    for fy in 0..4 {
        let sy0 = y0 + (fy as f64) * sub_dy;
        let sy1 = sy0 + sub_dy;
        for fx in 0..4 {
            let sx0 = x0 + (fx as f64) * sub_dx;
            let sx1 = sx0 + sub_dx;
            if box_covered(
                sx0 + 0.5 * sub_dx,
                sy0 + 0.5 * sub_dy,
                sub_h,
                r_in,
                grid,
                decimated_lower,
            ) {
                continue;
            }
            if is_boundary && !tile_intersects_ring(sx0, sy0, sx1, sy1, ring) {
                continue;
            }
            if levels_remaining > 1 {
                if !certify_subdivided_tile(
                    sx0,
                    sy0,
                    sub_dx,
                    sub_dy,
                    is_boundary,
                    levels_remaining - 1,
                    r_in,
                    ring,
                    grid,
                    decimated_lower,
                ) {
                    return false;
                }
            } else {
                return false;
            }
        }
    }
    true
}

fn decimate_ring_for_offset(points: &[Point2], shortest: f64) -> Option<Vec<Point2>> {
    let first = *points.first()?;
    let mut high = points.len() - 1;
    let shortest2 = shortest * shortest;
    let same = |a: Point2, b: Point2| -> bool {
        let dx = (a.x - b.x) as f64;
        let dy = (a.y - b.y) as f64;
        if shortest2 > 0.0 {
            dx * dx + dy * dy < shortest2
        } else {
            dx == 0.0 && dy == 0.0
        }
    };
    while high > 0 && same(points[high], first) {
        high -= 1;
    }
    let mut dst = Vec::with_capacity(high + 1);
    dst.push(first);
    let mut last = 0;
    for i in 1..=high {
        if !same(points[i], points[last]) {
            dst.push(points[i]);
            last = i;
        }
    }
    (dst.len() >= 3).then_some(dst)
}

fn ring_double_area(points: &[Point2]) -> i128 {
    if points.len() < 3 {
        return 0;
    }
    let mut acc: i128 = 0;
    for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
        acc += i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y);
    }
    acc
}

fn ring_bbox(points: &[Point2], max_abs: f64) -> Option<[f64; 4]> {
    if points.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in points {
        let x = p.x as f64;
        let y = p.y as f64;
        if x.abs() > max_abs || y.abs() > max_abs {
            return None;
        }
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    Some([min_x, min_y, max_x, max_y])
}

fn collect_windowed_segments(
    ring: &[Point2],
    win_min_x: f64,
    win_min_y: f64,
    win_max_x: f64,
    win_max_y: f64,
    out: &mut Vec<Seg2>,
) {
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let ax = a.x as f64;
        let ay = a.y as f64;
        let bx = b.x as f64;
        let by = b.y as f64;
        let min_x = ax.min(bx);
        let max_x = ax.max(bx);
        let min_y = ay.min(by);
        let max_y = ay.max(by);
        if max_x < win_min_x || min_x > win_max_x || max_y < win_min_y || min_y > win_max_y {
            continue;
        }
        out.push(Seg2 {
            ax,
            ay,
            bx,
            by,
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }
}

#[inline]
fn point_to_seg_dist_sq(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let len_sq = abx * abx + aby * aby;
    if len_sq == 0.0 {
        return apx * apx + apy * apy;
    }
    let t = ((apx * abx + apy * aby) / len_sq).clamp(0.0, 1.0);
    let dx = apx - t * abx;
    let dy = apy - t * aby;
    dx * dx + dy * dy
}

fn point_in_ring(px: f64, py: f64, ring: &[Point2]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let yi = ring[i].y as f64;
        let yj = ring[j].y as f64;
        if (yi > py) != (yj > py) {
            let xi = ring[i].x as f64;
            let xj = ring[j].x as f64;
            let x_int = xi + (py - yi) * (xj - xi) / (yj - yi);
            if px < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn point_in_decimated_lower(px: f64, py: f64, lower: &[DecimatedExPoly]) -> bool {
    for ep in lower {
        if px < ep.contour_bbox[0]
            || px > ep.contour_bbox[2]
            || py < ep.contour_bbox[1]
            || py > ep.contour_bbox[3]
        {
            continue;
        }
        if !point_in_ring(px, py, &ep.contour) {
            continue;
        }
        let mut in_hole = false;
        for (hole, hb) in &ep.holes {
            if px >= hb[0]
                && px <= hb[2]
                && py >= hb[1]
                && py <= hb[3]
                && point_in_ring(px, py, hole)
            {
                in_hole = true;
                break;
            }
        }
        if !in_hole {
            return true;
        }
    }
    false
}

fn tile_intersects_ring(x0: f64, y0: f64, x1: f64, y1: f64, ring: &[Point2]) -> bool {
    let px0 = x0 - 1.0;
    let py0 = y0 - 1.0;
    let px1 = x1 + 1.0;
    let py1 = y1 + 1.0;
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        if seg_intersects_aabb(
            a.x as f64, a.y as f64, b.x as f64, b.y as f64, px0, py0, px1, py1,
        ) {
            return true;
        }
    }
    point_in_ring(0.5 * (x0 + x1), 0.5 * (y0 + y1), ring)
}

#[allow(clippy::too_many_arguments)]
fn seg_intersects_aabb(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> bool {
    if ax.max(bx) < min_x || ax.min(bx) > max_x || ay.max(by) < min_y || ay.min(by) > max_y {
        return false;
    }
    let dx = bx - ax;
    let dy = by - ay;
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    for (p, q) in [
        (-dx, ax - min_x),
        (dx, max_x - ax),
        (-dy, ay - min_y),
        (dy, max_y - ay),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return false;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return false;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }
    t0 <= t1
}

/// Reconstructs the 4 quartile band polygons from `overhang_area` (already
/// `current \ previous`) by measuring distance outward from `previous`'s
/// boundary. **Diagnostic only** — the slice path does not call this: the
/// production quartile is derived from the per-vertex signed distance
/// ([`crate::perimeter_utils::overhang_quartile_from_distance`]). The
/// visual-debug renderer calls it at render time to draw band silhouettes,
/// which is why the band polygons are no longer carried in the IR.
///
/// Implementation strategy (reuses existing boolean/offset primitives —
/// no new polygon boolean code): for each interior threshold `t` in
/// [`BAND_BOUNDARY_MULTIPLIERS`], grow `previous` outward by `t` (`offset`)
/// and intersect the grown polygon with `overhang_area` — this yields the
/// cumulative overhang region within distance `t` of the previous boundary
/// (`(current ∩ grown) \ previous` and `(current \ previous) ∩ grown` are the
/// same set). Successive cumulative regions are subtracted from each other to
/// isolate each band; the final band (4) is whatever remains of
/// `overhang_area` after removing the cumulative region within the last
/// interior threshold.
#[must_use]
pub fn partition_into_bands(
    previous: &[ExPolygon],
    overhang_area: &[ExPolygon],
    line_width_mm: f32,
) -> Vec<QuartileBand> {
    // Cumulative overhang region within each interior threshold distance of
    // `previous`'s boundary.
    let cumulative: Vec<Vec<ExPolygon>> = BAND_BOUNDARY_MULTIPLIERS
        .iter()
        .map(|multiplier| {
            let threshold_mm = line_width_mm * multiplier;
            let grown_previous = offset(
                previous,
                threshold_mm,
                OffsetJoinType::Round,
                OFFSET_ARC_TOLERANCE_MM,
            );
            intersection_ex(overhang_area, &grown_previous)
        })
        .collect();

    let mut bands = Vec::with_capacity(4);

    // Band 1: cumulative region within the first (smallest) threshold.
    push_band(&mut bands, 1, cumulative[0].clone());

    // Bands 2-3: successive differences between cumulative regions.
    push_band(&mut bands, 2, difference_ex(&cumulative[1], &cumulative[0]));
    push_band(&mut bands, 3, difference_ex(&cumulative[2], &cumulative[1]));

    // Band 4: everything left over in the overhang region beyond the last
    // interior threshold — capped by the overhang region's own extent, not
    // by a fixed distance cutoff (see module doc-comment).
    push_band(&mut bands, 4, difference_ex(overhang_area, &cumulative[2]));

    bands
}

/// Pushes a [`QuartileBand`] for `quartile` iff `polygons` is non-empty.
/// Keeps empty bands out of the returned `Vec` (mirrors the map-level
/// "absent key means no overhang" convention at band granularity).
fn push_band(bands: &mut Vec<QuartileBand>, quartile: u8, polygons: Vec<ExPolygon>) {
    if !polygons.is_empty() {
        bands.push(QuartileBand { quartile, polygons });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice_mesh_ex;
    use slicer_ir::{IndexedTriangleSet, Point3};

    /// Slice `mesh` at each Z in `layer_zs` and pair each resulting footprint
    /// with its position index, producing the `annotate_overhangs` input.
    /// Overhang classification now consumes pre-computed cross-sections, so
    /// tests that start from a mesh slice it once here (as the real
    /// `PrePass::OverhangAnnotation` producer does from `SliceIR`).
    fn footprints(mesh: &IndexedTriangleSet, layer_zs: &[f32]) -> Vec<(u32, Vec<ExPolygon>)> {
        slice_mesh_ex(mesh, layer_zs)
            .into_iter()
            .enumerate()
            .map(|(i, poly)| (i as u32, poly))
            .collect()
    }

    /// 10x10x10mm cube fixture, matching the winding convention used by
    /// `mesh_cross_section`'s own tests (bottom CW-from-above via
    /// `0,1,2 / 0,2,3`, top CCW-from-above via `4,5,6 / 4,6,7`).
    fn flat_cube_mesh() -> IndexedTriangleSet {
        let vertices = vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 10.0,
                y: 10.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            Point3 {
                x: 10.0,
                y: 0.0,
                z: 10.0,
            },
            Point3 {
                x: 10.0,
                y: 10.0,
                z: 10.0,
            },
            Point3 {
                x: 0.0,
                y: 10.0,
                z: 10.0,
            },
        ];
        #[rustfmt::skip]
        let indices = vec![
            0, 1, 2,  0, 2, 3,
            4, 5, 6,  4, 6, 7,
            0, 1, 5,  0, 5, 4,
            1, 2, 6,  1, 6, 5,
            2, 3, 7,  2, 7, 6,
            3, 0, 4,  3, 4, 7,
        ];
        IndexedTriangleSet { vertices, indices }
    }

    #[test]
    fn straight_cube_layer0_has_no_previous_and_is_absent() {
        let mesh = flat_cube_mesh();
        let layer_zs = vec![0.5, 1.5];
        let result = annotate_overhangs(&footprints(&mesh, &layer_zs));
        assert!(
            !result.overhang_areas.contains_key(&0),
            "layer 0 has no previous layer and must never be classified as overhanging"
        );
    }

    /// `cube_count` unit (1mm) cubes stacked vertically with a 0.05mm gap
    /// between them, cube `i` spanning Z in `[i*1.05, i*1.05 + 1.0]`. Each
    /// cube is disjoint in Z from every other cube, so a mesh with N cubes
    /// has N narrow, non-overlapping Z-bands — this is the shape that
    /// stresses per-call vs. batched cross-sectioning cost: at any single Z,
    /// only one cube (12 of the mesh's `12*cube_count` triangles) is
    /// actually relevant.
    fn stacked_cubes_mesh(cube_count: usize) -> IndexedTriangleSet {
        const CUBE_SIZE_MM: f32 = 1.0;
        const GAP_MM: f32 = 0.05;
        let pitch = CUBE_SIZE_MM + GAP_MM;

        let mut vertices = Vec::with_capacity(cube_count * 8);
        let mut indices = Vec::with_capacity(cube_count * 36);

        for i in 0..cube_count {
            let z0 = i as f32 * pitch;
            let z1 = z0 + CUBE_SIZE_MM;
            let base = vertices.len() as u32;
            vertices.push(Point3 {
                x: 0.0,
                y: 0.0,
                z: z0,
            });
            vertices.push(Point3 {
                x: CUBE_SIZE_MM,
                y: 0.0,
                z: z0,
            });
            vertices.push(Point3 {
                x: CUBE_SIZE_MM,
                y: CUBE_SIZE_MM,
                z: z0,
            });
            vertices.push(Point3 {
                x: 0.0,
                y: CUBE_SIZE_MM,
                z: z0,
            });
            vertices.push(Point3 {
                x: 0.0,
                y: 0.0,
                z: z1,
            });
            vertices.push(Point3 {
                x: CUBE_SIZE_MM,
                y: 0.0,
                z: z1,
            });
            vertices.push(Point3 {
                x: CUBE_SIZE_MM,
                y: CUBE_SIZE_MM,
                z: z1,
            });
            vertices.push(Point3 {
                x: 0.0,
                y: CUBE_SIZE_MM,
                z: z1,
            });

            #[rustfmt::skip]
            let local: [u32; 36] = [
                0, 1, 2,  0, 2, 3,
                4, 5, 6,  4, 6, 7,
                0, 1, 5,  0, 5, 4,
                1, 2, 6,  1, 6, 5,
                2, 3, 7,  2, 7, 6,
                3, 0, 4,  3, 4, 7,
            ];
            indices.extend(local.iter().map(|&idx| base + idx));
        }

        IndexedTriangleSet { vertices, indices }
    }

    /// `annotate_overhangs` must be O(layers) in the number of pre-sliced
    /// layers: it now consumes already-computed cross-sections and only runs
    /// polygon boolean/offset work per layer transition (no mesh slicing at
    /// all — that moved to `PrePass::Slice`, whose committed `SliceIR` the
    /// `PrePass::OverhangAnnotation` producer reads instead of re-slicing).
    /// Slicing here is test setup, done once and excluded from the timed
    /// region; the assertion guards the per-transition difference sweep, not
    /// slicing.
    // KEEP-review (core-brittle): O(layers) sweep guard; timed region excludes one-time slicing setup; 1s threshold on 1200 pre-sliced layers.
    #[test]
    fn annotate_overhangs_is_fast_for_many_stacked_layers() {
        const CUBE_COUNT: usize = 1200;
        let mesh = stacked_cubes_mesh(CUBE_COUNT);
        let layer_zs: Vec<f32> = (0..CUBE_COUNT).map(|i| i as f32 * 1.05 + 0.5).collect();
        let layers = footprints(&mesh, &layer_zs);

        let start = std::time::Instant::now();
        let result = annotate_overhangs(&layers);
        let elapsed = start.elapsed();

        assert!(
            result.overhang_areas.is_empty(),
            "identically-sized stacked cubes must never classify as overhanging"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "annotate_overhangs took {elapsed:?} for {CUBE_COUNT} pre-sliced \
             stacked-cube layers (expected well under 1s) — the per-transition \
             difference sweep should be cheap O(layers) polygon work"
        );
    }
}
