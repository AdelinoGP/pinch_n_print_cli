// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source paths:
//   src/libslic3r/MultiPoint.cpp   (`MultiPoint::_douglas_peucker`)
//   src/libslic3r/ExPolygon.cpp    (`ExPolygon::simplify_p`)
//   src/libslic3r/ClipperUtils.cpp (`simplify_polygons`)
//   src/libslic3r/Line.hpp         (`line_alg::distance_to_squared`)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Slicing-time contour simplification — OrcaSlicer's `resolution` key.
//!
//! Canonical consumes `resolution` *before* walls and infill are generated:
//! [`ExPolygon::simplify_p`](https://) runs Douglas–Peucker over the closed
//! contour and then over each hole independently (`ExPolygon.cpp`), and
//! `simplify_polygons` (`ClipperUtils.cpp`) re-unions the resulting rings with
//! a NonZero fill so the simplification's self-intersections are repaired.
//! `PerimeterGenerator` calls it at the top of the per-surface wall pass with
//! `m_scaled_resolution`, and at the infill-boundary passes with the same
//! tolerance.
//!
//! # Units
//!
//! Canonical scales millimetres by [`SCALING_FACTOR`]: 1 canonical unit =
//! 1e-6 mm. This crate's [`Point2`] is 1 unit = 100 nm = 1e-4 mm, i.e.
//! [`UNITS_PER_MM`] = 10 000 units/mm — a hundredth of canonical's scale. A
//! tolerance is therefore **converted**, never copied.
//!
//! # What this is not
//!
//! The union half runs on [`clipper2_rust`] through
//! [`crate::host::clip_polygons`], whereas canonical `simplify_polygons` runs
//! on the legacy Clipper 6.4.2 with `StrictlySimple(true)`. Both use NonZero,
//! so the *set* matches; the *representation* need not (legacy Clipper splits
//! self-touching output into strictly-simple polygons, and this engine has no
//! strictly-simple pass). The invariant suite pins the properties that must
//! hold; byte-identical output is not claimable and is not claimed.

use crate::host::{clip_polygons, ClipOperation};
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

/// The key contract lives with the config row in `slicer-ir`, so the resolver's
/// clamp and this helper's conversion cannot drift apart.
pub use slicer_ir::{DEFAULT_RESOLUTION_MM, RESOLUTION_FLOOR_MM};

/// Canonical scale factor (millimetres per canonical unit) — documented so the
/// unit conversion in [`tolerance_units`] is auditable against the C++ source.
pub const CANONICAL_SCALING_FACTOR_MM: f64 = 1e-6;

/// Convert a `resolution` setting in millimetres into a `Point2` tolerance.
///
/// Applies canonical's [`RESOLUTION_FLOOR_MM`] clamp exactly as
/// `DynamicPrintConfig::normalize_fdm` does (`std::max(value, 0.001)`), then
/// converts millimetres to this crate's 100 nm units. A non-finite setting
/// falls back to [`DEFAULT_RESOLUTION_MM`] rather than propagating NaN through
/// every distance comparison.
#[must_use]
pub fn tolerance_units(resolution_mm: f32) -> f64 {
    let mm = if resolution_mm.is_finite() {
        resolution_mm.max(RESOLUTION_FLOOR_MM)
    } else {
        DEFAULT_RESOLUTION_MM
    };
    f64::from(mm) * UNITS_PER_MM
}

/// Simplify every ring of `polygons` and re-union the result.
///
/// Mirrors canonical's call-site shape — `union_ex(expolygon.simplify_p(tol))`
/// accumulated over a surface set — for the pre-wall and infill-boundary
/// passes. Each contour and each hole is simplified independently, so a hole
/// boundary and its enclosing contour are not coupled.
///
/// Returns the unioned replacement set, which may contain fewer, more, or
/// differently-vertexed expolygons than the input (canonical's NonZero union
/// can merge ring fragments and drops degenerate ones).
#[must_use]
pub fn simplify_p(polygons: &[ExPolygon], tolerance_units: f64) -> Vec<ExPolygon> {
    if !(tolerance_units > 0.0) || polygons.is_empty() {
        return polygons.to_vec();
    }

    // Canonical flattens the contour and every hole into one `Polygons` list
    // and unions with NonZero; passing one ring per expolygon reproduces that
    // input exactly (hole winding is preserved from the IR).
    let mut rings: Vec<ExPolygon> = Vec::with_capacity(polygons.len());
    for polygon in polygons {
        rings.push(ExPolygon {
            contour: Polygon {
                points: simplify_ring(&polygon.contour.points, tolerance_units),
            },
            holes: Vec::new(),
        });
        for hole in &polygon.holes {
            rings.push(ExPolygon {
                contour: Polygon {
                    points: simplify_ring(&hole.points, tolerance_units),
                },
                holes: Vec::new(),
            });
        }
    }

    clip_polygons(&rings, &[], ClipOperation::Union)
}

/// Douglas–Peucker on an expolygon's rings **without** re-unioning them.
///
/// Canonical `ExPolygon::douglas_peucker` (`ExPolygon.cpp`) simplifies the
/// contour and each hole in place; `PerimeterGenerator` uses it on each gap
/// candidate before the medial-axis pass, where a union would be redundant.
#[must_use]
pub fn douglas_peucker(polygon: &ExPolygon, tolerance_units: f64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: simplify_ring(&polygon.contour.points, tolerance_units),
        },
        holes: polygon
            .holes
            .iter()
            .map(|hole| Polygon {
                points: simplify_ring(&hole.points, tolerance_units),
            })
            .collect(),
    }
}

/// Douglas–Peucker over a **closed** ring, matching `ExPolygon::simplify_p`.
///
/// Canonical closes the ring before the pass (`points.push_back(points.front())`)
/// and pops the duplicate afterwards, which makes the pass wrap-around
/// sensitive: the anchor is the ring's first vertex and the initial floater is
/// that same vertex, so the first split is the vertex furthest from it.
///
/// A non-positive tolerance is the identity (canonical cannot reach that state:
/// `normalize_fdm` clamps `resolution` to at least 1 µm).
#[must_use]
pub fn simplify_ring(points: &[Point2], tolerance_units: f64) -> Vec<Point2> {
    if points.is_empty() || !(tolerance_units > 0.0) {
        return points.to_vec();
    }

    let mut closed = Vec::with_capacity(points.len() + 1);
    closed.extend_from_slice(points);
    closed.push(points[0]);

    let mut simplified = douglas_peucker_points(&closed, tolerance_units);
    // Canonical `p.points.pop_back()` after the pass.
    simplified.pop();
    simplified
}

/// `MultiPoint::_douglas_peucker` (`MultiPoint.cpp`), iterative form.
///
/// The stack discipline, the `<=` acceptance test, and the "furthest index wins
/// strictly greater distances" tie-break are all canonical.
fn douglas_peucker_points(points: &[Point2], tolerance_units: f64) -> Vec<Point2> {
    let tolerance_sq = tolerance_units * tolerance_units;
    let mut result = Vec::with_capacity(points.len());
    if points.is_empty() {
        return result;
    }

    let mut anchor_idx = 0usize;
    let mut floater_idx = points.len() - 1;
    result.push(points[anchor_idx]);

    if anchor_idx != floater_idx {
        let mut stack: Vec<usize> = Vec::with_capacity(points.len());
        stack.push(floater_idx);

        loop {
            let mut max_dist_sq = 0.0_f64;
            let mut furthest_idx = anchor_idx;
            for idx in (anchor_idx + 1)..floater_idx {
                let dist_sq =
                    distance_to_squared(points[idx], points[anchor_idx], points[floater_idx]);
                if dist_sq > max_dist_sq {
                    max_dist_sq = dist_sq;
                    furthest_idx = idx;
                }
            }

            if max_dist_sq <= tolerance_sq {
                result.push(points[floater_idx]);
                anchor_idx = floater_idx;
                stack.pop();
                let Some(&next_floater) = stack.last() else {
                    break;
                };
                floater_idx = next_floater;
            } else {
                floater_idx = furthest_idx;
                stack.push(furthest_idx);
            }
        }
    }

    result
}

/// `line_alg::distance_to_squared` (`Line.hpp`) for an integer point against a
/// segment, in the canonical expression order.
fn distance_to_squared(point: Point2, a: Point2, b: Point2) -> f64 {
    let vx = (b.x - a.x) as f64;
    let vy = (b.y - a.y) as f64;
    let vax = (point.x - a.x) as f64;
    let vay = (point.y - a.y) as f64;

    let l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        // Degenerate segment: the initial floater equals the anchor on a closed
        // ring, so this is the canonical first-iteration path.
        return vax * vax + vay * vay;
    }

    let t = (vax * vx + vay * vy) / l2;
    if t <= 0.0 {
        return vax * vax + vay * vay;
    }
    if t >= 1.0 {
        let bx = (point.x - b.x) as f64;
        let by = (point.y - b.y) as f64;
        return bx * bx + by * by;
    }

    let proj_x = t * vx - vax;
    let proj_y = t * vy - vay;
    proj_x * proj_x + proj_y * proj_y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point2 {
        Point2 { x, y }
    }

    /// `resolution = 0.01` mm at 10 000 units/mm.
    const TOL_001MM: f64 = 100.0;

    fn ring_area2(points: &[Point2]) -> i128 {
        let mut acc: i128 = 0;
        for idx in 0..points.len() {
            let a = points[idx];
            let b = points[(idx + 1) % points.len()];
            acc += i128::from(a.x) * i128::from(b.y) - i128::from(b.x) * i128::from(a.y);
        }
        acc
    }

    #[test]
    fn tolerance_conversion_uses_this_crates_scale_and_canonical_floor() {
        // 0.01 mm -> 100 units here, vs 10 000 in canonical's 1e-6 mm units.
        // The residual is f32 storage of the config value, not a scale error:
        // 0.01f32 widened to f64 is 0.009999999776…, so 99.99999776 units.
        assert!((tolerance_units(0.01) - 100.0).abs() < 1e-4);
        assert!(
            (tolerance_units(0.01) * CANONICAL_SCALING_FACTOR_MM / (1.0 / UNITS_PER_MM) - 1.0)
                .abs()
                < 1e-6,
            "conversion must be a scale change, not a copy"
        );
        // "Disabled" is not expressible: the floor is canonical's 1 um.
        assert!((tolerance_units(0.0) - 10.0).abs() < 1e-6);
        assert!((tolerance_units(0.0004) - 10.0).abs() < 1e-6);
        assert!((tolerance_units(f32::NAN) - 100.0).abs() < 1e-4);
    }

    #[test]
    fn collinear_run_collapses_to_its_endpoints() {
        // A straight 10-unit step repeated: interior vertices are exactly on the
        // chord, so a 1-unit tolerance removes all of them.
        let ring = vec![p(0, 0), p(10, 0), p(20, 0), p(30, 0), p(30, 30), p(0, 30)];
        let out = simplify_ring(&ring, 1.0);
        assert_eq!(out.first(), Some(&p(0, 0)), "anchor must be preserved");
        assert!(
            !out.contains(&p(10, 0)) && !out.contains(&p(20, 0)),
            "collinear interior vertices must be removed, got {out:?}"
        );
        assert!(out.len() < ring.len());
    }

    #[test]
    fn a_corner_survives_a_tolerance_smaller_than_its_deviation() {
        let ring = vec![p(0, 0), p(100, 0), p(100, 100), p(0, 100)];
        let out = simplify_ring(&ring, 10.0);
        assert_eq!(
            out.len(),
            4,
            "square corners deviate by far more than 10 units"
        );
        assert_eq!(ring_area2(&out), ring_area2(&ring));
    }

    #[test]
    fn every_removed_vertex_lies_within_tolerance_of_its_output_segment() {
        // Canonical's guarantee: when a floater is emitted, every input vertex
        // strictly between the anchor and that floater was tested against the
        // final segment and passed. Assert it directly on a noisy ring.
        let mut ring = Vec::new();
        for idx in 0..200_i64 {
            let angle = (idx as f64) * std::f64::consts::TAU / 200.0;
            let radius = 5_000.0 + ((idx % 7) as f64) * 3.0;
            ring.push(p(
                (radius * angle.cos()).round() as i64,
                (radius * angle.sin()).round() as i64,
            ));
        }
        let tol = 250.0;
        let out = simplify_ring(&ring, tol);
        assert!(out.len() < ring.len());

        // Output vertices are a subsequence of the input, anchored at index 0.
        assert_eq!(out[0], ring[0]);
        let mut search_from = 1usize;
        let mut positions = vec![0usize];
        for vertex in &out[1..] {
            let found = ring[search_from..]
                .iter()
                .position(|candidate| candidate == vertex)
                .map(|offset| search_from + offset)
                .expect("output vertex must come from the input ring");
            positions.push(found);
            search_from = found + 1;
        }

        for window in positions.windows(2) {
            let (start, end) = (window[0], window[1]);
            let (a, b) = (ring[start], ring[end]);
            for vertex in &ring[start + 1..end] {
                let dist = distance_to_squared(*vertex, a, b).sqrt();
                assert!(
                    dist <= tol + 1e-6,
                    "vertex {vertex:?} sits {dist} from its output segment, tolerance {tol}"
                );
            }
        }
    }

    #[test]
    fn larger_tolerance_never_adds_vertices() {
        let ring: Vec<Point2> = (0..64_i64)
            .map(|idx| {
                let wobble = if idx % 3 == 0 { 40 } else { 0 };
                p(idx * 100, wobble)
            })
            .map(|mut vertex| {
                if vertex.y == 0 && vertex.x % 200 == 0 {
                    vertex.y = 7;
                }
                vertex
            })
            .collect();
        let coarse = simplify_ring(&ring, 50.0).len();
        let finer = simplify_ring(&ring, 5.0).len();
        assert!(coarse <= finer, "coarse={coarse} finer={finer}");
    }

    #[test]
    fn repeated_simplification_does_not_drift_beyond_tolerance() {
        let ring = vec![
            p(0, 0),
            p(1_000, 0),
            p(1_000, 12),
            p(2_000, 15),
            p(2_000, 1_000),
            p(15, 1_000),
            p(0, 900),
        ];
        let tol = 60.0;
        let once = simplify_ring(&ring, tol);
        let twice = simplify_ring(&once, tol);
        assert!(twice.len() <= once.len());
        // Every vertex retained by the second pass is one the first pass kept,
        // so the second pass cannot introduce geometry the first discarded.
        for vertex in &twice {
            assert!(once.contains(vertex), "{vertex:?} was reintroduced");
        }
    }

    #[test]
    fn simplify_p_keeps_hole_topology_and_area() {
        let outer = Polygon {
            points: vec![p(0, 0), p(50_000, 0), p(50_000, 50_000), p(0, 50_000)],
        };
        let hole = Polygon {
            points: vec![
                p(10_000, 10_000),
                p(10_000, 40_000),
                p(40_000, 40_000),
                p(40_000, 10_000),
            ],
        };
        let input = vec![ExPolygon {
            contour: outer,
            holes: vec![hole],
        }];
        let out = simplify_p(&input, TOL_001MM);
        assert_eq!(out.len(), 1, "one outline: {out:?}");
        assert_eq!(out[0].holes.len(), 1, "the hole must survive: {out:?}");

        let input_area =
            i128::from(50_000) * i128::from(50_000) - i128::from(30_000) * i128::from(30_000);
        let output_area = (ring_area2(&out[0].contour.points)
            + out[0]
                .holes
                .iter()
                .map(|hole| ring_area2(&hole.points))
                .sum::<i128>())
        .abs()
            / 2;
        assert_eq!(output_area, input_area, "set-preserving at this tolerance");
    }

    #[test]
    fn simplify_p_is_the_identity_at_a_non_positive_tolerance() {
        let input = vec![ExPolygon {
            contour: Polygon {
                points: vec![p(0, 0), p(1, 0), p(0, 1)],
            },
            holes: Vec::new(),
        }];
        assert_eq!(simplify_p(&input, 0.0), input);
    }

    #[test]
    fn douglas_peucker_keeps_holes_and_skips_the_union() {
        let polygon = ExPolygon {
            contour: Polygon {
                points: vec![p(0, 0), p(10, 0), p(20, 0), p(20, 20), p(0, 20)],
            },
            holes: vec![Polygon {
                points: vec![p(5, 5), p(5, 15), p(15, 15), p(15, 5)],
            }],
        };
        let out = super::douglas_peucker(&polygon, 1.0);
        assert_eq!(
            out.holes.len(),
            1,
            "holes are simplified independently, not merged"
        );
        assert_eq!(out.holes[0].points, polygon.holes[0].points);
        assert!(!out.contour.points.contains(&p(10, 0)));
        assert_eq!(out.contour.points.len(), 4);
    }

    #[test]
    fn dp_seeds_the_first_split_at_the_vertex_furthest_from_the_ring_start() {
        // Canonical closes the ring and starts with a degenerate (anchor ==
        // floater) segment at the ring's first vertex, so the first split is the
        // vertex furthest from it — not the ring's last index.
        let ring = vec![
            p(0, 0),
            p(10, 0),
            p(10, 10),
            p(0, 40),
            p(-10, 10),
            p(-10, 0),
        ];
        let out = simplify_ring(&ring, 1.0);
        assert_eq!(out[0], p(0, 0));
        assert!(
            out.contains(&p(0, 40)),
            "the farthest vertex must be retained as the first split: {out:?}"
        );
    }
}
