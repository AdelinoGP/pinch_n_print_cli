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

/// A segment Voronoi diagram, half-edge indexed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HalfEdgeGraph {
    /// All Voronoi vertices (circle events), indexed by [`HalfEdge::start_vertex`].
    pub vertices: Vec<Vertex>,
    /// All half-edges, indexed by [`HalfEdge::twin`]/`next`/`prev`.
    pub edges: Vec<HalfEdge>,
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

    let diagram = Builder::<i64>::default()
        .with_segments(lines.iter())
        .map_err(map_bv_error)?
        .build()
        .map_err(map_bv_error)?;

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

    Ok(HalfEdgeGraph { vertices, edges })
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
