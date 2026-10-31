// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PolygonUtils.cpp,
// src/libslic3r/ClipperUtils.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Portable nearest-boundary distance query used by the owned Lightning builder.
//!
//! The query is deliberately simple: an exhaustive scan over the boundary
//! segments of a finite set of expolygons. Lightning's anchor/overhang decisions
//! need the distance from a candidate node to the nearest planning boundary, and
//! the number of boundary segments per layer is small (tens), so a linear scan
//! is both correct and cheap. No spatial acceleration structure is built: it
//! would add state that cannot be cross-checked against the recorded evidence.
//!
//! Units: IR integer units (`1 unit = 100 nm`). Distances are returned in units
//! as `f64`; convert with [`crate::polygon::units_to_mm`] when reporting mm.

use slicer_ir::{ExPolygon, Point2};

use crate::polygon;

/// Address of one boundary segment inside the queried expolygon set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentAddress {
    /// Index into the expolygon slice the field was built from.
    pub polygon_index: u32,
    /// `0` = contour, `1..n` = holes.
    pub ring_id: u32,
    /// `0..n-1` in the expolygon's ring order.
    pub segment_id: u32,
}

/// Result of a nearest-boundary query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryHit {
    /// Euclidean distance in IR units from the query point to the segment.
    pub distance: f64,
    /// The closest point on the boundary.
    pub point: Point2,
    /// Which boundary segment is closest.
    pub segment: SegmentAddress,
}

/// Exact nearest-boundary field over a fixed expolygon set.
#[derive(Debug, Clone, Default)]
pub struct BoundaryDistanceField {
    /// `(address, start, end)` for every boundary segment, in input order.
    segments: Vec<(SegmentAddress, Point2, Point2)>,
    polygons: Vec<ExPolygon>,
}

impl BoundaryDistanceField {
    /// Build a field over the boundary of every ring in `polys`.
    pub fn new(polys: &[ExPolygon]) -> Self {
        let mut segments = Vec::new();
        for (polygon_index, poly) in polys.iter().enumerate() {
            for (ring_id, ring) in std::iter::once(&poly.contour)
                .chain(poly.holes.iter())
                .enumerate()
            {
                let points = &ring.points;
                if points.len() < 2 {
                    continue;
                }
                for segment_id in 0..points.len() {
                    let start = points[segment_id];
                    let end = points[(segment_id + 1) % points.len()];
                    segments.push((
                        SegmentAddress {
                            polygon_index: polygon_index as u32,
                            ring_id: ring_id as u32,
                            segment_id: segment_id as u32,
                        },
                        start,
                        end,
                    ));
                }
            }
        }
        Self {
            segments,
            polygons: polys.to_vec(),
        }
    }

    /// Empty field: no boundary to measure against.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Number of boundary segments in the field.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Nearest boundary point to `pt`, or `None` for an empty field.
    pub fn nearest_boundary(&self, pt: Point2) -> Option<BoundaryHit> {
        let mut best: Option<BoundaryHit> = None;
        for &(address, start, end) in &self.segments {
            let candidate = closest_point_on_segment(pt, start, end);
            let distance = distance_units(pt, candidate);
            match best {
                Some(hit) if hit.distance <= distance => {}
                _ => {
                    best = Some(BoundaryHit {
                        distance,
                        point: candidate,
                        segment: address,
                    })
                }
            }
        }
        best
    }

    /// Euclidean distance to the nearest boundary segment, in IR units.
    pub fn distance(&self, pt: Point2) -> Option<f64> {
        self.nearest_boundary(pt).map(|hit| hit.distance)
    }

    /// True when `pt` lies inside the union of the field's expolygons.
    pub fn contains(&self, pt: Point2) -> bool {
        self.polygons
            .iter()
            .any(|poly| polygon::point_in_expolygon(pt, poly))
    }

    /// Distance to the nearest boundary, negated when `pt` is inside.
    ///
    /// The sign uses the union of the field's expolygons, so a point in a hole
    /// of one expolygon but inside another is still reported as inside.
    pub fn signed_distance(&self, pt: Point2) -> Option<f64> {
        let distance = self.distance(pt)?;
        Some(if self.contains(pt) {
            -distance
        } else {
            distance
        })
    }
}

/// Exact closest point on the segment `[a, b]` to `p`, rounded toward the
/// segment interior when the projection is not integral.
pub fn closest_point_on_segment(p: Point2, a: Point2, b: Point2) -> Point2 {
    let dx = (b.x - a.x) as i128;
    let dy = (b.y - a.y) as i128;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0 {
        return a;
    }
    let px = (p.x - a.x) as i128;
    let py = (p.y - a.y) as i128;
    let t = px * dx + py * dy;
    if t <= 0 {
        return a;
    }
    if t >= len_sq {
        return b;
    }
    // Round each projected coordinate to the nearest lattice point on the segment.
    let ratio = t as f64 / len_sq as f64;
    Point2 {
        x: a.x + (ratio * dx as f64).round() as i64,
        y: a.y + (ratio * dy as f64).round() as i64,
    }
}

/// Euclidean distance between two points in IR units.
pub fn distance_units(a: Point2, b: Point2) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::Polygon;

    fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
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
    fn distance_from_the_centre_of_a_square_is_half_the_edge() {
        let field = BoundaryDistanceField::new(&[square(0, 0, 10_000, 10_000)]);
        assert_eq!(field.segment_count(), 4);
        let hit = field.nearest_boundary(Point2 { x: 5000, y: 5000 }).unwrap();
        assert_eq!(hit.distance, 5000.0);
        assert_eq!(hit.segment.ring_id, 0);
        // Signed distance is negative inside, positive outside.
        assert_eq!(
            field.signed_distance(Point2 { x: 5000, y: 5000 }),
            Some(-5000.0)
        );
        assert_eq!(
            field.signed_distance(Point2 { x: 15_000, y: 5000 }),
            Some(5000.0)
        );
    }

    #[test]
    fn nearest_boundary_of_a_hole_is_the_hole_wall() {
        let mut poly = square(0, 0, 10_000, 10_000);
        poly.holes.push(Polygon {
            points: vec![
                Point2 { x: 4000, y: 4000 },
                Point2 { x: 6000, y: 4000 },
                Point2 { x: 6000, y: 6000 },
                Point2 { x: 4000, y: 6000 },
            ],
        });
        let field = BoundaryDistanceField::new(std::slice::from_ref(&poly));
        let hit = field.nearest_boundary(Point2 { x: 5000, y: 5000 }).unwrap();
        assert_eq!(hit.distance, 1000.0);
        assert_eq!(hit.segment.ring_id, 1, "closest segment is on the hole");
        assert!(!field.contains(Point2 { x: 5000, y: 5000 }));
        assert!(field.contains(Point2 { x: 1000, y: 1000 }));
    }

    #[test]
    fn closest_point_clamps_to_segment_endpoints() {
        let a = Point2 { x: 0, y: 0 };
        let b = Point2 { x: 4000, y: 0 };
        assert_eq!(closest_point_on_segment(Point2 { x: -500, y: 0 }, a, b), a);
        assert_eq!(closest_point_on_segment(Point2 { x: 9000, y: 0 }, a, b), b);
        let mid = closest_point_on_segment(Point2 { x: 2000, y: 3000 }, a, b);
        assert_eq!(mid, Point2 { x: 2000, y: 0 });
        assert_eq!(distance_units(Point2 { x: 2000, y: 3000 }, mid), 3000.0);
    }

    #[test]
    fn empty_field_has_no_boundary() {
        let field = BoundaryDistanceField::new(&[]);
        assert!(field.is_empty());
        assert!(field.nearest_boundary(Point2 { x: 0, y: 0 }).is_none());
        assert!(field.distance(Point2 { x: 0, y: 0 }).is_none());
    }
}
