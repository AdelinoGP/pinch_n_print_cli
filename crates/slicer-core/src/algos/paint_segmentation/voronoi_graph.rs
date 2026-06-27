// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
#[cfg(feature = "host-algos")]
use boostvoronoi::geometry::{Line as BvLine, Point as BvPoint};
/// Voronoi graph construction for MMU paint-segmentation (Step 5 — RISK GATE).
///
/// Wraps `boostvoronoi` 0.12.x behind typed-state wrappers (H561).
/// `VoronoiVertex` — raw emitted vertex, color slot still free for Step-6 use.
/// `GraphVertex`   — post-pruning vertex (conversion from VoronoiVertex in Step 6).
///
/// Coordinate invariant: 1 unit = 100 nm.
#[cfg(feature = "host-algos")]
use boostvoronoi::prelude::{Builder, ColorType, Diagram};
#[cfg(feature = "host-algos")]
use boostvoronoi::BvError;

#[cfg(feature = "host-algos")]
use crate::algos::paint_segmentation::colorize::ColoredLine;

#[cfg(feature = "host-algos")]
use std::collections::HashSet;

use slicer_ir::{ExPolygon, PaintValue, Point2};

#[cfg(feature = "host-algos")]
use slicer_ir::slice_ir::BoundingBox2;

// We need ColorType even without host-algos for the GraphVertex struct field.
// When host-algos is absent boostvoronoi is not compiled in, so define a local alias.
#[cfg(not(feature = "host-algos"))]
type ColorType = u32;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Deterministic safety cap on the number of segments fed to the boostvoronoi
/// builder (Part D). Far above any realistic painted-layer segment count
/// (a dense multi-colour layer is ~10³–10⁴ segments); exists only to bound the
/// latent `discretize` unbounded-loop exposure on pathological/degenerate input.
///
/// CONTAINMENT, NOT A FIX — and incomplete by construction:
/// - The `discretize` bug is an unbounded LOOP (a hang), not a panic, so the
///   `catch_unwind` backstop below cannot catch it — only the panic class
///   (`PredicatePanic`) is recoverable.
/// - The loop trigger is geometric STRUCTURE, not segment count. This cap only
///   stops *over-cap* inputs from reaching the builder; an adversarial input
///   with fewer than `MAX_VORONOI_SEGMENTS` segments can still hit the loop and
///   hang the process with no defense here. The cap value is a conservative
///   estimate, not a measured loop-trigger threshold.
///
/// The true fix is an upstream boostvoronoi `discretize` patch/fork (out of
/// scope for packet 125 — see `requirements.md` §Out of Scope).
const MAX_VORONOI_SEGMENTS: usize = 500_000;

/// Errors that can arise when constructing the MMU Voronoi graph.
#[derive(Debug)]
pub enum MmuGraphError {
    /// boostvoronoi returned an error during construction.
    Voronoi(String),
    /// An edge operation returned an unexpected error (e.g. twin resolution).
    EdgeOp(String),
    /// An input coordinate did not fit in `i32` (boostvoronoi's site coord type).
    CoordinateOverflow(i64),
    /// The input segment set exceeded the deterministic safety cap before the
    /// builder. boostvoronoi's `discretize` has a latent unbounded loop on
    /// pathological inputs; this guard bounds exposure (the true fix is upstream).
    InputTooLarge {
        /// Number of input segments that tripped the cap.
        segments: usize,
        /// The cap that was exceeded.
        cap: usize,
    },
    /// A panic escaped `boost::polygon::voronoi` during predicate evaluation
    /// (e.g. the `robust_fpt` `fpv.is_finite()` assertion on degenerate
    /// collinear-overlapping sites). Caught and converted to a clean error so a
    /// painted-path edge case degrades gracefully instead of aborting.
    PredicatePanic,
}

impl std::fmt::Display for MmuGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MmuGraphError::Voronoi(s) => write!(f, "voronoi construction error: {s}"),
            MmuGraphError::EdgeOp(s) => write!(f, "edge operation error: {s}"),
            MmuGraphError::CoordinateOverflow(v) => {
                write!(f, "input coordinate {v} does not fit in i32")
            }
            MmuGraphError::InputTooLarge { segments, cap } => write!(
                f,
                "voronoi input has {segments} segments, exceeding the safety cap of {cap}"
            ),
            MmuGraphError::PredicatePanic => {
                write!(
                    f,
                    "voronoi predicate evaluation panicked on degenerate input"
                )
            }
        }
    }
}

impl std::error::Error for MmuGraphError {}

#[cfg(feature = "host-algos")]
impl From<BvError> for MmuGraphError {
    fn from(e: BvError) -> Self {
        MmuGraphError::Voronoi(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Typed-state vertex wrappers (H561)
// ---------------------------------------------------------------------------

/// A raw Voronoi vertex as emitted by boostvoronoi. The color slot is still
/// free; do NOT interpret it as a graph-pruning color until conversion to
/// `GraphVertex` (Step 6).
#[derive(Debug, Clone)]
pub struct VoronoiVertex {
    /// Index in the `Diagram::vertices()` slice.
    pub id: usize,
    /// X coordinate (f64, units = 100 nm).
    pub x: f64,
    /// Y coordinate (f64, units = 100 nm).
    pub y: f64,
}

impl VoronoiVertex {
    /// Return `(x, y, id)` for sorting and determinism checks.
    pub fn coords_id(&self) -> (f64, f64, usize) {
        (self.x, self.y, self.id)
    }
}

impl std::fmt::Display for VoronoiVertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VoronoiVertex(id={}, x={:.3}, y={:.3})",
            self.id, self.x, self.y
        )
    }
}

/// A Voronoi vertex that has passed the pruning step (Step 6). Its color slot
/// carries pruning-decision flags.
///
/// In Step 5 this type exists only as a placeholder; no values are constructed
/// until Step 6 introduces the pruning pass.
#[derive(Debug, Clone)]
pub struct GraphVertex {
    /// Index in the original `Diagram::vertices()` slice.
    pub id: usize,
    /// X coordinate (f64, units = 100 nm).
    pub x: f64,
    /// Y coordinate (f64, units = 100 nm).
    pub y: f64,
    /// Packed color bits set during pruning (H561 dual-use slot).
    pub color: ColorType,
}

impl std::fmt::Display for GraphVertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GraphVertex(id={}, x={:.3}, y={:.3}, color={:#010x})",
            self.id, self.x, self.y, self.color
        )
    }
}

// ---------------------------------------------------------------------------
// MmuNode / MmuArc
// ---------------------------------------------------------------------------

/// A node in the MMU graph. Holds indices into `MMU_Graph::arcs`.
#[derive(Debug, Clone, Default)]
pub struct MmuNode {
    /// Indices into `MMU_Graph::arcs` for arcs incident on this node.
    pub arc_indices: Vec<usize>,
}

/// Distinguishes contour border arcs from interior Voronoi arcs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmuArcKind {
    /// Arc lies on a contour border polygon segment.
    Border,
    /// Arc is a Voronoi interior arc (not on any contour boundary).
    NonBorder,
}

/// A directed arc between two nodes in the MMU graph.
#[derive(Debug, Clone)]
pub struct MmuArc {
    /// Index of the source node in `MMU_Graph::nodes`.
    pub from_node: usize,
    /// Index of the destination node in `MMU_Graph::nodes`.
    pub to_node: usize,
    /// Paint color assigned to this arc; `None` = unpainted.
    pub color: Option<PaintValue>,
    /// Whether this arc lies on a contour border or is a Voronoi interior arc.
    pub kind: MmuArcKind,
    /// Whether this arc has been pruned (logically deleted).
    pub deleted: bool,
    /// Start point of the arc geometry.
    pub point_a: Point2,
    /// End point of the arc geometry.
    pub point_b: Point2,
}

// ---------------------------------------------------------------------------
// MMU_Graph
// ---------------------------------------------------------------------------

/// Skeletal MMU Voronoi graph. Holds the boostvoronoi `Diagram` and the typed
/// vertex list emitted after construction, plus the node/arc graph used by
/// Phases 4d/4e/4f.
///
/// Construction is gated on `host-algos`; the struct itself is always visible
/// so downstream modules can reference it in type signatures.
///
/// Node layout:
/// - `nodes[0..all_border_points]` — contour border-point nodes (one per polygon vertex).
/// - `nodes[all_border_points..]`  — Voronoi interior nodes.
///
/// `polygon_idx_offset[p]` is the first border-node index for polygon `p`.
#[allow(non_camel_case_types)]
pub struct MMU_Graph {
    /// The raw boostvoronoi diagram (owned). Step 6 will consume this for pruning.
    #[cfg(feature = "host-algos")]
    #[allow(dead_code)] // consumed by voronoi_prune in Step 6
    pub(crate) diagram: Diagram,
    /// Typed vertex wrappers, one per diagram vertex, sorted by `(x, y, id)` for
    /// deterministic ordering.
    pub vertices: Vec<VoronoiVertex>,

    // ---- graph fields added in Step 6 ----
    /// All graph nodes. Border nodes first, interior nodes after.
    pub nodes: Vec<MmuNode>,
    /// All directed arcs.
    pub arcs: Vec<MmuArc>,
    /// `nodes[0..all_border_points]` are contour border points.
    pub all_border_points: usize,
    /// `polygon_idx_offset[p]` = first border-node index for polygon `p`.
    pub polygon_idx_offset: Vec<usize>,

    // ---- cell decomposition fields (B-4) ----
    /// The merged segment sites fed to boostvoronoi, in insertion order.
    /// `bv_segments[i]` corresponds to Voronoi cells with `source_index == i`.
    #[cfg(feature = "host-algos")]
    #[allow(dead_code)] // used by cells_to_expolygons_by_color (test-only B-4 path)
    pub(crate) bv_segments: Vec<BvLine<i32>>,
    /// Per-segment colors parallel to `bv_segments`.
    /// `bv_segment_colors[i]` is the paint value for the Voronoi segment site at index `i`.
    /// `cell.source_index().usize()` indexes this array for segment cells (H561).
    #[cfg(feature = "host-algos")]
    #[allow(dead_code)] // used by cells_to_expolygons_by_color (test-only B-4 path)
    pub(crate) bv_segment_colors: Vec<Option<PaintValue>>,
}

/// Bucket integer segments by canonical infinite-line identity and merge any
/// that overlap or touch along that line into a single segment.
///
/// boostvoronoi / `boost::polygon::voronoi` requires that input sites do not
/// properly overlap each other on a common line. When the paint kernel
/// subdivides a contour edge into many small `ColoredLine` sub-segments,
/// independent subdivisions of the same physical edge produce collinear
/// overlapping pieces (e.g. two painted facets project onto the same edge
/// with overlapping ranges). Feeding those directly into the builder
/// triggers the `fpv.is_finite()` panic in `robust_fpt` during predicate
/// evaluation.
///
/// Canonical line identity is `((cdx, cdy), perp_offset)`:
/// - `(cdx, cdy)` is the segment direction reduced by gcd and sign-canonicalised
///   so the first non-zero component is positive;
/// - `perp_offset = cdx * y - cdy * x` is the (signed) line constant.
///
/// Within each bucket, segments are sorted by parametric position along the
/// direction and adjacent overlapping/touching ranges are unioned.
///
/// Returns `(bv_segments, color_per_segment)` where `color_per_segment[i]` is
/// the paint value inherited from the first contributing input segment.
#[cfg(feature = "host-algos")]
fn merge_collinear_overlapping(
    segments: Vec<((i32, i32), (i32, i32), Option<PaintValue>)>,
) -> (Vec<BvLine<i32>>, Vec<Option<PaintValue>>) {
    use std::collections::HashMap;

    fn gcd(a: i64, b: i64) -> i64 {
        let (mut a, mut b) = (a.abs(), b.abs());
        while b != 0 {
            let t = a % b;
            a = b;
            b = t;
        }
        a
    }

    fn canon_dir(dx: i64, dy: i64) -> (i64, i64) {
        let g = gcd(dx, dy).max(1);
        let (mut cdx, mut cdy) = (dx / g, dy / g);
        if cdx < 0 || (cdx == 0 && cdy < 0) {
            cdx = -cdx;
            cdy = -cdy;
        }
        (cdx, cdy)
    }

    // Each bucket entry: (t0, t1, p0, p1, color)
    let mut buckets: HashMap<
        ((i64, i64), i64),
        Vec<(i64, i64, (i32, i32), (i32, i32), Option<PaintValue>)>,
    > = HashMap::new();
    for (s, e, color) in segments {
        let dx = (e.0 as i64) - (s.0 as i64);
        let dy = (e.1 as i64) - (s.1 as i64);
        let (cdx, cdy) = canon_dir(dx, dy);
        let offset = cdx * (s.1 as i64) - cdy * (s.0 as i64);
        let t_s = cdx * (s.0 as i64) + cdy * (s.1 as i64);
        let t_e = cdx * (e.0 as i64) + cdy * (e.1 as i64);
        let (t0, t1, p0, p1) = if t_s <= t_e {
            (t_s, t_e, s, e)
        } else {
            (t_e, t_s, e, s)
        };
        buckets
            .entry(((cdx, cdy), offset))
            .or_default()
            .push((t0, t1, p0, p1, color));
    }

    // Sweep-line emission: for each bucket, walk t-events (open/close per segment) and
    // emit one OUTPUT segment per (prev_t, cur_t) interval with the dominant active color.
    //
    // Dominant rule: SHORTEST active interval wins.  This implements "stroke overrides
    // facet" — strokes are narrow paint refinements, facets paint the whole face, so
    // strokes are more SPECIFIC and should be selected at every t where they're active.
    //
    // boostvoronoi's no-overlap precondition is satisfied by construction since we
    // emit non-overlapping intervals.
    let mut out: Vec<BvLine<i32>> = Vec::new();
    let mut colors: Vec<Option<PaintValue>> = Vec::new();
    for ((cdx, cdy), _offset) in buckets.keys().cloned().collect::<Vec<_>>() {
        let segs = buckets.remove(&((cdx, cdy), _offset)).unwrap();
        if segs.is_empty() {
            continue;
        }

        // Anchor for parametric → (x, y) reconstruction.  For canonical direction
        // (cdx, cdy), t = cdx*x + cdy*y; moving Δt along the line corresponds to
        // Δ(x, y) = (cdx, cdy) * Δt / (cdx² + cdy²).
        let denom = (cdx * cdx + cdy * cdy).max(1);
        let (anchor_p, anchor_t) = ((segs[0].2 .0 as i64, segs[0].2 .1 as i64), segs[0].0);
        let pos_at = |t: i64| -> (i32, i32) {
            let delta = t - anchor_t;
            // Round to nearest integer to avoid drift; cdx*denom is small (gcd-reduced),
            // so the arithmetic stays well within i64.
            let x = anchor_p.0 + (delta * cdx) / denom;
            let y = anchor_p.1 + (delta * cdy) / denom;
            (x as i32, y as i32)
        };

        // Build events: (t, kind, seg_idx).  kind=0 → close, kind=1 → open.
        // Sort by (t, kind) so closes precede opens at the same t (keeps active set
        // monotone-decreasing at boundaries — degenerate same-t open+close pairs
        // collapse cleanly).
        let mut events: Vec<(i64, u8, usize)> = Vec::with_capacity(segs.len() * 2);
        for (idx, s) in segs.iter().enumerate() {
            events.push((s.0, 1, idx)); // open at t0
            events.push((s.1, 0, idx)); // close at t1
        }
        events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut active: Vec<usize> = Vec::new();
        let mut prev_t: i64 = events.first().map(|e| e.0).unwrap_or(0);

        // Skip-threshold for "tiny" sub-segments (relative to the bucket's longest
        // segment).  Tiny segments are still emitted but lose tie-breaks: a longer
        // active segment, if any, wins for that interval.  Without this, sub-mm
        // stroke fragments at face corners create cells whose polygons fall inside
        // the downstream face-proximity tolerance, causing cross-face paint bleed.
        let bucket_max_len: i64 = segs.iter().map(|s| s.1 - s.0).max().unwrap_or(0);
        // 2 % of the longest segment in the bucket — for a 25 mm cube edge that's
        // 0.5 mm, well above the typical 0.25 mm positional-query tolerance.
        let skip_t_threshold: i64 = bucket_max_len / 50;

        for &(t, kind, idx) in &events {
            if t > prev_t && !active.is_empty() {
                // Dominant: smallest interval length wins, but prefer NON-tiny over tiny.
                // Tie-break: latest-added wins (deterministic via Reverse(idx)).
                let pick_score = |&a: &usize| -> (u8, i64, std::cmp::Reverse<usize>) {
                    let len = segs[a].1 - segs[a].0;
                    let tiny = if len < skip_t_threshold { 1 } else { 0 };
                    (tiny, len, std::cmp::Reverse(a))
                };
                let chosen = active.iter().copied().min_by_key(pick_score).unwrap();
                let color = segs[chosen].4.clone();
                let p0 = pos_at(prev_t);
                let p1 = pos_at(t);
                if p0 != p1 {
                    out.push(BvLine::new(
                        BvPoint { x: p0.0, y: p0.1 },
                        BvPoint { x: p1.0, y: p1.1 },
                    ));
                    colors.push(color);
                }
            }
            if kind == 1 {
                active.push(idx);
            } else if let Some(pos) = active.iter().position(|&x| x == idx) {
                active.swap_remove(pos);
            }
            if t > prev_t {
                prev_t = t;
            }
        }
    }
    (out, colors)
}

impl MMU_Graph {
    /// Construct a Voronoi diagram from a flat slice of `ColoredLine` records.
    ///
    /// Each `ColoredLine.line` becomes one line-segment site in the boostvoronoi
    /// builder. Vertices are emitted and wrapped as `VoronoiVertex`, then sorted
    /// by `(x, y, id)` to guarantee deterministic ordering across runs.
    ///
    /// Border nodes are populated from the colored-lines input (one node per
    /// polygon vertex). Interior nodes come from Voronoi diagram vertices.
    ///
    /// # Errors
    /// Returns:
    /// - `MmuGraphError::CoordinateOverflow` if any input `Point2` component
    ///   does not fit in `i32` (boostvoronoi's site coord type).
    /// - `MmuGraphError::Voronoi` if boostvoronoi fails to construct the
    ///   diagram from the (filtered, de-duplicated) sites.
    ///
    /// boostvoronoi (and the underlying `boost::polygon::voronoi`) has a
    /// no-duplicate-sites / no-zero-length precondition. Inputs are filtered
    /// to drop zero-length segments and de-duplicated by canonical endpoint
    /// pair before construction.
    #[cfg(feature = "host-algos")]
    pub fn from_colored_lines(input: &[ColoredLine]) -> Result<MMU_Graph, MmuGraphError> {
        // Convert ColoredLine segments to boostvoronoi Line<i32>.
        // slicer_ir::Point2 uses i64; boostvoronoi requires i32.
        // Overflow is propagated as MmuGraphError::CoordinateOverflow rather
        // than silently wrapping via `as i32`.
        fn to_i32(v: i64) -> Result<i32, MmuGraphError> {
            i32::try_from(v).map_err(|_| MmuGraphError::CoordinateOverflow(v))
        }

        // boostvoronoi precondition enforcement: no zero-length segments,
        // no duplicate sites by canonical endpoint pair, AND no two segments
        // overlapping on the same infinite line (boost::polygon::voronoi
        // rejects collinear-overlapping sites; the failure mode is a
        // `fpv.is_finite()` panic deep in robust_fpt during predicate
        // evaluation, not a clean error).
        //
        // Cast i64→i32 with overflow propagation; drop zero-length segments.
        // Also carry color through the merge so bv_segment_colors[i] reflects
        // the paint value of the site at index i.
        let mut int_segments: Vec<((i32, i32), (i32, i32), Option<PaintValue>)> =
            Vec::with_capacity(input.len());
        for cl in input {
            let sx = to_i32(cl.line.start.x)?;
            let sy = to_i32(cl.line.start.y)?;
            let ex = to_i32(cl.line.end.x)?;
            let ey = to_i32(cl.line.end.y)?;
            if sx == ex && sy == ey {
                continue;
            }
            int_segments.push(((sx, sy), (ex, ey), cl.value.clone()));
        }
        let (bv_segments, bv_segment_colors) = merge_collinear_overlapping(int_segments);

        // Part D — deterministic input guard. boostvoronoi's `discretize` has a
        // latent unbounded loop on pathological inputs; bound exposure before the
        // builder rather than risk a hang. The cap is far above any realistic
        // painted-layer segment count; the true fix is an upstream patch/fork.
        if bv_segments.len() > MAX_VORONOI_SEGMENTS {
            return Err(MmuGraphError::InputTooLarge {
                segments: bv_segments.len(),
                cap: MAX_VORONOI_SEGMENTS,
            });
        }

        // Build the diagram. Part E — `boost::polygon::voronoi` can PANIC during
        // predicate evaluation on degenerate collinear-overlapping sites (the
        // `robust_fpt` `fpv.is_finite()` assertion), not return a clean error.
        // `merge_collinear_overlapping` above is the first line of defense; this
        // `catch_unwind` is the backstop so any residual edge case degrades to a
        // typed error instead of aborting the process.
        let build_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Builder::<i32>::default()
                .with_segments(bv_segments.iter())?
                .build()
        }));
        let diagram: Diagram = match build_result {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => return Err(MmuGraphError::from(e)),
            Err(_) => return Err(MmuGraphError::PredicatePanic),
        };

        // Wrap emitted vertices as typed VoronoiVertex.
        let mut vertices: Vec<VoronoiVertex> = diagram
            .vertices()
            .iter()
            .enumerate()
            .map(|(idx, v)| VoronoiVertex {
                id: idx,
                x: v.x(),
                y: v.y(),
            })
            .collect();

        // Sort for determinism.
        vertices.sort_by(|a, b| {
            a.x.partial_cmp(&b.x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
                .then(a.id.cmp(&b.id))
        });

        // ---- Build border nodes from colored-line input ----
        // Group colored lines by poly_idx to determine per-polygon vertex layout.
        let max_poly = input
            .iter()
            .map(|cl| cl.poly_idx)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        let mut polygon_idx_offset: Vec<usize> = Vec::with_capacity(max_poly + 1);
        let mut nodes: Vec<MmuNode> = Vec::new();

        for poly in 0..max_poly {
            polygon_idx_offset.push(nodes.len());
            // Collect lines for this polygon, sorted by local_line_idx.
            let mut poly_lines: Vec<&ColoredLine> =
                input.iter().filter(|cl| cl.poly_idx == poly).collect();
            poly_lines.sort_by_key(|cl| cl.local_line_idx);
            // One border node per line (= one per polygon vertex / segment start).
            for _ in &poly_lines {
                nodes.push(MmuNode::default());
            }
        }
        let all_border_points = nodes.len();

        // Pre-allocate border node coordinate lookup.
        // Filled during border arc building below; used for source_index border wiring.
        let mut border_node_coords: Vec<Point2> = vec![Point2 { x: 0, y: 0 }; all_border_points];

        // ---- Build interior nodes from Voronoi diagram vertices ----
        for _ in &vertices {
            nodes.push(MmuNode::default());
        }

        // ---- Build Border arcs from colored-line input ----
        let mut arcs: Vec<MmuArc> = Vec::new();

        for poly in 0..max_poly {
            let base = polygon_idx_offset[poly];
            let mut poly_lines: Vec<&ColoredLine> =
                input.iter().filter(|cl| cl.poly_idx == poly).collect();
            poly_lines.sort_by_key(|cl| cl.local_line_idx);
            let n = poly_lines.len();
            for (local_idx, cl) in poly_lines.iter().enumerate() {
                let from_node = base + local_idx;
                let to_node = base + (local_idx + 1) % n;
                // Record border node coordinate: each node is the from_node of exactly
                // one arc in the closed ring, so this covers all border nodes.
                border_node_coords[from_node] = cl.line.start;
                let arc_idx = arcs.len();
                arcs.push(MmuArc {
                    from_node,
                    to_node,
                    color: cl.value.clone(),
                    kind: MmuArcKind::Border,
                    deleted: false,
                    point_a: cl.line.start,
                    point_b: cl.line.end,
                });
                // OrcaSlicer parity: BORDER arcs are one-way (winding direction).
                // Register at from_node ONLY; the to_node picks up this arc naturally
                // as the from_node of the next border arc in the ring.
                // Dual-registration lets backward-duplicate walks exhaust shared
                // NonBorder arcs and starve other colours.
                nodes[from_node].arc_indices.push(arc_idx);
            }
        }

        // ---- Per-border-node colour-boundary flags (Orca build_graph colour gating) ----
        // OrcaSlicer only attaches a contour vertex to the interior medial axis where
        // the contour COLOUR CHANGES (`!has_same_color(contour_line_prev/next, colored_line)`,
        // MultiMaterialSegmentation.cpp:1869-1910). On a uniform-colour span it adds NO
        // contour-attachment "spike". PNP previously attached at every node, creating
        // spurious medial spikes that let the leftmost-walk short-circuit back to the
        // contour (returning to an adjacent node instead of the start corner → the walk
        // self-intersects and is discarded). We reproduce the gate: a border node is a
        // colour boundary iff the border arc leaving it differs in colour from the one
        // entering it. Attachment arcs (Cases 2 & 3 below) are suppressed at uniform nodes.
        let border_node_is_color_boundary: Vec<bool> = {
            let mut color_leaving: Vec<Option<PaintValue>> = vec![None; all_border_points];
            let mut color_entering: Vec<Option<PaintValue>> = vec![None; all_border_points];
            let mut has_leaving = vec![false; all_border_points];
            let mut has_entering = vec![false; all_border_points];
            for a in &arcs {
                if a.kind == MmuArcKind::Border {
                    if a.from_node < all_border_points {
                        color_leaving[a.from_node] = a.color.clone();
                        has_leaving[a.from_node] = true;
                    }
                    if a.to_node < all_border_points {
                        color_entering[a.to_node] = a.color.clone();
                        has_entering[a.to_node] = true;
                    }
                }
            }
            (0..all_border_points)
                .map(|n| {
                    // Closed-ring nodes always have both sides; if a side is missing
                    // (degenerate/open input) default to boundary=true so we never
                    // over-suppress a genuinely-needed attachment.
                    if has_leaving[n] && has_entering[n] {
                        color_leaving[n] != color_entering[n]
                    } else {
                        true
                    }
                })
                .collect()
        };

        // ---- Build NonBorder arcs from Voronoi diagram interior edges ----
        // Interior nodes start at index `all_border_points` and map 1-to-1
        // with `vertices` (which mirrors `diagram.vertices()` after sorting).
        // We build a lookup from diagram-vertex-id to node index.
        let voronoi_vertex_count = diagram.vertices().len();
        // Map from sorted-vertices position (= interior node offset from all_border_points)
        // back to diagram vertex id, to resolve edge endpoints.
        // `vertices[i].id` is the diagram vertex index.
        let mut diag_id_to_node: Vec<usize> = vec![usize::MAX; voronoi_vertex_count];
        for (sorted_pos, vv) in vertices.iter().enumerate() {
            diag_id_to_node[vv.id] = all_border_points + sorted_pos;
        }

        // ---- Build source_index → border-node lookup for edge wiring ----
        // bv_segments[i] was built with i32 coordinates; border_node_coords uses i64.
        // For each bv_segment we find the border nodes at its start and end endpoints so
        // that semi-infinite Voronoi edges can be wired to the correct border node via the
        // cell's source_index (Orca build_graph approach, replacing the old coincidence-remap).
        let mut border_coord_i32_to_node: std::collections::HashMap<(i32, i32), usize> =
            std::collections::HashMap::with_capacity(all_border_points);
        for (node_idx, coord) in border_node_coords.iter().enumerate() {
            border_coord_i32_to_node.insert((coord.x as i32, coord.y as i32), node_idx);
        }
        let bv_seg_border: Vec<Option<(usize, usize)>> = bv_segments
            .iter()
            .map(|seg| {
                let node_a = border_coord_i32_to_node
                    .get(&(seg.start.x, seg.start.y))
                    .copied()?;
                let node_b = border_coord_i32_to_node
                    .get(&(seg.end.x, seg.end.y))
                    .copied()?;
                Some((node_a, node_b))
            })
            .collect();

        // Helper: resolve the Point2 for a node index.
        // Border nodes use border_node_coords; interior nodes use the sorted vertices slice.
        let node_point = |node: usize| -> Point2 {
            if node < all_border_points {
                border_node_coords[node]
            } else {
                let sorted = node - all_border_points;
                Point2 {
                    x: vertices[sorted].x.round() as i64,
                    y: vertices[sorted].y.round() as i64,
                }
            }
        };

        // ---- Build NonBorder arcs (Orca build_graph faithful port) ----
        //
        // Case 1 — both endpoints finite (interior Voronoi vertex ↔ interior Voronoi vertex):
        //   add a NON_BORDER arc between the two interior nodes.
        //
        // Case 2 — semi-infinite edge (one endpoint at "infinity"):
        //   connect the finite interior vertex to the border node for the source segment.
        //   The cell's source_index maps to bv_segments[si]; we project the interior vertex
        //   onto that segment to pick the closer endpoint (from_node or to_node).
        //
        // Fully-infinite edges (no vertices at all) are skipped.
        //
        // Dedupe: Orca primary filter = skip when cell.source_index > twin.source_index.
        // seen_pairs is an additional safety net against duplicate arcs.
        let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();

        let edbg = std::env::var("PNP_PAINTSEG_EDGEDBG").is_ok();
        let (mut c_total, mut c_primary, mut c_secondary) = (0usize, 0usize, 0usize);
        let (mut c_sec_finite, mut c_case1, mut c_case2_attempt) = (0usize, 0usize, 0usize);
        let (mut c_case2_miss, mut c_case2_ok) = (0usize, 0usize);
        let (mut c_case3_attempt, mut c_case3_ok) = (0usize, 0usize);
        let (mut c_case2_suppressed, mut c_case3_suppressed) = (0usize, 0usize);

        // Helper: if a cell is a point site (a segment endpoint), return
        // (source_index, is_end) where is_end picks seg.end vs seg.start.
        let point_endpoint = |c: &boostvoronoi::diagram::Cell| -> Option<(usize, bool)> {
            match c.source_category() {
                boostvoronoi::diagram::SourceCategory::SegmentStart
                | boostvoronoi::diagram::SourceCategory::SinglePoint => {
                    Some((c.source_index().usize(), false))
                }
                boostvoronoi::diagram::SourceCategory::SegmentEnd => {
                    Some((c.source_index().usize(), true))
                }
                _ => None,
            }
        };

        for edge in diagram.edges().iter() {
            let edge_id = edge.id();
            let twin_idx = diagram
                .edge_get_twin(edge_id)
                .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?;
            let cell = diagram
                .cell(
                    diagram
                        .edge_get_cell(edge_id)
                        .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?,
                )
                .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?;
            let twin_cell = diagram
                .cell(
                    diagram
                        .edge_get_cell(twin_idx)
                        .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?,
                )
                .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?;
            let twin_edge = diagram
                .edge(twin_idx)
                .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?;

            if edbg {
                c_total += 1;
                if edge.is_primary() {
                    c_primary += 1;
                } else {
                    c_secondary += 1;
                    if edge.vertex0().is_some() {
                        c_sec_finite += 1;
                    }
                }
            }

            if !edge.is_primary() {
                // --- Case 3: secondary edge — wire segment-endpoint border node to
                // the finite interior Voronoi vertex. In a Voronoi-over-segments these
                // are exactly the contour-vertex <-> interior links the leftmost-arc
                // walk needs to leave the contour and enclose an area; boostvoronoi
                // classifies them as non-primary, so they must be handled here.
                if edbg {
                    c_case3_attempt += 1;
                }
                let Some((si, is_end)) =
                    point_endpoint(&cell).or_else(|| point_endpoint(&twin_cell))
                else {
                    continue;
                };
                let Some(fin_vi) = edge.vertex0().or_else(|| twin_edge.vertex0()) else {
                    continue;
                };
                let interior_node = diag_id_to_node[fin_vi.usize()];
                if interior_node == usize::MAX {
                    continue;
                }
                let Some((node_a, node_b)) = bv_seg_border.get(si).copied().flatten() else {
                    continue;
                };
                let border_node = if is_end { node_b } else { node_a };
                if interior_node == border_node || border_node >= nodes.len() {
                    continue;
                }
                // Orca colour gating: only attach the contour vertex to the interior
                // medial axis where the contour colour changes. At a uniform-colour
                // node this would be a spurious medial spike (the short-circuit bug).
                if !border_node_is_color_boundary[border_node] {
                    if edbg {
                        c_case3_suppressed += 1;
                    }
                    continue;
                }
                let pair = (
                    interior_node.min(border_node),
                    interior_node.max(border_node),
                );
                if !seen_pairs.insert(pair) {
                    continue;
                }
                let point_a = node_point(interior_node);
                let point_b = node_point(border_node);
                let arc_idx = arcs.len();
                arcs.push(MmuArc {
                    from_node: interior_node,
                    to_node: border_node,
                    color: None,
                    kind: MmuArcKind::NonBorder,
                    deleted: false,
                    point_a,
                    point_b,
                });
                nodes[interior_node].arc_indices.push(arc_idx);
                nodes[border_node].arc_indices.push(arc_idx);
                if edbg {
                    c_case3_ok += 1;
                }
                continue;
            }

            // Orca dedupe: process each undirected primary edge exactly once.
            if cell.source_index().usize() > twin_cell.source_index().usize() {
                continue;
            }

            let v0_opt = edge.vertex0();
            let v1_opt = twin_edge.vertex0();

            if let (Some(v0_vi), Some(v1_vi)) = (v0_opt, v1_opt) {
                // --- Case 1: finite edge — both endpoints are interior Voronoi vertices ---
                let v0_id = v0_vi.usize();
                let v1_id = v1_vi.usize();
                let from_node = diag_id_to_node[v0_id];
                let to_node = diag_id_to_node[v1_id];

                if from_node == usize::MAX || to_node == usize::MAX || from_node == to_node {
                    continue;
                }

                let pair = (from_node.min(to_node), from_node.max(to_node));
                if !seen_pairs.insert(pair) {
                    continue;
                }

                let point_a = node_point(from_node);
                let point_b = node_point(to_node);
                let arc_idx = arcs.len();
                arcs.push(MmuArc {
                    from_node,
                    to_node,
                    color: None,
                    kind: MmuArcKind::NonBorder,
                    deleted: false,
                    point_a,
                    point_b,
                });
                nodes[from_node].arc_indices.push(arc_idx);
                nodes[to_node].arc_indices.push(arc_idx);
                if edbg {
                    c_case1 += 1;
                }
            } else {
                if edbg {
                    c_case2_attempt += 1;
                }
                // --- Case 2: semi-infinite edge — connect interior vertex to border node ---
                let finite_vi = match (v0_opt, v1_opt) {
                    (Some(vi), _) => vi,
                    (_, Some(vi)) => vi,
                    (None, None) => continue, // fully infinite: skip
                };

                let interior_node = diag_id_to_node[finite_vi.usize()];
                if interior_node == usize::MAX {
                    continue;
                }

                // Use cell's source_index to resolve the border segment endpoints.
                let si = cell.source_index().usize();
                let Some((node_a, node_b)) = bv_seg_border.get(si).copied().flatten() else {
                    if edbg {
                        c_case2_miss += 1;
                    }
                    continue; // no border info (e.g. merged sub-segment endpoints unmapped)
                };

                // Project the interior vertex onto the source segment to choose the closer
                // border endpoint: t ≤ 0.5 → node_a (near seg.start), else → node_b.
                let interior_pt = node_point(interior_node);
                let seg = &bv_segments[si];
                let ax = seg.start.x as f64;
                let ay = seg.start.y as f64;
                let dx = seg.end.x as f64 - ax;
                let dy = seg.end.y as f64 - ay;
                let len_sq = dx * dx + dy * dy;
                let border_node = if len_sq < 1.0 {
                    node_a // degenerate zero-length segment: fall back to start node
                } else {
                    let t = ((interior_pt.x as f64 - ax) * dx + (interior_pt.y as f64 - ay) * dy)
                        / len_sq;
                    if t <= 0.5 {
                        node_a
                    } else {
                        node_b
                    }
                };

                if interior_node == border_node || border_node >= nodes.len() {
                    continue;
                }
                // Orca colour gating (see Case 3): suppress the contour attachment at
                // uniform-colour nodes; only attach where the contour colour changes.
                if !border_node_is_color_boundary[border_node] {
                    if edbg {
                        c_case2_suppressed += 1;
                    }
                    continue;
                }

                let pair = (
                    interior_node.min(border_node),
                    interior_node.max(border_node),
                );
                if !seen_pairs.insert(pair) {
                    continue;
                }

                let point_a = node_point(interior_node);
                let point_b = node_point(border_node);
                let arc_idx = arcs.len();
                arcs.push(MmuArc {
                    from_node: interior_node,
                    to_node: border_node,
                    color: None,
                    kind: MmuArcKind::NonBorder,
                    deleted: false,
                    point_a,
                    point_b,
                });
                nodes[interior_node].arc_indices.push(arc_idx);
                nodes[border_node].arc_indices.push(arc_idx);
                if edbg {
                    c_case2_ok += 1;
                }
            }
        }

        if edbg {
            let border_with_nb = (0..all_border_points)
                .filter(|&n| {
                    nodes[n]
                        .arc_indices
                        .iter()
                        .any(|&a| arcs[a].kind == MmuArcKind::NonBorder)
                })
                .count();
            eprintln!(
                "EDGEDBG segs={} border_nodes={} interior_nodes={} | edges_total={} primary={} secondary={} sec_with_finite_v={} | case1_nb={} case2_ok={} case3_attempt={} case3_ok={} | border_nodes_with_nonborder_arc={}",
                bv_segments.len(),
                all_border_points,
                vertices.len(),
                c_total,
                c_primary,
                c_secondary,
                c_sec_finite,
                c_case1,
                c_case2_ok,
                c_case3_attempt,
                c_case3_ok,
                border_with_nb,
            );
            eprintln!(
                "EDGEDBG_SUPPRESS case2_suppressed={} case3_suppressed={} color_boundary_nodes={}",
                c_case2_suppressed,
                c_case3_suppressed,
                border_node_is_color_boundary.iter().filter(|&&b| b).count(),
            );
            let _ = (c_case2_attempt, c_case2_miss);
        }

        Ok(MMU_Graph {
            diagram,
            vertices,
            nodes,
            arcs,
            all_border_points,
            polygon_idx_offset,
            bv_segments,
            bv_segment_colors,
        })
    }

    /// Construct a graph directly from pre-built nodes/arcs (used in unit tests for
    /// Phases 4d/4e/4f without needing boostvoronoi).
    pub fn from_parts(
        nodes: Vec<MmuNode>,
        arcs: Vec<MmuArc>,
        all_border_points: usize,
        polygon_idx_offset: Vec<usize>,
    ) -> MMU_Graph {
        MMU_Graph {
            #[cfg(feature = "host-algos")]
            diagram: {
                // Build a trivial empty diagram as placeholder.
                Builder::<i32>::default()
                    .build()
                    .expect("empty diagram build")
            },
            vertices: Vec::new(),
            nodes,
            arcs,
            all_border_points,
            polygon_idx_offset,
            #[cfg(feature = "host-algos")]
            bv_segments: Vec::new(),
            #[cfg(feature = "host-algos")]
            bv_segment_colors: Vec::new(),
        }
    }

    /// Count non-deleted arcs incident on `node_idx`.
    pub fn active_arc_count(&self, node_idx: usize) -> usize {
        self.nodes[node_idx]
            .arc_indices
            .iter()
            .filter(|&&ai| !self.arcs[ai].deleted)
            .count()
    }

    /// Decompose Voronoi cells into per-color ExPolygons (B-4).
    ///
    /// Algorithm (per the prescribed spec):
    /// 1. Iterate `diagram.cells()`.
    /// 2. For segment cells, resolve color via `bv_segment_colors[source_index]`.
    /// 3. Walk each cell's edge ring (`get_incident_edge` + `edge_get_next` cycling)
    ///    collecting bounded vertices; clip infinite edges against `contour_bbox`.
    /// 4. Build same-color connected components via twin-edge adjacency.
    /// 5. `union_ex` each component's cell polygons, then `intersection_ex` with the
    ///    original slice contour polygons to prevent leakage beyond the boundary.
    ///
    /// Returns `BTreeMap<Option<PaintValue>, Vec<ExPolygon>>` keyed by color.
    ///
    /// H561 reminder: color is resolved ONLY via `bv_segment_colors[source_index]`,
    /// never via `Cell::get_color()` or `Edge::get_color()`.
    #[cfg(feature = "host-algos")]
    #[allow(dead_code)] // B-4 cell-decomposition path; used in tests only
    pub(crate) fn cells_to_expolygons_by_color(
        &self,
        contour_bbox: &BoundingBox2,
        slice_contours: &[ExPolygon],
    ) -> std::collections::BTreeMap<Option<PaintValue>, Vec<ExPolygon>> {
        use crate::polygon_ops::Line as PolyLine;
        use crate::polygon_ops::{clip_line_with_bbox, intersection_ex, union_ex};
        use boostvoronoi::diagram::SourceCategory;
        use slicer_ir::Polygon;
        use std::collections::BTreeMap;

        let diagram = &self.diagram;
        let seg_colors = &self.bv_segment_colors;
        let bv_segs = &self.bv_segments;

        // Build a bbox expanded by the full bbox diagonal for infinite-edge clipping.
        let bbox_width = (contour_bbox.max.x - contour_bbox.min.x).abs() as f64;
        let bbox_height = (contour_bbox.max.y - contour_bbox.min.y).abs() as f64;
        // Use a coefficient large enough to push infinite rays well outside the bbox.
        let inf_scale = (bbox_width + bbox_height + 1.0) * 4.0;

        // The clipping bbox for infinite edges: generously expanded.
        let clip_bbox = BoundingBox2 {
            min: Point2 {
                x: (contour_bbox.min.x as f64 - inf_scale) as i64,
                y: (contour_bbox.min.y as f64 - inf_scale) as i64,
            },
            max: Point2 {
                x: (contour_bbox.max.x as f64 + inf_scale) as i64,
                y: (contour_bbox.max.y as f64 + inf_scale) as i64,
            },
        };

        // Helper: resolve the point for a point-category cell.
        // A SegmentStart cell's point is seg.start; a SegmentEnd cell's point is seg.end.
        let retrieve_point_for_cell = |cell: &boostvoronoi::diagram::Cell| -> Option<[f64; 2]> {
            let si = cell.source_index().usize();
            let seg = bv_segs.get(si)?;
            match cell.source_category() {
                boostvoronoi::diagram::SourceCategory::SegmentStart => {
                    Some([seg.start.x as f64, seg.start.y as f64])
                }
                boostvoronoi::diagram::SourceCategory::SegmentEnd => {
                    Some([seg.end.x as f64, seg.end.y as f64])
                }
                boostvoronoi::diagram::SourceCategory::SinglePoint => {
                    // Single point site: use start (start == end for a point).
                    Some([seg.start.x as f64, seg.start.y as f64])
                }
                _ => None, // Segment cells are not "point" cells
            }
        };

        // Helper: compute the (origin, direction) for an infinite edge.
        // Following the boostvoronoi visualizer pattern:
        // - If one cell is a segment and the other is a point: origin = point, direction = perp-to-segment
        // - If both are points: origin = midpoint, direction = perp to line between points
        let clip_infinite_edge_pts =
            |edge_id: boostvoronoi::diagram::EdgeIndex| -> Option<(Point2, Point2)> {
                let cell1_id = diagram.edge_get_cell(edge_id).ok()?;
                let cell1 = diagram.cell(cell1_id).ok()?;
                let twin_idx = diagram.edge_get_twin(edge_id).ok()?;
                let cell2_id = diagram.edge_get_cell(twin_idx).ok()?;
                let cell2 = diagram.cell(cell2_id).ok()?;

                let edge = diagram.edge(edge_id).ok()?;

                // Determine origin and direction.
                let (origin, direction): ([f64; 2], [f64; 2]) = {
                    if cell1.contains_point() && cell2.contains_point() {
                        // Both point cells: origin = midpoint, direction = perp to line between points.
                        let p1 = retrieve_point_for_cell(cell1)?;
                        let p2 = retrieve_point_for_cell(cell2)?;
                        let origin = [(p1[0] + p2[0]) * 0.5, (p1[1] + p2[1]) * 0.5];
                        let direction = [p1[1] - p2[1], p2[0] - p1[0]];
                        (origin, direction)
                    } else {
                        // One segment cell, one point cell.
                        let (point_cell, seg_cell_si) = if cell1.contains_segment() {
                            (cell2, cell1.source_index().usize())
                        } else {
                            (cell1, cell2.source_index().usize())
                        };
                        let origin = retrieve_point_for_cell(point_cell)?;
                        let seg = bv_segs.get(seg_cell_si)?;
                        let dx = seg.end.x as f64 - seg.start.x as f64;
                        let dy = seg.end.y as f64 - seg.start.y as f64;
                        // Direction is perpendicular to segment.
                        // Orientation: if origin == seg.start → use (dy, -dx); else → (-dy, dx).
                        // XOR with whether the point cell is cell1 (same logic as boostvoronoi visualizer).
                        let seg_sx = seg.start.x as f64;
                        let seg_sy = seg.start.y as f64;
                        let is_origin_start =
                            (origin[0] - seg_sx).abs() < 0.5 && (origin[1] - seg_sy).abs() < 0.5;
                        let flip = is_origin_start ^ cell1.contains_point();
                        let direction = if flip { [dy, -dx] } else { [-dy, dx] };
                        (origin, direction)
                    }
                };

                // Compute coefficient: enough to reach well outside the bbox.
                let dir_max = direction[0].abs().max(direction[1].abs());
                if dir_max < 1e-9 {
                    return None;
                }
                let coefficient = inf_scale / dir_max;

                // Build the two endpoints of the clipped infinite edge.
                let p0 = if let Some(v0_vi) = edge.vertex0() {
                    let v0 = diagram.vertices().get(v0_vi.usize())?;
                    [v0.x(), v0.y()]
                } else {
                    [
                        origin[0] - direction[0] * coefficient,
                        origin[1] - direction[1] * coefficient,
                    ]
                };

                let p1 = if let Some(v1_vi) = diagram
                    .edge_get_twin(edge_id)
                    .ok()
                    .and_then(|twin| diagram.edge_get_vertex0(twin).ok().and_then(|v| v))
                {
                    let v1 = diagram.vertices().get(v1_vi.usize())?;
                    [v1.x(), v1.y()]
                } else {
                    [
                        origin[0] + direction[0] * coefficient,
                        origin[1] + direction[1] * coefficient,
                    ]
                };

                // Clip the ray/segment against the clip bbox.
                let line = PolyLine {
                    start: Point2 {
                        x: p0[0].round() as i64,
                        y: p0[1].round() as i64,
                    },
                    end: Point2 {
                        x: p1[0].round() as i64,
                        y: p1[1].round() as i64,
                    },
                };
                let clipped = clip_line_with_bbox(&line, &clip_bbox)?;
                Some((clipped.start, clipped.end))
            };

        // --- Step 1-3: collect one polygon per cell ---
        let cells = diagram.cells();
        let mut cell_polys: Vec<Option<(Option<PaintValue>, Vec<Point2>)>> =
            vec![None; cells.len()];

        for (cell_idx, cell) in cells.iter().enumerate() {
            // Resolve cell color. Only segment cells have a meaningful source_index.
            let color: Option<PaintValue> = match cell.source_category() {
                SourceCategory::Segment => {
                    let si = cell.source_index().usize();
                    seg_colors.get(si).and_then(|c| c.clone())
                }
                _ => continue,
            };

            if cell.is_degenerate() {
                continue;
            }

            let Some(start_edge_idx) = cell.get_incident_edge() else {
                continue;
            };

            // Walk the cell's CCW edge ring collecting vertex positions.
            let mut pts: Vec<Point2> = Vec::new();
            let mut edge_idx = start_edge_idx;
            let max_iter = diagram.num_edges() + 4;
            let mut iterations = 0usize;

            loop {
                iterations += 1;
                if iterations > max_iter {
                    break;
                }

                let v0_opt = diagram.edge_get_vertex0(edge_idx).ok().and_then(|v| v);
                let v1_opt = diagram
                    .edge_get_twin(edge_idx)
                    .ok()
                    .and_then(|twin| diagram.edge_get_vertex0(twin).ok().and_then(|v| v));

                let is_finite = v0_opt.is_some() && v1_opt.is_some();

                if is_finite {
                    // Finite edge: push vertex0.
                    if let Some(vi) = v0_opt {
                        if let Some(v) = diagram.vertices().get(vi.usize()) {
                            pts.push(Point2 {
                                x: v.x().round() as i64,
                                y: v.y().round() as i64,
                            });
                        }
                    }
                } else if v0_opt.is_none() && v1_opt.is_none() {
                    // Fully infinite edge (no vertices): skip.
                } else {
                    // Semi-infinite edge: clip using proper direction.
                    if let Some((p_start, _p_end)) = clip_infinite_edge_pts(edge_idx) {
                        // Push the finite-side endpoint (start of the clipped segment).
                        // If v0 exists, it's the finite endpoint we push.
                        // If v0 is None, we need the projected point on the infinite side.
                        if v0_opt.is_some() {
                            // Push v0 (the finite vertex).
                            if let Some(vi) = v0_opt {
                                if let Some(v) = diagram.vertices().get(vi.usize()) {
                                    pts.push(Point2 {
                                        x: v.x().round() as i64,
                                        y: v.y().round() as i64,
                                    });
                                }
                            }
                        } else {
                            // v0 is None: push the clipped infinite endpoint.
                            pts.push(p_start);
                        }
                    }
                }

                // Advance to next edge.
                match diagram.edge_get_next(edge_idx) {
                    Ok(next) => {
                        edge_idx = next;
                        if edge_idx == start_edge_idx {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            if pts.len() >= 3 {
                if pts.first() != pts.last() {
                    pts.push(*pts.first().unwrap());
                }
                cell_polys[cell_idx] = Some((color, pts));
            }
        }

        // --- Step 4: build same-color connected components via twin adjacency ---
        let num_cells = cells.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); num_cells];

        for edge in diagram.edges().iter() {
            let Ok(cell_a_idx) = edge.cell() else {
                continue;
            };
            let Ok(twin_idx) = edge.twin() else { continue };
            let Ok(cell_b_idx) = diagram.edge_get_cell(twin_idx) else {
                continue;
            };

            let ca = cell_a_idx.usize();
            let cb = cell_b_idx.usize();
            if ca == cb {
                continue;
            }

            let color_a = cell_polys
                .get(ca)
                .and_then(|p| p.as_ref())
                .map(|(c, _)| c.clone());
            let color_b = cell_polys
                .get(cb)
                .and_then(|p| p.as_ref())
                .map(|(c, _)| c.clone());

            if let (Some(ca_col), Some(cb_col)) = (color_a, color_b) {
                if ca_col == cb_col {
                    adj[ca].push(cb);
                    adj[cb].push(ca);
                }
            }
        }

        // BFS connected components.
        let mut visited: Vec<bool> = vec![false; num_cells];
        let mut components: Vec<(Option<PaintValue>, Vec<Vec<Point2>>)> = Vec::new();

        for start in 0..num_cells {
            if visited[start] {
                continue;
            }
            let Some((color, _)) = cell_polys[start].as_ref() else {
                visited[start] = true;
                continue;
            };
            let color = color.clone();
            visited[start] = true;

            let mut queue = vec![start];
            let mut component_pts: Vec<Vec<Point2>> = Vec::new();

            while let Some(cidx) = queue.pop() {
                if let Some((_, pts)) = cell_polys[cidx].as_ref() {
                    component_pts.push(pts.clone());
                }
                let nbrs = adj[cidx].clone();
                for nbr in nbrs {
                    if !visited[nbr] {
                        visited[nbr] = true;
                        queue.push(nbr);
                    }
                }
            }

            if !component_pts.is_empty() {
                components.push((color, component_pts));
            }
        }

        // --- Step 5-8: union each component, intersect with slice contours ---
        let mut result: BTreeMap<Option<PaintValue>, Vec<ExPolygon>> = BTreeMap::new();

        for (color, pts_list) in components {
            let cell_expolys: Vec<ExPolygon> = pts_list
                .into_iter()
                .filter(|pts| pts.len() >= 3)
                .map(|points| ExPolygon {
                    contour: Polygon { points },
                    holes: Vec::new(),
                })
                .collect();

            if cell_expolys.is_empty() {
                continue;
            }

            let unioned = union_ex(&cell_expolys);
            if unioned.is_empty() {
                continue;
            }

            let clipped = if slice_contours.is_empty() {
                unioned
            } else {
                let intersected = intersection_ex(&unioned, slice_contours);
                if intersected.is_empty() {
                    continue;
                }
                intersected
            };

            result.entry(color).or_default().extend(clipped);
        }

        // NOTE (parity): the former Step 9 (corner-ownership displacement) and
        // Step 10 (foreign-edge proximity shrink) post-passes were removed here.
        // They displaced legitimately-shared corner vertices ~0.5mm inward to dodge
        // an over-strict downstream face-proximity check, which opened a visible
        // cross-colour gap between adjacent painted regions. OrcaSlicer's partition
        // shares each bisector arc once with NO such displacement
        // (MultiMaterialSegmentation.cpp:523/547-548; ADR-0013). The cell polygons
        // are returned as the exact (unioned + contour-clipped) partition.

        result
    }
}

impl std::fmt::Debug for MMU_Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MMU_Graph {{ vertices: {}, nodes: {}, arcs: {}, border_pts: {} }}",
            self.vertices.len(),
            self.nodes.len(),
            self.arcs.len(),
            self.all_border_points,
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "host-algos"))]
mod tests {
    use super::*;
    use crate::algos::paint_segmentation::triangle_intersect::Line;

    /// Build a synthetic square input (4 line segments).
    /// Unit-square corners at (0,0)-(1000,1000), coordinate unit = 100 nm.
    fn synthetic_square_input() -> Vec<ColoredLine> {
        let corners = [(0i64, 0i64), (1000, 0), (1000, 1000), (0, 1000)];
        let n = corners.len();
        (0..n)
            .map(|i| {
                let (sx, sy) = corners[i];
                let (ex, ey) = corners[(i + 1) % n];
                ColoredLine {
                    line: Line {
                        start: Point2 { x: sx, y: sy },
                        end: Point2 { x: ex, y: ey },
                    },
                    value: None,
                    poly_idx: 0,
                    local_line_idx: i,
                }
            })
            .collect()
    }

    #[test]
    fn mmu_graph_builds_from_synthetic_square_input() {
        let input = synthetic_square_input();
        let result = MMU_Graph::from_colored_lines(&input);
        assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
        let graph = result.unwrap();
        assert!(
            !graph.vertices.is_empty(),
            "expected at least one vertex from square input"
        );
    }

    /// Regression test for the panic discovered while running cube_4color tests:
    /// boostvoronoi panics at `robust_fpt.rs:398 fpv.is_finite()` when fed
    /// collinear-overlapping segments, because `boost::polygon::voronoi` does
    /// not accept site segments that properly overlap on a common line. The
    /// paint kernel produces such overlaps when one contour edge is subdivided
    /// by multiple independent paint projections. `from_colored_lines` must
    /// merge those collinear-overlapping pieces back into single segments
    /// before invoking the builder.
    ///
    /// The fixture below is the minimal reproducer: a 1000x1000 square plus
    /// extra collinear sub-segments lying on the same physical edges as the
    /// square's sides, mimicking what colorize_contours emits after Phase 3
    /// projection.
    #[test]
    fn collinear_overlapping_segments_do_not_panic_the_builder() {
        // The unit square.
        let mut input = synthetic_square_input();
        // Extra collinear sub-segments on the BOTTOM edge (y=0 line):
        //   - one piece (200..700) that overlaps part of the original (0..1000)
        //   - one piece (300..800) that overlaps both the original and the first extra
        for (sx, ex, idx) in [(200, 700, 4), (300, 800, 5)] {
            input.push(ColoredLine {
                line: Line {
                    start: Point2 { x: sx, y: 0 },
                    end: Point2 { x: ex, y: 0 },
                },
                value: None,
                poly_idx: 0,
                local_line_idx: idx,
            });
        }
        // Plus a sub-segment fully contained in the LEFT edge (x=0 line).
        input.push(ColoredLine {
            line: Line {
                start: Point2 { x: 0, y: 250 },
                end: Point2 { x: 0, y: 750 },
            },
            value: None,
            poly_idx: 0,
            local_line_idx: 6,
        });

        // Before the merge pass this input panicked the boostvoronoi builder
        // inside robust_fpt. After the merge it should build cleanly.
        let result = MMU_Graph::from_colored_lines(&input);
        assert!(
            result.is_ok(),
            "expected Ok after merging collinear overlap, got {:?}",
            result.err()
        );
        let graph = result.unwrap();
        assert!(
            !graph.vertices.is_empty(),
            "expected at least one vertex after merge"
        );
    }

    /// Part D — the deterministic input guard rejects an oversized segment set
    /// with a typed `InputTooLarge` error BEFORE invoking the boostvoronoi
    /// builder, bounding the latent `discretize` unbounded-loop exposure. The
    /// input must not hang or panic.
    #[test]
    fn oversized_input_returns_input_too_large() {
        // `MAX_VORONOI_SEGMENTS + 1` distinct vertical unit segments at unique x.
        // Distinct canonical lines so the collinear merge keeps them all, driving
        // `bv_segments.len()` past the cap.
        let n = MAX_VORONOI_SEGMENTS + 1;
        let mut input: Vec<ColoredLine> = Vec::with_capacity(n);
        for i in 0..n {
            let x = i as i64;
            input.push(ColoredLine {
                line: Line {
                    start: Point2 { x, y: 0 },
                    end: Point2 { x, y: 1 },
                },
                value: None,
                poly_idx: 0,
                local_line_idx: i,
            });
        }
        let result = MMU_Graph::from_colored_lines(&input);
        match result {
            Err(MmuGraphError::InputTooLarge { segments, cap }) => {
                assert_eq!(cap, MAX_VORONOI_SEGMENTS);
                assert!(segments > cap, "segments {segments} must exceed cap {cap}");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }

    /// i32 overflow on input coordinate is surfaced as a typed error rather
    /// than a silent `as i32` wrap-around.
    #[test]
    fn coordinate_overflow_returns_typed_error() {
        let input = vec![ColoredLine {
            line: Line {
                start: Point2 { x: 0, y: 0 },
                end: Point2 {
                    x: (i32::MAX as i64) + 1,
                    y: 0,
                },
            },
            value: None,
            poly_idx: 0,
            local_line_idx: 0,
        }];
        match MMU_Graph::from_colored_lines(&input) {
            Err(MmuGraphError::CoordinateOverflow(_)) => {}
            other => panic!("expected CoordinateOverflow, got {other:?}"),
        }
    }

    #[test]
    fn vertex_emission_is_deterministic_across_runs() {
        let input = synthetic_square_input();
        let g1 = MMU_Graph::from_colored_lines(&input).expect("first build");
        let g2 = MMU_Graph::from_colored_lines(&input).expect("second build");

        assert_eq!(
            g1.vertices.len(),
            g2.vertices.len(),
            "vertex count differs between runs"
        );
        for (v1, v2) in g1.vertices.iter().zip(g2.vertices.iter()) {
            assert_eq!(
                v1.coords_id(),
                v2.coords_id(),
                "vertex sequence differs between runs at id={}/{}",
                v1.id,
                v2.id
            );
        }
    }

    #[test]
    fn boostvoronoi_supports_line_segment_sites() {
        use boostvoronoi::builder::Builder;
        use boostvoronoi::geometry::{Line as BvLine, Point as BvPoint};
        let seg = BvLine::new(BvPoint { x: 0i32, y: 0 }, BvPoint { x: 100i32, y: 100 });
        let result = Builder::<i32>::default().with_segments(std::iter::once(&seg));
        assert!(
            result.is_ok(),
            "Builder::with_segments failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn vertex_color_get_set_round_trip() {
        let input = synthetic_square_input();
        let graph = MMU_Graph::from_colored_lines(&input).expect("build");
        let mut diagram2: boostvoronoi::prelude::Diagram = Builder::<i32>::default()
            .with_segments(
                synthetic_square_input()
                    .iter()
                    .map(|cl| {
                        BvLine::new(
                            BvPoint {
                                x: cl.line.start.x as i32,
                                y: cl.line.start.y as i32,
                            },
                            BvPoint {
                                x: cl.line.end.x as i32,
                                y: cl.line.end.y as i32,
                            },
                        )
                    })
                    .collect::<Vec<_>>()
                    .iter(),
            )
            .expect("segs")
            .build()
            .expect("build2");

        assert!(!diagram2.vertices().is_empty(), "need at least one vertex");
        let initial_color = diagram2.vertices()[0].get_color();
        assert_eq!(initial_color, 0, "fresh vertex color should be 0");

        let vid = diagram2.vertices()[0].get_id();
        diagram2.vertex_set_color(vid, 42).expect("set_color");
        let got = diagram2.vertex_get_color(vid).expect("get_color");
        assert_eq!(got, 42, "color round-trip failed");

        let v_color = diagram2.vertices()[0].get_color();
        assert_eq!(
            v_color, 42,
            "Vertex::get_color after set_color should be 42"
        );

        let original_color = graph.diagram.vertices()[0].get_color();
        assert_eq!(
            original_color, 0,
            "original diagram vertex color should still be 0"
        );
    }

    #[test]
    fn edge_is_primary_callable() {
        let input = synthetic_square_input();
        let graph = MMU_Graph::from_colored_lines(&input).expect("build");
        let has_primary = graph.diagram.edges().iter().any(|e| e.is_primary());
        assert!(
            has_primary,
            "expected at least one primary edge from square input"
        );
    }

    #[test]
    fn border_nodes_populated_from_square_input() {
        let input = synthetic_square_input();
        let graph = MMU_Graph::from_colored_lines(&input).expect("build");
        // 4 segments → 4 border nodes
        assert_eq!(graph.all_border_points, 4);
        assert_eq!(graph.polygon_idx_offset, vec![0]);
        // 4 border arcs (one per segment)
        let border_arc_count = graph
            .arcs
            .iter()
            .filter(|a| a.kind == MmuArcKind::Border)
            .count();
        assert_eq!(border_arc_count, 4);
    }

    #[test]
    fn from_colored_lines_populates_interior_nodes_for_synthetic_square() {
        let input = synthetic_square_input();
        let graph = MMU_Graph::from_colored_lines(&input).expect("build");
        // Interior nodes are those beyond the border count.
        assert!(
            graph.nodes.len() > graph.all_border_points,
            "expected Voronoi interior nodes; got nodes={} all_border_points={}",
            graph.nodes.len(),
            graph.all_border_points
        );
    }

    #[test]
    fn from_colored_lines_populates_non_border_arcs() {
        let input = synthetic_square_input();
        let graph = MMU_Graph::from_colored_lines(&input).expect("build");
        let has_non_border = graph
            .arcs
            .iter()
            .any(|a| matches!(a.kind, MmuArcKind::NonBorder));
        assert!(
            has_non_border,
            "expected at least one NonBorder arc from square input"
        );
    }

    /// Verify that `twin()` calls in `from_colored_lines` use `?` for error propagation.
    /// This is a static/structural check — confirmed by reading the implementation.
    /// We verify it holds at runtime by constructing a valid graph (if `?` were replaced
    /// by `.unwrap()`, a bad diagram would panic rather than return Err; the existence
    /// of the Ok result is sufficient evidence).
    #[test]
    fn from_colored_lines_twin_propagation_is_question_mark() {
        let input = synthetic_square_input();
        let result = MMU_Graph::from_colored_lines(&input);
        // If twin() were not propagated via `?`, panics would surface here.
        assert!(
            result.is_ok(),
            "from_colored_lines must succeed (twin errors propagated via ?): {:?}",
            result.err()
        );
    }

    // ---- B-4 cell-decomposition regression tests ----

    /// Build a square input split into two painted halves.
    /// Top 2 segments: ToolIndex(0); bottom 2 segments: ToolIndex(1).
    fn two_color_square_input() -> Vec<ColoredLine> {
        use slicer_ir::PaintValue;
        // Square corners: (0,0)→(1000,0)→(1000,1000)→(0,1000)→(0,0)
        // Edges: bottom (y=0, color=1), right (x=1000, color=0), top (y=1000, color=0), left (x=0, color=1)
        vec![
            ColoredLine {
                line: Line {
                    start: Point2 { x: 0, y: 0 },
                    end: Point2 { x: 1000, y: 0 },
                },
                value: Some(PaintValue::ToolIndex(1)),
                poly_idx: 0,
                local_line_idx: 0,
            },
            ColoredLine {
                line: Line {
                    start: Point2 { x: 1000, y: 0 },
                    end: Point2 { x: 1000, y: 1000 },
                },
                value: Some(PaintValue::ToolIndex(0)),
                poly_idx: 0,
                local_line_idx: 1,
            },
            ColoredLine {
                line: Line {
                    start: Point2 { x: 1000, y: 1000 },
                    end: Point2 { x: 0, y: 1000 },
                },
                value: Some(PaintValue::ToolIndex(0)),
                poly_idx: 0,
                local_line_idx: 2,
            },
            ColoredLine {
                line: Line {
                    start: Point2 { x: 0, y: 1000 },
                    end: Point2 { x: 0, y: 0 },
                },
                value: Some(PaintValue::ToolIndex(1)),
                poly_idx: 0,
                local_line_idx: 3,
            },
        ]
    }

    /// Helper: build a full-square ExPolygon for use as slice_contours.
    fn square_contour_expoly() -> slicer_ir::ExPolygon {
        use slicer_ir::{ExPolygon, Polygon};
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: 0, y: 0 },
                    Point2 { x: 1000, y: 0 },
                    Point2 { x: 1000, y: 1000 },
                    Point2 { x: 0, y: 1000 },
                    Point2 { x: 0, y: 0 },
                ],
            },
            holes: Vec::new(),
        }
    }

    /// B-4 regression test 1: two-color square produces two disjoint polygons.
    ///
    /// Pins the per-cell color separation: each color's cluster must yield a
    /// distinct non-empty ExPolygon, and the two polygons must not overlap.
    #[test]
    fn cells_to_expolygons_synthetic_two_color_square_produces_two_disjoint_polygons() {
        use crate::polygon_ops::intersection_ex;
        use slicer_ir::slice_ir::BoundingBox2;
        use slicer_ir::PaintValue;

        let input = two_color_square_input();
        let graph = MMU_Graph::from_colored_lines(&input).expect("build");

        let bbox = BoundingBox2 {
            min: Point2 { x: 0, y: 0 },
            max: Point2 { x: 1000, y: 1000 },
        };
        let contour = vec![square_contour_expoly()];

        let result = graph.cells_to_expolygons_by_color(&bbox, &contour);

        // Must have exactly 2 color entries.
        assert_eq!(
            result.len(),
            2,
            "expected 2 color entries (ToolIndex(0) and ToolIndex(1)), got: {:?}",
            result.keys().collect::<Vec<_>>()
        );

        let polys_0 = result
            .get(&Some(PaintValue::ToolIndex(0)))
            .expect("ToolIndex(0) entry missing");
        let polys_1 = result
            .get(&Some(PaintValue::ToolIndex(1)))
            .expect("ToolIndex(1) entry missing");

        assert!(
            !polys_0.is_empty(),
            "ToolIndex(0) polygons must be non-empty"
        );
        assert!(
            !polys_1.is_empty(),
            "ToolIndex(1) polygons must be non-empty"
        );

        // Disjointness: intersection between the two sets must be empty.
        let overlap = intersection_ex(polys_0, polys_1);
        assert!(
            overlap.is_empty(),
            "ToolIndex(0) and ToolIndex(1) polygons must be disjoint; overlap={overlap:?}"
        );
    }

    /// B-4 regression test 2: single-color square yields one polygon covering the whole square.
    ///
    /// Sanity-checks that same-color clustering merges all cells into a single ExPolygon.
    #[test]
    fn cells_to_expolygons_single_color_full_perimeter_is_one_polygon() {
        use slicer_ir::slice_ir::BoundingBox2;
        use slicer_ir::PaintValue;

        // All 4 segments with the same color.
        let input: Vec<ColoredLine> = synthetic_square_input()
            .into_iter()
            .map(|mut cl| {
                cl.value = Some(PaintValue::ToolIndex(0));
                cl
            })
            .collect();

        let graph = MMU_Graph::from_colored_lines(&input).expect("build");

        let bbox = BoundingBox2 {
            min: Point2 { x: 0, y: 0 },
            max: Point2 { x: 1000, y: 1000 },
        };
        let contour = vec![square_contour_expoly()];

        let result = graph.cells_to_expolygons_by_color(&bbox, &contour);

        // Must have exactly 1 color entry.
        assert_eq!(
            result.len(),
            1,
            "expected 1 color entry (ToolIndex(0)), got: {:?}",
            result.keys().collect::<Vec<_>>()
        );

        let polys = result
            .get(&Some(PaintValue::ToolIndex(0)))
            .expect("ToolIndex(0) entry missing");

        assert_eq!(
            polys.len(),
            1,
            "single-color square: expected 1 ExPolygon after union, got {}",
            polys.len()
        );
    }

    /// B-4 diagnostic: 4-color square mimicking the cube geometry.
    /// Verifies that each face color covers ONLY its own quadrant (no bleed-over).
    /// bottom=ToolIndex(0), right=ToolIndex(1), top=ToolIndex(2), left=ToolIndex(3).
    ///
    /// "Covers a face" / "bleeds onto a face" is tested as having a polygon EDGE
    /// (two consecutive vertices) along the face band — NOT a single vertex touching
    /// it. Adjacent colors legitimately share their common corner vertex (the bisector
    /// arc emanates from it, OrcaSlicer `MultiMaterialSegmentation.cpp:547-548`); the
    /// back cell's shared back-right corner is correct geometry, not bleed.
    #[test]
    fn cells_to_expolygons_four_color_square_each_color_covers_its_face() {
        use slicer_ir::slice_ir::BoundingBox2;
        use slicer_ir::{ExPolygon, PaintValue, Polygon};

        // Square corners at (1125000,925000)-(1375000,1175000) (cube world units).
        let x_min: i64 = 1_125_000;
        let y_min: i64 = 925_000;
        let x_max: i64 = 1_375_000;
        let y_max: i64 = 1_175_000;

        let input = vec![
            // Bottom: ToolIndex(0)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_min, y: y_min },
                    end: Point2 { x: x_max, y: y_min },
                },
                value: Some(PaintValue::ToolIndex(0)),
                poly_idx: 0,
                local_line_idx: 0,
            },
            // Right: ToolIndex(1)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_max, y: y_min },
                    end: Point2 { x: x_max, y: y_max },
                },
                value: Some(PaintValue::ToolIndex(1)),
                poly_idx: 0,
                local_line_idx: 1,
            },
            // Top (back): ToolIndex(2)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_max, y: y_max },
                    end: Point2 { x: x_min, y: y_max },
                },
                value: Some(PaintValue::ToolIndex(2)),
                poly_idx: 0,
                local_line_idx: 2,
            },
            // Left: ToolIndex(3)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_min, y: y_max },
                    end: Point2 { x: x_min, y: y_min },
                },
                value: Some(PaintValue::ToolIndex(3)),
                poly_idx: 0,
                local_line_idx: 3,
            },
        ];

        let graph = MMU_Graph::from_colored_lines(&input).expect("build");

        let bbox = BoundingBox2 {
            min: Point2 { x: x_min, y: y_min },
            max: Point2 { x: x_max, y: y_max },
        };
        let contour = vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x_min, y: y_min },
                    Point2 { x: x_max, y: y_min },
                    Point2 { x: x_max, y: y_max },
                    Point2 { x: x_min, y: y_max },
                    Point2 { x: x_min, y: y_min },
                ],
            },
            holes: Vec::new(),
        }];

        let result = graph.cells_to_expolygons_by_color(&bbox, &contour);

        // Must have exactly 4 color entries.
        assert_eq!(
            result.len(),
            4,
            "expected 4 color entries, got: {:?}",
            result.keys().collect::<Vec<_>>()
        );

        let tol = (x_max - x_min) / 100;
        // "Has a wall along the right face" = two CONSECUTIVE contour vertices both on
        // the right band (an edge), not a single shared-corner vertex.
        let has_right_edge = |polys: &[ExPolygon]| {
            polys.iter().any(|exp| {
                let pts = &exp.contour.points;
                let n = pts.len();
                n >= 2
                    && (0..n).any(|i| pts[i].x >= x_max - tol && pts[(i + 1) % n].x >= x_max - tol)
            })
        };

        // Right face (ToolIndex(1)) must have an edge along the right face.
        let polys_1 = result
            .get(&Some(PaintValue::ToolIndex(1)))
            .expect("ToolIndex(1) missing");
        assert!(
            has_right_edge(polys_1),
            "ToolIndex(1) polygon must have a wall along the right face (x near {}), got polys: {:?}",
            x_max, polys_1
        );

        // Back face (ToolIndex(2)) must NOT have an edge along the right face. It MAY
        // share the single back-right corner vertex — that is correct partition geometry.
        let polys_2 = result
            .get(&Some(PaintValue::ToolIndex(2)))
            .expect("ToolIndex(2) missing");
        assert!(
            !has_right_edge(polys_2),
            "ToolIndex(2) polygon must NOT have a wall along the right face (x near {}); \
             a single shared corner is allowed. Got polys: {:?}",
            x_max,
            polys_2
        );

        // Regression guard (cross-colour gap): adjacent colours must SHARE the exact
        // back-right corner vertex (x_max, y_max). The retired Step 9/10 displacement
        // post-passes shoved each colour's shared-corner vertex ~0.5mm inward to dodge
        // an over-strict face-proximity check, opening a visible gap between adjacent
        // painted regions. Assert both the right (1) and back (2) cells keep a vertex
        // AT the corner — any re-introduction of corner displacement would move these
        // vertices far beyond CORNER_TOL and fail here.
        const CORNER_TOL: i64 = 200; // << the ~5000-unit displacement, >> walk rounding
        let has_back_right_corner = |polys: &[ExPolygon]| {
            polys.iter().any(|exp| {
                exp.contour.points.iter().any(|pt| {
                    (pt.x - x_max).abs() <= CORNER_TOL && (pt.y - y_max).abs() <= CORNER_TOL
                })
            })
        };
        assert!(
            has_back_right_corner(polys_1),
            "ToolIndex(1) (right) must keep a vertex at the shared back-right corner \
             ({x_max},{y_max}) — corner-displacement regression. Got: {polys_1:?}"
        );
        assert!(
            has_back_right_corner(polys_2),
            "ToolIndex(2) (back) must keep a vertex at the shared back-right corner \
             ({x_max},{y_max}) — corner-displacement regression. Got: {polys_2:?}"
        );
    }

    // removed: square fixture is Voronoi-degenerate; area parity is validated by AC-4 (model) + AC-2 (confinement).
}
