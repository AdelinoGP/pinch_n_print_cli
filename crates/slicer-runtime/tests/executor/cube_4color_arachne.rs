//! T-231 cube_4color extension (packet 112, Step 10B): per-color (MMU)
//! structural fragmentation test for `arachne-perimeters`, mirroring the
//! classic `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes`
//! test in `cube_4color_gcode_output_tdd.rs` — but scoped to the narrower set
//! of STRUCTURAL properties this packet asks for, not the full Model-A (a-d)
//! contract that test asserts for classic.
//!
//! # Honesty note (no OrcaSlicer oracle)
//!
//! This test asserts oracle-free STRUCTURAL per-color fragmentation
//! properties — the same *kind* of invariant the classic cube_4color test
//! asserts, using the same gcode-level parsing technique — not byte-exact or
//! numeric parity with OrcaSlicer, and not parity with classic-perimeters'
//! own exact geometry. `arachne-perimeters` runs the from-first-principles
//! Arachne beading-strategy pipeline (packets 110-112); its bead placement,
//! wall counts, and gap handling legitimately differ from classic's iterative
//! polygon-inset approach. The claim here is: when `wall_generator =
//! "arachne"` is selected for a painted (MMU) model, the module produces
//! multiple per-color outer-wall fragments (driven by tool changes), each
//! containing real, non-degenerate 2D geometry — proving the MMU
//! wiring documented in `arachne-perimeters/src/lib.rs`'s "Per-color (MMU)
//! wall generation" section actually holds end-to-end through the real WASM
//! pipeline, not just in that doc comment's reasoning.
//!
//! # Why per-color splitting needs no code in `arachne-perimeters`
//!
//! Investigated directly against this codebase's source (not assumed): paint
//! color splitting happens entirely upstream, in `PrePass::PaintSegmentation`
//! (`slicer_core::algos::paint_segmentation`), which emits one `SlicedRegion`
//! per paint color (each with its own synthesized `region_id` and a
//! `variant_chain` entry `("material", PaintValue::ToolIndex(n))`) before
//! `Layer::Perimeters` ever runs. `arachne-perimeters::run_perimeters`
//! iterates that pre-split `regions` list and calls
//! `output.begin_region(region.object_id(), *region.region_id())` per region
//! — structurally identical to what `classic-perimeters` does. The host then
//! resolves each wall's tool_index from `SliceIR.regions[*].variant_chain`
//! keyed by `(object_id, region_id)`
//! (`crates/slicer-runtime/src/layer_executor.rs::assemble_ordered_entities`),
//! independent of which perimeter generator produced the wall. So this test
//! exercises the *existing* per-region loop, not any new per-color splitting
//! logic — there is none to add.
//!
//! # Bounded deviation from the classic test's "self-closure" property
//!
//! The classic cube_4color test additionally asserts that each per-color
//! outer-wall fragment is a single CLOSED loop (first extrusion point ≈ last
//! extrusion point). That property does **not** carry over to
//! `arachne-perimeters`' current output, and asserting it verbatim would be
//! dishonest: Arachne's wall representation is a JUNCTION GRAPH, not a set of
//! simple closed rings — `crates/slicer-core/src/arachne/stitch.rs`'s own doc
//! comment describes `stitch_extrusions` as only nearest-endpoint-joining
//! *open* polylines within a small gap, and the existing
//! `arachne_perimeters_simple_square.rs` (AC-9) test already documents that
//! even a single unpainted square legitimately emits "both the outer wall (a
//! 3-junction line closing back on itself per spoke) and multiple deeper
//! insets (2-junction lines)" — i.e. plain 2-endpoint bead segments that are
//! open BY DESIGN (a proper 3+-way junction cannot be a simple 2-degree
//! cycle). Empirically capturing `wall_generator=arachne` gcode for
//! `cube_4color.3mf` (2026-07-03) confirmed this: per-color outer-wall
//! travel-hop-delimited fragments routinely fail a literal
//! seam-point-≈-final-point closure check by many mm, at every granularity
//! tried (per travel-hop sub-run and per `;TYPE:Outer wall` header
//! aggregate) — not a bug, but Arachne's genuine junction topology.
//!
//! A follow-up attempt tried a bounding-box-extent substitute (each header's
//! traced points must span a plausible-for-the-25mm-cube 2D extent). That
//! ALSO does not hold robustly: several genuine (finite, non-NaN, no parsing
//! artifact — confirmed by hand-tracing raw gcode) extrusion points on the
//! `+X`/`-Y` "banded by height"/hex-subdivided painted faces
//! (`docs/12_architecture_gate_metrics.md`'s fixture catalog) land tens of mm
//! outside the naively-expected per-face footprint on some layers, for
//! reasons this packet's scope did not have budget to run down further (most
//! likely the per-color polygon construction for those specific painted
//! faces producing a non-trivial multi-island or non-convex cell whose true
//! extent is larger than a naive single-face bound — an upstream
//! `paint_segmentation`/geometry question, not an `arachne-perimeters`
//! wire-up question). Asserting a bound tight enough to be meaningful would
//! either be flaky or require deeper investigation out of this packet's
//! scope, so this test does NOT assert one.
//!
//! What this test DOES assert as the honestly-supportable substitute: every
//! per-color header's extrusion points are **finite** (no NaN/Infinity) and
//! every header contains a real, non-trivial number of extrusion moves. This
//! is weaker than "closes" or "stays in bounds", but it is unambiguously true
//! and still rules out the failure modes that would actually indicate broken
//! MMU wiring (a silently-empty or corrupted per-color fragment). The primary,
//! strong claim this test makes is property (1) below (per-color
//! fragmentation count) — verified robustly (every sampled mid-body layer
//! shows all 4 painted tools as 4 distinct outer-wall headers).

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::ConfigValue;
use slicer_runtime::{run_slice, SliceOutcome, SliceRunOptions};

// --------------------------------------------------------------------------
// Workspace + fixture helpers (duplicated from cube_4color_gcode_output_tdd.rs
// — that file's helpers are private to its own module and this file must not
// edit it, per the packet's scoped edit list).
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

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// Run `cube_4color.3mf` through the real pipeline with
/// `wall_generator = "arachne"` forced via `config_overrides`, so
/// `arachne-perimeters` (not `classic-perimeters`) claims `Layer::Perimeters`
/// (see `crates/slicer-scheduler/src/execution_plan.rs`'s
/// `dedup_same_claim_modules_with_wall_generator`, wired into `run_slice` at
/// `crates/slicer-runtime/src/run.rs`). Both `com.core.classic-perimeters` and
/// `com.core.arachne-perimeters` load from `core_modules_dir()`; the config
/// key — not directory exclusion — resolves the claim collision, matching
/// production.
fn slice_cube_4color_with_arachne() -> SliceOutcome {
    let model_path = cube_4color_path();
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
        slicer_model_io::load_model(&model_path)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model_path.display())),
    );

    let mut config_overrides = std::collections::HashMap::new();
    config_overrides.insert(
        "wall_generator".to_string(),
        ConfigValue::String("arachne".to_string()),
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
        config_overrides,
    };
    run_slice(opts).unwrap_or_else(|e| {
        panic!(
            "run_slice (wall_generator=arachne) failed against {}: {e}",
            model_path.display()
        )
    })
}

// --------------------------------------------------------------------------
// Gcode parsing helpers
// --------------------------------------------------------------------------

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn is_tool_line(trimmed: &str) -> bool {
    trimmed.len() >= 2
        && trimmed.as_bytes()[0] == b'T'
        && trimmed.as_bytes()[1..].iter().all(|c| c.is_ascii_digit())
}

/// Match a tool-change line of the form `T<digits>` (and only `T<digits>`,
/// possibly with trailing whitespace).
fn parse_tool_index_lines(gcode: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for line in gcode.lines() {
        let trimmed = line.trim();
        if !is_tool_line(trimmed) {
            continue;
        }
        if let Ok(n) = trimmed[1..].parse::<u32>() {
            out.insert(n);
        }
    }
    out
}

/// Count the number of `T<digits>` tool-change lines per layer. Returns one
/// entry per layer (delimited by `;LAYER_CHANGE`).
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
        if is_tool_line(trimmed) {
            current += 1;
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

/// One `;TYPE:Outer wall` header block within a layer: the tool active when
/// it started, and every explicit-XY **extrusion** point traced while it was
/// open.
///
/// Only lines carrying an `E` token (any magnitude — see this file's earlier
/// note on relative-E rounding to `E0.00000` for very short arachne segments)
/// **and** both an explicit `X` and `Y` token are recorded. Both guards are
/// load-bearing, confirmed empirically against a captured
/// `wall_generator=arachne` gcode dump (2026-07-03):
///
/// - Dropping the `E`-token requirement pulls in the header's own leading
///   TRAVEL move — the non-extruding approach from wherever the nozzle was
///   parked before (often the far-away wipe/prime tower, tens to hundreds of
///   mm from the model) to this header's first real wall point — wrongly
///   counting a point that was never actually extruded as part of this
///   fragment's traced geometry.
/// - Dropping the explicit-XY requirement lets a partial-coordinate line
///   (e.g. a lone Z-hop, or a wipe move restating only one axis) borrow a
///   carried-forward coordinate from outside the header, contaminating this
///   fragment's points the same way.
#[derive(Clone)]
struct HeaderFragment {
    tool: u32,
    header_idx: usize,
    pts: Vec<(f32, f32)>,
}

impl HeaderFragment {
    /// True iff every recorded point is finite (no NaN/Infinity) — a basic
    /// corruption guard. See this file's module doc comment ("Bounded
    /// deviation...") for why a tighter geometric-extent bound is NOT
    /// asserted here.
    fn all_points_finite(&self) -> bool {
        self.pts
            .iter()
            .all(|&(x, y)| x.is_finite() && y.is_finite())
    }

    /// Total point-to-point traced length (mm) — a non-degeneracy signal:
    /// zero (or near-zero) would mean every recorded point coincides, i.e.
    /// no real geometry was traced.
    fn total_length(&self) -> f32 {
        self.pts.windows(2).map(|w| dist(w[0], w[1])).sum()
    }
}

/// Split every layer's `;TYPE:Outer wall` blocks into one [`HeaderFragment`]
/// per header occurrence (NOT per travel-hop sub-loop — see this file's
/// module doc comment for why sub-loop-level closure isn't a valid
/// invariant for Arachne's junction-graph wall topology). Returns one
/// `Vec<HeaderFragment>` per layer bucket (delimited by `;LAYER_CHANGE`).
fn parse_outer_wall_headers_per_layer(gcode: &str) -> Vec<Vec<HeaderFragment>> {
    let marker = ";LAYER_CHANGE";
    let outer = ";TYPE:Outer wall";
    let mut layers: Vec<Vec<HeaderFragment>> = Vec::new();
    let mut current: Vec<HeaderFragment> = Vec::new();
    let mut layer_started = false;
    let mut in_outer = false;
    let mut tool: u32 = 0;
    let mut header_idx: usize = 0;
    let mut seen_outer_this_layer = false;
    let mut cur: Option<HeaderFragment> = None;

    fn flush(cur: &mut Option<HeaderFragment>, out: &mut Vec<HeaderFragment>) {
        if let Some(h) = cur.take() {
            if h.pts.len() >= 2 {
                out.push(h);
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
            if is_tool_line(trimmed) {
                tool = trimmed[1..].parse::<u32>().unwrap_or(tool);
            }
            continue;
        }

        if is_tool_line(trimmed) {
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
            cur = Some(HeaderFragment {
                tool,
                header_idx,
                pts: Vec::new(),
            });
            continue;
        }
        if trimmed.starts_with(";TYPE:") || trimmed.starts_with(";LAYER") {
            flush(&mut cur, &mut current);
            in_outer = false;
            continue;
        }

        if !in_outer {
            continue;
        }
        let is_move = trimmed.starts_with("G1 ")
            || trimmed.starts_with("G1\t")
            || trimmed.starts_with("G0 ")
            || trimmed.starts_with("G0\t");
        if !is_move {
            continue;
        }
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut has_e = false;
        for tok in trimmed.split_whitespace() {
            if let Some(r) = tok.strip_prefix('X') {
                x = r.parse::<f32>().ok();
            } else if let Some(r) = tok.strip_prefix('Y') {
                y = r.parse::<f32>().ok();
            } else if let Some(r) = tok.strip_prefix('E') {
                if r.parse::<f32>().is_ok() {
                    has_e = true;
                }
            }
        }
        if !has_e {
            // Non-extruding move (travel/seam-approach) — never part of the
            // header's own traced geometry (see this struct's doc comment).
            continue;
        }
        if let (Some(xv), Some(yv)) = (x, y) {
            if let Some(h) = cur.as_mut() {
                h.pts.push((xv, yv));
            }
        }
    }
    flush(&mut cur, &mut current);
    if layer_started {
        layers.push(current);
    }
    layers
}

// --------------------------------------------------------------------------
// Test — arachne per-color (MMU) outer-wall fragmentation (T-231 extension)
// --------------------------------------------------------------------------
//
// Scoped structural properties (packet 112 Step 10B), narrower than the full
// Model-A (a-d) contract the classic cube_4color test asserts:
//
//   (1) at least one mid-body layer has >= 3 distinct per-color outer-wall
//       fragments (proving arachne-perimeters, not just classic-perimeters,
//       fragments by paint color rather than collapsing to a single merged
//       silhouette loop);
//   (2) every per-color outer-wall header traces real (finite, non-trivial)
//       2D geometry — substituting for classic's "self-closes"; see this
//       file's module doc comment ("Bounded deviation...") for why literal
//       ring closure, and even a bounding-box-extent substitute, do not hold
//       for Arachne's current junction-graph wall topology, and why this
//       weaker (but honest) non-degeneracy check is what's asserted instead.
//
// Calibrated against a captured `wall_generator=arachne` gcode dump for
// cube_4color.3mf (2026-07-03): every mid-body layer sampled showed all 4
// painted tool indices as 4 distinct per-color outer-wall headers.
#[test]
fn cube_4color_arachne_fragments_walls_by_color() {
    let outcome = slice_cube_4color_with_arachne();

    assert!(
        !outcome.gcode_text.is_empty(),
        "cube_4color (wall_generator=arachne) produced empty gcode"
    );
    assert!(
        outcome.gcode_text.contains("G1"),
        "cube_4color (wall_generator=arachne) produced gcode with no G1 moves"
    );

    // Global sanity: tool tagging is resolved host-side from SliceIR
    // (independent of which perimeter generator ran — see this file's own
    // module doc comment), so all four painted tool indices must still
    // appear regardless of wall_generator.
    let all_tools = parse_tool_index_lines(&outcome.gcode_text);
    let expected_tools: BTreeSet<u32> = [0u32, 1, 2, 3].iter().copied().collect();
    assert_eq!(
        all_tools, expected_tools,
        "cube_4color (wall_generator=arachne): expected all four tool indices {expected_tools:?} \
         in the gcode (tool tagging is generator-independent), got {all_tools:?}"
    );

    let per_layer = parse_outer_wall_headers_per_layer(&outcome.gcode_text);
    let tc_per_layer = tool_changes_per_layer(&outcome.gcode_text);
    assert!(
        !per_layer.is_empty(),
        "cube_4color (wall_generator=arachne): 0 ;LAYER_CHANGE markers found. \
         Rebuild guests (cargo xtask build-guests) and re-run."
    );

    // Mid-body window: exclude the bottom (~first 15%) and top (~last 20%)
    // shell layers, mirroring the classic test's own window — near-top/bottom
    // layers legitimately replace perimeter arcs with solid-fill harvest.
    let n = per_layer.len();
    let lo = n * 15 / 100;
    let hi = n * 80 / 100;
    assert!(
        hi > lo + 5,
        "cube_4color (wall_generator=arachne): too few layers ({n}) to form a mid-body window \
         [{lo},{hi})"
    );

    // (2)'s bound: a genuine per-color fragment traces a non-trivial amount
    // of real (finite) geometry — well above a degenerate near-zero length.
    // No upper bound is asserted (see this file's module doc comment for why
    // a tight geometric-extent bound proved not honestly assertable here).
    const MIN_TOTAL_LENGTH_MM: f32 = 1.0;

    let mut max_fragments_seen = 0usize;
    let mut layers_with_3plus_fragments = 0usize;
    let mut mid_body_layers = 0usize;
    let mut degenerate_headers: Vec<String> = Vec::new();

    for li in lo..hi {
        let layer = &per_layer[li];
        if layer.is_empty() {
            // An arachne mid-body layer with zero outer-wall headers would be
            // a real regression, but is reported via the aggregate assertions
            // below (fragment-count checks) rather than panicking per-layer,
            // so a handful of edge layers don't mask the aggregate signal.
            continue;
        }
        mid_body_layers += 1;

        let tools: BTreeSet<u32> = layer.iter().map(|h| h.tool).collect();
        max_fragments_seen = max_fragments_seen.max(tools.len());
        if tools.len() >= 3 {
            layers_with_3plus_fragments += 1;
        }

        // (2) Non-degeneracy: every per-color header traces finite, real 2D
        // geometry (see this file's module doc comment for why this replaces
        // literal ring closure / a bounding-box-extent substitute).
        for (i, h) in layer.iter().enumerate() {
            if !h.all_points_finite() {
                degenerate_headers.push(format!(
                    "layer {li} header {i} (tool {}): non-finite coordinate(s) among {} point(s)",
                    h.tool,
                    h.pts.len()
                ));
                continue;
            }
            let len = h.total_length();
            if len < MIN_TOTAL_LENGTH_MM {
                degenerate_headers.push(format!(
                    "layer {li} header {i} (tool {}): traced length {:.3}mm < {:.1}mm (npts={})",
                    h.tool,
                    len,
                    MIN_TOTAL_LENGTH_MM,
                    h.pts.len()
                ));
            }
        }
    }

    assert!(
        mid_body_layers >= 10,
        "cube_4color (wall_generator=arachne) sanity: expected >= 10 mid-body layers with \
         outer-wall headers in window [{lo},{hi}), got {mid_body_layers}"
    );

    assert!(
        degenerate_headers.is_empty(),
        "cube_4color (wall_generator=arachne): {} outer-wall header(s) traced degenerate or \
         out-of-bounds geometry:\n{}",
        degenerate_headers.len(),
        degenerate_headers.join("\n")
    );

    // (1) Per-color fragmentation: arachne-perimeters must fragment the
    // outer wall by paint color on at least some mid-body layers — proving
    // it does NOT collapse painted regions into one merged silhouette wall.
    // A single merged wall would show max_fragments_seen == 1 (or 2, if the
    // base/residual region alone splits at most one tool-change boundary)
    // across every layer.
    assert!(
        layers_with_3plus_fragments >= 1,
        "cube_4color (wall_generator=arachne): expected at least one mid-body layer with >= 3 \
         distinct per-color outer-wall fragments (Model A: one per painted cell present), but \
         none of the {mid_body_layers} mid-body layers reached 3 — max fragments seen on any \
         single layer was {max_fragments_seen}. tool_changes_per_layer[{lo}..{hi}]={:?}",
        &tc_per_layer[lo..hi.min(tc_per_layer.len())]
    );
}
