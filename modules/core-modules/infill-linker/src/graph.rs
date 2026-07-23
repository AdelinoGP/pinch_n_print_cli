// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: FillBase.cpp + the BoundaryInfillGraph struct
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

use slicer_ir::{ExPolygon, Point2, Polygon};

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryRing {
    /// Ring polygon in deterministic traversal order.
    pub polygon: Polygon,
    /// Arc position of the ring's first point.
    pub pos_of_first_point: f64,
    /// Closed perimeter length of the ring.
    pub length: f64,
    /// Source expolygon index.
    pub polygon_index: usize,
    /// Source hole index, or `None` for an outer contour.
    pub hole_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryInfillGraph {
    polygons_outer: Vec<ExPolygon>,
    rings: Vec<BoundaryRing>,
    total_len: f64,
}

impl BoundaryInfillGraph {
    /// Builds an arc-length graph from outer contours and holes.
    #[must_use]
    pub fn new(polygons_outer: &[ExPolygon]) -> Self {
        let mut rings = Vec::new();
        let mut pos_of_first_point = 0.0;

        for (polygon_index, expolygon) in polygons_outer.iter().enumerate() {
            Self::push_ring(
                &mut rings,
                &mut pos_of_first_point,
                &expolygon.contour,
                polygon_index,
                None,
            );
            for (hole_index, hole) in expolygon.holes.iter().enumerate() {
                Self::push_ring(
                    &mut rings,
                    &mut pos_of_first_point,
                    hole,
                    polygon_index,
                    Some(hole_index),
                );
            }
        }

        Self {
            polygons_outer: polygons_outer.to_vec(),
            rings,
            total_len: pos_of_first_point,
        }
    }

    /// Returns the source boundary polygons.
    #[must_use]
    pub fn polygons_outer(&self) -> &[ExPolygon] {
        &self.polygons_outer
    }

    /// Returns the flattened contour and hole arc table.
    #[must_use]
    pub fn rings(&self) -> &[BoundaryRing] {
        &self.rings
    }

    /// Returns the total flattened boundary length in IR units.
    #[must_use]
    pub fn total_len(&self) -> f64 {
        self.total_len
    }

    /// Returns the starting arc position for a contour or hole.
    #[must_use]
    pub fn pos_of_first_point(
        &self,
        polygon_index: usize,
        hole_index: Option<usize>,
    ) -> Option<f64> {
        self.rings
            .iter()
            .find(|ring| ring.polygon_index == polygon_index && ring.hole_index == hole_index)
            .map(|ring| ring.pos_of_first_point)
    }

    /// Projects a point to the nearest boundary arc position.
    #[must_use]
    pub fn project(&self, point: Point2) -> Option<f64> {
        let mut nearest = None;

        for ring in &self.rings {
            let points = &ring.polygon.points;
            if points.is_empty() {
                continue;
            }

            let mut segment_start = 0.0;
            for index in 0..points.len() {
                let start = points[index];
                let end = points[(index + 1) % points.len()];
                let dx = end.x as f64 - start.x as f64;
                let dy = end.y as f64 - start.y as f64;
                let segment_len = dx.hypot(dy);
                let (parameter, projected_x, projected_y) = if segment_len == 0.0 {
                    (0.0, start.x as f64, start.y as f64)
                } else {
                    let point_dx = point.x as f64 - start.x as f64;
                    let point_dy = point.y as f64 - start.y as f64;
                    let parameter = ((point_dx * dx + point_dy * dy) / (segment_len * segment_len))
                        .clamp(0.0, 1.0);
                    (
                        parameter,
                        start.x as f64 + parameter * dx,
                        start.y as f64 + parameter * dy,
                    )
                };
                let distance_x = point.x as f64 - projected_x;
                let distance_y = point.y as f64 - projected_y;
                let distance_squared = distance_x * distance_x + distance_y * distance_y;
                let position = ring.pos_of_first_point + segment_start + parameter * segment_len;

                if nearest
                    .as_ref()
                    .is_none_or(|candidate: &(f64, f64)| distance_squared < candidate.0)
                {
                    nearest = Some((distance_squared, position));
                }
                segment_start += segment_len;
            }
        }

        nearest.map(|(_, position)| position)
    }

    /// Returns the forward boundary distance from one arc position to another.
    #[must_use]
    pub fn distance_along_boundary(&self, from: f64, to: f64) -> f64 {
        if self.total_len == 0.0 || !from.is_finite() || !to.is_finite() {
            return 0.0;
        }

        let from = from.rem_euclid(self.total_len);
        let to = to.rem_euclid(self.total_len);
        (to - from).rem_euclid(self.total_len)
    }

    /// Returns the forward distance when it is within the supplied threshold.
    #[must_use]
    pub fn walk_distance(&self, from: f64, to: f64, threshold: f64) -> Option<f64> {
        if threshold.is_sign_negative() || !threshold.is_finite() {
            return None;
        }
        let distance = self.distance_along_boundary(from, to);
        (distance <= threshold).then_some(distance)
    }

    fn push_ring(
        rings: &mut Vec<BoundaryRing>,
        next_position: &mut f64,
        polygon: &Polygon,
        polygon_index: usize,
        hole_index: Option<usize>,
    ) {
        let length = polygon_length(polygon);
        rings.push(BoundaryRing {
            polygon: polygon.clone(),
            pos_of_first_point: *next_position,
            length,
            polygon_index,
            hole_index,
        });
        *next_position += length;
    }
}

fn polygon_length(polygon: &Polygon) -> f64 {
    if polygon.points.is_empty() {
        return 0.0;
    }

    polygon
        .points
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = polygon.points[(index + 1) % polygon.points.len()];
            let dx = end.x as f64 - start.x as f64;
            let dy = end.y as f64 - start.y as f64;
            dx.hypot(dy)
        })
        .sum()
}
