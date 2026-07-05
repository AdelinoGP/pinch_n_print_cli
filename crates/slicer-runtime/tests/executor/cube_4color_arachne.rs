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
//! # Per-color footprint bound (P113b re-verification)
//!
//! Geometric partition invariant (per-color ExPolygon sets form a non-overlapping
//! Voronoi partition of cube_4color's painted face) is asserted in
//! `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs`.
//! The "extrusion-points-in-footprint" investigation was tracked as
//! D-112-MMU-TOPOLOGY (now Closed — see `docs/DEVIATION_LOG.md`). The P113b
//! re-verification test `cube_4color_arachne_per_color_footprint_within_bbox`
//! asserts per-color outer-wall header bboxes stay inside the corresponding
//! per-color `paint_segmentation` ExPolygon bbox + 2 mm tolerance.
//!
//! **2026-07-05 closure status:** after six production fixes (busy-hub DCEL
//! bug, `connectJunctions` quad-chain generalization, junction-position
//! faithful port, a `layer_executor` tool-attribution precedence fix, and a
//! `boostvoronoi` wild-vertex clamp) plus two test-harness pairing fixes —
//! nearest-Z lookup (10th pass: 113 → 10 escaping headers) and finally
//! rebuilding the reference `paint_segmentation` plan directly from the real
//! gcode's own per-layer Z values (11th pass, see `build_initial_slice_ir`'s
//! doc comment), eliminating the aliasing that caused the residual 10-header
//! failure — this test and the full executor suite are green (180/180).
//!
//! This test asserts, per per-color outer-wall header: (1) all extrusion
//! points are **finite** (no NaN/Infinity); (2) each header traces a
//! non-trivial amount of real geometry; and (3) each header's bbox stays
//! within its per-color `paint_segmentation` cell bbox + 2 mm tolerance. The
//! primary, strong claim this test makes is property (1) in the other test in
//! this file, `cube_4color_arachne_fragments_walls_by_color` (per-color
//! fragmentation count) — verified robustly (every sampled mid-body layer
//! shows all 4 painted tools as 4 distinct outer-wall headers).

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{slice_ir::BoundingBox2, ConfigValue, ExPolygon, PaintValue, Point2};
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

    let opts = slice_cube_4color_with_arachne_options(&model_path, module_dir);
    run_slice(opts).unwrap_or_else(|e| {
        panic!(
            "run_slice (wall_generator=arachne) failed against {}: {e}",
            model_path.display()
        )
    })
}

fn slice_cube_4color_with_arachne_options(
    model_path: &PathBuf,
    module_dir: PathBuf,
) -> SliceRunOptions {
    let mesh = Arc::new(
        slicer_model_io::load_model(model_path)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model_path.display())),
    );

    let mut config_overrides = std::collections::HashMap::new();
    config_overrides.insert(
        "wall_generator".to_string(),
        ConfigValue::String("arachne".to_string()),
    );

    SliceRunOptions {
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
    }
}

fn load_cube_4color_mesh() -> slicer_ir::MeshIR {
    let path = cube_4color_path();
    assert!(path.exists(), "fixture missing: {}", path.display());
    slicer_model_io::load_model(&path).expect("load cube_4color.3mf should succeed")
}

/// Build the initial (pre-paint-segmentation) `SliceIR` sequence directly from
/// `layer_zs_mm` — the REAL per-layer Z values (mm) parsed from the real
/// `wall_generator=arachne` gcode's own `;Z:` headers (`parse_layer_z_values`)
/// — rather than an independently-synthesized fixed-step reference grid.
///
/// # Why not a synthesized `LayerPlanIR` (D-112-MMU-TOPOLOGY, 11th pass)
///
/// `execute_paint_segmentation` (the function this feeds) does not consume a
/// `LayerPlanIR` at all — only a `Vec<SliceIR>` (for its Z values and input
/// polygons) and a `RegionMapIR` (for layer count / variant-chain keys). The
/// previous `build_50_layer_plan` synthesized a full `LayerPlanIR` purely to
/// extract a `Vec<f32>` of Z values from it, at an independent, coarser
/// 50-layer/0.5mm step unrelated to the real pipeline's own resolved layer
/// height. That mismatch aliased the true tool1/tool3 paint-color boundary —
/// see this file's module doc comment and D-112-MMU-TOPOLOGY's closing note.
/// Building the `zs` directly from the real gcode's own Z sequence gives
/// `paint_segmentation` one sample per real layer, at the real layer's exact
/// Z, eliminating both the layer-count mismatch and the aliasing.
///
/// `effective_layer_height` per region is derived from consecutective real Z
/// deltas (`layer_zs_mm[idx] - layer_zs_mm[idx - 1]`, or `layer_zs_mm[0]` for
/// the first layer) rather than a hardcoded constant — real gcode layers are
/// not perfectly uniform (e.g. first-layer-height overrides). This value is
/// not read anywhere in `execute_paint_segmentation`'s current pipeline (only
/// `region_map.configs[0].layer_height` — a separate, config-level scalar —
/// feeds the Phase 6 top/bottom shell-window math), so this is cosmetic
/// correctness, not a functional requirement, but it costs nothing to keep
/// honest.
fn build_initial_slice_ir(
    object_id: &str,
    object_mesh: &slicer_ir::IndexedTriangleSet,
    layer_zs_mm: &[f32],
) -> Vec<slicer_ir::SliceIR> {
    use slicer_ir::SlicedRegion;
    let slabs = slicer_core::slice_mesh_ex(object_mesh, layer_zs_mm);
    layer_zs_mm
        .iter()
        .enumerate()
        .map(|(idx, &z)| {
            let polys = slabs.get(idx).cloned().unwrap_or_default();
            let effective_layer_height = if idx == 0 {
                z
            } else {
                z - layer_zs_mm[idx - 1]
            };
            slicer_ir::SliceIR {
                global_layer_index: idx as u32,
                z,
                regions: vec![SlicedRegion {
                    object_id: object_id.to_string(),
                    region_id: 0,
                    polygons: polys.clone(),
                    infill_areas: polys,
                    effective_layer_height,
                    segment_annotations: std::collections::HashMap::new(),
                    ..Default::default()
                }],
                ..Default::default()
            }
        })
        .collect()
}

fn build_region_map(object_id: &str, layer_count: u32) -> Arc<slicer_ir::RegionMapIR> {
    use slicer_ir::{
        RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    };
    let mut entries = std::collections::HashMap::new();
    for i in 0..layer_count {
        entries.insert(
            RegionKey {
                global_layer_index: i,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        );
    }
    Arc::new(RegionMapIR {
        schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        entries,
        configs: vec![ResolvedConfig::default()],
    })
}

/// Run `paint_segmentation` directly to obtain the per-layer per-color cells,
/// sampled at exactly `layer_zs_mm` — the real per-layer Z values (mm) parsed
/// from the real `wall_generator=arachne` gcode (`parse_layer_z_values`). The
/// returned `Vec<SliceIR>` therefore has the same length and Z ordering as the
/// gcode's own layer sequence, so callers can index it directly by real layer
/// index instead of nearest-Z matching against an independent reference plan
/// (see `build_initial_slice_ir`'s doc comment / D-112-MMU-TOPOLOGY).
fn run_paint_segmentation(
    mesh: Arc<slicer_ir::MeshIR>,
    layer_zs_mm: &[f32],
) -> Arc<Vec<slicer_ir::SliceIR>> {
    let object_id = &mesh.objects[0].id;
    let object_mesh = mesh.objects[0].mesh.clone();
    let initial = build_initial_slice_ir(object_id, &object_mesh, layer_zs_mm);
    let region_map = build_region_map(object_id, layer_zs_mm.len() as u32);
    slicer_core::algos::paint_segmentation::execute_paint_segmentation(
        mesh,
        Arc::new(initial),
        region_map,
    )
    .expect("execute_paint_segmentation must succeed")
}

/// World-space XY bounding box of the first object, in scaled units.
fn model_xy_bounding_box(mesh: &slicer_ir::MeshIR) -> BoundingBox2 {
    let obj = &mesh.objects[0];
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    for v in &obj.mesh.vertices {
        let p = slicer_core::transform_point3(&obj.transform.matrix, *v);
        let x = slicer_ir::mm_to_units(p.x);
        let y = slicer_ir::mm_to_units(p.y);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    BoundingBox2 {
        min: Point2 { x: min_x, y: min_y },
        max: Point2 { x: max_x, y: max_y },
    }
}

/// Group per-layer per-color cells. The BASE (empty variant_chain) cell is
/// keyed by `None`; painted Material/ToolIndex cells are keyed by `Some(t)`.
fn cells_by_color(slice_ir: &slicer_ir::SliceIR) -> BTreeMap<Option<u32>, Vec<ExPolygon>> {
    let mut out: BTreeMap<Option<u32>, Vec<ExPolygon>> = BTreeMap::new();
    for region in &slice_ir.regions {
        let color = region
            .variant_chain
            .iter()
            .find(|(sem, _)| sem == "material")
            .and_then(|(_, pv)| {
                if let PaintValue::ToolIndex(t) = pv {
                    Some(*t)
                } else {
                    None
                }
            });
        let entry = out.entry(color).or_default();
        for ep in &region.polygons {
            entry.push(ep.clone());
        }
    }
    out
}

/// Tight bounding box of a set of `ExPolygon`s, in scaled units.
fn polys_bbox(polys: &[ExPolygon]) -> Option<BoundingBox2> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    let mut any = false;
    for ep in polys {
        for p in &ep.contour.points {
            any = true;
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        for hole in &ep.holes {
            for p in &hole.points {
                any = true;
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }
    }
    if !any {
        return None;
    }
    Some(BoundingBox2 {
        min: Point2 { x: min_x, y: min_y },
        max: Point2 { x: max_x, y: max_y },
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

/// Parse each `;LAYER_CHANGE`-delimited layer's own Z (mm) from its `;Z:`
/// header comment, emitted immediately after `;LAYER_CHANGE` by
/// `crates/slicer-gcode/src/emit.rs` (`;LAYER_CHANGE` / `;Z:{...}` /
/// `;HEIGHT:{...}`, in that fixed order, once per layer — see that file's
/// `LayerFinalization` emission loop). Buckets 1:1 with
/// `parse_outer_wall_headers_per_layer` / `tool_changes_per_layer` (same
/// `;LAYER_CHANGE`-delimited layer indexing), so `parse_layer_z_values(gcode)
/// [li]` is li's real Z, independent of any reference plan's own layer
/// indexing.
fn parse_layer_z_values(gcode: &str) -> Vec<f32> {
    let marker = ";LAYER_CHANGE";
    let z_prefix = ";Z:";
    let mut out: Vec<f32> = Vec::new();
    let mut layer_started = false;
    let mut current_z: Option<f32> = None;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                out.push(current_z.unwrap_or_else(|| {
                    panic!(
                        "cube_4color (wall_generator=arachne): layer {} ended with no ;Z: header \
                         (expected immediately after ;LAYER_CHANGE)",
                        out.len()
                    )
                }));
            }
            layer_started = true;
            current_z = None;
            continue;
        }
        if layer_started && current_z.is_none() {
            if let Some(z_str) = trimmed.strip_prefix(z_prefix) {
                current_z = z_str.trim().parse::<f32>().ok();
            }
        }
    }
    if layer_started {
        out.push(current_z.unwrap_or_else(|| {
            panic!(
                "cube_4color (wall_generator=arachne): last layer ended with no ;Z: header \
                 (expected immediately after ;LAYER_CHANGE)"
            )
        }));
    }
    out
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

/// T-231 / P113b closure: every per-color outer-wall header's extrusion points
/// fall within the corresponding per-color ExPolygon cell's bounding box, padded
/// by a small tolerance.
///
/// The tolerance accounts for:
///   - one half extrusion width bead offset from the cell boundary;
///   - rounding in mm↔unit conversion;
///   - a single outer-wall fragment tracing near (but inside) the cell boundary.
///
/// The original D-112-MMU-TOPOLOGY symptom — "extrusion points land tens of mm
/// outside the naive per-face footprint" — is caught by even the coarser model
/// XY bbox bound (a header that escapes its per-color cell by tens of mm will
/// also escape the whole model).
///
/// # Real-Z reference plan (D-112-MMU-TOPOLOGY, 11th / closing pass)
///
/// `painted_ir` is built by feeding `run_paint_segmentation` the REAL
/// per-layer Z values parsed from the real `wall_generator=arachne` gcode's
/// own `;Z:` headers (`parse_layer_z_values`), one entry per real layer, at
/// the real layer's exact Z — see `build_initial_slice_ir`'s doc comment for
/// why this replaces the prior independent, hardcoded 50-layer/0.5mm-step
/// reference plan (`build_50_layer_plan`, removed).
///
/// Two prior passes investigated pairing: raw array-index pairing (dominant
/// cause of the original 113-header failure) was replaced by nearest-Z
/// lookup (10th pass: 113 → 10 escaping headers), which in turn was found to
/// alias the true tool1/tool3 paint-color boundary because the reference grid
/// was independently coarser than the real pipeline's own layer height (see
/// this file's module doc comment, "Per-color footprint bound", and
/// D-112-MMU-TOPOLOGY's closing note in `docs/DEVIATION_LOG.md` for the full
/// arc). Building `painted_ir` directly from the real Z sequence makes
/// `painted_ir` and `per_layer` the same length with matching Z ordering, so
/// `painted_ir[li]` is now an exact (not nearest-Z) pairing.
#[test]
fn cube_4color_arachne_per_color_footprint_within_bbox() {
    let outcome = slice_cube_4color_with_arachne();
    let mesh = Arc::new(load_cube_4color_mesh());

    assert!(
        !outcome.gcode_text.is_empty(),
        "cube_4color (wall_generator=arachne) produced empty gcode"
    );

    let per_layer = parse_outer_wall_headers_per_layer(&outcome.gcode_text);
    assert!(
        !per_layer.is_empty(),
        "cube_4color (wall_generator=arachne): 0 ;LAYER_CHANGE markers found. \
         Rebuild guests (cargo xtask build-guests) and re-run."
    );

    // Real per-layer Z (mm), parsed from each layer's own `;Z:` gcode header.
    // Fed directly into `run_paint_segmentation` below so `painted_ir` is
    // built at these exact Z values (see this test's doc comment /
    // D-112-MMU-TOPOLOGY).
    let layer_zs = parse_layer_z_values(&outcome.gcode_text);
    assert_eq!(
        layer_zs.len(),
        per_layer.len(),
        "cube_4color (wall_generator=arachne): parsed {} layer Z values but {} outer-wall header \
         layer buckets — ;LAYER_CHANGE bucketing mismatch between parse_layer_z_values and \
         parse_outer_wall_headers_per_layer",
        layer_zs.len(),
        per_layer.len()
    );

    let painted_ir = run_paint_segmentation(mesh.clone(), &layer_zs);
    let model_bbox = model_xy_bounding_box(&mesh);
    assert_eq!(
        painted_ir.len(),
        per_layer.len(),
        "cube_4color (wall_generator=arachne): painted_ir built from the real per-layer Z \
         sequence must have one entry per real gcode layer, got {} painted_ir entries vs {} \
         gcode layers",
        painted_ir.len(),
        per_layer.len()
    );

    // Tolerance: 2 mm in scaled units.
    const FOOTPRINT_TOLERANCE_MM: f32 = 2.0;
    let tol = slicer_ir::mm_to_units(FOOTPRINT_TOLERANCE_MM);

    let mut violations: Vec<String> = Vec::new();
    let mut worst_delta_mm: f32 = 0.0;
    let mut worst_layer: usize = 0;
    let mut worst_tool: u32 = 0;
    let mut worst_header_bbox_mm: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);
    let mut worst_cell_bbox_mm: (f32, f32, f32, f32) = (0.0, 0.0, 0.0, 0.0);

    let n = per_layer.len();
    let lo = n * 15 / 100;
    let hi = n * 80 / 100;

    for li in lo..hi {
        let layer_headers = &per_layer[li];
        if layer_headers.is_empty() {
            continue;
        }
        // `painted_ir` was built from the real per-layer Z sequence, so it
        // shares indexing and Z ordering with `per_layer` — direct index, no
        // nearest-Z lookup needed (see this test's doc comment /
        // D-112-MMU-TOPOLOGY).
        let slice_ir = &painted_ir[li];
        let cells = cells_by_color(slice_ir);

        for (i, h) in layer_headers.iter().enumerate() {
            if h.pts.len() < 2 {
                continue;
            }
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for &(x, y) in &h.pts {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            let header_bbox_units = BoundingBox2 {
                min: Point2 {
                    x: slicer_ir::mm_to_units(min_x),
                    y: slicer_ir::mm_to_units(min_y),
                },
                max: Point2 {
                    x: slicer_ir::mm_to_units(max_x),
                    y: slicer_ir::mm_to_units(max_y),
                },
            };

            // Prefer per-color cell bbox; fall back to full model bbox if this
            // tool has no matching cell on this layer.
            let bounds_units = cells
                .get(&Some(h.tool))
                .and_then(|polys| polys_bbox(polys))
                .unwrap_or(model_bbox);

            let expanded = BoundingBox2 {
                min: Point2 {
                    x: bounds_units.min.x - tol,
                    y: bounds_units.min.y - tol,
                },
                max: Point2 {
                    x: bounds_units.max.x + tol,
                    y: bounds_units.max.y + tol,
                },
            };

            let mut header_violation = false;
            let mut header_worst_delta_units: i64 = 0;
            let corners = [
                header_bbox_units.min,
                Point2 {
                    x: header_bbox_units.max.x,
                    y: header_bbox_units.min.y,
                },
                Point2 {
                    x: header_bbox_units.min.x,
                    y: header_bbox_units.max.y,
                },
                header_bbox_units.max,
            ];
            for p in corners {
                let dx_min = (expanded.min.x - p.x).max(0);
                let dx_max = (p.x - expanded.max.x).max(0);
                let dy_min = (expanded.min.y - p.y).max(0);
                let dy_max = (p.y - expanded.max.y).max(0);
                let d = dx_min.max(dx_max).max(dy_min).max(dy_max);
                if d > 0 {
                    header_violation = true;
                    header_worst_delta_units = header_worst_delta_units.max(d);
                }
            }

            let header_worst_delta_mm = slicer_ir::units_to_mm(header_worst_delta_units);
            if header_violation {
                let is_worst = header_worst_delta_mm > worst_delta_mm;
                if is_worst {
                    worst_delta_mm = header_worst_delta_mm;
                    worst_layer = li;
                    worst_tool = h.tool;
                    worst_header_bbox_mm = (min_x, min_y, max_x, max_y);
                    worst_cell_bbox_mm = (
                        slicer_ir::units_to_mm(bounds_units.min.x),
                        slicer_ir::units_to_mm(bounds_units.min.y),
                        slicer_ir::units_to_mm(bounds_units.max.x),
                        slicer_ir::units_to_mm(bounds_units.max.y),
                    );
                }
                violations.push(format!(
                    "layer {li} header {i} (tool {}): bbox {:?} exceeds per-color cell bbox \
                     {:?} by {header_worst_delta_mm:.3}mm",
                    h.tool,
                    (min_x, min_y, max_x, max_y),
                    worst_cell_bbox_mm
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "cube_4color_arachne_per_color_footprint_within_bbox: {} outer-wall header(s) escape \
         the per-color cell bbox (+/-{FOOTPRINT_TOLERANCE_MM}mm tolerance). \
         Worst case: layer {worst_layer} tool {worst_tool} by {worst_delta_mm:.3}mm; \
         header bbox {worst_header_bbox_mm:?} vs cell bbox {worst_cell_bbox_mm:?}. \
         Full list:\n{}",
        violations.len(),
        violations.join("\n")
    );
}
