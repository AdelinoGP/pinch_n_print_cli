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

use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

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
    /// The number of extrusion beads (walls) this vertex should carry, as
    /// decided by the `assign_bead_counts` pass. `None` until that pass
    /// runs.
    pub bead_count: Option<u32>,
    /// Fractional position within a bead-count transition region, in the
    /// range `[0, 1]`. `0.0` for a vertex that is not on a transition
    /// boundary (or was snapped to an existing endpoint). Set by the
    /// `apply_transitions` pass and consumed by downstream propagation /
    /// toolpath emission to blend beadings across a transition ramp.
    ///
    /// Mirrors OrcaSlicer's per-node `transition_ratio` field
    /// (`SkeletalTrapezoidation.cpp`, `applyTransitions` L1487+).
    pub transition_ratio: f64,
}

/// A `SkeletalTrapezoidationGraph` half-edge: a Voronoi half-edge annotated
/// with the radius bounds and centrality flag Arachne's later passes need.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidationEdge` (the per-edge payload
/// of Arachne's `HalfEdge`). Topology fields (`start_vertex`/`twin`/`next`/
/// `prev`) are a direct, unmodified copy of the corresponding
/// [`crate::voronoi::HalfEdge`] fields — same index space, same sentinel
/// convention ([`crate::voronoi::NO_INDEX`] for "no value").
#[derive(Debug, Clone, PartialEq)]
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
    /// Whether this edge sits in the middle of a bead-count transition
    /// region. Set by Step 3's transition-region propagation pass.
    pub is_transition_middle: bool,
    /// Whether this edge sits at the end of a bead-count transition region.
    /// Set by Step 3's transition-region propagation pass.
    pub is_transition_end: bool,
    /// For rib edges (`EdgeType::EXTRA_VD`), the synthetic twin edge in the
    /// quad cell that points back toward the spine side. This is the existing
    /// graph `twin` relationship recorded from the rib edge's perspective.
    pub rib_twin: Option<usize>,
    /// The quad cell this edge belongs to, if any. Populated by Step 1's
    /// [`super::rib::build_quad_rib_topology`] pass.
    pub quad_cell: Option<u32>,
    /// Classification of this edge: normal skeleton edge, synthetic rib edge,
    /// or transition end. Populated by [`super::rib::build_quad_rib_topology`].
    pub edge_type: super::rib::EdgeType,
    /// Transition-mid annotations placed by
    /// [`super::propagation::generate_transition_mids`] before `apply_transitions`
    /// splits edges at these positions. Each entry records the split position
    /// along this half-edge, the bead count below the split, and the radius at
    /// which the transition occurs.
    pub transition_mids: Vec<TransitionMiddle>,
}

/// A single transition-middle annotation on a half-edge, placed by
/// [`generate_transition_mids`].
///
/// Mirrors OrcaSlicer's `TransitionMiddle` struct
/// (`SkeletalTrapezoidation.cpp:925-994`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionMiddle {
    /// Position along the half-edge at which the transition from
    /// `lower_bead_count` to `lower_bead_count + 1` occurs, expressed as a
    /// fraction of the edge length (`0.0..=1.0`).
    pub pos: f64,
    /// The bead count on the lower-R side of the transition. The higher-R
    /// side implicitly carries `lower_bead_count + 1`.
    pub lower_bead_count: u32,
    /// Radius (`distance_to_boundary`) at which the transition occurs.
    pub mid_r: f64,
}

impl Default for STHalfEdge {
    fn default() -> Self {
        Self {
            start_vertex: NO_INDEX,
            twin: NO_INDEX,
            next: NO_INDEX,
            prev: NO_INDEX,
            r_min: 0.0,
            r_max: 0.0,
            central: false,
            is_curved: false,
            is_transition_middle: false,
            is_transition_end: false,
            rib_twin: None,
            quad_cell: None,
            edge_type: super::rib::EdgeType::NORMAL,
            transition_mids: Vec::new(),
        }
    }
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
    /// Whether [`super::centrality::filter_central`] has been run on this
    /// graph yet. Always `false` immediately after
    /// [`SkeletalTrapezoidationGraph::from_polygons`]; `filter_central` sets
    /// it `true` on completion.
    ///
    /// Exists to disambiguate, for
    /// [`super::bead_count::assign_bead_counts`] (AC-N1), "centrality was
    /// never computed" from "centrality was computed and genuinely found no
    /// central edges" — both look identical if you only inspect
    /// [`STHalfEdge::central`], since a fresh graph's edges already default
    /// `central` to `false`.
    pub centrality_filtered: bool,
    /// Synthetic rib/quad-cell topology produced by
    /// [`super::rib::build_quad_rib_topology`]. Empty until that pass runs.
    pub rib: super::rib::RibData,
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

        // Defensive bounds-check on raw `boostvoronoi` vertex positions
        // (D-112-MMU-TOPOLOGY / D-113B-WIDE-REGION-COORD-INSTABILITY). See
        // `clamp_implausible_vertex`'s doc comment for the full rationale and
        // the empirical data backing the threshold.
        let input_bbox = segments_bbox(&segments);
        let vertices: Vec<STVertex> = he_graph
            .vertices
            .iter()
            .map(|v| {
                let position = clamp_implausible_vertex(*v, input_bbox);
                STVertex {
                    position,
                    distance_to_boundary: nearest_boundary_distance(
                        position.x, position.y, &segments,
                    ),
                    bead_count: None,
                    transition_ratio: 0.0,
                }
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
                    is_transition_middle: false,
                    is_transition_end: false,
                    rib_twin: None,
                    quad_cell: None,
                    edge_type: super::rib::EdgeType::NORMAL,
                    transition_mids: Vec::new(),
                }
            })
            .collect();

        Ok(Self {
            vertices,
            edges,
            centrality_filtered: false,
            rib: super::rib::RibData::default(),
        })
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

/// Minimum allowed clamp margin around an input polygon's bounding box, as a
/// fraction of that bbox's diagonal length. See [`clamp_implausible_vertex`]
/// for the full rationale and the empirical data this threshold is grounded
/// in.
const IMPLAUSIBLE_VERTEX_MARGIN_RATIO: f64 = 0.05;

/// Absolute floor (mm) for [`clamp_implausible_vertex`]'s margin, so a tiny
/// input polygon (small bbox diagonal) still gets a sane minimum allowance.
const IMPLAUSIBLE_VERTEX_MARGIN_FLOOR_MM: f64 = 1.0;

/// Bounding box (`min_x, min_y, max_x, max_y`) of a segment set, in
/// scaled-integer units represented as `f64` for direct comparison against
/// `f64` Voronoi vertex coordinates. `segments` is assumed non-empty (all
/// call sites in this module already guard on that via
/// [`SkeletalTrapezoidationGraph::from_polygons`]'s `EmptyInput` check).
fn segments_bbox(segments: &[Segment]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for s in segments {
        for p in [s.a, s.b] {
            min_x = min_x.min(p.x as f64);
            min_y = min_y.min(p.y as f64);
            max_x = max_x.max(p.x as f64);
            max_y = max_y.max(p.y as f64);
        }
    }
    (min_x, min_y, max_x, max_y)
}

/// Clamps a raw `boostvoronoi` vertex position back into a generous, bounded
/// margin around the input polygon's own bounding box (`bbox`).
///
/// # Why this exists (D-112-MMU-TOPOLOGY / D-113B-WIDE-REGION-COORD-INSTABILITY)
///
/// Direct instrumentation of the `cube_4color_arachne_per_color_footprint_within_bbox`
/// regression (see `docs/DEVIATION_LOG.md`) proved that `boostvoronoi`
/// occasionally reports a vertex tens to hundreds of thousands of mm away
/// from a small (~10-40mm bbox diagonal), sharp-cornered input polygon — the
/// same 2-D cell produces a clean graph on most layers and a corrupted one
/// (one wildly-misplaced vertex, an "x≈y runaway along a ~45° bisector"
/// signature) on a minority of layers whose cross-section develops a
/// slightly different near-degenerate corner/segment configuration.
/// `crate::voronoi::voronoi_from_segments` passes `boostvoronoi`'s own
/// `diagram.vertices()` output through verbatim (confirmed no scaling bug on
/// this crate's side), and per `docs/adr/0023-arachne-port-strategy.md`'s
/// degeneracy table, resolving the specific near-collinear/near-duplicate
/// multi-segment configurations that trigger this class of `boostvoronoi`
/// numerical instability is `preprocess.rs`'s (T-204) pre-snap
/// responsibility — but a captured reproduction (a per-color paint cell
/// fragmented into two disjoint quads, one carrying a ~17µm near-duplicate
/// wraparound vertex that survives pre-snapping intact) showed the
/// corruption is a multi-segment *interaction* across sites from both
/// fragments, not a single ring's own local near-collinearity, so it does
/// not reduce to one simple, provably-complete pre-snap rule. This
/// defensive check is the narrowly-scoped downstream safety net: it cannot
/// recover the numerically-correct vertex position (that information is
/// already lost once `boostvoronoi`'s internal arithmetic diverges), so it
/// clamps the implausible position back to a bounded region instead,
/// preventing the corrupted vertex from propagating into emitted wall
/// geometry tens to hundreds of thousands of mm away. This mirrors
/// `rib.rs`'s existing precedent of defensively filtering out other
/// `boostvoronoi` quirks (zero-length degenerate edges at segment endpoints)
/// rather than trying to "fix" the underlying sweep-line computation.
///
/// # Threshold derivation (not an arbitrary number)
///
/// A direct empirical sweep of every `SkeletalTrapezoidationGraph::from_polygons`
/// call made while re-running `cube_4color_arachne_per_color_footprint_within_bbox`
/// (temporary instrumentation, reverted — not part of this fix) measured, for
/// every produced vertex, how far (if at all) it fell outside its own input
/// polygon's raw bounding box, as a fraction of that bbox's diagonal length.
/// The result was a clean bimodal split with a wide gap and no borderline
/// cases: ordinary floating-point noise in legitimately-placed vertices never
/// exceeded ~0.47% of the bbox diagonal (worst observed: 0.1167mm on a
/// 25.0mm-diagonal cell), while every genuinely corrupted vertex escaped by
/// at least ~18.5% of the bbox diagonal (smallest observed: 6.27mm on a
/// 33.9mm-diagonal cell) — a ~40x gap between the two populations, and every
/// sample fell cleanly on one side or the other (no vertex was observed
/// escaping by an intermediate 1-15% fraction). [`IMPLAUSIBLE_VERTEX_MARGIN_RATIO`]
/// (5% of the bbox diagonal) sits centrally in that gap: roughly 10x above
/// the largest observed noise and comfortably (3.7x) below the smallest
/// observed real corruption, so it clamps every reproduced corruption case
/// while leaving every legitimately-placed vertex, even ones close to this
/// margin's boundary, untouched.
/// [`IMPLAUSIBLE_VERTEX_MARGIN_FLOOR_MM`] (1mm) exists only so a very small
/// input polygon's own bbox diagonal can't shrink the allowed margin below a
/// sane absolute minimum; it is not itself load-bearing for the reproduced
/// cases above (their 5%-of-diagonal margins, ~1.25-1.75mm, already exceed
/// it).
///
/// # Behavior
///
/// Clamps `v`'s `x`/`y` independently to `[bbox.min - margin, bbox.max +
/// margin]`. This is a lossy, best-effort recovery — the clamped position is
/// not claimed to be the numerically "correct" vertex position `boostvoronoi`
/// should have produced, only a bounded, plausible stand-in that keeps
/// downstream passes (bead-count assignment, centrality filtering, toolpath
/// emission) from ever seeing a wildly implausible coordinate. Topology
/// (vertex/edge indices, `next`/`prev`/`twin`) is untouched — only the
/// position of this one vertex slot changes. A no-op for every vertex
/// already within bounds (i.e. every legitimately-placed vertex).
fn clamp_implausible_vertex(v: Vertex, bbox: (f64, f64, f64, f64)) -> Vertex {
    let (min_x, min_y, max_x, max_y) = bbox;
    let diag = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    let margin = (IMPLAUSIBLE_VERTEX_MARGIN_RATIO * diag)
        .max(IMPLAUSIBLE_VERTEX_MARGIN_FLOOR_MM * UNITS_PER_MM);
    Vertex {
        x: v.x.clamp(min_x - margin, max_x + margin),
        y: v.y.clamp(min_y - margin, max_y + margin),
    }
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
pub fn edge_radius_bounds(vertices: &[STVertex], from_idx: usize, to_idx: usize) -> (f64, f64) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A vertex already within the input bbox is untouched.
    #[test]
    fn clamp_implausible_vertex_is_noop_inside_bbox() {
        let bbox = (0.0, 0.0, 1_000_000.0, 1_000_000.0);
        let v = Vertex {
            x: 500_000.0,
            y: 500_000.0,
        };
        let clamped = clamp_implausible_vertex(v, bbox);
        assert_eq!(clamped, v, "an in-bounds vertex must be returned unchanged");
    }

    /// A vertex within the allowed margin (5% of the bbox diagonal, floored
    /// at 1mm) just outside the raw bbox is also left untouched — this fix
    /// must not clip legitimately-placed medial-axis vertices that spill a
    /// small amount past the input polygon's own bbox.
    #[test]
    fn clamp_implausible_vertex_preserves_small_legitimate_overshoot() {
        // 1mm x 1mm bbox in units (UNITS_PER_MM = 10_000).
        let bbox = (0.0, 0.0, 1.0 * UNITS_PER_MM, 1.0 * UNITS_PER_MM);
        // Diagonal ~1.414mm; 5% of that (~0.0707mm) is smaller than the 1mm
        // floor, so the effective margin here is the 1mm floor.
        let v = Vertex {
            x: -0.5 * UNITS_PER_MM, // 0.5mm past the left edge: within the 1mm floor.
            y: 0.5 * UNITS_PER_MM,  // vertically centered in the bbox.
        };
        let clamped = clamp_implausible_vertex(v, bbox);
        assert_eq!(
            clamped, v,
            "a small overshoot within the margin must not be clamped"
        );
    }

    /// A `boostvoronoi`-style runaway vertex (matching the captured
    /// D-113B-WIDE-REGION-COORD-INSTABILITY reproduction: a vertex tens of
    /// thousands of units away from a ~25mm-diagonal input cell) is pulled
    /// back to within the allowed margin.
    #[test]
    fn clamp_implausible_vertex_clamps_wild_escape() {
        // Matches this fix's captured repro scale: a ~25mm x 25mm bbox.
        let side = 25.0 * UNITS_PER_MM;
        let bbox = (0.0, 0.0, side, side);
        let diag = (side * side + side * side).sqrt();
        let margin = (0.05 * diag).max(1.0 * UNITS_PER_MM);

        // A captured-scale runaway: escapes by ~65mm in x, matching the
        // reproduction's ~64.4mm worst-case escape.
        let v = Vertex {
            x: side + 65.0 * UNITS_PER_MM,
            y: -20.0 * UNITS_PER_MM,
        };
        let clamped = clamp_implausible_vertex(v, bbox);

        assert!(
            clamped.x <= side + margin + 1e-6,
            "x must be clamped to within the allowed margin: got {}, bound {}",
            clamped.x,
            side + margin
        );
        assert!(
            clamped.y >= 0.0 - margin - 1e-6,
            "y must be clamped to within the allowed margin: got {}, bound {}",
            clamped.y,
            -margin
        );
    }

    /// Regression test for the exact D-112-MMU-TOPOLOGY / D-113B reproduction
    /// captured this session: a per-color paint cell fragmented into two
    /// disjoint quads (one carrying a ~17µm near-duplicate wraparound vertex)
    /// that, before this fix, made `boostvoronoi` report a vertex tens of mm
    /// away (captured worst case: (1_125_000.0, -218_820.06), a ~64.4mm
    /// escape from the combined input bbox). After this fix, every vertex in
    /// the resulting graph must stay within a generous, bounded margin of the
    /// input polygons' own bbox.
    #[test]
    fn from_polygons_clamps_captured_runaway_reproduction() {
        let ring1 = expoly_from_units(vec![
            (1_374_998, 925_217),
            (1_374_998, 1_174_758),
            (1_249_998, 1_049_998),
            (1_374_843, 925_152),
        ]);
        let ring2 = expoly_from_units(vec![
            (1_246_202, 1_046_200),
            (1_125_000, 1_046_355),
            (1_125_000, 952_117),
            (1_152_119, 952_117),
        ]);

        let graph = SkeletalTrapezoidationGraph::from_polygons(&[ring1, ring2])
            .expect("captured reproduction cells should build a skeletal graph");

        let (min_x, min_y, max_x, max_y): (f64, f64, f64, f64) =
            (1_125_000.0, 925_152.0, 1_374_998.0, 1_174_758.0);
        let diag = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
        // Generous bound: comfortably above the fix's own margin*sqrt(2)
        // worst case, comfortably below the captured pre-fix escape (~64mm).
        let bound_units = (0.10 * diag).max(3.0 * UNITS_PER_MM);

        for (i, v) in graph.vertices.iter().enumerate() {
            let dx = if v.position.x < min_x - bound_units {
                min_x - bound_units - v.position.x
            } else if v.position.x > max_x + bound_units {
                v.position.x - (max_x + bound_units)
            } else {
                0.0
            };
            let dy = if v.position.y < min_y - bound_units {
                min_y - bound_units - v.position.y
            } else if v.position.y > max_y + bound_units {
                v.position.y - (max_y + bound_units)
            } else {
                0.0
            };
            let escape = (dx * dx + dy * dy).sqrt();
            assert!(
                escape <= 1e-6,
                "vertex {i} at {:?} escapes the bounded margin by {escape} units \
                 (pre-fix this reproduction produced an escape of ~644770 units / 64.4mm)",
                v.position
            );
        }
    }

    fn expoly_from_units(points: Vec<(i64, i64)>) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: points.into_iter().map(|(x, y)| Point2 { x, y }).collect(),
            },
            holes: Vec::new(),
        }
    }
}
