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

use slicer_ir::{ExPolygon, Point2, Point3WithWidth, Polygon};

/// Arc positions closer together than this are the same point.
///
/// One IR unit is 100 nm, which is below Clipper's integer resolution, so this
/// only ever suppresses a ring vertex that *coincides* with an infill endpoint
/// — never a vertex the connector genuinely has to route through.
const ARC_EPSILON_UNITS: f64 = 1.0;

/// Walk direction around a closed boundary ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingDirection {
    /// Increasing arc position — ring point order.
    Forward,
    /// Decreasing arc position — reverse ring point order.
    Backward,
}

/// Precomputed geometry for one closed boundary ring edge (`start -> next`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BoundaryEdge {
    pub(crate) start: Point2,
    pub(crate) dx: f64,
    pub(crate) dy: f64,
    pub(crate) len: f64,
    pub(crate) len_sq: f64,
    pub(crate) start_arc: f64,
    pub(crate) min_x: i64,
    pub(crate) min_y: i64,
    pub(crate) max_x: i64,
    pub(crate) max_y: i64,
}

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
    /// Tight axis-aligned bounding box of the ring in IR units (`i64`).
    pub(crate) bbox: (i64, i64, i64, i64),
    /// Precomputed per-edge geometry and cumulative arc offsets.
    pub(crate) edges: Vec<BoundaryEdge>,
}

impl BoundaryRing {
    /// True when `point` lies inside (or on) this ring's tight bounding box.
    #[inline]
    pub(crate) fn contains_bbox(&self, point: Point2) -> bool {
        let (min_x, min_y, max_x, max_y) = self.bbox;
        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
    }

    /// Exact squared distance from `point` to this ring's tight bounding box.
    #[inline]
    pub(crate) fn bbox_dist_sq(&self, point: Point2) -> f64 {
        let (min_x, min_y, max_x, max_y) = self.bbox;
        let px = point.x as f64;
        let py = point.y as f64;
        let dx = if px < min_x as f64 {
            min_x as f64 - px
        } else if px > max_x as f64 {
            px - max_x as f64
        } else {
            0.0
        };
        let dy = if py < min_y as f64 {
            min_y as f64 - py
        } else if py > max_y as f64 {
            py - max_y as f64
        } else {
            0.0
        };
        dx * dx + dy * dy
    }

    /// Projects `point` onto this ring, returning `(distance_squared, local_arc)`
    /// only when strictly closer than `best_d2` (pass `f64::INFINITY` for an
    /// unbounded query). Produces bit-identical results to an unpruned edge scan.
    #[must_use]
    pub(crate) fn project_bounded(&self, point: Point2, mut best_d2: f64) -> Option<(f64, f64)> {
        if self.edges.is_empty() || self.bbox_dist_sq(point) >= best_d2 {
            return None;
        }
        let px = point.x as f64;
        let py = point.y as f64;
        let mut best: Option<(f64, f64)> = None;
        for edge in &self.edges {
            let bx = if px < edge.min_x as f64 {
                edge.min_x as f64 - px
            } else if px > edge.max_x as f64 {
                px - edge.max_x as f64
            } else {
                0.0
            };
            let bx2 = bx * bx;
            if bx2 >= best_d2 {
                continue;
            }
            let by = if py < edge.min_y as f64 {
                edge.min_y as f64 - py
            } else if py > edge.max_y as f64 {
                py - edge.max_y as f64
            } else {
                0.0
            };
            if bx2 + by * by >= best_d2 {
                continue;
            }
            let parameter = if edge.len == 0.0 {
                0.0
            } else {
                let point_dx = px - edge.start.x as f64;
                let point_dy = py - edge.start.y as f64;
                ((point_dx * edge.dx + point_dy * edge.dy) / edge.len_sq).clamp(0.0, 1.0)
            };
            let projected_x = edge.start.x as f64 + parameter * edge.dx;
            let projected_y = edge.start.y as f64 + parameter * edge.dy;
            let distance_x = px - projected_x;
            let distance_y = py - projected_y;
            let distance_squared = distance_x * distance_x + distance_y * distance_y;
            if distance_squared < best_d2 {
                best_d2 = distance_squared;
                best = Some((distance_squared, edge.start_arc + parameter * edge.len));
                if best_d2 == 0.0 {
                    break;
                }
            }
        }
        best
    }

    /// Converts a graph-global arc position into a ring-local one.
    #[must_use]
    pub fn local_position(&self, position: f64) -> f64 {
        if self.length == 0.0 || !position.is_finite() {
            return 0.0;
        }
        (position - self.pos_of_first_point).rem_euclid(self.length)
    }

    /// Shorter arc distance between two graph-global positions, together with
    /// the direction that shorter walk runs in.
    ///
    /// The direction-preserving counterpart of the plain distance used to rank
    /// link candidates: a connector routed along the contour has to know *which*
    /// way round the ring is shorter, and a `min` of the two choices throws that
    /// away.
    #[must_use]
    pub fn directed_distance(&self, from: f64, to: f64) -> (f64, RingDirection) {
        if self.length == 0.0 {
            return (0.0, RingDirection::Forward);
        }
        let from = self.local_position(from);
        let to = self.local_position(to);
        let forward = (to - from).rem_euclid(self.length);
        let backward = (from - to).rem_euclid(self.length);
        if forward <= backward {
            (forward, RingDirection::Forward)
        } else {
            (backward, RingDirection::Backward)
        }
    }

    /// Ring vertices strictly between two graph-global arc positions, in walk
    /// order for `direction`.
    ///
    /// Vertices coincident with either end of the walk are excluded so the run
    /// can be spliced between two real points without duplicating them.
    #[must_use]
    pub fn vertices_between(&self, from: f64, to: f64, direction: RingDirection) -> Vec<Point2> {
        let count = self.polygon.points.len();
        if count == 0 || self.length == 0.0 {
            return Vec::new();
        }

        let from = self.local_position(from);
        let to = self.local_position(to);
        let span = match direction {
            RingDirection::Forward => (to - from).rem_euclid(self.length),
            RingDirection::Backward => (from - to).rem_euclid(self.length),
        };

        let mut walked = Vec::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let vertex_arc = edge.start_arc;
            let offset = match direction {
                RingDirection::Forward => (vertex_arc - from).rem_euclid(self.length),
                RingDirection::Backward => (from - vertex_arc).rem_euclid(self.length),
            };
            if offset > ARC_EPSILON_UNITS && offset < span - ARC_EPSILON_UNITS {
                walked.push((offset, self.polygon.points[index]));
            }
        }
        walked.sort_by(|(left, _), (right, _)| left.total_cmp(right));
        walked.into_iter().map(|(_, point)| point).collect()
    }
}

/// Materialises the contour walk between two joined infill endpoints.
///
/// Canonical `Fill::connect_infill` (`src/libslic3r/Fill/FillBase.cpp`) routes a
/// connector *along the contour*: `take_ccw_full` / `take_cw_full` copy the run
/// of ring vertices between two T-joints verbatim — no simplification, no arc
/// fitting, no collinearity merge. Canonical needs no containment test on the
/// result because the connector **is** exact boundary geometry; this reproduces
/// that property, which is what keeps a connector from cutting a chord across a
/// concave notch or a solid island.
///
/// Returns the intermediate points only: `start` and `end` stay owned by the two
/// paths being joined. `z` and `width` are lerped across the walk by arc
/// fraction; the remaining per-point attributes are inherited from `start`.
#[must_use]
pub fn contour_connector(
    ring: &BoundaryRing,
    from: f64,
    to: f64,
    start: &Point3WithWidth,
    end: &Point3WithWidth,
) -> Vec<Point3WithWidth> {
    let (total, direction) = ring.directed_distance(from, to);
    let start_point = Point2::from_mm(start.x, start.y);
    let end_point = Point2::from_mm(end.x, end.y);

    let mut connector = Vec::new();
    let mut previous = start_point;
    let mut walked = 0.0;
    for vertex in ring.vertices_between(from, to, direction) {
        if vertex == previous || vertex == end_point {
            continue;
        }
        walked += (vertex.x as f64 - previous.x as f64).hypot(vertex.y as f64 - previous.y as f64);
        let fraction = if total > 0.0 {
            (walked / total).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let (x, y) = vertex.to_mm();
        connector.push(Point3WithWidth {
            x,
            y,
            z: lerp(start.z, end.z, fraction),
            width: lerp(start.width, end.width, fraction),
            ..*start
        });
        previous = vertex;
    }
    connector
}

/// Materialises a bounded walk from a boundary position.
#[must_use]
pub fn contour_stub(
    ring: &BoundaryRing,
    from: f64,
    direction: RingDirection,
    budget_units: f64,
    anchor: &Point3WithWidth,
) -> Vec<Point3WithWidth> {
    if !budget_units.is_finite()
        || budget_units <= 0.0
        || !ring.length.is_finite()
        || ring.length <= 0.0
        || ring.polygon.points.is_empty()
    {
        return Vec::new();
    }

    let points = &ring.polygon.points;
    let count = points.len();
    let lengths: Vec<f64> = ring.edges.iter().map(|edge| edge.len).collect();
    let perimeter = ring.length;
    if !perimeter.is_finite() || perimeter <= 0.0 {
        return Vec::new();
    }

    let budget = budget_units.min(ring.length).min(perimeter);
    if budget <= 0.0 {
        return Vec::new();
    }

    let local = ring.local_position(from).min(perimeter);
    let mut segment = None;
    let mut arc = 0.0;
    match direction {
        RingDirection::Forward => {
            for (index, &length) in lengths.iter().enumerate() {
                if length > 0.0 && local < arc + length {
                    segment = Some((index, local - arc));
                    break;
                }
                arc += length;
            }
            if segment.is_none() {
                segment = lengths
                    .iter()
                    .enumerate()
                    .find(|(_, length)| **length > 0.0)
                    .map(|(index, length)| (index, 0.0_f64.min(*length)));
            }
        }
        RingDirection::Backward => {
            for (index, &length) in lengths.iter().enumerate() {
                if length > 0.0 && local > arc && local <= arc + length {
                    segment = Some((index, local - arc));
                    break;
                }
                arc += length;
            }
            if segment.is_none() {
                segment = lengths
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, length)| **length > 0.0)
                    .map(|(index, length)| (index, *length));
            }
        }
    }
    let Some((mut segment, mut offset)) = segment else {
        return Vec::new();
    };

    let point = |x: f32, y: f32| Point3WithWidth { x, y, ..*anchor };
    let mut walked = 0.0;
    let mut output = Vec::new();
    while walked < budget {
        let length = lengths[segment];
        let current = points[segment];
        let next = points[(segment + 1) % count];
        let remaining_in_segment = match direction {
            RingDirection::Forward => length - offset,
            RingDirection::Backward => offset,
        };
        let remaining_budget = budget - walked;

        if remaining_budget <= remaining_in_segment {
            let fraction = match direction {
                RingDirection::Forward => (offset + remaining_budget) / length,
                RingDirection::Backward => (offset - remaining_budget) / length,
            } as f32;
            let (current_x, current_y) = current.to_mm();
            let (next_x, next_y) = next.to_mm();
            output.push(point(
                lerp(current_x, next_x, fraction),
                lerp(current_y, next_y, fraction),
            ));
            break;
        }

        walked += remaining_in_segment;
        let vertex = match direction {
            RingDirection::Forward => next,
            RingDirection::Backward => current,
        };
        if walked < budget {
            let (x, y) = vertex.to_mm();
            output.push(point(x, y));
        }

        segment = match direction {
            RingDirection::Forward => (segment + 1) % count,
            RingDirection::Backward => (segment + count - 1) % count,
        };
        while lengths[segment] <= 0.0 {
            segment = match direction {
                RingDirection::Forward => (segment + 1) % count,
                RingDirection::Backward => (segment + count - 1) % count,
            };
        }
        offset = match direction {
            RingDirection::Forward => 0.0,
            RingDirection::Backward => lengths[segment],
        };
    }
    output
}

fn lerp(from: f32, to: f32, fraction: f32) -> f32 {
    from + (to - from) * fraction
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
        let mut best_d2 = f64::INFINITY;
        let mut nearest = None;

        for ring in &self.rings {
            if let Some((distance_squared, local_arc)) = ring.project_bounded(point, best_d2) {
                best_d2 = distance_squared;
                nearest = Some(ring.pos_of_first_point + local_arc);
                if best_d2 == 0.0 {
                    break;
                }
            }
        }

        nearest
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
        let points = &polygon.points;
        let n = points.len();
        let mut min_x = i64::MAX;
        let mut min_y = i64::MAX;
        let mut max_x = i64::MIN;
        let mut max_y = i64::MIN;
        let mut edges = Vec::with_capacity(n);
        let mut cumulative = 0.0;
        for index in 0..n {
            let start = points[index];
            let end = points[(index + 1) % n];
            min_x = min_x.min(start.x);
            min_y = min_y.min(start.y);
            max_x = max_x.max(start.x);
            max_y = max_y.max(start.y);
            let dx = end.x as f64 - start.x as f64;
            let dy = end.y as f64 - start.y as f64;
            let len = dx.hypot(dy);
            let len_sq = len * len;
            edges.push(BoundaryEdge {
                start,
                dx,
                dy,
                len,
                len_sq,
                start_arc: cumulative,
                min_x: start.x.min(end.x),
                min_y: start.y.min(end.y),
                max_x: start.x.max(end.x),
                max_y: start.y.max(end.y),
            });
            cumulative += len;
        }
        rings.push(BoundaryRing {
            polygon: polygon.clone(),
            pos_of_first_point: *next_position,
            length: cumulative,
            polygon_index,
            hole_index,
            bbox: (min_x, min_y, max_x, max_y),
            edges,
        });
        *next_position += cumulative;
    }
}
