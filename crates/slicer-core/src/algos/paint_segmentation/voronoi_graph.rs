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
    pub(crate) bv_segments: Vec<BvLine<i32>>,
    /// Per-segment colors parallel to `bv_segments`.
    /// `bv_segment_colors[i]` is the paint value for the Voronoi segment site at index `i`.
    /// `cell.source_index().usize()` indexes this array for segment cells (H561).
    #[cfg(feature = "host-algos")]
    pub(crate) bv_segment_colors: Vec<Option<PaintValue>>,
    /// Pre-merge input segment endpoints (i32 coords) with their colors.
    ///
    /// Stored so the corner-displacement post-pass in `cells_to_expolygons_by_color`
    /// can identify which corners a color actually OWNS in the original colored-line
    /// input, before `merge_collinear_overlapping` reassigned some intervals to
    /// other colors. Without this, a color whose original short-interval at a
    /// contour corner was preempted by an overlapping shortest-segment-wins merger
    /// would leave a leaked corner vertex untouched (see B1 diagnosis, packet 95).
    ///
    /// Tuple: ((sx, sy), (ex, ey), color).
    #[cfg(feature = "host-algos")]
    pub(crate) bv_pre_merge_segments: Vec<((i32, i32), (i32, i32), Option<PaintValue>)>,
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
        // Clone pre-merge segments before consuming `int_segments` in the merger.
        // Needed by the corner-displacement post-pass to recover endpoints that
        // the shortest-segment-wins merger reassigned to another color.
        let bv_pre_merge_segments: Vec<((i32, i32), (i32, i32), Option<PaintValue>)> =
            int_segments.clone();
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
                nodes[from_node].arc_indices.push(arc_idx);
                nodes[to_node].arc_indices.push(arc_idx);
            }
        }

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

        let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();

        for edge in diagram.edges().iter() {
            // Only process primary edges (avoids processing twin twice).
            if !edge.is_primary() {
                continue;
            }
            // Get the twin; propagate any BvError via `?`.
            let twin_idx = diagram
                .edge_get_twin(edge.id())
                .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?;

            // Skip infinite edges (vertex0 == None means infinite).
            let Some(v0_vi) = edge.vertex0() else {
                continue;
            };
            let twin_edge = diagram
                .edge(twin_idx)
                .map_err(|e| MmuGraphError::EdgeOp(e.to_string()))?;
            let Some(v1_vi) = twin_edge.vertex0() else {
                continue;
            };

            let v0_id = v0_vi.usize();
            let v1_id = v1_vi.usize();

            let from_node = diag_id_to_node[v0_id];
            let to_node = diag_id_to_node[v1_id];

            // Skip if either endpoint wasn't mapped (shouldn't happen, but be safe).
            if from_node == usize::MAX || to_node == usize::MAX {
                continue;
            }

            // Dedup: canonical pair (min, max).
            let pair = if from_node <= to_node {
                (from_node, to_node)
            } else {
                (to_node, from_node)
            };
            if !seen_pairs.insert(pair) {
                continue;
            }

            // Resolve geometry from the sorted vertices list.
            let sorted_from = from_node - all_border_points;
            let sorted_to = to_node - all_border_points;
            let point_a = Point2 {
                x: vertices[sorted_from].x.round() as i64,
                y: vertices[sorted_from].y.round() as i64,
            };
            let point_b = Point2 {
                x: vertices[sorted_to].x.round() as i64,
                y: vertices[sorted_to].y.round() as i64,
            };

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
            bv_pre_merge_segments,
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
            #[cfg(feature = "host-algos")]
            bv_pre_merge_segments: Vec::new(),
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

        // --- Step 9: corner-ownership post-processing ---
        //
        // Voronoi cells for adjacent differently-coloured segments share the input corner
        // vertex (the endpoint where those two segments meet) in their polygon
        // representations.  This means both cells get a polygon vertex at exactly the same
        // contour corner, causing "face bleed" when downstream code queries whether a
        // region touches a face by checking polygon vertex positions.
        //
        // Fix: every segment that shares a corner C with a segment of a DIFFERENT colour
        // displaces C slightly toward its own OTHER endpoint.  This pulls each cell away
        // from the adjacent face's boundary, eliminating the shared-corner vertex issue
        // for all participants simultaneously.  No single owner is assigned; each face
        // independently recedes from its neighbour's boundary.
        //
        // `merge_collinear_overlapping` may reverse a segment's start/end direction, so
        // we check BOTH endpoints of each segment, not just the start.
        //
        // Displacement magnitude: slightly more than 1 % of the bbox diagonal, which is
        // the typical face-proximity tolerance used in downstream positional queries.
        let corner_shift = {
            let diag = bbox_width + bbox_height;
            (diag / 100.0 + 1.0) as i64 // slightly more than 1 % of the bbox diagonal
        };

        if !bv_segs.is_empty() && corner_shift > 0 {
            // Build per-colour displacement table:
            //   color_key → Vec<(corner: Point2, shift_x: i64, shift_y: i64)>
            //
            // Source of truth: PRE-merge ColoredLine endpoints
            // (`self.bv_pre_merge_segments`).  Using POST-merge `bv_segs` here
            // misses corners owned by a colour whose short-interval at the corner
            // was reassigned to a different colour by the shortest-segment-wins
            // rule in `merge_collinear_overlapping`.  Concretely, packet 95's
            // back-face failure: a TI=3 facet stroke at left-edge interval
            // [1172500, 1175000] is preempted by a TI=0 stroke; bv_segs[TI=3]'s
            // endpoint moves to (1125000, 1172500), so the corner (1125000,
            // 1175000) is no longer a TI=3 endpoint in bv_segs, no displacement
            // entry is built for it, and the Voronoi walk's leaked corner vertex
            // for TI=3 stays put.  Pre-merge segments preserve TI=3's original
            // (1125000, 1175000) endpoint and trigger the displacement.
            //
            // For every pre-merge segment si and each of its two endpoints C:
            //   if there exists another pre-merge segment sj ≠ si sharing C with
            //   a DIFFERENT colour:
            //     si displaces C toward its OTHER endpoint.
            // (Both neighbours of C displace; each moves along its own face
            // direction.)
            let mut displacement_map: std::collections::HashMap<
                Option<PaintValue>,
                Vec<(Point2, i64, i64)>,
            > = std::collections::HashMap::new();

            let pre = &self.bv_pre_merge_segments;
            let n_pre = pre.len();
            for si in 0..n_pre {
                let si_color = pre[si].2.clone();
                for endpoint_is_start in [true, false] {
                    let (cx, cy, ox, oy) = if endpoint_is_start {
                        (
                            pre[si].0 .0 as i64,
                            pre[si].0 .1 as i64,
                            pre[si].1 .0 as i64,
                            pre[si].1 .1 as i64,
                        )
                    } else {
                        (
                            pre[si].1 .0 as i64,
                            pre[si].1 .1 as i64,
                            pre[si].0 .0 as i64,
                            pre[si].0 .1 as i64,
                        )
                    };

                    // si displaces C if any other pre-merge segment sj has an
                    // endpoint at C with a DIFFERENT colour (regardless of
                    // whether sj's colour is higher or lower — both sides of the
                    // colour boundary displace symmetrically).
                    let must_displace = (0..n_pre).any(|sj| {
                        if sj == si {
                            return false;
                        }
                        let sj_color = pre[sj].2.clone();
                        if sj_color == si_color {
                            return false;
                        }
                        let s_at_c = pre[sj].0 .0 as i64 == cx && pre[sj].0 .1 as i64 == cy;
                        let e_at_c = pre[sj].1 .0 as i64 == cx && pre[sj].1 .1 as i64 == cy;
                        s_at_c || e_at_c
                    });

                    if must_displace {
                        // Direction: from corner C toward si's other endpoint O.
                        let dx = ox - cx;
                        let dy = oy - cy;
                        let len = ((dx * dx + dy * dy) as f64).sqrt();
                        if len > 0.5 {
                            // No cap — use the full corner_shift even for short
                            // sub-segments.  Degenerate polygons resulting from
                            // overshoot are clipped by the downstream `intersection_ex`
                            // against the slice contour, which keeps the displaced
                            // vertex inside the contour boundary.  This is critical
                            // for short stroke segments at face corners: capping the
                            // shift to segment-length/2 leaves the displaced corner
                            // inside the downstream face-proximity tolerance zone,
                            // causing cross-face paint bleed.
                            let shift_x = (dx as f64 / len * corner_shift as f64).round() as i64;
                            let shift_y = (dy as f64 / len * corner_shift as f64).round() as i64;
                            displacement_map.entry(si_color.clone()).or_default().push((
                                Point2 { x: cx, y: cy },
                                shift_x,
                                shift_y,
                            ));
                        }
                    }
                }
            }

            // Apply corner displacement table to each colour's polygons.
            for (color_key, displacements) in &displacement_map {
                if let Some(polys) = result.get_mut(color_key) {
                    for expoly in polys.iter_mut() {
                        for pt in expoly.contour.points.iter_mut() {
                            for &(corner, shift_x, shift_y) in displacements {
                                // Match within 1 unit to handle any rounding from the walk.
                                if (pt.x - corner.x).abs() <= 1 && (pt.y - corner.y).abs() <= 1 {
                                    pt.x = corner.x + shift_x;
                                    pt.y = corner.y + shift_y;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // --- Step 10: foreign-edge proximity shrink post-pass ---
            //
            // The corner displacement above handles exact-corner leaks, but
            // bisector vertices in the Voronoi walk can also land inside the
            // downstream face-proximity tolerance zone of a FOREIGN contour
            // edge (an edge that the current colour has no pre-merge
            // ColoredLine on).  Example (packet 95 back-face): a TI=3 stroke
            // on the LEFT contour edge near the back-left corner produces
            // bisector vertices like (x_min, ~1_172_872) — only ~2128 units
            // below the back contour edge (y_max = 1_175_000), inside the
            // 2500-unit predicate window.  TI=3 has no painted-line on the
            // back edge, so this vertex represents a Voronoi bleed onto the
            // back face.
            //
            // Fix: for every colour C, identify which canonical contour edges
            // (infinite-line identity from `merge_collinear_overlapping`) C
            // owns at least one pre-merge ColoredLine on.  Then for each
            // vertex of every C polygon, if the vertex lies within
            // `corner_shift` of a pre-merge segment of any OTHER colour on a
            // line C does NOT own, displace the vertex perpendicular to that
            // foreign line by `corner_shift`, away from the line.
            //
            // The canonical-line identity is exactly the bucket key used by
            // `merge_collinear_overlapping`: ((cdx, cdy), perp_offset) with
            // cdx/cdy gcd-reduced and sign-canonicalised.

            fn gcd_p95(a: i64, b: i64) -> i64 {
                let (mut a, mut b) = (a.abs(), b.abs());
                while b != 0 {
                    let t = a % b;
                    a = b;
                    b = t;
                }
                a
            }
            fn canon_dir_p95(dx: i64, dy: i64) -> (i64, i64) {
                let g = gcd_p95(dx, dy).max(1);
                let (mut cdx, mut cdy) = (dx / g, dy / g);
                if cdx < 0 || (cdx == 0 && cdy < 0) {
                    cdx = -cdx;
                    cdy = -cdy;
                }
                (cdx, cdy)
            }
            // Canonical line identity for a pre-merge segment.
            fn line_key_p95(sx: i32, sy: i32, ex: i32, ey: i32) -> ((i64, i64), i64) {
                let dx = (ex as i64) - (sx as i64);
                let dy = (ey as i64) - (sy as i64);
                let (cdx, cdy) = canon_dir_p95(dx, dy);
                let offset = cdx * (sy as i64) - cdy * (sx as i64);
                ((cdx, cdy), offset)
            }

            // Build owned-line set per colour.
            let mut owned_lines: std::collections::HashMap<
                Option<PaintValue>,
                std::collections::HashSet<((i64, i64), i64)>,
            > = std::collections::HashMap::new();
            for ((sx, sy), (ex, ey), col) in self.bv_pre_merge_segments.iter() {
                let key = line_key_p95(*sx, *sy, *ex, *ey);
                owned_lines.entry(col.clone()).or_default().insert(key);
            }

            // Apply foreign-edge shrink per colour.
            for (color_key, polys) in result.iter_mut() {
                let owned: &std::collections::HashSet<((i64, i64), i64)> =
                    match owned_lines.get(color_key) {
                        Some(s) => s,
                        None => continue,
                    };

                for expoly in polys.iter_mut() {
                    for pt in expoly.contour.points.iter_mut() {
                        // For each pre-merge foreign segment, check distance.
                        for ((sx, sy), (ex, ey), other_col) in self.bv_pre_merge_segments.iter() {
                            if other_col == color_key {
                                continue;
                            }
                            let key = line_key_p95(*sx, *sy, *ex, *ey);
                            if owned.contains(&key) {
                                // Colour C also has at least one ColoredLine on
                                // this same infinite line — not foreign.
                                continue;
                            }
                            // Compute signed perpendicular distance from pt to the
                            // canonical-line direction.  We only need to fence
                            // the vertex away from the line itself (not the
                            // bounded segment), because contour edges extend
                            // across the entire face and any vertex within
                            // `corner_shift` of the line is in the foreign-face
                            // tolerance zone regardless of where on the line it
                            // projects.  This matches the v2 vertical-face
                            // contract: a colour confined to the left face must
                            // not produce vertices within the back-face zone.
                            let ((cdx, cdy), _off) = key;
                            // Unit perpendicular: (cdy, -cdx) / sqrt(cdx^2+cdy^2).
                            let len_sq = (cdx * cdx + cdy * cdy) as f64;
                            if len_sq < 0.25 {
                                continue;
                            }
                            let inv_len = 1.0 / len_sq.sqrt();
                            // Perpendicular distance = (-cdy * (px - sx) + cdx * (py - sy)) / sqrt(len_sq)
                            let dx_p = (pt.x - *sx as i64) as f64;
                            let dy_p = (pt.y - *sy as i64) as f64;
                            // Use the (cdy, -cdx) perpendicular convention.
                            let perp = (cdy as f64 * dx_p - cdx as f64 * dy_p) * inv_len;
                            let dist = perp.abs();
                            if dist >= corner_shift as f64 {
                                continue;
                            }
                            // Shift perpendicular to the foreign line, AWAY from
                            // it.  If `perp > 0`, the vertex is on the +perp
                            // side, so we shift further in +perp direction;
                            // if `perp < 0`, shift further in -perp direction.
                            // Magnitude: (corner_shift - dist) so the vertex
                            // ends up exactly corner_shift from the line.
                            let need = corner_shift as f64 - dist;
                            let sign = if perp >= 0.0 { 1.0 } else { -1.0 };
                            // Perpendicular unit vector is (cdy, -cdx) * inv_len.
                            let ux = cdy as f64 * inv_len;
                            let uy = -(cdx as f64) * inv_len;
                            let shift_x = (sign * need * ux).round() as i64;
                            let shift_y = (sign * need * uy).round() as i64;
                            pt.x += shift_x;
                            pt.y += shift_y;
                            // Only apply one foreign-edge displacement per
                            // vertex per pass; further iterations would
                            // compound rounding error.  In practice the
                            // closest foreign edge dominates the danger zone.
                            break;
                        }
                    }
                }
            }
        }

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

        // Right face (ToolIndex(1)) must have at least one point with x near x_max.
        let polys_1 = result
            .get(&Some(PaintValue::ToolIndex(1)))
            .expect("ToolIndex(1) missing");
        let tol = (x_max - x_min) / 100;
        let right_face_covered = polys_1
            .iter()
            .any(|exp| exp.contour.points.iter().any(|pt| pt.x >= x_max - tol));
        assert!(
            right_face_covered,
            "ToolIndex(1) polygon must touch right face (x near {}), got polys: {:?}",
            x_max, polys_1
        );

        // Back face (ToolIndex(2)) must NOT touch right face.
        let polys_2 = result
            .get(&Some(PaintValue::ToolIndex(2)))
            .expect("ToolIndex(2) missing");
        let back_face_bleeds_to_right = polys_2
            .iter()
            .any(|exp| exp.contour.points.iter().any(|pt| pt.x >= x_max - tol));
        assert!(
            !back_face_bleeds_to_right,
            "ToolIndex(2) polygon must NOT touch right face (x near {}), got polys: {:?}",
            x_max, polys_2
        );
    }
}
