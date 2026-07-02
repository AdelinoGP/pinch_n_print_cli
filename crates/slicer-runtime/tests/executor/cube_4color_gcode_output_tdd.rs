//! AC-22 RED bucket — gcode-output behavior gate for `cube_4color.3mf` and
//! `cube_fuzzyPainted.3mf`.
//!
//! These tests assert the *correct* end-to-end gcode behavior that the
//! Step 19 D9 dispatch wiring is expected to produce. They land RED on
//! today's code because the diagnose session traced two regressions:
//!
//!   1. `^T[0-9]+$` lines in the emitted gcode only contain `{0, 1}` — the
//!      `T2` and `T3` tool-change records are missing for the 4-color cube.
//!      Tests assert the full set `{0, 1, 2, 3}` per the four Material paint
//!      values present on the fixture.
//!
//!   2. Per-layer outer-wall block count is 4-9 instead of ~2: extra
//!      phantom perimeters are emitted along Voronoi cell boundaries.
//!      Tests assert the painted cube's per-layer `;TYPE:Outer wall` count
//!      stays within ±1 of an unpainted baseline cube sliced with identical
//!      settings.
//!
//! A third (Test 3) asserts that the painted face of `cube_fuzzyPainted.3mf`
//! produces visibly higher coordinate jitter on its outer-wall point sequence
//! than an unpainted face — i.e. the fuzzy module ran at all on the painted
//! side. This is a coarse, loose-but-clear ratio that may need tightening
//! once D9 dispatch is verified to route through the fuzzy module via the new
//! variant-chain.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};
use slicer_runtime::{run_slice, SliceOutcome, SliceRunOptions};

// --------------------------------------------------------------------------
// Workspace + fixture helpers
// --------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn cube_4color_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn cube_fuzzy_painted_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("cube_fuzzyPainted.3mf")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Point3 {
            x: 250.0,
            y: 250.0,
            z: 250.0,
        },
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Build a synthetic 25mm axis-aligned cube with NO paint_data. Used as the
/// unpainted baseline for the per-layer outer-wall count comparison. The
/// cube is centered horizontally near (125, 105, 12.5) — same general world
/// position as the cube_4color fixture (after its build transform) so the
/// pipeline routes through identical layer indices and config inheritance.
fn unpainted_25mm_cube() -> Arc<MeshIR> {
    // Match cube_4color's world-space footprint (25mm side, centered at
    // approximately (125, 105, 12.5)) so layer-height-derived layer counts
    // align.
    let cx = 125.0_f32;
    let cy = 105.0_f32;
    let z_min = 0.0_f32;
    let z_max = 25.0_f32;
    let half = 12.5_f32;
    let x_min = cx - half;
    let x_max = cx + half;
    let y_min = cy - half;
    let y_max = cy + half;

    let p = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        p(x_min, y_min, z_min), // 0
        p(x_max, y_min, z_min), // 1
        p(x_max, y_max, z_min), // 2
        p(x_min, y_max, z_min), // 3
        p(x_min, y_min, z_max), // 4
        p(x_max, y_min, z_max), // 5
        p(x_max, y_max, z_max), // 6
        p(x_min, y_max, z_max), // 7
    ];
    let indices: Vec<u32> = vec![
        // bottom (-Z)
        0, 2, 1, 0, 3, 2, // top (+Z)
        4, 5, 6, 4, 6, 7, // front (-Y)
        0, 1, 5, 0, 5, 4, // back (+Y)
        2, 3, 7, 2, 7, 6, // left (-X)
        3, 0, 4, 3, 4, 7, // right (+X)
        1, 2, 6, 1, 6, 5,
    ];

    let object = ObjectMesh {
        id: "unpainted_25mm_cube".to_string(),
        mesh: IndexedTriangleSet { vertices, indices },
        transform: identity_transform(),
        config: ObjectConfig {
            data: Default::default(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        // Seed the planner-visible world height (run.rs reads this and injects
        // `object_height:<id>` into the config map before module dispatch).
        world_z_extent: Some((z_min, z_max)),
    };

    Arc::new(MeshIR {
        schema_version: semver(),
        objects: vec![object],
        build_volume: build_volume(),
    })
}

// --------------------------------------------------------------------------
// Slice harness
// --------------------------------------------------------------------------

/// Run an end-to-end slice via the library entry point (`run_slice`) and
/// return the produced gcode. Loads the named fixture from `resources/`
/// and wires through `modules/core-modules`. Panics with a clear message
/// on any failure — the AC-22 contract treats a missing fixture or pipeline
/// crash as test infra failure, not assertion failure.
fn slice_fixture_file(model_path: &PathBuf) -> SliceOutcome {
    assert!(
        model_path.exists(),
        "fixture missing: {} — run from workspace root or restore resources/",
        model_path.display()
    );
    let module_dir = core_modules_dir();
    assert!(
        module_dir.exists(),
        "core-modules directory must exist at {}",
        module_dir.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(model_path)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model_path.display())),
    );
    let opts = SliceRunOptions {
        mesh,
        model_label: model_path.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        config_overrides: std::collections::HashMap::new(),
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed against {}: {e}", model_path.display()))
}

/// Same as [`slice_fixture_file`] but for a pre-constructed `MeshIR` (used
/// by the unpainted-baseline test).
fn slice_synthetic_mesh(label: &str, mesh: Arc<MeshIR>) -> SliceOutcome {
    let module_dir = core_modules_dir();
    let opts = SliceRunOptions {
        mesh,
        model_label: label.to_string(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
        config_overrides: std::collections::HashMap::new(),
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed against synthetic mesh {label}: {e}"))
}

// --------------------------------------------------------------------------
// Gcode parsing helpers
// --------------------------------------------------------------------------

/// Match a tool-change line of the form `T<digits>` (and only `T<digits>`,
/// possibly with trailing whitespace). Used by Test 1.
fn parse_tool_index_lines(gcode: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.len() < 2 {
            continue;
        }
        let bytes = trimmed.as_bytes();
        if bytes[0] != b'T' {
            continue;
        }
        if !bytes[1..].iter().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(n) = trimmed[1..].parse::<u32>() {
            out.insert(n);
        }
    }
    out
}

/// Split the gcode by `;LAYER_CHANGE` markers and count the number of
/// distinct outer-wall extrusion runs per layer. Each `;TYPE:Outer wall`
/// section can contain multiple disjoint extrusion runs (separated by
/// non-extrusion travel moves or by another `;TYPE:*` marker re-entering
/// outer-wall later). Counting the EXTRUSION MOVES (`G1 ... E<n>`) inside
/// outer-wall blocks captures the regression precisely: the painted cube
/// emits 7-23 extrusion moves per outer-wall layer (extra phantom perimeters
/// along Voronoi cell boundaries) where the unpainted baseline emits ~4
/// (one closed loop = 4 segments for a square).
fn outer_wall_counts_per_layer(gcode: &str) -> Vec<usize> {
    let marker = ";LAYER_CHANGE";
    let outer = ";TYPE:Outer wall";
    let mut counts = Vec::new();
    let mut current = 0usize;
    let mut layer_started = false;
    let mut in_outer = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                counts.push(current);
            }
            current = 0;
            layer_started = true;
            in_outer = false;
            continue;
        }
        if !layer_started {
            continue;
        }
        if trimmed.starts_with(outer) {
            in_outer = true;
            continue;
        }
        if trimmed.starts_with(";TYPE:") {
            in_outer = false;
            continue;
        }
        if !in_outer {
            continue;
        }
        // Outer-wall EXTRUSION SEGMENT: a G1 that moves in XY while extruding.
        // E-only moves (filament retract / unretract / static wipe) are NOT wall
        // segments and must be excluded — otherwise a retract that lands inside
        // the outer-wall block inflates the count by the retract/unretract pair.
        // This counts equally for painted and unpainted gcode, so the baseline
        // comparison stays apples-to-apples.
        let is_extrusion_segment = (trimmed.starts_with("G1 ") || trimmed.starts_with("G1\t"))
            && trimmed.contains(" E")
            && (trimmed.contains(" X") || trimmed.contains(" Y"));
        if is_extrusion_segment {
            current += 1;
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

/// Count the number of `;TYPE:Outer wall` header occurrences per layer bucket
/// (separated by `;LAYER_CHANGE` markers). Each per-color outer-wall fragment
/// begins a fresh `;TYPE:Outer wall` block (after a travel move or tool change),
/// so this directly measures per-color fragmentation — NOT G1 segment count.
///
/// An unpainted cube should have exactly 1 fragment per mid-body layer.
/// A 4-color painted cube should have >= 2 fragments on painted layers.
fn outer_wall_fragments_per_layer(gcode: &str) -> Vec<usize> {
    let marker = ";LAYER_CHANGE";
    let outer = ";TYPE:Outer wall";
    let mut counts = Vec::new();
    let mut current = 0usize;
    let mut layer_started = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                counts.push(current);
            }
            current = 0;
            layer_started = true;
            continue;
        }
        if !layer_started {
            continue;
        }
        // Each `;TYPE:Outer wall` header starts a new per-color outer-wall fragment.
        if trimmed == outer {
            current += 1;
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

// --------------------------------------------------------------------------
// Model A loop-level parser (ADR-0013)
// --------------------------------------------------------------------------
//
// A single `;TYPE:Outer wall` block may contain MULTIPLE independent closed
// loops (a per-color cell whose contour was traced, plus any disjoint pieces of
// the same colour, plus the seam re-approach). Treating a whole outer-wall block
// as one polyline mis-computes closure — the "first-to-last" gap then spans two
// separate loops. This parser splits each block into its constituent loops so
// each is closure-checked independently.

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// One closed outer-wall loop. `pts[0]` is the seam/approach point (the
/// non-extruding move that positions the nozzle); `pts[1..]` are the extrusion
/// endpoints in trace order. A closed loop returns its final extrusion point to
/// `pts[0]`.
#[derive(Clone)]
struct OuterLoop {
    tool: u32,
    /// 0-based index of the `;TYPE:Outer wall` header (within its layer) this
    /// loop was emitted under. Loops sharing a header share a tool + travel-in.
    header_idx: usize,
    pts: Vec<(f32, f32)>,
}

impl OuterLoop {
    /// Distance from the seam/approach point to the final extrusion point.
    /// Small ⇒ the loop closes.
    fn closure_gap(&self) -> f32 {
        match (self.pts.first(), self.pts.last()) {
            (Some(a), Some(b)) if self.pts.len() >= 2 => dist(*a, *b),
            _ => f32::INFINITY,
        }
    }
}

/// Parse a `G0`/`G1` move line into `(x, y, has_e)`. Missing X/Y are `None` so
/// callers carry forward the last known coordinate. `has_e` marks an extrusion.
fn parse_move(trimmed: &str) -> Option<(Option<f32>, Option<f32>, bool)> {
    let is_move = trimmed.starts_with("G1 ")
        || trimmed.starts_with("G1\t")
        || trimmed.starts_with("G0 ")
        || trimmed.starts_with("G0\t");
    if !is_move {
        return None;
    }
    let mut x = None;
    let mut y = None;
    let mut has_e = false;
    for tok in trimmed.split_whitespace() {
        if let Some(r) = tok.strip_prefix('X') {
            x = r.parse::<f32>().ok();
        } else if let Some(r) = tok.strip_prefix('Y') {
            y = r.parse::<f32>().ok();
        } else if let Some(r) = tok.strip_prefix('E') {
            // Only count a POSITIVE extrusion with XY motion as a wall segment;
            // retract/unretract (E-only, or negative E) is not a wall vertex.
            if let Ok(e) = r.parse::<f32>() {
                if e > 0.0 {
                    has_e = true;
                }
            }
        }
    }
    if x.is_none() && y.is_none() {
        return None;
    }
    Some((x, y, has_e))
}

/// Split every layer's outer-wall blocks into independent loops, each tagged with
/// its tool and header index. Returns one `Vec<OuterLoop>` per layer bucket.
fn parse_outer_wall_loops_per_layer(gcode: &str) -> Vec<Vec<OuterLoop>> {
    let marker = ";LAYER_CHANGE";
    let outer = ";TYPE:Outer wall";
    let mut layers: Vec<Vec<OuterLoop>> = Vec::new();
    let mut current: Vec<OuterLoop> = Vec::new();
    let mut layer_started = false;
    let mut in_outer = false;
    let mut tool: u32 = 0;
    let mut header_idx: usize = 0;
    let mut seen_outer_this_layer = false;
    let mut pos: (f32, f32) = (0.0, 0.0);
    let mut cur: Option<OuterLoop> = None;

    // Flush the in-progress loop if it carries at least one extrusion segment.
    fn flush(cur: &mut Option<OuterLoop>, out: &mut Vec<OuterLoop>) {
        if let Some(l) = cur.take() {
            if l.pts.len() >= 2 {
                out.push(l);
            }
        }
    }

    for line in gcode.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with(marker) {
            flush(&mut cur, &mut current);
            if layer_started {
                layers.push(std::mem::take(&mut current));
            }
            layer_started = true;
            in_outer = false;
            header_idx = 0;
            seen_outer_this_layer = false;
            continue;
        }
        if !layer_started {
            // still parse tool changes before the first layer for continuity
            if is_tool_line(trimmed) {
                tool = trimmed[1..].parse::<u32>().unwrap_or(tool);
            }
            continue;
        }

        // Tool changes can appear anywhere; keep `tool` current.
        if is_tool_line(trimmed) {
            flush(&mut cur, &mut current);
            tool = trimmed[1..].parse::<u32>().unwrap_or(tool);
            continue;
        }

        if trimmed == outer {
            flush(&mut cur, &mut current);
            if seen_outer_this_layer {
                header_idx += 1;
            }
            seen_outer_this_layer = true;
            in_outer = true;
            continue;
        }
        if trimmed.starts_with(";TYPE:") || trimmed.starts_with(";LAYER") {
            flush(&mut cur, &mut current);
            in_outer = false;
            continue;
        }

        if let Some((mx, my, has_e)) = parse_move(trimmed) {
            if let Some(x) = mx {
                pos.0 = x;
            }
            if let Some(y) = my {
                pos.1 = y;
            }
            if !in_outer {
                continue;
            }
            if has_e {
                // Extrusion segment: append to the current loop (starting one if
                // an extrusion somehow precedes its approach move).
                match cur.as_mut() {
                    Some(l) => l.pts.push(pos),
                    None => {
                        cur = Some(OuterLoop {
                            tool,
                            header_idx,
                            pts: vec![pos],
                        })
                    }
                }
            } else {
                // Non-extruding move = seam/approach. Starts a new loop unless the
                // current loop has no extrusion yet (consecutive approaches), in
                // which case just update its seam position.
                match cur.as_mut() {
                    Some(l) if l.pts.len() >= 2 => {
                        flush(&mut cur, &mut current);
                        cur = Some(OuterLoop {
                            tool,
                            header_idx,
                            pts: vec![pos],
                        });
                    }
                    Some(l) => {
                        l.tool = tool;
                        l.header_idx = header_idx;
                        l.pts = vec![pos];
                    }
                    None => {
                        cur = Some(OuterLoop {
                            tool,
                            header_idx,
                            pts: vec![pos],
                        })
                    }
                }
            }
        }
    }
    flush(&mut cur, &mut current);
    if layer_started {
        layers.push(current);
    }
    layers
}

fn is_tool_line(trimmed: &str) -> bool {
    trimmed.len() >= 2
        && trimmed.as_bytes()[0] == b'T'
        && trimmed.as_bytes()[1..].iter().all(|c| c.is_ascii_digit())
}

/// Axis-aligned bounding box of a layer's outer-wall extrusion points. Its four
/// edges are the layer's (inward-offset) external silhouette; a loop point lying
/// on an edge is tracing that silhouette arc, whereas an interior point (off all
/// four edges, toward centre) is a bisector wall between adjacent per-color cells.
struct SilhouetteBox {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

/// Which silhouette edge a point lies on (a corner point may report two).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum SilEdge {
    Left,
    Right,
    Front,
    Back,
}

impl SilhouetteBox {
    fn from_loops(layer: &[OuterLoop]) -> Self {
        let mut x_min = f32::INFINITY;
        let mut x_max = f32::NEG_INFINITY;
        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        for lp in layer {
            for &(x, y) in lp.pts.iter().skip(1) {
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
            }
        }
        SilhouetteBox {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    /// The constant coordinate of an edge's line (x for Left/Right, y for Front/Back).
    fn edge_coord(&self, edge: SilEdge) -> f32 {
        match edge {
            SilEdge::Left => self.x_min,
            SilEdge::Right => self.x_max,
            SilEdge::Front => self.y_min,
            SilEdge::Back => self.y_max,
        }
    }

    /// The along-edge extent (the perpendicular axis range).
    fn edge_extent(&self, edge: SilEdge) -> (f32, f32) {
        match edge {
            SilEdge::Left | SilEdge::Right => (self.y_min, self.y_max),
            SilEdge::Front | SilEdge::Back => (self.x_min, self.x_max),
        }
    }

    /// Project a point onto the edge's along-axis coordinate.
    fn proj(&self, edge: SilEdge, p: (f32, f32)) -> f32 {
        match edge {
            SilEdge::Left | SilEdge::Right => p.1,
            SilEdge::Front | SilEdge::Back => p.0,
        }
    }

    /// Is the point within `eps` of this edge's line?
    fn point_on(&self, edge: SilEdge, p: (f32, f32), eps: f32) -> bool {
        let c = self.edge_coord(edge);
        match edge {
            SilEdge::Left | SilEdge::Right => (p.0 - c).abs() <= eps,
            SilEdge::Front | SilEdge::Back => (p.1 - c).abs() <= eps,
        }
    }
}

/// Collect the along-edge intervals `(lo, hi, tool)` of every wall SEGMENT that
/// lies on `edge` (both endpoints within `eps` of the edge line). These are the
/// silhouette arcs traced by the per-color loops.
fn edge_intervals(
    layer: &[OuterLoop],
    sil: &SilhouetteBox,
    edge: SilEdge,
    eps: f32,
) -> Vec<(f32, f32, u32)> {
    let mut out = Vec::new();
    for lp in layer {
        // Segments are consecutive point pairs, including the closing pair
        // (last -> first) since the loop is closed.
        let n = lp.pts.len();
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let a = lp.pts[i];
            let b = lp.pts[(i + 1) % n];
            if sil.point_on(edge, a, eps) && sil.point_on(edge, b, eps) {
                let pa = sil.proj(edge, a);
                let pb = sil.proj(edge, b);
                out.push((pa.min(pb), pa.max(pb), lp.tool));
            }
        }
    }
    out.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
    out
}

/// Coverage report over an edge: `(max_gap, max_overlap, covered_len, extent_len)`
/// on the trimmed extent (corner margin `trim` removed at both ends).
fn coverage_report(
    intervals: &[(f32, f32, u32)],
    extent: (f32, f32),
    trim: f32,
) -> (f32, f32, f32, f32) {
    let lo = extent.0 + trim;
    let hi = extent.1 - trim;
    if hi <= lo {
        return (0.0, 0.0, 0.0, 0.0);
    }
    // Clip intervals to [lo, hi].
    let mut clipped: Vec<(f32, f32)> = intervals
        .iter()
        .map(|&(a, b, _)| (a.max(lo), b.min(hi)))
        .filter(|&(a, b)| b > a)
        .collect();
    clipped.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());

    // Overlap = sum of pairwise overlaps of adjacent (by start) intervals.
    let mut max_overlap = 0.0_f32;
    for w in clipped.windows(2) {
        let ov = w[0].1 - w[1].0;
        if ov > max_overlap {
            max_overlap = ov;
        }
    }
    // Merge to compute covered length and gaps.
    let mut covered = 0.0_f32;
    let mut max_gap = 0.0_f32;
    let mut cur_lo = lo;
    let mut cursor = lo;
    let mut started = false;
    for &(a, b) in &clipped {
        if !started {
            if a > cursor {
                max_gap = max_gap.max(a - cursor);
            }
            cur_lo = a.max(cursor);
            cursor = b.max(cur_lo);
            started = true;
            continue;
        }
        if a > cursor {
            // gap
            max_gap = max_gap.max(a - cursor);
            covered += cursor - cur_lo;
            cur_lo = a;
            cursor = b;
        } else {
            cursor = cursor.max(b);
        }
    }
    if started {
        covered += cursor - cur_lo;
        if hi > cursor {
            max_gap = max_gap.max(hi - cursor);
        }
    } else {
        max_gap = hi - lo;
    }
    (max_gap, max_overlap, covered, hi - lo)
}

/// Count the number of distinct `T<digits>` tool indices appearing per layer
/// (separated by `;LAYER_CHANGE` markers). Returns one entry per layer.
fn distinct_tool_indices_per_layer(gcode: &str) -> Vec<usize> {
    use std::collections::HashSet;
    let marker = ";LAYER_CHANGE";
    let mut result: Vec<usize> = Vec::new();
    let mut current: HashSet<u32> = HashSet::new();
    let mut layer_started = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                result.push(current.len());
            }
            current.clear();
            layer_started = true;
            continue;
        }
        if !layer_started {
            continue;
        }
        // Detect bare `T<digits>` lines.
        if trimmed.len() >= 2 {
            let bytes = trimmed.as_bytes();
            if bytes[0] == b'T' && bytes[1..].iter().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = trimmed[1..].parse::<u32>() {
                    current.insert(n);
                }
            }
        }
    }
    if layer_started {
        result.push(current.len());
    }
    result
}

/// Count the number of `T<digits>` tool-change lines per layer. Returns one
/// entry per layer (same index convention as `outer_wall_counts_per_layer`).
fn tool_changes_per_layer(gcode: &str) -> Vec<usize> {
    let marker = ";LAYER_CHANGE";
    let mut counts: Vec<usize> = Vec::new();
    let mut current = 0usize;
    let mut layer_started = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                counts.push(current);
            }
            current = 0;
            layer_started = true;
            continue;
        }
        if !layer_started {
            continue;
        }
        // Detect bare `T<digits>` lines (same logic as parse_tool_index_lines).
        if trimmed.len() >= 2 {
            let bytes = trimmed.as_bytes();
            if bytes[0] == b'T' && bytes[1..].iter().all(|c| c.is_ascii_digit()) {
                current += 1;
            }
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

/// Format a per-layer count diff for the assertion message (first N layers).
fn fmt_per_layer_diff(painted: &[usize], unpainted: &[usize], n: usize) -> String {
    let len = painted.len().min(unpainted.len()).min(n);
    let mut out = String::new();
    out.push_str("layer painted unpainted diff\n");
    for i in 0..len {
        out.push_str(&format!(
            "  {:>3}    {:>3}    {:>3}    {:+}\n",
            i,
            painted[i],
            unpainted[i],
            painted[i] as i64 - unpainted[i] as i64
        ));
    }
    out
}

// --------------------------------------------------------------------------
// Test 1 — cube_4color: all four tool indices must appear in gcode
// --------------------------------------------------------------------------

#[test]
fn cube_4color_gcode_emits_all_four_tool_indices() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let found = parse_tool_index_lines(&outcome.gcode_text);
    let expected: BTreeSet<u32> = [0u32, 1, 2, 3].iter().copied().collect();

    assert_eq!(
        found, expected,
        "cube_4color emitted tools {found:?}, expected {expected:?} per D9 dispatch wiring \
         (Step 19 must route per-region variant chains so each Material ToolIndex \
         produces its own `T<n>` line)"
    );
}

// --------------------------------------------------------------------------
// Test 2 — Model A: per-color outer-wall fragmentation on painted layers (AC-4)
// --------------------------------------------------------------------------
//
// ADR-0013 (accepted 2026-06-23, "Model A"): MMU multi-color perimeters are a
// NON-OVERLAPPING partition of the painted interior into per-color cells; each
// cell runs a COMPLETE INDEPENDENT perimeter pass from its FULL contour
// (bisector edge included), offset inward by ext_perimeter_width/2. Near a
// bisector, BOTH adjacent colors trace their own wall, parallel and ~one
// line-width apart — never coincident, never deduplicated. The retired
// union-trace / skip-mask model (which merged the outer wall into one silhouette
// loop) is REJECTED. See docs/adr/0013.
//
// This E2E test asserts the four Model-A sub-properties of AC-4 on the full
// `cube_4color.3mf` fixture over its mid-body layers:
//
//   (a) per painted layer, the number of distinct per-color outer-wall FRAGMENTS
//       (`;TYPE:Outer wall` extrusion sequences) equals the number of distinct
//       tool indices present on that layer;
//   (b) each per-color fragment is a CLOSED loop, and the union of the fragments'
//       silhouette-portions covers the layer's external silhouette with no gap
//       beyond a color-boundary line-width and no double-trace (each silhouette
//       point owned by exactly one fragment); the total outer-wall length far
//       exceeds the bare silhouette perimeter because per-cell loops include
//       interior bisector walls;
//   (c) each fragment is preceded by a `T<N>` matching its ToolIndex;
//   (d) color transitions occur at cell-partition boundary junctions (interior
//       cell boundaries meeting the silhouette), NOT only at the 4 outer
//       silhouette corners, and the per-color fragments are independent closed
//       loops rather than one merged wall whose color flips at corners.
//
// Model-A NOTE: sub-assertions (b) and (d) were re-modeled (packet 109) away from
// the RETIRED union-trace premises — "total length == a single silhouette loop"
// and "transitions at the cube's 4 outer geometric corners" — which are FALSE
// under ADR-0013. Interior bisector walls legitimately add length, and cells
// meet at interior bisector junctions.
//
// Tolerances below were calibrated 2026-07-02 against the live pipeline output
// (per-loop closure observed 0.000mm; uniform-edge gap 0.000mm / ratio 1.000;
// multi-color-edge gap <= 0.834mm / ratio >= 0.891; silhouette overlap 0.000mm;
// total-length/silhouette-perimeter >= 2.25x; tool_changes == distinct_tools-1;
// >=1 non-corner transition/layer). All assertions are STRUCTURAL/geometric and
// robust to the known boostvoronoi medial-axis non-determinism (byte-exact SHA
// pinning is intentionally NOT used — see the determinism note at the end).
#[test]
fn cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes() {
    let painted = slice_fixture_file(&cube_4color_path());

    // Layer alignment guard: if the pipeline emits zero layers (stale guests),
    // fail loudly — do NOT silently skip assertions.
    let per_layer = parse_outer_wall_loops_per_layer(&painted.gcode_text);
    let tc_per_layer = tool_changes_per_layer(&painted.gcode_text);
    assert!(
        !per_layer.is_empty(),
        "AC-4: painted gcode has 0 ;LAYER_CHANGE markers. \
         Rebuild guests (cargo xtask build-guests) and re-run."
    );

    // Global: all four tool indices must appear (also covered by Test 1).
    let all_tools = parse_tool_index_lines(&painted.gcode_text);
    let expected_tools: BTreeSet<u32> = [0u32, 1, 2, 3].iter().copied().collect();
    assert_eq!(
        all_tools, expected_tools,
        "AC-4: not all four tool indices appear in the gcode. \
         Found: {all_tools:?}, expected: {expected_tools:?}"
    );

    // Mid-body window: exclude the bottom (~first 15%) and top (~last 20%) shell
    // layers, whose top/bottom solid-fill harvest replaces some perimeter arcs
    // (near-top layers legitimately leave multi-mm silhouette gaps). The window is
    // fraction-based so it is robust to minor layer-count drift.
    let n = per_layer.len();
    let lo = n * 15 / 100;
    let hi = n * 80 / 100;
    assert!(
        hi > lo + 5,
        "AC-4: too few layers ({n}) to form a mid-body window [{lo},{hi})"
    );

    // Calibrated tolerances (see header comment).
    const CLOSURE_EPS_MM: f32 = 0.30; // per-loop self-closure (observed 0.000)
    const EDGE_EPS_MM: f32 = 0.12; // on-silhouette membership band
    const OVERLAP_EPS_MM: f32 = 0.20; // silhouette double-trace tolerance (observed 0.000)
    const CORNER_TRIM_MM: f32 = 0.35; // corner exclusion for coverage
    const UNIFORM_GAP_MM: f32 = 0.30; // max gap on a single-tool edge (observed 0.000)
    const UNIFORM_RATIO: f32 = 0.97; // min coverage on a single-tool edge (observed 1.000)
    const MULTI_GAP_MM: f32 = 1.20; // max gap on a multi-tool edge (line-width boundaries; observed 0.834)
    const MULTI_RATIO: f32 = 0.82; // min coverage on a multi-tool edge (observed 0.891)
    const CORNER_TOL_MM: f32 = 1.5; // corner classification for (d)
    const MIN_LEN_RATIO: f32 = 1.5; // total outer-wall length / silhouette perimeter (observed >= 2.25)

    let mut mid_body_layers = 0usize;

    for li in lo..hi {
        let layer = &per_layer[li];
        assert!(
            !layer.is_empty(),
            "AC-4: mid-body layer {li} has no outer-wall loops"
        );
        mid_body_layers += 1;

        let sil = SilhouetteBox::from_loops(layer);
        // Distinct `;TYPE:Outer wall` extrusion-sequences (fragments) that produced
        // at least one loop, and the distinct tool indices tracing outer walls.
        let headers: BTreeSet<usize> = layer.iter().map(|l| l.header_idx).collect();
        let tools: BTreeSet<u32> = layer.iter().map(|l| l.tool).collect();

        // ---- (a) fragment count == distinct tool count (multi-color) --------
        assert!(
            tools.len() >= 3,
            "AC-4(a) layer {li}: expected >= 3 distinct per-color outer-wall fragments \
             (Model A: one per painted cell present), got {} tool(s): {:?}",
            tools.len(),
            tools
        );
        assert_eq!(
            headers.len(),
            tools.len(),
            "AC-4(a) layer {li}: number of ;TYPE:Outer wall fragments ({}) must equal the number \
             of distinct tool indices ({}) — each color traces exactly ONE contiguous outer-wall \
             extrusion sequence (ADR-0013 line 38). headers={:?} tools={:?}",
            headers.len(),
            tools.len(),
            headers,
            tools
        );

        // ---- (b) closed loops + silhouette coverage (Model A re-model) -------
        // (b1) every per-color fragment self-closes: it traces its OWN closed
        //      contour (bisector edge included), so seam ≈ final extrusion point.
        for (i, lp) in layer.iter().enumerate() {
            let g = lp.closure_gap();
            assert!(
                g <= CLOSURE_EPS_MM,
                "AC-4(b) layer {li} loop {i} (tool {}): fragment does not self-close — gap {:.3}mm \
                 > {:.2}mm. Under Model A each per-color region traces its own CLOSED contour; a \
                 non-closing loop means the region contour was truncated. seam={:?} last={:?}",
                lp.tool,
                g,
                CLOSURE_EPS_MM,
                lp.pts.first(),
                lp.pts.last()
            );
        }

        // (b2) interior bisector walls: total outer-wall length far exceeds the
        //      bare silhouette perimeter (NOT a single silhouette trace — the
        //      retired union-trace premise). Do NOT assert length == silhouette.
        let mut total_len = 0.0_f32;
        for lp in layer {
            let m = lp.pts.len();
            for k in 0..m {
                total_len += dist(lp.pts[k], lp.pts[(k + 1) % m]);
            }
        }
        let sil_perim = 2.0 * ((sil.x_max - sil.x_min) + (sil.y_max - sil.y_min));
        assert!(
            total_len >= MIN_LEN_RATIO * sil_perim,
            "AC-4(b) layer {li}: total outer-wall length {:.1}mm must be >= {:.1}x the bare \
             silhouette perimeter {:.1}mm — Model A per-color loops include interior bisector \
             walls, they are NOT a single silhouette loop. Got ratio {:.2}.",
            total_len,
            MIN_LEN_RATIO,
            sil_perim,
            total_len / sil_perim
        );

        // (b3) silhouette coverage: on every silhouette edge the union of the
        //      per-color arcs covers the edge (no untraced arc) with no
        //      double-trace (each silhouette point owned by exactly one fragment).
        //      Single-tool (uniform-face) edges are covered essentially fully;
        //      multi-tool edges (painted circles / bands) tile the edge with
        //      ~line-width gaps at cell boundaries (Model A "walls one line-width
        //      apart"), so a looser but still-strong bound applies there.
        for edge in [SilEdge::Left, SilEdge::Right, SilEdge::Front, SilEdge::Back] {
            let iv = edge_intervals(layer, &sil, edge, EDGE_EPS_MM);
            assert!(
                !iv.is_empty(),
                "AC-4(b) layer {li} edge {edge:?}: no outer-wall arc traces this silhouette edge \
                 at all (a per-color fragment failed to trace its silhouette portion)."
            );
            let (max_gap, max_overlap, cov, ext) =
                coverage_report(&iv, sil.edge_extent(edge), CORNER_TRIM_MM);
            let edge_tools: BTreeSet<u32> = iv.iter().map(|&(_, _, t)| t).collect();
            assert!(
                max_overlap <= OVERLAP_EPS_MM,
                "AC-4(b) layer {li} edge {edge:?}: silhouette DOUBLE-TRACE — per-color arcs overlap \
                 by {:.3}mm > {:.2}mm. Each silhouette point must be owned by exactly one fragment. \
                 intervals={:?}",
                max_overlap,
                OVERLAP_EPS_MM,
                iv
            );
            let ratio = if ext > 0.0 { cov / ext } else { 0.0 };
            let (gap_tol, ratio_min) = if edge_tools.len() <= 1 {
                (UNIFORM_GAP_MM, UNIFORM_RATIO)
            } else {
                (MULTI_GAP_MM, MULTI_RATIO)
            };
            assert!(
                max_gap <= gap_tol,
                "AC-4(b) layer {li} edge {edge:?} ({} tool(s)): silhouette coverage GAP {:.3}mm > \
                 {:.2}mm — a silhouette arc is untraced by any fragment. intervals={:?}",
                edge_tools.len(),
                max_gap,
                gap_tol,
                iv
            );
            assert!(
                ratio >= ratio_min,
                "AC-4(b) layer {li} edge {edge:?} ({} tool(s)): silhouette coverage RATIO {:.3} < \
                 {:.2} — the per-color fragments fail to cover this edge. cov={:.2}/{:.2}mm \
                 intervals={:?}",
                edge_tools.len(),
                ratio,
                ratio_min,
                cov,
                ext,
                iv
            );
        }

        // ---- (c) each fragment preceded by a matching T<N> ------------------
        // With (a) proven (each color is a single contiguous fragment), the layer
        // must carry exactly (distinct_tools - 1) tool-change lines: one T<N>
        // selecting every color except the single color carried over from the
        // previous layer's final tool. This proves each fragment (other than the
        // carried-in one) is immediately preceded by its own T<N> matching its
        // ToolIndex, and none is emitted without a selecting tool change.
        let tc = *tc_per_layer.get(li).unwrap_or(&0);
        assert_eq!(
            tc,
            tools.len() - 1,
            "AC-4(c) layer {li}: expected exactly {} tool-change (T<N>) lines (one per distinct \
             color, minus the color carried over from the previous layer), got {}. Each per-color \
             outer-wall fragment must be preceded by a T<N> matching its ToolIndex. tools={:?}",
            tools.len() - 1,
            tc,
            tools
        );

        // ---- (d) transitions at cell-partition boundaries, not outer corners
        // (d1) At least one silhouette color transition lies at a NON-corner cell
        //      boundary (e.g. a left-face painted circle or a front-face band
        //      meeting the silhouette between the outer corners). The retired
        //      union-trace model changes color only at the 4 outer silhouette
        //      corners, so it would have ZERO non-corner transitions.
        let mut noncorner_transitions = 0usize;
        for edge in [SilEdge::Left, SilEdge::Right, SilEdge::Front, SilEdge::Back] {
            let iv = edge_intervals(layer, &sil, edge, EDGE_EPS_MM);
            let (elo, ehi) = sil.edge_extent(edge);
            for w in iv.windows(2) {
                let (_, hi_i, ti) = w[0];
                let (lo_j, _, tj) = w[1];
                if ti != tj {
                    let boundary = 0.5 * (hi_i + lo_j);
                    let near_corner = (boundary - elo).abs() <= CORNER_TOL_MM
                        || (boundary - ehi).abs() <= CORNER_TOL_MM;
                    if !near_corner {
                        noncorner_transitions += 1;
                    }
                }
            }
        }
        assert!(
            noncorner_transitions >= 1,
            "AC-4(d) layer {li}: no non-corner silhouette color transition found. Model A partitions \
             the painted interior into per-color cells that meet the silhouette at interior \
             (non-corner) boundary junctions; the retired union-trace model would change color \
             only at the 4 outer corners. Zero non-corner transitions indicates the union-trace \
             regression."
        );
        // (d2) The per-color fragments are independent CLOSED loops, NOT one merged
        //      wall whose color flips at corners: there are at least as many distinct
        //      closed loops as distinct colors (>= 3 separate closed loops). A single
        //      merged union-trace wall would yield ~1 loop.
        assert!(
            layer.len() >= tools.len() && layer.len() >= 3,
            "AC-4(d) layer {li}: expected >= {} independent closed outer-wall loops (one or more \
             per color), got {}. A single merged union-trace wall would yield ~1 loop.",
            tools.len(),
            layer.len()
        );
    }

    assert!(
        mid_body_layers >= 10,
        "AC-4 sanity: expected >= 10 mid-body layers in window [{lo},{hi}), got {mid_body_layers}"
    );

    // Determinism note: byte-exact SHA pinning is intentionally NOT used here. The
    // medial axis runs on boostvoronoi 0.12.1 → cpp_map 0.2.0 → rand 0.9.4, whose
    // RNG-seeded skiplist makes the Voronoi output (gap-fill + MMU partition) vary
    // across runs (~1/3 of slices differ; inner-wall / gap-fill geometry drift). A
    // byte-exact hash would be a guaranteed CI flake. Every assertion above is a
    // STRUCTURAL / geometric Model-A invariant, robust to that non-determinism.
    // Do NOT re-introduce a byte-exact pin until the Voronoi path is deterministic.
    assert!(
        !painted.gcode_text.is_empty(),
        "cube_4color produced empty gcode"
    );
}

// --------------------------------------------------------------------------
// Test 3 — cube_fuzzyPainted: painted face shows visibly more jitter than
//          an unpainted face (proxy: more outer-wall coordinate points)
// --------------------------------------------------------------------------

/// Extract `G1 X<f> Y<f>` coordinates from outer-wall blocks of layers whose
/// Z roughly equals `target_z_mm` (within `tolerance_mm`). The result is the
/// flat list of (x, y) extrusion-move endpoints in those blocks.
fn outer_wall_points_at_z(gcode: &str, target_z_mm: f32, tolerance_mm: f32) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    let mut current_z: Option<f32> = None;
    let mut in_outer = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(";Z:") {
            if let Ok(z) = rest.split_whitespace().next().unwrap_or("").parse::<f32>() {
                current_z = Some(z);
            }
            continue;
        }
        if trimmed.starts_with(";LAYER_CHANGE") {
            in_outer = false;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(";TYPE:") {
            in_outer = rest.starts_with("Outer wall");
            continue;
        }
        if trimmed.starts_with(';') {
            // Another semantic comment — leave `in_outer` as-is until the
            // next ;TYPE: or ;LAYER_CHANGE flips it.
            continue;
        }
        if !in_outer {
            continue;
        }
        let z_match = match current_z {
            Some(z) => (z - target_z_mm).abs() <= tolerance_mm,
            None => false,
        };
        if !z_match {
            continue;
        }
        // Look for `G1 ... X<f> Y<f> ... E<f>` (extrusion moves only).
        if !(trimmed.starts_with("G1 ") || trimmed.starts_with("G1\t")) {
            continue;
        }
        if !trimmed.contains(" E") {
            continue;
        }
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        for tok in trimmed.split_whitespace() {
            if let Some(rest) = tok.strip_prefix('X') {
                x = rest.parse::<f32>().ok();
            } else if let Some(rest) = tok.strip_prefix('Y') {
                y = rest.parse::<f32>().ok();
            }
        }
        if let (Some(xv), Some(yv)) = (x, y) {
            points.push((xv, yv));
        }
    }
    points
}

#[test]
fn cube_fuzzy_painted_face_jitter_present_on_painted_face_only() {
    // cube_fuzzyPainted layout (world-space after build transform 125,115,12.5):
    //   Front (-Y, y ≈ 102.7)  — bare (un-jittered)
    //   Back  (+Y, y ≈ 127.3)  — bare (un-jittered)
    //   Left  (-X, x ≈ 112.7)  — bare (un-jittered)
    //   Right (+X, x ≈ 137.5)  — fuzzy painted (densely jittered, full-face)
    //
    // Measured at the mid-height layer (gcode point dump): the RIGHT face (+X)
    // carries dense fuzzy jitter (~221 outer-wall points / ~208 turns spanning
    // its full y-extent); the LEFT, FRONT and BACK faces are bare and emit only
    // a handful of corner/segment endpoints (LEFT≈12). We compare the count of
    // outer-wall extrusion endpoints on the fuzzy RIGHT face vs. the bare LEFT
    // face. Fuzzy skin injects many intermediate points along the perimeter, so
    // the painted-face count is materially higher than the clean face's.
    //
    // Threshold: painted face count > 2× unpainted face count (loose-but-clear
    // proxy for jitter; may need tightening once D9 dispatch is verified to
    // route through the fuzzy-skin module via the new variant-chain).
    let outcome = slice_fixture_file(&cube_fuzzy_painted_path());
    let mid_z = 12.5_f32;
    let tol = 0.6_f32;
    let pts = outer_wall_points_at_z(&outcome.gcode_text, mid_z, tol);
    assert!(
        !pts.is_empty(),
        "cube_fuzzyPainted: no outer-wall extrusion points captured at z≈{mid_z}±{tol} mm. \
         gcode parser found 0 candidate moves. Verify cube_fuzzyPainted slices and a \
         mid-height layer is emitted."
    );

    // Face bins (world space, mm). Use generous margins to absorb fuzz/jitter.
    //
    // cube_fuzzyPainted world-space extents (confirmed from gcode point dump):
    //   Right face (+X, FUZZY): x ≈ 137.5,  full y-extent — densely jittered.
    //   Left  face (-X, BARE):  x ≈ 112.7,  y ∈ [102.7, 127.3] (corners at y=102.7, y=127.3).
    //
    // The left face is BARE (un-jittered) so only its corner/segment endpoints appear in
    // gcode; the original y-band [103.5, 126.5] excluded both corners → widened to [102.0, 128.0].
    let mut painted_face_pts = 0usize; // Right face (FUZZY): x ≈ 137.5
    let mut unpainted_face_pts = 0usize; // Left face (BARE): x ≈ 112.7
    for &(x, y) in &pts {
        if x > 136.0 {
            painted_face_pts += 1;
        }
        if x < 113.5 && y > 102.0 && y < 128.0 {
            unpainted_face_pts += 1;
        }
    }

    assert!(
        painted_face_pts > 0,
        "cube_fuzzyPainted: painted face (right, x>136.0) captured 0 points. \
         Bin misalignment or fuzzy-skin module did not run. total_pts={}",
        pts.len()
    );
    assert!(
        unpainted_face_pts > 0,
        "cube_fuzzyPainted: unpainted face (left, x<113.5, 102.0<y<128.0) captured 0 points. \
         Bin misalignment. total_pts={}",
        pts.len()
    );

    // AC-4: painted face count must be > 2× unpainted face count.
    // The fuzzy-skin module injects intermediate perimeter points on the painted face;
    // the bare left face emits only its 2 corner endpoints.
    assert!(
        painted_face_pts as f32 > unpainted_face_pts as f32 * 2.0,
        "cube_fuzzyPainted: fuzzy face point count ({painted_face_pts}) is NOT > 2× \
         clean face point count ({unpainted_face_pts}) at z≈{mid_z}mm. Either the fuzzy-skin \
         module did not run on the painted face (D9 dispatch did not route through the \
         variant-chain for the painted region) or the proxy threshold needs revisiting."
    );

    // AC-4: the painted layer must have >= 2 distinct PaintValue colour regions.
    //
    // GCODE-LEVEL PROXY — WHY: `run_slice` returns `SliceOutcome` which exposes only
    // `gcode_text`, `layer_count`, and `wallclock_ms`. The intermediate `SliceIR` /
    // `PaintValue` region structures are consumed internally and not surfaced in the
    // public API. Adding them would require plumbing `SliceIR` through `SliceOutcome`
    // and threading it out of the full pipeline — non-trivial scope for this packet.
    // The sibling test (`cube_fuzzy_painted_tdd.rs`) exercises real `PaintValue`
    // assertions at the unit level via a direct `run_v2` call that returns `Vec<SliceIR>`.
    //
    // FAITHFULNESS: Each distinct paint zone (fuzzy region vs bare region) in the
    // perimeter module's output starts its own `;TYPE:Outer wall` block (after a
    // travel move or tool change). A slice that correctly dispatched the fuzzy and
    // bare zones as separate regions therefore produces >= 2 such headers on the
    // painted mid-body layer. This is a structural gcode-level proxy for ">=2 distinct
    // PaintValue regions reached the perimeter stage and each emitted its own wall
    // sequence". A monochrome (single-region) dispatch would produce exactly 1 header.
    let outer_wall_frags_at_mid_z: usize = {
        let mut count = 0usize;
        let mut in_target_layer = false;
        for line in outcome.gcode_text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix(";Z:") {
                if let Ok(z) = rest.split_whitespace().next().unwrap_or("").parse::<f32>() {
                    in_target_layer = (z - mid_z).abs() <= tol;
                }
                continue;
            }
            if t.starts_with(";LAYER_CHANGE") {
                in_target_layer = false;
                continue;
            }
            if in_target_layer && t == ";TYPE:Outer wall" {
                count += 1;
            }
        }
        count
    };
    assert!(
        outer_wall_frags_at_mid_z >= 2,
        "cube_fuzzyPainted: expected >= 2 distinct outer-wall fragments (PaintValue regions) \
         on the painted layer at z≈{mid_z}mm, got {outer_wall_frags_at_mid_z}. \
         The fuzzy and bare paint zones should produce separate outer-wall sequences."
    );
}

// --------------------------------------------------------------------------
// Regression tests — parity gaps found in the 2026-06-24 diagnose session.
//
// A G-code preview comparison against OrcaSlicer surfaced four shipped-but-broken
// behaviours on the painted cube. Each test below locks in one fix. They assert
// STRUCTURAL properties (presence / bounded counts), never byte-exact output, so
// they are robust to the known boostvoronoi medial-axis non-determinism.
// --------------------------------------------------------------------------

/// Count `;TYPE:<name>` block headers in gcode.
fn count_type(gcode: &str, ty: &str) -> usize {
    let needle = format!(";TYPE:{ty}");
    gcode.lines().filter(|l| l.trim() == needle).count()
}

/// Gap #1 regression: a PAINTED model must emit top/bottom solid surfaces.
///
/// Root cause (fixed): `PrePass::ShellClassification` ran before
/// `PrePass::PaintSegmentation`, which replaced every region with
/// `..Default::default()` — discarding the classified top/bottom solid fill. The
/// painted cube emitted ZERO `Top surface` / `Bottom surface` (open top, ~4×
/// extrusion deficit) while unpainted models were fine. This guards against the
/// per-color regions silently losing their solid fill again.
#[test]
fn cube_4color_painted_model_emits_top_and_bottom_solid_surfaces() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let top = count_type(&outcome.gcode_text, "Top surface");
    let bottom = count_type(&outcome.gcode_text, "Bottom surface");
    assert!(
        top > 0 && bottom > 0,
        "painted cube must emit top AND bottom solid surfaces (was 0/0 before the \
         ShellClassification→PaintSegmentation propagation fix); got Top={top}, Bottom={bottom}"
    );
}

/// Gap #2 regression: gap-fill must not flood the per-color bisector seams.
///
/// Root cause (fixed): the single-shot `difference_ex(innermost, infill_inset)`
/// rang the entire innermost contour — including the per-color MMU bisector edge —
/// producing ~302 phantom GapFill slivers ("wavy walls"). The OrcaSlicer-parity
/// incremental + infill-transition collection drops that to a small count of
/// genuine thin-feature gaps. The bound is generous (well under the old 302 and
/// well over the deterministic-ish ~86) so the known medial-axis non-determinism
/// cannot flake it.
#[test]
fn cube_4color_gapfill_does_not_flood_bisector_seams() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let gapfill = count_type(&outcome.gcode_text, "GapFill");
    assert!(
        gapfill < 150,
        "GapFill block count {gapfill} exceeds the regression ceiling (150). The pre-fix \
         single-shot collection flooded ~302 bisector-seam slivers; the incremental + \
         infill-transition port should keep this well below 150."
    );
}

/// Gap #3 regression: a multi-tool (MMU) model must auto-enable the wipe tower.
///
/// OrcaSlicer enables a prime/wipe tower automatically for multi-tool prints.
/// Ours defaulted `wipe_tower_enabled = false` with no auto-enable, so painted
/// models emitted zero `Prime tower` blocks. `run_slice` now auto-enables it when
/// the model paints ≥2 distinct tool indices (this fixture has 4). Sliced here
/// with no config, so only the auto-enable path can produce the tower.
#[test]
fn cube_4color_auto_enables_wipe_tower_for_mmu() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let prime = count_type(&outcome.gcode_text, "Prime tower");
    assert!(
        prime > 0,
        "multi-tool model must auto-enable the wipe tower (got {prime} Prime tower blocks). \
         run_slice should inject wipe_tower_enabled=true when >= 2 tool indices are painted."
    );
}

/// Gap #4 regression: the G-code header must declare per-filament colours.
///
/// OrcaSlicer's filament-view preview colours extrusions by the
/// `filament_colour` / `extruder_colour` header directives. Without them a
/// multi-tool print renders monochrome despite `T<n>` tool changes. The header
/// must now list one colour per filament slot (semicolon-separated).
#[test]
fn cube_4color_header_declares_per_filament_colours() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let colour_line = outcome
        .gcode_text
        .lines()
        .find(|l| l.trim_start().starts_with("; filament_colour ="))
        .unwrap_or_else(|| panic!("gcode header missing `; filament_colour =` directive"));
    let has_extruder = outcome
        .gcode_text
        .lines()
        .any(|l| l.trim_start().starts_with("; extruder_colour ="));
    assert!(
        has_extruder,
        "gcode header missing `; extruder_colour =` directive"
    );
    // The 4-color cube must declare multiple distinct, semicolon-separated colours.
    let value = colour_line.split('=').nth(1).unwrap_or("").trim();
    let colours: Vec<&str> = value.split(';').filter(|s| !s.trim().is_empty()).collect();
    assert!(
        colours.len() >= 2,
        "filament_colour must list >= 2 colours for a multi-tool model; got {colours:?}"
    );
}

/// AC-5 repeat test: the painted slice path must complete 10× in one process
/// without any single allocation reaching 1 GiB (the Voronoi/emit OOM signature).
///
/// The guarded #[global_allocator] (in executor/main.rs) would call exit(173)
/// and fail the test binary immediately if a ≥1 GiB alloc occurred. The fact
/// that all 10 iterations complete and produce non-empty gcode is therefore the
/// direct witness that the OOM is permanently gone — no tripwire fired.
#[test]
fn mmu_no_oversized_alloc_repeat() {
    let path = cube_fuzzy_painted_path();
    for i in 0..10 {
        let outcome = slice_fixture_file(&path);
        assert!(
            !outcome.gcode_text.is_empty(),
            "mmu_no_oversized_alloc_repeat: iteration {i} produced empty gcode — \
             pipeline returned an empty result"
        );
        assert!(
            outcome.gcode_text.contains("G1"),
            "mmu_no_oversized_alloc_repeat: iteration {i} produced gcode with no G1 moves — \
             slice was effectively a no-op"
        );
    }
    // If we reach here, all 10 iterations completed and the OOM guard never
    // tripped (it would have called std::process::exit(173) on any ≥1 GiB alloc).
}
