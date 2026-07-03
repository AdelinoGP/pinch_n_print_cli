// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/utils/SkeletalTrapezoidationGraph.hpp,
// SkeletalTrapezoidationJoint.hpp, SkeletalTrapezoidationEdge.hpp, HalfEdge.hpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `SkeletalTrapezoidationGraph`: the Orca-shaped half-edge graph, built by
//! wrapping [`crate::voronoi::voronoi_from_segments`]'s boostvoronoi-shaped
//! [`crate::voronoi::HalfEdgeGraph`] with the per-vertex/per-edge fields
//! Arachne's bead-count and centrality passes need.
//!
//! # Design notes
//!
//! - **Topology is a 1:1 mirror of the source diagram.** `STVertex` index
//!   `i` corresponds to `HalfEdgeGraph::vertices[i]`, and `STHalfEdge` index
//!   `i` corresponds to `HalfEdgeGraph::edges[i]`. `start_vertex`/`twin`/
//!   `next`/`prev` are copied straight across unchanged, so the involution
//!   and winding invariants of the source diagram carry over unmodified.
//! - **Sentinel convention.** Topology fields use [`crate::voronoi::NO_INDEX`]
//!   (`usize::MAX`) exactly as Step 2 does, not `Option<usize>` — kept
//!   consistent with the layer below rather than diverging.
//! - **`distance_to_boundary`** (per vertex) is computed directly as the
//!   nearest point-to-segment distance from the Voronoi vertex to any input
//!   polygon edge, rather than trusting boostvoronoi's cell-equidistance
//!   property algebraically. This is O(V·E), which is acceptable at the
//!   scale this graph operates on (single-layer perimeter/hole polygons).
//! - **`r_min`/`r_max`** (per edge) are derived on demand from the edge's
//!   two endpoints' `distance_to_boundary`, mirroring OrcaSlicer's
//!   `start_R = edge.from->data.distance_to_boundary` /
//!   `end_R = edge.to->data.distance_to_boundary` pattern
//!   (`SkeletalTrapezoidation.cpp:932-933`). An edge's "to" vertex is not
//!   stored directly (this is a half-edge structure) — it is read off the
//!   edge's twin's `start_vertex`, matching boostvoronoi's own
//!   `edge.vertex1() == edge.twin().vertex0()` convention. When an endpoint
//!   is unresolvable (an infinite ray/line edge has no finite vertex on one
//!   or both ends — see [`crate::voronoi::NO_INDEX`]), the missing side
//!   falls back to the resolvable side's value, or `0.0` if neither side
//!   resolves, so `r_min`/`r_max` are always defined, finite, and
//!   `r_min <= r_max`, never `NaN`.
//! - **`central`** is a plain `bool` (OrcaSlicer's tri-state `UNKNOWN/NO/YES`
//!   simplifies to this per the design doc) and always defaults to `false`
//!   here — P112's centrality pass is responsible for filling it in.
//! - **Error type.** [`SktError`] wraps [`crate::voronoi::VoronoiError`]
//!   (via `From`) rather than reusing it directly, because this layer has
//!   its own failure modes that don't fit `VoronoiError`'s variants (empty
//!   polygon input, a degenerate ring with fewer than 3 points) before
//!   `voronoi_from_segments` is ever called.

use std::fmt;

use slicer_ir::{ExPolygon, Point2, Polygon};

use crate::voronoi::{self, Segment, Vertex, VoronoiError, NO_INDEX};

/// A `SkeletalTrapezoidationGraph` vertex: a Voronoi vertex (circle event)
/// annotated with its distance to the nearest input polygon boundary edge.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidationJoint` (the per-node payload
/// of Arachne's `HalfEdgeNode`), whose `distance_to_boundary` field this
/// carries forward.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct STVertex {
    /// This vertex's position, in the same representation as
    /// [`crate::voronoi::HalfEdgeGraph::vertices`] (`f64` coordinates in the
    /// scaled-integer unit space of the input polygons).
    pub position: Vertex,
    /// Nearest distance from `position` to any input polygon boundary edge.
    ///
    /// A Voronoi vertex is, by construction, equidistant from its ≥2
    /// generating sites; this is that equidistant value, computed directly
    /// via point-to-segment distance rather than trusted algebraically from
    /// `boostvoronoi`'s internal state.
    pub distance_to_boundary: f64,
}

/// A `SkeletalTrapezoidationGraph` half-edge: a Voronoi half-edge annotated
/// with the radius bounds and centrality flag Arachne's later passes need.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidationEdge` (the per-edge payload
/// of Arachne's `HalfEdge`). Topology fields (`start_vertex`/`twin`/`next`/
/// `prev`) are a direct, unmodified copy of the corresponding
/// [`crate::voronoi::HalfEdge`] fields — same index space, same sentinel
/// convention ([`crate::voronoi::NO_INDEX`] for "no value").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct STHalfEdge {
    /// Index into [`SkeletalTrapezoidationGraph::vertices`] for this
    /// half-edge's start point, or [`crate::voronoi::NO_INDEX`] if absent.
    pub start_vertex: usize,
    /// Index into [`SkeletalTrapezoidationGraph::edges`] for this
    /// half-edge's twin, or [`crate::voronoi::NO_INDEX`] if absent.
    pub twin: usize,
    /// Index into [`SkeletalTrapezoidationGraph::edges`] for the next
    /// half-edge around the incident cell, or [`crate::voronoi::NO_INDEX`]
    /// if absent.
    pub next: usize,
    /// Index into [`SkeletalTrapezoidationGraph::edges`] for the previous
    /// half-edge around the incident cell, or [`crate::voronoi::NO_INDEX`]
    /// if absent.
    pub prev: usize,
    /// Minimum of this edge's two endpoints' `distance_to_boundary`.
    /// Always finite, non-negative, and `<= r_max`.
    pub r_min: f64,
    /// Maximum of this edge's two endpoints' `distance_to_boundary`.
    /// Always finite, non-negative, and `>= r_min`.
    pub r_max: f64,
    /// Whether this edge is on the "central" spine of the skeleton.
    /// Always `false` here — filled in by P112's centrality pass.
    pub central: bool,
    /// `true` for a curved (parabolic point-to-segment bisector) edge;
    /// `false` for a straight edge. Copied straight from the source
    /// [`crate::voronoi::HalfEdge::is_curved`]; consumed by Step 4's
    /// parabolic edge discretization.
    pub is_curved: bool,
}

/// The Orca-shaped skeletal trapezoidation half-edge graph.
///
/// Built via [`SkeletalTrapezoidationGraph::from_polygons`] by wrapping
/// [`crate::voronoi::voronoi_from_segments`]'s output. Topology is a direct
/// 1:1 mirror of the source [`crate::voronoi::HalfEdgeGraph`]: `vertices[i]`
/// and `edges[i]` correspond to the source diagram's vertex/edge `i`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkeletalTrapezoidationGraph {
    /// All graph vertices, indexed by [`STHalfEdge::start_vertex`].
    pub vertices: Vec<STVertex>,
    /// All half-edges, indexed by [`STHalfEdge::twin`]/`next`/`prev`.
    pub edges: Vec<STHalfEdge>,
}

/// Errors from [`SkeletalTrapezoidationGraph::from_polygons`].
#[derive(Debug, Clone, PartialEq)]
pub enum SktError {
    /// `from_polygons` was called with an empty polygon slice, or every
    /// supplied polygon contributed zero boundary segments.
    EmptyInput,
    /// An input polygon ring (contour or hole) has fewer than 3 points, so
    /// it cannot be turned into a closed loop of boundary segments.
    DegeneratePolygon(String),
    /// The underlying segment Voronoi diagram construction failed.
    Voronoi(VoronoiError),
}

impl fmt::Display for SktError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SktError::EmptyInput => {
                write!(
                    f,
                    "SkeletalTrapezoidationGraph::from_polygons: empty polygon input"
                )
            }
            SktError::DegeneratePolygon(msg) => {
                write!(
                    f,
                    "SkeletalTrapezoidationGraph::from_polygons: degenerate polygon: {msg}"
                )
            }
            SktError::Voronoi(err) => {
                write!(
                    f,
                    "SkeletalTrapezoidationGraph::from_polygons: voronoi construction failed: {err}"
                )
            }
        }
    }
}

impl std::error::Error for SktError {}

impl From<VoronoiError> for SktError {
    fn from(err: VoronoiError) -> Self {
        SktError::Voronoi(err)
    }
}

impl SkeletalTrapezoidationGraph {
    /// Builds a `SkeletalTrapezoidationGraph` from closed input polygons.
    ///
    /// Each polygon's contour and holes are turned into closed loops of
    /// boundary [`Segment`]s (consecutive points, wrapping last-to-first),
    /// fed to [`crate::voronoi::voronoi_from_segments`], and the resulting
    /// diagram is annotated with `distance_to_boundary` per vertex and
    /// `r_min`/`r_max`/`central` per edge.
    ///
    /// Returns [`SktError::EmptyInput`] for an empty `polys` slice or when
    /// every polygon contributes zero segments, and
    /// [`SktError::DegeneratePolygon`] for any ring with fewer than 3
    /// points. Never panics.
    pub fn from_polygons(polys: &[ExPolygon]) -> Result<Self, SktError> {
        if polys.is_empty() {
            return Err(SktError::EmptyInput);
        }

        let mut segments = Vec::new();
        for poly in polys {
            segments.extend(ring_segments(&poly.contour)?);
            for hole in &poly.holes {
                segments.extend(ring_segments(hole)?);
            }
        }

        if segments.is_empty() {
            return Err(SktError::EmptyInput);
        }

        let he_graph = voronoi::voronoi_from_segments(&segments)?;

        let vertices: Vec<STVertex> = he_graph
            .vertices
            .iter()
            .map(|v| STVertex {
                position: *v,
                distance_to_boundary: nearest_boundary_distance(v.x, v.y, &segments),
            })
            .collect();

        let edges: Vec<STHalfEdge> = he_graph
            .edges
            .iter()
            .map(|e| {
                let to_vertex = if e.twin == NO_INDEX {
                    NO_INDEX
                } else {
                    he_graph
                        .edges
                        .get(e.twin)
                        .map(|twin_edge| twin_edge.start_vertex)
                        .unwrap_or(NO_INDEX)
                };
                let (r_min, r_max) = edge_radius_bounds(&vertices, e.start_vertex, to_vertex);
                STHalfEdge {
                    start_vertex: e.start_vertex,
                    twin: e.twin,
                    next: e.next,
                    prev: e.prev,
                    r_min,
                    r_max,
                    central: false,
                    is_curved: e.is_curved,
                }
            })
            .collect();

        Ok(Self { vertices, edges })
    }
}

/// Turns a closed polygon ring into its consecutive boundary segments
/// (wrapping last point back to first). Errors if `ring` has fewer than 3
/// points, since such a ring cannot represent a closed polygon loop.
fn ring_segments(ring: &Polygon) -> Result<Vec<Segment>, SktError> {
    let pts = &ring.points;
    if pts.len() < 3 {
        return Err(SktError::DegeneratePolygon(format!(
            "polygon ring has {} point(s); at least 3 required",
            pts.len()
        )));
    }

    let mut segments = Vec::with_capacity(pts.len());
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        segments.push(Segment { a, b });
    }
    Ok(segments)
}

/// Nearest distance from floating-point point `(x, y)` to any segment in
/// `segments`, via point-to-segment distance. `segments` is assumed
/// non-empty by all call sites in this module.
fn nearest_boundary_distance(x: f64, y: f64, segments: &[Segment]) -> f64 {
    segments
        .iter()
        .map(|s| point_to_segment_distance_f64(x, y, s.a, s.b))
        .fold(f64::INFINITY, f64::min)
}

/// Distance from floating-point point `(px, py)` to the closest point on
/// integer-coordinate segment `[a, b]`.
///
/// This duplicates the shape of
/// [`crate::geometry::point_to_segment_distance_squared`] rather than
/// reusing it directly: that helper takes an integer [`Point2`] query point
/// and rounds its projected closest point back to integer coordinates,
/// neither of which fits a `boostvoronoi`-computed `f64` Voronoi vertex
/// (see [`Vertex`]'s doc comment on why vertex coordinates are `f64`).
fn point_to_segment_distance_f64(px: f64, py: f64, a: Point2, b: Point2) -> f64 {
    let ax = a.x as f64;
    let ay = a.y as f64;
    let bx = b.x as f64;
    let by = b.y as f64;
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;

    let t = if len_sq == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0)
    };
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    let ddx = px - cx;
    let ddy = py - cy;
    (ddx * ddx + ddy * ddy).sqrt()
}

/// Derives `(r_min, r_max)` for an edge from its two endpoint vertex
/// indices, each either a valid index into `vertices` or
/// [`crate::voronoi::NO_INDEX`].
///
/// - Both resolvable: `(min, max)` of the two `distance_to_boundary`s.
/// - One resolvable: that side's value for both `r_min` and `r_max`.
/// - Neither resolvable (a fully unbounded line edge): `(0.0, 0.0)`.
///
/// Always returns finite, non-negative values with `r_min <= r_max`.
fn edge_radius_bounds(vertices: &[STVertex], from_idx: usize, to_idx: usize) -> (f64, f64) {
    let from_d = (from_idx != NO_INDEX)
        .then(|| vertices.get(from_idx).map(|v| v.distance_to_boundary))
        .flatten();
    let to_d = (to_idx != NO_INDEX)
        .then(|| vertices.get(to_idx).map(|v| v.distance_to_boundary))
        .flatten();

    match (from_d, to_d) {
        (Some(a), Some(b)) => (a.min(b), a.max(b)),
        (Some(a), None) | (None, Some(a)) => (a, a),
        (None, None) => (0.0, 0.0),
    }
}
