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
/// AC-22b — per-edge bisector ownership tagging for classic-perimeters skip-mask.
pub mod bisector_ownership;
/// Paint-segmentation algorithm modules (ported from OrcaSlicer).
///
/// Coordinate constants divided by 100 (OrcaSlicer: 1 nm, Pinch 'n Print: 100 nm).
/// Spatial cell index for 2D line segments.
pub mod colorize;
/// Phase 7 — variant-chain composition: compose per-semantic outputs into disjoint chains per layer.
pub mod compose_variants;
/// Spatial grid for fast line-segment lookup and intersection queries.
pub mod edge_grid;
/// Phase 4f — walk the pruned graph and emit colored segments.
pub mod extract_segments;
/// Step 10 / AC-13 / D14 — slice modifier volumes and route to BASE segment_annotations.
pub mod modifier_volumes;
/// Painted line with semantic value and spatial cell membership.
pub mod painted_line;
/// Collects painted lines by intersecting painted triangles with the layer
/// Z plane and projecting them onto the layer's contour edges.
pub mod painted_line_collection;
/// Phase 1 preprocess — extracts per-layer paint data from mesh objects.
pub mod preprocess;
/// Phase 6 — top/bottom surface propagation across layers.
pub mod top_bottom;
/// Z-plane intersection for triangles.
pub mod triangle_intersect;
/// Voronoi graph construction for MMU segmentation (boostvoronoi wrapper, H561 typed vertices).
pub mod voronoi_graph;
/// Phase 4d/4e — prune redundant arcs and dangling nodes from the MMU graph.
pub mod voronoi_prune;
/// Phase 5 — width limiting and interlocking kernel (`cut_segmented_layers`).
pub mod width_limit;

// ---------------------------------------------------------------------------
// Step 9 — execute_paint_segmentation driver (AC-12)
// ---------------------------------------------------------------------------

use std::sync::Arc;

/// Errors from `execute_paint_segmentation`.
#[derive(Debug)]
pub enum PaintSegmentationError {
    /// boostvoronoi / MMU graph error during Voronoi propagation.
    Voronoi(voronoi_graph::MmuGraphError),
    /// An unexpected empty input was detected after the short-circuit checks passed.
    EmptyInputUnexpected(String),
    /// Catch-all for other errors.
    Other(String),
    /// A Phase 5 config parameter had an out-of-range value (e.g. negative).
    InvalidPhase5Config {
        /// The config key that was invalid (e.g. `"mmu_segmented_region_max_width"`).
        key: String,
        /// The rejected value.
        value: i64,
    },
}

impl std::fmt::Display for PaintSegmentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Voronoi(e) => write!(f, "voronoi error: {e}"),
            Self::EmptyInputUnexpected(s) => write!(f, "unexpected empty input: {s}"),
            Self::Other(s) => write!(f, "paint segmentation v2 error: {s}"),
            Self::InvalidPhase5Config { key, value } => {
                write!(f, "invalid Phase 5 config: {key} = {value}")
            }
        }
    }
}

impl std::error::Error for PaintSegmentationError {}

impl From<voronoi_graph::MmuGraphError> for PaintSegmentationError {
    fn from(e: voronoi_graph::MmuGraphError) -> Self {
        Self::Voronoi(e)
    }
}

/// Multiplier used to stride synthesized painted-chain `region_id`s above any
/// base region_id observed in production. Base region_ids are typically 0..a
/// few-hundred; striding by 1_000_000 keeps all painted-chain synthesized ids
/// well above that floor while leaving room for the per-variant hash.
///
/// Wired into `paint_variant_region_id`. See Fix 1 in
/// `.ralph/specs/95_paint-segmentation-orca-port/implementation-plan.md`
/// (Step 19 dispatch wiring).
pub const PAINT_VARIANT_REGION_ID_STRIDE: u64 = 1_000_000;

/// Deterministic 64-bit content hash of a single `(semantic, value)` chain
/// entry, used to synthesize a unique `region_id` per painted variant chain in
/// `execute_paint_segmentation`.
///
/// The scheme is deliberately simple and stable (no `DefaultHasher` — its seed
/// is per-process random). For `Material/ToolIndex(N)` it returns `N + 1` so
/// the four-color cube fixture lands on tidy 1..=4 hashes (multiplied by the
/// stride to keep them comfortably above base-region floor). For other
/// variants it XOR-folds the semantic-name bytes with a value-discriminant
/// prime and the value payload bits.
fn paint_variant_hash(chain_key: &[(String, slicer_ir::PaintValue)]) -> u64 {
    // BASE chain (no variants) hashes to 0 by definition.
    if chain_key.is_empty() {
        return 0;
    }

    // Per Option B′: cube_4color paints exactly one `Material/ToolIndex` entry
    // per chain. Fast-path the common case so it's trivially auditable: the
    // synthesized region_id is `base * STRIDE + (N + 1)` where N is the tool
    // index. Other variants fall through to the deterministic XOR-fold.
    if chain_key.len() == 1 {
        let (sem_name, value) = &chain_key[0];
        if sem_name == "material" {
            if let slicer_ir::PaintValue::ToolIndex(n) = value {
                return (*n as u64) + 1;
            }
        }
    }

    // Deterministic XOR-fold for arbitrary chains. Primes per discriminant
    // keep distinct value variants from collapsing onto each other.
    let mut h: u64 = 0xCBF2_9CE4_8422_2325; // FNV-64 offset basis
    for (sem_name, value) in chain_key {
        for chunk in sem_name.as_bytes().chunks(8) {
            let mut buf = [0u8; 8];
            buf[..chunk.len()].copy_from_slice(chunk);
            h ^= u64::from_le_bytes(buf);
            h = h.wrapping_mul(0x100_0000_01B3); // FNV-64 prime
        }
        let (disc_prime, payload): (u64, u64) = match value {
            slicer_ir::PaintValue::Flag(b) => (0x9E37_79B9_7F4A_7C15, *b as u64),
            slicer_ir::PaintValue::Scalar(f) => (0xBF58_476D_1CE4_E5B9, (*f).to_bits() as u64),
            slicer_ir::PaintValue::ToolIndex(n) => (0x94D0_49BB_1331_11EB, *n as u64),
            slicer_ir::PaintValue::Custom(s) => {
                let mut fold: u64 = 0;
                for chunk in s.as_bytes().chunks(8) {
                    let mut buf = [0u8; 8];
                    buf[..chunk.len()].copy_from_slice(chunk);
                    fold ^= u64::from_le_bytes(buf);
                    fold = fold.wrapping_mul(0x100_0000_01B3);
                }
                (0x0A0B_0C0D_0E0F_0101, fold)
            }
        };
        h = h
            .wrapping_add(disc_prime)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= payload;
    }
    // Ensure non-zero (0 is reserved for BASE).
    if h == 0 {
        1
    } else {
        h
    }
}

/// Compute the synthesized `region_id` for a painted variant chain rooted at
/// `base_region_id`. For the BASE chain (`chain_key.is_empty()`) returns
/// `base_region_id` unchanged so D14 modifier-volume annotation routing and
/// downstream consumers that key off the source region's id keep working.
fn paint_variant_region_id(
    base_region_id: u64,
    chain_key: &[(String, slicer_ir::PaintValue)],
) -> u64 {
    if chain_key.is_empty() {
        return base_region_id;
    }
    base_region_id
        .saturating_mul(PAINT_VARIANT_REGION_ID_STRIDE)
        .saturating_add(paint_variant_hash(chain_key))
}

/// Returns `true` if any object in `mesh` has at least one painted facet, stroke,
/// or a non-empty support-semantic modifier-volume.  Modifier volumes (D14) are
/// paint sources for the BASE-chain `segment_annotations`, so the short-circuit
/// MUST NOT skip them when the mesh has no facet/stroke paint.
fn mesh_has_any_paint(mesh: &slicer_ir::MeshIR) -> bool {
    for obj in &mesh.objects {
        if let Some(pd) = &obj.paint_data {
            for layer in &pd.layers {
                if layer.facet_values.iter().any(|v| v.is_some()) {
                    return true;
                }
                if !layer.strokes.is_empty() {
                    return true;
                }
            }
        }
        // D14: modifier-volume paint sources.
        for mv in &obj.modifier_volumes {
            let is_support_semantic = matches!(
                mv.config_delta.fields.get("subtype"),
                Some(slicer_ir::ConfigValue::String(s))
                    if s == "support_enforcer" || s == "support_blocker"
            );
            if is_support_semantic && !mv.mesh.vertices.is_empty() && !mv.mesh.indices.is_empty() {
                return true;
            }
        }
    }
    false
}

/// Phase 2–4f pipeline for one layer: build contours, build EdgeGrid, collect
/// painted lines, colorize, build MMU graph, prune, extract segments.
///
/// Returns `Vec<(poly_idx, Option<PaintValue>)>` — one entry per ColoredSegment —
/// ready for conversion to ExPolygons.
///
/// Retained for reference; superseded by the B-4 cell-decomposition path in
/// `execute_paint_segmentation` (which calls `cells_to_expolygons_by_color` directly).
#[cfg(feature = "host-algos")]
#[allow(dead_code)]
fn run_kernel_for_layer(
    layer_slice: &slicer_ir::SliceIR,
    mesh: &slicer_ir::MeshIR,
    num_color_states: usize,
) -> Result<Vec<extract_segments::ColoredSegment>, PaintSegmentationError> {
    use colorize::Contour;
    use triangle_intersect::Line;

    // Build per-region contours from polygons.
    let mut contours: Vec<Contour> = Vec::new();
    for region in &layer_slice.regions {
        for exp in &region.polygons {
            let pts = &exp.contour.points;
            if pts.len() < 2 {
                continue;
            }
            let edges: Vec<Line> = pts
                .windows(2)
                .map(|w| Line {
                    start: w[0],
                    end: w[1],
                })
                .chain(std::iter::once(Line {
                    start: *pts.last().unwrap(),
                    end: pts[0],
                }))
                .collect();
            if !edges.is_empty() {
                contours.push(Contour { edges });
            }
        }
    }

    if contours.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 3 — collect painted lines.
    let painted_lines =
        painted_line_collection::collect_painted_lines(layer_slice, mesh, &contours);
    if painted_lines.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 4a — post-process.
    let filtered = colorize::post_process_painted_lines(&contours, painted_lines);

    // Phase 4b — colorize contours.
    let colored_per_contour = colorize::colorize_contours(&contours, &filtered);

    // Flatten to one Vec<ColoredLine>.
    let colored_lines: Vec<colorize::ColoredLine> =
        colored_per_contour.iter().flatten().cloned().collect();

    if colored_lines.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 4c — build MMU graph.
    let mut graph = voronoi_graph::MMU_Graph::from_colored_lines(&colored_lines)?;

    // NODEDBG — env-gated diagnostic: classify why arc-walks dead-end after one step.
    // Enabled by PNP_PAINTSEG_NODEDBG=1. Prints once for the first layer with ≥4 border arcs.
    #[cfg(debug_assertions)]
    let _nodedbg_noop = (); // suppress unused-variable warnings in release
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static NODEDBG_FIRED: AtomicBool = AtomicBool::new(false);
        if std::env::var("PNP_PAINTSEG_NODEDBG").is_ok() && !NODEDBG_FIRED.load(Ordering::Relaxed) {
            // Find the first border arc seed from border nodes; require ≥4 border arcs total.
            let total_border_arcs = graph
                .arcs
                .iter()
                .filter(|a| !a.deleted && matches!(a.kind, voronoi_graph::MmuArcKind::Border))
                .count();
            if total_border_arcs >= 4 {
                NODEDBG_FIRED.store(true, Ordering::Relaxed);
                // Pick first non-deleted border arc.
                if let Some((seed_ai, seed_arc)) = graph.arcs.iter().enumerate().find(|(_, a)| {
                    !a.deleted && matches!(a.kind, voronoi_graph::MmuArcKind::Border)
                }) {
                    let from_node = seed_arc.from_node;
                    let to_node = seed_arc.to_node;
                    let color = seed_arc.color.clone();
                    eprintln!(
                        "NODEDBG A seed: arc_idx={} from_node={} to_node={} color={:?}  total_border_arcs={}  total_arcs={}  all_border_points={}",
                        seed_ai, from_node, to_node, color, total_border_arcs, graph.arcs.len(), graph.all_border_points
                    );
                    // Print every arc at to_node (pre-prune).
                    let nb_total_pre = graph
                        .arcs
                        .iter()
                        .filter(|a| {
                            !a.deleted && matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder)
                        })
                        .count();
                    eprintln!("NODEDBG A total_NonBorder_arcs={}", nb_total_pre);
                    let nb_incident_pre: Vec<usize> = graph
                        .arcs
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| {
                            !a.deleted
                                && matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder)
                                && (a.from_node == to_node || a.to_node == to_node)
                        })
                        .map(|(i, _)| i)
                        .collect();
                    eprintln!(
                        "NODEDBG A NonBorder_arcs_incident_to_to_node={}: {:?}",
                        nb_incident_pre.len(),
                        nb_incident_pre
                    );
                    for &ai in &graph.nodes[to_node].arc_indices {
                        let a = &graph.arcs[ai];
                        eprintln!(
                            "NODEDBG A   arc[{}] kind={:?} color={:?} from={} to={} deleted={}",
                            ai, a.kind, a.color, a.from_node, a.to_node, a.deleted
                        );
                    }
                    if graph.nodes[to_node].arc_indices.is_empty() {
                        eprintln!("NODEDBG A   <to_node {} has NO arc_indices>", to_node);
                    }
                }
            }
        }
    }

    // Phase 4d/4e — prune.
    // remove_multiple_edges_in_vertices expects &[Vec<ColoredLine>] (colored_per_contour).
    if std::env::var("PNP_PAINTSEG_NOPRUNE").is_err() {
        voronoi_prune::remove_multiple_edges_in_vertices(&mut graph, &colored_per_contour);
        voronoi_prune::remove_nodes_with_one_arc(&mut graph);
    }

    // NODEDBG B — post-prune snapshot of same seed arc's to_node.
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static NODEDBG_B_FIRED: AtomicBool = AtomicBool::new(false);
        if std::env::var("PNP_PAINTSEG_NODEDBG").is_ok() && !NODEDBG_B_FIRED.load(Ordering::Relaxed)
        {
            let total_border_arcs = graph
                .arcs
                .iter()
                .filter(|a| !a.deleted && matches!(a.kind, voronoi_graph::MmuArcKind::Border))
                .count();
            if total_border_arcs >= 4
                || graph
                    .arcs
                    .iter()
                    .any(|a| matches!(a.kind, voronoi_graph::MmuArcKind::Border))
            {
                NODEDBG_B_FIRED.store(true, Ordering::Relaxed);
                if let Some((seed_ai, seed_arc)) = graph
                    .arcs
                    .iter()
                    .enumerate()
                    .find(|(_, a)| matches!(a.kind, voronoi_graph::MmuArcKind::Border))
                {
                    let from_node = seed_arc.from_node;
                    let to_node = seed_arc.to_node;
                    let color = seed_arc.color.clone();
                    eprintln!(
                        "NODEDBG B seed: arc_idx={} from_node={} to_node={} color={:?}  deleted={}",
                        seed_ai, from_node, to_node, color, seed_arc.deleted
                    );
                    let nb_total_post = graph
                        .arcs
                        .iter()
                        .filter(|a| {
                            !a.deleted && matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder)
                        })
                        .count();
                    eprintln!("NODEDBG B total_NonBorder_arcs_alive={}", nb_total_post);
                    let nb_total_all = graph
                        .arcs
                        .iter()
                        .filter(|a| matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder))
                        .count();
                    eprintln!(
                        "NODEDBG B total_NonBorder_arcs_all(incl.deleted)={}",
                        nb_total_all
                    );
                    let nb_incident_post: Vec<(usize, bool)> = graph
                        .arcs
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| {
                            matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder)
                                && (a.from_node == to_node || a.to_node == to_node)
                        })
                        .map(|(i, a)| (i, a.deleted))
                        .collect();
                    eprintln!(
                        "NODEDBG B NonBorder_arcs_incident_to_to_node={}: {:?}",
                        nb_incident_post.len(),
                        nb_incident_post
                    );
                    for &ai in &graph.nodes[to_node].arc_indices {
                        let a = &graph.arcs[ai];
                        eprintln!(
                            "NODEDBG B   arc[{}] kind={:?} color={:?} from={} to={} deleted={}",
                            ai, a.kind, a.color, a.from_node, a.to_node, a.deleted
                        );
                    }
                    if graph.nodes[to_node].arc_indices.is_empty() {
                        eprintln!(
                            "NODEDBG B   <to_node {} has NO arc_indices post-prune>",
                            to_node
                        );
                    }
                }
            }
        }
    }

    // Phase 4f — extract segments.
    let segments = extract_segments::extract_colored_segments(&graph, num_color_states);

    Ok(segments)
}

/// Convert colored segments for one layer into ExPolygons keyed by paint value.
///
/// Emits exactly ONE `ExPolygon` per walk under the walk's SEED colour.
/// The seed colour is the colour of the BORDER arc that seeded the walk
/// (i.e., `seg.color` of the first `ColoredSegment` in each `poly_idx` group,
/// which is always the seeding Border arc). This faithfully mirrors
/// `MultiMaterialSegmentation.cpp`'s `expolygons_segments[seed_arc.color]` push.
///
/// Walks whose seed BORDER arc has no colour (`None`) are emitted under `None`
/// (unpainted / BASE residual) so the compose stage sees them.
fn segments_to_expolygons_by_color(
    segments: &[extract_segments::ColoredSegment],
) -> std::collections::BTreeMap<Option<slicer_ir::PaintValue>, Vec<slicer_ir::ExPolygon>> {
    use slicer_ir::{ExPolygon, Polygon};
    use std::collections::BTreeMap;

    let mut result: BTreeMap<Option<slicer_ir::PaintValue>, Vec<ExPolygon>> = BTreeMap::new();

    if segments.is_empty() {
        return result;
    }

    // Per-walk point list; seed colour recorded from the FIRST segment of each walk.
    let mut walk_pts: BTreeMap<usize, Vec<slicer_ir::Point2>> = BTreeMap::new();
    let mut walk_seed_color: BTreeMap<usize, Option<slicer_ir::PaintValue>> = BTreeMap::new();
    for seg in segments {
        walk_pts
            .entry(seg.poly_idx)
            .or_default()
            .push(seg.line.start);
        // Seed colour = colour of the FIRST segment of this walk (the seeding Border arc).
        walk_seed_color
            .entry(seg.poly_idx)
            .or_insert_with(|| seg.color.clone());
    }

    // Signed area (shoelace) over an open point ring (first != last assumed).
    // Positive = counter-clockwise in this coordinate system.
    fn signed_area(pts: &[slicer_ir::Point2]) -> f64 {
        let n = pts.len();
        if n < 3 {
            return 0.0;
        }
        let mut a = 0.0_f64;
        for i in 0..n {
            let p = pts[i];
            let q = pts[(i + 1) % n];
            a += (p.x as f64) * (q.y as f64) - (q.x as f64) * (p.y as f64);
        }
        a * 0.5
    }

    // Collapse consecutive near-duplicate and collinear vertices on an OPEN ring.
    //
    // The arc-walk emits a cluster of near-coincident points at each region corner
    // (the corner attachment arc plus tiny medial steps). Left in place these form
    // degenerate zero-length edges that (a) are invalid polygon geometry and (b)
    // make a region's *shared corner vertex* read as an EDGE running along a
    // neighbouring face — a false cross-colour "bleed" under the confinement
    // predicate (`regions_with_edge_on_face`, which only permits a single shared
    // corner). Collapsing the clusters yields the clean face-quadrant whose only
    // contact with a neighbour's face is the permitted single corner vertex.
    fn clean_ring(pts: &[slicer_ir::Point2]) -> Vec<slicer_ir::Point2> {
        // 0.1 mm — below one extrusion width, so this only merges true corner
        // clusters, never a real geometric feature.
        const EPS: i64 = 1000;
        let near = |a: &slicer_ir::Point2, b: &slicer_ir::Point2| -> bool {
            (a.x - b.x).abs() <= EPS && (a.y - b.y).abs() <= EPS
        };
        // 1. Drop consecutive near-duplicates (compare against the last kept point
        //    so a whole tight cluster collapses to its first member).
        let mut out: Vec<slicer_ir::Point2> = Vec::with_capacity(pts.len());
        for &p in pts {
            if !out.last().is_some_and(|q| near(q, &p)) {
                out.push(p);
            }
        }
        while out.len() >= 2 && near(&out[0], out.last().unwrap()) {
            out.pop();
        }
        if out.len() < 3 {
            return out;
        }
        // 2. Drop exactly-collinear midpoints (i128 cross == 0). A point is removed
        //    iff it is collinear with its two original neighbours, which correctly
        //    strips an entire straight run down to its endpoints.
        let m = out.len();
        let mut simp: Vec<slicer_ir::Point2> = Vec::with_capacity(m);
        for i in 0..m {
            let a = out[(i + m - 1) % m];
            let b = out[i];
            let c = out[(i + 1) % m];
            let cross = (b.x as i128 - a.x as i128) * (c.y as i128 - b.y as i128)
                - (b.y as i128 - a.y as i128) * (c.x as i128 - b.x as i128);
            if cross != 0 {
                simp.push(b);
            }
        }
        if simp.len() < 3 {
            out
        } else {
            simp
        }
    }

    // Emit exactly ONE ExPolygon per walk under the walk's seed colour.
    for (poly_idx, pts_raw) in walk_pts {
        // The walk point list is an OPEN ring (one point per segment start);
        // clean corner clusters / collinear runs before the winding test so a
        // shared corner is a single vertex, not a degenerate edge.
        let mut pts = clean_ring(&pts_raw);
        if pts.len() < 3 {
            continue;
        }
        // Normalise winding to CCW: a walk traversed in the reverse sense yields a
        // clockwise (negative-area) polygon which cancels against its neighbours in
        // the downstream union, leaving the layer ~half-covered. Reverse such walks
        // so every emitted region carries positive area. Drop near-zero slivers
        // (degenerate walks closed only by the synthetic repair chord) — they
        // contribute no area and only seed cross-colour bleed.
        let area = signed_area(&pts);
        if area.abs() < 1.0e6 {
            continue;
        }
        if area < 0.0 {
            pts.reverse();
        }
        // Emit an OPEN contour ring (no explicit closing duplicate), matching the
        // pipeline convention established by `slice_mesh_ex` (a square slice has 4
        // points, not 5) and consumed everywhere downstream. An explicit closing
        // duplicate would also make a region's start vertex read as a degenerate
        // edge under `regions_with_edge_on_face`, falsely flagging a shared corner
        // that sits on a neighbouring face as cross-colour bleed.
        let seed_color = walk_seed_color.get(&poly_idx).cloned().unwrap_or(None);
        result.entry(seed_color).or_default().push(ExPolygon {
            contour: Polygon { points: pts },
            holes: Vec::new(),
        });
    }

    result
}

/// Legacy single-color variant: returns all painted polygons as one flat Vec.
/// Used only by the `#[cfg(not(feature = "host-algos"))]` stub path.
#[allow(dead_code)]
fn segments_to_expolygons(
    segments: &[extract_segments::ColoredSegment],
) -> Vec<slicer_ir::ExPolygon> {
    use slicer_ir::{ExPolygon, Point2, Polygon};
    use std::collections::BTreeMap;

    if segments.is_empty() {
        return Vec::new();
    }

    // Group by poly_idx.
    let mut by_poly: BTreeMap<usize, Vec<Point2>> = BTreeMap::new();
    for seg in segments {
        let pts = by_poly.entry(seg.poly_idx).or_default();
        pts.push(seg.line.start);
    }
    // Close each polygon.
    for (poly_idx, pts) in &mut by_poly {
        let _ = poly_idx; // suppress unused warning
        if let Some(&first) = pts.first() {
            pts.push(first);
        }
    }

    by_poly
        .into_values()
        .filter(|pts| pts.len() >= 3)
        .map(|points| ExPolygon {
            contour: Polygon { points },
            holes: Vec::new(),
        })
        .collect()
}

/// Execute the full paint-segmentation v2 pipeline.
///
/// # Short-circuit rules (AC-N2)
/// - Empty mesh → return input slice_ir unchanged.
/// - No painted facets or strokes → return input slice_ir unchanged.
/// - Empty region_map → return input slice_ir unchanged.
///
/// # Pipeline (AC-12)
/// For each layer: Phase 3 → Phase 4a/4b/4c/4d/4e/4f → Phase 7 compose →
/// rebuild SlicedRegions per (RegionKey × variant_chain) tuple.
pub fn execute_paint_segmentation(
    mesh: Arc<slicer_ir::MeshIR>,
    slice_ir: Arc<Vec<slicer_ir::SliceIR>>,
    region_map: Arc<slicer_ir::RegionMapIR>,
) -> Result<Arc<Vec<slicer_ir::SliceIR>>, PaintSegmentationError> {
    // ---- AC-N2: short-circuit checks ----------------------------------------
    if mesh.objects.is_empty() {
        return Ok(slice_ir.clone());
    }
    if !mesh_has_any_paint(&mesh) {
        return Ok(slice_ir.clone());
    }
    if region_map.entries.is_empty() {
        return Ok(slice_ir.clone());
    }

    // ---- Working copy --------------------------------------------------------
    let mut working: Vec<slicer_ir::SliceIR> = Vec::from_iter(slice_ir.iter().cloned());

    // ---- Step 10 / AC-13 / D14: slice modifier volumes once for all layers ----
    // Produces per-layer per-semantic polygon lists; routed onto BASE chains only.
    let layer_zs: Vec<f32> = working.iter().map(|s| s.z).collect();
    let modifier_vol_per_layer = modifier_volumes::slice_modifier_volumes(&mesh, &layer_zs);

    for i in 0..working.len() {
        let layer_slice = &working[i];

        if layer_slice.regions.is_empty() {
            continue;
        }

        let global_layer_index = layer_slice.global_layer_index;

        // Collect layer-total contours (BASE chain polygons and reference for per-color regions).
        let layer_total_contours: Vec<slicer_ir::ExPolygon> = layer_slice
            .regions
            .iter()
            .flat_map(|r| r.polygons.iter().cloned())
            .collect();

        // Determine num_color_states from PaintLayer facet values.
        // Passed to extract_colored_segments (API parity with OrcaSlicer).
        let num_color_states: usize = {
            let mut max_tool: usize = 0;
            for obj in &mesh.objects {
                let Some(pd) = &obj.paint_data else { continue };
                for layer in &pd.layers {
                    for fv in &layer.facet_values {
                        if let Some(slicer_ir::PaintValue::ToolIndex(t)) = fv {
                            max_tool = max_tool.max(*t as usize + 1);
                        }
                    }
                }
            }
            max_tool.max(2)
        };

        // Determine the dominant PaintSemantic for this object (first painted layer's semantic).
        // Used to label the SemanticOutput entries with the correct semantic family.
        let dominant_semantic: slicer_ir::PaintSemantic = {
            let mut sem = slicer_ir::PaintSemantic::Material; // default
            'outer: for obj in &mesh.objects {
                let Some(pd) = &obj.paint_data else { continue };
                for layer in &pd.layers {
                    if layer.facet_values.iter().any(|v| v.is_some()) || !layer.strokes.is_empty() {
                        sem = layer.semantic.clone();
                        break 'outer;
                    }
                }
            }
            sem
        };

        // Run kernel (feature-gated).
        #[cfg(feature = "host-algos")]
        let polys_by_color = {
            // Build contours → colored lines → MMU graph → arc-walk decomposition.
            use colorize::Contour;
            use triangle_intersect::Line;

            let mut contours: Vec<Contour> = Vec::new();
            for region in &working[i].regions {
                for exp in &region.polygons {
                    let pts = &exp.contour.points;
                    if pts.len() < 2 {
                        continue;
                    }
                    let edges: Vec<Line> = pts
                        .windows(2)
                        .map(|w| Line {
                            start: w[0],
                            end: w[1],
                        })
                        .chain(std::iter::once(Line {
                            start: *pts.last().unwrap(),
                            end: pts[0],
                        }))
                        .collect();
                    if !edges.is_empty() {
                        contours.push(Contour { edges });
                    }
                }
            }

            if contours.is_empty() {
                std::collections::BTreeMap::new()
            } else {
                let painted_lines =
                    painted_line_collection::collect_painted_lines(&working[i], &mesh, &contours);
                if painted_lines.is_empty() {
                    std::collections::BTreeMap::new()
                } else {
                    let filtered = colorize::post_process_painted_lines(&contours, painted_lines);
                    let colored_per_contour = colorize::colorize_contours(&contours, &filtered);
                    let colored_lines: Vec<colorize::ColoredLine> =
                        colored_per_contour.iter().flatten().cloned().collect();

                    if colored_lines.is_empty() {
                        std::collections::BTreeMap::new()
                    } else {
                        match voronoi_graph::MMU_Graph::from_colored_lines(&colored_lines) {
                            Err(e) => return Err(PaintSegmentationError::from(e)),
                            Ok(mut graph) => {
                                // NODEDBG A — pre-prune snapshot (gated by PNP_PAINTSEG_NODEDBG).
                                {
                                    use std::sync::atomic::{AtomicBool, Ordering};
                                    static NODEDBG2_A_FIRED: AtomicBool = AtomicBool::new(false);
                                    if std::env::var("PNP_PAINTSEG_NODEDBG").is_ok()
                                        && !NODEDBG2_A_FIRED.load(Ordering::Relaxed)
                                    {
                                        let total_border = graph
                                            .arcs
                                            .iter()
                                            .filter(|a| {
                                                !a.deleted
                                                    && matches!(
                                                        a.kind,
                                                        voronoi_graph::MmuArcKind::Border
                                                    )
                                            })
                                            .count();
                                        if total_border >= 4 {
                                            NODEDBG2_A_FIRED.store(true, Ordering::Relaxed);
                                            if let Some((seed_ai, seed_arc)) =
                                                graph.arcs.iter().enumerate().find(|(_, a)| {
                                                    !a.deleted
                                                        && matches!(
                                                            a.kind,
                                                            voronoi_graph::MmuArcKind::Border
                                                        )
                                                })
                                            {
                                                let from_node = seed_arc.from_node;
                                                let to_node = seed_arc.to_node;
                                                let color = seed_arc.color.clone();
                                                eprintln!(
                                                    "NODEDBG A seed: arc_idx={} from_node={} to_node={} color={:?}  total_border={} total_arcs={} all_border_points={}",
                                                    seed_ai, from_node, to_node, color, total_border, graph.arcs.len(), graph.all_border_points
                                                );
                                                let nb_total = graph.arcs.iter()
                                                    .filter(|a| !a.deleted && matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder))
                                                    .count();
                                                eprintln!(
                                                    "NODEDBG A total_NonBorder_arcs={}",
                                                    nb_total
                                                );
                                                let nb_incident: Vec<usize> = graph.arcs.iter().enumerate()
                                                    .filter(|(_, a)| !a.deleted
                                                        && matches!(a.kind, voronoi_graph::MmuArcKind::NonBorder)
                                                        && (a.from_node == to_node || a.to_node == to_node))
                                                    .map(|(i, _)| i)
                                                    .collect();
                                                eprintln!("NODEDBG A NonBorder_incident_to_node[{}]: count={} arcs={:?}", to_node, nb_incident.len(), nb_incident);
                                                for &ai in &graph.nodes[to_node].arc_indices {
                                                    let a = &graph.arcs[ai];
                                                    eprintln!(
                                                        "NODEDBG A   arc[{}] kind={:?} color={:?} from={} to={} deleted={}",
                                                        ai, a.kind, a.color, a.from_node, a.to_node, a.deleted
                                                    );
                                                }
                                                if graph.nodes[to_node].arc_indices.is_empty() {
                                                    eprintln!("NODEDBG A   <to_node {} has NO arc_indices>", to_node);
                                                }
                                            }
                                        }
                                    }
                                }
                                if std::env::var("PNP_PAINTSEG_NOPRUNE").is_err() {
                                    voronoi_prune::remove_multiple_edges_in_vertices(
                                        &mut graph,
                                        &colored_per_contour,
                                    );
                                    voronoi_prune::remove_nodes_with_one_arc(&mut graph);
                                }
                                // NODEDBG B — post-prune snapshot.
                                {
                                    use std::sync::atomic::{AtomicBool, Ordering};
                                    static NODEDBG2_B_FIRED: AtomicBool = AtomicBool::new(false);
                                    if std::env::var("PNP_PAINTSEG_NODEDBG").is_ok()
                                        && !NODEDBG2_B_FIRED.load(Ordering::Relaxed)
                                    {
                                        // Find seed again (post-prune, same arc index 0 approach)
                                        if let Some((seed_ai, seed_arc)) =
                                            graph.arcs.iter().enumerate().find(|(_, a)| {
                                                matches!(a.kind, voronoi_graph::MmuArcKind::Border)
                                            })
                                        {
                                            NODEDBG2_B_FIRED.store(true, Ordering::Relaxed);
                                            let from_node = seed_arc.from_node;
                                            let to_node = seed_arc.to_node;
                                            let color = seed_arc.color.clone();
                                            let deleted = seed_arc.deleted;
                                            eprintln!(
                                                "NODEDBG B seed: arc_idx={} from_node={} to_node={} color={:?} deleted={}",
                                                seed_ai, from_node, to_node, color, deleted
                                            );
                                            let nb_alive = graph
                                                .arcs
                                                .iter()
                                                .filter(|a| {
                                                    !a.deleted
                                                        && matches!(
                                                            a.kind,
                                                            voronoi_graph::MmuArcKind::NonBorder
                                                        )
                                                })
                                                .count();
                                            let nb_all = graph
                                                .arcs
                                                .iter()
                                                .filter(|a| {
                                                    matches!(
                                                        a.kind,
                                                        voronoi_graph::MmuArcKind::NonBorder
                                                    )
                                                })
                                                .count();
                                            eprintln!("NODEDBG B total_NonBorder_alive={} total_NonBorder_all={}", nb_alive, nb_all);
                                            let nb_incident_post: Vec<(usize, bool)> = graph
                                                .arcs
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, a)| {
                                                    matches!(
                                                        a.kind,
                                                        voronoi_graph::MmuArcKind::NonBorder
                                                    ) && (a.from_node == to_node
                                                        || a.to_node == to_node)
                                                })
                                                .map(|(i, a)| (i, a.deleted))
                                                .collect();
                                            eprintln!(
                                                "NODEDBG B NonBorder_incident_to_node[{}]: count={} (idx,deleted)={:?}",
                                                to_node, nb_incident_post.len(), nb_incident_post
                                            );
                                            for &ai in &graph.nodes[to_node].arc_indices {
                                                let a = &graph.arcs[ai];
                                                eprintln!(
                                                    "NODEDBG B   arc[{}] kind={:?} color={:?} from={} to={} deleted={}",
                                                    ai, a.kind, a.color, a.from_node, a.to_node, a.deleted
                                                );
                                            }
                                            if graph.nodes[to_node].arc_indices.is_empty() {
                                                eprintln!("NODEDBG B   <to_node {} has NO arc_indices post-prune>", to_node);
                                            }
                                        }
                                    }
                                }
                                // Faithful arc-walk decomposition (Orca parity).
                                let segments = extract_segments::extract_colored_segments(
                                    &graph,
                                    num_color_states,
                                );
                                let polys_by_color_result =
                                    segments_to_expolygons_by_color(&segments);
                                // FACEDBG: dump, for the mid-height layer (z≈12.5), each
                                // output colour-polygon's bbox + which cube face its contour
                                // has an EDGE on (mirrors the AC-2 confinement predicate
                                // `regions_with_edge_on_face`). Gated; remove after diagnosis.
                                if std::env::var("PNP_PAINTSEG_FACEDBG").is_ok()
                                    && (layer_zs[i] - 12.5).abs() < 0.3
                                {
                                    let (xmn, xmx, ymn, ymx) =
                                        (1_125_000i64, 1_375_000i64, 925_000i64, 1_175_000i64);
                                    let tol = 2500i64;
                                    let edge_on =
                                        |pts: &[slicer_ir::Point2],
                                         f: &dyn Fn(slicer_ir::Point2) -> bool|
                                         -> bool {
                                            let n = pts.len();
                                            n >= 2
                                                && (0..n)
                                                    .any(|k| f(pts[k]) && f(pts[(k + 1) % n]))
                                        };
                                    eprintln!(
                                        "FACEDBG layer z={:.2} colors={}",
                                        layer_zs[i],
                                        polys_by_color_result.len()
                                    );
                                    for (col, polys) in &polys_by_color_result {
                                        for (pi, ep) in polys.iter().enumerate() {
                                            let pts = &ep.contour.points;
                                            let back = edge_on(pts, &|p| p.y >= ymx - tol);
                                            let front = edge_on(pts, &|p| p.y <= ymn + tol);
                                            let right = edge_on(pts, &|p| p.x >= xmx - tol);
                                            let left = edge_on(pts, &|p| p.x <= xmn + tol);
                                            let (mut bxmn, mut bxmx, mut bymn, mut bymx) =
                                                (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
                                            for p in pts {
                                                bxmn = bxmn.min(p.x);
                                                bxmx = bxmx.max(p.x);
                                                bymn = bymn.min(p.y);
                                                bymx = bymx.max(p.y);
                                            }
                                            eprintln!(
                                                "FACEDBG  color={:?} poly#{} npts={} bbox=[{}..{}]x[{}..{}] edges back={} front={} right={} left={}",
                                                col, pi, pts.len(), bxmn, bxmx, bymn, bymx, back, front, right, left
                                            );
                                            if pts.len() <= 30 {
                                                let s: Vec<String> = pts
                                                    .iter()
                                                    .map(|p| {
                                                        format!(
                                                            "({:.1},{:.1})",
                                                            p.x as f64 / 10000.0,
                                                            p.y as f64 / 10000.0
                                                        )
                                                    })
                                                    .collect();
                                                eprintln!("FACEDBG    pts_mm={}", s.join(" "));
                                            }
                                        }
                                    }
                                }
                                // TEMPORARY diagnostic instrumentation — remove after diagnosis.
                                // Gate: PNP_PAINTSEG_WALK_DEBUG (any non-empty value).
                                if std::env::var("PNP_PAINTSEG_WALK_DEBUG").is_ok() {
                                    fn shoelace(pts: &[slicer_ir::Point2]) -> f64 {
                                        let n = pts.len();
                                        if n < 3 {
                                            return 0.0;
                                        }
                                        let mut acc: i128 = 0;
                                        for k in 0..n {
                                            let j = (k + 1) % n;
                                            acc += (pts[k].x as i128) * (pts[j].y as i128)
                                                - (pts[j].x as i128) * (pts[k].y as i128);
                                        }
                                        (acc as f64).abs() * 0.5
                                    }
                                    let poly_ids: std::collections::BTreeSet<usize> =
                                        segments.iter().map(|s| s.poly_idx).collect();
                                    let walks_seeded = poly_ids.len();
                                    let walks_needed_chord = segments
                                        .iter()
                                        .filter(|s| s.arc_idx.is_none())
                                        .map(|s| s.poly_idx)
                                        .collect::<std::collections::BTreeSet<usize>>()
                                        .len();
                                    let walks_closed_natural =
                                        walks_seeded.saturating_sub(walks_needed_chord);
                                    let mut polys_count = 0usize;
                                    let mut polys_with_area_count = 0usize;
                                    let mut polys_sliver = 0usize;
                                    let mut total_painted_area = 0.0_f64;
                                    for (color_opt, polys) in &polys_by_color_result {
                                        if color_opt.is_none() {
                                            continue;
                                        }
                                        for ep in polys {
                                            polys_count += 1;
                                            let a = shoelace(&ep.contour.points);
                                            if a > 1e6_f64 {
                                                polys_with_area_count += 1;
                                                total_painted_area += a;
                                            } else {
                                                polys_sliver += 1;
                                            }
                                        }
                                    }
                                    let input_area: f64 = layer_total_contours
                                        .iter()
                                        .map(|ep| shoelace(&ep.contour.points))
                                        .sum();
                                    let area_ratio = if input_area > 0.0 {
                                        total_painted_area / input_area
                                    } else {
                                        0.0
                                    };
                                    eprintln!(
                                        "WALKDBG layer={} seeded={} closed_natural={} needed_chord={} polys={} area_polys={} slivers={} area_ratio={:.4}",
                                        global_layer_index,
                                        walks_seeded,
                                        walks_closed_natural,
                                        walks_needed_chord,
                                        polys_count,
                                        polys_with_area_count,
                                        polys_sliver,
                                        area_ratio,
                                    );
                                }
                                polys_by_color_result
                            }
                        }
                    }
                }
            }
        };
        #[cfg(not(feature = "host-algos"))]
        let polys_by_color: std::collections::BTreeMap<
            Option<slicer_ir::PaintValue>,
            Vec<slicer_ir::ExPolygon>,
        > = std::collections::BTreeMap::new();

        // AC-12 (d): rebuild SlicedRegions.
        //
        // For multi-value same-semantic paint (the common case for MMU: multiple
        // ToolIndex values of the Material semantic), each color is independent —
        // there is no cross-product intersection to compute. We therefore bypass
        // compose_variants (which is designed for multi-SEMANTIC fan-out, e.g.
        // Material × SupportEnforcer) and emit one SlicedRegion per (color, polys)
        // pair directly.
        //
        // The BASE chain (variant_chain = []) uses the full layer contours and
        // carries modifier-volume annotations (D14 invariant).
        {
            // Helper: produce a chain key `[(semantic_name, value)]` for one color.
            let sem_name: String = match &dominant_semantic {
                slicer_ir::PaintSemantic::Material => "material".to_owned(),
                slicer_ir::PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
                slicer_ir::PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
                slicer_ir::PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
                slicer_ir::PaintSemantic::Custom(name) => name.clone(),
            };

            let mut new_regions: Vec<slicer_ir::SlicedRegion> = Vec::new();

            // BASE chain — empty variant_chain; carries modifier-volume annotations.
            //
            // Fix 4 (Step 19 / Option B′ residual): the BASE chain's polygons
            // must NOT be the full layer contour when there are also painted
            // chains, because classic-perimeters / arachne-perimeters would
            // emit a SECOND set of outer-wall extrusions on top of the
            // per-color painted chains, doubling the per-layer outer-wall
            // count (AC-22 Test 2 failure). Instead:
            //
            //  * If BASE has modifier-volume `segment_annotations`, we keep
            //    BASE with the FULL layer-contour polygons so the annotation
            //    routing continues to work (modifier-volume fixtures take a
            //    slower but correct dual-emit path).
            //
            //  * Otherwise BASE acts as the RESIDUAL carrier: its polygons
            //    are the unpainted-area cells emitted by the Voronoi cell
            //    decomposition (`color_opt == None` entry in
            //    `polys_by_color`). This preserves the v2 contract that the
            //    unpainted portion of a partly-painted face appears as a
            //    region with empty `variant_chain` (see
            //    `cube_4color_bottom_face_painted_and_unpainted_requires_projection_coverage`)
            //    while keeping the per-layer outer-wall count close to the
            //    unpainted baseline.
            //
            //  * If neither modifier annotations nor residual cells exist for
            //    the layer (i.e. the whole layer is covered by painted
            //    chains), we drop BASE entirely so classic-perimeters'
            //    `polygons.is_empty()` early-return skips it.
            let base_segment_annotations = build_modifier_segment_annotations(
                i,
                &layer_total_contours,
                &modifier_vol_per_layer,
            );
            let base_has_modifier_annotations = !base_segment_annotations.is_empty();
            let base_chain_key: Vec<(String, slicer_ir::PaintValue)> = vec![];

            // Residual polygons for the BASE chain when no modifier annotations exist.
            // Sourced from the `None`-keyed entry in `polys_by_color` (the Voronoi
            // cell decomposition emits a `None` entry whenever there are unpainted
            // cells in the layer).
            let residual_polys: Vec<slicer_ir::ExPolygon> =
                polys_by_color.get(&None).cloned().unwrap_or_default();

            let base_polygons: Vec<slicer_ir::ExPolygon> = if base_has_modifier_annotations {
                layer_total_contours.clone()
            } else {
                residual_polys
            };

            // Always emit BASE so the v2 contract holds: every painted layer
            // has at least one SlicedRegion with empty `variant_chain` (see
            // `cube_4color_bottom_face_painted_and_unpainted_requires_projection_coverage`).
            // When BASE has neither modifier annotations nor residual cells,
            // its `polygons` is empty and perimeter generators short-circuit
            // via `if polygons.is_empty() { continue; }`.
            {
                let matching_base: Vec<&slicer_ir::RegionKey> = region_map
                    .entries
                    .keys()
                    .filter(|rk| {
                        rk.global_layer_index == global_layer_index
                            && rk.variant_chain == base_chain_key
                    })
                    .collect();
                if matching_base.is_empty() {
                    if let Some(existing) = working[i].regions.first() {
                        new_regions.push(slicer_ir::SlicedRegion {
                            object_id: existing.object_id.clone(),
                            region_id: existing.region_id,
                            polygons: base_polygons.clone(),
                            variant_chain: base_chain_key.clone(),
                            segment_annotations: base_segment_annotations,
                            ..Default::default()
                        });
                    }
                } else {
                    for rk in matching_base {
                        new_regions.push(slicer_ir::SlicedRegion {
                            object_id: rk.object_id.clone(),
                            region_id: rk.region_id,
                            polygons: base_polygons.clone(),
                            variant_chain: base_chain_key.clone(),
                            segment_annotations: base_segment_annotations.clone(),
                            ..Default::default()
                        });
                    }
                }
            }

            // One painted chain per distinct (semantic, value) color.
            // The `None` entry was consumed above as BASE residual polygons;
            // here we only iterate `Some(value)` colors.
            for (color_opt, polys) in &polys_by_color {
                let Some(value) = color_opt else { continue }; // residual consumed by BASE
                if polys.is_empty() {
                    continue;
                }
                let chain_key: Vec<(String, slicer_ir::PaintValue)> =
                    vec![(sem_name.clone(), value.clone())];

                let matching_keys: Vec<&slicer_ir::RegionKey> = region_map
                    .entries
                    .keys()
                    .filter(|rk| {
                        rk.global_layer_index == global_layer_index && rk.variant_chain == chain_key
                    })
                    .collect();

                // Fix 1 (Step 19 / Option B′): synthesize a unique region_id
                // per painted variant chain so the host's PerimeterRegionOrigin
                // = (object_id, region_id) bucketing emits one perimeter region
                // per color rather than collapsing all painted chains onto the
                // BASE region_id.
                //
                // FuzzySkin routing (D14): the painted FuzzySkin signal travels
                // on `variant_chain` (`("fuzzy_skin", Flag(true))`, set below),
                // NOT in `segment_annotations` — that field is reserved for
                // modifier-volume semantics (SupportEnforcer/SupportBlocker).
                // The host projects the fuzzy flag from `variant_chain` onto the
                // guest's `slice-region-view` so `build_wall_flags` can enable
                // per-vertex jitter without conflating the two channels.

                if matching_keys.is_empty() {
                    if let Some(existing) = working[i].regions.first() {
                        new_regions.push(slicer_ir::SlicedRegion {
                            object_id: existing.object_id.clone(),
                            region_id: paint_variant_region_id(existing.region_id, &chain_key),
                            polygons: polys.clone(),
                            variant_chain: chain_key.clone(),
                            // segment_annotations stays empty (D14): FuzzySkin
                            // travels on variant_chain, not here.
                            ..Default::default()
                        });
                    }
                } else {
                    for rk in matching_keys {
                        new_regions.push(slicer_ir::SlicedRegion {
                            object_id: rk.object_id.clone(),
                            region_id: paint_variant_region_id(rk.region_id, &chain_key),
                            polygons: polys.clone(),
                            variant_chain: chain_key.clone(),
                            // segment_annotations stays empty (D14): FuzzySkin
                            // travels on variant_chain, not here.
                            ..Default::default()
                        });
                    }
                }
            }

            // Fix 1 cell-tiling diagnostic (Step 19 / Option B′):
            // Verify that the union of all painted-chain polygons covers the
            // BASE polygon area within 1% relative error. If a gap is observed,
            // the arc-walk cell decomposition has left holes — tracked as a
            // follow-up in the closure-log. The diagnostic is gated behind
            // env var `PNP_PAINTSEG_CELL_TILING_DEBUG=1` to keep debug-build
            // tests quiet by default.
            #[cfg(feature = "host-algos")]
            if std::env::var("PNP_PAINTSEG_CELL_TILING_DEBUG")
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                use crate::polygon_ops::union_ex;
                fn expoly_area_signed_sum(polys: &[slicer_ir::ExPolygon]) -> f64 {
                    let mut a = 0.0_f64;
                    for ep in polys {
                        let pts = &ep.contour.points;
                        if pts.len() >= 3 {
                            let mut acc = 0i128;
                            for i in 0..pts.len() {
                                let j = (i + 1) % pts.len();
                                acc += (pts[i].x as i128) * (pts[j].y as i128)
                                    - (pts[j].x as i128) * (pts[i].y as i128);
                            }
                            a += (acc as f64) * 0.5;
                            for hole in &ep.holes {
                                let hpts = &hole.points;
                                if hpts.len() < 3 {
                                    continue;
                                }
                                let mut hacc = 0i128;
                                for i in 0..hpts.len() {
                                    let j = (i + 1) % hpts.len();
                                    hacc += (hpts[i].x as i128) * (hpts[j].y as i128)
                                        - (hpts[j].x as i128) * (hpts[i].y as i128);
                                }
                                a -= (hacc as f64).abs() * 0.5;
                            }
                        }
                    }
                    a.abs()
                }
                let mut painted_polys: Vec<slicer_ir::ExPolygon> = Vec::new();
                for r in &new_regions {
                    if !r.variant_chain.is_empty() {
                        painted_polys.extend(r.polygons.iter().cloned());
                    }
                }
                if !painted_polys.is_empty() {
                    let unioned = union_ex(&painted_polys);
                    let union_area = expoly_area_signed_sum(&unioned);
                    let base_area = expoly_area_signed_sum(&layer_total_contours);
                    let diff = (base_area - union_area).abs();
                    let rel = if base_area > 0.0 {
                        diff / base_area
                    } else {
                        0.0
                    };
                    if rel > 0.01 {
                        eprintln!(
                            "[paint-seg cell-tiling] layer {global_layer_index}: \
                             base_area={base_area}, union_area={union_area}, \
                             diff={diff}, rel_diff={rel:.4}"
                        );
                    }
                }
            }

            // Fix (diagnose 2026-06-24, gap #1): `PrePass::ShellClassification`
            // runs BEFORE `PrePass::PaintSegmentation` and writes top/bottom solid
            // fill into the pre-segmentation BASE region. The wholesale
            // `working[i].regions = new_regions` replacement below discards it, and
            // every new per-color region is built with `..Default::default()`
            // (empty `top_solid_fill`/`bottom_solid_fill`). The net effect was that
            // PAINTED models emitted ZERO top/bottom/internal-solid infill (open
            // tops, 4x extrusion deficit vs OrcaSlicer) while unpainted models were
            // fine. Propagate the classified fill into each new region, clipped to
            // that region's own polygon area so each per-color cell gets exactly
            // its share (mirrors region_partition::sync_perimeter_infill_areas_into_slice,
            // which re-clips to perimeter.infill_areas downstream).
            {
                use crate::polygon_ops::intersection_ex;
                let mut saved_top: Vec<slicer_ir::ExPolygon> = Vec::new();
                let mut saved_bottom: Vec<slicer_ir::ExPolygon> = Vec::new();
                let mut saved_bridge: Vec<slicer_ir::ExPolygon> = Vec::new();
                let mut saved_top_idx: Option<u8> = None;
                let mut saved_bottom_idx: Option<u8> = None;
                for r in &working[i].regions {
                    saved_top.extend(r.top_solid_fill.iter().cloned());
                    saved_bottom.extend(r.bottom_solid_fill.iter().cloned());
                    saved_bridge.extend(r.bridge_areas.iter().cloned());
                    saved_top_idx = saved_top_idx.or(r.top_shell_index);
                    saved_bottom_idx = saved_bottom_idx.or(r.bottom_shell_index);
                }
                if !saved_top.is_empty() || !saved_bottom.is_empty() || !saved_bridge.is_empty() {
                    for r in &mut new_regions {
                        if !saved_top.is_empty() {
                            r.top_solid_fill = intersection_ex(&saved_top, &r.polygons);
                        }
                        if !saved_bottom.is_empty() {
                            r.bottom_solid_fill = intersection_ex(&saved_bottom, &r.polygons);
                        }
                        if !saved_bridge.is_empty() {
                            r.bridge_areas = intersection_ex(&saved_bridge, &r.polygons);
                        }
                        r.top_shell_index = saved_top_idx;
                        r.bottom_shell_index = saved_bottom_idx;
                    }
                }
            }

            if !new_regions.is_empty() {
                working[i].regions = new_regions;
            }
        }
    }

    // ---- External-contour tagging (AC-22b bisector-edge dedup) ----------------
    //
    // Must run AFTER variant-composition writes working[i].regions (so the contour
    // reflects the final pre-erosion polygons) and BEFORE Phase 5 width-limiting
    // (which may clip or replace polygons). Per object, the union of the original
    // (pre-segmentation) slice polygons is the gap-free model perimeter; it is
    // attached to every painted cell so the guest can keep only the real perimeter
    // edges of each cell and skip paint-cell interfaces. `union_ex` is computed
    // here (host) because boolean polygon ops are unreliable in the WASM guest.
    bisector_ownership::populate_external_contours(&mut working, &slice_ir);

    // ---- Phase 5 — width limiting / interlocking (OrcaSlicer parity) ----------
    //
    // OrcaSlicer: `cut_segmented_layers` (MultiMaterialSegmentation.cpp:1294).
    // Guarded by `!interlocking_beam` inside `run_phase5_width_limit`.
    #[cfg(feature = "host-algos")]
    {
        run_phase5_width_limit(&mut working, &region_map)?;
    }

    // ---- Phase 6 — top/bottom propagation (OrcaSlicer parity) ----------------
    //
    // OrcaSlicer order (MultiMaterialSegmentation.cpp:1331-1419,
    // PrintObjectSlice.cpp:924-1081, MultiMaterialSegmentation.cpp:2053-2092):
    //   Phase 4 (colorize + cell decomposition) →
    //   Phase 6 (top/bottom propagation, NEW outputs per extruder) →
    //   Phase 7 merge (diff_ex BASE − phase6 + append/union into per-color regions).
    //
    // Shell-window propagation: a top-painted facet propagates DOWN by
    // `top_shell_layers` layers; a bottom-painted facet propagates UP by
    // `bottom_shell_layers` layers. At shells = 0 both windows collapse to the
    // single layer slab — i.e. intersection(top_proj[l] ∪ bot_proj[l],
    // layer_input_polygons[l]) — preserving the first-cut behaviour. Shell
    // counts are read from the BASE ResolvedConfig (configs[0]); if absent the
    // ResolvedConfig defaults (top=3, bottom=3, matching OrcaSlicer) apply.
    #[cfg(feature = "host-algos")]
    {
        use crate::polygon_ops::{difference_ex, intersection_ex, union_ex};
        use std::collections::BTreeMap;

        // Collect distinct (semantic, value) pairs present in the mesh. For
        // each pair build a painted-only IndexedTriangleSet that carries both
        // facet-painted triangles (vertex indices into the object's mesh) AND
        // stroke triangles (whose raw vertex coordinates are appended to the
        // subset's vertex pool with fresh contiguous indices).
        let mut painted_subsets: BTreeMap<
            (String, slicer_ir::PaintValue),
            (slicer_ir::PaintSemantic, slicer_ir::IndexedTriangleSet),
        > = BTreeMap::new();

        let sem_name = |s: &slicer_ir::PaintSemantic| -> String {
            match s {
                slicer_ir::PaintSemantic::Material => "material".to_owned(),
                slicer_ir::PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
                slicer_ir::PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
                slicer_ir::PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
                slicer_ir::PaintSemantic::Custom(name) => name.clone(),
            }
        };

        // The Phase 6 slab projection runs in WORLD space (it is intersected with
        // the world-space layer contours and sliced against world `layer_zs`), so
        // each subset stores world-transformed triangle vertices. The mesh and the
        // stroke geometry are in object-LOCAL space, so apply the object transform.
        for obj in &mesh.objects {
            let Some(pd) = &obj.paint_data else { continue };
            let m = &obj.transform.matrix;
            let push_tri = |entry: &mut slicer_ir::IndexedTriangleSet,
                            a: slicer_ir::Point3,
                            b: slicer_ir::Point3,
                            c: slicer_ir::Point3| {
                let base = entry.vertices.len() as u32;
                entry.vertices.push(crate::transform_point3(m, a));
                entry.vertices.push(crate::transform_point3(m, b));
                entry.vertices.push(crate::transform_point3(m, c));
                entry.indices.push(base);
                entry.indices.push(base + 1);
                entry.indices.push(base + 2);
            };
            let new_set = || slicer_ir::IndexedTriangleSet {
                vertices: Vec::new(),
                indices: Vec::new(),
            };
            for layer in &pd.layers {
                // Facet paint: emit each painted facet's three world vertices.
                for (facet_idx, fv) in layer.facet_values.iter().enumerate() {
                    // A facet with an explicit Material value uses it; a facet with
                    // NO value (genuinely unpainted) defaults to the object's base
                    // extruder (tool 0) so its top/bottom HORIZONTAL faces feed the
                    // default-colour top/bottom projection. Vertical unpainted facets
                    // (e.g. subdivided faces whose paint lives in `strokes`) are
                    // ignored by the slab projection, so this is safe. Non-Material
                    // layers have no default-fill semantics → skip their None facets.
                    let value = match fv {
                        Some(v) => v.clone(),
                        None => {
                            if layer.semantic != slicer_ir::PaintSemantic::Material {
                                continue;
                            }
                            slicer_ir::PaintValue::ToolIndex(0)
                        }
                    };
                    let base = facet_idx * 3;
                    if base + 2 >= obj.mesh.indices.len() {
                        continue;
                    }
                    let (i0, i1, i2) = (
                        obj.mesh.indices[base] as usize,
                        obj.mesh.indices[base + 1] as usize,
                        obj.mesh.indices[base + 2] as usize,
                    );
                    if i0 >= obj.mesh.vertices.len()
                        || i1 >= obj.mesh.vertices.len()
                        || i2 >= obj.mesh.vertices.len()
                    {
                        continue;
                    }
                    let key = (sem_name(&layer.semantic), value.clone());
                    let entry = painted_subsets
                        .entry(key)
                        .or_insert_with(|| (layer.semantic.clone(), new_set()));
                    push_tri(
                        &mut entry.1,
                        obj.mesh.vertices[i0],
                        obj.mesh.vertices[i1],
                        obj.mesh.vertices[i2],
                    );
                }
                // Stroke paint: world-transform each stroke triangle. Strokes carry
                // their own semantic/value (overriding the layer semantic when they
                // differ, matching `extract_stroke_data` in painted_line_collection prep).
                for stroke in &layer.strokes {
                    let key = (sem_name(&stroke.semantic), stroke.value.clone());
                    let entry = painted_subsets
                        .entry(key)
                        .or_insert_with(|| (stroke.semantic.clone(), new_set()));
                    for tri in &stroke.triangles {
                        push_tri(&mut entry.1, tri[0], tri[1], tri[2]);
                    }
                }
            }
        }

        // Shell-window counts come from the BASE ResolvedConfig (configs[0]).
        // RegionMapIR pre-seeds configs[0] with `ResolvedConfig::default()`, so
        // the fallback for missing keys is OrcaSlicer's default (top=3, bottom=3).
        let (top_shell_layers, bottom_shell_layers, shell_line_width, shell_layer_height): (
            usize,
            usize,
            f32,
            f32,
        ) = match region_map.configs.first() {
            Some(cfg) => (
                cfg.top_shell_layers as usize,
                cfg.bottom_shell_layers as usize,
                cfg.line_width,
                cfg.layer_height,
            ),
            // TODO: when per-object/per-region paint configs are wired through
            // execute_paint_segmentation, prefer the region-specific config
            // here instead of the BASE default.
            None => (3, 3, 0.4, 0.2),
        };

        if !painted_subsets.is_empty() && !working.is_empty() {
            // layer_zs already computed above for modifier volumes.
            // Per-layer BASE contours come from each layer's current BASE
            // SlicedRegion (the empty-variant_chain region produced by Phase 4
            // above). Fall back to a union over all regions if BASE is missing.
            // Full layer cross-section = union of ALL regions (BASE + painted).
            // Using only the BASE region is wrong here: Phase 4 progressively
            // diffs painted colours OUT of BASE, so at layers fully covered by
            // paint (e.g. the top cap) BASE is empty — which would collapse the
            // top/bottom shell propagation's running intersection to nothing.
            let layer_input_polygons: Vec<Vec<slicer_ir::ExPolygon>> = working
                .iter()
                .map(|s| {
                    let all: Vec<slicer_ir::ExPolygon> = s
                        .regions
                        .iter()
                        .flat_map(|r| r.polygons.iter().cloned())
                        .collect();
                    if all.is_empty() {
                        all
                    } else {
                        union_ex(&all)
                    }
                })
                .collect();

            // Run Phase 6 for each (semantic, value) and merge into working.
            for ((sname, value), (semantic, painted_mesh)) in &painted_subsets {
                if painted_mesh.indices.is_empty() {
                    continue;
                }
                let phase6 = top_bottom::propagate_top_bottom(
                    painted_mesh,
                    semantic.clone(),
                    value.clone(),
                    &layer_zs,
                    &layer_input_polygons,
                    top_shell_layers,
                    bottom_shell_layers,
                    shell_line_width,
                    shell_layer_height,
                );

                let chain_key: Vec<(String, slicer_ir::PaintValue)> =
                    vec![(sname.clone(), value.clone())];

                for (l, polys) in phase6.per_layer.iter().enumerate() {
                    if polys.is_empty() || l >= working.len() {
                        continue;
                    }
                    // Phase 7 merge:
                    //  (1) The top/bottom-face projection wins WITHIN its area over
                    //      every other region. Remove `polys` from each region that
                    //      is not the target colour (BASE and the vertical-side
                    //      colours) so the spatial tool lookup returns this colour
                    //      for the projected top/bottom solid surface. At the contact
                    //      (cap) layer the projection is full-area → side walls there
                    //      also take the face colour; at shell layers it is inset →
                    //      side walls keep their colour, only the inner solid fill flips.
                    //  (2) Harvest the BASE region's top/bottom SOLID FILL that falls
                    //      under the projection and hand it to the painted region, so
                    //      the surface is emitted under (and coloured by) this tool.
                    let mut top_clip: Vec<slicer_ir::ExPolygon> = Vec::new();
                    let mut bot_clip: Vec<slicer_ir::ExPolygon> = Vec::new();
                    for region in working[l].regions.iter_mut() {
                        if region.variant_chain == chain_key {
                            continue;
                        }
                        if region.variant_chain.is_empty() {
                            if !region.top_solid_fill.is_empty() {
                                top_clip = intersection_ex(&region.top_solid_fill, polys);
                                region.top_solid_fill =
                                    difference_ex(&region.top_solid_fill, polys);
                            }
                            if !region.bottom_solid_fill.is_empty() {
                                bot_clip = intersection_ex(&region.bottom_solid_fill, polys);
                                region.bottom_solid_fill =
                                    difference_ex(&region.bottom_solid_fill, polys);
                            }
                        }
                        region.polygons = difference_ex(&region.polygons, polys);
                    }

                    // (3) Append/union the projection into the per-colour SlicedRegion,
                    // carrying the harvested solid fill. Create one cloning the BASE's
                    // object_id / region_id if this colour has no region on the layer.
                    let existing_idx = working[l]
                        .regions
                        .iter()
                        .position(|r| r.variant_chain == chain_key);
                    match existing_idx {
                        Some(idx) => {
                            let region = &mut working[l].regions[idx];
                            let mut combined = region.polygons.clone();
                            combined.extend(polys.iter().cloned());
                            region.polygons = union_ex(&combined);
                            if !top_clip.is_empty() {
                                let mut t = region.top_solid_fill.clone();
                                t.extend(top_clip);
                                region.top_solid_fill = union_ex(&t);
                            }
                            if !bot_clip.is_empty() {
                                let mut b = region.bottom_solid_fill.clone();
                                b.extend(bot_clip);
                                region.bottom_solid_fill = union_ex(&b);
                            }
                        }
                        None => {
                            // Use first existing region as template for ids.
                            if let Some(template) = working[l].regions.first() {
                                let object_id = template.object_id.clone();
                                let region_id = template.region_id;
                                working[l].regions.push(slicer_ir::SlicedRegion {
                                    object_id,
                                    region_id,
                                    polygons: polys.clone(),
                                    variant_chain: chain_key.clone(),
                                    segment_annotations: std::collections::HashMap::new(),
                                    top_solid_fill: top_clip,
                                    bottom_solid_fill: bot_clip,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Arc::new(working))
}

// ---------------------------------------------------------------------------
// Step 10 helpers
// ---------------------------------------------------------------------------

/// Build `segment_annotations` for a BASE-chain `SlicedRegion` from the
/// pre-sliced modifier-volume polygons for this layer.
///
/// For each modifier-volume semantic present at this layer, we emit one
/// "perimeter" (outer Vec entry) with one segment per point-pair midpoint in
/// `chain_polygons`.  Each segment gets `Some(PaintValue::Flag(true))` when
/// its midpoint falls inside at least one modifier-volume polygon.
///
/// D14 invariant: callers MUST only call this for BASE chains
/// (`chain_key.is_empty() == true`).
fn build_modifier_segment_annotations(
    layer_idx: usize,
    chain_polygons: &[slicer_ir::ExPolygon],
    modifier_vol_per_layer: &[Vec<modifier_volumes::ModifierVolumeLayer>],
) -> std::collections::HashMap<slicer_ir::PaintSemantic, Vec<Vec<Option<slicer_ir::PaintValue>>>> {
    let mut annotations: std::collections::HashMap<
        slicer_ir::PaintSemantic,
        Vec<Vec<Option<slicer_ir::PaintValue>>>,
    > = std::collections::HashMap::new();

    let Some(mv_layers) = modifier_vol_per_layer.get(layer_idx) else {
        return annotations;
    };

    if mv_layers.is_empty() || chain_polygons.is_empty() {
        return annotations;
    }

    for mv_layer in mv_layers {
        // One "perimeter" per ExPolygon in the chain.
        let mut perimeters: Vec<Vec<Option<slicer_ir::PaintValue>>> = Vec::new();

        for exp in chain_polygons {
            let pts = &exp.contour.points;
            if pts.len() < 2 {
                perimeters.push(Vec::new());
                continue;
            }
            // One segment per edge; use the midpoint for classification.
            let n = pts.len();
            let mut segs: Vec<Option<slicer_ir::PaintValue>> = Vec::with_capacity(n);
            for k in 0..n {
                let a = pts[k];
                let b = pts[(k + 1) % n];
                let mid = slicer_ir::Point2 {
                    x: (a.x + b.x) / 2,
                    y: (a.y + b.y) / 2,
                };
                let inside =
                    modifier_volumes::any_expolygon_contains_point(&mv_layer.polygons, mid);
                segs.push(if inside {
                    Some(slicer_ir::PaintValue::Flag(true))
                } else {
                    None
                });
            }
            perimeters.push(segs);
        }

        annotations
            .entry(mv_layer.semantic.clone())
            .or_default()
            .extend(perimeters);
    }

    annotations
}

// ---------------------------------------------------------------------------
// Step 3 (WI-3): painted-chain annotation propagation
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Phase 5 width-limit integration helper
// ---------------------------------------------------------------------------

/// Read Phase 5 config from `region_map`, guard on `!interlocking_beam`, build
/// adapter slices, invoke [`width_limit::cut_segmented_layers`], and write the
/// eroded polygons back into `working`.
///
/// Returns `true` if the kernel was invoked, `false` if the beam guard or
/// zero-default short-circuit applied. The boolean is used only by AC-N3.
#[cfg(feature = "host-algos")]
fn run_phase5_width_limit(
    working: &mut [slicer_ir::SliceIR],
    region_map: &slicer_ir::RegionMapIR,
) -> Result<bool, PaintSegmentationError> {
    // Read MMU Phase 5 config from the first available RegionKey.
    // These parameters are global, not per-region. The map-empty case is
    // defensive — the driver already short-circuits on an empty map.
    let (width_units, depth_units, interlocking_beam) = match region_map.entries.keys().next() {
        Some(key) => {
            let cfg = region_map.config_for(key);
            (
                slicer_ir::mm_to_units(cfg.mmu_segmented_region_max_width),
                slicer_ir::mm_to_units(cfg.mmu_segmented_region_interlocking_depth),
                cfg.mmu_segmented_region_interlocking_beam,
            )
        }
        None => (0, 0, false),
    };

    // AC-2 / OrcaSlicer parity: beam=true skips Phase 5 entirely.
    if interlocking_beam {
        return Ok(false);
    }

    // AC-8: zero defaults — skip adapter work entirely (no mutation).
    if width_units == 0 && depth_units == 0 {
        return Ok(false);
    }

    // Build per-layer variant maps (painted chains only).
    let mut variants_per_layer = working
        .iter()
        .map(|s| {
            let mut map = std::collections::BTreeMap::new();
            for r in &s.regions {
                if r.variant_chain.is_empty() {
                    continue;
                }
                map.insert(r.variant_chain.clone(), r.polygons.clone());
            }
            map
        })
        .collect::<Vec<_>>();

    // Input geometry per layer: full union of all regions (BASE + painted).
    let input_expolygons_per_layer = working
        .iter()
        .map(|s| {
            s.regions
                .iter()
                .flat_map(|r| r.polygons.iter().cloned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // Invoke Phase 5 kernel (AC-4: all three config keys read above).
    width_limit::cut_segmented_layers(
        &mut variants_per_layer,
        &input_expolygons_per_layer,
        width_units,
        depth_units,
    )?;

    // Write-back: update painted region polygons (D15: empty result persists).
    for (i, layer_map) in variants_per_layer.iter().enumerate() {
        if i >= working.len() {
            break;
        }
        for region in &mut working[i].regions {
            if region.variant_chain.is_empty() {
                continue;
            }
            if let Some(polys) = layer_map.get(&region.variant_chain) {
                region.polygons = polys.clone();
            }
        }
    }

    Ok(true)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod driver_v2_tests {
    use super::*;
    use slicer_ir::{
        BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, FacetPaintData, IndexedTriangleSet,
        ModifierScope, ModifierVolume, ObjectConfig, ObjectMesh, PaintLayer, PaintSemantic,
        PaintValue, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, SliceIR,
        SlicedRegion, Transform3d, CURRENT_MESH_IR_SCHEMA_VERSION,
        CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    };
    use std::sync::Arc;

    fn identity_transform() -> Transform3d {
        Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    fn default_build_volume() -> BoundingBox3 {
        BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
        }
    }

    fn empty_mesh() -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: Vec::new(),
            build_volume: default_build_volume(),
        }
    }

    fn mesh_with_no_paint() -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.5,
                            y: 1.0,
                            z: 0.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: None,
                world_z_extent: None,
            }],
            build_volume: default_build_volume(),
        }
    }

    fn mesh_with_paint(value: PaintValue) -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: slicer_ir::IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.5,
                            y: 1.0,
                            z: 1.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: identity_transform(),
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: Some(FacetPaintData {
                    layers: vec![PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(value)],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: default_build_volume(),
        }
    }

    fn one_layer_slice_ir() -> Vec<SliceIR> {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };
        let region = SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: u(0.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(1.0),
                        },
                        Point2 {
                            x: u(0.0),
                            y: u(1.0),
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            ..Default::default()
        };
        vec![SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions: vec![region],
        }]
    }

    fn empty_region_map() -> RegionMapIR {
        RegionMapIR {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries: std::collections::HashMap::new(),
            configs: Vec::new(),
        }
    }

    fn region_map_with_base_entry() -> RegionMapIR {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj1".to_string(),
                region_id: 0u64,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj1".to_string(),
                region_id: 0u64,
                variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
            },
            RegionPlan::default(),
        );
        RegionMapIR {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries,
            configs: vec![slicer_ir::ResolvedConfig::default()],
        }
    }

    // ---- AC-N2 short-circuit tests ----------------------------------------

    /// AC-N2 (1): empty mesh → return input slice_ir unchanged.
    #[test]
    fn driver_v2_empty_mesh_returns_input_slice_ir() {
        let mesh = Arc::new(empty_mesh());
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice.clone(), rmap).unwrap();
        // Must be pointer-equal (same Arc content) or structurally equal.
        assert_eq!(
            result.len(),
            slice.len(),
            "short-circuit: length must match input"
        );
        assert_eq!(result[0].global_layer_index, 0);
        assert_eq!(result[0].regions.len(), slice[0].regions.len());
    }

    /// AC-N2 (2): mesh has objects but no PaintLayer has any Some/strokes → short-circuit.
    #[test]
    fn driver_v2_no_paint_data_short_circuits() {
        let mesh = Arc::new(mesh_with_no_paint());
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice.clone(), rmap).unwrap();
        assert_eq!(result.len(), slice.len());
        // Regions must be unchanged (short-circuit path, no mutation).
        assert_eq!(result[0].regions.len(), slice[0].regions.len());
        assert_eq!(result[0].regions[0].polygons, slice[0].regions[0].polygons);
    }

    /// AC-N2 (3): region_map.entries is empty → short-circuit.
    #[test]
    fn driver_v2_empty_region_map_short_circuits() {
        let mesh = Arc::new(mesh_with_paint(PaintValue::ToolIndex(1)));
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(empty_region_map());

        let result = execute_paint_segmentation(mesh, slice.clone(), rmap).unwrap();
        assert_eq!(result.len(), slice.len());
        assert_eq!(result[0].regions[0].polygons, slice[0].regions[0].polygons);
    }

    // ---- Full-pipeline tests (require kernel; #[ignore] for AC-12 stubs) ---

    /// AC-12 (b)(d): synthetic 1-layer SliceIR + 1-object MeshIR with 1 painted facet.
    /// Expected: result has ≥ 1 SlicedRegion; painted variant_chain entry is present.
    ///
    /// TODO: setting up a valid MMU_Graph from a single painted triangle that produces
    /// non-degenerate Voronoi regions requires carefully constructed geometry.
    /// Stubbed until the Step 11 prepass wiring validates end-to-end geometry.
    #[test]
    #[ignore = "TODO(step 11): requires non-degenerate painted triangle geometry for MMU_Graph"]
    fn driver_v2_synthetic_painted_facet_emits_sliced_region() {
        let mesh = Arc::new(mesh_with_paint(PaintValue::ToolIndex(1)));
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();
        assert_eq!(result.len(), 1);
        // AC-12(d): count == |base_regions| * |variant_chains| = 1 * 2 = 2.
        assert_eq!(
            result[0].regions.len(),
            2,
            "expected 2 regions: BASE + painted variant"
        );
        // AC-12(b): painted chain polygons must be non-empty.
        let painted = result[0]
            .regions
            .iter()
            .find(|r| r.variant_chain == vec![("material".to_string(), PaintValue::ToolIndex(1))]);
        assert!(painted.is_some(), "painted variant region must be present");
        assert!(
            !painted.unwrap().polygons.is_empty(),
            "painted variant polygons must not be empty"
        );
    }

    /// AC-12 (e): two variant chains should have disjoint polygon sets.
    ///
    /// TODO: requires same geometry fix as above.
    #[test]
    #[ignore = "TODO(step 11): requires non-degenerate painted geometry for disjointness check"]
    fn driver_v2_disjoint_variant_polygons() {
        use crate::polygon_ops::intersection_ex;

        let mesh = Arc::new(mesh_with_paint(PaintValue::ToolIndex(1)));
        let slice = Arc::new(one_layer_slice_ir());
        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();
        assert!(result[0].regions.len() >= 2);

        // All pairs of regions must have disjoint polygon sets.
        let regions = &result[0].regions;
        for i in 0..regions.len() {
            for j in (i + 1)..regions.len() {
                let overlap = intersection_ex(&regions[i].polygons, &regions[j].polygons);
                assert!(
                    overlap.is_empty(),
                    "regions[{i}] and regions[{j}] have overlapping polygons"
                );
            }
        }
    }

    // ---- Step 10 / D14 invariant test ----------------------------------------

    /// D14 invariant: modifier-volume SupportEnforcer annotations go to the BASE
    /// chain's `segment_annotations` only, NOT to painted variant chains.
    ///
    /// Geometry setup is non-trivial (requires a mesh that survives the Voronoi
    /// pipeline to produce both a BASE and a painted chain). Ignored until the
    /// Step 14 cube exercise provides concrete sliceable geometry.
    ///
    /// TODO(step 14): replace ignore with a real 1mm cube + SupportEnforcer modifier
    /// that produces both a BASE chain and a ToolIndex(1) chain, then assert
    /// base.segment_annotations[SupportEnforcer].is_non_empty() and
    /// painted.segment_annotations[SupportEnforcer].is_empty().
    #[test]
    #[ignore = "TODO(step 14): cube exercise needed for non-degenerate BASE+painted chain geometry"]
    fn driver_v2_routes_modifier_volume_to_base_segment_annotations_only() {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };

        // Build a 1×1×1 mm SupportEnforcer cube modifier volume.
        let mv_mesh = IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 1.0,
                },
                Point3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 1.0,
                },
            ],
            indices: vec![
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0,
                7, 3, 1, 2, 6, 1, 6, 5,
            ],
        };
        let mut mv_fields = std::collections::HashMap::new();
        mv_fields.insert(
            "subtype".to_string(),
            ConfigValue::String("support_enforcer".to_string()),
        );
        let mv = ModifierVolume {
            id: "mv1".to_string(),
            mesh: mv_mesh,
            config_delta: ConfigDelta { fields: mv_fields },
            priority: 0,
            applies_to: ModifierScope::AllFeatures,
        };

        // Build a mesh with the modifier volume AND a painted facet (ToolIndex(1)).
        let mesh = Arc::new(slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.5,
                            y: 1.0,
                            z: 1.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                transform: Transform3d {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                config: ObjectConfig {
                    data: std::collections::HashMap::new(),
                },
                modifier_volumes: vec![mv],
                paint_data: Some(FacetPaintData {
                    layers: vec![PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(PaintValue::ToolIndex(1))],
                        strokes: Vec::new(),
                    }],
                }),
                world_z_extent: None,
            }],
            build_volume: BoundingBox3 {
                min: Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                max: Point3 {
                    x: 250.0,
                    y: 210.0,
                    z: 220.0,
                },
            },
        });

        // SliceIR: one layer at z=0.5 with a BASE region and a painted variant region.
        let base_region = SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: u(0.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(0.0),
                        },
                        Point2 {
                            x: u(1.0),
                            y: u(1.0),
                        },
                        Point2 {
                            x: u(0.0),
                            y: u(1.0),
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            variant_chain: vec![],
            ..Default::default()
        };
        let painted_region = SlicedRegion {
            object_id: "obj1".to_string(),
            region_id: 0u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: u(0.2),
                            y: u(0.2),
                        },
                        Point2 {
                            x: u(0.8),
                            y: u(0.2),
                        },
                        Point2 {
                            x: u(0.8),
                            y: u(0.8),
                        },
                        Point2 {
                            x: u(0.2),
                            y: u(0.8),
                        },
                    ],
                },
                holes: Vec::new(),
            }],
            variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
            ..Default::default()
        };
        let slice = Arc::new(vec![SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions: vec![base_region, painted_region],
        }]);

        let rmap = Arc::new(region_map_with_base_entry());

        let result = execute_paint_segmentation(mesh, slice, rmap).unwrap();
        assert_eq!(result.len(), 1);

        // Find BASE and painted regions in the result.
        let base = result[0]
            .regions
            .iter()
            .find(|r| r.variant_chain.is_empty());
        let painted = result[0]
            .regions
            .iter()
            .find(|r| !r.variant_chain.is_empty());

        assert!(base.is_some(), "BASE chain region must exist");
        assert!(painted.is_some(), "painted chain region must exist");

        let base = base.unwrap();
        let painted = painted.unwrap();

        // D14 (a): BASE chain must have SupportEnforcer annotations (modifier-volume overlaps layer).
        let base_ann = base
            .segment_annotations
            .get(&PaintSemantic::SupportEnforcer);
        assert!(
            base_ann.is_some()
                && base_ann
                    .unwrap()
                    .iter()
                    .any(|p| p.iter().any(|s| s.is_some())),
            "BASE chain must have non-empty SupportEnforcer segment_annotations"
        );

        // D14 (b): painted chain must NOT have SupportEnforcer annotations.
        let painted_ann = painted
            .segment_annotations
            .get(&PaintSemantic::SupportEnforcer);
        assert!(
            painted_ann.is_none()
                || painted_ann
                    .unwrap()
                    .iter()
                    .all(|p| p.iter().all(|s| s.is_none())),
            "painted chain must NOT receive modifier-volume SupportEnforcer annotations (D14)"
        );
    }

    // ---- Phase 5 driver-level test (AC-N3) -----------------------------------

    /// AC-N3: `interlocking_beam = true` with nonzero width/depth → Phase 5 skipped.
    ///
    /// Proves that `run_phase5_width_limit` returns `Ok(false)` and leaves
    /// `working` unchanged when `mmu_segmented_region_interlocking_beam = true`.
    #[test]
    #[cfg(feature = "host-algos")]
    fn interlocking_beam_true_skips_phase5_driver() {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };

        // ResolvedConfig with beam=true and nonzero width/depth.
        let cfg = slicer_ir::ResolvedConfig {
            mmu_segmented_region_max_width: 2.0,
            mmu_segmented_region_interlocking_depth: 0.5,
            mmu_segmented_region_interlocking_beam: true,
            ..slicer_ir::ResolvedConfig::default()
        };

        // RegionMapIR with one painted-chain entry using the custom config.
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj1".to_string(),
                region_id: 0u64,
                variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
            },
            RegionPlan::default(), // ConfigId(0) resolves to `cfg` above
        );
        let region_map = slicer_ir::RegionMapIR {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries,
            configs: vec![cfg],
        };

        // One layer with one painted region.
        let painted_polys = vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: u(0.0),
                        y: u(0.0),
                    },
                    Point2 {
                        x: u(1.0),
                        y: u(0.0),
                    },
                    Point2 {
                        x: u(1.0),
                        y: u(1.0),
                    },
                    Point2 {
                        x: u(0.0),
                        y: u(1.0),
                    },
                ],
            },
            holes: Vec::new(),
        }];
        let mut working = vec![SliceIR {
            schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.5,
            regions: vec![SlicedRegion {
                object_id: "obj1".to_string(),
                region_id: 1u64,
                polygons: painted_polys.clone(),
                variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
                ..Default::default()
            }],
        }];
        let working_snapshot = working.clone();

        // Phase 5 must be skipped when beam=true.
        let invoked = super::run_phase5_width_limit(&mut working, &region_map)
            .expect("run_phase5_width_limit must not error");
        assert!(!invoked, "beam=true must skip Phase 5 (return false)");

        // working must be byte-for-byte identical to the pre-call snapshot.
        assert_eq!(
            working, working_snapshot,
            "beam=true must leave working unmodified"
        );
    }
}

// ---------------------------------------------------------------------------
// Arc-walk tiling tests (host-algos only)
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "host-algos"))]
mod arc_walk_tiling_tests {
    use super::*;
    use crate::algos::paint_segmentation::{
        colorize::ColoredLine, extract_segments, triangle_intersect::Line,
        voronoi_graph::MMU_Graph, voronoi_prune,
    };
    use slicer_ir::{PaintValue, Point2};

    /// AC-4: arc-walk decomposition of a fully-painted 4-colour face tiles without
    /// leaving a gap wider than one extrusion width (~0.5 mm; reject > 0.6 mm),
    /// AND produces true area-covering polygons (not degenerate boundary slivers).
    ///
    /// Geometry: a 250 000 × 250 000 unit square (~25 mm × 25 mm at 100 nm/unit)
    /// divided into 8 boundary segments (2 per side), mirroring the left-face
    /// geometry of cube_4color.3mf where 4 painted circles interrupt the red base.
    ///
    /// Colour assignment: [0,1,2,3,3,2,1,0] (symmetric). Each colour appears on at
    /// least one forward-traversed arc so the seed-colour test is reliable even though
    /// the arc-walk's backward-traversal of the closing arc N-1→0 produces a
    /// degenerate zero-length repair chord (a known property of the current graph's
    /// closed-ring topology; the REAL segment is still emitted and counted in coverage).
    ///
    /// Checks:
    ///   1. All 4 tool colours appear as walk seed colours.
    ///   2. No residual (`None`-keyed) polygon has bounding-box width > 0.6 mm (6 000 units).
    ///   3. The total arc-length covered by real (non-repair) segments equals the
    ///      full perimeter within the gap tolerance (6 000 units).
    ///   4. (Falsifiable area gate) The total area of all non-None colour polygons
    ///      is ≥ 90% of the input square area. Before the from_colored_lines fix the
    ///      arc-walk produced only boundary slivers (area ≈ 0) and would fail here.
    #[test]
    fn cube_4color_left_face_circles_tile_without_gap() {
        let x_min: i64 = 1_125_000;
        let y_min: i64 = 925_000;
        let x_max: i64 = 1_375_000;
        let y_max: i64 = 1_175_000;
        let x_mid: i64 = (x_min + x_max) / 2;
        let y_mid: i64 = (y_min + y_max) / 2;

        // 8 boundary segments, 2 per side.  Colours [0,1,2,3,3,2,1,0] ensure each
        // colour has at least one forward-traversed arc (avoiding the degenerate
        // closed-ring backward-traversal issue for arc 7→0).
        let colored_lines: Vec<ColoredLine> = vec![
            // Bottom-left half: ToolIndex(0)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_min, y: y_min },
                    end: Point2 { x: x_mid, y: y_min },
                },
                value: Some(PaintValue::ToolIndex(0)),
                poly_idx: 0,
                local_line_idx: 0,
            },
            // Bottom-right half: ToolIndex(1)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_mid, y: y_min },
                    end: Point2 { x: x_max, y: y_min },
                },
                value: Some(PaintValue::ToolIndex(1)),
                poly_idx: 0,
                local_line_idx: 1,
            },
            // Right-bottom half: ToolIndex(2)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_max, y: y_min },
                    end: Point2 { x: x_max, y: y_mid },
                },
                value: Some(PaintValue::ToolIndex(2)),
                poly_idx: 0,
                local_line_idx: 2,
            },
            // Right-top half: ToolIndex(3)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_max, y: y_mid },
                    end: Point2 { x: x_max, y: y_max },
                },
                value: Some(PaintValue::ToolIndex(3)),
                poly_idx: 0,
                local_line_idx: 3,
            },
            // Top-right half: ToolIndex(3)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_max, y: y_max },
                    end: Point2 { x: x_mid, y: y_max },
                },
                value: Some(PaintValue::ToolIndex(3)),
                poly_idx: 0,
                local_line_idx: 4,
            },
            // Top-left half: ToolIndex(2)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_mid, y: y_max },
                    end: Point2 { x: x_min, y: y_max },
                },
                value: Some(PaintValue::ToolIndex(2)),
                poly_idx: 0,
                local_line_idx: 5,
            },
            // Left-top half: ToolIndex(1)
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_min, y: y_max },
                    end: Point2 { x: x_min, y: y_mid },
                },
                value: Some(PaintValue::ToolIndex(1)),
                poly_idx: 0,
                local_line_idx: 6,
            },
            // Left-bottom half: ToolIndex(0)  [arc 7, seeded backward from node 0 — known degenerate walk;
            // ToolIndex(0) is still present via arc 0's forward walk]
            ColoredLine {
                line: Line {
                    start: Point2 { x: x_min, y: y_mid },
                    end: Point2 { x: x_min, y: y_min },
                },
                value: Some(PaintValue::ToolIndex(0)),
                poly_idx: 0,
                local_line_idx: 7,
            },
        ];

        let mut graph =
            MMU_Graph::from_colored_lines(&colored_lines).expect("graph construction must succeed");
        voronoi_prune::remove_multiple_edges_in_vertices(&mut graph, &[colored_lines.clone()]);
        voronoi_prune::remove_nodes_with_one_arc(&mut graph);

        let segments = extract_segments::extract_colored_segments(&graph, 4);
        assert!(
            !segments.is_empty(),
            "arc-walk must produce at least one segment for a fully-painted face"
        );

        let result = segments_to_expolygons_by_color(&segments);
        assert!(
            !result.is_empty(),
            "arc-walk decomposition must produce at least one coloured region"
        );

        // ---- Check 1: all 4 tool colours appear as walk seed colours ----
        for tool_idx in 0..4u32 {
            assert!(
                result.contains_key(&Some(PaintValue::ToolIndex(tool_idx))),
                "ToolIndex({tool_idx}) missing from arc-walk decomposition output"
            );
        }

        // ---- Check 2: no large residual (None-keyed) polygon ----
        // A residual polygon with bounding-box extent > 6 000 units (0.6 mm) represents
        // a gap too wide to ignore. With a fully-painted face (all 4 sides coloured), no
        // `None`-keyed polygon should appear at all.
        let gap_threshold_units: i64 = 6_000; // 0.6 mm
        if let Some(residual) = result.get(&None) {
            for exp in residual {
                let pts = &exp.contour.points;
                if pts.is_empty() {
                    continue;
                }
                let x_extent = pts.iter().map(|p| p.x).max().unwrap_or(0)
                    - pts.iter().map(|p| p.x).min().unwrap_or(0);
                let y_extent = pts.iter().map(|p| p.y).max().unwrap_or(0)
                    - pts.iter().map(|p| p.y).min().unwrap_or(0);
                assert!(
                    x_extent <= gap_threshold_units && y_extent <= gap_threshold_units,
                    "residual (unassigned) polygon bounding box {x_extent}×{y_extent} units \
                     exceeds the 0.6 mm gap threshold ({gap_threshold_units} units) — \
                     arc-walk left a gap"
                );
            }
        }

        // ---- Check 3: boundary-coverage (no arc left unrepresented) ----
        // Real segments (arc_idx.is_some()) must sum to ≥ (perimeter − 6 000 units).
        // For this 4-side square each side is 250 000 units; perimeter = 1 000 000 units.
        let total_perimeter: i64 = (x_max - x_min) * 2 + (y_max - y_min) * 2;
        let covered_length: i64 = segments
            .iter()
            .filter(|s| s.arc_idx.is_some())
            .map(|s| {
                let dx = (s.line.end.x - s.line.start.x) as f64;
                let dy = (s.line.end.y - s.line.start.y) as f64;
                (dx * dx + dy * dy).sqrt() as i64
            })
            .sum();
        assert!(
            covered_length >= total_perimeter - gap_threshold_units,
            "arc-walk boundary coverage {covered_length} units < perimeter {total_perimeter} units \
             − tolerance {gap_threshold_units} — a gap exists in the walk"
        );

        // ---- Check 4: area coverage (falsifiable sliver guard) ----
        // Before the from_colored_lines connectivity fix the walk produced boundary
        // slivers with area ≈ 0. After the fix, NonBorder bisector arcs connect border
        // nodes to interior Voronoi vertices, enabling closed loops that enclose real area.
        // Total area of all non-None colour polygons must be ≥ 90% of the input square.
        let total_sq_area = ((x_max - x_min) as f64) * ((y_max - y_min) as f64);
        fn polygon_area_abs(pts: &[Point2]) -> f64 {
            if pts.len() < 3 {
                return 0.0;
            }
            let mut acc: i128 = 0;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                acc += (pts[i].x as i128) * (pts[j].y as i128)
                    - (pts[j].x as i128) * (pts[i].y as i128);
            }
            (acc as f64).abs() * 0.5
        }
        let covered_area: f64 = result
            .iter()
            .filter(|(k, _)| k.is_some())
            .flat_map(|(_, polys)| polys.iter())
            .map(|exp| polygon_area_abs(&exp.contour.points))
            .sum();
        assert!(
            covered_area >= 0.9 * total_sq_area,
            "arc-walk covered area {covered_area:.0} units² < 90% of square area \
             {total_sq_area:.0} units² — walk produced slivers instead of area polygons \
             (from_colored_lines connectivity fix missing or incomplete)"
        );
    }
}
