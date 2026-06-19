// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PerimeterGenerator.cpp
//   (split_top_surfaces, approximately line 775)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Sub-top single-wall surface carve.
//!
//! OrcaSlicer's `split_top_surfaces` re-derives the top surface area from
//! `upper_slices` + bridge exclusion + offsets at generation time. This port
//! reuses the pre-classified `top_solid_fill` area produced by
//! `PrePass::ShellClassification` instead of that inline derivation — the
//! information is equivalent but already available, avoiding redundant
//! Clipper2 boolean work.
//!
//! # Algorithm
//! For a region at `top_shell_index = Some(N)` with `N > 0` (a shell layer
//! below the exposed surface):
//!
//! ```text
//! top_portion     = intersection(region_polygons, top_solid_fill)
//! non_top_portion = difference(region_polygons, top_solid_fill)
//! ```
//!
//! Both portions then pass through the normal inset → `build_wall_flags` →
//! emit pipeline. The `top_portion` uses `wall_count = 1`; the
//! `non_top_portion` uses the full configured `wall_count`.
//!
//! # Sliver filtering
//! Near-zero-area fragments produced by Clipper2 at collinear or near-collinear
//! boundaries are removed with a minimum-area threshold of **300 000 units²**
//! (= 3 × 10⁻³ mm² at 1 unit = 100 nm). This is small enough to keep all
//! geometrically meaningful portions of a 1 mm² region (which has area
//! 100 000 000 units²) while reliably discarding degenerate edge slivers that
//! Clipper2 emits for grazing intersections. A direct area threshold is used
//! rather than an `offset2_ex` shrink+grow because the boolean outputs are
//! already clean convex/concave rings — the extra offset pass would distort
//! axis-aligned fixtures used by the golden tests.
use slicer_ir::ExPolygon;

use crate::polygon_ops::{difference, intersection};

/// Minimum area (units²) below which a polygon fragment is considered a sliver
/// and discarded. 300 000 units² ≈ 3 × 10⁻³ mm².
const MIN_AREA_UNITS_SQ: f64 = 300_000.0;

/// Result of [`split_top_surfaces`].
#[derive(Debug, Clone)]
pub struct TopSurfaceSplit {
    /// Intersection of region polygons with top_solid_fill.
    ///
    /// This sub-region should be emitted with `wall_count = 1`.
    pub top_portion: Vec<ExPolygon>,
    /// Difference of region polygons minus top_solid_fill.
    ///
    /// This sub-region should be emitted with the full configured `wall_count`.
    pub non_top_portion: Vec<ExPolygon>,
}

/// Partition `region_polygons` into a top portion and a non-top portion using
/// `top_solid_fill` as the carving mask.
///
/// If `top_solid_fill` is empty the function returns immediately with
/// `top_portion` empty and `non_top_portion` cloned from `region_polygons`,
/// avoiding unnecessary Clipper2 calls.
///
/// Both output vectors are filtered to drop fragments whose contour area
/// is below [`MIN_AREA_UNITS_SQ`].
pub fn split_top_surfaces(
    region_polygons: &[ExPolygon],
    top_solid_fill: &[ExPolygon],
) -> TopSurfaceSplit {
    if top_solid_fill.is_empty() {
        return TopSurfaceSplit {
            top_portion: Vec::new(),
            non_top_portion: region_polygons.to_vec(),
        };
    }

    let raw_top = intersection(region_polygons, top_solid_fill);
    let raw_non_top = difference(region_polygons, top_solid_fill);

    TopSurfaceSplit {
        top_portion: filter_slivers(raw_top),
        non_top_portion: filter_slivers(raw_non_top),
    }
}

/// Remove fragments whose (unsigned) contour area is below [`MIN_AREA_UNITS_SQ`].
///
/// Uses the shoelace formula on the contour vertices. Holes are not considered
/// because Clipper2's flat-path output typically produces no hole rings —
/// the contour area alone is sufficient to detect degenerate edge slivers.
fn filter_slivers(polys: Vec<ExPolygon>) -> Vec<ExPolygon> {
    polys
        .into_iter()
        .filter(|p| contour_area_abs(&p.contour.points) >= MIN_AREA_UNITS_SQ)
        .collect()
}

/// Unsigned shoelace area of a closed polygon contour (scaled units²).
fn contour_area_abs(pts: &[slicer_ir::Point2]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut area2: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        area2 += (pts[i].x as i128) * (pts[j].y as i128);
        area2 -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    (area2.unsigned_abs() as f64) * 0.5
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use slicer_ir::{Point2, Polygon};

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn empty_top_fill_returns_full_region() {
        let region = vec![rect(0, 0, 100_000, 100_000)];
        let result = split_top_surfaces(&region, &[]);
        assert!(result.top_portion.is_empty());
        assert_eq!(result.non_top_portion.len(), 1);
    }

    #[test]
    fn contour_area_abs_square() {
        // 10mm square: side = 100_000 units → area = 10^10 units²
        let pts = vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 100_000, y: 0 },
            Point2 {
                x: 100_000,
                y: 100_000,
            },
            Point2 { x: 0, y: 100_000 },
        ];
        let area = contour_area_abs(&pts);
        let expected = 100_000_f64 * 100_000_f64;
        assert!((area - expected).abs() < 1.0, "area = {area}");
    }
}
