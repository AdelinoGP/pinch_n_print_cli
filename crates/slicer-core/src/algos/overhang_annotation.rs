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
//! # Band thresholds and deviation from OrcaSlicer
//!
//! OrcaSlicer's actual banded overhang classification
//! (`ExtrusionProcessor.hpp::estimate_extrusion_quality`, `GCode.cpp` overhang
//! speed bands) uses **6** bands at `extrusion_width × {0.1, 0.25, 0.5, 0.75,
//! 0.87, 1.0}`, derived from overlap percentages `{90, 75, 50, 25, 13, 0}`,
//! and is applied to wall extrusion geometry at gcode-emission time. This
//! packet intentionally deviates and uses **4** bands at
//! `line_width × {0.5, 1.0, 1.5, 2.0}` per roadmap decision O-4, evaluated at
//! pre-pass time against raw cross-section geometry rather than wall
//! extrusion paths. This is a recorded, intentional deviation — not a bug.
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
//! returned map — callers must treat a missing key as "no overhang", not
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

use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::ExPolygon;

use crate::polygon_ops::{difference_ex, intersection_ex, offset, OffsetJoinType};

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
/// A map from layer index to that layer's `QuartileBand` partition. **Layers
/// with no overhang have their key absent** — see the module doc-comment's
/// "Empty-layer semantics" section.
pub fn annotate_overhangs(
    layer_footprints: &[(u32, Vec<ExPolygon>)],
    line_width_mm: f32,
) -> HashMap<u32, Vec<QuartileBand>> {
    let mut result = HashMap::new();
    if layer_footprints.len() < 2 {
        return result;
    }

    // One O(layers) sweep over already-computed cross-sections — the object's
    // slice footprints, supplied by the caller. Overhang is derived from the
    // slices (not a second mesh pass), matching OrcaSlicer's
    // `detect_overhangs_for_lift` (`PrintObject.cpp:880-908`), which diffs
    // consecutive `lslices`. Consecutive entries must be adjacent layers in
    // increasing-Z order; each entry's `u32` is the layer index used to key
    // the returned map.
    for i in 1..layer_footprints.len() {
        let (_, previous) = &layer_footprints[i - 1];
        let (layer_index, current) = &layer_footprints[i];

        if current.is_empty() {
            continue;
        }

        let overhang_area = difference_ex(current, previous);
        if overhang_area.is_empty() {
            continue;
        }

        let bands = partition_into_bands(current, previous, &overhang_area, line_width_mm);
        if !bands.is_empty() {
            result.insert(*layer_index, bands);
        }
    }

    result
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
        let result = annotate_overhangs(&footprints(&mesh, &layer_zs), 0.4);
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
        let result = annotate_overhangs(&layers, 0.4);
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
