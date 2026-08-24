//! Per-layer overhang quartile-band annotation (Step 4, O-T021/O-T022).
//!
//! Deterministic pure function: given a per-layer sequence of already-computed
//! cross-section footprints, classifies the *overhanging* portion of each
//! layer's cross-section — the part of layer `n`'s footprint that is NOT
//! supported by layer `n - 1`'s footprint — into 4 concentric distance bands
//! measured from the previous layer's cross-section boundary. Overhang is
//! derived from the slices, never a second mesh-slicing pass, matching
//! OrcaSlicer's `detect_overhangs_for_lift` (`PrintObject.cpp:880-908`), which
//! diffs consecutive `lslices`. No host-services, scheduler, or runtime
//! dependency: this is pure geometry over [`ExPolygon`], reusing existing
//! [`crate::polygon_ops`] boolean/offset primitives (no new polygon boolean
//! code is implemented here). The caller (`PrePass::OverhangAnnotation`)
//! supplies each object's per-layer footprints from the committed `SliceIR`.
//!
//! # Band thresholds and emission-time speed sections
//!
//! The emission-time speed profile consumes the restored `speed_sections` table
//! `{90, 75, 50, 25, 13, 0}` in `overhang-classifier-default` and interpolates
//! those sections from each point's prepass-stamped `overhang_distance_mm`.
//! The four concentric `overhang_quartile` bands below, bounded by
//! `line_width x {0.5, 1.0, 1.5, 2.0}`, remain PnP's classification geometry,
//! evaluated at prepass time against raw cross-section geometry; they are not
//! the emission-time speed schedule.
//!
//! Band semantics (distance measured outward from the previous layer's
//! cross-section boundary, i.e. how far a point in the overhang region sits
//! from the last supported edge):
//!
//! | band | distance range              | meaning                       |
//! |------|------------------------------|-------------------------------|
//! | 1    | `(0, 0.5 × lw]`               | least overhanging (nearest support) |
//! | 2    | `(0.5 × lw, 1.0 × lw]`        | moderate                      |
//! | 3    | `(1.0 × lw, 1.5 × lw]`        | severe                        |
//! | 4    | `> 1.5 × lw`                  | most overhanging (capped by the region's own extent, not by the `2.0 × lw` multiplier — see [`BAND_BOUNDARY_MULTIPLIERS`]) |
//!
//! # Empty-layer semantics
//!
//! A layer with **no** overhang (including layer 0, which has no previous
//! layer and is therefore never overhanging) has its key **absent** from the
//! returned maps — callers must treat a missing key as "no overhang", not
//! distinguish it from an explicit empty `Vec`. This is the chosen semantics
//! for this packet (the alternative — an explicit empty `Vec<QuartileBand>`
//! entry — was rejected to keep the map's cardinality proportional to actual
//! overhang, matching `SurfaceClassificationIR.overhang_quartile_polygons`'s
//! doc-comment).
//!
//! # Config wiring note (for the Step 5 host stage)
//!
//! `line_width_mm` is taken as a plain parameter here — this module has no
//! config-key knowledge. The Step 5 host stage is expected to resolve it by
//! reading config key `outer_wall_line_width`, falling back to `line_width`
//! (both snake_case per repo convention) before calling
//! [`annotate_overhangs`].

use std::collections::HashMap;

use rayon::prelude::*;
use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::ExPolygon;

use crate::polygon_ops::{difference_ex, intersection_ex, offset, union_ex, OffsetJoinType};

/// Arc tolerance (mm) passed to the underlying `clipper2` offset calls.
/// Small relative to expected line-width-scale thresholds (0.2-0.8mm range);
/// matches the fine-tolerance convention used by other round-join offsets in
/// this crate (see `polygon_ops::opening`/`closing_ex`, which use `0.05`).
const OFFSET_ARC_TOLERANCE_MM: f32 = 0.01;

/// Multipliers (of `line_width_mm`) defining the 3 interior band boundaries
/// for the 4-band partition. Per roadmap decision O-4 the nominal threshold
/// tuple is `{0.5, 1.0, 1.5, 2.0}`; the `2.0` multiplier is intentionally
/// **not** used as an offset boundary here because band 4's outer edge is
/// defined as "the rest of the overhang region" (capped by the region's own
/// extent), not by a fixed distance cutoff — see the module doc-comment's
/// band-semantics table. `2.0` is retained here only in a comment for
/// traceability to the roadmap decision text, not as a runtime constant.
const BAND_BOUNDARY_MULTIPLIERS: [f32; 3] = [0.5, 1.0, 1.5];

/// Classifies overhanging cross-section area at every layer into 4 quartile
/// distance bands, keyed by layer index.
///
/// # Parameters
/// - `layer_footprints`: one entry per layer, ordered by increasing Z, each
///   `(layer_index, footprint)` pairing the global layer index (used as the
///   returned map's key) with that layer's cross-section polygons in
///   millimeters. Consecutive entries must be physically adjacent layers so
///   that `diff(current, previous)` is the true unsupported area. For a single
///   object these are its per-layer `SliceIR` polygons; the first entry has no
///   predecessor and is therefore never overhanging.
/// - `line_width_mm`: extrusion line width in millimeters used to derive the
///   band distance thresholds (`line_width_mm × {0.5, 1.0, 1.5}`). See the
///   module doc-comment's "Config wiring note" for how the host stage should
///   resolve this value from config.
///
/// # Returns
///
/// A tuple containing maps from layer index to the layer's `QuartileBand`
/// partition and to the previous layer's slice boundary contours. **Layers
/// with no overhang have their key absent** in both maps — see the module
/// doc-comment's "Empty-layer semantics" section.
pub fn annotate_overhangs(
    layer_footprints: &[(u32, Vec<ExPolygon>)],
    line_width_mm: f32,
) -> (
    HashMap<u32, Vec<QuartileBand>>,
    HashMap<u32, Vec<ExPolygon>>,
) {
    if layer_footprints.len() < 2 {
        return (HashMap::new(), HashMap::new());
    }

    // One O(layers) sweep over already-computed cross-sections — the object's
    // slice footprints, supplied by the caller. Overhang is derived from the
    // slices (not a second mesh pass), matching OrcaSlicer's
    // `detect_overhangs_for_lift`, which diffs consecutive `lslices`.
    // Consecutive entries must be adjacent layers in increasing-Z order; each
    // entry's `u32` is the layer index used to key the returned map.
    //
    // The sweep is parallel: iteration `i` reads only `layer_footprints[i - 1]`
    // and `[i]` and contributes exactly one `(layer_index, bands)` entry, so
    // there is no cross-iteration state. Because the result is a map keyed by
    // `layer_index` — and each index is produced by exactly one iteration —
    // the collected contents are identical regardless of completion order,
    // which is what keeps the stage's output byte-stable. `difference_ex`,
    // `intersection_ex` and `offset` are pure Clipper2 wrappers holding no
    // shared mutable state, so they are safe to call concurrently.
    let classified: HashMap<u32, (Vec<QuartileBand>, Vec<ExPolygon>)> = (1..layer_footprints.len())
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

            let bands = partition_into_bands(current, previous, &overhang_area, line_width_mm);
            if bands.is_empty() {
                return None;
            }

            Some((*layer_index, (bands, previous.clone())))
        })
        .collect();

    // Keep both outputs keyed by the same global layer index.
    let mut bands_map: HashMap<u32, Vec<QuartileBand>> = HashMap::new();
    let mut prev_map: HashMap<u32, Vec<ExPolygon>> = HashMap::new();
    for (layer_index, (bands, prev)) in classified {
        bands_map.insert(layer_index, bands);
        prev_map.insert(layer_index, prev);
    }
    (bands_map, prev_map)
}

/// Parameters for [`detect_support_contacts`], mirroring the config inputs
/// canonical `detect_overhangs` (`SupportMaterial.cpp`) reads per layer.
///
/// **All lengths are millimetres.** `slicer_core::polygon_ops::offset` scales
/// internally, so canonical's `scale_()` calls are deliberately not ported.
#[derive(Debug, Clone, Copy, PartialEq)]
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
/// layer count) is not modelled here; see [`detect_support_contacts`]'s
/// "Not modelled" section.
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
/// 1. `lower_layer_offset == 0` -> plain `diff(region, lower)`.
/// 2. otherwise -> `diff(region, expand(lower, offset))`, then, when non-empty,
///    the **expand-back**:
///    `diff(intersection(expand(diff, offset), region), lower)`. This grows the
///    contact back out to the full overhang so downstream support columns are
///    wide enough; without it contacts are systematically under-sized.
/// 3. subtract `blockers`.
/// 4. tiny-spot filter: drop the result entirely if it vanishes under a
///    `-0.1 * fw` erosion.
/// 5. XY expansion by `support_expansion`, when non-zero.
/// 6. `union_ex`.
///
/// Every offset uses [`SUPPORT_SURFACES_JOIN`] (canonical
/// `SUPPORT_SURFACES_OFFSET_PARAMETERS` = `jtSquare, 0.`).
///
/// # Not modelled (canonical features needing inputs the host stage lacks)
///
/// sharp-tail detection (`g_config_support_sharp_tails`),
/// `bridge_no_support` / `remove_bridges_from_contacts`, `buildplate_covered`,
/// the cantilever pass, and `enforce_support_layers`.
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

    let lower_layer_offset = lower_layer_offset_mm(params);

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

    // Step 3: support blockers.
    if !blockers.is_empty() {
        diff_polygons = difference_ex(&diff_polygons, blockers);
    }

    // Step 4: tiny-spot filter -- a contact that erodes away under a tenth of a
    // line width cannot be printed on, so canonical drops it wholesale.
    if diff_polygons.is_empty() {
        return Vec::new();
    }
    let erosion = -0.1 * params.external_perimeter_width_mm;
    if erosion != 0.0
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

    // Step 5: XY expansion (`support_expansion`, default 0).
    if params.xy_expansion_mm != 0.0 {
        diff_polygons = offset(
            &diff_polygons,
            params.xy_expansion_mm,
            SUPPORT_SURFACES_JOIN,
            OFFSET_ARC_TOLERANCE_MM,
        );
    }

    // Step 6.
    union_ex(&diff_polygons)
}

/// Partitions `overhang_area` (already `current \ previous`) into the 4
/// quartile bands, measuring distance outward from `previous`'s boundary.
///
/// Implementation strategy (reuses existing boolean/offset primitives —
/// no new polygon boolean code):
/// for each interior threshold `t` in [`BAND_BOUNDARY_MULTIPLIERS`], grow
/// `previous` outward by `t` (`offset`), intersect the grown polygon with
/// `current`, then subtract `previous` itself — this yields the cumulative
/// overhang region within distance `t` of the previous boundary. Successive
/// cumulative regions are subtracted from each other to isolate each band;
/// the final band (4) is whatever remains of `overhang_area` after removing
/// the cumulative region within the last interior threshold.
fn partition_into_bands(
    current: &[ExPolygon],
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
            let within_threshold = intersection_ex(current, &grown_previous);
            difference_ex(&within_threshold, previous)
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
        let (result, _) = annotate_overhangs(&footprints(&mesh, &layer_zs), 0.4);
        assert!(
            !result.contains_key(&0),
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
    /// region; the assertion guards the band-partition sweep, not slicing.
    #[test]
    fn annotate_overhangs_is_fast_for_many_stacked_layers() {
        const CUBE_COUNT: usize = 1200;
        let mesh = stacked_cubes_mesh(CUBE_COUNT);
        let layer_zs: Vec<f32> = (0..CUBE_COUNT).map(|i| i as f32 * 1.05 + 0.5).collect();
        let layers = footprints(&mesh, &layer_zs);

        let start = std::time::Instant::now();
        let (result, _) = annotate_overhangs(&layers, 0.4);
        let elapsed = start.elapsed();

        assert!(
            result.is_empty(),
            "identically-sized stacked cubes must never classify as overhanging"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "annotate_overhangs took {elapsed:?} for {CUBE_COUNT} pre-sliced \
             stacked-cube layers (expected well under 1s) — the per-transition \
             band partition should be cheap O(layers) polygon work"
        );
    }
}
