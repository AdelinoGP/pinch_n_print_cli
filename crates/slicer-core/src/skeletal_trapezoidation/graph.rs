// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/SkeletalTrapezoidation.cpp
// (`constructFromPolygons`, `transferEdge`, `discretize`),
// src/libslic3r/Arachne/utils/SkeletalTrapezoidationGraph.cpp (`makeRib`),
// src/libslic3r/Geometry/VoronoiUtils.cpp (`compute_segment_cell_range`,
// `compute_point_cell_range`, `get_source_point`, `get_source_segment`),
// src/libslic3r/Geometry.cpp (`is_point_inside_polygon_corner`).
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! `SkeletalTrapezoidationGraph`: the Orca-shaped half-edge graph, built by a
//! faithful per-cell port of Arachne's `constructFromPolygons` /
//! `transferEdge` / `makeRib` over [`crate::voronoi::voronoi_from_segments`]'s
//! boostvoronoi-shaped [`crate::voronoi::HalfEdgeGraph`].
//!
//! # Topology (packet 113c — faithful graph construction)
//!
//! Unlike the earlier port (packets 112 / 113b), `next`/`prev`/`twin` are **no
//! longer a 1:1 copy** of boostvoronoi's raw per-cell DCEL. That verbatim copy
//! encoded "walk around one Voronoi cell's own boundary," which is not the same
//! as "continue along the medial-axis spine," and broke `getNextUnconnected`-style
//! domain traversal at every junction (`D-112-MMU-TOPOLOGY`,
//! `D-113B-CONNECTJUNCTIONS`). Instead, [`SkeletalTrapezoidationGraph::from_polygons`]
//! now:
//!
//! - iterates the raw Voronoi **cells** (both point-cells at polygon vertices
//!   and segment-cells at polygon edges), narrowing each to the medial-axis
//!   sub-arc via `compute_segment_cell_range` / `compute_point_cell_range`
//!   (vertex-coordinate boundary matching plus a point-cell polygon-corner
//!   membership gate);
//! - builds a **fresh** local half-edge chain per cell (`transferEdge`), one
//!   graph edge per (discretized) raw Voronoi sub-segment, chained via
//!   `prev`/`next`;
//! - inserts a rib pair (`EdgeType::EXTRA_VD`) **after every transferred edge
//!   except each cell's final closing edge** (`makeRib`), reassigning the
//!   caller's cursor to the rib's `back_edge` so the chain **interleaves**
//!   (`spine → forth_rib` dead-end; `back_rib → next spine → forth_rib` …);
//! - cross-links adjacent cells by mirror-constructing the matching chain when
//!   a raw Voronoi edge's twin was already transferred, stepping
//!   `twin.prev.twin.prev` and setting `.twin` bidirectionally on the shared
//!   spine edges.
//!
//! This interleaving is exactly what makes `getNextUnconnected`
//! (`walk .next until a dead end, then take that edge's .twin`) correctly walk
//! through junction/branch vertices of any degree.
//!
//! # Half-edge `to` vertex
//!
//! [`STHalfEdge`] stores only `start_vertex` (the "from" vertex); the "to"
//! vertex is recovered from the edge's twin's `start_vertex` (boostvoronoi's
//! `edge.vertex1() == edge.twin().vertex0()` convention). The construction sets
//! `.twin` bidirectionally on every spine and rib edge, so this holds for every
//! edge in a well-formed closed-polygon graph.
//!
//! # Sentinel / storage conventions
//!
//! Topology fields use [`crate::voronoi::NO_INDEX`] (`usize::MAX`) for "no
//! value", and vertices/edges live in `Vec`s indexed by `usize` — Vec+index is
//! actually safer than OrcaSlicer's raw-pointer/`std::list` scheme (index
//! stability across `push` is guaranteed). Iteration is over stable index
//! order with no `HashMap`, so construction is deterministic (AC-N3).
//!
//! - **`distance_to_boundary`** (per vertex): boundary (rib-foot) nodes are the
//!   sentinel `0.0`; spine nodes are the nearest point-to-segment clearance,
//!   which `makeRib` additionally (re)asserts via its perpendicular-foot
//!   projection onto the source segment's infinite line.
//! - **`r_min`/`r_max`** (per edge): `(min, max)` of the edge's two endpoints'
//!   `distance_to_boundary`, always finite and `r_min <= r_max`.
//! - **`central`** defaults `false` — the centrality pass fills it in.

use std::fmt;

use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

use super::discretize::discretize_parabolic_edge;
use super::rib::{EdgeType, RibData};
use crate::voronoi::{self, Segment, SourceCategory, Vertex, VoronoiError, NO_INDEX};

/// Numerical tolerance (in `f64` scaled-integer units) below which a length or
/// direction magnitude is treated as zero.
const EPS: f64 = 1e-6;

/// Spacing bound (in scaled-integer units) between consecutive sample points
/// when discretizing a curved (parabolic) Voronoi edge, mirroring OrcaSlicer's
/// `discretization_step_size`. 0.2 mm in this crate's unit space
/// (`1 unit = 100 nm`, so `0.2 mm = 2000 units`). Only affects the geometric
/// fidelity of curved edges (more sub-segments = closer chord fit); topology,
/// closure, and determinism are independent of its exact value.
const DISCRETIZATION_STEP_UNITS: f64 = 0.2 * UNITS_PER_MM;

/// A `SkeletalTrapezoidationGraph` vertex: a graph node annotated with its
/// distance to the nearest input polygon boundary edge.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidationJoint`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct STVertex {
    /// This vertex's position, in the scaled-integer unit space of the input
    /// polygons (`f64` coordinates).
    pub position: Vertex,
    /// Nearest distance from `position` to any input polygon boundary edge.
    /// Rib-foot (boundary) nodes carry the sentinel `0.0`.
    pub distance_to_boundary: f64,
    /// The number of extrusion beads (walls) this vertex should carry, as
    /// decided by the `assign_bead_counts` pass. `None` until that pass runs.
    pub bead_count: Option<u32>,
    /// Fractional position within a bead-count transition region, in `[0, 1]`.
    /// Set by the `apply_transitions` pass.
    pub transition_ratio: f64,
}

/// A `SkeletalTrapezoidationGraph` half-edge.
///
/// Mirrors OrcaSlicer's `SkeletalTrapezoidationEdge`. Topology fields
/// (`start_vertex`/`twin`/`next`/`prev`) encode the freshly-constructed
/// per-cell-chain-with-interleaved-ribs topology (packet 113c), **not** a 1:1
/// copy of the raw boostvoronoi DCEL. Same sentinel convention
/// ([`crate::voronoi::NO_INDEX`] for "no value").
#[derive(Debug, Clone, PartialEq)]
pub struct STHalfEdge {
    /// Index into [`SkeletalTrapezoidationGraph::vertices`] for this
    /// half-edge's start ("from") point, or [`crate::voronoi::NO_INDEX`].
    pub start_vertex: usize,
    /// Index into [`SkeletalTrapezoidationGraph::edges`] for this half-edge's
    /// twin, or [`crate::voronoi::NO_INDEX`]. The half-edge's "to" vertex is
    /// `edges[twin].start_vertex`.
    pub twin: usize,
    /// Index into [`SkeletalTrapezoidationGraph::edges`] for the next
    /// half-edge along the spine/rib chain, or [`crate::voronoi::NO_INDEX`]
    /// (a dead end — e.g. every rib `forth_edge`).
    pub next: usize,
    /// Index into [`SkeletalTrapezoidationGraph::edges`] for the previous
    /// half-edge along the chain, or [`crate::voronoi::NO_INDEX`] (a
    /// domain/quad start — e.g. every rib `back_edge`).
    pub prev: usize,
    /// Minimum of this edge's two endpoints' `distance_to_boundary`.
    pub r_min: f64,
    /// Maximum of this edge's two endpoints' `distance_to_boundary`.
    pub r_max: f64,
    /// Whether this edge is on the "central" spine of the skeleton. Filled in
    /// by the centrality pass; `false` here.
    pub central: bool,
    /// `true` for a curved (parabolic point-to-segment bisector) spine edge;
    /// `false` for straight spine edges and all rib edges.
    pub is_curved: bool,
    /// Vestigial since packet 113c (ribs are now the ordinary `twin` pair, so
    /// this is never populated by construction). Retained as a field for
    /// source compatibility with hand-built test fixtures; always `None`.
    pub rib_twin: Option<usize>,
    /// Vestigial since packet 113c (the separate quad-cell side table was
    /// removed; ribs live directly in `edges`). Retained as a field for source
    /// compatibility; always `None`.
    pub quad_cell: Option<u32>,
    /// Classification of this edge: normal spine edge (`NORMAL`) or synthetic
    /// rib edge (`EXTRA_VD`). Set directly by construction.
    pub edge_type: EdgeType,
    /// Transition-mid annotations placed by
    /// [`super::propagation::generate_transition_mids`] before `apply_transitions`
    /// splits edges at these positions.
    pub transition_mids: Vec<TransitionMiddle>,
}

/// A single transition-middle annotation on a half-edge, placed by
/// [`super::propagation::generate_transition_mids`].
///
/// Mirrors OrcaSlicer's `TransitionMiddle` struct.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionMiddle {
    /// Position along the half-edge at which the transition from
    /// `lower_bead_count` to `lower_bead_count + 1` occurs, as a fraction of
    /// the edge length (`0.0..=1.0`).
    pub pos: f64,
    /// The bead count on the lower-R side of the transition.
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
            rib_twin: None,
            quad_cell: None,
            edge_type: EdgeType::NORMAL,
            transition_mids: Vec::new(),
        }
    }
}

/// The Orca-shaped skeletal trapezoidation half-edge graph.
///
/// Built via [`SkeletalTrapezoidationGraph::from_polygons`]. See the module
/// docs for the per-cell chain + interleaved-rib topology.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SkeletalTrapezoidationGraph {
    /// All graph vertices, indexed by [`STHalfEdge::start_vertex`].
    pub vertices: Vec<STVertex>,
    /// All half-edges, indexed by [`STHalfEdge::twin`]/`next`/`prev`.
    pub edges: Vec<STHalfEdge>,
    /// Whether [`super::centrality::filter_central`] has been run yet.
    pub centrality_filtered: bool,
    /// Rib-topology payload. Empty since packet 113c (ribs live in `edges`).
    pub rib: RibData,
}

/// Errors from [`SkeletalTrapezoidationGraph::from_polygons`].
#[derive(Debug, Clone, PartialEq)]
pub enum SktError {
    /// `from_polygons` was called with an empty polygon slice, or every
    /// supplied polygon contributed zero boundary segments.
    EmptyInput,
    /// An input polygon ring (contour or hole) has fewer than 3 points.
    DegeneratePolygon(String),
    /// The underlying segment Voronoi diagram construction failed.
    Voronoi(VoronoiError),
}

impl fmt::Display for SktError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SktError::EmptyInput => write!(
                f,
                "SkeletalTrapezoidationGraph::from_polygons: empty polygon input"
            ),
            SktError::DegeneratePolygon(msg) => write!(
                f,
                "SkeletalTrapezoidationGraph::from_polygons: degenerate polygon: {msg}"
            ),
            SktError::Voronoi(err) => write!(
                f,
                "SkeletalTrapezoidationGraph::from_polygons: voronoi construction failed: {err}"
            ),
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
    /// boundary [`Segment`]s, fed to [`crate::voronoi::voronoi_from_segments`],
    /// and the resulting diagram is converted into the per-cell chain +
    /// interleaved-rib topology described in the module docs.
    ///
    /// Returns [`SktError::EmptyInput`] for an empty `polys` slice or when
    /// every polygon contributes zero segments, and
    /// [`SktError::DegeneratePolygon`] for any ring with fewer than 3 points.
    /// Never panics.
    pub fn from_polygons(polys: &[ExPolygon]) -> Result<Self, SktError> {
        if polys.is_empty() {
            return Err(SktError::EmptyInput);
        }

        let (segments, rings, seg_prov) = flatten_polys(polys)?;
        if segments.is_empty() {
            return Err(SktError::EmptyInput);
        }

        let he = voronoi::voronoi_from_segments(&segments)?;

        // Defensive bounds-clamp on raw boostvoronoi vertex positions
        // (D-112-MMU-TOPOLOGY / D-113B-WIDE-REGION-COORD-INSTABILITY). See
        // `clamp_implausible_vertex`'s doc comment for the full rationale.
        let input_bbox = segments_bbox(&segments);
        let clamped: Vec<Vertex> = he
            .vertices
            .iter()
            .map(|v| clamp_implausible_vertex(*v, input_bbox))
            .collect();

        let inp = Inputs {
            he: &he,
            segments: &segments,
            rings: &rings,
            seg_prov: &seg_prov,
            clamped: &clamped,
            step: DISCRETIZATION_STEP_UNITS,
        };

        let mut builder = Builder {
            vertices: Vec::new(),
            edges: Vec::new(),
            edge_to: Vec::new(),
            vd_node_to_he_node: vec![NO_INDEX; he.vertices.len()],
            vd_edge_to_he_edge: vec![NO_INDEX; he.edges.len()],
        };
        builder.build(&inp);

        // Finalize per-edge radius bounds from the two endpoints' distances.
        for i in 0..builder.edges.len() {
            let (r_min, r_max) = edge_radius_bounds(
                &builder.vertices,
                builder.edges[i].start_vertex,
                builder.edge_to[i],
            );
            builder.edges[i].r_min = r_min;
            builder.edges[i].r_max = r_max;
        }

        Ok(Self {
            vertices: builder.vertices,
            edges: builder.edges,
            centrality_filtered: false,
            rib: RibData::default(),
        })
    }
}

/// Read-only inputs to the graph construction: the raw Voronoi diagram plus
/// the flatten-time provenance side tables. Kept separate from [`Builder`]'s
/// mutable output so reads and writes never borrow-conflict.
struct Inputs<'a> {
    he: &'a voronoi::HalfEdgeGraph,
    segments: &'a [Segment],
    /// Ring point lists, one per contour/hole, in flatten order.
    rings: &'a [Vec<Point2>],
    /// Per flattened segment: `(ring_id, position_of_point_a_in_ring)`. Used to
    /// recover a point-cell's polygon-adjacent (prev/next) corner vertices for
    /// the membership gate, resolved from `source_index` + `source_category`
    /// directly (no fragile "previous-segment-wins" assumption; see packet
    /// 113c design §Step 2 Spike Findings).
    seg_prov: &'a [(usize, usize)],
    /// boostvoronoi vertex positions, bounds-clamped, indexed like `he.vertices`.
    clamped: &'a [Vertex],
    /// Curved-edge discretization spacing bound (units).
    step: f64,
}

/// Mutable output of the graph construction.
struct Builder {
    vertices: Vec<STVertex>,
    edges: Vec<STHalfEdge>,
    /// Parallel to `edges`: each edge's "to" vertex index. `STHalfEdge` has no
    /// `to` field (it is derived from the twin downstream), but construction
    /// needs it explicitly before every edge's twin is wired.
    edge_to: Vec<usize>,
    /// boostvoronoi vertex index -> graph node index (`NO_INDEX` = not created).
    vd_node_to_he_node: Vec<usize>,
    /// boostvoronoi edge index -> last graph edge of that edge's chain
    /// (`NO_INDEX` = not yet transferred). Mirrors `vd_edge_to_he_edge`.
    vd_edge_to_he_edge: Vec<usize>,
}

impl Builder {
    /// Runs the per-cell `constructFromPolygons` loop.
    fn build(&mut self, inp: &Inputs) {
        for cell_idx in 0..inp.he.cells.len() {
            let cell = &inp.he.cells[cell_idx];
            if cell.is_degenerate || cell.incident_edge == NO_INDEX {
                continue; // "There is no spoon"
            }

            let (start_src, end_src, begin, end) = if cell.contains_point {
                match inp.point_cell_range(cell_idx) {
                    Some((src, b, e)) => (src, src, b, e),
                    None => continue,
                }
            } else if cell.contains_segment {
                match inp.segment_cell_range(cell_idx) {
                    Some((s, e_pt, b, e)) => (s, e_pt, b, e),
                    None => continue,
                }
            } else {
                continue;
            };

            if begin == NO_INDEX || end == NO_INDEX {
                continue;
            }

            let mut prev_edge = NO_INDEX;

            // Starting edge.
            self.transfer_edge(inp, begin, &mut prev_edge, start_src, end_src);
            let v0_begin = inp.vertex0_idx(begin);
            if v0_begin != NO_INDEX {
                let node = self.vd_node_to_he_node[v0_begin];
                if node != NO_INDEX {
                    self.vertices[node].distance_to_boundary = 0.0;
                }
            }
            self.make_rib(&mut prev_edge, start_src, end_src);

            // Middle edges (starting.next() .. ending, exclusive), each ribbed.
            let mut vd_edge = inp.he.edges[begin].next;
            let mut guard = 0usize;
            while vd_edge != NO_INDEX && vd_edge != end && guard <= inp.he.edges.len() {
                self.transfer_edge(inp, vd_edge, &mut prev_edge, start_src, end_src);
                self.make_rib(&mut prev_edge, start_src, end_src);
                vd_edge = inp.he.edges[vd_edge].next;
                guard += 1;
            }

            // Ending (closing) edge — NO rib after it; just mark its end node.
            self.transfer_edge(inp, end, &mut prev_edge, start_src, end_src);
            if prev_edge != NO_INDEX {
                let to = self.edge_to[prev_edge];
                if to != NO_INDEX {
                    self.vertices[to].distance_to_boundary = 0.0;
                }
            }
        }
    }

    /// Ports `transferEdge`: converts one raw Voronoi edge into one or more
    /// graph half-edges, chaining them onto `prev_edge`.
    fn transfer_edge(
        &mut self,
        inp: &Inputs,
        vd_edge: usize,
        prev_edge: &mut usize,
        start_src: Point2,
        end_src: Point2,
    ) {
        let vd_twin = inp.he.edges[vd_edge].twin;
        let source_twin = if vd_twin != NO_INDEX {
            self.vd_edge_to_he_edge[vd_twin]
        } else {
            NO_INDEX
        };

        if source_twin != NO_INDEX {
            // Branch A — the twin raw edge was already transferred; mirror it.
            let v1_idx = inp.vertex1_idx(vd_edge);
            let end_node = if v1_idx != NO_INDEX {
                self.vd_node_to_he_node[v1_idx]
            } else {
                NO_INDEX
            };
            let is_curved = inp.he.edges[vd_edge].is_curved;

            let mut twin = source_twin;
            let mut guard = 0usize;
            loop {
                if twin == NO_INDEX || guard > self.edges.len() {
                    return;
                }
                let new_from = self.edge_to[twin]; // twin.to
                let new_to = self.edges[twin].start_vertex; // twin.from
                let new_edge = self.push_edge(new_from, new_to, is_curved, EdgeType::NORMAL);
                self.edges[new_edge].twin = twin;
                self.edges[twin].twin = new_edge;
                if *prev_edge != NO_INDEX {
                    self.edges[new_edge].prev = *prev_edge;
                    self.edges[*prev_edge].next = new_edge;
                }
                *prev_edge = new_edge;

                if end_node != NO_INDEX && self.edge_to[new_edge] == end_node {
                    return;
                }

                // Step twin = twin.prev.twin.prev, guarding each hop.
                let tp = self.edges[twin].prev;
                if tp == NO_INDEX {
                    return;
                }
                let tpt = self.edges[tp].twin;
                if tpt == NO_INDEX {
                    return;
                }
                let tptp = self.edges[tpt].prev;
                if tptp == NO_INDEX {
                    return;
                }

                self.make_rib(prev_edge, start_src, end_src);
                twin = tptp;
                guard += 1;
            }
        } else {
            // Branch B — fresh transfer; discretize and build the chain.
            let disc = inp.discretize_edge(vd_edge);
            let n = disc.len();
            if n < 2 {
                return;
            }

            let mut v0 = if *prev_edge != NO_INDEX {
                self.edge_to[*prev_edge]
            } else {
                self.make_node_vd(inp, inp.vertex0_idx(vd_edge), disc[0])
            };

            let is_curved = inp.he.edges[vd_edge].is_curved;
            for p1_idx in 1..n {
                let p1 = disc[p1_idx];
                let v1 = if p1_idx < n - 1 {
                    // Interior discretization node — never registered, and
                    // (F5 fix) created with the `NAN` "uncomputed" sentinel
                    // matching `make_node_vd`, since these nodes are never
                    // ribbed so no `make_rib` overwrite will set a real
                    // perpendicular-foot distance. The previous
                    // `nearest_boundary_distance` (global min) was
                    // semantically wrong for these nodes.
                    self.push_node(p1.0, p1.1, f64::NAN)
                } else {
                    self.make_node_vd(inp, inp.vertex1_idx(vd_edge), p1)
                };

                let new_edge = self.push_edge(v0, v1, is_curved, EdgeType::NORMAL);
                if *prev_edge != NO_INDEX {
                    self.edges[new_edge].prev = *prev_edge;
                    self.edges[*prev_edge].next = new_edge;
                }
                *prev_edge = new_edge;
                v0 = v1;

                if p1_idx < n - 1 {
                    // Rib for the last sub-segment is inserted by the caller.
                    self.make_rib(prev_edge, start_src, end_src);
                }
            }

            if *prev_edge != NO_INDEX {
                self.vd_edge_to_he_edge[vd_edge] = *prev_edge;
            }
        }
    }

    /// Ports `makeRib`: inserts a rib pair from `prev_edge`'s "to" node out to
    /// its perpendicular foot on the source segment's infinite line, and
    /// reassigns the caller's cursor to the rib's `back_edge`.
    fn make_rib(&mut self, prev_edge: &mut usize, start_src: Point2, end_src: Point2) {
        let pe = *prev_edge;
        if pe == NO_INDEX {
            return;
        }
        let spine_node = self.edge_to[pe];
        if spine_node == NO_INDEX {
            return;
        }

        let sp = self.vertices[spine_node].position;
        let (px, py) = project_onto_infinite_line((sp.x, sp.y), start_src, end_src);
        let dist = ((sp.x - px).powi(2) + (sp.y - py).powi(2)).sqrt();
        self.vertices[spine_node].distance_to_boundary = dist;

        // Boundary (rib-foot) node — sentinel distance 0.
        let node = self.push_node(px, py, 0.0);

        let forth = self.push_edge(spine_node, node, false, EdgeType::EXTRA_VD);
        let back = self.push_edge(node, spine_node, false, EdgeType::EXTRA_VD);

        self.edges[pe].next = forth;
        self.edges[forth].prev = pe;
        self.edges[forth].twin = back;
        self.edges[back].twin = forth;
        // back_edge.prev / back_edge.next / forth_edge.next intentionally unset:
        // back_edge having no prev is what seeds it as an unprocessed quad start.

        *prev_edge = back;
    }

    /// Looks up or creates the graph node for a boostvoronoi vertex, mirroring
    /// `makeNode` (endpoints are shared across cells via `vd_node_to_he_node`).
    ///
    /// # F5 fix (Arachne parity audit)
    ///
    /// The node is created with `distance_to_boundary = f64::NAN` — an
    /// explicit "uncomputed" sentinel — rather than the previous
    /// `nearest_boundary_distance` (global min over all segments).
    /// OrcaSlicer's `makeNode` (`SkeletalTrapezoidation.cpp:132-143`)
    /// leaves the node at the sentinel `-1` until `makeRib` sets the
    /// perpendicular-foot value (line 459) or
    /// `constructFromPolygons` sets `0.0` on boundary start/end nodes.
    /// The previous eager global-min was semantically wrong for un-ribbed
    /// interior discretization nodes (Branch B of `transfer_edge`), which
    /// retained a plausible-looking but incorrect distance. The two
    /// consumers that actually use the value are unaffected:
    ///
    /// - `make_rib` overwrites the spine node with the perpendicular-foot
    ///   distance (line 552 in this file).
    /// - `build` sets `0.0` on boundary start/end nodes (lines 399, 419).
    ///
    /// `f64::NAN` is used instead of OrcaSlicer's `-1.0` because downstream
    /// `distance_to_boundary` math assumes non-negative values (radii), and
    /// `NAN` propagates visibly if a code path mistakenly reads an un-ribbed
    /// node's distance — surfacing the bug rather than hiding it behind a
    /// plausible-looking wrong number.
    fn make_node_vd(&mut self, inp: &Inputs, vd_idx: usize, fallback: (f64, f64)) -> usize {
        if vd_idx != NO_INDEX {
            let existing = self.vd_node_to_he_node[vd_idx];
            if existing != NO_INDEX {
                return existing;
            }
            let pos = inp.clamped[vd_idx];
            let node = self.push_node(pos.x, pos.y, f64::NAN);
            self.vd_node_to_he_node[vd_idx] = node;
            node
        } else {
            self.push_node(fallback.0, fallback.1, f64::NAN)
        }
    }

    /// Appends a vertex and returns its index.
    fn push_node(&mut self, x: f64, y: f64, distance_to_boundary: f64) -> usize {
        let idx = self.vertices.len();
        self.vertices.push(STVertex {
            position: Vertex { x, y },
            distance_to_boundary,
            bead_count: None,
            transition_ratio: 0.0,
        });
        idx
    }

    /// Appends a half-edge (with its "to" vertex recorded in `edge_to`) and
    /// returns its index.
    fn push_edge(&mut self, from: usize, to: usize, is_curved: bool, edge_type: EdgeType) -> usize {
        let idx = self.edges.len();
        self.edges.push(STHalfEdge {
            start_vertex: from,
            is_curved,
            edge_type,
            ..STHalfEdge::default()
        });
        self.edge_to.push(to);
        idx
    }
}

impl Inputs<'_> {
    /// boostvoronoi `vertex0()` index of a raw edge (its `start_vertex`).
    fn vertex0_idx(&self, e: usize) -> usize {
        self.he.edges[e].start_vertex
    }

    /// boostvoronoi `vertex1()` index of a raw edge (its twin's `start_vertex`).
    fn vertex1_idx(&self, e: usize) -> usize {
        let twin = self.he.edges[e].twin;
        if twin == NO_INDEX {
            NO_INDEX
        } else {
            self.he.edges[twin].start_vertex
        }
    }

    /// Clamped position of a boostvoronoi vertex, as an `(x, y)` `f64` pair.
    fn vertex_pos(&self, idx: usize) -> (f64, f64) {
        let v = self.clamped[idx];
        (v.x, v.y)
    }

    /// `true` if the raw edge has no finite start or end vertex.
    fn is_infinite(&self, e: usize) -> bool {
        self.vertex0_idx(e) == NO_INDEX || self.vertex1_idx(e) == NO_INDEX
    }

    /// Rounded `(i64, i64)` position of a raw edge's `vertex0`.
    fn round_v0(&self, e: usize) -> (i64, i64) {
        round_pt(self.vertex_pos(self.vertex0_idx(e)))
    }

    /// Rounded `(i64, i64)` position of a raw edge's `vertex1`.
    fn round_v1(&self, e: usize) -> (i64, i64) {
        round_pt(self.vertex_pos(self.vertex1_idx(e)))
    }

    /// Resolves a point-cell's generating polygon vertex (the "source point")
    /// from `source_index` + `source_category`.
    fn source_point_of(&self, cell_idx: usize) -> Option<Point2> {
        let cell = &self.he.cells[cell_idx];
        let seg = self.segments.get(cell.source_index)?;
        match cell.source_category {
            SourceCategory::SegmentStart => Some(seg.a),
            SourceCategory::SegmentEnd => Some(seg.b),
            _ => None,
        }
    }

    /// Resolves a segment-cell's generating source segment.
    fn source_segment_of(&self, cell_idx: usize) -> Option<Segment> {
        let cell = &self.he.cells[cell_idx];
        self.segments.get(cell.source_index).copied()
    }

    /// The three polygon corner vertices `(prev, source, next)` around a
    /// point-cell's source vertex, for the membership gate.
    fn source_corner(&self, cell_idx: usize) -> Option<(Point2, Point2, Point2)> {
        let cell = &self.he.cells[cell_idx];
        let (ring_id, pos) = *self.seg_prov.get(cell.source_index)?;
        let ring = self.rings.get(ring_id)?;
        let n = ring.len();
        if n < 3 {
            return None;
        }
        let b_pos = match cell.source_category {
            SourceCategory::SegmentStart => pos,
            SourceCategory::SegmentEnd => (pos + 1) % n,
            _ => return None,
        };
        let a = ring[(b_pos + n - 1) % n];
        let b = ring[b_pos];
        let c = ring[(b_pos + 1) % n];
        Some((a, b, c))
    }

    /// Ports `compute_segment_cell_range`. Returns
    /// `(start_source_point, end_source_point, edge_begin, edge_end)` where
    /// `start = segment.to (HIGH)` and `end = segment.from (LOW)`.
    fn segment_cell_range(&self, cell_idx: usize) -> Option<(Point2, Point2, usize, usize)> {
        let cell = &self.he.cells[cell_idx];
        let seg = self.segments.get(cell.source_index)?;
        let from = seg.a; // LOW
        let to = seg.b; // HIGH
        let from_i = (from.x, from.y);
        let to_i = (to.x, to.y);

        let incident = cell.incident_edge;
        if incident == NO_INDEX {
            return None;
        }

        let mut seen_possible_start = false;
        let mut after_start = false;
        let mut ending_before_start = false;
        let mut edge_begin = NO_INDEX;
        let mut edge_end = NO_INDEX;

        let mut edge = incident;
        let mut guard = 0usize;
        loop {
            if !self.is_infinite(edge) {
                let v0 = self.round_v0(edge);
                let v1 = self.round_v1(edge);
                if v0 == to_i && !after_start {
                    edge_begin = edge;
                    seen_possible_start = true;
                } else if seen_possible_start {
                    after_start = true;
                }
                if v1 == from_i && (edge_end == NO_INDEX || ending_before_start) {
                    ending_before_start = !after_start;
                    edge_end = edge;
                }
            }
            edge = self.he.edges[edge].next;
            guard += 1;
            if edge == NO_INDEX || edge == incident || guard > self.he.edges.len() {
                break;
            }
        }

        if edge_begin == NO_INDEX || edge_end == NO_INDEX || edge_begin == edge_end {
            return None;
        }
        Some((to, from, edge_begin, edge_end))
    }

    /// Ports `compute_point_cell_range`. Returns `(source_point, edge_begin,
    /// edge_end)`, or `None` when the cell is outside the polygon (corner
    /// membership gate) or otherwise degenerate.
    fn point_cell_range(&self, cell_idx: usize) -> Option<(Point2, usize, usize)> {
        let cell = &self.he.cells[cell_idx];
        let src = self.source_point_of(cell_idx)?;
        let src_i = (src.x, src.y);

        let incident = cell.incident_edge;
        if incident == NO_INDEX || self.is_infinite(incident) {
            return None;
        }

        // Corner-membership gate: is the incident edge's far endpoint inside the
        // (prev, source, next) polygon corner?
        let (a, b, c) = self.source_corner(cell_idx)?;
        let v0 = self.round_v0(incident);
        let query = if v0 == src_i {
            let p = self.vertex_pos(self.vertex1_idx(incident));
            Point2 {
                x: p.0.round() as i64,
                y: p.1.round() as i64,
            }
        } else {
            let p = self.vertex_pos(self.vertex0_idx(incident));
            Point2 {
                x: p.0.round() as i64,
                y: p.1.round() as i64,
            }
        };
        if !is_point_inside_polygon_corner(a, b, c, query) {
            return None;
        }

        // Find the edge whose vertex1 == source point; begin = its successor.
        let mut edge_begin = NO_INDEX;
        let mut edge_end = NO_INDEX;
        let mut edge = incident;
        let mut guard = 0usize;
        loop {
            if !self.is_infinite(edge) && self.round_v1(edge) == src_i {
                edge_end = edge;
                edge_begin = self.he.edges[edge].next;
            }
            edge = self.he.edges[edge].next;
            guard += 1;
            if edge == NO_INDEX || edge == incident || guard > self.he.edges.len() {
                break;
            }
        }

        if edge_begin == NO_INDEX || edge_end == NO_INDEX || edge_begin == edge_end {
            return None;
        }
        Some((src, edge_begin, edge_end))
    }

    /// Ports `discretize`: returns the polyline of `(x, y)` points for a raw
    /// Voronoi edge. Straight edges become the chord `[start, end]`; curved
    /// (parabolic) edges are sampled via [`discretize_parabolic_edge`], bounded
    /// by the two Voronoi vertices projected onto the directrix line.
    fn discretize_edge(&self, e: usize) -> Vec<(f64, f64)> {
        let start = self.vertex_pos(self.vertex0_idx(e));
        let end = self.vertex_pos(self.vertex1_idx(e));

        if !self.he.edges[e].is_curved {
            return vec![start, end];
        }

        let twin = self.he.edges[e].twin;
        let left = self.he.edges[e].cell;
        let right = if twin != NO_INDEX {
            self.he.edges[twin].cell
        } else {
            NO_INDEX
        };
        if left == NO_INDEX || right == NO_INDEX {
            return vec![start, end];
        }

        let (focus_cell, seg_cell) = if self.he.cells[left].contains_point {
            (left, right)
        } else if self.he.cells[right].contains_point {
            (right, left)
        } else {
            return vec![start, end];
        };

        let focus = match self.source_point_of(focus_cell) {
            Some(p) => p,
            None => return vec![start, end],
        };
        let seg = match self.source_segment_of(seg_cell) {
            Some(s) => s,
            None => return vec![start, end],
        };

        // Project the two Voronoi vertices onto the directrix (source segment)
        // line, so `discretize_parabolic_edge`'s local-x bounds match the arc
        // between the vertices rather than the directrix's own endpoints.
        let pa = project_onto_infinite_line(start, seg.a, seg.b);
        let pb = project_onto_infinite_line(end, seg.a, seg.b);
        let line_a = Point2 {
            x: pa.0.round() as i64,
            y: pa.1.round() as i64,
        };
        let line_b = Point2 {
            x: pb.0.round() as i64,
            y: pb.1.round() as i64,
        };

        let pts = discretize_parabolic_edge(focus, line_a, line_b, self.step);
        if pts.len() < 2 {
            return vec![start, end];
        }
        pts.iter().map(|p| (p.x as f64, p.y as f64)).collect()
    }
}

/// Rounds an `(x, y)` `f64` pair to the nearest `(i64, i64)`.
fn round_pt(p: (f64, f64)) -> (i64, i64) {
    (p.0.round() as i64, p.1.round() as i64)
}

/// Projects point `p` (f64) onto the **infinite** line through integer points
/// `a`/`b`, returning the foot of the perpendicular. Falls back to `a` when
/// `a == b` (degenerate line — e.g. a point-cell whose source start == end).
fn project_onto_infinite_line(p: (f64, f64), a: Point2, b: Point2) -> (f64, f64) {
    let ax = a.x as f64;
    let ay = a.y as f64;
    let bx = b.x as f64;
    let by = b.y as f64;
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    if len2 < EPS {
        return (ax, ay);
    }
    let t = ((p.0 - ax) * dx + (p.1 - ay) * dy) / len2;
    (ax + t * dx, ay + t * dy)
}

/// Ports `LinearAlg2D::isInsideCorner` / `is_point_inside_polygon_corner`:
/// is `query_point` inside the polygon corner `A-B-C` (CCW, `B` the shared
/// vertex)? Used to reject point-cells lying outside the input polygon.
fn is_point_inside_polygon_corner(a: Point2, b: Point2, c: Point2, query_point: Point2) -> bool {
    let mut bax = (a.x - b.x) as f64;
    let mut bay = (a.y - b.y) as f64;
    let mut bcx = (c.x - b.x) as f64;
    let mut bcy = (c.y - b.y) as f64;
    let mut bqx = (query_point.x - b.x) as f64;
    let mut bqy = (query_point.y - b.y) as f64;

    let na = (bax * bax + bay * bay).sqrt();
    let nc = (bcx * bcx + bcy * bcy).sqrt();
    let nq = (bqx * bqx + bqy * bqy).sqrt();
    if na < EPS || nc < EPS || nq < EPS {
        return false;
    }
    bax /= na;
    bay /= na;
    bcx /= nc;
    bcy /= nc;
    bqx /= nq;
    bqy /= nq;

    // Left normal of BQ.
    let lnx = -bqy;
    let lny = bqx;
    let proj_a_on_bq_normal = bax * lnx + bay * lny;
    let proj_c_on_bq_normal = bcx * lnx + bcy * lny;

    if (proj_a_on_bq_normal > 0.0 && proj_c_on_bq_normal <= 0.0)
        || (proj_a_on_bq_normal <= 0.0 && proj_c_on_bq_normal > 0.0)
    {
        // Q lies angularly between BA and BC: inside iff A is on BQ's left side.
        proj_a_on_bq_normal > 0.0
    } else {
        let proj_a_on_bq = bax * bqx + bay * bqy;
        let proj_c_on_bq = bcx * bqx + bcy * bqy;
        (proj_a_on_bq_normal > 0.0 && proj_c_on_bq < proj_a_on_bq)
            || (proj_a_on_bq_normal <= 0.0 && proj_c_on_bq >= proj_a_on_bq)
    }
}

/// Flattens polygons into boundary segments plus the ring/provenance side
/// tables graph construction needs. Errors on any ring with fewer than 3
/// points.
#[allow(clippy::type_complexity)]
fn flatten_polys(
    polys: &[ExPolygon],
) -> Result<(Vec<Segment>, Vec<Vec<Point2>>, Vec<(usize, usize)>), SktError> {
    let mut segments = Vec::new();
    let mut rings: Vec<Vec<Point2>> = Vec::new();
    let mut seg_prov: Vec<(usize, usize)> = Vec::new();
    for poly in polys {
        push_ring(&poly.contour, &mut segments, &mut rings, &mut seg_prov)?;
        for hole in &poly.holes {
            push_ring(hole, &mut segments, &mut rings, &mut seg_prov)?;
        }
    }
    Ok((segments, rings, seg_prov))
}

/// Pushes one closed ring's segments (and its provenance) into the flattened
/// tables. Errors if `ring` has fewer than 3 points.
fn push_ring(
    ring: &Polygon,
    segments: &mut Vec<Segment>,
    rings: &mut Vec<Vec<Point2>>,
    seg_prov: &mut Vec<(usize, usize)>,
) -> Result<(), SktError> {
    let pts = &ring.points;
    if pts.len() < 3 {
        return Err(SktError::DegeneratePolygon(format!(
            "polygon ring has {} point(s); at least 3 required",
            pts.len()
        )));
    }
    let ring_id = rings.len();
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        seg_prov.push((ring_id, i));
        segments.push(Segment { a, b });
    }
    rings.push(pts.clone());
    Ok(())
}

/// Minimum clamp margin around an input polygon's bounding box, as a fraction
/// of the bbox diagonal. See [`clamp_implausible_vertex`].
const IMPLAUSIBLE_VERTEX_MARGIN_RATIO: f64 = 0.05;

/// Absolute floor (mm) for [`clamp_implausible_vertex`]'s margin.
const IMPLAUSIBLE_VERTEX_MARGIN_FLOOR_MM: f64 = 1.0;

/// Bounding box (`min_x, min_y, max_x, max_y`) of a segment set. `segments` is
/// assumed non-empty by all call sites.
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

/// Clamps a raw boostvoronoi vertex position back into a bounded margin around
/// the input polygon's own bounding box (`bbox`).
///
/// # Why this exists (D-112-MMU-TOPOLOGY / D-113B-WIDE-REGION-COORD-INSTABILITY)
///
/// Direct instrumentation of the
/// `cube_4color_arachne_per_color_footprint_within_bbox` regression (see
/// `docs/DEVIATION_LOG.md`) proved that boostvoronoi occasionally reports a
/// vertex tens to hundreds of thousands of mm away from a small (~10-40mm bbox
/// diagonal), sharp-cornered input polygon. Per `docs/adr/0023`'s degeneracy
/// table, resolving the specific near-collinear/near-duplicate configurations
/// that trigger this instability is `preprocess.rs`'s pre-snap responsibility —
/// but a captured reproduction showed the corruption is a multi-segment
/// interaction across sites from two fragments, not a single ring's own local
/// near-collinearity, so it does not reduce to one provably-complete pre-snap
/// rule. This defensive check is the narrowly-scoped downstream safety net: it
/// clamps the implausible position back to a bounded region, preventing the
/// corrupted vertex from propagating into emitted wall geometry.
///
/// # Threshold derivation (not an arbitrary number)
///
/// An empirical sweep over the regression measured, per produced vertex, how
/// far it fell outside its input polygon's raw bbox as a fraction of the bbox
/// diagonal. The result was a clean bimodal split: legitimate floating-point
/// noise never exceeded ~0.47% of the diagonal (worst: 0.1167mm on a 25.0mm
/// diagonal), while every genuine corruption escaped by ≥~18.5% (smallest:
/// 6.27mm on a 33.9mm diagonal) — a ~40x gap with no borderline cases.
/// [`IMPLAUSIBLE_VERTEX_MARGIN_RATIO`] (5% of the diagonal) sits centrally in
/// that gap: ~10x above the largest noise, ~3.7x below the smallest real
/// corruption. [`IMPLAUSIBLE_VERTEX_MARGIN_FLOOR_MM`] (1mm) only guards a very
/// small input polygon from an under-sized margin.
///
/// # Behavior
///
/// Clamps `v`'s `x`/`y` independently to `[bbox.min - margin, bbox.max +
/// margin]`. Lossy, best-effort recovery — a no-op for every in-bounds vertex.
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
/// `segments`, via point-to-segment distance. `segments` is assumed non-empty.
///
/// Unused since the F5 fix replaced `make_node_vd`'s eager call with the
/// `NAN` sentinel; retained for the future F5-strengthening test that
/// asserts an un-ribbed node's distance equals the perpendicular-foot to
/// its own source segment (which would re-derive this via the cell's
/// source-segment provenance — see the F5 section of
/// `target/arachne_parity_audit_*.md`).
#[allow(dead_code)]
fn nearest_boundary_distance(x: f64, y: f64, segments: &[Segment]) -> f64 {
    segments
        .iter()
        .map(|s| point_to_segment_distance_f64(x, y, s.a, s.b))
        .fold(f64::INFINITY, f64::min)
}

/// Distance from floating-point point `(px, py)` to the closest point on
/// integer-coordinate segment `[a, b]`.
#[allow(dead_code)]
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

/// Derives `(r_min, r_max)` for an edge from its two endpoint vertex indices,
/// each either a valid index into `vertices` or [`crate::voronoi::NO_INDEX`].
///
/// - Both resolvable: `(min, max)` of the two `distance_to_boundary`s.
/// - One resolvable: that side's value for both.
/// - Neither resolvable: `(0.0, 0.0)`.
///
/// **NAN handling (F5 fix):** A `distance_to_boundary` of `f64::NAN` (the
/// "uncomputed" sentinel set by `make_node_vd` for un-ribbed interior
/// nodes) is treated as if the endpoint were unresolved — i.e. the other
/// endpoint's real value is used for both `r_min` and `r_max`. This keeps
/// downstream centrality/bead-count math finite and non-negative
/// (matching OrcaSlicer's treatment of its `-1` sentinel) while still
/// surfacing a `NAN` in any direct read of the node's own
/// `distance_to_boundary` field (the F5 diagnostic invariant).
///
/// Always returns finite, non-negative values with `r_min <= r_max`.
pub fn edge_radius_bounds(vertices: &[STVertex], from_idx: usize, to_idx: usize) -> (f64, f64) {
    let from_d = (from_idx != NO_INDEX)
        .then(|| vertices.get(from_idx).map(|v| v.distance_to_boundary))
        .flatten()
        .filter(|d| d.is_finite());
    let to_d = (to_idx != NO_INDEX)
        .then(|| vertices.get(to_idx).map(|v| v.distance_to_boundary))
        .flatten()
        .filter(|d| d.is_finite());

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

    /// A vertex within the allowed margin (5% of the bbox diagonal, floored at
    /// 1mm) just outside the raw bbox is also left untouched.
    #[test]
    fn clamp_implausible_vertex_preserves_small_legitimate_overshoot() {
        let bbox = (0.0, 0.0, 1.0 * UNITS_PER_MM, 1.0 * UNITS_PER_MM);
        let v = Vertex {
            x: -0.5 * UNITS_PER_MM,
            y: 0.5 * UNITS_PER_MM,
        };
        let clamped = clamp_implausible_vertex(v, bbox);
        assert_eq!(
            clamped, v,
            "a small overshoot within the margin must not be clamped"
        );
    }

    /// A boostvoronoi-style runaway vertex is pulled back to within the margin.
    #[test]
    fn clamp_implausible_vertex_clamps_wild_escape() {
        let side = 25.0 * UNITS_PER_MM;
        let bbox = (0.0, 0.0, side, side);
        let diag = (side * side + side * side).sqrt();
        let margin = (0.05 * diag).max(1.0 * UNITS_PER_MM);

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

    /// Regression test for the captured D-112-MMU-TOPOLOGY / D-113B
    /// reproduction: two disjoint quads that, pre-fix, made boostvoronoi report
    /// a vertex ~64.4mm away. After the fix every vertex stays within a
    /// generous bounded margin of the input polygons' own bbox.
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
                "vertex {i} at {:?} escapes the bounded margin by {escape} units",
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
