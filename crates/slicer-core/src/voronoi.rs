//! Segment Voronoi diagram construction, wrapping the `boostvoronoi` crate.
//!
//! This is the T-201 foundations layer for the M2 Arachne port
//! (`docs/adr/0023-arachne-port-strategy.md`). Arachne's
//! `SkeletalTrapezoidationGraph` is built from a **segment** Voronoi diagram
//! of a polygon's edges, so this module wraps `boostvoronoi::Builder`'s
//! mixed point/segment sweep-line construction rather than a plain
//! Fortune's-algorithm point-Voronoi implementation.
//!
//! `boostvoronoi` requires host-only compilation (it is not `wasm32`-safe),
//! so this module is gated behind the `host-algos` feature, matching
//! `slicer_core::algos` and `slicer_core::medial_axis`.
//!
//! # Degeneracy-handling contract (ADR-0023)
//!
//! `voronoi_from_segments` assumes its input has *already* been pre-snapped
//! by the caller (T-204's pre-processing pipeline) for the degeneracy
//! classes that Boost-VD cannot handle on its own:
//!
//! | Class | Handling |
//! |---|---|
//! | Collinear input points | Relies on Boost-VD's own built-in handling — no pre-snap needed. |
//! | T-junctions (segment endpoint touching another segment's interior) | Caller must pre-snap: subdivide the touched segment so the contact becomes a shared endpoint. |
//! | Duplicate vertices (coincident endpoints) | Caller must pre-snap: dedupe coincident endpoints before calling in. |
//! | Near-collinear-within-`epsilon_offset` segments | Caller must pre-snap using `epsilon_offset` (~115 units) as tolerance. |
//!
//! This wrapper does not perform any of that pre-snapping itself; it only
//! guards against empty input and surfaces `boostvoronoi`'s own build
//! errors (e.g. unresolved self-intersection) as [`VoronoiError`].

use boostvoronoi::builder::Builder;
use boostvoronoi::diagram::SourceCategory as BvSourceCategory;
use boostvoronoi::geometry::{Line as BvLine, Point as BvPoint};
use boostvoronoi::BvError;
use slicer_ir::Point2;
use std::fmt;

/// Sentinel index used for [`HalfEdge`] fields when `boostvoronoi` reports no
/// value for that slot (e.g. an infinite ray/line edge has no start vertex).
///
/// `usize::MAX` can never be a real index into [`HalfEdgeGraph::vertices`] or
/// [`HalfEdgeGraph::edges`] for any diagram this wrapper can produce, since
/// both vectors are built directly from `boostvoronoi`'s own (much smaller)
/// index space.
pub const NO_INDEX: usize = usize::MAX;

/// A 2-D segment site in slicer scaled-integer coordinates
/// (1 unit = 100 nm = 10⁻⁴ mm, see `docs/08_coordinate_system.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Segment start point.
    pub a: Point2,
    /// Segment end point.
    pub b: Point2,
}

/// A Voronoi diagram vertex (a "circle event" in Boost's sweep-line
/// terminology), in floating-point coordinates.
///
/// `boostvoronoi` vertex coordinates are computed in `f64` even though the
/// input sites are integral: a vertex is generally the point equidistant
/// from three sites, which is almost never exactly representable on the
/// input integer grid (e.g. the centroid of an odd-sized polygon). `f64` is
/// therefore the correct representation here, not the scaled-integer
/// [`Point2`] type used for input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// X coordinate, in the same scaled-integer unit space as the input
    /// segments' `Point2` coordinates, but represented as `f64`.
    pub x: f64,
    /// Y coordinate, in the same scaled-integer unit space as the input
    /// segments' `Point2` coordinates, but represented as `f64`.
    pub y: f64,
}

/// One directed half-edge of the Voronoi diagram, mirroring `boostvoronoi`'s
/// own `Edge` half-edge topology 1:1 by index (edge `i` in the source
/// diagram becomes `edges[i]` here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HalfEdge {
    /// Index into [`HalfEdgeGraph::vertices`] for this half-edge's start
    /// point, or [`NO_INDEX`] if the edge is an infinite ray/line with no
    /// finite start point.
    pub start_vertex: usize,
    /// Index into [`HalfEdgeGraph::edges`] for this half-edge's twin.
    pub twin: usize,
    /// Index into [`HalfEdgeGraph::edges`] for the next half-edge (CCW
    /// winding) around the incident cell.
    pub next: usize,
    /// Index into [`HalfEdgeGraph::edges`] for the previous half-edge (CCW
    /// winding) around the incident cell.
    pub prev: usize,
    /// Index of the Voronoi cell this half-edge borders, or [`NO_INDEX`] if
    /// `boostvoronoi` did not report one (should not occur in practice).
    pub cell: usize,
    /// `false` if the edge passes through an input segment's endpoint;
    /// `true` otherwise. Mirrors `boostvoronoi::diagram::Edge::is_primary`.
    pub is_primary: bool,
    /// `true` for a curved (parabolic point-to-segment bisector) edge;
    /// `false` for a straight edge.
    pub is_curved: bool,
}

/// Which part of a Voronoi cell's originating input site this cell was
/// generated from.
///
/// Mirrors `boostvoronoi::diagram::SourceCategory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCategory {
    /// The site was a standalone point (not part of a segment).
    SinglePoint,
    /// The site was a segment's start point.
    SegmentStart,
    /// The site was a segment's end point.
    SegmentEnd,
    /// The site was a full segment.
    Segment,
}

/// One Voronoi diagram cell, mirroring `boostvoronoi::diagram::Cell`'s
/// per-cell metadata 1:1 by index (cell `i` in the source diagram becomes
/// `cells[i]` here).
///
/// This is the per-cell metadata the faithful port of Arachne's
/// `transferEdge`/`makeRib` graph construction walks (per-cell, not just at
/// reflex corners) — see `docs/adr/0023-arachne-port-strategy.md` and
/// packet 113c.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VCell {
    /// `true` if this cell's site is a standalone point.
    /// Mirrors `boostvoronoi::diagram::Cell::contains_point`.
    pub contains_point: bool,
    /// `true` if this cell's site is a segment.
    /// Mirrors `boostvoronoi::diagram::Cell::contains_segment`.
    pub contains_segment: bool,
    /// `true` if this cell's site is a segment's start point.
    /// Mirrors `boostvoronoi::diagram::Cell::contains_segment_startpoint`.
    pub contains_segment_startpoint: bool,
    /// `true` if this cell's site is a segment's end point.
    /// Mirrors `boostvoronoi::diagram::Cell::contains_segment_endpoint`.
    pub contains_segment_endpoint: bool,
    /// Index of this cell's site within the original input (the `segments`
    /// slice passed to [`voronoi_from_segments`]).
    /// Mirrors `boostvoronoi::diagram::Cell::source_index`.
    pub source_index: usize,
    /// Which part of the input site this cell was generated from.
    /// Mirrors `boostvoronoi::diagram::Cell::source_category`.
    pub source_category: SourceCategory,
    /// Index into [`HalfEdgeGraph::edges`] for a half-edge incident to this
    /// cell, or [`NO_INDEX`] if the cell has no incident edges (see
    /// `is_degenerate`). Mirrors
    /// `boostvoronoi::diagram::Cell::get_incident_edge`.
    pub incident_edge: usize,
    /// `true` if this cell has no incident edges.
    /// Mirrors `boostvoronoi::diagram::Cell::is_degenerate`.
    pub is_degenerate: bool,
}

/// A segment Voronoi diagram, half-edge indexed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HalfEdgeGraph {
    /// All Voronoi vertices (circle events), indexed by [`HalfEdge::start_vertex`].
    pub vertices: Vec<Vertex>,
    /// All half-edges, indexed by [`HalfEdge::twin`]/`next`/`prev`.
    pub edges: Vec<HalfEdge>,
    /// All Voronoi cells, indexed by [`HalfEdge::cell`]. Mirrors
    /// `boostvoronoi::diagram::Diagram::cells`.
    pub cells: Vec<VCell>,
}

/// Errors from [`voronoi_from_segments`].
#[derive(Debug, Clone, PartialEq)]
pub enum VoronoiError {
    /// `voronoi_from_segments` was called with an empty segment slice.
    EmptyInput,
    /// The input violates `boostvoronoi`'s non-overlap contract in a way
    /// that survived pre-snapping (e.g. unresolved self-intersection).
    /// Per ADR-0023, resolving T-junctions and duplicate vertices ahead of
    /// this call is the caller's responsibility (T-204's pre-processing
    /// pipeline); this variant surfaces inputs where that did not happen.
    DegenerateInput(String),
    /// An unexpected error surfaced by the `boostvoronoi` crate itself.
    InternalBoostError(String),
    /// `boostvoronoi`'s `robust_fpt::is_finite()` predicate panicked during
    /// `Builder::build()`. Captures the segment-set shape so callers can
    /// triage which input caused the failure; never maps to an empty graph.
    /// Coordinate bounds are in internal 100-nm units.
    PredicatePanic {
        /// Number of input segments.
        segment_count: usize,
        /// Minimum input endpoint X coordinate in internal units.
        min_x: i64,
        /// Minimum input endpoint Y coordinate in internal units.
        min_y: i64,
        /// Maximum input endpoint X coordinate in internal units.
        max_x: i64,
        /// Maximum input endpoint Y coordinate in internal units.
        max_y: i64,
        /// Whether two distinct segments share an endpoint.
        has_duplicate_endpoint: bool,
        /// Whether any input segment has identical endpoints.
        has_zero_length_segment: bool,
        /// Whether two segments sharing an endpoint are nearly parallel.
        has_near_collinear_pair: bool,
    },
}

impl fmt::Display for VoronoiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoronoiError::EmptyInput => {
                write!(f, "voronoi_from_segments: empty segment input")
            }
            VoronoiError::DegenerateInput(msg) => {
                write!(f, "voronoi_from_segments: degenerate input: {msg}")
            }
            VoronoiError::InternalBoostError(msg) => {
                write!(f, "voronoi_from_segments: boostvoronoi error: {msg}")
            }
            VoronoiError::PredicatePanic {
                segment_count,
                min_x,
                min_y,
                max_x,
                max_y,
                has_duplicate_endpoint,
                has_zero_length_segment,
                has_near_collinear_pair,
            } => write!(
                f,
                "voronoi_from_segments: boostvoronoi predicate panic on {segment_count} segments (bounds: x=[{min_x}, {max_x}], y=[{min_y}, {max_y}], duplicate={has_duplicate_endpoint}, zero_length={has_zero_length_segment}, near_collinear={has_near_collinear_pair})"
            ),
        }
    }
}

impl std::error::Error for VoronoiError {}

/// Maps a `boostvoronoi` build-time error onto [`VoronoiError`].
fn map_bv_error(err: BvError) -> VoronoiError {
    match err {
        BvError::SelfIntersecting(msg) => VoronoiError::DegenerateInput(msg),
        other => VoronoiError::InternalBoostError(other.to_string()),
    }
}

/// Maps a `boostvoronoi` source category onto [`SourceCategory`].
fn map_source_category(cat: BvSourceCategory) -> SourceCategory {
    match cat {
        BvSourceCategory::SinglePoint => SourceCategory::SinglePoint,
        BvSourceCategory::SegmentStart => SourceCategory::SegmentStart,
        BvSourceCategory::SegmentEnd => SourceCategory::SegmentEnd,
        BvSourceCategory::Segment => SourceCategory::Segment,
    }
}

/// Builds a segment Voronoi diagram from `segments` via `boostvoronoi`.
///
/// Guards `segments.is_empty()` and returns [`VoronoiError::EmptyInput`]
/// *before* constructing anything or touching `boostvoronoi` — no
/// allocation past the error path, no panic.
///
/// Deterministic for a given input segment order: `boostvoronoi`'s
/// sweep-line construction is not seeded by hashing (no `HashMap`/`HashSet`
/// over float keys anywhere in this wrapper), so repeated calls with the
/// same `segments` slice produce identical output.
///
/// Callers are responsible for pre-snapping T-junctions, duplicate
/// vertices, and near-collinear-within-`epsilon_offset` segments per the
/// module-level degeneracy table (ADR-0023); this function does not
/// perform that pre-snapping itself.
pub fn voronoi_from_segments(segments: &[Segment]) -> Result<HalfEdgeGraph, VoronoiError> {
    if segments.is_empty() {
        return Err(VoronoiError::EmptyInput);
    }

    let lines: Vec<BvLine<i64>> = segments
        .iter()
        .map(|s| {
            BvLine::new(
                BvPoint { x: s.a.x, y: s.a.y },
                BvPoint { x: s.b.x, y: s.b.y },
            )
        })
        .collect();

    let builder = Builder::<i64>::default()
        .with_segments(lines.iter())
        .map_err(map_bv_error)?;

    // `AssertUnwindSafe` is required because `Builder` does not implement
    // `UnwindSafe`; this is safe because all boostvoronoi state is discarded
    // when `build()` panics and no partially-constructed state is observed.
    let build_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.build()));
    let diagram = match build_result {
        Ok(Ok(diagram)) => diagram,
        Ok(Err(err)) => return Err(map_bv_error(err)),
        Err(_) => {
            let segment_count = segments.len();
            let first_segment = segments[0];
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (
                first_segment.a.x,
                first_segment.a.y,
                first_segment.a.x,
                first_segment.a.y,
            );
            let mut has_zero_length_segment = false;
            for segment in segments {
                min_x = min_x.min(segment.a.x).min(segment.b.x);
                min_y = min_y.min(segment.a.y).min(segment.b.y);
                max_x = max_x.max(segment.a.x).max(segment.b.x);
                max_y = max_y.max(segment.a.y).max(segment.b.y);
                has_zero_length_segment |= segment.a == segment.b;
            }

            const NEAR_COLLINEAR_EPSILON: f64 = 1e-9;
            let mut has_duplicate_endpoint = false;
            let mut has_near_collinear_pair = false;
            for (left_index, left) in segments.iter().enumerate() {
                for right in segments.iter().skip(left_index + 1) {
                    let shares_endpoint = left.a == right.a
                        || left.a == right.b
                        || left.b == right.a
                        || left.b == right.b;
                    if !shares_endpoint {
                        continue;
                    }

                    has_duplicate_endpoint = true;
                    let left_dx = (i128::from(left.b.x) - i128::from(left.a.x)) as f64;
                    let left_dy = (i128::from(left.b.y) - i128::from(left.a.y)) as f64;
                    let right_dx = (i128::from(right.b.x) - i128::from(right.a.x)) as f64;
                    let right_dy = (i128::from(right.b.y) - i128::from(right.a.y)) as f64;
                    let cross = (left_dx * right_dy - left_dy * right_dx).abs();
                    let direction_scale = left_dx.hypot(left_dy) * right_dx.hypot(right_dy);
                    if direction_scale > 0.0 && cross <= NEAR_COLLINEAR_EPSILON * direction_scale {
                        has_near_collinear_pair = true;
                    }
                }
            }

            return Err(VoronoiError::PredicatePanic {
                segment_count,
                min_x,
                min_y,
                max_x,
                max_y,
                has_duplicate_endpoint,
                has_zero_length_segment,
                has_near_collinear_pair,
            });
        }
    };

    let vertices = diagram
        .vertices()
        .iter()
        .map(|v| Vertex { x: v.x(), y: v.y() })
        .collect();

    let edges = diagram
        .edges()
        .iter()
        .map(|e| HalfEdge {
            start_vertex: e.vertex0().map(|v| v.usize()).unwrap_or(NO_INDEX),
            twin: e.twin().map(|t| t.usize()).unwrap_or(NO_INDEX),
            next: e.next().map(|n| n.usize()).unwrap_or(NO_INDEX),
            prev: e.prev().map(|p| p.usize()).unwrap_or(NO_INDEX),
            cell: e.cell().map(|c| c.usize()).unwrap_or(NO_INDEX),
            is_primary: e.is_primary(),
            is_curved: e.is_curved(),
        })
        .collect();

    let cells = diagram
        .cells()
        .iter()
        .map(|c| VCell {
            contains_point: c.contains_point(),
            contains_segment: c.contains_segment(),
            contains_segment_startpoint: c.contains_segment_startpoint(),
            contains_segment_endpoint: c.contains_segment_endpoint(),
            source_index: c.source_index().usize(),
            source_category: map_source_category(c.source_category()),
            incident_edge: c.get_incident_edge().map(|e| e.usize()).unwrap_or(NO_INDEX),
            is_degenerate: c.is_degenerate(),
        })
        .collect();

    Ok(HalfEdgeGraph {
        vertices,
        edges,
        cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point2 {
        Point2 { x, y }
    }

    #[test]
    fn empty_input_returns_err_before_touching_boost() {
        let result = voronoi_from_segments(&[]);
        assert_eq!(result, Err(VoronoiError::EmptyInput));
    }

    #[test]
    fn single_segment_builds_without_panic() {
        let segments = [Segment {
            a: p(0, 0),
            b: p(1000, 0),
        }];
        match voronoi_from_segments(&segments) {
            Ok(graph) => assert!(!graph.edges.is_empty()),
            Err(err) => panic!("single segment should build, got {err}"),
        }
    }
}
