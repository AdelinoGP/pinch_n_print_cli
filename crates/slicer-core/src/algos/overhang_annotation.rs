//! Per-layer overhang quartile-band annotation (Step 4, O-T021/O-T022).
//!
//! Deterministic pure function: given a mesh and a set of layer Z heights
//! (mm), classifies the *overhanging* portion of each layer's cross-section
//! — the part of layer `n`'s footprint that is NOT supported by layer
//! `n - 1`'s footprint — into 4 concentric distance bands measured from the
//! previous layer's cross-section boundary. No host-services, scheduler, or
//! runtime dependency: this is pure geometry over [`IndexedTriangleSet`] +
//! [`ExPolygon`], reusing existing [`crate::polygon_ops`] boolean/offset
//! primitives (no new polygon boolean code is implemented here).
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
use slicer_ir::{ExPolygon, IndexedTriangleSet};

use crate::algos::mesh_cross_section::cross_section_at_z;
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

/// Classifies overhanging cross-section area at every layer in `layer_zs`
/// into 4 quartile distance bands, keyed by layer index.
///
/// # Parameters
/// - `mesh`: single-object mesh in millimeters (see
///   [`cross_section_at_z`]'s unit-convention doc-comment). Callers slicing a
///   `MeshIR` object should pass `object_mesh.mesh` with any transform
///   pre-applied.
/// - `layer_zs`: per-layer Z heights in millimeters, ordered by increasing
///   layer index (`layer_zs[i]` is layer `i`'s Z height). Layer 0 has no
///   previous layer and therefore is never overhanging.
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
    mesh: &IndexedTriangleSet,
    layer_zs: &[f32],
    line_width_mm: f32,
) -> HashMap<u32, Vec<QuartileBand>> {
    let mut result = HashMap::new();

    for i in 1..layer_zs.len() {
        let prev_z = layer_zs[i - 1];
        let curr_z = layer_zs[i];

        let previous = cross_section_at_z(mesh, prev_z);
        let current = cross_section_at_z(mesh, curr_z);

        if current.is_empty() {
            continue;
        }

        let overhang_area = difference_ex(&current, &previous);
        if overhang_area.is_empty() {
            continue;
        }

        let bands = partition_into_bands(&current, &previous, &overhang_area, line_width_mm);
        if !bands.is_empty() {
            result.insert(i as u32, bands);
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
    use slicer_ir::Point3;

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
        let result = annotate_overhangs(&mesh, &layer_zs, 0.4);
        assert!(
            !result.contains_key(&0),
            "layer 0 has no previous layer and must never be classified as overhanging"
        );
    }
}
